//! "Add a remote connection": details (name, URL, health probe), then the pairing code.

use gpui::prelude::*;
use gpui::{div, px, App, Context, Entity, FontWeight, Render, Subscription, Window};
use quiver_state::i18n::en::remote;
use quiver_state::remote::flows::is_reachable;
use quiver_state::remote::{AddConnectionForm, AddStage, PairingCode};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader,
    DialogPanel, DialogTitle, Input, InputEvent, InputState,
};

use crate::bridge::spawn_action;
use crate::model::{app_model, close_modal, open_modal};
use crate::views::remote::actions::ADD_MODAL;
use crate::views::remote::RemoteView;

pub fn open_add(remote: Entity<RemoteView>, window: &mut Window, cx: &mut App) {
    let modal = cx.new(|cx| AddModal::new(remote, window, cx));
    open_modal(cx, ADD_MODAL, modal.into());
}

struct AddModal {
    remote: Entity<RemoteView>,
    form: AddConnectionForm,
    name: Entity<InputState>,
    url: Entity<InputState>,
    code: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl AddModal {
    fn new(remote: Entity<RemoteView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app = app_model(cx);
        let (name_hint, url_hint) = {
            let m = app.read(cx);
            (
                m.t(remote::ADD_NAME_PLACEHOLDER),
                m.t(remote::ADD_URL_PLACEHOLDER),
            )
        };
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(name_hint));
        let url = cx.new(|cx| InputState::new(window, cx).placeholder(url_hint));
        let code = cx.new(|cx| InputState::new(window, cx).placeholder("123456"));
        let subscriptions = vec![
            cx.subscribe_in(&name, window, Self::field_event),
            cx.subscribe_in(&url, window, Self::field_event),
            cx.subscribe_in(&code, window, Self::field_event),
            cx.observe(&remote, |_, _, cx| cx.notify()),
        ];
        name.update(cx, |input, cx| input.focus(window, cx));
        Self {
            remote,
            form: AddConnectionForm::default(),
            name,
            url,
            code,
            _subscriptions: subscriptions,
        }
    }

    fn field_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                if *input == self.name {
                    self.form.set_name(&text);
                } else if *input == self.url {
                    self.form.set_url(&text);
                } else {
                    let mut code = PairingCode::default();
                    code.paste(&text);
                    let digits = code.value();
                    self.form.code = code;
                    if digits != text {
                        input.update(cx, |i, cx| i.set_value(digits, window, cx));
                    }
                }
            }
            InputEvent::PressEnter { .. } => {
                if self.form.stage == AddStage::Details {
                    self.check(cx);
                } else {
                    self.submit(cx);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn check(&mut self, cx: &mut Context<Self>) {
        let Some(url) = self.form.begin_check() else {
            return;
        };
        let (backend, handle) = {
            let m = app_model(cx).read(cx);
            (m.services.backend.clone(), m.handle().clone())
        };
        spawn_action(
            cx,
            &handle,
            async move { is_reachable(backend.as_ref(), &url).await },
            |this: &mut Self, reachable, cx| {
                this.form.finish_check(reachable);
                cx.notify();
            },
        );
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let busy = self.remote.read(cx).add_busy;
        if let Some(submission) = self.form.submit(busy) {
            self.remote.update(cx, |v, cx| v.add(submission, cx));
        }
    }
}

fn labelled(
    cx: &Context<AddModal>,
    label: String,
    input: impl IntoElement,
    id: &str,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .flex()
        .flex_col()
        .gap(px(6.))
        .mb(px(12.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.foreground)
                .child(label.clone()),
        )
        .child(input)
        .child(probe(id.to_owned(), Kind::Input, label, true, ""))
}

fn button(
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

impl Render for AddModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        let m = app_model(cx).read(cx);
        let t = |key| m.t(key);
        let busy = self.remote.read(cx).add_busy;
        let this = cx.entity();
        let pairing = self.form.stage == AddStage::Pairing;
        let (body, footer): (gpui::AnyElement, gpui::AnyElement) = if pairing {
            let (back, submit) = (this.clone(), this.clone());
            (
                div()
                    .child(labelled(
                        cx,
                        t(remote::ADD_CODE_LABEL),
                        Input::new("remote-add-code", &self.code),
                        "remote.add.code",
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted_foreground)
                            .child(t(remote::ADD_CODE_HINT)),
                    )
                    .into_any_element(),
                DialogFooter::new()
                    .child(button(
                        "remote.add.back",
                        t(remote::ADD_BACK),
                        ButtonVariant::Outline,
                        !busy,
                        move |_, cx| {
                            back.update(cx, |v, cx| {
                                v.form.back();
                                cx.notify();
                            })
                        },
                    ))
                    .child(button(
                        "remote.add.submit",
                        t(if busy {
                            remote::ADD_SUBMITTING
                        } else {
                            remote::ADD_SUBMIT
                        }),
                        ButtonVariant::Default,
                        self.form.can_submit(busy),
                        move |_, cx| submit.update(cx, |v, cx| v.submit(cx)),
                    ))
                    .into_any_element(),
            )
        } else {
            let go = this.clone();
            (
                div()
                    .child(labelled(
                        cx,
                        t(remote::ADD_NAME_LABEL),
                        Input::new("remote-add-name", &self.name),
                        "remote.add.name",
                    ))
                    .child(labelled(
                        cx,
                        t(remote::ADD_URL_LABEL),
                        Input::new("remote-add-url", &self.url).invalid(self.form.health_error),
                        "remote.add.url",
                    ))
                    .when(self.form.health_error, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.destructive)
                                .child(t(remote::ADD_HEALTH_ERROR)),
                        )
                    })
                    .into_any_element(),
                DialogFooter::new()
                    .child(button(
                        "remote.add.cancel",
                        t(remote::ADD_CANCEL),
                        ButtonVariant::Outline,
                        true,
                        |_, cx| close_modal(cx, ADD_MODAL),
                    ))
                    .child(button(
                        "remote.add.continue",
                        t(if self.form.checking {
                            remote::ADD_CHECKING
                        } else {
                            remote::ADD_CONTINUE
                        }),
                        ButtonVariant::Default,
                        self.form.can_continue(),
                        move |_, cx| go.update(cx, |v, cx| v.check(cx)),
                    ))
                    .into_any_element(),
            )
        };
        Dialog::new("remote-add-dialog")
            .open(true)
            .on_dismiss(|_, cx| close_modal(cx, ADD_MODAL))
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child(t(remote::ADD_TITLE)))
                    .child(DialogDescription::new().child(t(if pairing {
                        remote::ADD_DESCRIPTION_PAIRING
                    } else {
                        remote::ADD_DESCRIPTION
                    }))),
            )
            .child(DialogPanel::new().pb(px(20.)).child(body))
            .child(footer)
    }
}
