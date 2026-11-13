//! `Tooltip`: port of `src/components/ui/tooltip.tsx` (`side="top"`, `align="center"`, `sideOffset=4`).
//!
//! GPUI's hover tooltip supplies the timing (show after the hover delay, hide on leave) but places the
//! popup at the pointer. A [`Tip`] also records its trigger's window bounds with an invisible canvas
//! laid over the trigger, and the popup draws itself `anchored` in window coordinates, centred 4px above
//! that box, kept inside the window like the old positioner's collision padding.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    anchored, black, canvas, div, point, px, AnyView, App, Bounds, Context, Corner, Div, Pixels,
    SharedString, Stateful, Window,
};
use quiver_theme::{FontFamily, Radius};

use crate::edge::control_edge;
use crate::elevation::shadow_md;
use crate::theme::ActiveQuiver;

/// `sideOffset`: the gap between the trigger's top edge and the popup.
const SIDE_OFFSET: f32 = 4.;
/// `text-xs`: 12px on a 16px line.
const TEXT_SIZE: f32 = 12.;
const LINE_HEIGHT: f32 = 16.;
/// `px-2`.
const PAD_X: f32 = 8.;

type Anchor = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The popup: `rounded-md border bg-popover text-popover-foreground text-xs shadow-md/5 px-2 py-1`.
pub struct TooltipPopup {
    text: SharedString,
    anchor: Option<Bounds<Pixels>>,
}

impl Render for TooltipPopup {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let q = *cx.quiver();
        let p = q.palette;
        let popup = div()
            .relative()
            .flex_none()
            .whitespace_nowrap()
            .rounded(px(Radius::MD))
            .border_1()
            .border_color(p.border)
            .bg(p.popover)
            .text_color(p.popover_foreground)
            .font_family(FontFamily::Sans.family())
            .text_size(px(TEXT_SIZE))
            .line_height(px(LINE_HEIGHT))
            .px(px(PAD_X))
            .py(px(4.))
            .shadow(shadow_md(black().opacity(0.05)))
            .child(control_edge(&q, Radius::MD, 0.06))
            .child(self.text.clone());
        let Some(anchor) = self.anchor else {
            // No trigger box recorded (the trigger has not painted yet): GPUI's pointer placement.
            return popup.into_any_element();
        };
        let mut style = window.text_style();
        style.font_family = FontFamily::Sans.family().into();
        let text_width = window
            .text_system()
            .shape_line(
                self.text.clone(),
                px(TEXT_SIZE),
                &[style.to_run(self.text.len())],
                None,
            )
            .width;
        // Border (1 + 1) plus the horizontal padding around the shaped line.
        let width = text_width + px(2. * PAD_X + 2.);
        let x = anchor.center().x - width / 2.;
        let y = anchor.top() - px(SIDE_OFFSET);
        anchored()
            .position_mode(gpui::AnchoredPositionMode::Window)
            .position(point(x, y))
            .anchor(Corner::BottomLeft)
            .snap_to_window_with_margin(px(4.))
            .child(popup)
            .into_any_element()
    }
}

/// A tooltip for one trigger: `Button::tooltip(quiver_ui::tooltip("Settings"))`, or
/// `div().id("x").tip(quiver_ui::tooltip("Settings"))` on a plain element.
pub struct Tip {
    text: SharedString,
    anchor: Anchor,
}

/// Builds a [`Tip`] showing `text`.
pub fn tooltip(text: impl Into<SharedString>) -> Tip {
    Tip {
        text: text.into(),
        anchor: Rc::default(),
    }
}

impl Tip {
    /// The invisible box laid over the trigger that records where it is in the window.
    pub(crate) fn probe(&self) -> impl IntoElement {
        let anchor = self.anchor.clone();
        canvas(
            move |bounds, _, _| anchor.set(Some(bounds)),
            |_, (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }

    /// The view builder GPUI's `.tooltip(...)` takes.
    pub(crate) fn builder(self) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let Tip { text, anchor } = self;
        move |_, cx| {
            let anchor = anchor.get();
            let text = text.clone();
            cx.new(|_| TooltipPopup { text, anchor }).into()
        }
    }
}

/// `.tip(tooltip(..))` for any element with an id.
pub trait TipExt: Sized {
    fn tip(
        self,
        tip: Tip,
    ) -> Self;
}

impl TipExt for Stateful<Div> {
    fn tip(
        self,
        tip: Tip,
    ) -> Self {
        self.child(tip.probe()).tooltip(tip.builder())
    }
}
