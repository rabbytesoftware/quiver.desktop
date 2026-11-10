//! One connection (RemoteAfter): icon, name and caption, a plain-text label, a plain-text status and the
//! actions menu. Local gets no menu while it is the active connection.

use gpui::prelude::*;
use gpui::{div, hsla, linear_color_stop, linear_gradient, px, Context, FontWeight, IntoElement};
use quiver_state::domain::ConnectionStatus;
use quiver_state::i18n::en::{design, remote, screens};
use quiver_state::remote::ConnectionRowView;
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{icon, ActiveQuiver, Button, ButtonSize, ButtonVariant, Glyph, Menu, MenuItem};

use super::confirm_modals::{open_remove, open_rename};
use crate::views::remote::RemoteView;

/// A stable hue per connection, like the arrow tiles' colour.
fn hue(id: &str) -> f32 {
    let sum: u32 = id.bytes().map(u32::from).sum();
    (sum * 37 % 360) as f32 / 360.0
}

fn status_text(
    row: &ConnectionRowView,
    view: &RemoteView,
    cx: &Context<RemoteView>,
) -> String {
    let m = view.app.read(cx);
    match row.status {
        None => m.t(design::REMOTE_NOT_CONNECTED),
        Some(ConnectionStatus::Ready) => m.t(remote::STATUS_READY),
        Some(ConnectionStatus::Starting) => m.t(remote::STATUS_STARTING),
        Some(ConnectionStatus::Disconnected) => m.t(remote::STATUS_DISCONNECTED),
    }
}

fn actions_menu(
    row: &ConnectionRowView,
    view: &RemoteView,
    cx: &mut Context<RemoteView>,
) -> impl IntoElement {
    let m = view.app.read(cx);
    let (switch, rename, remove_label, more) = (
        m.t(remote::MENU_SWITCH),
        m.t(remote::MENU_RENAME),
        m.t(remote::MENU_REMOVE),
        m.t(remote::MENU_MORE),
    );
    let this = cx.entity();
    let (id, name) = (row.id.clone(), row.name.clone());
    let trigger = Button::new(gpui::SharedString::from(format!("remote-menu-{id}")))
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::IconSm)
        .child(icon(Glyph::Ellipsis, 16., cx.quiver().palette.foreground));
    let mut menu = Menu::new(
        gpui::SharedString::from(format!("remote-menu-popup-{id}")),
        trigger,
    );
    if row.show_connect {
        let (this, id, name) = (this.clone(), id.clone(), name.clone());
        menu = menu.item(
            MenuItem::new(switch)
                .probe(format!("remote.row.{id}.switch"))
                .on_select(move |_, cx| {
                    this.update(cx, |v, cx| v.connect(id.clone(), name.clone(), cx))
                }),
        );
    }
    if row.is_remote {
        let (rename_this, rename_id, rename_name) = (this.clone(), id.clone(), name.clone());
        menu = menu.item(MenuItem::new(rename).on_select(move |window, cx| {
            open_rename(
                rename_this.clone(),
                rename_id.clone(),
                rename_name.clone(),
                window,
                cx,
            )
        }));
        let (remove_this, remove_id, remove_name, active) = (this, id.clone(), name, row.is_active);
        menu = menu.item(
            MenuItem::new(remove_label)
                .destructive(true)
                .on_select(move |_, cx| {
                    open_remove(
                        remove_this.clone(),
                        remove_id.clone(),
                        remove_name.clone(),
                        active,
                        cx,
                    )
                }),
        );
    }
    div().relative().child(menu).child(probe(
        format!("remote.row.{id}.menu"),
        Kind::Button,
        more,
        true,
        "",
    ))
}

pub fn connection_row(
    view: &RemoteView,
    row: &ConnectionRowView,
    last: bool,
    cx: &mut Context<RemoteView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let h = hue(&row.id);
    let tint = |s, l| hsla(h, s, l, 1.);
    let caption = if row.is_local {
        view.app.read(cx).t(screens::REMOTE_LOCAL_CAPTION)
    } else {
        row.subtitle.clone().unwrap_or_default()
    };
    let label = view.app.read(cx).t(if row.is_local {
        design::REMOTE_THIS_DEVICE
    } else {
        design::REMOTE_REMOTE
    });
    let status = status_text(row, view, cx);
    let menu = (row.show_menu_button).then(|| actions_menu(row, view, cx));
    div()
        .relative()
        .flex()
        .items_center()
        .gap(px(16.))
        .py(px(12.))
        .when(!last, |d| d.border_b_1().border_color(p.border))
        .child(
            div()
                .flex_none()
                .size(px(28.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .justify_center()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(tint(0.48, 0.50), 0.),
                    linear_color_stop(tint(0.52, 0.35), 1.),
                ))
                .child(icon(
                    if row.is_local {
                        Glyph::Home
                    } else {
                        Glyph::Server
                    },
                    14.,
                    gpui::white(),
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(18.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(if row.is_local { 12. } else { 11.5 }))
                        .line_height(px(15.))
                        .text_color(p.foreground_subtle)
                        .when(row.is_remote, |d| d.font_family(FontFamily::Mono.family()))
                        .truncate()
                        .child(caption),
                ),
        )
        .child(
            div()
                .flex_none()
                .w(px(96.))
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted_foreground)
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .flex_none()
                .w(px(104.))
                .text_right()
                .text_size(px(12.))
                .text_color(p.muted_foreground)
                .child(status.clone()),
        )
        .child(div().flex_none().w(px(28.)).children(menu))
        .child(probe(
            format!("remote.row.{}", row.id),
            Kind::Other,
            row.name.clone(),
            true,
            if row.is_active {
                format!("selected {status}")
            } else {
                status
            },
        ))
}
