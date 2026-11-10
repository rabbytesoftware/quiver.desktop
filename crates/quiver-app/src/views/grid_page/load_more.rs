use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, FontWeight, IntoElement};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, WavyLine};

/// "Load more" centred on a light wavy line that closes the grid.
pub fn load_more<V: 'static>(
    probe_id: &'static str,
    label: String,
    cx: &mut Context<V>,
    on_click: impl Fn(&mut V, &mut Context<V>) + 'static,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .h(px(64.))
        .mt(px(26.))
        .child(
            div().absolute().left_0().right_0().top(px(21.)).child(
                WavyLine::new()
                    .color(p.border)
                    .amplitude(3.)
                    .wavelength(14.),
            ),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(17.))
                .flex()
                .justify_center()
                .child(
                    div()
                        .id(probe_id)
                        .relative()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(124.))
                        .h(px(30.))
                        .rounded(px(15.))
                        .bg(p.background)
                        .border_1()
                        .border_color(p.foreground.opacity(0.17))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .cursor(CursorStyle::PointingHand)
                        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
                        .child(label.clone())
                        .child(probe(probe_id, Kind::Button, label, true, "")),
                ),
        )
}
