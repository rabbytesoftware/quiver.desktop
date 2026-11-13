//! A "cover" image with rounded corners.
//!
//! GPUI's `img` with `ObjectFit::Cover` paints the picture at the *cover* size (larger than the box on one
//! axis) and rounds the corners of that oversized quad, so the visible part, cut by a rectangular clip, has
//! square corners whenever the picture's aspect differs from the box's. Here the picture is cropped to the
//! box's aspect on the CPU so the quad painted is exactly the box and its corners round.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    canvas, px, size, App, Bounds, Corners, Element, ImageSource, ImgResourceLoader, Pixels,
    RenderImage, Styled, Window,
};
use image::{Frame, RgbaImage};

/// Where, in a `src_w`×`src_h` picture, the part that covers a `box_w`×`box_h` box lies, centred: `(x, y, w, h)`.
pub(crate) fn cover_crop(
    src_w: u32,
    src_h: u32,
    box_w: u32,
    box_h: u32,
) -> (u32, u32, u32, u32) {
    if src_w == 0 || src_h == 0 || box_w == 0 || box_h == 0 {
        return (0, 0, src_w, src_h);
    }
    // Compare src_w/src_h with box_w/box_h without division.
    let wide = u64::from(src_w) * u64::from(box_h) > u64::from(box_w) * u64::from(src_h);
    if wide {
        let w = (u64::from(src_h) * u64::from(box_w) / u64::from(box_h)).clamp(1, u64::from(src_w))
            as u32;
        ((src_w - w) / 2, 0, w, src_h)
    } else {
        let h = (u64::from(src_w) * u64::from(box_h) / u64::from(box_w)).clamp(1, u64::from(src_h))
            as u32;
        (0, (src_h - h) / 2, src_w, h)
    }
}

/// The part of the `sw`×`sh` RGBA `bytes` that covers a `box_px` box, cropped by copying whole row slices.
/// Nothing per-pixel and nothing from the `image` crate's generic code: that compiles with this crate's
/// settings, and unoptimised a crop-and-resize of one 1280×640 banner took 230 ms on the UI thread, which
/// froze the window while a page of banners arrived.
fn crop_to_cover(
    bytes: &[u8],
    sw: u32,
    sh: u32,
    box_px: (u32, u32),
) -> Option<RgbaImage> {
    let (x, y, w, h) = cover_crop(sw, sh, box_px.0, box_px.1);
    if bytes.len() != sw as usize * sh as usize * 4 || w == 0 || h == 0 {
        return None;
    }
    let (stride, row) = (sw as usize * 4, w as usize * 4);
    let mut out = Vec::with_capacity(row * h as usize);
    for line in y as usize..(y + h) as usize {
        let from = line * stride + x as usize * 4;
        out.extend_from_slice(&bytes[from..from + row]);
    }
    RgbaImage::from_raw(w, h, out)
}

/// Aspect ratios are told apart to 1/ASPECT_STEPS: a window being resized changes a tile's size every frame
/// but keeps its shape, and one crop serves them all (the GPU scales the quad). A finer key made an entry, and
/// an atlas texture, per frame.
const ASPECT_STEPS: u32 = 64;
/// Entries are only released once this many have piled up...
const SWEEP_ABOVE: usize = 48;
/// ...and only those unused for this long. A texture the current frame's scene still points at must never be
/// freed (GPUI panics in its Metal atlas when it is), and a frame takes milliseconds.
const IDLE_BEFORE_RELEASE: Duration = Duration::from_secs(10);

struct Entry {
    image: Arc<RenderImage>,
    used: Instant,
}

thread_local! {
    static CACHE: RefCell<HashMap<(usize, u32), Entry>> = RefCell::new(HashMap::new());
}

/// The box's aspect ratio (width over height) in `1/ASPECT_STEPS`.
pub(crate) fn aspect_key(
    box_w: u32,
    box_h: u32,
) -> u32 {
    (u64::from(box_w) * u64::from(ASPECT_STEPS) * 2 / (u64::from(box_h.max(1)) * 2)).max(1) as u32
}

/// `data` cropped to the aspect of a box, made once per image and aspect.
fn covered(
    data: &Arc<RenderImage>,
    aspect: u32,
    window: &mut Window,
) -> Option<Arc<RenderImage>> {
    let key = (data.id.0, aspect);
    let now = Instant::now();
    let hit = CACHE.with(|c| {
        c.borrow_mut().get_mut(&key).map(|entry| {
            entry.used = now;
            entry.image.clone()
        })
    });
    if hit.is_some() {
        return hit;
    }
    let src = data.size(0);
    let (sw, sh) = (
        u32::try_from(src.width.0).ok()?,
        u32::try_from(src.height.0).ok()?,
    );
    let fitted = crop_to_cover(data.as_bytes(0)?, sw, sh, (aspect, ASPECT_STEPS))?;
    let made = Arc::new(RenderImage::new(vec![Frame::new(fitted)]));
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= SWEEP_ABOVE {
            let idle: Vec<_> = c
                .iter()
                .filter(|(_, e)| now.duration_since(e.used) >= IDLE_BEFORE_RELEASE)
                .map(|(k, _)| *k)
                .collect();
            for key in idle {
                if let Some(old) = c.remove(&key) {
                    window.drop_image(old.image).ok();
                }
            }
        }
        c.insert(
            key,
            Entry {
                image: made.clone(),
                used: now,
            },
        );
    });
    Some(made)
}

/// An element filling its parent that paints `source` covering it, corners rounded by `radius`.
pub(crate) fn cover(
    source: ImageSource,
    radius: f32,
) -> impl Element + Styled {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, cx: &mut App| {
            let data = match &source {
                ImageSource::Resource(resource) => {
                    window.use_asset::<ImgResourceLoader>(resource, cx)
                }
                ImageSource::Render(data) => Some(Ok(data.clone())),
                _ => None,
            };
            let Some(Ok(data)) = data else { return };
            let device = |v: Pixels| f32::from(v).round().max(1.0) as u32;
            let aspect = aspect_key(device(bounds.size.width), device(bounds.size.height));
            let Some(cropped) = covered(&data, aspect, window) else {
                return;
            };
            let radii = Corners::all(px(radius))
                .clamp_radii_for_quad_size(size(bounds.size.width, bounds.size.height));
            window.paint_image(bounds, radii, cropped, 0, false).ok();
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_picture_loses_its_sides_and_a_tall_one_its_top_and_bottom() {
        // 1600x630 into a 440x218 box: height kept, width cut to the box's aspect.
        let (x, y, w, h) = cover_crop(1600, 630, 440, 218);
        assert_eq!((y, h), (0, 630));
        assert!((x * 2 + w).abs_diff(1600) <= 1, "centred");
        assert!((i64::from(w) * 218 - i64::from(h) * 440).abs() < 218);
        // 600x1200 into 440x218: width kept, height cut.
        let (x, y, w, h) = cover_crop(600, 1200, 440, 218);
        assert_eq!((x, w), (0, 600));
        assert!((y * 2 + h).abs_diff(1200) <= 1);
        assert!((i64::from(w) * 218 - i64::from(h) * 440).abs() < 440);
        // 1200x630 (a GitHub social card) is *narrower* than that box, so it loses top and bottom.
        let (x, y, w, h) = cover_crop(1200, 630, 440, 218);
        assert_eq!((x, w), (0, 1200));
        assert!((y * 2 + h).abs_diff(630) <= 1);
    }

    #[test]
    fn a_picture_of_the_boxs_own_aspect_is_kept_whole() {
        assert_eq!(cover_crop(880, 436, 440, 218), (0, 0, 880, 436));
    }

    #[test]
    fn the_crop_keeps_the_centre_pixels_of_each_row() {
        // 6x2 picture whose pixel (x, y) is [x, y, 0, 255]; a 2x2 box takes the centre 2 columns.
        let (sw, sh) = (6u32, 2u32);
        let bytes: Vec<u8> = (0..sh)
            .flat_map(|y| (0..sw).flat_map(move |x| [x as u8, y as u8, 0, 255]))
            .collect();
        let out = crop_to_cover(&bytes, sw, sh, (2, 2)).unwrap();
        assert_eq!(out.dimensions(), (2, 2));
        assert_eq!(out.get_pixel(0, 0).0, [2, 0, 0, 255]);
        assert_eq!(out.get_pixel(1, 1).0, [3, 1, 0, 255]);
    }

    #[test]
    fn a_crop_of_a_real_banner_is_not_slow_even_unoptimised() {
        // The regression: crop + resize took 230 ms per banner here. A copy of rows is orders cheaper.
        let bytes = vec![120u8; 1280 * 640 * 4];
        let started = std::time::Instant::now();
        let out = crop_to_cover(&bytes, 1280, 640, (878, 436)).unwrap();
        assert!(out.width() > 0 && out.height() > 0);
        assert!(
            started.elapsed() < std::time::Duration::from_millis(100),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_resize_keeps_the_aspect_key_so_it_keeps_the_cached_crop() {
        // A 440x218 tile being dragged one pixel at a time across a few widths, shape unchanged.
        let keys: std::collections::HashSet<_> =
            (438..=446).map(|w| aspect_key(w, w * 218 / 440)).collect();
        assert!(keys.len() <= 2, "{keys:?}");
        assert_ne!(aspect_key(440, 218), aspect_key(213, 213));
    }

    #[test]
    fn degenerate_sizes_keep_the_whole_picture() {
        assert_eq!(cover_crop(0, 0, 10, 10), (0, 0, 0, 0));
        assert_eq!(cover_crop(10, 10, 0, 5), (0, 0, 10, 10));
    }
}
