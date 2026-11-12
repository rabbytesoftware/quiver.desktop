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
    Photo, TrackedText,
};

use crate::bridge::spawn_action;
use crate::views::collection::CollectionView;

const FOLLOW_INFO: &str = "Following keeps this collection under Collections. Unfollowing only removes it from that list; the arrows you installed from it stay in your library.";

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

fn follow_group(
    view: &CollectionView,
    hero: &HeroModel,
    cx: &mut Context<CollectionView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let tip = view
        .app
        .read(cx)
        .t(quiver_state::i18n::en::arrow::ACTION_INFO);
    let outline = hero.follow.outline;
    let fg = if outline {
        p.foreground
    } else {
        p.primary_foreground
    };
    let variant = if outline {
        ButtonVariant::Outline
    } else {
        ButtonVariant::Default
    };
    let info = Button::new("collection-follow-info")
        .variant(variant)
        .size(ButtonSize::Xl)
        .w(px(35.))
        .px(px(0.))
        .border_l_0()
        .pressed(view.info_open)
        .tooltip(quiver_ui::tooltip(tip.clone()))
        .child(icon(Glyph::Info, 17., fg))
        .on_click(cx.listener(|this, _, _, cx| {
            this.info_open = !this.info_open;
            cx.notify();
        }))
        .child(probe("collection.follow.info", Kind::Button, tip, true, ""));
    div()
        .relative()
        .flex()
        .child(corners(follow_button(view, hero, cx), 11., 0.))
        .child(corners(info, 0., 11.))
        .when(view.info_open, |d| {
            d.child(
                quiver_ui::layer(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(48.))
                        .w(px(300.))
                        .p(px(14.))
                        .rounded(px(12.))
                        .bg(p.popover)
                        .text_color(p.popover_foreground)
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .border_1()
                        .border_color(p.border)
                        .shadow(vec![gpui::BoxShadow {
                            color: gpui::black().opacity(0.18),
                            offset: gpui::point(px(0.), px(8.)),
                            blur_radius: px(28.),
                            spread_radius: px(0.),
                        }])
                        .child(FOLLOW_INFO)
                        .child(probe(
                            "collection.follow.popover",
                            Kind::Other,
                            "Info",
                            true,
                            "",
                        )),
                )
                .with_priority(2),
            )
        })
}

fn follow_button(
    view: &CollectionView,
    hero: &HeroModel,
    cx: &mut Context<CollectionView>,
) -> Button {
    let p = cx.quiver().palette;
    let label = view.app.read(cx).t(hero.follow.label);
    let was_followed = hero.follow.checked;
    let ns = view.id.clone();
    let mut button = Button::new("collection-follow")
        .size(ButtonSize::Xl)
        .text_size(px(15.))
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
    let monogram = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(10.))
        .bg(hsla(hue, 0.55, 0.42, 1.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(16.))
        .font_weight(FontWeight::BOLD)
        .text_color(gpui::white())
        .child(initials);
    // Old `ArrowIcon`: `media.icon` replaces the monogram once it has loaded; until then, or if it cannot, the
    // monogram shows.
    let icon_url = detail.media.icon.clone().filter(|i| !i.is_empty());
    let identity = div()
        .flex()
        .items_center()
        .gap(px(16.))
        .child(
            div()
                .relative()
                .flex_none()
                .size(px(44.))
                .child(match icon_url {
                    Some(url) => Photo::new(url)
                        .radius(10.)
                        .stand_in(monogram)
                        .into_any_element(),
                    None => monogram.into_any_element(),
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    TrackedText::new(detail.item.name.clone(), 17., 22., -0.2, p.foreground)
                        .weight(FontWeight::SEMIBOLD),
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
                .child(follow_group(view, &model, cx)),
        );
    // Old `collection-banner`: the image as a cover background; the drawn card only until it loads.
    let card = quiver_ui::CardArt {
        initial: detail
            .item
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "?".into())
            .into(),
        name: detail.item.name.clone().into(),
        publisher: detail.item.namespace.clone().into(),
        hue: hue_for(&detail.item.namespace),
    };
    let banner = detail
        .media
        .banner
        .clone()
        .filter(|b| !b.is_empty())
        .map(|url| quiver_ui::banner_photo(url, &card, 440., None, cx).h(px(218.)));
    div()
        .flex()
        .justify_between()
        .gap(px(40.))
        .child(left)
        .children(banner)
}
