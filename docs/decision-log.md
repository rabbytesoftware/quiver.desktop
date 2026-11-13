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

## 2026-11-10 — Arrow apps: child webview embedded in the main window (supersedes the separate-process decision)

An earlier slice put every arrow app in its own OS window (helper binary `quiver-arrow-window`). That was a
regression against the design ("Arrow app — AFTER": the app's page fills the pane under the Quiver header), so
it was removed. Arrow apps now render in a wry **child webview** placed over the pane body of the main window.

- `quiver-app/src/services/arrow_windows/embedded/`: `EmbeddedPages` implements `HelperLauncher`, so the
  supervisor (`ArrowWindows`: 4-app cap, eviction, reload, stop, prune) and its channel protocol
  (`HostToWindow`/`WindowToHost`) are unchanged; only the far end of the channel is now an in-process webview.
  `quiver-arrow-host` rules (CSP, host allow-list, shim injection, ws scoping) are untouched.
- Placement: the pane body reports its rectangle from a gpui `canvas` prepaint to `ArrowWindows::set_bounds`
  (deduplicated); `Visible` shows/hides pages, so only the visible app's page is on screen.
- **Linux**: wry's X11 backend needs GTK and an Xlib parent. GPUI's `X11Window::window_handle()` is
  `unimplemented!()`, so the main window's XID is found with x11-dl (the biggest window of this process via
  `_NET_WM_PID`), lazily on the page thread (resolving it during window creation left the first frame
  unpainted). A dedicated `quiver-gtk` thread runs `gtk::init` + `gtk::main`; jobs reach it through a glib channel
  (deprecated API, `#[allow(deprecated)]` with a comment). On a Wayland session `display::prefer_x11()` drops
  `WAYLAND_DISPLAY` when `DISPLAY` is set so the app runs through XWayland (a child X window cannot live in a
  Wayland surface).
- **macOS / Windows**: webviews are created on the UI thread from a GPUI foreground task (`thread_ui.rs`) with the
  window's own handle. **Written but never compiled-and-run on those OSes.**
- Overlay limit: a native view paints above GPUI's pixels, so the page hides (`set_covered`) while the header's
  actions menu is open. Dialogs live on the arrow's own page, not over the pane, so they are not affected.
- WebSocket relay (parity A2): `WsSession` (quiver-arrow-host) handles `ws-open/send/close` from the shim; the shim
  talks through `window.ipc.postMessage`, the host answers with `evaluate_script("__arrowShimReceive(..)")`.
- GPUI links GTK/WebKit on Linux again (the earlier "no GTK in the GPUI binary" benefit is gone).

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
`quiverdesktop[.exe]` and the quiver.core sidecar as `quiver[.exe]`, both in one directory, whatever the cargo bin is called. Consequence for dev:
the cargo bin `quiver` of `quiver-app` cannot have the sidecar copied beside it under the same name, so the
`[[bin]]` should be renamed `quiverdesktop` (then `make dev` drops the sidecar into `target/debug/quiver`).
Asset names stay selectable by `quiver-release`: `Quiver_<v>_<arch>.AppImage`, `Quiver_<v>_universal.dmg`,
`Quiver_<v>_x64-setup.exe`, plus `checksums.txt` (sha256sum format).

## Late decisions (integration, gate, e2e)

- **Pointer input for the e2e bridge goes through the OS.** GPUI 0.2.2's `Window::dispatch_event` returns a private
  type and cannot be called from outside the crate. Keyboard input uses `Window::dispatch_keystroke`; mouse and wheel
  events are injected with `xdotool` (X11), so they also exercise the platform input path. macOS and Windows have no
  pointer driver yet (the bridge reports a clear error there).
- **Screenshots** shell out to ImageMagick `import` (Linux), `screencapture` (macOS) or PowerShell (Windows): GPUI
  cannot read back its own frame buffer.
- **cargo-deny / cargo-audit.** Every ignored advisory is an "unmaintained" notice for a crate that arrives only through
  GPUI, wry or the keyring stack (see `deny.toml`). `cargo audit` still prints "unsound" warnings for `glib 0.18` and
  `rand 0.7` (transitive, via wry/GTK and GPUI); they are warnings, not vulnerabilities, and are re-checked on every
  GPUI bump. `NCSA` is allowed for `libfuzzer-sys` (build-time only).
- **Secret Service is required on Linux**, exactly as the Tauri app required it (the `keyring` crate). The e2e box starts
  `dbus` + `gnome-keyring-daemon`. No file-based fallback was added: that is a security decision for the owner.
- **Tokio context on the UI thread.** `app::run` enters the services' runtime for the whole GPUI thread, and the state
  streams capture a runtime handle when opened (a bare `tokio::spawn` from the UI thread panicked on Update).
- **Daemon reaping.** The sidecar supervisor reaps the daemon child on exit even while a supervised arrow keeps the
  stdio pipes open; otherwise quiver.core's `kill(pid, 0)` liveness probe sees a zombie for the full 90 s swap timeout.
- **After a reconnect the runtime overlay is dropped before the re-read** (`ArrowStore::forget_live_state`): the old React
  store kept it, which was invisible there but left "updating"/"running" stale after a Core swap natively.
- **Installed names** follow `ARROW.md` and the sidecar lookup: `quiverdesktop[.exe]` and the
  quiver.core sidecar as `quiver[.exe]`.
- **Removal of the Tauri tree** is its own commit; the old sources are reachable in git history (`git show b599652^:src-tauri/...`).
