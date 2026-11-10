//! The "installed / available / check / update" rows of quiver.core and of this app. Both update like
//! any other arrow (an ARROW.md lifecycle run by Core), so the same card drives them; the desktop's
//! carries the `QUIVER_RELEASE_*` variables of its own release asset.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context};
use quiver_state::arrow_details::invoke::ActionFailure;
use quiver_state::arrow_details::RunState;
use quiver_state::domain::QUIVER_CORE_NAMESPACE;
use quiver_state::i18n::en::{arrow, settings as keys};
use quiver_state::namespace::bare_namespace;
use quiver_state::queries::mutations::{Mutations, UpdateOutcome, Variables};
use quiver_state::settings::core_update::{build_core_update, CoreUpdateInput, CoreUpdateNote};
use quiver_ui::ActiveQuiver;

use super::engine_actions::to_failure;
use super::rows::{probed, row, small_button};
use crate::bridge::spawn_action;
use crate::services::resolve_release_variables;
use crate::views::settings::SettingsView;

#[derive(Default)]
pub struct UpdateCard {
    available: Option<String>,
    update_pending: bool,
    recheck_pending: bool,
    update_error: Option<ActionFailure>,
    recheck_error: Option<ActionFailure>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Core,
}

impl Which {
    fn namespace(self) -> &'static str {
        match self {
            Self::Core => QUIVER_CORE_NAMESPACE,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Core => "settings.core",
        }
    }
}

impl SettingsView {
    fn card(
        &mut self,
        which: Which,
    ) -> &mut UpdateCard {
        match which {
            Which::Core => &mut self.core,
        }
    }

    pub(in crate::views::settings) fn recheck(
        &mut self,
        which: Which,
        cx: &mut Context<Self>,
    ) {
        let card = self.card(which);
        card.recheck_pending = true;
        card.update_error = None;
        card.recheck_error = None;
        let (mutations, handle) = self.mutations(cx);
        spawn_action(
            cx,
            &handle,
            async move { mutations.recheck_arrow(which.namespace()).await },
            move |this: &mut Self, result, cx| {
                let card = this.card(which);
                card.recheck_pending = false;
                match result {
                    Ok(found) => {
                        card.available = found
                            .available
                            .map(|a| a.reference)
                            .filter(|r| !r.is_empty());
                    }
                    Err(err) => card.recheck_error = Some(to_failure(err)),
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    pub(in crate::views::settings) fn run_update(
        &mut self,
        which: Which,
        cx: &mut Context<Self>,
    ) {
        let card = self.card(which);
        card.update_pending = true;
        card.update_error = None;
        card.recheck_error = None;
        let tag = card.available.clone();
        let (mutations, handle) = self.mutations(cx);
        let release = self.app.read(cx).services.release.clone();
        spawn_action(
            cx,
            &handle,
            async move {
                let vars: Variables =
                    resolve_release_variables(release.as_ref(), which.namespace(), tag.as_deref())
                        .await
                        .map_err(|f| ActionFailure {
                            status: None,
                            message: f.detail,
                        })?
                        .into_iter()
                        .collect();
                mutations
                    .update(which.namespace(), &vars)
                    .await
                    .map_err(to_failure)
            },
            move |this: &mut Self, result, cx| {
                let card = this.card(which);
                card.update_pending = false;
                match result {
                    Ok(UpdateOutcome::Current) => card.available = None,
                    Ok(UpdateOutcome::Started) => {}
                    Err(failure) => card.update_error = Some(failure),
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    fn mutations(
        &self,
        cx: &Context<Self>,
    ) -> (Mutations, tokio::runtime::Handle) {
        let m = self.app.read(cx);
        (
            Mutations::new(m.services.api.clone(), m.services.queries.clone()),
            m.handle().clone(),
        )
    }
}

/// The three rows of one component.
pub fn update_rows(
    view: &mut SettingsView,
    which: Which,
    cx: &mut Context<SettingsView>,
) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let app = view.app.read(cx);
    let entry = app
        .services
        .arrows
        .entries()
        .into_iter()
        .find(|a| bare_namespace(&a.namespace) == which.namespace());
    let t = |key| app.t(key);
    let card = match which {
        Which::Core => &view.core,
    };
    let resolved = entry
        .as_ref()
        .map(|a| a.version.clone())
        .unwrap_or_default();
    let rows = build_core_update(&CoreUpdateInput {
        resolved_ref: &resolved,
        available_ref: card.available.as_deref(),
        state: entry
            .as_ref()
            .map_or(RunState::Absent, |a| RunState::from(a.state)),
        update_pending: card.update_pending,
        recheck_pending: card.recheck_pending,
        update_error: card.update_error.as_ref(),
        recheck_error: card.recheck_error.as_ref(),
        problem: None,
    });
    let description = match &rows.note {
        CoreUpdateNote::Failure(message) => Some(message.clone()),
        CoreUpdateNote::Restarts => Some(match which {
            Which::Core => t(keys::ENGINE_SELF_UPDATE_RESTARTS),
        }),
        CoreUpdateNote::FailedRun(detail) => detail.clone(),
        CoreUpdateNote::None => None,
    };
    let id = which.id();
    let this = cx.entity();
    let mut controls = Vec::new();
    if rows.show_retry {
        let this = this.clone();
        let is_update = rows.retry_is_update;
        controls.push(small_button(
            &format!("{id}.retry"),
            t(arrow::UPDATE_RETRY),
            quiver_ui::ButtonVariant::Outline,
            true,
            move |_, cx| {
                this.update(cx, |v, cx| {
                    if is_update {
                        v.run_update(which, cx)
                    } else {
                        v.recheck(which, cx)
                    }
                })
            },
        ));
    }
    {
        let this = this.clone();
        controls.push(small_button(
            &format!("{id}.check"),
            t(keys::ENGINE_SELF_UPDATE_CHECK),
            quiver_ui::ButtonVariant::Outline,
            rows.check_enabled,
            move |_, cx| this.update(cx, |v, cx| v.recheck(which, cx)),
        ));
    }
    if rows.show_update {
        let updating = !rows.update_enabled;
        controls.push(small_button(
            &format!("{id}.update"),
            t(if updating {
                arrow::ACTION_UPDATING
            } else {
                arrow::ACTION_UPDATE
            }),
            quiver_ui::ButtonVariant::Default,
            rows.update_enabled,
            move |_, cx| this.update(cx, |v, cx| v.run_update(which, cx)),
        ));
    }
    // The value is what the e2e driver reads: the row's own probe carries the label, not the text.
    let value = |name: &str, text: String| {
        probed(
            div()
                .text_size(px(13.))
                .text_color(p.foreground)
                .child(text.clone()),
            format!("{id}.{name}.value"),
            quiver_ui::inspect::Kind::Text,
            text.into(),
            String::new(),
        )
    };
    vec![
        row(
            &p,
            &format!("{id}.installed"),
            t(keys::ENGINE_SELF_UPDATE_INSTALLED),
            None,
            vec![value(
                "installed",
                rows.installed
                    .unwrap_or_else(|| t(keys::ENGINE_SELF_UPDATE_UNKNOWN)),
            )],
        ),
        row(
            &p,
            &format!("{id}.available"),
            t(keys::ENGINE_SELF_UPDATE_AVAILABLE),
            None,
            vec![value(
                "available",
                rows.available
                    .unwrap_or_else(|| t(keys::ENGINE_SELF_UPDATE_CURRENT)),
            )],
        ),
        row(
            &p,
            &format!("{id}.actions"),
            t(keys::ENGINE_SELF_UPDATE_CHECK),
            description.map(Into::into),
            controls,
        ),
    ]
}
