# `e2e/` — tauri-driver scenarios for self-update, self-arrows and the console

Five scenarios driven through the **real, built** Quiver Desktop app and a
**real** `quiver.core` daemon: no mocks, no stubbed resolver, no in-process
test seam.

| Spec | Claim |
|---|---|
| `scenarios/bootstrap.spec.ts` | A clean `QUIVER_HOME` comes up with a self-installed Core at `<QUIVER_HOME>/self/quiver`, both self-arrows are registered `user_installed: true`, and quiver.desktop is filed under one channel identity (`<ns>@<channel>`), never a pin of its version. |
| `scenarios/self-update-while-running.spec.ts` | A process Core supervises keeps its PID across Core's own self-update swap. |
| `scenarios/generic-outdated-badge.spec.ts` | An outdated self-arrow gets the *generic* outdated badge, via the *generic* `/v0/runtime` broadcast. |
| `scenarios/console.spec.ts` | The build indicator and the daemon console. The rail shows both builds once the daemon has answered `/versions`; the console opens from the indicator and shows the daemon's own log, typed; a command runs on the daemon and its output arrives; what the daemon does not offer (`daemon`, a bare `add`, `--server`) is refused in the daemon's own words; a destructive command without `--yes` refuses at once instead of waiting on a prompt; the daemon logs each command it ran. Needs a quiver.core with the console (`features` has `console.v1`), so point `QUIVER_CORE_DEV_PATH` at one. |
| `scenarios/update-core.spec.ts` | Updating quiver.core from the arrow page and from Settings, Engine: core downloads the release, verifies it against the release's `checksums.txt` and swaps the daemon in place, the app stays up and reconnects on the same socket, and a supervised arrow survives detached with the same pid. A wrong checksum changes nothing and says why, a build that never becomes healthy is rolled back to the old version with the row Outdated, and quitting the app stops the daemon running by then. **Runs only in the E2E box**, see below. |

`self-update-while-running.spec.ts` also needs the box: it publishes a release, updates, and checks the
supervised process across the swap.

## Running the update scenario (Linux container)

`update-core.spec.ts` publishes new quiver.core releases mid-test and reads the daemon
replacing itself, so it needs more than a runner can give: a GitHub stand-in that can gain a
release, quiver.core builds that really differ, and a desktop to click in. `docker/e2e`
provides all of it. From the repository root, with a quiver.core checkout at
`QUIVER_CORE_DEV_PATH`:

```bash
make test-e2e-box QUIVER_CORE_DEV_PATH=../quiver.core                  # every spec
make test-e2e-box QUIVER_CORE_DEV_PATH=../quiver.core SPECS=update-core
# or, without make:
QUIVER_CORE_DEV_PATH=../quiver.core docker compose -f docker/e2e/docker-compose.yml \
  run --rm e2e scenarios/run-wdio.sh update-core
```

`SKIP_BUILD=1` (pass it with `-e SKIP_BUILD=1` to `docker compose run`) reuses the
builds from the previous run; only `e2e/` is re-synced, so editing a spec does not
rebuild anything. The first run compiles quiver.core four times (Go) and the app
once (Rust) and is slow; later runs reuse the build volume.

What it runs, in order, on one installation: Settings shows the installed and available
versions and checks for updates; Update from the arrow page swaps the daemon; Update from
Settings swaps it again; a release whose `checksums.txt` lists the wrong hash is refused and
nothing moves; a release whose build never becomes healthy is rolled back; and closing the
app stops the daemon that is running at that point and leaves no quiver process behind.

Logs, per-spec daemon logs, the stand-in's request log and screenshots land in
`docker/e2e/results/wdio/`. In CI this is the `e2e-box` job of
`.github/workflows/e2e.yml`.

Natively (`bun run test`, outside the box) this spec and
`self-update-while-running.spec.ts` are left out of the run: they need the
stand-in's `QUIVER_E2E_UPSTREAM_STATE`.

## Platform support — read this first

**This harness cannot run on macOS.** That is a property of `tauri-driver`,
not of this repo. `tauri-driver`'s `src/main.rs` gates its real entry point
behind `#[cfg(any(target_os = "linux", windows))]` and compiles a stub
everywhere else:

```rust
#[cfg(not(any(target_os = "linux", windows)))]
fn main() {
  println!("tauri-driver is not supported on this platform");
  std::process::exit(1);
}
```

`cargo install tauri-driver` succeeds on macOS and installs that stub — so
even `tauri-driver --version` prints the message above and exits `1`. Its
README lists macOS as *"[Todo] … (probably)"* via the Appium Mac2 driver.
Verified against **tauri-driver 2.0.6**.

Supported: **Linux** (via `WebKitWebDriver`, the `webkit2gtk-driver` package)
and **Windows** (via Microsoft Edge Driver). On a Mac, run the suite in a
Linux container or on CI (`.github/workflows/e2e.yml`).

## There is no `tauri-driver.conf.json`

`tauri-driver` reads **no configuration file, in any version**. Its whole
config surface (verified against 2.0.6's `src/cli.rs` and `src/server.rs`):

**Process flags** — strict; any unrecognised argument is a hard exit.

| Flag | Default | Notes |
|---|---|---|
| `--port` | `4444` | the intermediary port WebdriverIO talks to |
| `--native-port` | `4445` | the native WebDriver behind it |
| `--native-host` | `127.0.0.1` | Linux only |
| `--native-driver` | *(searched on `PATH`)* | path to `WebKitWebDriver` / `msedgedriver.exe` |

**One capability** — `tauri:options`, on `capabilities.alwaysMatch`, parsed by
`struct TauriOptions` (serde `rename_all = "camelCase"`):

| Field | Required | Notes |
|---|---|---|
| `application` | yes | path to the built executable |
| `args` | no | argv for it |
| `webviewOptions` | no | Windows only |

`map_capabilities` strips `tauri:options` and substitutes the native object:
`webkitgtk:browserOptions {binary, args}` on Linux, `ms:edgeOptions` +
`browserName: "webview2"` on Windows.

So all configuration lives in [`wdio.conf.ts`](./wdio.conf.ts).

## Why a *release* build

`tauri:options.application` must point at
`src-tauri/target/release/quiverdesktop`, built with `tauri build --no-bundle`.

A **debug** build will not do, and not for speed reasons. `dev_quiver_home`
(`src-tauri/src/connection/local/mod.rs`) pins a debug build's `QUIVER_HOME`
to `<CARGO_MANIFEST_DIR>/.quiver` **at compile time**, and its unix socket to a
hash of that same path. Every spec would then share one home, and scenario 1's
"clean `QUIVER_HOME`" precondition could not be stated at all. In a release
build `quiver_home()` returns `None`, so the app resolves the home from the
environment and the harness can hand each spec its own.

## Why the harness sets `HOME`, not `QUIVER_HOME`

These are not interchangeable. In a release build the app passes core a bare
`unix://` host argument (`LocalHost::host_arg(false)`), meaning *"core's own
default socket"* — which core resolves under `QUIVER_HOME`. The app's own
transport meanwhile dials `default_socket_path()`, built from **`$HOME`**
(`format!("{}/.quiver/quiver.sock", home)`).

Those agree only while `QUIVER_HOME` is its platform default of
`$HOME/.quiver`. Point `QUIVER_HOME` at some third directory and core's socket
moves out from under the app, which then never connects. So `wdio.conf.ts`
overrides `HOME` per spec and lets `QUIVER_HOME` default beneath it.

## Running locally (Linux)

Needs Node on `PATH` alongside bun -- `expect-webdriverio` (a transitive
dependency of `webdriverio`/`@wdio/*`) requires a real Node runtime; bun alone
fails instantly with `TypeError: The superclass is not a constructor`.

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev build-essential \
  pkg-config webkit2gtk-driver xvfb
cargo install tauri-driver --locked

bun install && make fetch-sidecar
bun run tauri build --no-bundle

TRIPLE=$(rustc -vV | grep '^host:' | awk '{print $2}')
cp "src-tauri/binaries/quiver-${TRIPLE}" src-tauri/target/release/quiver

cd e2e && bun install
xvfb-run -a --server-args="-screen 0 1280x800x24" bun run test
```

### Environment overrides

| Variable | Purpose |
|---|---|
| `QUIVER_E2E_APP_BINARY` | use an already-built app binary |
| `QUIVER_E2E_TMP` | where per-spec homes are created (default `$TMPDIR/quiver-e2e`) |
| `QUIVER_E2E_DRIVER_PORT` / `QUIVER_E2E_NATIVE_PORT` | driver ports |
| `QUIVER_E2E_TAURI_DRIVER` | path to `tauri-driver` |
| `QUIVER_E2E_BUILD_CHANNEL` | the channel the app under test was built for (`VITE_QUIVER_BUILD_CHANNEL`); default: the first channel quiver.desktop's repository lists |
| `QUIVER_E2E_FIXTURE_NS` | the supervised arrow for scenario 2 (see below) |
| `QUIVER_E2E_OUTDATED_NS` | the arrow driven outdated for scenario 3 |

## Known preconditions these scenarios depend on

These are real gaps in the system under test, not harness shortcuts. Each is
asserted loudly — the specs fail with the diagnosis rather than skipping
quietly and reporting green.

1. **`quiver.desktop`'s arrow row needs a resolvable remote manifest.**
   `quiver.core`'s self-arrow is seeded from its **embedded** `ARROW.md`
   (`internal/core/selfmanifest`), so it works offline. `quiver.desktop`'s is
   not embedded anywhere in core: `announceSelf` POSTs
   `/v0/arrow/<ns>@<channel>` (the channel the build was published on, or the
   repository's default one) and core resolves that channel's manifest from
   the remote through `metadata.GetPlatforms()`. The repository must publish
   at least one channel whose tag carries an `ARROW.md`.

2. **Scenarios 2 and 3 need an arrow a *production* daemon can resolve.**
   quiver.core's own `quiver-test/self-update-fixture` is injected through the
   Go integration suite's in-process stub resolver (`newTestResolver` /
   `stubEngines`, `tests/kit/env.go`). A production daemon has no such seam —
   it resolves only through `metadata.yaml`'s embedded `platforms`
   (`github.com`, `gitlab.com`, `bitbucket.org`), which **`config.yaml` cannot
   override**. Set `QUIVER_E2E_FIXTURE_NS` / `QUIVER_E2E_OUTDATED_NS` once such
   an arrow exists, or add a local platform override to core.

## A note on where the outdated badge actually renders

The brief for scenario 3 describes *"the sidebar renders the badge"*. Read
against the components, it does not: `ArrowRow` — what the sidebar's own
`ArrowList` renders — draws an icon, a name and a namespace, and has **no
status badge at all**. The badge belongs to `ArrowTile`, which lives under
`src/features/sidebar/components/arrows/` (hence the brief's file path) but is
consumed by **Home, Library and the collection grids**. `generic-outdated-badge.spec.ts`
therefore asserts against the Library grid, which is where a user really sees it.
