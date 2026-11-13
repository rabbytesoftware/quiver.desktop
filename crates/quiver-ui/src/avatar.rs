//! `Avatar`: port of `src/components/ui/avatar.tsx` (image with a muted fallback).

use gpui::prelude::*;
use gpui::{
    div, img, px, App, ElementId, FontWeight, ImageSource, ObjectFit, SharedString,
    StyleRefinement, Window,
};

use crate::theme::ActiveQuiver;

#[derive(IntoElement)]
pub struct Avatar {
    id: ElementId,
    size: f32,
    src: Option<ImageSource>,
    fallback: SharedString,
    style: StyleRefinement,
}

impl Avatar {
    /// `size-8` (32px) by default.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            size: 32.0,
            src: None,
            fallback: SharedString::default(),
            style: StyleRefinement::default(),
        }
    }

    pub fn size(
        mut self,
        px_: f32,
    ) -> Self {
        self.size = px_;
        self
    }

    pub fn src(
        mut self,
        src: impl Into<ImageSource>,
    ) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Text shown while there is no image or it fails to load (`AvatarFallback`).
    pub fn fallback(
        mut self,
        text: impl Into<SharedString>,
    ) -> Self {
        self.fallback = text.into();
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn fallback_circle(
    text: SharedString,
    muted: gpui::Hsla,
    fg: gpui::Hsla,
) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(muted)
        .text_color(fg)
        .child(text)
}

impl RenderOnce for Avatar {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let (muted, fg) = (p.muted, p.foreground);
        let key = self.id.to_string();
        let text = self.fallback.clone();
        let mut el = div()
            .id(self.id)
            .debug_selector(move || key)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .size(px(self.size))
            .rounded_full()
            .bg(p.background)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM);
        el = match self.src {
            Some(src) => el.child(
                img(src)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(move || {
                        fallback_circle(text.clone(), muted, fg).into_any_element()
                    }),
            ),
            None => el.child(fallback_circle(text, muted, fg)),
        };
        crate::style_merge::apply(el.style(), &self.style);
        el
    }
}
