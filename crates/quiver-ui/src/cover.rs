//! A "cover" image with rounded corners.
//!
//! GPUI's `img` with `ObjectFit::Cover` paints the picture at the *cover* size (larger than the box on one
//! axis) and rounds the corners of that oversized quad, so the visible part, cut by a rectangular clip, has
//! square corners whenever the picture's aspect differs from the box's. Here the picture is cropped to the
//! box's aspect on the CPU (and shrunk to the box's pixel size, which also keeps far less memory resident than
//! the full-resolution original), so the quad painted is exactly the box and its corners round.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    canvas, px, size, App, Bounds, Corners, Element, ImageSource, ImgResourceLoader, Pixels,
    RenderImage, Styled, Window,
};
use image::imageops::{crop_imm, resize, FilterType};
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

const CACHE_CAPACITY: usize = 256;

thread_local! {
    static CACHE: RefCell<HashMap<(usize, u32, u32), Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// `data` cropped to cover a `box_w`×`box_h` pixel box (never enlarged), made once per image and size.
fn covered(
    data: &Arc<RenderImage>,
    box_w: u32,
    box_h: u32,
    window: &mut Window,
) -> Option<Arc<RenderImage>> {
    let key = (data.id.0, box_w, box_h);
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(hit);
    }
    let src = data.size(0);
    let (sw, sh) = (
        u32::try_from(src.width.0).ok()?,
        u32::try_from(src.height.0).ok()?,
    );
    let (x, y, w, h) = cover_crop(sw, sh, box_w, box_h);
    let whole = RgbaImage::from_raw(sw, sh, data.as_bytes(0)?.to_vec())?;
    let cropped = crop_imm(&whole, x, y, w, h).to_image();
    let fitted = if w > box_w && h > box_h {
        resize(&cropped, box_w, box_h, FilterType::Triangle)
    } else {
        cropped
    };
    let made = Arc::new(RenderImage::new(vec![Frame::new(fitted)]));
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= CACHE_CAPACITY {
            for (_, old) in c.drain() {
                window.drop_image(old).ok();
            }
        }
        c.insert(key, made.clone());
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
            let scale = window.scale_factor();
            let device = |v: Pixels| (f32::from(v) * scale).round().max(1.0) as u32;
            let Some(cropped) = covered(
                &data,
                device(bounds.size.width),
                device(bounds.size.height),
                window,
            ) else {
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
    fn degenerate_sizes_keep_the_whole_picture() {
        assert_eq!(cover_crop(0, 0, 10, 10), (0, 0, 0, 0));
        assert_eq!(cover_crop(10, 10, 0, 5), (0, 0, 10, 10));
    }
}
