//! "Add a remote connection": details (name, URL, health probe), then the pairing code.

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, Focusable as _, FontWeight, Render, Subscription, Window,
};
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

pub fn open_add(
    remote: Entity<RemoteView>,
    window: &mut Window,
    cx: &mut App,
) {
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
    fn new(
        remote: Entity<RemoteView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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

    fn check(
        &mut self,
        cx: &mut Context<Self>,
    ) {
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

    fn submit(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let busy = self.remote.read(cx).add_busy;
        if let Some(submission) = self.form.submit(busy) {
            self.remote.update(cx, |v, cx| v.add(submission, cx));
        }
    }
}

/// A labelled field. Its e2e probe covers the INPUT (not the label above it, which a click would miss) and carries
/// `[focused]` while the field holds the keyboard focus.
fn labelled(
    cx: &Context<AddModal>,
    label: String,
    input: impl IntoElement,
    id: &str,
    focused: bool,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    div()
        .relative()
        .flex()
        .flex_col()
        .gap(px(6.))
        .mb(px(14.))
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.foreground)
                .child(label.clone()),
        )
        .child(div().relative().child(input).child(probe(
            id.to_owned(),
            Kind::Input,
            label,
            true,
            if focused { "[focused]" } else { "" },
        )))
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

impl AddModal {
    /// Six boxes with a dash after the third, drawn from the pairing code; the real field is a one-pixel
    /// input behind them that takes the typing and the paste.
    fn code_slots(
        &self,
        focused: bool,
        cx: &Context<AddModal>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let input = self.code.clone();
        let filled = (0..6).filter(|i| self.form.code.slot(*i).is_some()).count();
        let active = filled.min(5);
        let slot = |i: usize| {
            let digit = self.form.code.slot(i);
            div()
                .flex_none()
                .w(px(36.))
                .h(px(42.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(if focused && i == active {
                    p.ring
                } else {
                    p.input
                })
                .font_family(quiver_theme::FontFamily::Mono.family())
                .text_size(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if digit.is_some() {
                    p.foreground
                } else {
                    p.muted_foreground
                })
                .child(digit.map_or_else(|| "•".to_owned(), String::from))
        };
        let mut row = div().relative().flex().items_center().gap(px(6.));
        for i in 0..6 {
            if i == 3 {
                row = row.child(div().text_color(p.muted_foreground).child("–"));
            }
            row = row.child(slot(i));
        }
        row.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
        })
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size(px(1.))
                .overflow_hidden()
                .opacity(0.)
                .child(Input::new("remote-add-code", &self.code)),
        )
        .child(probe(
            "remote.add.code",
            Kind::Input,
            "code",
            true,
            if focused { "[focused]" } else { "" },
        ))
    }
}

impl Render for AddModal {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focused =
            |input: &Entity<InputState>| input.read(cx).focus_handle(cx).is_focused(window);
        let (name_focus, url_focus, code_focus) =
            (focused(&self.name), focused(&self.url), focused(&self.code));
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
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .child(t(remote::ADD_CODE_LABEL)),
                    )
                    .child(self.code_slots(code_focus, cx))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .line_height(px(18.4))
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
                        name_focus,
                    ))
                    .child(labelled(
                        cx,
                        t(remote::ADD_URL_LABEL),
                        Input::new("remote-add-url", &self.url).invalid(self.form.health_error),
                        "remote.add.url",
                        url_focus,
                    ))
                    .when(self.form.health_error, |d| {
                        d.child(
                            div()
                                .mt(px(-8.))
                                .text_size(px(11.5))
                                .line_height(px(18.4))
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
