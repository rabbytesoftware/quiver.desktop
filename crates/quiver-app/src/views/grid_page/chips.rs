use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, FontWeight, IntoElement, SharedString};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::ActiveQuiver;

/// A filter chip: dark when active, outlined otherwise. `probe_id` is its e2e id.
pub fn chip<V: 'static>(
    probe_id: String,
    label: String,
    active: bool,
    cx: &mut Context<V>,
    on_click: impl Fn(&mut V, &mut Context<V>) + 'static,
) -> impl IntoElement {
    counted_chip(probe_id, label, None, active, cx, on_click)
}

/// [`chip`] with a muted count after the label (`github.com 11`).
pub fn counted_chip<V: 'static>(
    probe_id: String,
    label: String,
    count: Option<usize>,
    active: bool,
    cx: &mut Context<V>,
    on_click: impl Fn(&mut V, &mut Context<V>) + 'static,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let spoken = match count {
        Some(n) => format!("{label} {n}"),
        None => label.clone(),
    };
    div()
        .id(SharedString::from(probe_id.replace('.', "-")))
        .relative()
        .flex()
        .items_center()
        .h(px(28.))
        .px(px(12.))
        .rounded(px(14.))
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor(CursorStyle::PointingHand)
        .when(active, |d| d.bg(p.primary).text_color(p.primary_foreground))
        .when(!active, |d| {
            d.bg(p.background)
                .text_color(p.foreground)
                .border_1()
                .border_color(p.foreground.opacity(0.17))
        })
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(label)
        .children(count.map(|n| {
            div()
                .ml(px(6.))
                .text_color(if active {
                    p.primary_foreground.opacity(0.64)
                } else {
                    p.muted_foreground
                })
                .child(n.to_string())
        }))
        .child(probe(
            probe_id,
            Kind::Button,
            spoken,
            true,
            if active { "selected" } else { "" },
        ))
}
