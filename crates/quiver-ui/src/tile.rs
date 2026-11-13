//! `Tile`: THE arrow tile, used by Home, Search, Library, Collections, Collection and Recommended.
//!
//! Art (banner or drawn) with the status badge top-right; the caption under it is the description
//! only (two lines). On hover a banner tile shows its title and tags over the banner, and a tile with
//! an action swaps the badge for the Start button. Plain props, no `quiver-state` types.
//!
//! Hovering lifts the art 30px and uncovers a strip under it with the arrow's mark and a mono line (the old
//! tiles' motion, kept by the boards: "tile art, size and animation are unchanged"); see `tile_lift`. The
//! board's 150ms fade of the hover overlay is an instant switch.

use std::rc::Rc;
use std::time::Instant;

use gpui::prelude::*;
use gpui::{
    black, div, linear_color_stop, linear_gradient, px, white, App, ClickEvent, CursorStyle,
    ElementId, FontWeight, SharedString, Window,
};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::icon::{icon, Glyph};
use crate::inspect::{probe, Kind};
use crate::photo::Photo;
use crate::theme::ActiveQuiver;
use crate::tile_art::{art_children, card_stand_in, mark};
use crate::tile_badge::{badge_element, TileBadge};
use crate::tile_lift::{Lift, TileReveal, REVEAL};
use crate::tracked::TrackedText;

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
    reveal: TileReveal,
    banner_image: Option<SharedString>,
    icon_image: Option<SharedString>,
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
            reveal: TileReveal::default(),
            banner_image: None,
            icon_image: None,
        }
    }

    /// The arrow's own images (`media.banner`, `media.icon`): the banner covers a banner tile's art, the icon
    /// fills the drawn tile's mark and the strip's. Empty URLs count as none.
    pub fn media(
        mut self,
        banner: Option<SharedString>,
        icon: Option<SharedString>,
    ) -> Self {
        self.banner_image = banner.filter(|u| !u.is_empty());
        self.icon_image = icon.filter(|u| !u.is_empty());
        self
    }

    /// What the strip under the lifted art says.
    pub fn reveal(
        mut self,
        reveal: TileReveal,
    ) -> Self {
        self.reveal = reveal;
        self
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

    /// The e2e id of the tile (`home.library.tile.<ns>`); the Start button reports `tile.<ns>.start`.
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
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let w = self.size.width();
        let lift_state =
            window.use_keyed_state((self.id.clone(), "lift"), cx, |_, _| Lift::default());
        let frame = lift_state.read(cx).at(Instant::now());
        if frame.moving {
            window.request_animation_frame();
        }
        // Keyed by the tile, not the arrow: the same arrow on two shelves (Recents and Library) is two tiles.
        let group: SharedString = format!("tile-group-{}", self.id).into();
        let pad = w * 0.0625;
        let is_banner = matches!(self.art, TileArtKind::Banner(_));
        let has_start = self.start.is_some();

        let mut art = div()
            .relative()
            .overflow_hidden()
            .rounded(px(12.))
            .w(px(w))
            .h(px(w / 2.0))
            .flex_none();
        let card = self.art.card();
        art = match (&self.art, &self.banner_image) {
            // Old `card-banner`: `bg-cover bg-center`. The card is drawn only until the image has loaded,
            // never under it.
            (TileArtKind::Banner(_), Some(url)) => {
                art.child(Photo::new(url.clone()).radius(12.).stand_in(card_stand_in(
                    card,
                    w,
                    self.icon_image.as_ref(),
                    cx,
                )))
            }
            _ => art
                .bg(p.muted)
                .children(art_children(card, w, self.icon_image.as_ref(), cx)),
        };

        if let (true, Some((title, tags))) = (is_banner, &self.hover) {
            let scrim = div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .rounded_b(px(12.))
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
                    .debug_selector(|| "tile-tag".into())
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
                .child({
                    // Board: 620 max(15, w*.062)px/1.15, letter-spacing -.02em.
                    let size = (w * 0.062).max(15.);
                    TrackedText::new(title.clone(), size, size * 1.15, -0.02 * size, white())
                        .weight(FontWeight::SEMIBOLD)
                        .wrap()
                })
                // Tags are for the big variants only: a small tile has no room for them.
                .when(!tags.is_empty() && w >= TileSize::Wide.width(), |d| {
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

        let hue = match &self.art {
            TileArtKind::Banner(card) | TileArtKind::Drawn(card) => card.hue,
        };
        let strip = strip(
            &self.reveal,
            hue,
            &self.art.card().initial,
            self.icon_image.as_ref(),
            frame.strip,
            cx,
        );
        // The card: the strip at its bottom, the art over it, lifted by the hover.
        let card = div()
            .relative()
            .w(px(w))
            .h(px(w / 2.0))
            .flex_none()
            .child(strip)
            .child(art.absolute().left_0().top(px(-frame.lift)));

        let on_click = self.on_click;
        let mut root = div()
            .id(self.id)
            .group(group)
            .relative()
            .w(px(w))
            .flex_none()
            .rounded(px(Radius::XL))
            .on_hover(move |hovered, _, cx| {
                lift_state.update(cx, |lift, cx| {
                    lift.set_hovered(*hovered, Instant::now());
                    cx.notify();
                });
            })
            .when_some(on_click, |d, f| {
                d.cursor(CursorStyle::PointingHand)
                    .on_click(move |e, window, cx| f(e, window, cx))
            })
            .child(card)
            .child(
                div()
                    .pt(px(7.))
                    .px(px(2.))
                    // `line-clamp-2`, measured per line (see `TrackedText::max_lines`).
                    .child(
                        TrackedText::new(self.caption, 11.5, 15., 0., p.muted_foreground)
                            .max_lines(2),
                    ),
            );
        if let Some(id) = self.probe_id {
            // `lifted` once the hover lift has fully risen (what the e2e checks the motion by).
            let state = if frame.lift >= REVEAL - 0.5 {
                "lifted"
            } else {
                ""
            };
            root = root.child(probe(id, Kind::Button, self.probe_label, true, state));
        }
        root
    }
}

/// The strip the lift uncovers (old `card-info`): the mark at 20px, the mono line, then for a search result the
/// OS letters (L M W, dimmed where there is no build) and the stars.
fn strip(
    reveal: &TileReveal,
    hue: u16,
    initial: &SharedString,
    image: Option<&SharedString>,
    opacity: f32,
    cx: &App,
) -> gpui::Div {
    let p = cx.quiver().palette;
    let mono = quiver_theme::FontFamily::Mono.family();
    let os = reveal.os.map(|has| {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.))
            .children(
                [Glyph::OsLinux, Glyph::OsApple, Glyph::OsWindows]
                    .into_iter()
                    .zip(has)
                    .map(|(glyph, on)| {
                        div().opacity(if on { 0.85 } else { 0.25 }).child(icon(
                            glyph,
                            10.,
                            p.foreground,
                        ))
                    }),
            )
    });
    let stars = reveal.stars.clone().map(|count| {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(3.))
            .opacity(0.6)
            .child(icon(Glyph::Star, 8., p.foreground))
            .child(count)
    });
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(px(REVEAL))
        .px(px(2.))
        .flex()
        .items_center()
        .gap(px(6.))
        .opacity(opacity)
        .text_color(p.foreground)
        .font_family(mono)
        .text_size(px(9.5))
        .line_height(px(12.))
        .child(mark(hue, 20., image, initial).rounded(px(5.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .opacity(0.9)
                        .child(reveal.text.clone()),
                )
                .children(os),
        )
        .children(stars)
}
