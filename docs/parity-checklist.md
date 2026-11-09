# Parity checklist (Tauri -> GPUI)

Every row is a behaviour of the Tauri app that the native app must keep. `Status` is filled as work lands;
`Evidence` names the test or manual check that proves it. `-` means not yet done. Rows marked **FIX** are the
one intentional deviation.

## Local sidecar
| # | Behaviour | Source | Native home | Status | Evidence |
|---|---|---|---|---|---|
| L1 | Locate/fetch the Core binary (resource sidecar, `core-version.local`) | `connection/local/sidecar.rs` | core-client::local | - | - |
| L2 | Pinned core version and channels (`nightly-rolling`, stable) | `scripts/resolve-core-version.mjs`, sidecar | core-client::local, release | - | - |
| L3 | Start, supervise, stop the sidecar; no orphan on quit | `sidecar.rs`, `lib.rs` run callback | core-client::local, app shutdown | - | - |
| L4 | Raise RLIMIT_NOFILE at start-up and report it | `fdlimit/` | platform::fdlimit | - | - |
## Remote connections
| # | Behaviour | Source | Native home | Status | Evidence |
|---|---|---|---|---|---|
| R1 | Health probe before pairing (`/v0/health`, tokenless) | `commands/connection.rs` | core-client::remote | - | - |
| R2 | Add (name, url, pairing code), remove, switch, rename, list | `manager.rs` | core-client::manager | - | - |
| R3 | Transports: unix, pipe, tcp/http, stream; per-request transport resolution | `transport/` | core-client::transport | - | - |
| R4 | Reconnection and connection-changed events | `manager.rs`, `tauri_emitter.rs` | core-client::manager | - | - |
| R5 | WebSocket bridge (descriptor-leak regression) | `bridge.rs` | core-client::bridge | - | - |
| R6 | **FIX** Desktop self-announce (register/adopt/forget) runs on the LOCAL connection only | `self-announce.ts` | core-client::self_announce | - | - |
## Secrets and settings
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| S1 | Keyring secrets (same service/user names) | platform::secrets | - | - |
| S2 | Persisted settings/connections (same bundle id, same files) | platform::settings | - | - |
## Updates
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| U1 | App self-update from release page (not api.github.com), checksum, relaunch | release | - | - |
| U2 | Core update (Settings > Engine and arrow page), wrong checksum refused, rollback | release + state | - | - |
| U3 | Outdated badge from generic /v0/runtime broadcast | state | - | - |
## Arrow apps
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| A1 | `arrow-app` scheme, host allow-list, CSP, script injection | arrow-host | - | - |
| A2 | WebSocket relay path built from registered host | arrow-host | - | - |
| A3 | 4-app cap, keep-alive, Reload, Stop, Details | arrow-host + app | - | - |
| A4 | Sandbox/isolation (no daemon base leaked, header stripping) | arrow-host | - | - |
## Console, menus, platform
| # | Behaviour | Native home | Status | Evidence |
|---|---|---|---|---|
| C1 | Daemon console: log stream, exec, cancel, 403/400 refusals | core-client::console + state::console | - | - |
| M1 | macOS menu (no Undo/Redo, Close kept) | platform::menu + app | - | - |
| W1 | Window 1200x800, min 800x600, title Quiver | app | - | - |
| W2 | No tray, no deep links, no single-instance (none exist today) | - | - | - |
## Features (src/features)
sidebar, shell, home, recommended, search, library, collections, collection, arrow-details, arrow-app, console, remote, settings (incl. automatic arrow registration switch for Fletcher): each ported as state logic in `quiver-state::<feature>` and a view in `quiver-app/src/views/<feature>`; i18n in `quiver-state::i18n`.
