//! What the pane shows while a GPUI menu or dialog covers an arrow app's page.
//!
//! The page is a native webview above GPUI's pixels, so anything GPUI draws over it (the "⋯" menu, the
//! Details dialog) would sit under it: the page steps aside meanwhile ([`super::ArrowWindows::set_covered`]).
//! Hiding it bare left the pane empty (on a dark page: a black hole) where Tauri, whose app was part of the
//! same page, kept it in sight under its overlays. So the page is photographed first and the pane draws the
//! photograph under the overlay until the page is back.

use std::sync::{Arc, LazyLock};

use gpui::RenderImage;
use tokio::sync::watch;

static COVER: LazyLock<watch::Sender<Option<Arc<RenderImage>>>> =
    LazyLock::new(|| watch::channel(None).0);

/// The photograph of the covered page, while it is covered.
pub fn current() -> Option<Arc<RenderImage>> {
    COVER.borrow().clone()
}

/// Changes whenever a photograph is taken or dropped: the pane redraws on it.
pub fn subscribe() -> watch::Receiver<Option<Arc<RenderImage>>> {
    COVER.subscribe()
}

pub(crate) fn publish(image: Option<Arc<RenderImage>>) {
    COVER.send_replace(image);
}

/// A cairo ARGB32 surface as a GPUI image. Cairo's ARGB32 is native-endian premultiplied, so on the
/// little-endian targets we ship its bytes are B, G, R, A: the order GPUI's sprite atlas takes as is.
#[cfg(target_os = "linux")]
pub(crate) fn from_cairo(surface: &gtk::cairo::Surface) -> Option<RenderImage> {
    use gtk::cairo::{Context, Format, ImageSurface};
    let source = ImageSurface::try_from(surface.clone()).ok()?;
    let (w, h) = (source.width(), source.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut copy = ImageSurface::create(Format::ARgb32, w, h).ok()?;
    {
        let cr = Context::new(&copy).ok()?;
        cr.set_source_surface(&source, 0., 0.).ok()?;
        cr.paint().ok()?;
    }
    copy.flush();
    let stride = usize::try_from(copy.stride()).ok()?;
    let (w, h) = (usize::try_from(w).ok()?, usize::try_from(h).ok()?);
    let data = copy.data().ok()?;
    let mut bytes = Vec::with_capacity(w * h * 4);
    for row in 0..h {
        bytes.extend_from_slice(data.get(row * stride..row * stride + w * 4)?);
    }
    let buffer = image::RgbaImage::from_raw(u32::try_from(w).ok()?, u32::try_from(h).ok()?, bytes)?;
    Some(RenderImage::new(vec![image::Frame::new(buffer)]))
}
