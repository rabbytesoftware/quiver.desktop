//! The root view: the sidebar, the page the router points at, and the modal layer on top.

mod outlet;

use gpui::prelude::*;
use gpui::{div, px, AnyView, Context, Entity, Render, Window};
use quiver_state::shell::SidebarSide;
use quiver_theme::{FontFamily, TypeScale};
use quiver_ui::inspect;
use quiver_ui::ActiveQuiver;

use crate::model::{AppModel, Route};
use crate::views::sidebar::Sidebar;

pub struct AppShell {
    app: Entity<AppModel>,
    sidebar: Entity<Sidebar>,
    /// The page view and the route it was built for.
    page: Option<(Route, AnyView)>,
}

impl AppShell {
    pub fn new(app: Entity<AppModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(app.clone(), window, cx));
        cx.observe(&app, |_, _, cx| cx.notify()).detach();
        Self {
            app,
            sidebar,
            page: None,
        }
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        inspect::begin_frame();
        let p = cx.quiver().palette;
        let route = self.app.read(cx).route().clone();
        let side = self.app.read(cx).shell.sidebar_side;
        let page = match &self.page {
            Some((built_for, view)) if *built_for == route => view.clone(),
            _ => {
                let view = outlet::page_for(&route, &self.app, window, cx);
                self.page = Some((route, view.clone()));
                view
            }
        };
        let content = div().flex_1().min_w_0().h_full().child(page);
        let rail = self.sidebar.clone();
        div()
            .size_full()
            .flex()
            .when(side == SidebarSide::Right, |d| d.flex_row_reverse())
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(FontFamily::Sans.family())
            .text_size(px(TypeScale::BODY))
            .child(rail)
            .child(content)
            .children(self.app.read(cx).modals.views().cloned())
    }
}
