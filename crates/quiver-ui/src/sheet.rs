//! `Sheet` and its parts: port of `src/components/ui/sheet.tsx` (side panel from any edge;
//! `default` and `inset` variants). Same dismissal and focus behaviour as [`crate::Dialog`].

use gpui::prelude::*;
use gpui::{
    black, div, px, relative, Animation, AnimationExt, AnyElement, App, ElementId, FontWeight,
    MouseButton, StyleRefinement, Window,
};
use quiver_theme::Radius;
use std::rc::Rc;

use crate::dialog::close_button;
use crate::elevation::shadow_lg;
use crate::ids::child_id;
use crate::overlay::{escape_handler, popup_focus, DismissFn, ModalLayer, Placement, ENTER};
use crate::parts::part;
use crate::theme::ActiveQuiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetSide {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetVariant {
    #[default]
    Default,
    Inset,
}

#[derive(IntoElement)]
pub struct Sheet {
    id: ElementId,
    open: bool,
    side: SheetSide,
    variant: SheetVariant,
    show_close: bool,
    on_dismiss: Option<DismissFn>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Sheet {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            side: SheetSide::Right,
            variant: SheetVariant::Default,
            show_close: true,
            on_dismiss: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub fn variant(mut self, variant: SheetVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show_close(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    pub fn on_dismiss(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl Styled for Sheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let p = cx.quiver().palette;
        let focus = popup_focus(&self.id, window, cx);
        let inset = self.variant == SheetVariant::Inset;
        let side_width = (f32::from(window.viewport_size().width) - 48.0).min(448.0);
        let key = self.id.to_string();
        let mut popup = div()
            .id(child_id(&self.id, "popup"))
            .debug_selector(move || key)
            .track_focus(&focus)
            .relative()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .bg(p.popover)
            .text_color(p.popover_foreground)
            .border_color(p.border)
            .shadow(shadow_lg(black().opacity(0.05)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(escape_handler(self.on_dismiss.clone()));
        popup = match self.side {
            SheetSide::Right => popup.w(px(side_width)).h_full().border_l_1(),
            SheetSide::Left => popup.w(px(side_width)).h_full().border_r_1(),
            SheetSide::Top => popup.w_full().max_h_full().border_b_1(),
            SheetSide::Bottom => popup.w_full().max_h_full().border_t_1(),
        };
        if inset {
            popup = popup.rounded(px(Radius::XXL)).border_1();
        }
        popup = popup.children(self.children).when(self.show_close, |d| {
            d.child(
                div()
                    .absolute()
                    .right(px(8.))
                    .top(px(8.))
                    .child(close_button(
                        &self.id,
                        self.on_dismiss.clone(),
                        p.foreground,
                    )),
            )
        });
        popup.style().refine(&self.style);
        let animated = popup.with_animation(
            child_id(&self.id, "enter"),
            Animation::new(ENTER),
            |el, t| el.opacity(t),
        );
        let placement = match self.side {
            SheetSide::Right => Placement::Right,
            SheetSide::Left => Placement::Left,
            SheetSide::Top => Placement::Top,
            SheetSide::Bottom => Placement::Bottom,
        };
        let layer = ModalLayer {
            id: self.id,
            placement,
            inset: if inset { 16.0 } else { 0.0 },
            popup: animated.into_any_element(),
            on_dismiss: self.on_dismiss,
        };
        layer.into_element(window, cx)
    }
}

part!(
    /// `flex flex-col gap-2 p-6`.
    SheetHeader,
    |_p, d| d.flex().flex_col().gap(px(8.)).p(px(24.))
);

part!(
    /// `font-semibold text-xl leading-none`.
    SheetTitle,
    |p, d| d.text_size(px(20.)).font_weight(FontWeight::SEMIBOLD).line_height(relative(1.)).text_color(p.popover_foreground)
);

part!(
    /// `text-muted-foreground text-sm`.
    SheetDescription,
    |p, d| d.text_size(px(14.)).text_color(p.muted_foreground)
);

part!(
    /// `p-6`.
    SheetPanel,
    |_p, d| d.p(px(24.)).flex_1().min_h_0()
);

part!(
    /// Default footer: `flex justify-end gap-2 px-6 py-4 border-t bg-muted/72`.
    SheetFooter,
    |p, d| d.flex().justify_end().gap(px(8.)).px(px(24.)).py(px(16.)).border_t_1().border_color(p.border).bg(p.muted.opacity(0.72))
);
