# The E2E box

A disposable Linux desktop in a container, with a real X session, a real
window manager and a real VNC view of both, used to prove three things about
quiver.core and quiver.desktop that cannot be proved anywhere else: that core
replaces itself, that the app adopts a daemon it did not start, and that core
can stop, replace and reopen the app.

```
docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh
docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh 2 3
SKIP_BUILD=1 docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-all.sh
```

The WebDriver specs under `e2e/` run in the same box:

```
docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-wdio.sh
docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-wdio.sh update-core
```

`update-core` publishes releases of quiver.core as the daemon sees them on GitHub: a
`quiver-linux-<arch>` asset and the `checksums.txt` the manifest's `fetch` step verifies it
against, both under `releases/<user>/<repo>/<tag>/` in the stand-in's state. It also
publishes a release that lists a wrong checksum and one whose asset hands over to a real
build's `self-update` but is not a daemon itself, to prove the refusal and the rollback.

`QUIVER_CORE_DEV_PATH` names the quiver.core checkout both commands build from. The
`e2e-box` job of `.github/workflows/e2e.yml` runs it; `e2e/README.md` says what each
spec covers. Results go to `docker/e2e/results/wdio/`. The stand-in serves
whatever sits in a release's directory at `releases/download/<tag>/<asset>`, which
includes the `checksums.txt` quiver.core's own update verifies against, and answers
`releases/expanded_assets/<tag>`, which is how quiver.core lists a release's assets.

To watch it happen instead, bring the box up on its own and open
<http://localhost:6080/vnc.html>:

```
docker compose -f docker/e2e/docker-compose.yml up --build
```

Everything a run produces lands in `docker/e2e/results/` on the host:
`SUMMARY.txt`, a log per scenario, the upstream fixture's request log, and the
screenshots.

## Arrow apps

`arrow-apps` (e2e/scenarios/arrow-apps.spec.ts) also needs a quiver.chat checkout, mounted
read-only at `/workspace/quiver.chat`; `run-wdio.sh` adds the spec to the default list only
when it is there. `build.sh` turns it into the two linux release archives its ARROW.md fetches
from the `nightly` release (`quiver-chat-linux-arm64.tar.gz`, `quiver-chat-linux-amd64.tar.gz`),
building the frontend first if the checkout has not. The stand-in serves the chat's own
ARROW.md and git repository, the archives, and three fixture arrows from `fixtures/arrows/`:
`e2e-static-app` (a `static` interface, rendered as three copies so several apps can be open
at once) and `e2e-echo-app` (a `listen` interface that binds five seconds late and echoes
request headers). With a compose override that adds the mount:

```
services:
  e2e:
    volumes:
      - /path/to/quiver.chat:/workspace/quiver.chat:ro
```

```
docker compose -f docker/e2e/docker-compose.yml -f override.yml run --rm e2e scenarios/run-wdio.sh arrow-apps bootstrap
```

The chat's ARROW.md is served exactly as committed in its checkout; the spec's first check
(A0) asserts it validates against the core under test and that the stand-in serves it byte for
byte.

## What is real, and what stands in for something

Real: both applications, built from the two mounted checkouts by their own
toolchains. Real processes, a real X display, real HTTP over the unix socket
the daemon actually binds, real `fetch` steps with real sha256 verification,
and the real `quiver` CLI for the parts a user would type.

Three things stand in, each for a reason that is not "it was easier":

**github.com.** Every scenario needs an upstream that can gain a release
mid-test, and quiver.desktop has none published at all. `/etc/hosts` points
the three GitHub hostnames at `fixtures/upstream.py`, behind a CA the
container trusts (`fixtures/trust.sh`). Nothing in quiver.core is patched,
stubbed or rebuilt for this: it still derives every URL from its own embedded
`metadata.yaml`, still reads the repository's tags for its version check, still runs
the same fetch and checksum code. Only name resolution and the trust store
differ, which is what any TLS-inspecting corporate proxy does to every Go
program on earth. The fixture also serves real git smart-HTTP through git's
own `http-backend`, because a globbed dependency constraint is resolved by
`ls-remote` with no host-specific shortcut.

**The AppImage.** `fixtures/pack-appimage.sh` produces a single executable
file containing the built binary and its sidecar, not a squashfs image behind
a FUSE runtime — building one needs appimagetool, and running one needs
`/dev/fuse`. It is faithful in the two respects the lifecycle actually
touches: it is ONE chmod-able file at the path the manifest places, and the
process it leaves behind is named `quiverdesktop`, exactly as Tauri's own
`AppRun` leaves behind, which is what `pkill -x quiverdesktop` has to match.
A fixture that just renamed the binary `Quiver.AppImage` would present a
process called `Quiver.AppImage` and quietly break that assumption instead of
testing it. See that script's header for the full accounting.

**The update click** is not a stand-in at all any more. Scenario 3 opens
Quiver's own page in the running app, finds the hero's primary action by
reading the pixels (`fixtures/find-button.py`, so a layout change moves the
click rather than silently missing it), and presses a real mouse button on it
with `xdotool`. Everything after that is the app's own code: the click
handler, `releaseVariables()`, the `invoke` into `src-tauri/src/release/`, a
real HTTPS request to the releases API, and the mutation that posts the
result to core. The fixture's own request log is what proves the app went to
the releases API, because nothing else in the run ever asks it anything.

## Why the app is built with `tauri build --no-bundle`

Not for the bundle, which is skipped. A plain `cargo build --release` produces
a binary that still loads `tauri.conf.json`'s `devUrl`: whether Tauri serves
its embedded frontend is decided by the `tauri` crate's `custom-protocol`
feature, not by the cargo profile. This box caught that the loud way — the
window came up, `wmctrl` saw it, the screenshot was taken, and the screenshot
said "Could not connect to localhost: Connection refused" while every
Rust-side assertion passed. Release profile also matters on its own: a debug
build takes `dev_quiver_home()` and `dev_socket_override()`, which point the
app at a checkout-scoped home and a `/tmp` socket, and scenario 2 is entirely
about the app finding a daemon at the production address.
