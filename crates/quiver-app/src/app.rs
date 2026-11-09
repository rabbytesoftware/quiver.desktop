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
        let model = AppModel::install(cx, services, demo);
        if let Some(seed) = seed {
            seed(model.read(cx));
        }
        cx.activate(true);
        open_main_window(cx);
    });
}
