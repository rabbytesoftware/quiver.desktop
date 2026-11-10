//! The main window: size and title from `quiver_platform::window::MAIN_WINDOW`, the theme from the
//! stored preference, and the shutdown that leaves no orphan daemon.

use gpui::{
    point, px, size, App, AppContext, Bounds, TitlebarOptions, Window, WindowAppearance,
    WindowBounds, WindowOptions,
};
use quiver_platform::window::MAIN_WINDOW;
use quiver_theme::Mode;
use quiver_ui::{install, window_root};

use crate::model::AppModel;
use crate::shell::AppShell;

fn mode_of(model: &AppModel, appearance: WindowAppearance) -> Mode {
    let system_dark = matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    if model.theme.is_dark(system_dark) {
        Mode::Dark
    } else {
        Mode::Light
    }
}

/// Opens the main window. Closing it shuts the services down (the local daemon this app started stops)
/// and ends the app.
pub fn open_main_window(cx: &mut App) -> Option<gpui::AnyWindowHandle> {
    let app = crate::model::app_model(cx);
    let mode = mode_of(app.read(cx), cx.window_appearance());
    install(cx, mode);
    let spec = MAIN_WINDOW;
    let bounds = Bounds::centered(
        None,
        size(px(spec.width as f32), px(spec.height as f32)),
        cx,
    );
    let overlay = cfg!(target_os = "macos") && spec.macos_overlay_titlebar;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(spec.min_width as f32), px(spec.min_height as f32))),
        is_resizable: spec.resizable,
        titlebar: Some(TitlebarOptions {
            title: Some(spec.title.into()),
            appears_transparent: overlay,
            traffic_light_position: overlay.then(|| {
                point(
                    px(spec.macos_traffic_lights.x as f32),
                    px(spec.macos_traffic_lights.y as f32),
                )
            }),
        }),
        app_id: Some(quiver_platform::APP_IDENTIFIER.to_owned()),
        ..WindowOptions::default()
    };
    let services = app.read(cx).services.clone();
    let opened = cx.open_window(options, |window: &mut Window, cx| {
        let shell = cx.new(|cx| AppShell::new(app.clone(), window, cx));
        let services = services.clone();
        if let Some(pages) = &services.pages {
            pages.attach(window);
        }
        window.on_window_should_close(cx, move |_, _| {
            services.shutdown();
            true
        });
        window_root(shell, window, cx)
    });
    if let Err(err) = &opened {
        log::error!("could not open the main window: {err}");
        cx.quit();
        return None;
    }
    cx.on_window_closed(|cx| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();
    opened.ok().map(Into::into)
}
