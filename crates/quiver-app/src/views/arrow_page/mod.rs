//! The arrow route (`/arrow/<namespace>`). Like the Tauri app, one route decides what the page is: while the
//! arrow's run has an interface it is the app view (header over the arrow's page), otherwise the arrow's
//! details. There is no route of its own for the app; stopping the arrow removes the surface and the page
//! returns to the details by itself.

use gpui::prelude::*;
use gpui::{div, AnyElement, Context, Entity, IntoElement, Render, Window};
use quiver_state::namespace::bare_namespace;

use crate::bridge::observe_store;
use crate::demo::screens::APP_NAMESPACE;
use crate::model::AppModel;
use crate::views::arrow_app::ArrowAppView;
use crate::views::arrow_details::ArrowDetailsView;

pub struct ArrowPageView {
    app: Entity<AppModel>,
    namespace: String,
    details: Entity<ArrowDetailsView>,
    app_view: Option<Entity<ArrowAppView>>,
}

impl ArrowPageView {
    pub fn new(
        app: Entity<AppModel>,
        namespace: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let services = app.read(cx).services.clone();
        observe_store(cx, services.arrows.store());
        let details = cx.new(|cx| ArrowDetailsView::new(app.clone(), namespace.clone(), cx));
        Self {
            app,
            namespace,
            details,
            app_view: None,
        }
    }

    fn has_surface(
        &self,
        cx: &Context<Self>,
    ) -> bool {
        let model = self.app.read(cx);
        if model.demo {
            return self.namespace == APP_NAMESPACE;
        }
        model
            .services
            .arrows
            .entries()
            .iter()
            .find(|e| bare_namespace(&e.namespace) == bare_namespace(&self.namespace))
            .and_then(|e| e.active_run.as_ref())
            .is_some_and(|run| run.surface.is_some())
    }
}

impl Render for ArrowPageView {
    fn render(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let page: AnyElement = if self.has_surface(cx) {
            let (app, namespace, details) = (
                self.app.clone(),
                self.namespace.clone(),
                self.details.clone(),
            );
            self.app_view
                .get_or_insert_with(|| cx.new(|cx| ArrowAppView::new(app, namespace, details, cx)))
                .clone()
                .into_any_element()
        } else {
            self.app_view = None;
            self.details.clone().into_any_element()
        };
        div().size_full().child(page)
    }
}
