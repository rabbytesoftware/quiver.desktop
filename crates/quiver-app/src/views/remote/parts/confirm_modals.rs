//! Rename and Remove: one question each.

use gpui::prelude::*;
use gpui::{div, px, App, Context, Entity, Render, Subscription, Window};
use quiver_state::i18n::en::remote;
use quiver_state::i18n::Params;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader,
    DialogPanel, DialogTitle, Input, InputEvent, InputState,
};

use crate::model::{app_model, close_modal, open_modal};
use crate::views::remote::actions::{REMOVE_MODAL, RENAME_MODAL};
use crate::views::remote::RemoteView;

pub fn open_rename(
    remote: Entity<RemoteView>,
    id: String,
    name: String,
    window: &mut Window,
    cx: &mut App,
) {
    let modal = cx.new(|cx| RenameModal::new(remote, id, name, window, cx));
    open_modal(cx, RENAME_MODAL, modal.into());
}

pub fn open_remove(
    remote: Entity<RemoteView>,
    id: String,
    name: String,
    active: bool,
    cx: &mut App,
) {
    let modal = cx.new(|_| RemoveModal {
        remote,
        id,
        name,
        active,
    });
    open_modal(cx, REMOVE_MODAL, modal.into());
}

fn footer_button(
    id: &'static str,
    label: String,
    variant: ButtonVariant,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(gpui::SharedString::from(id.replace('.', "-")))
        .variant(variant)
        .disabled(!enabled)
        .label(label.clone())
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(probe(id.to_owned(), Kind::Button, label, enabled, ""))
}

struct RenameModal {
    remote: Entity<RemoteView>,
    id: String,
    input: Entity<InputState>,
    _subscription: Subscription,
}

impl RenameModal {
    fn new(
        remote: Entity<RemoteView>,
        id: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        input.update(cx, |i, cx| i.focus(window, cx));
        let subscription = cx.subscribe(&input, |this: &mut Self, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.save(cx);
            }
        });
        Self {
            remote,
            id,
            input,
            _subscription: subscription,
        }
    }

    fn save(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let name = self.input.read(cx).value().trim().to_owned();
        if !name.is_empty() {
            let id = self.id.clone();
            self.remote.update(cx, |v, cx| v.rename(id, name, cx));
        }
    }
}

impl Render for RenameModal {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let m = app_model(cx).read(cx);
        let (title, label, cancel, save) = (
            m.t(remote::RENAME_TITLE),
            m.t(remote::RENAME_LABEL),
            m.t(remote::RENAME_CANCEL),
            m.t(remote::RENAME_SUBMIT),
        );
        let this = cx.entity();
        let enabled = !self.input.read(cx).value().trim().is_empty();
        Dialog::new("remote-rename-dialog")
            .open(true)
            .on_dismiss(|_, cx| close_modal(cx, RENAME_MODAL))
            .child(DialogHeader::new().child(DialogTitle::new().child(title)))
            .child(
                DialogPanel::new().pb(px(20.)).child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(div().text_size(px(13.)).child(label.clone()))
                        .child(Input::new("remote-rename-input", &self.input))
                        .child(probe("remote.rename.input", Kind::Input, label, true, "")),
                ),
            )
            .child(
                DialogFooter::new()
                    .child(footer_button(
                        "remote.rename.cancel",
                        cancel,
                        ButtonVariant::Outline,
                        true,
                        |_, cx| close_modal(cx, RENAME_MODAL),
                    ))
                    .child(footer_button(
                        "remote.rename.save",
                        save,
                        ButtonVariant::Default,
                        enabled,
                        move |_, cx| this.update(cx, |v, cx| v.save(cx)),
                    )),
            )
    }
}

struct RemoveModal {
    remote: Entity<RemoteView>,
    id: String,
    name: String,
    active: bool,
}

impl Render for RemoveModal {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let m = app_model(cx).read(cx);
        let title = m.translator.t_with(
            remote::REMOVE_TITLE,
            &Params::new().with("name", self.name.as_str()),
        );
        let (description, warning, cancel, submit) = (
            m.t(remote::REMOVE_DESCRIPTION),
            m.t(remote::REMOVE_ACTIVE_WARNING),
            m.t(remote::REMOVE_CANCEL),
            m.t(remote::REMOVE_SUBMIT),
        );
        let (remote_view, id, name) = (self.remote.clone(), self.id.clone(), self.name.clone());
        Dialog::new("remote-remove-dialog")
            .open(true)
            .on_dismiss(|_, cx| close_modal(cx, REMOVE_MODAL))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(title))
                    .child(DialogDescription::new().child(description))
                    .when(self.active, |d| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.destructive)
                                .child(warning),
                        )
                    }),
            )
            .child(
                DialogFooter::new()
                    .child(footer_button(
                        "remote.remove.cancel",
                        cancel,
                        ButtonVariant::Outline,
                        true,
                        |_, cx| close_modal(cx, REMOVE_MODAL),
                    ))
                    .child(footer_button(
                        "remote.remove.confirm",
                        submit,
                        ButtonVariant::Destructive,
                        true,
                        move |_, cx| {
                            let (id, name) = (id.clone(), name.clone());
                            remote_view.update(cx, |v, cx| v.remove(id, name, cx))
                        },
                    )),
            )
    }
}
