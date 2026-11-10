//! The message dialogs of the page: the problem chip, a refused action, a release that could not be
//! resolved (Quiver's own row) and the platform notice before adding an arrow.

use gpui::prelude::*;
use gpui::{AnyElement, Context, IntoElement, SharedString};
use quiver_state::arrow_details::ArrowActionKind;
use quiver_state::i18n::{en::arrow, Params};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};

use crate::views::arrow_details::ArrowDetailsView;

type Run = Box<dyn Fn(&mut ArrowDetailsView, &mut Context<ArrowDetailsView>)>;

struct Choice {
    id: &'static str,
    label: String,
    primary: bool,
    run: Run,
}

fn choice(
    id: &'static str,
    label: String,
    primary: bool,
    run: impl Fn(&mut ArrowDetailsView, &mut Context<ArrowDetailsView>) + 'static,
) -> Choice {
    Choice {
        id,
        label,
        primary,
        run: Box::new(run),
    }
}

fn dialog(
    id: &'static str,
    title: String,
    body: String,
    choices: Vec<Choice>,
    dismiss: fn(&mut ArrowDetailsView, &mut Context<ArrowDetailsView>),
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let buttons: Vec<_> = choices
        .into_iter()
        .map(|c| {
            let run = c.run;
            Button::new(c.id)
                .variant(if c.primary {
                    ButtonVariant::Default
                } else {
                    ButtonVariant::Outline
                })
                .label(c.label.clone())
                .on_click(cx.listener(move |this, _, _, cx| {
                    run(this, cx);
                    cx.notify();
                }))
                .child(probe(c.id, Kind::Button, c.label, true, ""))
        })
        .collect();
    Dialog::new(SharedString::from(id))
        .open(true)
        .on_dismiss({
            let view = cx.entity();
            move |_, cx| {
                view.update(cx, |this, cx| {
                    dismiss(this, cx);
                    cx.notify();
                })
            }
        })
        .child(
            DialogHeader::new()
                .child(DialogTitle::new().child(title))
                .child(DialogDescription::new().child(body)),
        )
        .child(DialogFooter::new().children(buttons))
        .child(probe(id, Kind::Dialog, id, true, ""))
        .into_any_element()
}

pub fn dialogs(
    view: &ArrowDetailsView,
    problem: Option<String>,
    cx: &mut Context<ArrowDetailsView>,
) -> Vec<AnyElement> {
    let tr = view.app.read(cx).translator;
    let mut out = Vec::new();
    if let (true, Some(text)) = (view.problem_open, problem) {
        out.push(dialog(
            "arrow.problem.dialog",
            tr.t(arrow::PROBLEM_LABEL),
            text,
            vec![choice(
                "arrow.problem.dismiss",
                tr.t(arrow::UPDATE_DISMISS),
                true,
                |t, _| {
                    t.problem_open = false;
                },
            )],
            |t, _| t.problem_open = false,
            cx,
        ));
    }
    if let Some(message) = view.invoke.action_error.clone() {
        let retry = view.invoke.retry;
        let mut choices = Vec::new();
        let (title, body) = if retry.is_some() {
            (tr.t(arrow::ACTION_ERROR_TITLE), tr.t(arrow::UPDATE_BUSY))
        } else {
            (tr.t(arrow::ACTION_ERROR_TITLE), message)
        };
        if let Some(kind) = retry {
            choices.push(choice(
                "arrow.error.try-again",
                tr.t(arrow::UPDATE_RETRY),
                true,
                move |t, cx| {
                    t.invoke.dismiss_action_error();
                    t.invoke_action(kind, false, cx);
                },
            ));
        }
        choices.push(choice(
            "arrow.error.dismiss",
            tr.t(arrow::UPDATE_DISMISS),
            retry.is_none(),
            |t, _| {
                t.invoke.dismiss_action_error();
            },
        ));
        out.push(dialog(
            "arrow.action-error",
            title,
            body,
            choices,
            |t, _| t.invoke.dismiss_action_error(),
            cx,
        ));
    }
    if let Some(error) = view.invoke.release_error.clone() {
        let body = format!("{}\n{}", tr.t(error.message), error.detail);
        out.push(dialog(
            "arrow.release-error",
            tr.t(arrow::RELEASE_TITLE),
            body,
            vec![choice(
                "arrow.release-error.dismiss",
                tr.t(arrow::UPDATE_DISMISS),
                true,
                |t, _| {
                    t.invoke.dismiss_release_error();
                },
            )],
            |t, _| t.invoke.dismiss_release_error(),
            cx,
        ));
    }
    if let Some(platform) = view.invoke.platform_warning.clone() {
        let body = tr.t_with(
            arrow::PLATFORM_WARNING_MESSAGE,
            &Params::new().with("platform", platform),
        );
        out.push(dialog(
            "arrow.platform-warning",
            tr.t(arrow::PLATFORM_WARNING_TITLE),
            body,
            vec![
                choice(
                    "arrow.platform-warning.cancel",
                    tr.t(arrow::PLATFORM_WARNING_CANCEL),
                    false,
                    |t, _| {
                        t.invoke.dismiss_platform_warning();
                    },
                ),
                choice(
                    "arrow.platform-warning.confirm",
                    tr.t(arrow::PLATFORM_WARNING_CONFIRM),
                    true,
                    |t, cx| {
                        t.invoke.dismiss_platform_warning();
                        t.invoke_action(ArrowActionKind::AddToLibrary, true, cx);
                    },
                ),
            ],
            |t, _| t.invoke.dismiss_platform_warning(),
            cx,
        ));
    }
    out
}
