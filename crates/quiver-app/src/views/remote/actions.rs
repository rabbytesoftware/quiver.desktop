//! What the Remote screen does: each flow runs on the tokio runtime and its outcome comes back as a
//! toast (and, for the successes, closes the dialog that asked).

use gpui::{App, Context, Entity};
use quiver_state::remote::flows::{add, connect, remove, rename};
use quiver_state::remote::{toast_for, AddSubmission, ConnectionOutcome};

use super::RemoteView;
use crate::bridge::spawn_action;
use crate::model::{close_modal, AppModel};

pub(super) const ADD_MODAL: &str = "remote-add";
pub(super) const RENAME_MODAL: &str = "remote-rename";
pub(super) const REMOVE_MODAL: &str = "remote-remove";

/// The Remote dialogs the command palette closes when it opens (`CLOSE_OTHER_OVERLAYS`).
pub(super) const DIALOGS: [&str; 3] = [ADD_MODAL, RENAME_MODAL, REMOVE_MODAL];

/// Switches to connection `id` and toasts what the status became (or that the switch failed). Shared by
/// the Remote rows and the command palette, so "switch and toast" is written once.
pub(crate) fn connect_to(
    app: &Entity<AppModel>,
    id: String,
    name: String,
    cx: &mut App,
) {
    app.update(cx, |m, cx| {
        let (backend, connections, status, handle) = (
            m.services.backend.clone(),
            m.services.connections.clone(),
            m.services.status.clone(),
            m.handle().clone(),
        );
        spawn_action(
            cx,
            &handle,
            async move { connect(backend.as_ref(), &connections, &id, || status.status()).await },
            move |m: &mut AppModel, outcome, cx| {
                let message = toast_for(&m.translator, outcome, &name);
                m.push_toast(message, cx);
            },
        );
    });
}

impl RemoteView {
    fn finish(
        &mut self,
        outcome: ConnectionOutcome,
        name: &str,
        modal: &str,
        cx: &mut Context<Self>,
    ) {
        if outcome.closes_dialog() {
            close_modal(cx, modal);
        }
        self.app.update(cx, |m, cx| {
            let message = toast_for(&m.translator, outcome, name);
            m.push_toast(message, cx);
        });
        cx.notify();
    }

    pub(super) fn connect(
        &mut self,
        id: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        connect_to(&self.app.clone(), id, name, cx);
    }

    pub(crate) fn add(
        &mut self,
        submission: AddSubmission,
        cx: &mut Context<Self>,
    ) {
        self.add_busy = true;
        let (backend, handle) = {
            let m = self.app.read(cx);
            (m.services.backend.clone(), m.handle().clone())
        };
        let name = submission.name.clone();
        spawn_action(
            cx,
            &handle,
            async move { add(backend.as_ref(), &submission).await },
            move |this: &mut Self, outcome, cx| {
                this.add_busy = false;
                this.finish(outcome, &name, ADD_MODAL, cx);
            },
        );
        cx.notify();
    }

    pub(crate) fn rename(
        &mut self,
        id: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let (backend, handle) = {
            let m = self.app.read(cx);
            (m.services.backend.clone(), m.handle().clone())
        };
        let new_name = name.clone();
        spawn_action(
            cx,
            &handle,
            async move { rename(backend.as_ref(), &id, &name).await },
            move |this: &mut Self, outcome, cx| this.finish(outcome, &new_name, RENAME_MODAL, cx),
        );
    }

    pub(crate) fn remove(
        &mut self,
        id: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let (backend, handle) = {
            let m = self.app.read(cx);
            (m.services.backend.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move { remove(backend.as_ref(), &id).await },
            move |this: &mut Self, outcome, cx| this.finish(outcome, &name, REMOVE_MODAL, cx),
        );
    }
}
