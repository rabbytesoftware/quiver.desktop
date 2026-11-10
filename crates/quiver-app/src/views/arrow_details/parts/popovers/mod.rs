//! The two popovers under the action row: the live Steps checklist of the running action
//! (`ArrowActivityAfter`) and the ⓘ preview of what an action would run.

mod steps;

use gpui::prelude::*;
use gpui::{black, deferred, div, px, AnyElement, Context, FontWeight, IntoElement};
use quiver_state::arrow_details::progress::plain_text_for;
use quiver_state::i18n::en::arrow;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, Button, ButtonSize, ButtonVariant, Palette};

use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

/// Below the action row: it ends at 178 + 40, plus the 8 px gap of the board.
const TOP: f32 = 226.0;

pub(super) fn card(p: &Palette) -> gpui::Div {
    div()
        .absolute()
        .left_0()
        .top(px(TOP))
        .w(px(380.))
        .rounded(px(12.))
        .bg(p.popover)
        .text_color(p.popover_foreground)
        .shadow(vec![
            gpui::BoxShadow {
                color: black().opacity(0.18),
                offset: gpui::point(px(0.), px(8.)),
                blur_radius: px(28.),
                spread_radius: px(0.),
            },
            gpui::BoxShadow {
                color: black().opacity(0.08),
                offset: gpui::point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(1.),
            },
        ])
}

fn info_card(
    view: &ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> Option<AnyElement> {
    let kind = view.info_open?;
    let info = page.row.button(kind)?.info.as_ref()?;
    let p = cx.quiver().palette;
    let tr = view.app.read(cx).translator;
    let rows: Vec<_> = info
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            div()
                .flex()
                .gap(px(10.))
                .child(
                    div()
                        .w(px(18.))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.muted_foreground)
                        .child((i + 1).to_string()),
                )
                .child(
                    div()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(s.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted_foreground)
                                .child(tr.t(plain_text_for(&s.step_type))),
                        ),
                )
        })
        .collect();
    let uses = (!info.uses_variables.is_empty()).then(|| {
        format!(
            "{} {}",
            tr.t(arrow::PREVIEW_USES),
            info.uses_variables.join(", ")
        )
    });
    let configure = tr.t(arrow::PREVIEW_CONFIGURE);
    Some(
        card(&p)
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr.t(info.title)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted_foreground)
                            .child(tr.t(arrow::PREVIEW_SUBTITLE)),
                    ),
            )
            .child(div().flex().flex_col().gap(px(10.)).children(rows))
            .children(uses.map(|text| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.))
                    .text_color(p.muted_foreground)
                    .child(text)
                    .child(
                        Button::new("arrow.info.configure")
                            .variant(ButtonVariant::Link)
                            .size(ButtonSize::Xs)
                            .label(configure.clone())
                            .on_click(cx.listener(|this, _, _, cx| this.open_settings(cx)))
                            .child(probe(
                                "arrow.info.configure",
                                Kind::Button,
                                configure,
                                true,
                                "",
                            )),
                    )
            }))
            .child(probe("arrow.info.popover", Kind::Other, "Info", true, ""))
            .into_any_element(),
    )
}

pub fn popovers(
    view: &mut ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> Vec<AnyElement> {
    let mut out = Vec::new();
    if view.steps_open {
        if let Some(control) = page.row.progress() {
            out.push(
                deferred(steps::steps_card(view, control, cx))
                    .with_priority(2)
                    .into_any_element(),
            );
        }
    } else if let Some(card) = info_card(view, page, cx) {
        out.push(deferred(card).with_priority(2).into_any_element());
    }
    out
}
