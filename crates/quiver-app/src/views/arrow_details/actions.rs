//! Running an action of the row: the platform gate, Quiver's own release variables, the call to
//! Core, and what the page does with the answer. The decisions are `quiver_state::arrow_details::
//! invoke`; this is the glue that performs the calls and applies the outcome.

use std::sync::Arc;
use std::time::Duration;

use gpui::{AppContext as _, Context};
use quiver_state::arrow_details::invoke::{
    needs_platform_check, needs_release, ActionFailure, UpdateOutcome as Outcome, UP_TO_DATE_MS,
};
use quiver_state::arrow_details::release_variables::{
    install_tag_for, update_target, ReleaseFailure,
};
use quiver_state::arrow_details::variable_form::invalid_fields;
use quiver_state::arrow_details::{ArrowActionKind, RunState};
use quiver_state::domain::is_platform_supported;
use quiver_state::queries::keys::arrow_detail_prefix;
use quiver_state::queries::mutations::{Mutations, UpdateOutcome, Variables};

use super::ArrowDetailsView;
use crate::bridge::spawn_action;
use crate::model::open_modal;
use crate::services::{resolve_release_variables, ReleaseSource};

/// What the call to Core needs, owned so it can run on the runtime.
struct Job {
    kind: ArrowActionKind,
    mutations: Mutations,
    release: Arc<dyn ReleaseSource>,
    /// The identity the call is made at.
    namespace: String,
    user_installed: bool,
    variables: Variables,
    resolved_ref: String,
    available_ref: Option<String>,
}

enum Done {
    Ok(Option<Outcome>),
    Release(ReleaseFailure),
    Failed(ActionFailure),
}

fn failure(err: &quiver_state::Error) -> ActionFailure {
    ActionFailure {
        status: match err {
            quiver_state::Error::Api(api) => Some(api.status),
            _ => None,
        },
        message: err.to_string(),
    }
}

async fn perform(job: Job) -> Done {
    use ArrowActionKind as K;
    let Job {
        kind,
        mutations,
        release,
        namespace,
        user_installed,
        mut variables,
        resolved_ref,
        available_ref,
    } = job;
    let release_vars = if needs_release(kind, &namespace) {
        let tag = if kind == K::Update {
            match update_target(&namespace, available_ref.as_deref()) {
                Ok(tag) => Some(tag),
                Err(f) => return Done::Release(f),
            }
        } else {
            install_tag_for(&resolved_ref).map(str::to_owned)
        };
        match resolve_release_variables(release.as_ref(), &namespace, tag.as_deref()).await {
            Ok(vars) => vars,
            Err(f) => return Done::Release(f),
        }
    } else {
        Vec::new()
    };
    // Install and Start on an arrow outside the library add it first.
    if matches!(kind, K::Install | K::Reinstall | K::Execute) && !user_installed {
        if let Err(e) = mutations.register_arrow(&namespace).await {
            return Done::Failed(failure(&e));
        }
    }
    let result = match kind {
        K::AddToLibrary => mutations.register_arrow(&namespace).await,
        K::RemoveFromLibrary => mutations.remove_arrow(&namespace).await,
        K::Install | K::Reinstall => {
            variables.extend(release_vars);
            mutations.install(&namespace, &variables).await
        }
        K::Uninstall => mutations.uninstall(&namespace, &Variables::new()).await,
        K::Update => {
            let vars: Variables = release_vars.into_iter().collect();
            return match mutations.update(&namespace, &vars).await {
                Ok(UpdateOutcome::Started) => Done::Ok(Some(Outcome::Started)),
                Ok(UpdateOutcome::Current) => Done::Ok(Some(Outcome::Current)),
                Err(e) => Done::Failed(failure(&e)),
            };
        }
        K::Execute => mutations.execute_arrow(&namespace, &variables).await,
        K::Stop | K::Restart => mutations.stop(&namespace, &Variables::new()).await,
    };
    match result {
        Ok(()) => Done::Ok(None),
        Err(e) => Done::Failed(failure(&e)),
    }
}

impl ArrowDetailsView {
    /// Runs `kind`. `skip_warning` is the "Add anyway" answer to the platform notice.
    pub(super) fn invoke_action(
        &mut self,
        kind: ArrowActionKind,
        skip_warning: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = self.merged(cx) else {
            return;
        };
        self.invoke.begin(kind);
        self.info_open = None;
        let platform = quiver_platform::platform_key();
        if needs_platform_check(kind, detail.user_installed, skip_warning)
            && !is_platform_supported(&detail.targets, &platform)
        {
            self.invoke.platform_refused(&platform);
            cx.notify();
            return;
        }
        let carries_values = matches!(
            kind,
            ArrowActionKind::Install
                | ArrowActionKind::Reinstall
                | ArrowActionKind::Execute
                | ArrowActionKind::Restart
        );
        let variables = self.values.read(cx).values.clone();
        let asked = super::settings_modal::visible_variables(&detail);
        if carries_values && !invalid_fields(&asked, &variables).is_empty() {
            // A value the daemon would refuse: show the form with its errors instead.
            self.invoke.pending = None;
            self.open_settings(cx);
            return;
        }
        let model = self.app.read(cx);
        let job = Job {
            kind,
            mutations: Mutations::new(model.services.api.clone(), model.services.queries.clone()),
            release: model.services.release.clone(),
            namespace: detail.namespace.clone(),
            user_installed: detail.user_installed,
            variables,
            resolved_ref: detail.resolved_ref.clone(),
            available_ref: detail.available.as_ref().map(|a| a.reference.clone()),
        };
        let handle = model.handle().clone();
        cx.notify();
        spawn_action(cx, &handle, perform(job), move |this, done, cx| {
            this.settle(kind, done, cx);
        });
    }

    fn settle(
        &mut self,
        kind: ArrowActionKind,
        done: Done,
        cx: &mut Context<Self>,
    ) {
        let services = self.app.read(cx).services.clone();
        match done {
            Done::Ok(update) => {
                self.invoke.succeeded(kind, update);
                // The one-time read does not follow the stream: library membership and the version
                // check are re-read, and the sidebar's catalog with them.
                if !matches!(kind, ArrowActionKind::Stop | ArrowActionKind::Execute) {
                    services.queries.invalidate(&arrow_detail_prefix());
                    services.arrows.refresh_catalog();
                }
                if update == Some(Outcome::Current) {
                    self.expire_up_to_date_later(cx);
                }
            }
            Done::Release(failure) => {
                let stale = self.invoke.release_failed(&failure, failure.kind.wire());
                if stale {
                    services.queries.invalidate(&arrow_detail_prefix());
                }
            }
            Done::Failed(failure) => self.invoke.failed(kind, &failure),
        }
        cx.notify();
    }

    fn expire_up_to_date_later(
        &self,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(UP_TO_DATE_MS))
                .await;
            view.update(cx, |this, cx| {
                this.invoke.expire_up_to_date();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Restart's second leg: Core only accepts `execute` once the row has reached `ready`.
    pub(super) fn watch_live(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let mut revisions = self.app.read(cx).services.arrows.store().subscribe();
        cx.spawn(async move |view, cx| {
            while revisions.changed().await.is_ok() {
                let alive = view.update(cx, |this, cx| this.live_changed(cx));
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn live_changed(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = self.merged(cx) else {
            return;
        };
        if !self
            .invoke
            .restart_ready_for_execute(RunState::from(detail.state))
        {
            return;
        }
        let model = self.app.read(cx);
        let mutations = Mutations::new(model.services.api.clone(), model.services.queries.clone());
        let handle = model.handle().clone();
        let (namespace, variables) = (detail.namespace, self.values.read(cx).values.clone());
        spawn_action(
            cx,
            &handle,
            async move { mutations.execute_arrow(&namespace, &variables).await },
            |this, result, cx| {
                let failed = result.err().map(|e| failure(&e));
                this.invoke.restart_finished(failed.as_ref());
                cx.notify();
            },
        );
    }

    pub(super) fn open_settings(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = self.merged(cx) else {
            return;
        };
        let (app, values) = (self.app.clone(), self.values.clone());
        let variables = super::settings_modal::visible_variables(&detail);
        let view = cx.new(|cx| {
            super::settings_modal::SettingsModal::new(app, detail.name, variables, values, cx)
        });
        open_modal(cx, "arrow-settings", view.into());
    }

    pub(super) fn open_methods(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = self.merged(cx) else {
            return;
        };
        let facts = quiver_state::arrow_details::facts_from_detail(
            &detail,
            &quiver_platform::platform_key(),
        );
        let model = quiver_state::arrow_details::methods::build_methods(&facts);
        let app = self.app.clone();
        let view = cx.new(|cx| super::methods_modal::MethodsModal::new(app, model, cx));
        open_modal(cx, "arrow-methods", view.into());
    }
}
