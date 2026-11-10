//! The header block: identity at the left, banner art at the right, then the Follow button.

use gpui::prelude::*;
use gpui::{div, hsla, px, Context, FontWeight, IntoElement};
use quiver_state::collection::{build_hero, HeroModel};
use quiver_state::domain::CollectionDetail;
use quiver_state::i18n::en::design;
use quiver_state::tile::hue_for;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    icon, ActiveQuiver, Badge, BadgeSize, BadgeVariant, Button, ButtonSize, ButtonVariant, Glyph,
};

use crate::bridge::spawn_action;
use crate::views::collection::CollectionView;

fn follow_button(
    view: &CollectionView,
    hero: &HeroModel,
    cx: &mut Context<CollectionView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let label = view.app.read(cx).t(hero.follow.label);
    let was_followed = hero.follow.checked;
    let ns = view.id.clone();
    let mut button = Button::new("collection-follow")
        .size(ButtonSize::Lg)
        .disabled(!hero.follow.enabled)
        .variant(if hero.follow.outline {
            ButtonVariant::Outline
        } else {
            ButtonVariant::Default
        });
    if hero.follow.checked {
        button = button.child(icon(Glyph::Check, 16., p.foreground));
    }
    button
        .label(label.clone())
        .child(probe(
            "collection.follow",
            Kind::Button,
            label,
            hero.follow.enabled,
            if was_followed { "checked" } else { "" },
        ))
        .on_click(cx.listener(move |this, _, _, cx| {
            let (mutations, handle) = {
                let a = this.app.read(cx);
                (
                    quiver_state::queries::mutations::Mutations::new(
                        a.services.api.clone(),
                        a.services.queries.clone(),
                    ),
                    a.handle().clone(),
                )
            };
            this.follow_pending = true;
            if this.demo {
                this.flip_followed();
            }
            let ns = ns.clone();
            spawn_action(
                cx,
                &handle,
                async move {
                    if was_followed {
                        mutations.unfollow_collection(&ns).await
                    } else {
                        mutations.follow_collection(&ns).await
                    }
                },
                |this: &mut CollectionView, result, cx| {
                    this.follow_pending = false;
                    if let Err(err) = result {
                        log::warn!("following {} failed: {err}", this.id);
                    }
                    cx.notify();
                },
            );
            cx.notify();
        }))
}

pub fn hero(
    view: &CollectionView,
    detail: &CollectionDetail,
    cx: &mut Context<CollectionView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let model = build_hero(detail, view.follow_pending);
    let hue = f32::from(hue_for(&detail.item.namespace) % 360) / 360.;
    let count = view
        .app
        .read(cx)
        .translator
        .t_count(design::COLLECTION_ARROWS, model.arrow_count);
    let initials: String = detail
        .item
        .name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    let identity = div()
        .flex()
        .items_center()
        .gap(px(16.))
        .child(
            div()
                .flex_none()
                .size(px(44.))
                .rounded(px(10.))
                .bg(hsla(hue, 0.55, 0.42, 1.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(16.))
                .font_weight(FontWeight::BOLD)
                .text_color(gpui::white())
                .child(initials),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .text_size(px(17.))
                        .line_height(px(22.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(detail.item.name.clone()),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(11.))
                        .text_color(p.muted_foreground)
                        .child(detail.item.namespace.clone()),
                ),
        );
    let mut meta = div().flex().items_center().gap(px(8.)).child(
        Badge::new()
            .variant(BadgeVariant::Outline)
            .size(BadgeSize::Sm)
            .label(count),
    );
    if model.unresolved > 0 {
        meta = meta.child(
            Badge::new()
                .variant(BadgeVariant::Error)
                .size(BadgeSize::Sm)
                .label(format!("{} unresolved", model.unresolved)),
        );
    }
    let left = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .w(px(390.))
        .child(identity)
        .child(meta)
        .when(!detail.item.description.is_empty(), |d| {
            d.child(
                div()
                    .text_size(px(14.5))
                    .line_height(px(21.))
                    .text_color(p.foreground.opacity(0.8))
                    .child(detail.item.description.clone()),
            )
        })
        .when(!model.maintainers.is_empty(), |d| {
            d.child(
                div()
                    .flex()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .child(div().text_color(p.muted_foreground).child("Maintained by"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(model.maintainers.join(", ")),
                    ),
            )
        })
        .child(
            div()
                .mt(px(4.))
                .flex()
                .child(follow_button(view, &model, cx)),
        );
    let banner = detail
        .media
        .banner
        .is_some()
        .then(|| quiver_ui::banner_block(hue_for(&detail.item.namespace), 440.).h(px(218.)));
    div()
        .flex()
        .justify_between()
        .gap(px(40.))
        .child(left)
        .children(banner)
}
