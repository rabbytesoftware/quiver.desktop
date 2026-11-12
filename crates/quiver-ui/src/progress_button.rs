//! `ProgressButton`: the action that becomes its own progress (design board "The action is the
//! progress"). A soft fill is the progress; line 1 names the action, line 2 the step and count;
//! the only pressable part is the trailing "Steps" toggle. No business logic: callers feed text,
//! a 0..=1 fraction and the toggle state.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, FontWeight, SharedString, StyleRefinement,
    Window,
};

use crate::button::Button;
use crate::button_style::ButtonVariant;
use crate::flicker_spinner::FlickerSpinner;
use crate::icon::{icon, Glyph};
use crate::ids::child_id;
use crate::theme::ActiveQuiver;

const HEIGHT: f32 = 40.0;
const RADIUS: f32 = 11.0;

/// Width in px of the fill for `fraction` of a `width`-px track.
pub(crate) fn fill_width(
    fraction: f32,
    width: f32,
) -> f32 {
    fraction.clamp(0.0, 1.0) * width
}

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct ProgressButton {
    id: ElementId,
    title: SharedString,
    detail: SharedString,
    fraction: f32,
    width: f32,
    steps_open: bool,
    leading: Option<AnyElement>,
    on_steps: Option<ClickFn>,
    style: StyleRefinement,
}

impl ProgressButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: SharedString::default(),
            detail: SharedString::default(),
            fraction: 0.0,
            width: 256.0,
            steps_open: false,
            leading: None,
            on_steps: None,
            style: StyleRefinement::default(),
        }
    }

    /// Line 1 (`Updating`).
    pub fn title(
        mut self,
        t: impl Into<SharedString>,
    ) -> Self {
        self.title = t.into();
        self
    }

    /// Line 2 (`Verify checksum · 2 of 4`).
    pub fn detail(
        mut self,
        t: impl Into<SharedString>,
    ) -> Self {
        self.detail = t.into();
        self
    }

    /// Progress in `0.0..=1.0`.
    pub fn progress(
        mut self,
        fraction: f32,
    ) -> Self {
        self.fraction = fraction;
        self
    }

    /// Width of the progress track (the Steps button sits outside it).
    pub fn width(
        mut self,
        px_: f32,
    ) -> Self {
        self.width = px_;
        self
    }

    pub fn steps_open(
        mut self,
        open: bool,
    ) -> Self {
        self.steps_open = open;
        self
    }

    /// Replaces the default 15px FlickerSpinner.
    pub fn leading(
        mut self,
        el: impl IntoElement,
    ) -> Self {
        self.leading = Some(el.into_any_element());
        self
    }

    pub fn on_steps(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_steps = Some(Rc::new(f));
        self
    }
}

impl Styled for ProgressButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ProgressButton {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let spinner = self.leading.unwrap_or_else(|| {
            FlickerSpinner::new(child_id(&self.id, "spinner"))
                .size(15.)
                .color(p.foreground)
                .into_any_element()
        });
        let track = div()
            .relative()
            .flex_none()
            .w(px(self.width))
            .h(px(HEIGHT))
            .overflow_hidden()
            .rounded(px(RADIUS))
            .bg(p.muted)
            // Painted already cut to the track's rounded ends: `overflow_hidden` clips to the rectangle only, and a
            // plain fill showed square corners outside the track.
            .child(crate::clipped_rect(
                (0., 0., fill_width(self.fraction, self.width), HEIGHT),
                (self.width, HEIGHT, RADIUS),
                p.foreground.opacity(0.12),
            ))
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px((HEIGHT - 15.) / 2.))
                    .child(spinner),
            )
            .child(
                div()
                    .absolute()
                    .left(px(38.))
                    .top_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(3.))
                    .whitespace_nowrap()
                    .child(
                        div()
                            .text_size(px(14.5))
                            .line_height(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.foreground)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(14.))
                            .text_color(p.muted_foreground)
                            .child(self.detail),
                    ),
            );
        let glyph = if self.steps_open {
            Glyph::ChevronUp
        } else {
            Glyph::ChevronDown
        };
        let mut steps = Button::new(child_id(&self.id, "steps"))
            .variant(ButtonVariant::Outline)
            .w(px(36.))
            .h(px(HEIGHT))
            .px(px(0.))
            .rounded(px(RADIUS))
            .pressed(self.steps_open)
            .child(icon(glyph, 15., p.foreground));
        if let Some(f) = self.on_steps {
            steps = steps.on_click(move |e, w, cx| f(e, w, cx));
        }
        let mut el = div()
            .id(self.id)
            .flex()
            .items_start()
            .gap(px(4.))
            .child(track)
            .child(steps);
        crate::style_merge::apply(el.style(), &self.style);
        el
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_is_clamped() {
        assert_eq!(fill_width(0.36, 256.0), 0.36 * 256.0);
        assert_eq!(fill_width(2.0, 100.0), 100.0);
        assert_eq!(fill_width(-1.0, 100.0), 0.0);
    }
}
