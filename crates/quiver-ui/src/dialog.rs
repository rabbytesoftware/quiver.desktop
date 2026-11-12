//! `Dialog` and its parts: port of `src/components/ui/dialog.tsx`.
//! Controlled (`open` + `on_dismiss`). Escape and a backdrop press dismiss; the popup takes focus
//! when it opens. Focus is not trapped (Tab can leave the popup), and there is no backdrop blur.

use gpui::prelude::*;
use gpui::{
    black, div, px, Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, MouseButton,
    StyleRefinement, Window,
};
use quiver_theme::Radius;
use std::rc::Rc;

use crate::button::Button;
use crate::button_style::{ButtonSize, ButtonVariant};
use crate::elevation::shadow_lg;
use crate::icon::{icon, Glyph};
use crate::ids::child_id;
use crate::overlay::{escape_handler, popup_focus, DismissFn, ModalLayer, Placement, ENTER};
use crate::parts::part;
use crate::theme::ActiveQuiver;

#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    open: bool,
    show_close: bool,
    width: f32,
    on_dismiss: Option<DismissFn>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: false,
            show_close: true,
            width: 512.0,
            on_dismiss: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn open(
        mut self,
        open: bool,
    ) -> Self {
        self.open = open;
        self
    }

    /// The popup's width in px (`max-w-lg`, 512, by default); narrower windows shrink it to fit.
    /// Pixels, not `w_full().max_w(..)`: a percentage width made the popup's auto height count a
    /// wrapping description as one line, so the footer was squeezed out of the card.
    pub fn width(
        mut self,
        px_width: f32,
    ) -> Self {
        self.width = px_width;
        self
    }

    /// `showCloseButton`.
    pub fn show_close(
        mut self,
        show: bool,
    ) -> Self {
        self.show_close = show;
        self
    }

    /// Called for Escape, a backdrop press and the close button.
    pub fn on_dismiss(
        mut self,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Dialog {
    fn extend(
        &mut self,
        elements: impl IntoIterator<Item = AnyElement>,
    ) {
        self.children.extend(elements);
    }
}

pub(crate) fn close_button(
    id: &ElementId,
    on_dismiss: Option<DismissFn>,
    fg: gpui::Hsla,
) -> impl IntoElement {
    Button::new(child_id(id, "close"))
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Icon)
        .child(div().opacity(0.8).child(icon(Glyph::Close, 16., fg)))
        .on_click(move |_, window, cx| {
            if let Some(f) = &on_dismiss {
                f(window, cx);
            }
        })
}

impl RenderOnce for Dialog {
    fn render(
        self,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let p = cx.quiver().palette;
        let focus = popup_focus(&self.id, window, cx);
        let key = self.id.to_string();
        let mut popup = div()
            .id(child_id(&self.id, "popup"))
            .debug_selector(move || key)
            .track_focus(&focus)
            .relative()
            .flex()
            .flex_col()
            .w(px(self
                .width
                .min(f32::from(window.viewport_size().width) - 32.0)))
            .max_h_full()
            .min_h_0()
            .rounded(px(Radius::XXL))
            .border_1()
            .border_color(p.border)
            .bg(p.popover)
            .text_color(p.popover_foreground)
            .shadow(shadow_lg(black().opacity(0.05)))
            .child(crate::edge::control_edge(cx.quiver(), Radius::XXL, 0.06))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(escape_handler(self.on_dismiss.clone()))
            .children(self.children)
            .when(self.show_close, |d| {
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
        crate::style_merge::apply(popup.style(), &self.style);
        let animated = popup.with_animation(
            child_id(&self.id, "enter"),
            Animation::new(ENTER),
            |el, t| el.opacity(t),
        );
        ModalLayer {
            id: self.id,
            placement: Placement::Dialog,
            inset: 16.0,
            popup: animated.into_any_element(),
            on_dismiss: self.on_dismiss,
        }
        .into_element(window, cx)
    }
}

part!(
    /// `flex flex-col gap-2 p-6`.
    DialogHeader,
    |_p, d| d.flex().flex_col().gap(px(8.)).p(px(24.))
);

part!(
    /// `font-heading font-semibold text-xl leading-none`.
    DialogTitle,
    |p, d| d.text_size(px(20.)).font_weight(FontWeight::SEMIBOLD).line_height(gpui::relative(1.)).text_color(p.popover_foreground)
);

part!(
    /// `text-muted-foreground text-sm`.
    DialogDescription,
    |p, d| d.text_size(px(14.)).text_color(p.muted_foreground)
);

part!(
    /// `p-6` (the scrolling body between header and footer), `pt-1` under a header. Its 24px bottom stays above
    /// the bordered footer; only a bare footer (which brings its own `pt-4`) drops it to `pb-1` in the old CSS,
    /// and no panel here sits on one.
    DialogPanel,
    |_p, d| d.px(px(24.)).pt(px(4.)).pb(px(24.))
);

part!(
    /// Default footer: `flex justify-end gap-2 px-6 py-4 border-t bg-muted/72`, rounded at the bottom.
    DialogFooter,
    |p, d| d
        .flex()
        .justify_end()
        .gap(px(8.))
        .px(px(24.))
        .py(px(16.))
        .border_t_1()
        .border_color(p.border)
        .bg(p.muted.opacity(0.72))
        .rounded_b(px(Radius::XXL - 1.))
);

part!(
    /// `variant="bare"` footer: `flex justify-end gap-2 px-6 pt-4 pb-6`.
    DialogFooterBare,
    |_p, d| d.flex().justify_end().gap(px(8.)).px(px(24.)).pt(px(16.)).pb(px(24.))
);
