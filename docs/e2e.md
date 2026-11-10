# End-to-end testing and driving the app

Two things share one mechanism, the **e2e bridge**: the Rust scenario tests (`crates/quiver-e2e/tests/`) and an
agent (through the MCP server) both control the real app by talking JSON-RPC to a localhost socket the app opens
when it is built with the `e2e-bridge` feature.

## Debug-only guarantee

* The bridge exists only in binaries built with `cargo build -p quiver-app --features e2e-bridge`. Without the
  feature the code is not compiled in; with it, the socket still stays closed unless `QUIVER_E2E_PORT` is set.
* It binds `127.0.0.1` only.
* Release builds never enable the feature: `make build`, the `build-native` action and `cargo xtask package` do not.
  `cargo xtask check-release-artifact <binary>...` fails when a binary contains the strings `QUIVER_E2E_PORT` or
  `quiver_e2e`. CI runs it on the release build (`native-release-artifact` job), `cargo xtask package` runs it on
  the app and helper before bundling, and `scenarios/build.sh` asserts the opposite for the box's binary.

## The box (`make test-e2e-box`)

`docker/e2e` is a disposable Linux desktop: Xvfb + openbox on `:1`, Mesa lavapipe (software Vulkan), WebKitGTK
for the arrow-app helper, ImageMagick/xdotool/scrot, Rust, and Go to build quiver.core. No browser driver, node or
bun.

```
QUIVER_CORE_DEV_PATH=../quiver.core make test-e2e-box               # all scenarios
QUIVER_CORE_DEV_PATH=../quiver.core make test-e2e-box TESTS="update_core qol"   # names containing these
SKIP_BUILD=1 docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-native.sh
docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh   # headless shell scenarios
docker compose -f docker/e2e/docker-compose.yml up --build                          # watch: http://localhost:6080/vnc.html
```

`scenarios/run-native.sh`:

1. `build.sh` builds four stamped quiver.core versions from `QUIVER_CORE_DEV_PATH` (`stable-26.5.90`..`93`,
   `-ldflags` stamps as the release workflows do), then the app with `--features e2e-bridge` (release profile,
   LTO off, stamped `QUIVER_DESKTOP_RELEASE_TAG=stable-1.0`) and the `quiver-arrow-window` helper, then the
   scenarios test binary.
2. Lays the app out as an installed bundle: `quiverdesktop`, `quiver-arrow-window`, and the oldest quiver.core as
   the sidecar `quiver`.
3. Stands up the local GitHub stand-in (`fixtures/upstream-up.sh`): `github.com` resolves to it inside the box.
4. For every test in `cargo test -p quiver-e2e --test scenarios -- --list`: kills leftovers, wipes
   `QUIVER_E2E_TMP`, reseeds the upstream baseline (quiver.core release 1, quiver.desktop's `ARROW.md`, the
   fixture arrows; tests named `*generic_outdated_badge*` also get core release 2, `*arrow_apps*` get quiver.chat
   and the echo/short/static fixtures) and runs that one test with `--exact`.

Environment the tests read (same names the WebDriver runner exported): `QUIVER_E2E_APP_BINARY` (the installed
`quiverdesktop`), `QUIVER_E2E_TMP`, `QUIVER_E2E_BUILD_CHANNEL`, `QUIVER_E2E_FIXTURE_NS`, `QUIVER_E2E_REQUIRED_NS`,
`QUIVER_E2E_UPDATABLE_NS`, `QUIVER_E2E_VERSION_CHECK_TTL`, `QUIVER_E2E_RESULTS`, `QUIVER_E2E_UPSTREAM_STATE`,
`QUIVER_E2E_UPSTREAM_LOG`, `QUIVER_E2E_CORE_MANIFEST`, `QUIVER_E2E_CORE_TAGS`, `QUIVER_E2E_CORE_ASSETS`,
`QUIVER_E2E_CORE_ASSET`, `QUIVER_E2E_CHAT_NS`/`_MANIFEST`, `QUIVER_E2E_ECHO_NS`, `QUIVER_E2E_SHORT_NS`,
`QUIVER_E2E_STATIC_NS`, `QUIVER_E2E_TCP_PORT`; plus `QUIVER_E2E_TEST_NAME` (new). Results land in
`docker/e2e/results/native/` (`SUMMARY.txt`, one `.log` and one directory per test, `upstream.log`, daemon logs, a
final screenshot).

The stand-in serves whatever sits in a release's directory at `releases/download/<tag>/<asset>` (including the
`checksums.txt` quiver.core verifies against) and answers `releases/expanded_assets/<tag>`.

Not ported from the WebDriver era: shell scenarios 2 (sideload), 3 (desktop update) and the quit-lifecycle one
clicked the Tauri window with xdotool and read pixels (`find-button.py`); they, and `demo-up.sh`, were deleted and
are expected as Rust tests over the bridge. `scenarios/scenario-1-core-self-update.sh` needs no UI and stays.

## Writing a scenario (`crates/quiver-e2e/tests/`)

`tests/scenarios.rs` is the one test target (`--test scenarios`); each scenario is a file in `tests/scenarios/` pulled
in with `#[path]`, and everything they share is in `tests/support/` (reuse it, do not copy it):

| Module | What it gives |
| --- | --- |
| `env` | `Env::load("name")`: reads the `QUIVER_E2E_*` names the box exports. Returns `None` after printing `SKIP ...` when `QUIVER_E2E_APP_BINARY` is unset, so plain `cargo test` stays green: `let Some(env) = Env::load("x") else { return };`. `required(NAME)` / `optional(NAME)` read the other variables. |
| `home` | `Home::fresh(&env, name)`: a wiped `$QUIVER_E2E_TMP/<name>` (HOME; `QUIVER_HOME` is `<HOME>/.quiver`), with the version-check TTL config; `socket()`, `self_binary()`, `daemon_log()`, `env()`. |
| `app` | `App::launch(&env, &home)` starts the app with the bridge env and connects; `wait_ready()`, `signal`, `route`, `node`, `matching(filter)`, `gone`, `find`, `click`, `enter_text`, `with_prefix`, `shot(name)` (PNG into `$QUIVER_E2E_RESULTS/<test>/`). All waits are bridge signals or `wait.node`. |
| `daemon` | `Daemon::unix(socket)` / `Daemon::tcp(port)` (+ `with_bearer`): plain HTTP to a quiver.core, not through the app. |
| `arrows` | the arrow API of a daemon: `arrow`, `wait_arrow`, `channels`, `library`, `check_now`, `add`, `runtime`. |
| `upstream` | the GitHub stand-in: `Upstream::load()`, `publish_core_release(tag)`, `publish_repo_tag(repo, tag)`, `hits(path)`, `core_tags()`. |
| `procs` | `alive`, `pgrep`, `daemon_version`, `kill`. |
| `wait` | `eventually`: the one bounded re-read, used ONLY for what happens inside another process (the daemon's API, the file system) and which the app does not signal. UI conditions never use it. |

Per-test isolation comes from `run-native.sh` (it runs every test alone with `--exact`, a wiped `QUIVER_E2E_TMP`, a
freshly seeded stand-in, and `QUIVER_E2E_TEST_NAME`). Ids the views report are listed in
`crates/quiver-app/src/views/README.md`; the ones the scenarios rely on beyond the screen/tile/sidebar ones are
`sidebar.build` (state: `core <build>; app <build>`), `console.line.<n>` (label: the row text, state: `log:<level>`,
`cmd`, `out:stdout|stderr`, `note:<tone>`), `tile.<namespace>.badge` (label: the badge text) and the `[focused]`
suffix on `sidebar.search`'s state. App signals: `first-frame`, `core.ready`, `connection.ready:<id>`,
`route:<path>`.

## Bridge protocol

Newline-delimited JSON-RPC 2.0 over TCP, `127.0.0.1:<port>`. The app prints `QUIVER_BRIDGE_PORT=<port>` on stdout
when it listens (`QUIVER_E2E_PORT=0` picks a free port). Element ids are the stable ids components report.
Source of truth: `crates/quiver-e2e/src/protocol.rs` and `server.rs`.

| Method | Params | Result |
| --- | --- | --- |
| `app.ping` | | version info |
| `ui.tree` | | every element painted in the last frame (id, kind, label, enabled, state, bounds) |
| `ui.find` | `{id}` | the node or `null` |
| `ui.click` / `ui.double_click` / `ui.hover` | `{id}` | `{}` |
| `ui.type` | `{id?, text}` | focuses `id` when given, types `text` |
| `ui.key` | `{keys}` | presses a chord: `enter`, `cmd-k`, `ctrl-shift-p` |
| `ui.scroll` | `{id, dx, dy}` | logical pixels |
| `wait.signal` | `{signal, timeout_ms}` | waits for a named, latched app signal. Never a sleep; error `-32001` on timeout |
| `wait.node` | `{id? , prefix?, label_contains?, state_contains?, y_lt?, y_gt?, present, timeout_ms}` | waits for an element matching the filter to appear (or, with `present: false`, for none to match); returns the node |
| `state.dump` | | JSON snapshot of the app state |
| `logs.recent` | `{n}` | last `n` log lines |
| `window.screenshot` | | `{png_base64}` |
| `window.resize` | `{width, height}` | `{}` |

Unknown method: `-32601`. Tests never sleep: they wait on `wait.signal` / `wait.node`.

## MCP server: drive the app from an agent

`quiver-mcp` (`crates/quiver-e2e`, binary `quiver-mcp`) is an MCP stdio server over the same bridge. Tools:
`app_launch {binary, env?}`, `app_stop`, `ui_tree`, `ui_find {id}`, `ui_click {id}`, `ui_type {id?, text}`,
`ui_key {keys}`, `ui_scroll {id, dx, dy}`, `wait_signal {signal, timeout_ms?}`, `wait_node {id, present?,
timeout_ms?}`, `state_dump`, `logs {n?}`, `screenshot` (returns a PNG).

Build both binaries with the bridge, then register the server, for example in `.mcp.json` at the repo root:

```
cargo build -p quiver-app --features e2e-bridge
cargo build -p quiver-e2e --bin quiver-mcp
```

```json
{
  "mcpServers": {
    "quiver": {
      "command": "target/debug/quiver-mcp",
      "args": []
    }
  }
}
```

Then ask the agent to `app_launch` with `binary` set to `target/debug/quiver` (or `quiverdesktop` once the bin is
renamed) and an optional `env` such as `{"QUIVER_HOME": "/tmp/quiver-agent"}` to keep it away from your real
home. On Linux without a display run it under `xvfb-run -a` with `ZED_ALLOW_EMULATED_GPU=1`, or inside the box.
