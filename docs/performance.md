# Performance: native Quiver Desktop vs the old Tauri app

Measured with `cargo xtask perf` (`xtask/src/perf/`). Raw JSON (every run, medians, thread breakdowns) is in
`docker/native/evidence/perf/`.

## Read this first: what these numbers are not

Everything ran in the Linux perf box (`scripts/perf-box.sh`, the `quiver-native-dev` image): **Xvfb + openbox, software
rendering** (Mesa lavapipe/llvmpipe for GPUI's Vulkan; WebKitGTK with `WEBKIT_DISABLE_COMPOSITING_MODE=1`,
`WEBKIT_DISABLE_DMABUF_RENDERER=1`), **4 shared cores**, a container on a host where other agents were building and
testing at the same time (load average at the end of the runs: 0.5 to 3). There is no GPU. A full software-rendered GPUI
frame costs about 100 ms of CPU here; on a real GPU it is a fraction of a millisecond. So:

* Absolute times, CPU% and frame times are NOT what a desktop user would see. The native app pays for software
  Vulkan, the Tauri app for software WebKit; the two penalties are different and not comparable to each other.
* Memory (RSS/PSS), binary size and wake-up counts are more transferable than CPU and frame times.
* N=5 cold starts per variant (fresh HOME each, so quiver.core also starts cold; the page cache is warm).

## Method

| What | How |
| --- | --- |
| Builds | native: `[profile.release]` as shipped (LTO, opt-level s, stripped), plain binary (`native-release`); and the release profile with LTO off plus `--features e2e-bridge` (`native-bridge`, like the e2e box). Tauri: `bun run tauri build --no-bundle` from the same tree (`tauri`). quiver.core is the same stamped binary for all, started by the app as its sidecar (real daemon, no mocks). |
| First window | `xev -root` started before the app, MapNotify stamped on arrival, window resolved with `xdotool search --pid`. Same method for native and Tauri. |
| First painted | after the map, the window's client area is grabbed every 20 ms (PIL/xcb) until it holds more than a flat fill (at least 12 colours in a 120x80 thumbnail). It is a poll, but the same one for both apps. For native this is the first frame (a skeleton screen), for Tauri the first non-blank WebKit paint (React shell). |
| Bridge signals | `native-bridge` only: `wait.signal first-frame` and `core.ready`. The signals are latched, so each value is an upper bound if the signal was already raised; the `_waited_ms` fields show that (first-frame answered in under 1 ms, i.e. the bridge connected before the frame; core.ready waited 300 to 800 ms, so those are exact). |
| core socket | 10 ms probe of `~/.quiver/quiver.sock` until it accepts (a socket has no event). |
| Home | native-bridge: `core.ready` + `sidebar.list` painted. Tauri/native-release: max(core socket, first painted). |
| RSS | 10 s after Home. Sum of VmRSS of the app process and its helpers (Tauri: WebKitWebProcess + WebKitNetworkProcess), quiver.core excluded and reported separately. PSS (shared pages split) is reported too because WebKit's helpers share libraries with the main process. |
| Idle CPU and wake-ups | 30 s window after the settle time, no input, nothing polling the app: `/proc/<pid>/task/*/stat` ticks and `voluntary + nonvoluntary` context switches summed over every thread of every counted process, delta over the window. Wake-ups = context switches per second. |
| Frames | `native-bridge`: `app.ping` frame counter (root renders = window draws) before and after the idle window. |
| Scrolling | `native-bridge --demo`, real wheel events through `xdotool`; single notch: ms from the notch to the next frame; burst: frames painted and elapsed for 180 (8 ms apart) and 60 (40 ms apart) notches. |

Run it: `scripts/perf-box.sh cargo xtask perf build` then `... cargo xtask perf startup` and `... cargo xtask perf scroll`.

## Results: startup, memory, idle (median [min..max], N=5)

| | native (release) | native (bridge build) | Tauri |
| --- | --- | --- | --- |
| Window mapped (ms) | 120 [116..125] | 127 [119..131] | 70 [68..144] |
| First painted frame, pixel method (ms) | 1182 [1163..1206] | 1171 [1154..1235] | 1067 [605..1230] |
| `first-frame` bridge signal (ms) | n/a | 1202 [1179..1262] | n/a |
| `core.ready` bridge signal (ms) | n/a | 1787 [1532..2405] | n/a |
| quiver.core socket accepting (ms) | 21 | 22 | 149 [131..254] |
| RSS app + helpers (MB) | 131 [130..132] | 137 [136..138] | 300 [297..301] |
| PSS app + helpers (MB) | 117 | 123 | 193 |
| quiver.core RSS (MB, same in both) | 110 | 116 | 109 |
| Idle CPU, 30 s (% of one core) | 0.27 [0.27..0.87] | 1.07 [0.4..1.13] | 17.0 [16.9..17.2] |
| Idle wake-ups (/s) | 77 | 85 | 846 |
| Idle frames (/s) | n/a | 0.03 | n/a |
| quiver.core idle CPU / wake-ups | 1.5 % / 357 | 1.9 % / 416 | 1.6 % / 410 |
| Binary: app | 23.3 MB | 34.4 MB (LTO off, not shipped) | 8.8 MB (+ system WebKitGTK, not counted) |
| Binary: core sidecar | 56.5 MB | 56.5 MB | 56.5 MB |

> **Stale for the app binary row and arrow apps:** these figures were measured before arrow apps moved into an
> embedded webview (the GPUI binary now links GTK/WebKit on Linux, so the 23.3 MB app size and the idle RSS with no
> arrow open are not re-measured). The idle-CPU and wake-up figures are for the Home screen, which is unchanged.

Reading it honestly:

* **Memory and wake-ups**: native uses less than half of Tauri's RSS (131 vs 300 MB; 117 vs 193 MB PSS) and about a
  tenth of the wake-ups. The Tauri figure includes WebKit's two helper processes.
* **Idle CPU**: Tauri's 17 % is WebKit in software compositing mode re-rasterising (it also polls home while core
  says it is refreshing); native is under 1 %. This gap is inflated by the box (no GPU path for WebKit).
* **Startup**: roughly a tie at about one second to first paint, in the box. Native pays about a second of lavapipe
  pipeline setup before its first frame; the Tauri window maps earlier (70 ms) but paints after the WebKit process
  spawns and loads the page. The first Tauri start after boot was 6.4 s (cold disk), not in the table. The native
  app starts quiver.core earlier (socket up at 21 ms vs 149 ms).
* The app binary is 2.6x bigger than Tauri's because GPUI, fonts and the renderer are linked in; Tauri's WebKitGTK
  is a system library and is not in its 8.8 MB.
* The Tauri row has no frame counter or signals; no Tauri `core.ready` equivalent exists, so only the pixel method is
  comparable across the two apps.
* Remaining native wake-ups (about 62/s on the main thread) are GPUI 0.2.2's X11 backend: it runs a timer at the
  display refresh rate while the window is visible (`platform/linux/x11/client.rs`, `start_refresh_loop`) and only
  draws when something is dirty. It costs 0.27 % CPU and is not fixable from this repository without patching gpui.

## Idle redraw: found and fixed

Before: with a real daemon on Home, idle CPU was 7.6 % (native release) and 7.8 % (bridge), 0.37 frames/s. Cause:
core reports `refreshing: true` for the home snapshot, `start_home_feed` re-reads `GET /v0/home` every 3 s (a 1:1 port of
the Tauri refetch interval), and `Query::run` published `loading=true` and then the data on every poll, so each poll
repainted the whole window (about 200 ms of software rendering) even though the data was identical. Fix
(`quiver-state` `Query::run`, `Store::update_quiet`/`notify`): a refetch of data already on screen is silent and
publishes once, only if the result differs; the first load still always publishes. Regression tests:
`a_refetch_returning_the_same_data_wakes_nobody`, `the_first_load_publishes_even_when_it_fails_twice_the_same_way`.

| native release, Home, real core | before | after |
| --- | --- | --- |
| idle CPU (30 s, median) | 7.6 % [7.0..7.8] | 0.27 % [0.27..0.87] |
| idle wake-ups/s | 202 | 77 |
| idle frames/s (bridge build) | 0.37 | 0.03 |

(`startup-*-before.json` are the pre-fix builds, kept on the box that measured the fix.)

Animations: spinners (FlickerSpinner, Spinner, StatusGlyph Running, WavyLine) are `with_animation` elements that only
exist while a busy state is shown; none runs at Home with no busy arrows (0.03 frames/s above). While one is visible
GPUI redraws at the display rate: the unmodified `--demo` (several busy arrows in the sidebar) used about 300 % CPU
in this box. On a GPU that is cheap, but a software-rendered machine pays for it; FlickerSpinner only has 8 steps per
second, so throttling its redraw to 8 Hz would be a cheap future win (not done).

## Long lists and a huge README (demo, calm mode)

`QUIVER_DEMO_ARROWS=<n>` adds n generated arrows (sidebar rows and Library tiles) and turns every demo arrow Ready, so
no spinner contaminates the numbers. `QUIVER_DEMO_README_KB=<kb>` replaces the Minecraft demo README
(`QUIVER_DEMO_ROUTE=arrow:github.com/rabbyte/minecraft`). Software rendering; the empty-ish baseline is about 100 ms
per frame, which is the cost of one full frame here.

| Scenario | ready (ms) | one notch to next frame, median (ms) | ms per frame, burst |
| --- | --- | --- | --- |
| Sidebar, 15 arrows (baseline) | 172 | 95 | 103 |
| Sidebar, 5000 arrows | 216 | 109 | 110 to 117 |
| Library, 2000 tiles | 1524 | 525 | 464 to 554 |
| Arrow page, 1 KB README (baseline) | 210 | 93 | 98 to 99 |
| Arrow page, 500 KB README | 4066 | 1821 | 1208 |

* The sidebar list is virtualised (`uniform_list`): 5000 rows cost the same as 15.
* **Known limit, not fixed**: the Library grid is not virtualised (`views/library/mod.rs` builds every tile and the
  flex-wrap grid lays all of them out each frame): 2000 tiles take 5x the baseline frame time and 1.5 s to first show.
  A fix needs row windowing inside `ScrollArea` (column count from the viewport, scroll offset exposed by the
  area); it was not cheap enough to do here. Search/Recommended/Collections grids use the same pattern.
* **Known limit, not fixed**: a 500 KB README is parsed and laid out in full by gpui-component's `TextView`: 4 s to
  show and about 1.2 to 1.8 s per scrolled frame. Real READMEs are one to two orders of magnitude smaller.
* Open observation: on the arrow page the bridge counts 0.33 frames/s even when idle (a 3 s period), in demo mode;
  not investigated.

## Caveats

* Software rendering in a shared container: see the top. Do not quote these CPU/frame numbers as desktop performance.
* The first-frame pixel check treats any non-flat paint as painted, so a skeleton counts for native and a WebKit
  shell for Tauri.
* The Tauri binary was built from the tree as it is (`QUIVER_DESKTOP_RELEASE_TAG=stable-1.0`, bun 1.4.4), release
  profile, `tauri build --no-bundle`; it ran with WebKit's sandbox and compositing/dmabuf paths disabled as the old e2e
  box did.
* Native bridge builds are LTO-off, so their size and CPU are not the shipped numbers; use the release column.
