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

To watch it happen instead, bring the box up on its own and open
<http://localhost:6080/vnc.html>:

```
docker compose -f docker/e2e/docker-compose.yml up --build
```

Everything a run produces lands in `docker/e2e/results/` on the host:
`SUMMARY.txt`, a log per scenario, the upstream fixture's request log, and the
screenshots.

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
`metadata.yaml`, still follows `/releases/latest` for its redirect, still runs
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

**The update click.** Scenario 3 calls `POST /v0/runtime/:ns/update` directly
rather than driving the button. Everything either side of it — the drift
check that raises the badge, and the four lifecycle steps that run — is real.

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
