//! `compare-boards --shots <dir> --boards <dir> [--out <dir>] [--threshold <0-255>]`: for every PNG that
//! exists in both directories (same file name) writes `<out>/<name>.png` (app | board | difference) and a
//! `summary.tsv` with the share of pixels that differ. It renders nothing itself: the boards are
//! exported by the design tool, the shots come from `crates/quiver-app/capture.sh`.
//!
//! The ratio is a pure measure (a pixel counts when any channel differs by more than the threshold,
//! default 24). It is a pointer to where to look, not a pass/fail score.

use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use crate::args::Args;

pub struct Rgba {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

fn decode(path: &Path) -> Result<Rgba> {
    let mut decoder =
        png::Decoder::new(File::open(path).with_context(|| path.display().to_string())?);
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    buf.truncate(info.buffer_size());
    let data = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => bail!("{}: indexed PNG was not expanded", path.display()),
    };
    Ok(Rgba {
        width: info.width as usize,
        height: info.height as usize,
        data,
    })
}

fn encode(
    path: &Path,
    img: &Rgba,
) -> Result<()> {
    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        img.width as u32,
        img.height as u32,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&img.data)?;
    Ok(())
}

fn pixel(
    img: &Rgba,
    x: usize,
    y: usize,
) -> [u8; 4] {
    if x >= img.width || y >= img.height {
        return [255, 0, 255, 255];
    }
    let i = (y * img.width + x) * 4;
    [img.data[i], img.data[i + 1], img.data[i + 2], 255]
}

fn differs(
    a: [u8; 4],
    b: [u8; 4],
    threshold: u8,
) -> bool {
    (0..3).any(|c| a[c].abs_diff(b[c]) > threshold)
}

/// Share of the compared area (the larger of the two sizes) whose pixels differ; pixels outside the
/// smaller image count as different.
pub fn difference_ratio(
    a: &Rgba,
    b: &Rgba,
    threshold: u8,
) -> f64 {
    let (w, h) = (a.width.max(b.width), a.height.max(b.height));
    let total = (w * h).max(1);
    let bad = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| differs(pixel(a, x, y), pixel(b, x, y), threshold))
        .count();
    bad as f64 / total as f64
}

fn side_by_side(
    a: &Rgba,
    b: &Rgba,
    threshold: u8,
) -> Rgba {
    let (w, h) = (a.width.max(b.width), a.height.max(b.height));
    let gap = 8;
    let width = w * 3 + gap * 2;
    let mut data = vec![128u8; width * h * 4];
    let mut put = |x: usize, y: usize, px: [u8; 4]| {
        let i = (y * width + x) * 4;
        data[i..i + 4].copy_from_slice(&px);
    };
    for y in 0..h {
        for x in 0..w {
            let (pa, pb) = (pixel(a, x, y), pixel(b, x, y));
            put(x, y, pa);
            put(w + gap + x, y, pb);
            let d = if differs(pa, pb, threshold) {
                [220, 38, 38, 255]
            } else {
                [pa[0] / 4 + 190, pa[1] / 4 + 190, pa[2] / 4 + 190, 255]
            };
            put(2 * (w + gap) + x, y, d);
        }
    }
    Rgba {
        width,
        height: h,
        data,
    }
}

pub fn run(raw: Vec<String>) -> Result<()> {
    let args = Args::parse(raw, &["shots", "boards", "out", "threshold"])?;
    let shots = Path::new(args.flag("shots").context("--shots <dir> is required")?);
    let boards = Path::new(args.flag("boards").context("--boards <dir> is required")?);
    let out = Path::new(args.flag("out").unwrap_or("docker/native/evidence/compare"));
    let threshold: u8 = args
        .flag("threshold")
        .map(str::parse)
        .transpose()?
        .unwrap_or(24);
    fs::create_dir_all(out)?;
    let mut names: Vec<_> = fs::read_dir(boards)?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".png"))
        .collect();
    names.sort();
    let mut summary = String::from("screen\tdiffering_pixels\n");
    for name in names {
        let shot = shots.join(&name);
        if !shot.exists() {
            println!("{name}: no app screenshot, skipped");
            continue;
        }
        let (app, board) = (decode(&shot)?, decode(&boards.join(&name))?);
        let ratio = difference_ratio(&app, &board, threshold);
        encode(&out.join(&name), &side_by_side(&app, &board, threshold))?;
        println!("{name}: {:.2}% differ", ratio * 100.0);
        summary.push_str(&format!("{name}\t{:.4}\n", ratio * 100.0));
    }
    fs::write(out.join("summary.tsv"), summary)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(
        w: usize,
        h: usize,
        v: u8,
    ) -> Rgba {
        Rgba {
            width: w,
            height: h,
            data: vec![v; w * h * 4],
        }
    }

    #[test]
    fn identical_images_do_not_differ() {
        assert_eq!(difference_ratio(&flat(4, 4, 10), &flat(4, 4, 10), 24), 0.0);
    }

    #[test]
    fn small_deltas_stay_below_the_threshold() {
        assert_eq!(difference_ratio(&flat(2, 2, 10), &flat(2, 2, 30), 24), 0.0);
        assert_eq!(difference_ratio(&flat(2, 2, 10), &flat(2, 2, 200), 24), 1.0);
    }

    #[test]
    fn the_missing_area_counts_as_different() {
        assert_eq!(difference_ratio(&flat(2, 1, 10), &flat(1, 1, 10), 24), 0.5);
    }
}
