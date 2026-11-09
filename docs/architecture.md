# Architecture

Quiver Desktop is a native Rust application on GPUI. Dependencies point one way; `cargo xtask check-deps`
enforces it (CI runs it) and `cargo xtask check-layout` enforces the file-size and naming rules.

## Crate graph

```
quiver-theme      (tokens, no deps)
quiver-platform   (paths, settings files, keyring, fd limit, menu/window specs)
quiver-core-client(transports, sidecar, remote, ConnectionManager, proxy, WS bridge, console exec, self-announce)
quiver-arrow-host (arrow-app:// protocol, hosts, CSP, injection, registry, helper IPC)  -> core-client
quiver-release    (release resolution, build stamp)                                    -> core-client, platform
quiver-state      (stores, queries, listeners, feature view-models, i18n)              -> core-client, release, arrow-host, platform
quiver-ui         (GPUI Coss component kit + gallery)                                  -> theme, gpui
quiver-e2e        (debug control bridge protocol/server/client/MCP)                    -> no app crates
quiver-app        (views, services glue, binaries)                                     -> everything above
xtask             (check-deps, check-layout, gate)
```

The no-GPUI crates (`theme`, `platform`, `core-client`, `arrow-host`, `release`, `state`) may not depend on
`gpui` or `tauri`. `quiver-app` is the only crate that wires them to the UI.

## Rules

- Files under 300 lines, hard limit 500. No `utils`/`helpers`/`common`/`misc` modules.
- `thiserror` per library crate; `anyhow` only in `quiver-app` and `xtask`.
- No `unwrap`/`expect` outside tests (workspace clippy lints).
- Views hold no business logic: decisions live in `quiver-state`.

## Module mapping (Tauri/React -> Rust)

| Old | New |
| --- | --- |
| `src-tauri/src/connection/*` (local, remote, manager, proxy, bridge, transports) | `quiver-core-client` |
| `src-tauri/src/console.rs` | `quiver-core-client::console`, `quiver-state::console` |
| `src-tauri/src/arrow_app/*` | `quiver-arrow-host` + `quiver-app/src/services/arrow_windows` + `bin/arrow_window` |
| `src-tauri/src/release/*`, `build.rs` | `quiver-release` |
| `src-tauri/src/fdlimit`, `menu.rs`, window config | `quiver-platform` |
| `src-tauri/src/lib.rs` (setup, shutdown) | `quiver-app/src/services/lifecycle.rs`, `main.rs` |
| `src/lib/core-store/*`, `src/lib/*` data layer | `quiver-state` (`domain`, `dto`, `store`, `listeners`, `queries`, `persistence`) |
| `src/index.css` tokens | `quiver-theme` |
| `src/components/ui/*` (Coss) | `quiver-ui` |
| `src/features/<name>` | `quiver-state/src/<name>` (model) + `quiver-app/src/views/<name>` (view) |
| `src/arrow-shim/shim.js` | `quiver-arrow-host/assets/shim.js` |
| `e2e/*` (WebdriverIO) | `quiver-e2e` bridge + `quiver-e2e/tests` |

Features: arrow-app, arrow-details, collection, collections, console, home, library, recommended, remote,
search, settings, shell, sidebar.

## Arrow apps

Arrow apps render in a separate OS window served by the `quiver-arrow-window` helper (tao + wry), so the
GPUI binary links no GTK/WebKit and no main-thread is shared. See `docs/decision-log.md`.

## E2E

Feature `e2e-bridge` (never in release artifacts) serves a JSON-RPC bridge; `quiver_mcp` exposes it as an MCP
server. See `docs/e2e.md`.
