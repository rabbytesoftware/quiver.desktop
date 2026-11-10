//! The member grid: tiles at the Home library's width, an unresolved member as a fourth kind of cell.

use gpui::prelude::*;
use gpui::{div, px, Context, CursorStyle, FontWeight, IntoElement};
use quiver_state::arrow_details::RunState;
use quiver_state::collection::{grid_items, is_member_installed, GridItem, MemberTile};
use quiver_state::domain::CollectionDetail;
use quiver_state::i18n::en::design;
use quiver_state::queries::keys::collection_key;
use quiver_state::tile::{BadgePolicy, TileInput};
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, TileSize};

use crate::model::Route;
use crate::views::collection::CollectionView;
use crate::views::grid_page::tile_grid;
use crate::views::tiles::tile;

fn input(member: &MemberTile, installed: bool) -> TileInput {
    TileInput {
        description: member.description.clone(),
        icon: member.icon.clone(),
        banner: member.banner.clone(),
        state: Some(if installed {
            RunState::Ready
        } else {
            RunState::Absent
        }),
        user_installed: installed,
        policy: BadgePolicy::WithLibraryState,
        ..TileInput::arrow(&member.route, &member.name)
    }
}

/// A member that did not resolve: same width as a tile, tap to read the collection again.
fn unresolved(
    view: &CollectionView,
    route: &str,
    cx: &mut Context<CollectionView>,
) -> impl IntoElement {
    let p = cx.quiver().palette;
    let a = view.app.read(cx);
    let (title, retry) = (
        a.t(design::COLLECTION_UNRESOLVED_TITLE),
        a.t(design::COLLECTION_UNRESOLVED_RETRY),
    );
    let queries = a.services.queries.clone();
    let key = collection_key(&view.id);
    let id = format!("collection.unresolved.{route}");
    let name = route
        .split('@')
        .next()
        .and_then(|bare| bare.rsplit('/').next())
        .unwrap_or(route)
        .to_owned();
    div()
        .id(gpui::SharedString::from(id.replace('.', "-")))
        .relative()
        .w(px(TileSize::Compact.width()))
        .flex()
        .flex_col()
        .gap(px(8.))
        .cursor(CursorStyle::PointingHand)
        .on_click(move |_, _, _| queries.invalidate(&key))
        .child(
            div()
                .h(px(TileSize::Compact.width() / 2.))
                .rounded(px(12.))
                .bg(p.muted.opacity(0.5))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(6.))
                .px(px(14.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(p.destructive)
                        .child(title),
                ),
        )
        .child(
            div()
                .text_size(px(11.5))
                .line_height(px(15.))
                .text_color(p.muted_foreground)
                .child(retry.clone()),
        )
        .child(probe(id, Kind::Button, retry, true, ""))
}

pub fn members(
    view: &CollectionView,
    detail: &CollectionDetail,
    cx: &mut Context<CollectionView>,
) -> impl IntoElement {
    let library: Vec<String> = view
        .app
        .read(cx)
        .services
        .arrows
        .store()
        .read(|s| s.arrows.keys().cloned().collect());
    let cells = grid_items(&detail.arrows)
        .into_iter()
        .zip(&detail.arrows)
        .map(|(item, arrow)| match item {
            GridItem::Tile(member) => {
                let held = is_member_installed(arrow, &library);
                let go = view.app.clone();
                let route = member.route.clone();
                tile(
                    "collection",
                    &input(&member, held),
                    TileSize::Compact,
                    &view.app,
                    cx,
                )
                .on_click(move |_, _, cx| {
                    let route = Route::Arrow {
                        namespace: route.clone(),
                    };
                    go.update(cx, |m, cx| m.navigate(route, cx));
                })
                .into_any_element()
            }
            GridItem::Unresolved { route } => unresolved(view, &route, cx).into_any_element(),
        })
        .collect::<Vec<_>>();
    tile_grid(cells)
}
