//! The two popovers under the action row: the live Steps checklist of the running action
//! (`ArrowActivityAfter`) and the ⓘ preview of what an action would run.

use gpui::prelude::*;
use gpui::{black, deferred, div, px, AnyElement, Context, FontWeight, IntoElement, SharedString};
use quiver_state::arrow_details::progress::plain_text_for;
use quiver_state::arrow_details::{ProgressControl, ProgressStep, StepState};
use quiver_state::i18n::en::arrow;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon, ActiveQuiver, Button, ButtonSize, ButtonVariant, FlickerSpinner, Glyph, Palette,
};

use super::icons::{pict, Pict};
use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

/// Below the action row: it ends at 178 + 40, plus the 8 px gap of the board.
const TOP: f32 = 226.0;

fn card(p: &Palette) -> gpui::Div {
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

fn marker(state: StepState, p: &Palette) -> AnyElement {
    match state {
        StepState::Done => div()
            .flex_none()
            .size(px(18.))
            .rounded_full()
            .bg(p.primary)
            .flex()
            .items_center()
            .justify_center()
            .child(icon(Glyph::Check, 11., p.primary_foreground))
            .into_any_element(),
        StepState::Running => div()
            .flex_none()
            .size(px(18.))
            .flex()
            .items_center()
            .justify_center()
            .child(
                FlickerSpinner::new("arrow.steps.spinner")
                    .size(16.)
                    .color(p.foreground),
            )
            .into_any_element(),
        StepState::Pending => div()
            .flex_none()
            .size(px(18.))
            .rounded_full()
            .border_2()
            .border_color(p.border)
            .into_any_element(),
        StepState::Failed => div()
            .flex_none()
            .size(px(18.))
            .rounded_full()
            .bg(p.destructive)
            .flex()
            .items_center()
            .justify_center()
            .child(icon(Glyph::Close, 11., gpui::white()))
            .into_any_element(),
    }
}

fn step_row(
    index: usize,
    step: &ProgressStep,
    p: &Palette,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let (weight, color) = match step.state {
        StepState::Running => (FontWeight::SEMIBOLD, p.foreground),
        StepState::Pending => (FontWeight::MEDIUM, p.muted_foreground),
        StepState::Done | StepState::Failed => (FontWeight::MEDIUM, p.foreground),
    };
    let id = format!("arrow.steps.{}", index + 1);
    div()
        .relative()
        .h(px(36.))
        .px(px(10.))
        .rounded(px(8.))
        .flex()
        .items_center()
        .gap(px(12.))
        .when(step.state == StepState::Running, |d| d.bg(p.muted))
        .child(marker(step.state, p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(14.))
                .font_weight(weight)
                .text_color(color)
                .child(step.title.clone()),
        )
        .child(
            Button::new(SharedString::from(format!("{id}.inspect")))
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSm)
                .child(pict(Pict::Info, 16., p.muted_foreground))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.inspected_step = Some(index);
                    cx.notify();
                })),
        )
        .child(probe(
            id,
            Kind::Other,
            step.title.clone(),
            true,
            format!("{:?}", step.state).to_lowercase(),
        ))
}

fn steps_card(
    view: &ArrowDetailsView,
    control: &ProgressControl,
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let tr = view.app.read(cx).translator;
    let current = view
        .inspected_step
        .filter(|i| *i < control.steps.len())
        .or_else(|| {
            control
                .steps
                .iter()
                .position(|s| s.state == StepState::Running)
        })
        .unwrap_or(0);
    let rows: Vec<_> = control
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| step_row(i, s, &p, cx))
        .collect();
    let detail = control.steps.get(current).map(|step| {
        let text = step
            .error
            .clone()
            .unwrap_or_else(|| tr.t(plain_text_for(&step.step_type)));
        div()
            .border_t_1()
            .border_color(p.border)
            .mx(px(16.))
            .pt(px(12.))
            .pb(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.foreground.opacity(0.78))
                    .child(text),
            )
            .when(!step.code.is_empty(), |d| {
                d.child(
                    div()
                        .p(px(10.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.border)
                        .bg(p.muted)
                        .font_family(FontFamily::Mono.family())
                        .text_size(px(11.5))
                        .line_height(px(17.))
                        .child(step.code.clone()),
                )
            })
    });
    card(&p)
        .child(div().flex().flex_col().gap(px(2.)).p(px(6.)).children(rows))
        .children(detail)
        .child(probe("arrow.steps.popover", Kind::Other, "Steps", true, ""))
        .into_any_element()
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
                deferred(steps_card(view, control, cx))
                    .with_priority(2)
                    .into_any_element(),
            );
        }
    } else if let Some(card) = info_card(view, page, cx) {
        out.push(deferred(card).with_priority(2).into_any_element());
    }
    out
}
