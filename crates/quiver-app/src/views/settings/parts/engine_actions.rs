//! The engine tab's calls: load the config, patch it, load the channels and the PATH state. Results come
//! back into the view on the UI thread.

use std::sync::Arc;

use quiver_state::arrow_details::invoke::ActionFailure;
use quiver_state::domain::QUIVER_CORE_NAMESPACE;
use quiver_state::queries::arrow::fetch_channels;
use quiver_state::settings::engine_io::{load_config, load_path, patch_and_reload, setup_path};
use serde_json::Value;

use crate::bridge::spawn_action;
use crate::services::TransportCoreApi;
use crate::views::settings::SettingsView;

pub(in crate::views::settings) fn to_failure(error: quiver_state::Error) -> ActionFailure {
    match error {
        quiver_state::Error::Api(api) => ActionFailure {
            status: Some(api.status).filter(|s| *s != 0),
            message: api.message,
        },
        other => ActionFailure {
            status: None,
            message: other.to_string(),
        },
    }
}

impl SettingsView {
    pub(in crate::views::settings) fn load_engine(&mut self, cx: &mut gpui::Context<Self>) {
        self.engine.begin_load();
        let (api, handle) = {
            let m = self.app.read(cx);
            (m.services.api.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move { load_config(api.as_ref()).await },
            |this: &mut Self, result, cx| {
                this.engine.loaded(result);
                cx.notify();
            },
        );
    }

    /// One patch, then a re-read: the daemon owns what is pending a restart and what it corrected.
    pub(in crate::views::settings) fn patch(&mut self, patch: Value, cx: &mut gpui::Context<Self>) {
        self.engine.begin_patch();
        let (api, raw, handle) = {
            let m = self.app.read(cx);
            (
                m.services.api.clone(),
                Arc::new(TransportCoreApi::new(m.services.manager.clone())),
                m.handle().clone(),
            )
        };
        spawn_action(
            cx,
            &handle,
            async move { patch_and_reload(raw.as_ref(), api.as_ref(), &patch).await },
            |this: &mut Self, result, cx| {
                this.engine.patched(result);
                cx.notify();
            },
        );
        cx.notify();
    }

    pub(in crate::views::settings) fn load_channels(&mut self, cx: &mut gpui::Context<Self>) {
        self.channels = None;
        let (api, handle) = {
            let m = self.app.read(cx);
            (m.services.api.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move {
                fetch_channels(api.as_ref(), QUIVER_CORE_NAMESPACE)
                    .await
                    .map_err(to_failure)
            },
            |this: &mut Self, result, cx| {
                this.channels = Some(result);
                cx.notify();
            },
        );
    }

    pub(in crate::views::settings) fn load_path(&mut self, cx: &mut gpui::Context<Self>) {
        self.path.begin_load();
        let (api, handle) = {
            let m = self.app.read(cx);
            (m.services.api.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move { load_path(api.as_ref()).await },
            |this: &mut Self, result, cx| {
                this.path.loaded(result);
                cx.notify();
            },
        );
    }

    pub(in crate::views::settings) fn run_setup_path(&mut self, cx: &mut gpui::Context<Self>) {
        self.path.begin_setup();
        let (api, handle) = {
            let m = self.app.read(cx);
            (m.services.api.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move { setup_path(api.as_ref()).await },
            |this: &mut Self, result, cx| {
                this.path.setup_done(result);
                cx.notify();
            },
        );
        cx.notify();
    }
}
