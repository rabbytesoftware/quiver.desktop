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
