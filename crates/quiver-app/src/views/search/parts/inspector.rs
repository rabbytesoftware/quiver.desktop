//! "This search" (`SearchInspector`): the right-edge sheet behind the source pill. The pass (query, job,
//! found / verified / skipped), each host asked and what it answered, and the running `search` settings.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, IntoElement};
use quiver_state::core_api::CoreApiExt;
use quiver_state::dto::config::to_config_display_rows;
use quiver_state::i18n::en::search as keys;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Button, ButtonVariant, Sheet, SheetDescription, SheetFooter, SheetHeader,
    SheetPanel, SheetTitle, TrackedText,
};

use crate::bridge::spawn_action;
use crate::views::search::SearchView;

impl SearchView {
    pub(in crate::views::search) fn open_inspector(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.inspector_open = true;
        self.running_settings = None;
        let model = self.app.read(cx);
        if !model.demo {
            let api = model.services.api.clone();
            let handle = model.handle().clone();
            spawn_action(
                cx,
                &handle,
                async move { api.get_json("/v0/config").await.ok() },
                |this, config, cx| {
                    this.running_settings = config
                        .as_ref()
                        .and_then(|c| c.get("running"))
                        .and_then(|r| r.get("search"))
                        .and_then(|s| s.as_object())
                        .map(|section| {
                            to_config_display_rows(section)
                                .into_iter()
                                .map(|r| (r.key, r.value))
                                .collect()
                        });
                    cx.notify();
                },
            );
        }
        cx.notify();
    }

    fn close_inspector(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.inspector_open = false;
        cx.notify();
    }
}

fn field(
    label: String,
    value: String,
    mono: bool,
    p: &quiver_ui::Palette,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .child(div().text_color(p.muted_foreground).child(label))
        .child(
            div()
                .truncate()
                .when(mono, |d| {
                    d.font_family(quiver_theme::FontFamily::Mono.family())
                })
                .child(value),
        )
}

fn caption(
    text: String,
    p: &quiver_ui::Palette,
) -> impl IntoElement {
    // text-xs medium uppercase tracking-wide (0.025em).
    TrackedText::new(
        text.to_uppercase(),
        12.,
        12. * 1.618,
        0.025 * 12.,
        p.muted_foreground,
    )
    .weight(gpui::FontWeight::MEDIUM)
}

pub fn inspector(
    view: &SearchView,
    cx: &mut Context<SearchView>,
) -> Option<AnyElement> {
    if !view.inspector_open {
        return None;
    }
    let p = cx.quiver().palette;
    let tr = view.app.read(cx).translator;
    let state = view.session.store.snapshot();
    let summary = state.summary.clone();
    let (found, verified, skipped) = summary
        .as_ref()
        .map_or((0, 0, 0), |s| (s.found, s.verified, s.skipped));
    let mut pass = div().flex().flex_col().gap(px(4.)).child(field(
        tr.t(keys::INSPECTOR_QUERY),
        view.query.clone(),
        false,
        &p,
    ));
    if let Some(job) = &state.job {
        pass = pass.child(field(tr.t(keys::INSPECTOR_JOB), job.id.clone(), true, &p));
    }
    pass = pass.child(field(
        tr.t(keys::INSPECTOR_COUNTS),
        format!("{found} / {verified} / {skipped}"),
        true,
        &p,
    ));
    if let Some(job) = &state.job {
        pass = pass.child(field(
            tr.t(keys::INSPECTOR_EXPIRES),
            job.expires_at.clone(),
            false,
            &p,
        ));
    }
    let hosts = match &summary {
        Some(s) => div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .children(s.providers.iter().map(|provider| {
                let detail = if provider.ok {
                    tr.t_count(keys::RESULTS_COUNT, provider.returned as usize)
                } else {
                    let retry = provider.retry_after.map(|secs| {
                        format!(
                            " · {}",
                            tr.t_with(
                                keys::RESULTS_RETRY,
                                &quiver_state::i18n::Params::new().with("seconds", secs)
                            )
                        )
                    });
                    format!(
                        "{}{}",
                        provider.reason.clone().unwrap_or_default(),
                        retry.unwrap_or_default()
                    )
                };
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .size(px(8.))
                                    .flex_none()
                                    .rounded_full()
                                    .when(provider.ok, |d| d.bg(p.foreground))
                                    .when(!provider.ok, |d| {
                                        d.border_1().border_color(p.foreground.opacity(0.4))
                                    }),
                            )
                            .child(provider.host.clone()),
                    )
                    .child(div().text_color(p.muted_foreground).child(detail))
            }))
            .into_any_element(),
        None => div()
            .text_color(p.muted_foreground)
            .child(tr.t(keys::INSPECTOR_RUNNING))
            .into_any_element(),
    };
    let asked = summary
        .as_ref()
        .map(|s| tr.t_count(keys::INSPECTOR_ASKED, s.providers.len()));
    let settings = view.running_settings.clone().map(|rows| {
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(rows.into_iter().map(|(k, v)| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(div().text_color(p.muted_foreground).child(k))
                    .child(div().child(v))
            }))
    });
    let section = |title: String, aside: Option<String>, body: AnyElement| {
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(caption(title, &p))
                    .children(aside.map(|a| {
                        div()
                            .text_size(px(12.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(p.muted_foreground)
                            .child(a)
                    })),
            )
            .child(body)
    };
    Some(
        Sheet::new("search-inspector")
            .open(true)
            .on_dismiss({
                let this = cx.entity();
                move |_, cx| this.update(cx, |t, cx| t.close_inspector(cx))
            })
            .child(
                SheetHeader::new()
                    .child(SheetTitle::new().child(tr.t(keys::INSPECTOR_TITLE)))
                    .child(SheetDescription::new().child(tr.t(keys::INSPECTOR_DESCRIPTION))),
            )
            .child(
                SheetPanel::new().child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(20.))
                        .text_size(px(14.))
                        .child(section(
                            tr.t(keys::INSPECTOR_PASS),
                            None,
                            pass.into_any_element(),
                        ))
                        .child(section(tr.t(keys::INSPECTOR_HOSTS), asked, hosts))
                        .child(section(
                            tr.t(keys::INSPECTOR_SETTINGS),
                            Some("running.search".into()),
                            settings.map_or_else(
                                || div().into_any_element(),
                                IntoElement::into_any_element,
                            ),
                        )),
                ),
            )
            .child(
                SheetFooter::new().child(
                    Button::new("search.inspector.close")
                        .variant(ButtonVariant::Outline)
                        .label(tr.t(keys::INSPECTOR_CLOSE))
                        .on_click(cx.listener(|this, _, _, cx| this.close_inspector(cx))),
                ),
            )
            .child(probe(
                "search.inspector",
                Kind::Dialog,
                "This search",
                true,
                "",
            ))
            .into_any_element(),
    )
}
