# Decision log

Short, dated entries for choices made during the Tauri -> GPUI migration.

## 2026-11-09 — Linux render spike (gpui 0.2.2, amd64)

- **Result: works.** A "Hello, GPUI" window built in an Ubuntu 24.04 **amd64** container
  (`docker/native/Dockerfile`), rendered under Xvfb 1280x800 + openbox + Mesa lavapipe, and was
  captured with `import -window root`. Evidence: `docker/native/evidence/gpui-spike-amd64.png`.
- Needs a window manager (openbox) or the capture is black; needs
  `ZED_ALLOW_EMULATED_GPU=1`, `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, no `WAYLAND_DISPLAY`.
- **`xattr 0.2.3` fails to compile on amd64 glibc too** (`ENOATTR` missing from libc). Fix: vendor the
  crate under `vendor/xattr`, replace `::libc::ENOATTR` with `::libc::ENODATA`, and
  `[patch.crates-io] xattr = { path = "vendor/xattr" }`. To re-check on each gpui bump whether a newer
  `xattr` makes the patch unnecessary.
- Cold GPUI release build: ~4.5 min on 4 cores. `target/` and the cargo registry live in Docker volumes.
- Not yet verified: input events via xdotool, Wayland, macOS, Windows.

## 2026-11-09 — Bug found: desktop self-registers on remote connections

`src/lib/core-store/listeners/self-announce.ts` (`announceSelf`) registers quiver.desktop in the Core
catalog (`POST /v0/arrow/:ns`, then `adopt`) on every ready connection. `listeners/index.ts` calls it at
`adoptRunningCore` and on the `ready` core-status event with no check of the connection kind, so a
remote daemon is registered into too. The native app gates it on the local sidecar connection only.

## 2026-11-09 — Stack pin: gpui 0.2.2 + gpui-component 0.5.1

`gpui-component` 0.9.1 (latest) depends on `gpui-pre 0.3.12`, a different fork that was not spiked.
0.5.1 depends on `gpui 0.2.2`, the version proven to render in the container, so both are pinned with `=`.

## 2026-11-09 — Arrow apps: separate WebView process (`quiver-arrow-window`)

- `gpui-component`'s `webview` feature (wry child view) and `gpui-wry` draw a native child view over the
  window; on Linux wry needs GTK (GPUI is not GTK), and on macOS tao/wry need the main thread GPUI owns.
  An embedded child view is therefore not viable on all three platforms.
- **Decision: every arrow app opens in its own native window**, hosted by a small helper binary
  `quiver-arrow-window` (tao + wry), one process per open arrow app, speaking JSON lines on stdin/stdout
  (`quiver_arrow_host::{HostToWindow, WindowToHost}`). The helper registers the `arrow-app` custom protocol
  and forwards each request to the GPUI host, which answers through `quiver-arrow-host` (CSP, host
  allow-list, shim injection, ws scoping: unchanged rules). The helper holds no daemon credentials.
  The GPUI binary links no GTK/WebKit; the helper does.
- Same header and Stop behaviour: the header (name, status, Reload, Details, Stop) lives in the GPUI app's
  "Arrow app" pane; Stop/Reload send `Close`/`Reload` to the helper. The window closes with its host
  (stdin EOF => close), so no orphaned windows.
- **Verified on Linux/amd64**: `scripts/arrow-window-spike.sh` serves a page through the IPC and renders it
  (`docker/native/evidence/arrow-window-spike.png`). macOS and Windows: not run here (compile-only via CI).
- Limit: the arrow app is a separate OS window, not embedded in the main window's pane as in the Tauri app.

## 2026-11-09 — Build facts found while porting

- The Rust release code only resolves the app's own release asset (page scrape, no api.github.com). The
  app self-update, Core update and rollback are ARROW.md lifecycles run by quiver.core; the desktop supplies
  `QUIVER_RELEASE_ASSET_URL`/`QUIVER_RELEASE_CHECKSUM` and the UI drives them over the Core API
  (`quiver-state::arrow_details`, `settings`). Fetching the sidecar Core binary and resolving
  `core-version.local`/channels happen at BUILD time (`scripts/*.mjs`, Makefile); `xtask` takes that over.
- No deep links, tray, single-instance or window-state persistence exist in the Tauri app: none ported.
- New crate `quiver-e2e` (bridge protocol/server/client/MCP/scenarios) is outside the prescribed layout:
  the bridge's wire types must be shared by the app (feature `e2e-bridge`) and the test driver.

## 2026-11-09 — Installed binary names stay as the Tauri release had them

The updater and installer contracts are text in the shipped `ARROW.md`, not code: `pkill -x quiverdesktop` /
`taskkill /IM quiverdesktop.exe`, the Linux/Windows/macOS detect commands, `%LOCALAPPDATA%\Quiver\uninstall.exe`
and the sidecar lookup (`quiver[.exe]` next to the running executable, `quiver-core-client`
`LocalConfig::bundled_next_to_current_exe`). So `cargo xtask package` installs the GPUI app as
`quiverdesktop[.exe]`, the quiver.core sidecar as `quiver[.exe]` and the arrow helper as
`quiver-arrow-window[.exe]`, all in one directory, whatever the cargo bin is called. Consequence for dev:
the cargo bin `quiver` of `quiver-app` cannot have the sidecar copied beside it under the same name, so the
`[[bin]]` should be renamed `quiverdesktop` (then `make dev` drops the sidecar into `target/debug/quiver`).
Asset names stay selectable by `quiver-release`: `Quiver_<v>_<arch>.AppImage`, `Quiver_<v>_universal.dmg`,
`Quiver_<v>_x64-setup.exe`, plus `checksums.txt` (sha256sum format).
