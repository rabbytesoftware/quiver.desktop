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
| R4 | Reconnection and connection-changed events | `manager.rs`, `tauri_emitter.rs` | core-client::manager | partial | Real run (macOS, isolated HOME, own daemons on the dev socket): SIGKILL of the daemon changes nothing in the app (status stays `ready`, as in Tauri, which emitted status only from `start()`); a second daemon on the same socket makes `arrowStates` and the sidebar glyph converge to the new daemon's truth (running -> detached). Gap found and fixed: after a swap the per-daemon reads Home and followed collections were never re-read (the reconnect sentinel invalidated only arrow-detail and collection prefixes); failing test first, fix in `listeners/streams.rs`, tests in `listeners/tests/lifecycle.rs`. Home staleness is not observable against a real core here (no local core serves `/v0/home`); app-spawned daemon respawn and the box e2e were not run |
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
| U1 | App self-update from release page (not api.github.com), checksum, relaunch | release | partial | quiver-release unit tests (page, checksum, select); e2e `self_update_while_running`, `generic_outdated_badge` (PASS). Run locally on macOS from two version-stamped builds (`stable-1.0`, `stable-2.0`): `cargo xtask package macos` for both dmgs, `xtask verify-update` (ok; refuses same binary and bridge build), the app resolver against a local release page (url + sha256), a real daemon running the real ARROW.md darwin steps (fetch with checksum, stop, hdiutil+ditto, relaunch) into a scratch `$HOME/Applications`: old app stopped, installed binary equals the new dmg's, new process running, row `ready` at `stable-2.0`; a wrong checksum fails at the fetch step and leaves app and bundle untouched. Manifest was rewritten only for DEST, stop (`pkill -x`) and relaunch (`open -a`) so nothing outside the scratch dir was touched. Not run: the in-app Update click (no click injection on macOS), the github.com redirect (needs hosts/TLS, root), a real published release, signing and notarization, Windows and Linux packaging |
| U2 | Core update (Settings > Engine and arrow page), wrong checksum refused, rollback | release + state | done | e2e `update_core` (six tests incl. wrong checksum refused, rollback); second-update-after-swap pending bug A |
| U3 | Outdated badge from generic /v0/runtime broadcast | state | done | e2e `generic_outdated_badge` (PASS); quiver-state tests |
## Arrow apps
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| A1 | `arrow-app` scheme, host allow-list, CSP, script injection | arrow-host | done | arrow-host unit tests (scheme, hosts, csp, inject); e2e `arrow_apps` |
| A2 | WebSocket relay path built from registered host | arrow-host | done | `WsSession` + `WsRelay` unit tests; the shim reaches the host through `window.ipc` and the embedded page (decision log); not exercised end to end against a real WS app |
| A3 | 4-app cap, keep-alive, Reload, Stop, Details | arrow-host + app | done | arrow-host registry tests; e2e `arrow_apps` (4-app cap, Reload, Stop: PASS) |
| A4 | Sandbox/isolation (no daemon base leaked, header stripping) | arrow-host | partial | arrow-host handler tests; e2e checks daemon-side isolation and header stripping. Arrow apps are a wry child webview, not an iframe, so there is no sandbox attribute to observe. Opening one needs a sidebar click plus Start, which cannot be injected on macOS (xdotool path is X11 only), so in-page claims stay unobserved here; a probe arrow installed and ran on a real daemon but the app never loaded its page. Open finding from code reading, unverified at runtime: `with_ipc_handler` in `embedded/pages.rs` forwards shim messages without checking the sender document origin (the Tauri origin-checked hello has no native counterpart); proposed fix is to carry the document URI in `WindowToHost::Shim` and check it against the registered host |
## Console, menus, platform
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| C1 | Daemon console: log stream, exec, cancel, 403/400 refusals | core-client::console + state::console | done | core-client console tests, quiver-state console controller tests; e2e `console` (PASS) |
| M1 | macOS menu (no Undo/Redo, Close kept) | platform::menu + app | done | platform::menu tests; `crates/quiver-app/src/menu.rs`; macOS not run |
| W1 | Window 1200x800, min 800x600, title Quiver | app | done | quiver-platform window spec; window opens at 1200x800 (screenshots) |
| W2 | No tray, no deep links, no single-instance (none exist today) | - | done | none existed; none added |
## Features (src/features)
sidebar, shell, home, recommended, search, library, collections, collection, arrow-details, arrow-app, console, remote, settings (incl. automatic arrow registration switch for Fletcher) are all built (state + view + e2e ids; screenshots in `docker/native/evidence/final-*`). Each ported as state logic in `quiver-state::<feature>` and a view in `quiver-app/src/views/<feature>`; i18n in `quiver-state::i18n`.
