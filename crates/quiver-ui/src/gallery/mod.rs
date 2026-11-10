//! The component gallery: every component in every state the app uses, light and dark.
//!
//! `QUIVER_GALLERY_MODE=dark` and `QUIVER_GALLERY_PAGE=<name>` pick the initial view (screenshots).

mod actions;
mod forms;
mod layout;
mod navigation;
mod overlays;
mod state;
mod surfaces;
mod tiles;

use layout::gid;
use state::{Gallery, Page};

use gpui::prelude::*;
use gpui::{
    div, px, size, App, AppContext, Application, Bounds, Context, FontWeight, Window, WindowBounds,
    WindowOptions,
};
use quiver_theme::{FontFamily, Mode, TypeScale};

use crate::theme::{install, window_root, ActiveQuiver};
use crate::{Button, ButtonSize, ButtonVariant};

impl Render for Gallery {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let body = match self.page {
            Page::Actions => actions::page(cx),
            Page::Forms => forms::page(self, cx),
            Page::Navigation => navigation::page(self, cx),
            Page::Overlays => overlays::page(self, cx),
            Page::Surfaces => surfaces::page(self, cx),
            Page::Tiles => tiles::page(self, cx),
        };
        let toggle = if self.mode == Mode::Dark {
            "Light mode"
        } else {
            "Dark mode"
        };
        let tabs = Page::ALL.iter().map(|(page, name)| {
            let page = *page;
            Button::new(gid(format!("g-page-{name}")))
                .variant(if self.page == page {
                    ButtonVariant::Default
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Sm)
                .label(*name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.page = page;
                    cx.notify();
                }))
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(FontFamily::Sans.family())
            .text_size(px(TypeScale::BODY))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(20.))
                    .h(px(48.))
                    .flex_none()
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .mr(px(12.))
                            .child("quiver-ui gallery"),
                    )
                    .children(tabs)
                    .child(div().flex_1())
                    .child(
                        Button::new("g-mode")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .label(toggle)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.mode = if this.mode == Mode::Dark {
                                    Mode::Light
                                } else {
                                    Mode::Dark
                                };
                                install(cx, this.mode);
                                window.refresh();
                            })),
                    ),
            )
            .child(
                div()
                    .id("g-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(20.))
                    .py(px(16.))
                    .flex()
                    .flex_col()
                    .children(body),
            )
    }
}

/// Registers font files (TTF/OTF bytes) with the text system. Call once at start-up, before any window.
pub fn load_fonts(
    cx: &mut App,
    fonts: Vec<std::borrow::Cow<'static, [u8]>>,
) {
    if let Err(err) = cx.text_system().add_fonts(fonts) {
        eprintln!("quiver-ui: could not load fonts: {err}");
    }
}

/// Opens the gallery window and runs the app loop until it closes. `fonts` are registered first.
pub fn run_gallery(fonts: Vec<std::borrow::Cow<'static, [u8]>>) {
    let mode = if std::env::var("QUIVER_GALLERY_MODE").is_ok_and(|m| m == "dark") {
        Mode::Dark
    } else {
        Mode::Light
    };
    Application::new().run(move |cx: &mut App| {
        load_fonts(cx, fonts);
        install(cx, mode);
        cx.activate(true);
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Gallery::new(mode, window, cx));
                window_root(view, window, cx)
            },
        );
        if opened.is_err() {
            cx.quit();
        }
    });
}
