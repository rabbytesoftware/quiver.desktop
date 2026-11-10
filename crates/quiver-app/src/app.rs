//! Starts the GPUI application around [`AppServices`].

use std::sync::Arc;

use gpui::Application;

use crate::model::AppModel;
use crate::services::AppServices;
use crate::window::open_main_window;

/// Runs the app until the main window closes. `seed` fills the stores after the model exists
/// (`--demo` uses it); a real run passes `None`.
pub fn run(services: Arc<AppServices>, demo: bool, seed: Option<fn(&AppModel)>) {
    Application::new().run(move |cx| {
        crate::fonts::load(cx);
        crate::menu::install(cx);
        let model = AppModel::install(cx, services, demo);
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
