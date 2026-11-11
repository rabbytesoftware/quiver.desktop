//! Right-click on a sidebar arrow row (`use-arrow-context-menu.ts`): the OS's own context menu offering
//! what the arrow's details page offers, without opening the page. "Open details" comes first; an action
//! that needs input (variables) is routed to the page instead of guessed at. The items are built from the
//! arrow's detail, fetched first as the Tauri app did, so they match the page exactly.
//!
//! When no native menu can be shown (no GTK on this display), the same items appear as an in-app menu.

use gpui::{Context, IntoElement, Pixels, Point};
use quiver_platform::platform_key;
use quiver_state::arrow_details::{facts_from_detail, ArrowActionKind, ArrowFacts, RunState};
use quiver_state::i18n::en::arrow;
use quiver_state::namespace::bare_namespace;
use quiver_state::queries::arrow::fetch_arrow_detail;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_state::sidebar::{arrow_menu_items, ArrowMenuItem};
use quiver_ui::{ContextMenu, MenuItem};

use super::super::Sidebar;
use crate::bridge::spawn_detached;
use crate::model::{AppModel, Route};
use crate::services::native_menu::{popup, MenuAnchor, MenuEntry, Picked};

/// How long the menu waits for the arrow's detail before it opens from the row's own facts.
const DETAIL_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// The open menu's arrow, its items and (for the in-app fallback) where it opens.
#[derive(Clone)]
pub struct RowMenu {
    pub namespace: String,
    pub items: Vec<ArrowMenuItem>,
    pub position: Point<Pixels>,
}

fn run(
    model: &AppModel,
    namespace: String,
    item: &ArrowMenuItem,
) {
    let mutations = Mutations::new(model.services.api.clone(), model.services.queries.clone());
    let kind = item.kind;
    spawn_detached(model.handle(), async move {
        let vars = Variables::new();
        let outcome = match kind {
            ArrowActionKind::Execute => mutations.execute_arrow(&namespace, &vars).await,
            ArrowActionKind::Stop => mutations.stop(&namespace, &vars).await,
            ArrowActionKind::Install | ArrowActionKind::Reinstall => {
                mutations.install(&namespace, &vars).await
            }
            ArrowActionKind::Update => mutations.update(&namespace, &vars).await.map(drop),
            ArrowActionKind::Uninstall => mutations.uninstall(&namespace, &vars).await,
            ArrowActionKind::RemoveFromLibrary => mutations.remove_arrow(&namespace).await,
            _ => Ok(()),
        };
        if let Err(err) = outcome {
            log::warn!("{kind:?} on {namespace} failed: {err}");
        }
    });
}

/// "Open details", or an item that needs input: the arrow's page. Anything else runs from here.
fn choose(
    app: &gpui::Entity<AppModel>,
    namespace: &str,
    item: Option<&ArrowMenuItem>,
    cx: &mut gpui::App,
) {
    match item {
        Some(item) if !item.via_details => run(app.read(cx), namespace.to_owned(), item),
        _ => {
            let route = Route::Arrow {
                namespace: namespace.to_owned(),
            };
            app.update(cx, |m, cx| m.navigate(route, cx));
        }
    }
}

impl Sidebar {
    pub(in crate::views::sidebar) fn open_row_menu(
        &mut self,
        namespace: String,
        position: Point<Pixels>,
        anchor: MenuAnchor,
        cx: &mut Context<Self>,
    ) {
        // The menu is up until the person answers it: a second right-click meanwhile would stack a second.
        if self.menu_busy {
            return;
        }
        let model = self.app.read(cx);
        let Some(entry) = model
            .services
            .arrows
            .entries()
            .into_iter()
            .find(|a| bare_namespace(&a.namespace) == namespace)
        else {
            return;
        };
        self.menu_busy = true;
        let tr = model.translator;
        let fallback = ArrowFacts {
            namespace: entry.namespace.clone(),
            name: entry.name.clone(),
            state: RunState::from(entry.state),
            user_installed: true,
            resolved_ref: entry.version.clone(),
            ..ArrowFacts::default()
        };
        // `--demo` has no daemon to ask: its arrow page's own detail stands in.
        let demo = model
            .demo
            .then(|| crate::views::arrow_details::demo::detail(&entry));
        let api = model.services.api.clone();
        let full = entry.namespace.clone();
        let detail = model.handle().spawn(async move {
            if let Some(detail) = demo {
                return Some(detail);
            }
            // A daemon that does not answer quickly gets the row's own facts, not a menu that never opens.
            tokio::time::timeout(DETAIL_WAIT, fetch_arrow_detail(api.as_ref(), &full))
                .await
                .ok()?
                .ok()
        });
        let app = self.app.clone();
        cx.spawn(async move |this, cx| {
            let facts = match detail.await {
                Ok(Some(detail)) => facts_from_detail(&detail, &platform_key()),
                _ => fallback,
            };
            let items = arrow_menu_items(&facts);
            let mut entries = vec![
                MenuEntry::Item(tr.t(arrow::MENU_DETAILS)),
                MenuEntry::Separator,
            ];
            entries.extend(items.iter().map(|item| {
                let label = tr.t(item.label);
                MenuEntry::Item(if item.via_details {
                    format!("{label}…")
                } else {
                    label
                })
            }));
            let picked = popup(anchor, entries, cx).await;
            this.update(cx, |sidebar, cx| {
                sidebar.menu_busy = false;
                match picked {
                    Picked::Item(0) => choose(&app, &namespace, None, cx),
                    Picked::Item(ix) => choose(&app, &namespace, items.get(ix - 2), cx),
                    Picked::Dismissed => {}
                    Picked::Unavailable => {
                        sidebar.context = Some(RowMenu {
                            namespace,
                            items,
                            position,
                        });
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }
}

/// The in-app menu, only when the OS one could not be shown.
pub fn context_menu(
    sidebar: &Sidebar,
    cx: &mut Context<Sidebar>,
) -> Option<impl IntoElement> {
    let RowMenu {
        namespace,
        items,
        position,
    } = sidebar.context.clone()?;
    let app = sidebar.app.clone();
    let this = cx.entity();
    let tr = app.read(cx).translator;
    let details = {
        let (app, namespace) = (app.clone(), namespace.clone());
        MenuItem::new(tr.t(arrow::MENU_DETAILS))
            .on_select(move |_, cx| choose(&app, &namespace, None, cx))
    };
    let mut menu = ContextMenu::new("sidebar-row-menu", position)
        .probes("sidebar.menu")
        .on_dismiss(move |_, cx| {
            this.update(cx, |s, cx| {
                s.context = None;
                cx.notify();
            })
        })
        .item(details)
        .separator();
    for item in items {
        let label = tr.t(item.label);
        let label = if item.via_details {
            format!("{label}…")
        } else {
            label
        };
        let (app, namespace) = (app.clone(), namespace.clone());
        menu = menu.item(
            MenuItem::new(label).on_select(move |_, cx| choose(&app, &namespace, Some(&item), cx)),
        );
    }
    Some(menu)
}
