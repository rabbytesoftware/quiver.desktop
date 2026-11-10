//! The Methods modal (`ArrowMethodsAfter`): the five lifecycle actions, then the arrow's own
//! methods, each with its steps inspectable in place. Inspect only: methods run from the action row.

use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, FontWeight, Render, SharedString, Window};
use quiver_state::arrow_details::methods::{MethodName, MethodRow, MethodsModel};
use quiver_state::arrow_details::StepSpec;
use quiver_state::i18n::en::{arrow, design};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Dialog,
    DialogDescription, DialogHeader, DialogPanel, DialogTitle,
};

use super::parts::icons::{pict, Pict};
use crate::model::{close_modal, AppModel};

const ID: &str = "arrow-methods";

pub struct MethodsModal {
    app: Entity<AppModel>,
    model: MethodsModel,
    open: HashSet<String>,
}

impl MethodsModal {
    pub fn new(
        app: Entity<AppModel>,
        model: MethodsModel,
        _: &mut Context<Self>,
    ) -> Self {
        Self {
            app,
            model,
            open: HashSet::new(),
        }
    }

    fn name_of(
        &self,
        row: &MethodRow,
        cx: &gpui::App,
    ) -> String {
        match &row.name {
            MethodName::Lifecycle(key) => self.app.read(cx).t(*key).to_lowercase(),
            MethodName::Custom(name) => name.clone(),
        }
    }

    /// `1  stop-saving    exec: save-off`
    fn code(steps: &[StepSpec]) -> String {
        let width = steps
            .iter()
            .map(|s| s.title.chars().count())
            .max()
            .unwrap_or(0)
            + 4;
        steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let detail = s
                    .command
                    .as_deref()
                    .map_or_else(|| s.step_type.clone(), |c| format!("exec: {c}"));
                format!("{:<3}{:<width$}{detail}", i + 1, s.title)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn row(
        &self,
        row: &MethodRow,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let name = self.name_of(row, cx);
        let model = self.app.read(cx);
        let steps = model
            .translator
            .t_count(arrow::METHODS_STEPS, row.steps.len());
        let open = self.open.contains(&name);
        let id = format!("arrow.methods.{name}");
        let toggle_name = name.clone();
        div()
            .relative()
            .py(px(10.))
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .w(px(104.))
                            .flex_none()
                            .font_family(FontFamily::Mono.family())
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted_foreground)
                            .child(steps),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(p.muted_foreground)
                            .children(row.description.clone()),
                    )
                    .child(
                        Button::new(SharedString::from(format!("{id}.inspect")))
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .pressed(open)
                            .child(pict(Pict::Info, 16., p.muted_foreground))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.open.remove(&toggle_name) {
                                    this.open.insert(toggle_name.clone());
                                }
                                cx.notify();
                            }))
                            .child(probe(
                                format!("{id}.inspect"),
                                Kind::Button,
                                name.clone(),
                                true,
                                "",
                            )),
                    ),
            )
            .when(open, |d| {
                d.child(
                    div()
                        .mt(px(10.))
                        .p(px(10.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.border)
                        .bg(p.muted)
                        .font_family(FontFamily::Mono.family())
                        .text_size(px(11.5))
                        .line_height(px(17.))
                        .child(Self::code(&row.steps)),
                )
            })
            .child(probe(
                id,
                Kind::Other,
                name,
                true,
                if open { "expanded" } else { "" },
            ))
    }

    fn group(
        &self,
        label: String,
        rows: &[MethodRow],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let rows: Vec<_> = rows.iter().map(|r| self.row(r, cx)).collect();
        div()
            .mt(px(16.))
            .child(
                div()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(p.border)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.muted_foreground)
                    .child(label.to_uppercase()),
            )
            .children(rows)
    }
}

impl Render for MethodsModal {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let model = self.app.read(cx);
        let (title, note, lifecycle, custom) = (
            model.t(arrow::TAB_METHODS),
            model.t(design::METHODS_INSPECT_NOTE),
            model.t(design::METHODS_LIFECYCLE),
            model.t(design::METHODS_CUSTOM),
        );
        let count = self.model.count().to_string();
        let (lifecycle_rows, custom_rows) =
            (self.model.lifecycle.clone(), self.model.custom.clone());
        Dialog::new(ID)
            .open(true)
            .width(520.)
            .on_dismiss(|_, cx| close_modal(cx, ID))
            .child(
                DialogHeader::new()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(DialogTitle::new().text_size(px(16.)).child(title))
                            .child(Badge::new().variant(BadgeVariant::Secondary).child(count)),
                    )
                    .child(DialogDescription::new().text_size(px(12.5)).child(note)),
            )
            .child(
                DialogPanel::new().pb(px(20.)).child(
                    div()
                        .id("arrow.methods.list")
                        .max_h(px(520.))
                        .overflow_y_scroll()
                        .when(!lifecycle_rows.is_empty(), |d| {
                            d.child(self.group(lifecycle, &lifecycle_rows, cx))
                        })
                        .when(!custom_rows.is_empty(), |d| {
                            d.child(self.group(custom, &custom_rows, cx))
                        }),
                ),
            )
            .child(probe("arrow.methods", Kind::Dialog, "Methods", true, ""))
    }
}
