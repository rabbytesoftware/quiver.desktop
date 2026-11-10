//! What the Remote screen does: each flow runs on the tokio runtime and its outcome comes back as a
//! toast (and, for the successes, closes the dialog that asked).

use std::time::Duration;

use gpui::Context;
use quiver_state::remote::flows::{add, connect, remove, rename};
use quiver_state::remote::{toast_for, AddSubmission, ConnectionOutcome, TOAST_DURATION_MS};

use super::RemoteView;
use crate::bridge::spawn_action;
use crate::model::close_modal;

pub(super) const ADD_MODAL: &str = "remote-add";
pub(super) const RENAME_MODAL: &str = "remote-rename";
pub(super) const REMOVE_MODAL: &str = "remote-remove";

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
        let message = toast_for(&self.app.read(cx).translator, outcome, name);
        let id = self.toasts.push(message);
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(TOAST_DURATION_MS))
                .await;
            view.update(cx, |view, cx| {
                view.toasts.dismiss(id);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn connect(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        let (backend, connections, status, handle) = {
            let m = self.app.read(cx);
            (
                m.services.backend.clone(),
                m.services.connections.clone(),
                m.services.status.clone(),
                m.handle().clone(),
            )
        };
        spawn_action(
            cx,
            &handle,
            async move { connect(backend.as_ref(), &connections, &id, || status.status()).await },
            move |this: &mut Self, outcome, cx| this.finish(outcome, &name, "", cx),
        );
    }

    pub(crate) fn add(&mut self, submission: AddSubmission, cx: &mut Context<Self>) {
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

    pub(crate) fn rename(&mut self, id: String, name: String, cx: &mut Context<Self>) {
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

    pub(crate) fn remove(&mut self, id: String, name: String, cx: &mut Context<Self>) {
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
