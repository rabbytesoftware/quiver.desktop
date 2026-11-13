//! `Photo`: a remote image filling its (relatively positioned) parent, with a stand-in drawn only until the image
//! has loaded. Once it has, the stand-in is not drawn at all (an image with transparent areas must not show it
//! through), and if the image cannot be loaded the stand-in stays.

use gpui::prelude::*;

use crate::cover::cover;
use gpui::{div, img, AnyElement, App, ImageSource, ImgResourceLoader, ObjectFit, Window};

#[derive(IntoElement)]
pub struct Photo {
    source: ImageSource,
    fit: ObjectFit,
    radius: f32,
    stand_in: Option<AnyElement>,
}

impl Photo {
    pub fn new(url: impl Into<ImageSource>) -> Self {
        Self {
            source: url.into(),
            fit: ObjectFit::Cover,
            radius: 0.0,
            stand_in: None,
        }
    }

    pub fn fit(
        mut self,
        fit: ObjectFit,
    ) -> Self {
        self.fit = fit;
        self
    }

    /// Corner radius of the image and of the layer that holds it.
    pub fn radius(
        mut self,
        radius: f32,
    ) -> Self {
        self.radius = radius;
        self
    }

    /// What shows until the image has loaded (and for good if it fails). It should position itself absolutely
    /// to fill the same parent as the photo.
    pub fn stand_in(
        mut self,
        stand_in: impl IntoElement,
    ) -> Self {
        self.stand_in = Some(stand_in.into_any_element());
        self
    }
}

/// Whether the image behind `source` is decoded and ready, as the `img` element will find it.
fn loaded(
    source: &ImageSource,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    match source {
        ImageSource::Resource(resource) => matches!(
            window.get_asset::<ImgResourceLoader>(resource, cx),
            Some(Ok(_))
        ),
        ImageSource::Render(_) => true,
        _ => false,
    }
}

impl RenderOnce for Photo {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let ready = loaded(&self.source, window, cx);
        let radius = gpui::px(self.radius);
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .rounded(radius)
            .when(!ready, |layer| layer.children(self.stand_in))
            .child(if matches!(self.fit, ObjectFit::Cover) {
                // Cropped to the box first, or the corners would not round (see `cover`).
                cover(self.source, self.radius)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .into_any_element()
            } else {
                img(self.source)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(self.fit)
                    .rounded(radius)
                    .into_any_element()
            })
    }
}
