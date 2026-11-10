//! The settings layout: a small-caps section title over rows of `label + description` on the left and
//! the control on the right (SettingsAfter / SettingsEngineAfter).

use gpui::prelude::*;
use gpui::PathBuilder;
use gpui::{
    canvas, div, point, px, AnyElement, App, Bounds, FontWeight, IntoElement, Pixels, SharedString,
    Window,
};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::Palette;

/// A section: its heading, then the rows. The first section's heading is left out, as in the old app.
pub fn section(
    p: &Palette,
    title: impl Into<SharedString>,
    first: bool,
    rows: Vec<AnyElement>,
) -> impl IntoElement {
    let title: SharedString = title.into();
    div()
        .px(px(4.))
        .py(px(2.))
        .flex()
        .flex_col()
        .when(!first, |d| {
            d.child(
                div()
                    .mb(px(8.))
                    .px(px(4.))
                    .pt(px(16.))
                    .pb(px(6.))
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.muted_foreground)
                    .child(title.to_uppercase()),
            )
        })
        .child(div().flex().flex_col().gap(px(8.)).children(rows))
}

/// One setting. `controls` sit at the right edge; `id` names the row for the e2e bridge.
pub fn row(
    p: &Palette,
    id: &str,
    label: impl Into<SharedString>,
    description: Option<SharedString>,
    controls: Vec<AnyElement>,
) -> AnyElement {
    row_with_reset(p, id, label, description, controls, None)
}

/// [`row`] with the reset affordance beside the label (see [`reset_button`]).
pub fn row_with_reset(
    p: &Palette,
    id: &str,
    label: impl Into<SharedString>,
    description: Option<SharedString>,
    controls: Vec<AnyElement>,
    reset: Option<AnyElement>,
) -> AnyElement {
    let label: SharedString = label.into();
    let described = description.clone();
    let hover = p.muted.opacity(0.5);
    div()
        .id(SharedString::from(format!("{id}.hover")))
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .px(px(4.))
        .py(px(8.))
        .rounded(px(8.))
        .hover(move |s| s.bg(hover))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(16.25))
                                .text_color(p.foreground)
                                .child(label.clone()),
                        )
                        .children(reset),
                )
                .when_some(description, |d, text| {
                    d.child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .line_height(px(19.5))
                            .text_color(p.muted_foreground)
                            .child(text),
                    )
                }),
        )
        .when(!controls.is_empty(), |d| {
            d.child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .children(controls),
            )
        })
        .children(
            described.map(|text| probe(format!("{id}.description"), Kind::Text, text, true, "")),
        )
        .child(probe(format!("{id}.row"), Kind::Other, label, true, ""))
        .into_any_element()
}

/// Wraps a control so the e2e bridge can find and drive it (the kit's switches and selects report
/// nothing themselves).
pub fn probed(
    control: impl IntoElement,
    id: String,
    kind: Kind,
    label: SharedString,
    state: String,
) -> AnyElement {
    div()
        .relative()
        .child(control)
        .child(probe(id, kind, label, true, state))
        .into_any_element()
}

/// A small outline button that reports itself as `probe_id`.
pub fn small_button(
    probe_id: &str,
    label: impl Into<SharedString>,
    variant: quiver_ui::ButtonVariant,
    enabled: bool,
    on_click: impl Fn(&mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let label: SharedString = label.into();
    quiver_ui::Button::new(SharedString::from(probe_id.replace('.', "-")))
        .variant(variant)
        .size(quiver_ui::ButtonSize::Sm)
        .disabled(!enabled)
        .label(label.clone())
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(probe(probe_id.to_owned(), Kind::Button, label, enabled, ""))
        .into_any_element()
}

/// The reset affordance beside a row's label: a 20px icon button, kept in the layout but invisible
/// while the row is at its default (as in the old app).
pub fn reset_button(
    p: &Palette,
    probe_id: &str,
    label: impl Into<SharedString>,
    can_reset: bool,
    on_click: impl Fn(&mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let label: SharedString = label.into();
    let color = p.muted_foreground;
    let hover = p.accent;
    let slot = div()
        .relative()
        .flex_none()
        .size(px(20.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center();
    if !can_reset {
        return slot.into_any_element();
    }
    slot.id(SharedString::from(probe_id.replace('.', "-")))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(reset_glyph(12., color))
        .child(probe(probe_id.to_owned(), Kind::Button, label, true, ""))
        .into_any_element()
}

fn reset_glyph(
    size: f32,
    color: gpui::Hsla,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            let k = size / 24.0;
            let at =
                |x: f32, y: f32| point(bounds.origin.x + px(x * k), bounds.origin.y + px(y * k));
            let mut arc = PathBuilder::stroke(px(2.0 * k));
            let steps = 20;
            for i in 0..=steps {
                let a = (-40.0_f32 + 290.0 * i as f32 / steps as f32).to_radians();
                let (x, y) = (12.0 + 8.0 * a.cos(), 12.0 - 8.0 * a.sin());
                if i == 0 {
                    arc.move_to(at(x, y));
                } else {
                    arc.line_to(at(x, y));
                }
            }
            if let Ok(path) = arc.build() {
                window.paint_path(path, color);
            }
            let mut head = PathBuilder::stroke(px(2.0 * k));
            head.move_to(at(3.0, 3.0));
            head.line_to(at(3.0, 8.5));
            head.line_to(at(8.5, 8.5));
            if let Ok(path) = head.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(size))
    .flex_none()
}
