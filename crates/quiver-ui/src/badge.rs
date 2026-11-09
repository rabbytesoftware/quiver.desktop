//! `Badge`: port of `src/components/ui/badge.tsx` (8 variants, 3 sizes; hover only when clickable).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, transparent_black, white, AnyElement, App, ClickEvent, ElementId, FontWeight, Hsla,
    SharedString, StyleRefinement, Window,
};
use quiver_theme::Radius;

use crate::focus::{keyed_focus, offset_ring, FocusableExt};
use crate::theme::{ActiveQuiver, QuiverTheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Destructive,
    Error,
    Info,
    Outline,
    Secondary,
    Success,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeSize {
    #[default]
    Default,
    Lg,
    Sm,
}

struct Colors {
    bg: Hsla,
    fg: Hsla,
    border: Hsla,
    hover_bg: Hsla,
}

fn colors(v: BadgeVariant, q: &QuiverTheme) -> Colors {
    let p = &q.palette;
    let (soft, soft_hover) = if q.is_dark() {
        (0.16, 0.16)
    } else {
        (0.08, 0.08)
    };
    let tint = |bg: Hsla, fg: Hsla| Colors {
        bg: bg.opacity(soft),
        fg,
        border: transparent_black(),
        hover_bg: bg.opacity(soft_hover),
    };
    match v {
        BadgeVariant::Default => Colors {
            bg: p.primary,
            fg: p.primary_foreground,
            border: transparent_black(),
            hover_bg: p.primary.opacity(0.9),
        },
        BadgeVariant::Destructive => Colors {
            bg: p.destructive,
            fg: white(),
            border: transparent_black(),
            hover_bg: p.destructive.opacity(0.9),
        },
        BadgeVariant::Error => tint(p.destructive, p.destructive_foreground),
        BadgeVariant::Info => tint(p.info, p.info_foreground),
        BadgeVariant::Success => tint(p.success, p.success_foreground),
        BadgeVariant::Warning => tint(p.warning, p.warning_foreground),
        BadgeVariant::Secondary => Colors {
            bg: p.secondary,
            fg: p.secondary_foreground,
            border: transparent_black(),
            hover_bg: p.secondary.opacity(0.9),
        },
        BadgeVariant::Outline => {
            let bg = if q.is_dark() {
                p.input.opacity(0.32)
            } else {
                p.background
            };
            let hover_bg = if q.is_dark() {
                p.input.opacity(0.48)
            } else {
                p.accent.opacity(0.5)
            };
            Colors {
                bg,
                fg: p.foreground,
                border: p.input,
                hover_bg,
            }
        }
    }
}

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Badge {
    id: Option<ElementId>,
    variant: BadgeVariant,
    size: BadgeSize,
    on_click: Option<ClickFn>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Badge {
    pub fn new() -> Self {
        Self {
            id: None,
            variant: BadgeVariant::Default,
            size: BadgeSize::Default,
            on_click: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Stable id for the e2e bridge; required for `on_click`.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn variant(mut self, v: BadgeVariant) -> Self {
        self.variant = v;
        self
    }

    pub fn size(mut self, s: BadgeSize) -> Self {
        self.size = s;
        self
    }

    pub fn label(self, text: impl Into<SharedString>) -> Self {
        self.child(text.into())
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl Default for Badge {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Badge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = colors(self.variant, cx.quiver());
        let (height, pad, text, radius) = match self.size {
            BadgeSize::Default => (18.0, 3.0, 12.0, Radius::SM),
            BadgeSize::Lg => (22.0, 5.0, 14.0, Radius::SM),
            BadgeSize::Sm => (16.0, 3.0, 10.0, 4.0),
        };
        let ring = offset_ring(cx);
        let hover = c.hover_bg;
        let clickable = self.on_click.is_some() && self.id.is_some();
        let focus = self.id.as_ref().map(|id| keyed_focus(id, window, cx));
        let mut el = div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .h(px(height))
            .min_w(px(height))
            .px(px(pad))
            .rounded(px(radius))
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .text_color(c.fg)
            .text_size(px(text))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .children(self.children);
        if let (Some(id), Some(focus)) = (self.id, focus) {
            let key = id.to_string();
            let mut el = el.id(id).debug_selector(move || key);
            if clickable {
                el = el
                    .cursor_pointer()
                    .track_focus(&focus)
                    .tab_stop(true)
                    .no_pointer_focus()
                    .hover(move |s| s.bg(hover))
                    .focus(move |mut s| {
                        s.box_shadow = Some(ring.clone());
                        s
                    });
                if let Some(f) = self.on_click {
                    el = el.on_click(move |e, w, cx| f(e, w, cx));
                }
            }
            el.style().refine(&self.style);
            return el.into_any_element();
        }
        el.style().refine(&self.style);
        el.into_any_element()
    }
}
