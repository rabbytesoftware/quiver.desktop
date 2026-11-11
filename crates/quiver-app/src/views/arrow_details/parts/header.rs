//! The header (`ArrowRunningAfter`): icon, name, namespace and version, the status row, the
//! description and the action row, with the banner art at the right when the arrow has one.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, FontWeight, IntoElement};
use quiver_state::arrow_details::channel::ChannelSelection;
use quiver_state::arrow_details::header::{SwitchAffordance, VersionControl};
use quiver_state::arrow_details::StatusGlyph as StateGlyph;
use quiver_state::i18n::en::arrow;
use quiver_state::namespace::{bare_namespace, with_selector};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon, ActiveQuiver, Badge, BadgeVariant, Glyph, GlyphKind, StatusGlyph, TrackedText,
};

use super::action_row::action_row;
use super::banner::{app_icon, banner_art, Art, BANNER_W, HEIGHT};
use super::icons::{pict, Pict};
use super::popovers::popovers;
use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

pub(super) fn glyph_kind(glyph: StateGlyph) -> Option<GlyphKind> {
    match glyph {
        StateGlyph::Busy => Some(GlyphKind::Busy),
        StateGlyph::Running => Some(GlyphKind::Running),
        StateGlyph::NeedsAttention => Some(GlyphKind::NeedsAttention),
        StateGlyph::UpdateAvailable => Some(GlyphKind::UpdateAvailable),
        StateGlyph::Ready | StateGlyph::Inactive => None,
    }
}

fn status_badge(
    page: &Page,
    label: String,
    cx: &Context<ArrowDetailsView>,
) -> impl IntoElement {
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

/// The version the picked channel resolves to: the version picked, else the channel's newest.
fn picked_version(selection: &ChannelSelection) -> Option<String> {
    selection
        .selected_version()
        .map(str::to_owned)
        .or_else(|| selection.selected_channel_entry().map(|c| c.latest.clone()))
}

/// The namespace line: what the page follows, `@selector` included, in and out of the library alike.
fn shown_namespace(
    view: &ArrowDetailsView,
    page: &Page,
) -> String {
    match (&page.header.version, view.selection.selector()) {
        (VersionControl::Picker { .. }, Some(selector)) => {
            with_selector(bare_namespace(&page.header.namespace), &selector)
        }
        _ => page.header.namespace.clone(),
    }
}

fn version_chip(
    view: &ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> Option<AnyElement> {
    let p = cx.quiver().palette;
    // One control in and out of the library: `namespace@selector · version ⌄`. In the library the chip opens
    // the switch flow; outside it, the same dialog chooses what Install and Add to Library will use.
    let (version, selector, switch) = match &page.header.version {
        VersionControl::Fixed {
            version,
            selector,
            switch,
        } => (version.clone(), selector.clone(), *switch),
        VersionControl::Picker { loading } => {
            let adding = view.invoke.pending
                == Some(quiver_state::arrow_details::ArrowActionKind::AddToLibrary);
            let version = if *loading {
                view.app.read(cx).t(quiver_state::i18n::en::arrow::LOADING)
            } else {
                picked_version(&view.selection).unwrap_or_default()
            };
            let switch = if *loading || adding {
                SwitchAffordance::Disabled
            } else {
                SwitchAffordance::Enabled
            };
            (
                version,
                view.selection.selector().unwrap_or_default(),
                switch,
            )
        }
        VersionControl::None => return None,
    };
    let channels = page.detail.channels.clone();
    let picker = switch != SwitchAffordance::Hidden;
    Some(
        div()
            .id("arrow.version.chip")
            .relative()
            .h(px(18.))
            .px(px(4.))
            .flex()
            .items_center()
            .gap(px(3.))
            .rounded(px(5.))
            .bg(p.muted)
            .font_family(FontFamily::Mono.family())
            .text_size(px(11.))
            .when(switch == SwitchAffordance::Disabled, |d| d.opacity(0.5))
            .when(switch == SwitchAffordance::Enabled, |d| {
                d.cursor_pointer().hover(|s| s.bg(p.accent)).on_click(
                    cx.listener(move |this, _, _, cx| this.open_switch(&selector, &channels, cx)),
                )
            })
            .child(version.clone())
            .children(picker.then(|| icon(Glyph::ChevronDown, 11., p.foreground)))
            .child(probe(
                "arrow.version",
                Kind::Button,
                version.clone(),
                true,
                match switch {
                    SwitchAffordance::Enabled => "switch=enabled",
                    SwitchAffordance::Disabled => "switch=disabled",
                    SwitchAffordance::Hidden => "switch=hidden",
                },
            ))
            .into_any_element(),
    )
}

pub fn header(
    view: &mut ArrowDetailsView,
    page: &Page,
    fit: super::Fit,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let model = view.app.read(cx);
    let tr = model.translator;
    let h = &page.header;
    let namespace = shown_namespace(view, page);
    let bare = quiver_state::namespace::bare_namespace(&h.namespace);
    let hue_value = if model.demo {
        Art::mock(bare)
    } else {
        Art::hashed(bare)
    };
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
        .flex()
        .flex_col()
        .justify_between()
        .pr(px(12.))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(16.))
                        .child(app_icon(hue_value, h.icon.as_deref()))
                        .child(
                            div()
                                .relative()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .relative()
                                        .w_full()
                                        .mt(px(1.))
                                        .h(px(22.))
                                        .child(
                                            TrackedText::new(
                                                h.name.clone(),
                                                17.,
                                                22.,
                                                -0.2,
                                                p.foreground,
                                            )
                                            .weight(FontWeight::SEMIBOLD),
                                        )
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
                                        .mt(px(1.))
                                        .w_full()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.))
                                        // A long namespace gives way (ellipsis); the version chip keeps its size.
                                        .child(
                                            div()
                                                .relative()
                                                .min_w_0()
                                                .truncate()
                                                .font_family(FontFamily::Mono.family())
                                                .text_size(px(11.))
                                                .line_height(px(14.))
                                                .text_color(p.muted_foreground)
                                                .child(namespace.clone())
                                                .child(probe(
                                                    "arrow.namespace",
                                                    Kind::Text,
                                                    namespace.clone(),
                                                    true,
                                                    "",
                                                )),
                                        )
                                        .children(version_chip(view, page, cx).map(|chip| {
                                            div()
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .gap(px(10.))
                                                .child(
                                                    div().text_color(p.muted_foreground).child("·"),
                                                )
                                                .child(chip)
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .mt(px(16.))
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
    let banner = h.banner.is_some();
    let text = div()
        .relative()
        .flex()
        // The banner's height, or more when the action row wraps.
        .min_h(px(HEIGHT))
        .child(left)
        .children(
            (banner && fit.banner_beside)
                .then(|| banner_art(hue_value, BANNER_W, h.banner.as_deref(), p.muted)),
        )
        .children(popovers(view, page, cx));
    if banner && !fit.banner_beside {
        // A narrow window: the banner above, the content's width, as the old hero stacked it below `lg`.
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(banner_art(
                hue_value,
                fit.content,
                h.banner.as_deref(),
                p.muted,
            ))
            .child(text)
            .into_any_element()
    } else {
        text.into_any_element()
    }
}
