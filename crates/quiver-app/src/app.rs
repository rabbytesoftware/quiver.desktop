//! Starts the GPUI application around [`AppServices`].

use std::sync::Arc;

use gpui::Application;

use crate::model::AppModel;
use crate::services::AppServices;
use crate::window::open_main_window;

/// Runs the app until the main window closes. `seed` fills the stores after the model exists
/// (`--demo` uses it); a real run passes `None`.
pub fn run(services: Arc<AppServices>, demo: bool, seed: Option<fn(&AppModel)>) {
    // Defence in depth: the UI thread is not a runtime thread, so a bare `tokio::spawn` reached from
    // a view would panic with "no reactor running". The call sites pass handles explicitly; this
    // keeps one forgotten site from taking the app down.
    let _runtime = services.handle().clone();
    let _in_runtime = _runtime.enter();
    Application::new().run(move |cx| {
        crate::fonts::load(cx);
        crate::menu::install(cx);
        let model = AppModel::install(cx, services, demo);
        if let Some(theme) = demo
            .then(crate::demo::route::theme_from_environment)
            .flatten()
        {
            model.update(cx, |m, _| m.theme = theme);
        }
        if let Some(seed) = seed {
            seed(model.read(cx));
        }
        cx.activate(true);
        let _window = open_main_window(cx);
        #[cfg(feature = "e2e-bridge")]
        if let Some(window) = _window {
            crate::e2e_bridge::attach(cx, window, &model);
        }
        if demo {
            if let Some(route) = crate::demo::route::from_environment() {
                model.update(cx, |m, cx| m.navigate(route, cx));
            }
        }
    });
}
