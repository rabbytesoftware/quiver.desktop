//! The app-wide model every view builds against: services, translator, router, modal layer and the
//! app-level queries. One `Entity<AppModel>` lives in a GPUI global; views reach it through the
//! free functions below ([`app_model`], [`navigate`], [`open_modal`], [`close_modal`], [`handle`]).

mod feeds;
mod history;
mod modals;
mod preferences;
mod route;
mod shell_state;

use std::sync::Arc;

use gpui::{AnyView, App, AppContext, Context, Entity, Global, SharedString};
use quiver_platform::window::{MAIN_WINDOW, TITLEBAR_ROW};
use quiver_state::console::CoreVersions;
use quiver_state::domain::Home;
use quiver_state::i18n::{Key, Locale, Translator};
use quiver_state::observable::Store;
use quiver_state::queries::Query;
use quiver_state::remote::{RemoteOverlays, ToastStack, TOAST_DURATION_MS};
use quiver_state::shell::{ShellSettings, SidebarSide, ThemePreference};
use tokio::runtime::Handle;

use crate::services::AppServices;

pub use history::History;
pub use modals::ModalStack;
pub use preferences::{build_indicator, desktop_channel};
pub use route::Route;
pub use shell_state::save_shell;

struct AppHandle(Entity<AppModel>);

impl Global for AppHandle {}

pub struct AppModel {
    pub services: Arc<AppServices>,
    pub translator: Translator,
    pub history: History,
    pub modals: ModalStack,
    /// `GET /v0/home`; kept fresh by a background feed (seeded, not fetched, in demo mode).
    pub home: Query<Home>,
    /// `GET /versions`, for the build indicator; `None` until the daemon answered.
    pub core_versions: Store<Option<CoreVersions>>,
    pub shell: ShellSettings,
    pub theme: ThemePreference,
    /// Quiver's own components (core, desktop) show in lists only when the user asked for them.
    pub show_self_components: bool,
    /// The console dock is open. It is a dock, not a route.
    pub console_open: bool,
    /// The Remote feature's app-wide overlays: the command palette (`cmd_*`). The Add / Rename / Remove
    /// dialogs are modals in [`ModalStack`]; the row menus own their open state.
    pub remote: RemoteOverlays,
    /// The Remote feature's toasts, drawn at the window's bottom-right whatever the route.
    pub toasts: ToastStack,
    pub demo: bool,
}

impl AppModel {
    pub fn new(
        services: Arc<AppServices>,
        demo: bool,
    ) -> Self {
        let home = Query::default();
        let core_versions = Store::new(None);
        if !demo {
            feeds::start_home_feed(&services, home.clone());
            feeds::start_versions_feed(&services, core_versions.clone());
        }
        let (shell, theme) = shell_state::load(services.key_value.as_ref());
        let show_self_components =
            preferences::initial_show_self_components(services.key_value.as_ref());
        Self {
            services,
            translator: Translator::new(Locale::En),
            history: History::default(),
            modals: ModalStack::default(),
            home,
            core_versions,
            shell,
            theme,
            show_self_components,
            console_open: false,
            remote: RemoteOverlays::default(),
            toasts: ToastStack::default(),
            demo,
        }
    }

    /// Creates the model and installs it as the app's global.
    pub fn install(
        cx: &mut App,
        services: Arc<AppServices>,
        demo: bool,
    ) -> Entity<AppModel> {
        let model = cx.new(|_| Self::new(services, demo));
        cx.set_global(AppHandle(model.clone()));
        model
    }

    pub fn route(&self) -> &Route {
        self.history.current()
    }

    pub fn t(
        &self,
        key: Key,
    ) -> String {
        self.translator.t(key)
    }

    pub fn handle(&self) -> &Handle {
        self.services.handle()
    }

    pub fn navigate(
        &mut self,
        route: Route,
        cx: &mut Context<Self>,
    ) {
        if self.history.push(route) {
            cx.notify();
        }
    }

    /// Opens or closes the console dock.
    pub fn set_console_open(
        &mut self,
        open: bool,
        cx: &mut Context<Self>,
    ) {
        if self.console_open != open {
            self.console_open = open;
            cx.notify();
        }
    }

    pub fn replace(
        &mut self,
        route: Route,
        cx: &mut Context<Self>,
    ) {
        if self.history.replace(route) {
            cx.notify();
        }
    }

    pub fn back(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.history.back() {
            cx.notify();
        }
    }

    pub fn forward(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.history.forward() {
            cx.notify();
        }
    }

    pub fn open_modal(
        &mut self,
        id: impl Into<SharedString>,
        view: AnyView,
        cx: &mut Context<Self>,
    ) {
        self.modals.open(id, view);
        cx.notify();
    }

    pub fn close_modal(
        &mut self,
        id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.modals.close(id) {
            cx.notify();
        }
    }

    /// Shows a toast; it goes away by itself after [`TOAST_DURATION_MS`].
    pub fn push_toast(
        &mut self,
        message: String,
        cx: &mut Context<Self>,
    ) {
        let id = self.toasts.push(message);
        cx.spawn(async move |model, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(TOAST_DURATION_MS))
                .await;
            model
                .update(cx, |model, cx| {
                    model.toasts.dismiss(id);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        cx.notify();
    }

    /// Opens the command palette with an empty query (a no-op while it is open).
    pub fn open_cmd(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if !self.remote.cmd_open {
            self.remote.open_cmd();
            cx.notify();
        }
    }

    pub fn close_cmd(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.remote.cmd_open {
            self.remote.close_cmd();
            cx.notify();
        }
    }

    /// How far the page and the console dock start below the top of the window. The traffic lights sit over
    /// the top-left corner: with the sidebar on the left that is the sidebar's own top row, but with it on
    /// the right the corner belongs to the page, which must clear them.
    pub fn titlebar_inset(&self) -> f32 {
        inset_for(self.shell.sidebar_side, MAIN_WINDOW.overlay_titlebar())
    }

    pub fn set_sidebar_width(
        &mut self,
        width: f64,
        cx: &mut Context<Self>,
    ) {
        self.shell.set_width(width);
        shell_state::save_shell(self.services.key_value.as_ref(), &self.shell);
        cx.notify();
    }
}

/// The app model of this process. Panics never: the global is installed before the first window.
pub fn app_model(cx: &App) -> Entity<AppModel> {
    cx.global::<AppHandle>().0.clone()
}

pub fn navigate(
    cx: &mut App,
    route: Route,
) {
    app_model(cx).update(cx, |model, cx| model.navigate(route, cx));
}

pub fn open_modal(
    cx: &mut App,
    id: impl Into<SharedString>,
    view: AnyView,
) {
    app_model(cx).update(cx, |model, cx| model.open_modal(id, view, cx));
}

pub fn close_modal(
    cx: &mut App,
    id: &str,
) {
    app_model(cx).update(cx, |model, cx| model.close_modal(id, cx));
}

/// The tokio handle for work that must not run on the UI thread.
pub fn handle(cx: &App) -> Handle {
    app_model(cx).read(cx).handle().clone()
}

/// The strip the page leaves for the traffic lights: only where the window has an overlay title bar and the
/// sidebar, which normally sits under them, is on the other side.
fn inset_for(
    side: SidebarSide,
    overlay: bool,
) -> f32 {
    if overlay && side == SidebarSide::Right {
        TITLEBAR_ROW
    } else {
        0.
    }
}

#[cfg(test)]
mod inset_tests {
    use super::*;

    #[test]
    fn the_page_clears_the_traffic_lights_only_when_the_sidebar_is_not_under_them() {
        assert_eq!(inset_for(SidebarSide::Right, true), TITLEBAR_ROW);
        assert_eq!(inset_for(SidebarSide::Left, true), 0.);
        assert_eq!(
            inset_for(SidebarSide::Right, false),
            0.,
            "no overlay title bar, no lights to clear"
        );
    }
}
