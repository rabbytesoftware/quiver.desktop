# Writing a view

A view draws what `quiver-state` decided. It owns GPUI state (scroll, open menus, a text field) and nothing
else: decisions live in `quiver_state::<feature>` (add a tested function there if you need a new one), colours
come from `cx.quiver().palette`, components from `quiver_ui`, strings from the translator.

## Layout

```
views/<feature>/mod.rs        the view struct + `impl Render`, short
views/<feature>/parts/*.rs    one file per section/row/state (< 300 lines each)
views/tiles/                  THE arrow tile mapping (`tiles::tile(screen, &TileInput, TileSize, &app, cx)`)
```

Register the module in `views/mod.rs`, then add the route arm in `shell/outlet.rs` (`page_for`).

## The model

`crate::model::{app_model, AppModel, Route, navigate, open_modal, close_modal, handle}`.

```rust
let app: Entity<AppModel> = app_model(cx);          // or pass it into your view's `new`
let m = app.read(cx);
m.services      // Arc<AppServices>: arrows, status, connections, queries, api, backend, console, ...
m.t(KEY)        // translator: `quiver_state::i18n::en::<ns>::KEY`; plurals: m.translator.t_count(KEY, n)
m.route()       // &Route (Home, Recommended, Search{query}, Library, Collections, Collection{id},
                //          Arrow{namespace}, ArrowApp{namespace}, Remote, Settings{tab})
m.home          // Query<Home> (`GET /v0/home`, kept fresh by a feed)
m.show_self_components, m.shell (rail width/side), m.theme, m.console_open (dock, not a route)
```

## Reading state

Stores are `quiver_state::observable::Store<T>`. Subscribe once in `new`, read in `render`:

```rust
observe_store(cx, services.arrows.store());          // crate::bridge: re-render on every change
let (arrows, catalog) = services.arrows.store().read(|s| (s.arrows.values().cloned().collect::<Vec<_>>(), s.catalog));
```

`QueryState<T> { data, error, loading }` (`Query::snapshot()`) is the shape of every server read: render
`loading && data.is_none()` as a skeleton, `error && data.is_none()` as an error state, never blank.

## Dispatching an action

Async work runs on the tokio runtime, the result comes back on the UI thread, inside your view:

```rust
spawn_action(cx, m.handle(), async move { mutations.stop(&ns, &vars).await },
    |this: &mut MyView, result, cx| { this.error = result.err().map(|e| e.to_string()); cx.notify(); });
spawn_detached(m.handle(), async move { ... });       // outcome reaches views through the stores
```

Build mutations with `Mutations::new(m.services.api.clone(), m.services.queries.clone())`. Navigate with
`navigate(cx, Route::Arrow { namespace })` or `app.update(cx, |m, cx| m.navigate(route, cx))`;
`m.replace(route, cx)` replaces the current entry (typing in search); `m.back(cx)` / `m.forward(cx)`.

## Modals

A modal is its own view that renders `quiver_ui::Dialog` / `Sheet` with `.open(true)` and closes itself:

```rust
let view = cx.new(|cx| ArrowSettingsModal::new(app.clone(), ns, cx));
open_modal(cx, "arrow-settings", view.into());        // later: close_modal(cx, "arrow-settings")
// inside the modal: Dialog::new("arrow-settings").open(true).on_dismiss(|_, cx| close_modal(cx, "arrow-settings"))
```

The shell draws the stack in opening order (last on top). Opening an id twice replaces it.

## E2E ids

Every interactive element and every screen reports a stable id with `quiver_ui::inspect::probe(id, Kind,
label, enabled, state)`, added as the LAST child of the element it describes (the parent must be `relative`).
Convention `<screen>.<element>[.<key>]`, lower-case, dashes: `sidebar.search`, `sidebar.tab.home`,
`sidebar.row.<namespace>`, `home.tile.<namespace>`, `tile.<namespace>.start`, `home.popular.browse-all`.
Screens: `views::screen_probe("home")` as the last child of the page root (id `screen.home`). `state` carries
`"selected"`, `"checked"`, `"value=..."`. Ids are part of the contract with the e2e driver: never rename one
without telling it. The shell calls `inspect::begin_frame()` once per root render; views do not.
`quiver_ui` components that take ids: `Tile::probe`, `SearchInput::probes`, `SectionHeader::link`,
`ContextMenu::probes`; for `Button`/`Switch`/... add a `probe(..)` child yourself.
