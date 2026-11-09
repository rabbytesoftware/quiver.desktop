# The E2E box

A disposable Linux desktop in a container (Xvfb + openbox, Mesa lavapipe, VNC/noVNC view) that builds the native
app with its debug-only e2e bridge, builds real quiver.core versions, stands up a local GitHub stand-in and runs
`cargo test -p quiver-e2e --test scenarios` against them.

```
QUIVER_CORE_DEV_PATH=../quiver.core make test-e2e-box
```

Usage, environment variables, the bridge protocol and the MCP server are documented in `docs/e2e.md`.

Layout: `Dockerfile`/`docker-compose.yml`/`start.sh` (the machine), `scenarios/build.sh` (artifacts),
`scenarios/run-native.sh` (the Rust scenarios), `scenarios/run-all.sh` + `scenario-1-core-self-update.sh`
(headless shell scenario), `scenarios/lib.sh` (assertions, upstream publishing helpers), `fixtures/` (the
GitHub stand-in `upstream.py`, its launcher, trust store, seed commit, fixture arrows).

## Arrow apps

The arrow-apps tests drive quiver.chat, so that checkout must be mounted at `/workspace/quiver.chat` (add a volume
to `docker-compose.yml`); without it they are skipped by `build.sh` (`[chat] no checkout`).
