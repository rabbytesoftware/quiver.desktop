//! The settings layout: a small-caps section title over rows of `label + description` on the left and
//! the control on the right (SettingsAfter / SettingsEngineAfter).

use gpui::prelude::*;
use gpui::{div, px, AnyElement, FontWeight, IntoElement, SharedString};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::Palette;

/// A section: its title, then the rows. `first` leaves out the divider above it.
pub fn section(
    p: &Palette,
    title: impl Into<SharedString>,
    first: bool,
    rows: Vec<AnyElement>,
) -> impl IntoElement {
    let title: SharedString = title.into();
    div()
        .flex()
        .flex_col()
        .when(!first, |d| {
            d.mt(px(28.))
                .pt(px(26.))
                .border_t_1()
                .border_color(p.border)
        })
        .child(
            div()
                .mb(px(16.))
                .text_size(px(11.))
                .line_height(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted_foreground)
                .child(title.to_uppercase()),
        )
        .children(rows)
}

/// One setting. `controls` sit at the right edge; `id` names the row for the e2e bridge.
pub fn row(
    p: &Palette,
    id: &str,
    label: impl Into<SharedString>,
    description: Option<SharedString>,
    controls: Vec<AnyElement>,
) -> AnyElement {
    let label: SharedString = label.into();
    div()
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(24.))
        .mb(px(26.))
        .min_h(px(36.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .min_w_0()
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.foreground)
                        .child(label.clone()),
                )
                .when_some(description, |d, text| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted_foreground)
                            .child(text),
                    )
                }),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(8.))
                .children(controls),
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

/// The "reset" affordance of a row that differs from its default; absent otherwise.
pub fn reset_button(
    probe_id: &str,
    label: impl Into<SharedString>,
    can_reset: bool,
    on_click: impl Fn(&mut gpui::Window, &mut gpui::App) + 'static,
) -> Option<AnyElement> {
    can_reset.then(|| {
        small_button(
            probe_id,
            label,
            quiver_ui::ButtonVariant::Ghost,
            true,
            on_click,
        )
    })
}
