# Parity checklist (Tauri -> GPUI)

Every row is a behaviour of the Tauri app that the native app must keep. `Status` is filled as work lands;
`Evidence` names the test or manual check that proves it. `-` means not yet done. Rows marked **FIX** are the
one intentional deviation.

## Local sidecar
| # | Behaviour | Source | Native home | Status | Evidence |
|---|---|---|---|---|---|
| L1 | Locate/fetch the Core binary (resource sidecar, `core-version.local`) | `connection/local/sidecar.rs` | core-client::local | done | core-client::local unit tests; e2e `bootstrap` (box: PASS) |
| L2 | Pinned core version and channels (`nightly-rolling`, stable) | `scripts/resolve-core-version.mjs`, sidecar | core-client::local, release | done | xtask `resolve-core-version`/`fetch-sidecar` tests (ported from the .mjs tests); not run against GitHub |
| L3 | Start, supervise, stop the sidecar; no orphan on quit | `sidecar.rs`, `lib.rs` run callback | core-client::local, app shutdown | done | core-client::local tests; e2e `update_core::closing_the_window_stops_the_daemon_and_leaves_no_quiver_process` (PASS); `tests/lifecycle.rs` |
| L4 | Raise RLIMIT_NOFILE at start-up and report it | `fdlimit/` | platform::fdlimit | done | platform::fdlimit tests; raised first in `AppServices::start` |
## Remote connections
| # | Behaviour | Source | Native home | Status | Evidence |
|---|---|---|---|---|---|
| R1 | Health probe before pairing (`/v0/health`, tokenless) | `commands/connection.rs` | core-client::remote | done | core-client `tests/remote_connection.rs` (ported), app `tests/remote_connections.rs` |
| R2 | Add (name, url, pairing code), remove, switch, rename, list | `manager.rs` | core-client::manager | done | core-client manager tests, app `tests/remote_connections.rs`, e2e `auto_register` (adds a remote through the UI; PASS) |
| R3 | Transports: unix, pipe, tcp/http, stream; per-request transport resolution | `transport/` | core-client::transport | done | core-client::transport unit tests; app `tests/core_api.rs` (per-call transport resolution) |
| R4 | Reconnection and connection-changed events | `manager.rs`, `tauri_emitter.rs` | core-client::manager | partial | manager/events tests pass; after a daemon swap the app kept stale runtime state (bug A), fix in flight, see report |
| R5 | WebSocket bridge (descriptor-leak regression) | `bridge.rs` | core-client::bridge | done | core-client::bridge tests incl. descriptor-leak regression |
| R6 | **FIX** Desktop self-announce (register/adopt/forget) runs on the LOCAL connection only | `self-announce.ts` | core-client::self_announce | done | **FIX** core-client self_announce tests; app `tests/auto_register.rs` (remote never writes, local does); e2e `auto_register` (real daemons, PASS) |
## Secrets and settings
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| S1 | Keyring secrets (same service/user names) | platform::secrets | done | platform::secrets tests; same service `quiver.desktop`; e2e box uses a Secret Service (needed, as in the Tauri app) |
| S2 | Persisted settings/connections (same bundle id, same files) | platform::settings | done | platform::settings tests (same file names under the bundle id data dir) |
## Updates
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| U1 | App self-update from release page (not api.github.com), checksum, relaunch | release | partial | quiver-release unit tests (page, checksum, select); e2e `self_update_while_running`, `generic_outdated_badge` (PASS). Update from a previous *release* is file-level only (`xtask verify-update`), not run end to end |
| U2 | Core update (Settings > Engine and arrow page), wrong checksum refused, rollback | release + state | done | e2e `update_core` (six tests incl. wrong checksum refused, rollback); second-update-after-swap pending bug A |
| U3 | Outdated badge from generic /v0/runtime broadcast | state | done | e2e `generic_outdated_badge` (PASS); quiver-state tests |
## Arrow apps
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| A1 | `arrow-app` scheme, host allow-list, CSP, script injection | arrow-host | done | arrow-host unit tests (scheme, hosts, csp, inject); e2e `arrow_apps` |
| A2 | WebSocket relay path built from registered host | arrow-host | partial | `WsRelay` implemented and unit-tested; the helper IPC carries no WebSocket frames yet, so the relay is not reachable from the window (see decision log) |
| A3 | 4-app cap, keep-alive, Reload, Stop, Details | arrow-host + app | done | arrow-host registry tests; e2e `arrow_apps` (4-app cap, Reload, Stop: PASS) |
| A4 | Sandbox/isolation (no daemon base leaked, header stripping) | arrow-host | partial | arrow-host handler tests; e2e checks daemon-side isolation and header stripping; in-page DOM claims (sandbox attr, Tauri globals) cannot be observed from the bridge |
## Console, menus, platform
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| C1 | Daemon console: log stream, exec, cancel, 403/400 refusals | core-client::console + state::console | done | core-client console tests, quiver-state console controller tests; e2e `console` (PASS) |
| M1 | macOS menu (no Undo/Redo, Close kept) | platform::menu + app | done | platform::menu tests; `crates/quiver-app/src/menu.rs`; macOS not run |
| W1 | Window 1200x800, min 800x600, title Quiver | app | done | quiver-platform window spec; window opens at 1200x800 (screenshots) |
| W2 | No tray, no deep links, no single-instance (none exist today) | - | done | none existed; none added |
## Features (src/features)
sidebar, shell, home, recommended, search, library, collections, collection, arrow-details, arrow-app, console, remote, settings (incl. automatic arrow registration switch for Fletcher) are all built (state + view + e2e ids; screenshots in `docker/native/evidence/final-*`). Each ported as state logic in `quiver-state::<feature>` and a view in `quiver-app/src/views/<feature>`; i18n in `quiver-state::i18n`.
