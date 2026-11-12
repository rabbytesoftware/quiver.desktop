//! The single action row (`ArrowRunningAfter`): 40 px buttons each with an attached ⓘ, the running
//! action as a `ProgressButton`, and Settings / Methods as ghost icons at the end.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, IntoElement, SharedString, Styled};
use quiver_state::arrow_details::{ActionButton, ActionSlot, ArrowActionKind, ArrowActionVariant};
use quiver_state::i18n::en::{arrow, design};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon, tooltip, ActiveQuiver, Button, ButtonSize, ButtonVariant, Glyph, Palette, ProgressButton,
};

use super::icons::{pict, Pict};
use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

/// The e2e name of an action (`arrow.action.<name>`).
fn name_of(
    kind: ArrowActionKind,
    opens_app: bool,
) -> &'static str {
    match kind {
        ArrowActionKind::AddToLibrary => "add-to-library",
        ArrowActionKind::Install => "install",
        ArrowActionKind::RemoveFromLibrary => "remove-from-library",
        ArrowActionKind::Execute if opens_app => "open",
        ArrowActionKind::Execute => "start",
        ArrowActionKind::Uninstall => "uninstall",
        ArrowActionKind::Update => "update",
        ArrowActionKind::Stop => "stop",
        ArrowActionKind::Restart => "restart",
        ArrowActionKind::Reinstall => "reinstall",
    }
}

fn picture(
    kind: ArrowActionKind,
    opens_app: bool,
) -> Option<Pict> {
    Some(match kind {
        ArrowActionKind::AddToLibrary => Pict::Plus,
        ArrowActionKind::Install => Pict::Download,
        ArrowActionKind::Execute if opens_app => Pict::External,
        ArrowActionKind::Execute => Pict::Play,
        ArrowActionKind::Update | ArrowActionKind::Reinstall | ArrowActionKind::Restart => {
            Pict::Refresh
        }
        ArrowActionKind::Uninstall => Pict::Trash,
        ArrowActionKind::Stop => Pict::Stop,
        ArrowActionKind::RemoveFromLibrary => return None,
    })
}

fn variant(v: ArrowActionVariant) -> ButtonVariant {
    match v {
        ArrowActionVariant::Default => ButtonVariant::Default,
        ArrowActionVariant::Outline => ButtonVariant::Outline,
        ArrowActionVariant::Destructive => ButtonVariant::Destructive,
        ArrowActionVariant::DestructiveOutline => ButtonVariant::DestructiveOutline,
    }
}

fn foreground(
    v: ArrowActionVariant,
    p: &Palette,
) -> gpui::Hsla {
    match v {
        ArrowActionVariant::Default => p.primary_foreground,
        ArrowActionVariant::Destructive => gpui::white(),
        ArrowActionVariant::Outline => p.foreground,
        ArrowActionVariant::DestructiveOutline => p.destructive,
    }
}

fn corners<S: Styled>(
    mut el: S,
    left: f32,
    right: f32,
) -> S {
    let radii = &mut el.style().corner_radii;
    radii.top_left = Some(px(left).into());
    radii.bottom_left = Some(px(left).into());
    radii.top_right = Some(px(right).into());
    radii.bottom_right = Some(px(right).into());
    el
}

fn split_button(
    button: &ActionButton,
    opens_app: bool,
    label: String,
    info_open: Option<ArrowActionKind>,
    tip: &str,
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let p = cx.quiver().palette;
    let name = name_of(button.kind, opens_app);
    let id = format!("arrow.action.{name}");
    let kind = button.kind;
    let fg = foreground(button.variant, &p);
    let main = Button::new(SharedString::from(id.clone()))
        .variant(variant(button.variant))
        .size(ButtonSize::Xl)
        .text_size(px(15.))
        .disabled(!button.enabled && !button.busy)
        .loading(button.busy)
        .on_click(cx.listener(move |this, _, _, cx| this.invoke_action(kind, false, cx)))
        .children(match kind {
            ArrowActionKind::Execute if !opens_app => {
                Some(icon(Glyph::Play, 16., fg).into_any_element())
            }
            _ => picture(kind, opens_app).map(|pic| pict(pic, 16., fg).into_any_element()),
        })
        .child(div().text_color(fg).child(label.clone()))
        .child(probe(id.clone(), Kind::Button, label, button.enabled, ""));
    let info_open = info_open == Some(kind);
    // The ⓘ is a direct child of the group with its own fixed width and no left border (the main
    // button's right border is the seam). It used to sit in a 36 px wrapper holding a `w_full` button
    // with a -1 px margin: the group's auto width then counted the main button only (89 px instead of
    // 124), and the next group was laid over the ⓘ.
    let info = button.info.as_ref().map(|_| {
        let info_id = format!("{id}.info");
        let tip = tip.to_owned();
        let button = Button::new(SharedString::from(info_id.clone()))
            .variant(variant(button.variant))
            .size(ButtonSize::Xl)
            .w(px(35.))
            .px(px(0.))
            .border_l_0()
            .pressed(info_open)
            .tooltip(tooltip(tip.clone()))
            .child(pict(Pict::Info, 17., fg))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.info_open = if this.info_open == Some(kind) {
                    None
                } else {
                    Some(kind)
                };
                this.steps_open = false;
                cx.notify();
            }))
            .child(probe(info_id, Kind::Button, tip, true, ""));
        corners(button, 0., 11.)
    });
    let has_info = info.is_some();
    div()
        .flex()
        .flex_none()
        .child(corners(main, 11., if has_info { 0. } else { 11. }))
        .children(info)
        .into_any_element()
}

fn progress(
    page: &Page,
    view: &ArrowDetailsView,
    cx: &mut Context<ArrowDetailsView>,
) -> Option<AnyElement> {
    let control = page.row.progress()?;
    let tr = view.app.read(cx).translator;
    let steps_tip = tr.t(design::STEPS);
    let fraction = control.fraction.unwrap_or(0.0);
    Some(
        div()
            .relative()
            .child(
                ProgressButton::new("arrow.progress")
                    .title(tr.t(control.verb))
                    .detail(control.step_line(&tr))
                    .progress(fraction)
                    .steps_open(view.steps_open)
                    .on_steps(cx.listener(|this, _, _, cx| {
                        this.steps_open = !this.steps_open;
                        this.info_open = None;
                        cx.notify();
                    })),
            )
            .child(probe(
                "arrow.progress",
                Kind::Other,
                tr.t(control.verb),
                true,
                format!("fraction={fraction:.2}"),
            ))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .w(px(36.))
                    .h(px(40.))
                    .child(probe("arrow.steps", Kind::Button, steps_tip, true, "")),
            )
            .into_any_element(),
    )
}

fn ghost(
    id: &'static str,
    tip: String,
    glyph: AnyElement,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Icon)
        .tooltip(tooltip(tip.clone()))
        .child(glyph)
        .on_click(on_click)
        .child(probe(id, Kind::Button, tip, true, ""))
}

pub fn action_row(
    view: &ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let tr = view.app.read(cx).translator;
    let opens_app = page.facts.lifecycle.as_ref().is_some_and(|_| {
        quiver_state::arrow_details::actions::launch_label(page.facts.lifecycle.as_ref())
            == arrow::ACTION_OPEN
    });
    let mut slots: Vec<AnyElement> = Vec::new();
    for slot in &page.row.slots {
        match slot {
            ActionSlot::Button(button) => {
                let tip = tr.t(arrow::ACTION_INFO);
                slots.push(split_button(
                    button,
                    opens_app,
                    tr.t(button.label),
                    view.info_open,
                    &tip,
                    cx,
                ));
            }
            ActionSlot::Progress(_) => slots.extend(progress(page, view, cx)),
        }
    }
    let settings = page.row.show_settings.then(|| {
        ghost(
            "arrow.settings.open",
            tr.t(design::SETTINGS_OPEN),
            icon(Glyph::Settings, 16., p.muted_foreground).into_any_element(),
            cx.listener(|this, _, _, cx| this.open_settings(cx)),
        )
    });
    let methods = page.row.show_methods.then(|| {
        ghost(
            "arrow.methods.open",
            tr.t(design::METHODS_INSPECT_ONLY),
            pict(Pict::Methods, 16., p.muted_foreground).into_any_element(),
            cx.listener(|this, _, _, cx| this.open_methods(cx)),
        )
    });
    div()
        .relative()
        .flex()
        .items_center()
        .gap(px(10.))
        // The old hero's `pt-4`: room under the description when the row wraps and the column fills up.
        .pt(px(16.))
        // The old hero's `flex flex-wrap gap-2.5`: a row too wide for the column (an install's progress next to
        // a disabled Remove from Library, beside the banner) goes on a second line instead of under the art.
        .child(
            // One wrapping run: the buttons, then the settings/methods icons, so a row that breaks takes the
            // icons with the last button instead of stranding them on a line of their own.
            div()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(10.))
                .children(slots)
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .children(settings)
                        .children(methods),
                ),
        )
}
