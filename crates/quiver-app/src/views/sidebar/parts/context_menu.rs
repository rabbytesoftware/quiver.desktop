//! The row context menu. Its items mirror the details page's action set
//! (`quiver_state::sidebar::arrow_menu_items`); an action that needs input opens the details page.

use gpui::{Context, IntoElement};
use quiver_state::arrow_details::{ArrowActionKind, ArrowFacts, RunState};
use quiver_state::namespace::bare_namespace;
use quiver_state::queries::mutations::{Mutations, Variables};
use quiver_state::sidebar::{arrow_menu_items, ArrowMenuItem};
use quiver_ui::{ContextMenu, MenuItem};

use super::super::Sidebar;
use crate::bridge::spawn_detached;
use crate::model::{AppModel, Route};

fn run(
    model: &AppModel,
    namespace: String,
    item: &ArrowMenuItem,
) {
    if item.via_details {
        return;
    }
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

pub fn context_menu(
    sidebar: &Sidebar,
    cx: &mut Context<Sidebar>,
) -> Option<impl IntoElement> {
    let (namespace, position) = sidebar.context.clone()?;
    let app = sidebar.app.clone();
    let entry = app
        .read(cx)
        .services
        .arrows
        .entries()
        .into_iter()
        .find(|a| bare_namespace(&a.namespace) == namespace)?;
    let facts = ArrowFacts {
        namespace: namespace.clone(),
        name: entry.name.clone(),
        state: RunState::from(entry.state),
        user_installed: true,
        resolved_ref: entry.version.clone(),
        ..ArrowFacts::default()
    };
    let this = cx.entity();
    let mut menu = ContextMenu::new("sidebar-row-menu", position)
        .probes("sidebar.menu")
        .on_dismiss({
            let this = this.clone();
            move |_, cx| {
                this.update(cx, |s, cx| {
                    s.context = None;
                    cx.notify();
                })
            }
        });
    for item in arrow_menu_items(&facts) {
        let label = app.read(cx).t(item.label);
        let (app, namespace) = (app.clone(), namespace.clone());
        menu = menu.item(MenuItem::new(label).on_select(move |_, cx| {
            if item.via_details {
                let route = Route::Arrow {
                    namespace: namespace.clone(),
                };
                app.update(cx, |m, cx| m.navigate(route, cx));
            } else {
                run(app.read(cx), namespace.clone(), &item);
            }
        }));
    }
    Some(menu)
}
