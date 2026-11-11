//! Right-click on a sidebar row opens the OS's own context menu, as the Tauri app's did (GTK in the box):
//! "Open details", a separator, then what the arrow's page offers. Driven for real: the right-click and the keys
//! go through X, the menu is GTK's (not drawn by the app: the in-app fallback never shows), and the actions it
//! starts run on the real quiver.core.

use crate::support::app::App;
use crate::support::arrows::wait_arrow;
use crate::support::daemon::Daemon;
use crate::support::env::Env;
use crate::support::home::Home;
use crate::support::lifecycle::register;
use serde_json::json;

const SLOW: &str = "github.com/rabbytesoftware/e2e-slow";

/// Right-clicks `row`, waits for the `nth` native menu to be up and returns its labels ("-" is a separator).
async fn open_menu(
    app: &mut App,
    row: &str,
    nth: usize,
) -> Vec<String> {
    if let Err(e) = app.bridge.right_click(row).await {
        app.fail(&format!("right-click {row}: {e}")).await;
    }
    app.signal(&format!("menu.native.open.{nth}")).await;
    let state = app.bridge.state().await.expect("state.dump");
    let menu = &state["nativeMenu"];
    assert_eq!(menu["shown"], nth, "{menu}");
    assert_eq!(menu["open"], true, "{menu}");
    // The OS menu, not the in-app one.
    assert!(app.find("sidebar.menu").await.is_none());
    let items: Vec<String> = menu["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i.as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        items.first().map(String::as_str),
        Some("Open details"),
        "{items:?}"
    );
    assert_eq!(items.get(1).map(String::as_str), Some("-"), "{items:?}");
    items
}

/// Chooses `label` with the keyboard (the menu has it; GTK skips the separator) and waits for the menu to close.
async fn choose(
    app: &mut App,
    items: &[String],
    label: &str,
    nth: usize,
) {
    let position = items
        .iter()
        .filter(|i| *i != "-")
        .position(|i| i == label)
        .unwrap_or_else(|| panic!("{label} is not in the menu: {items:?}"));
    let keys = format!("{} Return", vec!["Down"; position + 1].join(" "));
    if let Err(e) = app.bridge.os_key(&keys).await {
        app.fail(&format!("choose {label}: {e}")).await;
    }
    app.signal(&format!("menu.native.closed.{nth}")).await;
}

#[tokio::test]
async fn the_row_menu_is_the_os_menu_and_runs_what_the_page_offers() {
    let Some(env) = Env::load("row_menu") else {
        return;
    };
    let home = Home::fresh(&env, "row_menu");
    let mut app = App::launch(&env, &home).await;
    app.wait_ready().await;
    let core = Daemon::unix(home.socket());
    let slow = register(&core, SLOW).await;
    let row = format!("sidebar.row.{SLOW}");
    app.node(&row).await;

    // Home's Library shelf: the title on the left, "View 1 arrow" at the right end of the hairline.
    let link = app.node("home.library.view-all").await;
    let shelf = app.node("section.home-library").await;
    // One tile on the shelf: the row still spans the page (a row as wide as its content would pass the first check).
    let main = app.node("screen.home").await;
    if link.x + link.w < shelf.x + shelf.w - 1. || shelf.w < main.w - 120. {
        app.fail(&format!(
            "the Library link {link:?} does not end a full-width row {shelf:?} in {main:?}"
        ))
        .await;
    }

    // Hovering a tile lifts its art 30px and uncovers the strip under it (the old tiles' motion, kept by the
    // boards); moving away lets it fall back.
    let tile = format!("home.library.tile.{SLOW}");
    app.node(&tile).await;
    if let Err(e) = app.bridge.hover(&tile).await {
        app.fail(&format!("hover {tile}: {e}")).await;
    }
    app.matching(json!({"id": tile, "state_contains": "lifted"}))
        .await;
    app.shot("tile-lifted").await;
    if let Err(e) = app.bridge.hover("home.library.view-all").await {
        app.fail(&format!("hover away: {e}")).await;
    }
    app.gone_matching(json!({"id": tile, "state_contains": "lifted"}))
        .await;

    // Escape closes it and nothing happens.
    open_menu(&mut app, &row, 1).await;
    app.shot("menu-open").await;
    if let Err(e) = app.bridge.os_key("Escape").await {
        app.fail(&format!("escape: {e}")).await;
    }
    app.signal("menu.native.closed.1").await;
    assert!(app.find("screen.home").await.is_some());

    // "Open details" opens the arrow's page.
    let items = open_menu(&mut app, &row, 2).await;
    choose(&mut app, &items, "Open details", 2).await;
    app.node("screen.arrow").await;

    // Not installed: Install, run from the menu without the page's button.
    let items = open_menu(&mut app, &row, 3).await;
    assert!(!items.iter().any(|i| i == "Start"), "{items:?}");
    choose(&mut app, &items, "Install", 3).await;
    wait_arrow(&core, &slow, "installed from the menu", |a| {
        a.state == "ready"
    })
    .await;

    // Installed: Start; running: Stop.
    let items = open_menu(&mut app, &row, 4).await;
    choose(&mut app, &items, "Start", 4).await;
    wait_arrow(&core, &slow, "started from the menu", |a| {
        a.state == "running"
    })
    .await;
    let items = open_menu(&mut app, &row, 5).await;
    assert!(!items.iter().any(|i| i == "Start"), "{items:?}");
    choose(&mut app, &items, "Stop", 5).await;
    wait_arrow(&core, &slow, "stopped from the menu", |a| {
        a.state == "ready"
    })
    .await;
}
