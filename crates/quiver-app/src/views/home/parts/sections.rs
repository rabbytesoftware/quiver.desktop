//! The stack of Home sections in the order `HomeModel::sections` says.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement};
use quiver_state::domain::Home;
use quiver_state::home::{HomeModel, HomeSection};

use super::{collections, library, popular, recents, skeleton};
use crate::views::home::HomeView;

pub fn sections(
    view: &mut HomeView,
    model: &HomeModel,
    home: Option<&Home>,
    width: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let mut out = div().w_full().flex().flex_col().gap(px(38.));
    for section in &model.sections {
        out = match section {
            HomeSection::Recents => out.child(
                div()
                    .w(px(width))
                    .child(recents::recents(view, model, width, cx)),
            ),
            HomeSection::Library => out.child(
                div()
                    .w(px(width))
                    .child(library::library(view, model, width, cx)),
            ),
            HomeSection::LeadShelf | HomeSection::TrailingShelves => {
                // The lead shelf and the shelves after it are one "Popular" block on Home.
                if matches!(section, HomeSection::LeadShelf) {
                    out.child(
                        div()
                            .w(px(width))
                            .child(popular::popular(view, home, width, cx)),
                    )
                } else {
                    out
                }
            }
            HomeSection::Skeleton => out.child(div().w(px(width)).child(skeleton::skeleton(cx))),
            HomeSection::Collections => out.child(
                div()
                    .w(px(width))
                    .child(collections::collections(view, model, width, cx)),
            ),
        };
    }
    out
}
