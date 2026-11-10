//! `Tile`: THE arrow tile, used by Home, Search, Library, Collections, Collection and Recommended.
//!
//! Art (banner or drawn) with the status badge top-right; the caption under it is the description
//! only (two lines). On hover a banner tile shows its title and tags over the banner, and a tile with
//! an action swaps the badge for the Start button. Plain props, no `quiver-state` types.
//!
//! GPUI has no CSS transitions, so the 150ms fade of the board is an instant switch.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    black, div, linear_color_stop, linear_gradient, px, white, App, ClickEvent, CursorStyle,
    ElementId, FontWeight, SharedString, Window,
};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::icon::{icon, Glyph};
use crate::inspect::{probe, Kind};
use crate::theme::ActiveQuiver;
use crate::tile_art::{art_children, banner_background};
use crate::tile_badge::{badge_element, TileBadge};

pub use crate::tile_art::TileArtKind;

/// The widths the boards use; the art is always twice as wide as it is tall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileSize {
    /// Home Recents and shelves (288).
    Shelf,
    /// Home Library and dense grids (213).
    Compact,
    /// Two-up rows (439).
    Wide,
    /// Collection hero grid (445).
    Large,
    Custom(f32),
}

impl TileSize {
    pub fn width(self) -> f32 {
        match self {
            Self::Shelf => 288.0,
            Self::Compact => 213.0,
            Self::Wide => 439.0,
            Self::Large => 445.0,
            Self::Custom(w) => w,
        }
    }
}

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The hover button (top-right): "Start" / "Open" / "Stop".
pub struct TileStart {
    pub label: SharedString,
    pub on_click: ClickFn,
}

impl TileStart {
    pub fn new(
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            on_click: Rc::new(on_click),
        }
    }
}

#[derive(IntoElement)]
pub struct Tile {
    id: ElementId,
    namespace: SharedString,
    size: TileSize,
    art: TileArtKind,
    badge: Option<TileBadge>,
    caption: SharedString,
    hover: Option<(SharedString, Vec<SharedString>)>,
    start: Option<TileStart>,
    on_click: Option<ClickFn>,
    probe_id: Option<SharedString>,
    probe_label: SharedString,
}

impl Tile {
    pub fn new(
        id: impl Into<ElementId>,
        namespace: impl Into<SharedString>,
        art: TileArtKind,
    ) -> Self {
        Self {
            id: id.into(),
            namespace: namespace.into(),
            size: TileSize::Shelf,
            art,
            badge: None,
            caption: SharedString::default(),
            hover: None,
            start: None,
            on_click: None,
            probe_id: None,
            probe_label: SharedString::default(),
        }
    }

    pub fn size(
        mut self,
        size: TileSize,
    ) -> Self {
        self.size = size;
        self
    }

    pub fn badge(
        mut self,
        badge: Option<TileBadge>,
    ) -> Self {
        self.badge = badge;
        self
    }

    /// The description (two lines at most).
    pub fn caption(
        mut self,
        caption: impl Into<SharedString>,
    ) -> Self {
        self.caption = caption.into();
        self
    }

    /// Title and tags shown over a banner on hover.
    pub fn hover(
        mut self,
        title: impl Into<SharedString>,
        tags: Vec<SharedString>,
    ) -> Self {
        self.hover = Some((title.into(), tags));
        self
    }

    pub fn start(
        mut self,
        start: Option<TileStart>,
    ) -> Self {
        self.start = start;
        self
    }

    pub fn on_click(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// The e2e id of the tile (`home.tile.<ns>`); the Start button reports `tile.<ns>.start`.
    pub fn probe(
        mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.probe_id = Some(id.into());
        self.probe_label = label.into();
        self
    }
}

impl RenderOnce for Tile {
    fn render(
        self,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let w = self.size.width();
        let group: SharedString = format!("tile-{}-{}", self.namespace, w).into();
        let pad = w * 0.0625;
        let is_banner = matches!(self.art, TileArtKind::Banner { .. });
        let has_start = self.start.is_some();

        let mut art = div()
            .relative()
            .overflow_hidden()
            .rounded(px(12.))
            .w(px(w))
            .h(px(w / 2.0))
            .flex_none();
        art = match &self.art {
            TileArtKind::Banner { hue } => art.bg(banner_background(*hue)),
            TileArtKind::Drawn { .. } => art.bg(p.muted),
        };
        art = art.children(art_children(&self.art, w, cx));

        if let (true, Some((title, tags))) = (is_banner, &self.hover) {
            let scrim = div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(w / 2.0 * 0.62))
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(black().opacity(0.0), 0.),
                    linear_color_stop(black().opacity(0.62), 1.),
                ))
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.));
            let pills = tags.iter().map(|t| {
                div()
                    .flex()
                    .items_center()
                    .h(px(20.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(white().opacity(0.22))
                    .text_color(white())
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(t.clone())
            });
            let info = div()
                .absolute()
                .left(px(pad))
                .right(px(pad))
                .bottom(px(pad * 0.8))
                .flex()
                .flex_col()
                .items_start()
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .child(
                    div()
                        .text_size(px((w * 0.062).max(15.)))
                        .line_height(px((w * 0.062).max(15.) * 1.15))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(white())
                        .child(title.clone()),
                )
                .when(!tags.is_empty(), |d| {
                    d.child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(5.))
                            .mt(px(6.))
                            .children(pills),
                    )
                });
            art = art.child(scrim).child(info);
        }

        if let Some(badge) = &self.badge {
            let mut el = badge_element(badge, (self.id.clone(), "badge").into(), cx).child(probe(
                format!("tile.{}.badge", self.namespace),
                Kind::Text,
                badge.label.clone(),
                true,
                "",
            ));
            if has_start {
                el = el.group_hover(group.clone(), |s| s.opacity(0.));
            }
            art = art.child(el);
        }

        if let Some(start) = self.start {
            let on_click = start.on_click.clone();
            let probe_id: SharedString = format!("tile.{}.start", self.namespace).into();
            art = art.child(
                div()
                    .id((self.id.clone(), "start"))
                    .absolute()
                    .top(px(10.))
                    .right(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(26.))
                    .px(px(12.))
                    .rounded(px(8.))
                    .bg(p.popover)
                    .text_color(p.popover_foreground)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .shadow(shadow_xs(black().opacity(0.14)))
                    .cursor(CursorStyle::PointingHand)
                    .opacity(0.)
                    .group_hover(group.clone(), |s| s.opacity(1.))
                    .on_click(move |e, window, cx| {
                        cx.stop_propagation();
                        on_click(e, window, cx);
                    })
                    .child(icon(Glyph::Play, 11., p.popover_foreground))
                    .child(start.label.clone())
                    .child(probe(probe_id, Kind::Button, start.label, true, "")),
            );
        }

        let on_click = self.on_click;
        let mut root = div()
            .id(self.id)
            .group(group)
            .relative()
            .w(px(w))
            .flex_none()
            .rounded(px(Radius::XL))
            .when_some(on_click, |d, f| {
                d.cursor(CursorStyle::PointingHand)
                    .on_click(move |e, window, cx| f(e, window, cx))
            })
            .child(art)
            .child(
                div()
                    .pt(px(7.))
                    .px(px(2.))
                    .text_size(px(11.5))
                    .line_height(px(15.))
                    .text_color(p.muted_foreground)
                    .line_clamp(2)
                    .child(self.caption),
            );
        if let Some(id) = self.probe_id {
            root = root.child(probe(id, Kind::Button, self.probe_label, true, ""));
        }
        root
    }
}
