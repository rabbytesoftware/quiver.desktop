//! The header (`ArrowRunningAfter`): icon, name, namespace and version, the status row, the
//! description and the action row, with the banner art at the right when the arrow has one.

use gpui::prelude::*;
use gpui::{
    div, hsla, linear_color_stop, linear_gradient, px, white, Context, FontWeight, IntoElement,
};
use quiver_state::arrow_details::header::VersionControl;
use quiver_state::arrow_details::StatusGlyph as StateGlyph;
use quiver_state::i18n::en::arrow;
use quiver_state::tile::hue_for;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon_with_stroke, ActiveQuiver, Badge, BadgeVariant, Glyph, GlyphKind, StatusGlyph,
};

use super::action_row::action_row;
use super::icons::{pict, Pict};
use super::popovers::popovers;
use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

const BANNER_W: f32 = 440.0;
const HEIGHT: f32 = 218.0;

fn hue(h: u16, s: f32, l: f32) -> gpui::Hsla {
    hsla(f32::from(h % 360) / 360.0, s, l, 1.0)
}

fn banner_art(h: u16) -> impl IntoElement {
    let w = BANNER_W;
    let period = w * 0.08;
    let stripes = (1..12).map(move |i| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(period * i as f32 - w * 0.002))
            .w(px(w * 0.002))
            .bg(white().opacity(0.07))
    });
    div()
        .relative()
        .flex_none()
        .w(px(w))
        .h(px(HEIGHT))
        .overflow_hidden()
        .rounded(px(12.))
        .bg(hue(h, 0.55, 0.24))
        .children(stripes)
        .child(
            div()
                .absolute()
                .rounded_full()
                .size(px(w * 0.5))
                .right(px(-w * 0.04))
                .top(px(-w * 0.2))
                .bg(hue(h, 0.55, 0.32)),
        )
        .child(
            div()
                .absolute()
                .left(px(149.6))
                .top(px(38.6))
                .child(icon_with_stroke(Glyph::Mark, w * 0.32, white(), 1.6)),
        )
        .child(probe("arrow.banner", Kind::Other, "banner", true, ""))
}

fn app_icon(h: u16) -> impl IntoElement {
    div()
        .flex_none()
        .size(px(44.))
        .rounded(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            180.,
            linear_color_stop(hue(h, 0.48, 0.50), 0.),
            linear_color_stop(hue(h, 0.52, 0.33), 1.),
        ))
        .child(icon_with_stroke(Glyph::Mark, 27., white(), 1.9))
}

fn glyph_kind(glyph: StateGlyph) -> Option<GlyphKind> {
    match glyph {
        StateGlyph::Busy => Some(GlyphKind::Busy),
        StateGlyph::Running => Some(GlyphKind::Running),
        StateGlyph::NeedsAttention => Some(GlyphKind::NeedsAttention),
        StateGlyph::UpdateAvailable => Some(GlyphKind::UpdateAvailable),
        StateGlyph::Ready | StateGlyph::Inactive => None,
    }
}

fn status_badge(page: &Page, label: String, cx: &Context<ArrowDetailsView>) -> impl IntoElement {
    let p = cx.quiver().palette;
    let glyph = page.header.status.glyph;
    let (variant, fg) = match glyph {
        StateGlyph::Running => (BadgeVariant::Default, p.primary_foreground),
        StateGlyph::Busy | StateGlyph::Ready => (BadgeVariant::Secondary, p.secondary_foreground),
        StateGlyph::NeedsAttention => (BadgeVariant::Error, p.destructive),
        StateGlyph::UpdateAvailable | StateGlyph::Inactive => (BadgeVariant::Outline, p.foreground),
    };
    Badge::new()
        .variant(variant)
        .children(glyph_kind(glyph).map(|kind| {
            StatusGlyph::new("arrow.status.glyph", kind)
                .size(12.)
                .color(fg)
        }))
        .child(label.clone())
        .child(probe("arrow.status", Kind::Text, label, true, ""))
}

fn version_chip(page: &Page, cx: &Context<ArrowDetailsView>) -> Option<impl IntoElement> {
    let p = cx.quiver().palette;
    let VersionControl::Fixed { version, .. } = &page.header.version else {
        return None;
    };
    Some(
        div()
            .relative()
            .h(px(18.))
            .px(px(4.))
            .flex()
            .items_center()
            .rounded(px(5.))
            .bg(p.muted)
            .font_family(FontFamily::Mono.family())
            .text_size(px(11.))
            .child(version.clone())
            .child(probe(
                "arrow.version",
                Kind::Text,
                version.clone(),
                true,
                "",
            )),
    )
}

pub fn header(
    view: &mut ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let model = view.app.read(cx);
    let tr = model.translator;
    let h = &page.header;
    let hue_value = hue_for(quiver_state::namespace::bare_namespace(&h.namespace));
    let status_label = tr.t(h.status.label);
    let platform = h.platform_badge.as_ref().map(|b| b.text(&tr));
    let problem = h.problem.as_ref().map(|_| tr.t(arrow::PROBLEM_LABEL));
    let inferred = h.inferred.then(|| tr.t(arrow::INFERRED_BADGE));
    let up_to_date = view.invoke.up_to_date.then(|| tr.t(arrow::UPDATE_CURRENT));
    let tags: Vec<_> = h
        .tags
        .iter()
        .map(|t| {
            Badge::new()
                .variant(BadgeVariant::Outline)
                .child(t.clone())
                .child(probe(
                    format!("arrow.tag.{t}"),
                    Kind::Text,
                    t.clone(),
                    true,
                    "",
                ))
        })
        .collect();
    let problem_badge = problem.map(|label| {
        Badge::new()
            .id("arrow.problem")
            .variant(BadgeVariant::Error)
            .child(pict(Pict::Alert, 11., p.destructive))
            .child(label.clone())
            .on_click(cx.listener(|this, _, _, cx| {
                this.problem_open = true;
                cx.notify();
            }))
            .child(probe("arrow.problem", Kind::Button, label, true, ""))
    });
    let left = div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .justify_between()
        .pr(px(12.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div().flex().gap(px(16.)).child(app_icon(hue_value)).child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .relative()
                                    .text_size(px(17.))
                                    .line_height(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(h.name.clone())
                                    .child(probe(
                                        "arrow.title",
                                        Kind::Text,
                                        h.name.clone(),
                                        true,
                                        "",
                                    )),
                            )
                            .child(
                                div()
                                    .mt(px(5.))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .relative()
                                            .font_family(FontFamily::Mono.family())
                                            .text_size(px(11.))
                                            .line_height(px(14.))
                                            .text_color(p.muted_foreground)
                                            .child(h.namespace.clone())
                                            .child(probe(
                                                "arrow.namespace",
                                                Kind::Text,
                                                h.namespace.clone(),
                                                true,
                                                "",
                                            )),
                                    )
                                    .children(version_chip(page, cx).map(|chip| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(div().text_color(p.muted_foreground).child("·"))
                                            .child(chip)
                                    })),
                            ),
                    ),
                )
                .child(
                    div()
                        .mt(px(17.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(8.))
                        .child(status_badge(page, status_label, cx))
                        .children(problem_badge)
                        .children(platform.map(|text| {
                            Badge::new()
                                .variant(BadgeVariant::Error)
                                .child(pict(Pict::Alert, 11., p.destructive))
                                .child(text.clone())
                                .child(probe("arrow.platform-badge", Kind::Text, text, true, ""))
                        }))
                        .children(
                            inferred
                                .map(|text| Badge::new().variant(BadgeVariant::Info).child(text)),
                        )
                        .children(tags),
                )
                .child(
                    div()
                        .relative()
                        .mt(px(14.))
                        .max_w(px(390.))
                        .text_size(px(14.5))
                        .line_height(px(21.))
                        .text_color(p.foreground.opacity(0.78))
                        .child(h.description.clone())
                        .child(probe(
                            "arrow.description",
                            Kind::Text,
                            h.description.clone(),
                            true,
                            "",
                        )),
                )
                .children(up_to_date.map(|text| {
                    div()
                        .relative()
                        .mt(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.muted_foreground)
                        .child(text.clone())
                        .child(probe("arrow.up-to-date", Kind::Text, text, true, ""))
                })),
        )
        .child(action_row(view, page, cx));
    div()
        .relative()
        .flex()
        .h(px(HEIGHT))
        .child(left)
        .children(h.banner.is_some().then(|| banner_art(hue_value)))
        .children(popovers(view, page, cx))
}
