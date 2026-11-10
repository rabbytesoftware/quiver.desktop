//! `Button`: port of `src/components/ui/button.tsx` (7 variants, 10 sizes, loading, disabled).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, transparent_black, AnyElement, App, ClickEvent, CursorStyle, ElementId, FontWeight,
    SharedString, StyleRefinement, Window,
};

use crate::button_style::{ButtonSize, ButtonVariant};
use crate::focus::{keyed_focus, offset_ring, FocusableExt};
use crate::spinner::Spinner;
use crate::theme::ActiveQuiver;

type TooltipFn = Rc<dyn Fn(&mut Window, &mut App) -> gpui::AnyView>;
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
    tooltip: Option<TooltipFn>,
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

    /// Hover tooltip, e.g. `.tooltip(quiver_ui::tooltip("Settings"))`.
    pub fn tooltip(
        mut self,
        f: impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static,
    ) -> Self {
        self.tooltip = Some(Rc::new(f));
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

        let mut ring = offset_ring(cx);
        ring.extend(v.shadow.clone());
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
                    .focus(move |mut s| {
                        s.box_shadow = Some(ring.clone());
                        s
                    })
                    .when_some(self.on_click.clone(), |d, f| {
                        d.on_click(move |e, w, cx| f(e, w, cx))
                    })
            })
            .when(self.selected, |d| d.bg(hover))
            .when_some(self.tooltip, |d, f| d.tooltip(move |w, cx| f(w, cx)))
            .children(children);
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
        el.style().box_shadow = Some(v.shadow);
        el.style().refine(&self.style);
        el
    }
}
