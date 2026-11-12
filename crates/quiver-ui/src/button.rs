//! `Button`: port of `src/components/ui/button.tsx` (7 variants, 10 sizes, loading, disabled).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    black, div, px, transparent_black, AnyElement, App, ClickEvent, CursorStyle, ElementId,
    FontWeight, SharedString, StyleRefinement, Window,
};

use crate::button_style::{ButtonEdge, ButtonSize, ButtonVariant};
use crate::edge::{control_edge, inner_highlight};
use crate::focus::{keyed_focus, FocusableExt};
use crate::spinner::Spinner;
use crate::theme::ActiveQuiver;
use crate::tooltip::Tip;

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    variant: ButtonVariant,
    size: ButtonSize,
    loading: bool,
    disabled: bool,
    selected: bool,
    on_click: Option<ClickFn>,
    tooltip: Option<Tip>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            loading: false,
            disabled: false,
            selected: false,
            on_click: None,
            tooltip: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(
        mut self,
        v: ButtonVariant,
    ) -> Self {
        self.variant = v;
        self
    }

    pub fn size(
        mut self,
        s: ButtonSize,
    ) -> Self {
        self.size = s;
        self
    }

    /// Disables the button and shows a spinner over its (hidden) content.
    pub fn loading(
        mut self,
        loading: bool,
    ) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(
        mut self,
        disabled: bool,
    ) -> Self {
        self.disabled = disabled;
        self
    }

    /// `data-pressed`: keeps the hover look (an open menu or popover trigger).
    pub fn pressed(
        mut self,
        pressed: bool,
    ) -> Self {
        self.selected = pressed;
        self
    }

    /// Hover tooltip centred above the button, e.g. `.tooltip(quiver_ui::tooltip("Settings"))`.
    pub fn tooltip(
        mut self,
        tip: Tip,
    ) -> Self {
        self.tooltip = Some(tip);
        self
    }

    pub(crate) fn element_id(&self) -> &ElementId {
        &self.id
    }

    pub fn label(
        self,
        text: impl Into<SharedString>,
    ) -> Self {
        self.child(text.into())
    }

    pub fn on_click(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(
        &mut self,
        elements: impl IntoIterator<Item = AnyElement>,
    ) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let q = *cx.quiver();
        let spec = self.size.spec();
        let v = self.variant.resolve(&q);
        let inert = self.disabled || self.loading;
        let focus = keyed_focus(&self.id, window, cx);

        // `ring-2 ring-offset-1`, drawn as two outline-only overlays rather than box-shadow spreads:
        // GPUI paints a spread as a filled box under the element, so a transparent (ghost) button
        // would show the offset colour across its whole face.
        let focused = !inert && focus.is_focused(window);
        let ring_color = q.palette.ring;
        let offset_color = q.palette.background;
        let hover = v.hover_bg;
        let hover_border = v.hover_border;
        let underline = v.underline_on_hover;
        let (fg, children) = (
            if self.loading {
                transparent_black()
            } else {
                v.fg
            },
            self.children,
        );

        let mut el = div()
            .id(self.id.clone())
            .debug_selector({
                let id = self.id.to_string();
                move || id
            })
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(spec.gap))
            .h(px(spec.height))
            .when_some(spec.width, |d, w| d.w(px(w)))
            .px(px(spec.pad_x))
            .rounded(px(spec.radius))
            .border_1()
            .border_color(v.border)
            .bg(v.bg)
            .text_color(fg)
            .text_size(px(spec.text))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .when(self.disabled, |d| d.opacity(0.64))
            .when(self.loading, |d| {
                d.opacity(0.64).cursor(CursorStyle::OperationNotAllowed)
            })
            .when(!inert, |d| {
                d.cursor_pointer()
                    .track_focus(&focus)
                    .tab_stop(true)
                    .no_pointer_focus()
                    .hover(move |s| {
                        let s = s.bg(hover);
                        let s = if let Some(b) = hover_border {
                            s.border_color(b)
                        } else {
                            s
                        };
                        if underline {
                            s.text_decoration_1().text_decoration_solid()
                        } else {
                            s
                        }
                    })
                    .when_some(self.on_click.clone(), |d, f| {
                        d.on_click(move |e, w, cx| f(e, w, cx))
                    })
            })
            .when(self.selected, |d| d.bg(hover))
            .when_some(self.tooltip, |d, tip| {
                d.child(tip.probe()).tooltip(tip.builder())
            })
            .children(children)
            .when(focused, |d| {
                // Insets are from the padding box, so the 1px border is added to each.
                let band = |outset: f32, width: f32, color| {
                    div()
                        .absolute()
                        .top(px(-(outset + 1.)))
                        .left(px(-(outset + 1.)))
                        .right(px(-(outset + 1.)))
                        .bottom(px(-(outset + 1.)))
                        .rounded(px(spec.radius + outset))
                        .border(px(width))
                        .border_color(color)
                };
                d.child(band(1., 1., offset_color))
                    .child(band(3., 2., ring_color))
            });
        let group = SharedString::from(format!("button-{}", self.id));
        // Corners the caller squared (the halves of a split button) are square on the edge layer too, or its
        // rounded line leaves a notch at the seam.
        let squared = self.style.corner_radii.clone();
        let square = move |mut edge: gpui::Div| {
            let zero = |c: &Option<gpui::AbsoluteLength>| matches!(c, Some(gpui::AbsoluteLength::Pixels(v)) if f32::from(*v) == 0.);
            let radii = &mut edge.style().corner_radii;
            for (set, from) in [
                (&mut radii.top_left, &squared.top_left),
                (&mut radii.top_right, &squared.top_right),
                (&mut radii.bottom_left, &squared.bottom_left),
                (&mut radii.bottom_right, &squared.bottom_right),
            ] {
                if zero(from) {
                    *set = Some(px(0.).into());
                }
            }
            edge
        };
        if !inert {
            el = el.group(group.clone());
            el = match v.edge {
                ButtonEdge::None => el,
                ButtonEdge::Highlight => el.child(
                    square(inner_highlight(spec.radius, gpui::white().opacity(0.16)))
                        .id((self.id.clone(), "edge"))
                        .group_active(group.clone(), |s| s.border_color(black().opacity(0.08))),
                ),
                ButtonEdge::Control => el.child(
                    square(control_edge(&q, spec.radius, 0.06))
                        .id((self.id.clone(), "edge"))
                        .group_active(group.clone(), |s| s.border_color(transparent_black())),
                ),
            };
            if !matches!(v.edge, ButtonEdge::None) {
                el = el.active(|mut s| {
                    s.box_shadow = Some(Vec::new());
                    s
                });
            }
            if self.variant == ButtonVariant::Secondary {
                let pressed = q.palette.secondary.opacity(0.8);
                el = el.active(move |s| s.bg(pressed));
            }
        }
        if self.loading {
            let spinner = Spinner::new((self.id.clone(), "spinner"))
                .size(spec.icon)
                .color(v.fg);
            el = el.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(spinner),
            );
        }
        if !inert {
            el.style().box_shadow = Some(v.shadow);
        }
        crate::style_merge::apply(el.style(), &self.style);
        el
    }
}
