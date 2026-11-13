//! Moving what a library entry follows (`SwitchSelectorDialog`, the old Hero's "Switch…"): the dialog
//! that picks a channel and version, the sequence that registers the new identity and then replaces the
//! old one, and the message a failure after the registration leaves behind.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{Context, Entity, Render, Window};
use quiver_state::arrow_details::channel::{
    ChannelFacts, ChannelKind as FactsKind, ChannelSelection,
};
use quiver_state::arrow_details::switch::{
    SwitchFailure, SwitchPlan, SwitchStep, UninstallProgress, UninstallWatch,
};
use quiver_state::arrow_details::RunState;
use quiver_state::domain::{ArrowChannel, ChannelKind};
use quiver_state::i18n::en::arrow;
use quiver_state::i18n::Params;
use quiver_state::namespace::with_selector;
use quiver_state::queries::keys::arrow_detail_prefix;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};

use super::ArrowDetailsView;
use crate::bridge::spawn_action;
use crate::model::{close_modal, navigate, open_modal, AppModel, Route};

const FAILED_ID: &str = "arrow-switch-failed";
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(60);

pub(super) fn channel_facts(channels: &[ArrowChannel]) -> Vec<ChannelFacts> {
    channels
        .iter()
        .map(|c| ChannelFacts {
            name: c.name.clone(),
            kind: match c.kind {
                ChannelKind::Ordered => FactsKind::Ordered,
                ChannelKind::Pointer => FactsKind::Pointer,
            },
            latest: c.latest.clone(),
            members: c.members.clone().unwrap_or_default(),
        })
        .collect()
}

async fn uninstall_and_wait(
    mutations: &Mutations,
    arrows: &quiver_state::store::ArrowStore,
    namespace: &str,
) -> Result<(), String> {
    let mut revisions = arrows.store().subscribe();
    mutations
        .uninstall(namespace, &Variables::new())
        .await
        .map_err(|e| e.to_string())?;
    let mut watch = UninstallWatch::new(namespace);
    let wait = async {
        loop {
            let state = arrows.entry(namespace).map(|e| RunState::from(e.state));
            match watch.inspect(state) {
                UninstallProgress::Done => return Ok(()),
                UninstallProgress::Failed(reason) => return Err(reason),
                UninstallProgress::Waiting => {}
            }
            if revisions.changed().await.is_err() {
                return Err(format!("{namespace} was not uninstalled"));
            }
        }
    };
    tokio::time::timeout(UNINSTALL_TIMEOUT, wait)
        .await
        .unwrap_or_else(|_| Err(format!("{namespace} was not uninstalled within 60s")))
}

/// Everything after the registration: uninstall the old row, forget it, install the new one.
async fn replace(
    mutations: Mutations,
    arrows: quiver_state::store::ArrowStore,
    plan: SwitchPlan,
    variables: Variables,
) -> Result<(), SwitchFailure> {
    let failed = |step: SwitchStep, reason: String| SwitchFailure {
        step,
        from: plan.from.clone(),
        to: plan.to.clone(),
        reason,
    };
    for step in plan.steps() {
        match step {
            SwitchStep::Register => {}
            SwitchStep::Uninstall => uninstall_and_wait(&mutations, &arrows, &plan.from)
                .await
                .map_err(|r| failed(step, r))?,
            SwitchStep::Remove => mutations
                .remove_arrow(&plan.from)
                .await
                .map_err(|e| failed(step, e.to_string()))?,
            SwitchStep::Install => mutations
                .install(&plan.to, &variables)
                .await
                .map_err(|e| failed(step, e.to_string()))?,
        }
    }
    Ok(())
}

pub struct SwitchFailedModal {
    app: Entity<AppModel>,
    failure: SwitchFailure,
}

impl Render for SwitchFailedModal {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tr = self.app.read(cx).translator;
        let f = &self.failure;
        let params = Params::new()
            .with("from", f.from.clone())
            .with("to", f.to.clone());
        let key = match f.step {
            SwitchStep::Register | SwitchStep::Install => arrow::SELECTOR_FAILED_INSTALL,
            SwitchStep::Uninstall => arrow::SELECTOR_FAILED_UNINSTALL,
            SwitchStep::Remove => arrow::SELECTOR_FAILED_REMOVE,
        };
        let body = format!("{}\n\n{}", tr.t_with(key, &params), f.reason);
        let from = f.from.clone();
        let mut buttons = Vec::new();
        if f.step != SwitchStep::Install {
            buttons.push(
                Button::new("arrow.switch-failed.dismiss")
                    .variant(ButtonVariant::Outline)
                    .label(tr.t(arrow::SELECTOR_DISMISS))
                    .on_click(|_, _, cx| close_modal(cx, FAILED_ID)),
            );
            buttons.push(
                Button::new("arrow.switch-failed.open-old")
                    .label(tr.t(arrow::SELECTOR_OPEN_OLD))
                    .on_click(move |_, _, cx| {
                        close_modal(cx, FAILED_ID);
                        navigate(
                            cx,
                            Route::Arrow {
                                namespace: from.clone(),
                            },
                        );
                    }),
            );
        } else {
            buttons.push(
                Button::new("arrow.switch-failed.dismiss")
                    .label(tr.t(arrow::UPDATE_DISMISS))
                    .on_click(|_, _, cx| close_modal(cx, FAILED_ID)),
            );
        }
        Dialog::new(FAILED_ID)
            .open(true)
            .on_dismiss(|_, cx| close_modal(cx, FAILED_ID))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(tr.t(arrow::SELECTOR_SWITCH_FAILED)))
                    .child(DialogDescription::new().child(body)),
            )
            .child(DialogFooter::new().children(buttons))
            .child(probe(FAILED_ID, Kind::Dialog, FAILED_ID, true, ""))
    }
}

impl ArrowDetailsView {
    /// Seeds the picker from the channels the repository publishes; again whenever they arrive or change.
    pub(super) fn sync_selection(
        &mut self,
        selector: &str,
        channels: &[ArrowChannel],
    ) {
        let seen = (selector.to_owned(), channels.len());
        if self.selection_seen.as_ref() == Some(&seen) {
            return;
        }
        self.selection_seen = Some(seen);
        self.selection = ChannelSelection::seed(selector, channel_facts(channels));
    }

    pub(super) fn open_switch(
        &mut self,
        selector: &str,
        channels: &[ArrowChannel],
        cx: &mut Context<Self>,
    ) {
        self.switch_selection = Some(ChannelSelection::seed(selector, channel_facts(channels)));
        self.register_error = None;
        cx.notify();
    }

    /// Not in the library: the dialog's pick becomes what Install / Add to Library use. Nothing is sent to Core.
    pub(super) fn choose_version(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(picked) = self.switch_selection.take() {
            self.selection = picked;
        }
        self.register_error = None;
        cx.notify();
    }

    pub(super) fn close_switch(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.switch_selection = None;
        self.register_error = None;
        cx.notify();
    }

    /// Registers the picked identity first (a failure leaves everything as it was and the dialog open on
    /// it); once it exists the page moves to it and the old row is replaced in the background.
    pub(super) fn confirm_switch(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = self.merged(cx) else {
            return;
        };
        let Some(selector) = self
            .switch_selection
            .as_ref()
            .and_then(ChannelSelection::selector)
        else {
            return;
        };
        let from = detail.namespace.clone();
        let to = with_selector(&from, &selector);
        if to == from || self.switch_pending {
            return;
        }
        self.switch_pending = true;
        self.register_error = None;
        let plan = SwitchPlan {
            from: from.clone(),
            to: to.clone(),
            installed: detail.state != quiver_state::domain::ArrowState::Absent,
        };
        let model = self.app.read(cx);
        let mutations = Mutations::new(model.services.api.clone(), model.services.queries.clone());
        let arrows = model.services.arrows.clone();
        let queries = model.services.queries.clone();
        let handle = model.handle().clone();
        let variables = self.values.read(cx).values.clone();
        let registering = mutations.clone();
        let target = to.clone();
        cx.notify();
        spawn_action(
            cx,
            &handle.clone(),
            async move {
                registering
                    .register_arrow(&target)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |this, result, cx| {
                this.switch_pending = false;
                match result {
                    Err(reason) => this.register_error = Some(reason),
                    Ok(()) => {
                        this.switch_selection = None;
                        arrows.refresh_catalog();
                        let app = this.app.clone();
                        app.update(cx, |m, cx| {
                            m.navigate(
                                Route::Arrow {
                                    namespace: plan.to.clone(),
                                },
                                cx,
                            )
                        });
                        let join =
                            handle.spawn(replace(mutations, arrows.clone(), plan, variables));
                        cx.spawn(async move |_, cx| {
                            let outcome = join.await;
                            queries.invalidate(&arrow_detail_prefix());
                            arrows.refresh_catalog();
                            if let Ok(Err(failure)) = outcome {
                                cx.update(|cx| {
                                    let view = cx.new(|_| SwitchFailedModal {
                                        app: app.clone(),
                                        failure,
                                    });
                                    open_modal(cx, FAILED_ID, view.into());
                                })
                                .ok();
                            }
                        })
                        .detach();
                    }
                }
                cx.notify();
            },
        );
    }
}
