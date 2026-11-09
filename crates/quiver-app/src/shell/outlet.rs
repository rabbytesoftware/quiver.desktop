//! Route -> page view. Each feature agent adds its arm here when its view lands; until then a route
//! shows a placeholder naming the screen (with its `screen.<name>` e2e id).

use gpui::prelude::*;
use gpui::{div, px, AnyView, AppContext, Context, Entity, Render, Window};
use quiver_ui::ActiveQuiver;

use super::AppShell;
use crate::model::{AppModel, Route};
use crate::views::home::HomeView;
use crate::views::screen_probe;

pub(super) fn page_for(
    route: &Route,
    app: &Entity<AppModel>,
    _: &mut Window,
    cx: &mut Context<AppShell>,
) -> AnyView {
    match route {
        Route::Home => cx.new(|cx| HomeView::new(app.clone(), cx)).into(),
        other => cx
            .new(|_| Placeholder {
                screen: other.screen(),
            })
            .into(),
    }
}

/// A route whose view has not been built yet.
struct Placeholder {
    screen: &'static str,
}

impl Render for Placeholder {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.quiver().palette;
        div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.))
            .text_color(p.muted_foreground)
            .child(format!("{} (not built yet)", self.screen))
            .child(screen_probe(self.screen))
    }
}
