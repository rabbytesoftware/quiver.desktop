//! Gallery page: dialog and sheet.

use gpui::prelude::*;
use gpui::{div, AnyElement, Context};

use super::layout::{row, section};
use super::state::{dismiss as g_dismiss, Gallery};
use crate::{
    Button, ButtonVariant, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogPanel,
    DialogTitle, Sheet, SheetDescription, SheetFooter, SheetHeader, SheetPanel, SheetSide,
    SheetTitle,
};

pub(super) fn page(
    g: &mut Gallery,
    cx: &mut Context<Gallery>,
) -> Vec<AnyElement> {
    let open_dialog = Button::new("g-open-dialog")
        .variant(ButtonVariant::Outline)
        .label("Open dialog")
        .on_click(cx.listener(|this, _, _, cx| {
            this.dialog_open = true;
            cx.notify();
        }));
    let sheet_button = |id: &'static str, label: &'static str, side: SheetSide| {
        Button::new(id)
            .variant(ButtonVariant::Outline)
            .label(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sheet_open = Some(side);
                cx.notify();
            }))
    };
    let close_dialog = cx.listener(|this: &mut Gallery, _: &gpui::ClickEvent, _, cx| {
        this.dialog_open = false;
        cx.notify();
    });
    let dialog = Dialog::new("g-dialog")
        .open(g.dialog_open)
        .on_dismiss(g_dismiss(cx, |this| this.dialog_open = false))
        .child(
            DialogHeader::new()
                .child(DialogTitle::new().child("Remove connection"))
                .child(
                    DialogDescription::new()
                        .child("This removes the saved connection. The daemon keeps running."),
                ),
        )
        .child(
            DialogPanel::new()
                .child(div().child("Dialogs dim the window; Escape and a backdrop press dismiss.")),
        )
        .child(
            DialogFooter::new()
                .child(
                    Button::new("g-dialog-cancel")
                        .variant(ButtonVariant::Outline)
                        .label("Cancel")
                        .on_click(close_dialog),
                )
                .child(
                    Button::new("g-dialog-ok")
                        .variant(ButtonVariant::Destructive)
                        .label("Remove"),
                ),
        );
    let side = g.sheet_open.unwrap_or_default();
    let sheet = Sheet::new("g-sheet")
        .open(g.sheet_open.is_some())
        .side(side)
        .on_dismiss(g_dismiss(cx, |this| this.sheet_open = None))
        .child(
            SheetHeader::new()
                .child(SheetTitle::new().child("Arrow settings"))
                .child(SheetDescription::new().child("Edit variables for this Arrow.")),
        )
        .child(SheetPanel::new().child(div().child("Sheet body")))
        .child(SheetFooter::new().child(Button::new("g-sheet-ok").label("Save")));
    vec![
        section(
            "Dialog",
            vec![row("popup", vec![open_dialog.into_any_element()], cx)],
            cx,
        ),
        section(
            "Sheet",
            vec![row(
                "sides",
                vec![
                    sheet_button("g-open-sheet-r", "Right", SheetSide::Right).into_any_element(),
                    sheet_button("g-open-sheet-l", "Left", SheetSide::Left).into_any_element(),
                    sheet_button("g-open-sheet-t", "Top", SheetSide::Top).into_any_element(),
                    sheet_button("g-open-sheet-b", "Bottom", SheetSide::Bottom).into_any_element(),
                ],
                cx,
            )],
            cx,
        ),
        dialog.into_any_element(),
        sheet.into_any_element(),
    ]
}
