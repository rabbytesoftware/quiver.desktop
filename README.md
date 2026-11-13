<p align="center">
  <img src=".github/quiver.svg" alt="Quiver" width="450" />
  <br/>
  <em>Software distribution without a middleman.</em>
</p>

Software distribution remains a fragmented and technically demanding process for both developers and end users. Existing solutions are platform-specific, require technical knowledge, and maintain some degree of centralized control — leaving a gap for an open, accessible, and truly cross-platform alternative.

Quiver is a truly decentralized, cross-platform, open-source application store. Developers publish software in under five minutes by hosting a single file (a declarative manifest) on any Git-compatible repository, with no approval process, no fees. End users install any published application in two clicks through an intuitive visual interface, without technical knowledge. The platform runs as a local service on each machine, enabling remote management of multiple hosts from a single desktop interface.

#### What's in this repo

`quiver.desktop` is the visual interface for Quiver: a fully native Rust application on [GPUI](https://www.gpui.rs) that lets users browse, install, and manage Arrows, and remotely manage multiple hosts from a single window. The core engine lives in a separate repository. The architecture (crate graph, module mapping from the former Tauri app) is in [docs/architecture.md](docs/architecture.md).

---

## Install

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/install.sh | bash
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/rabbytesoftware/quiver.desktop/develop/install.ps1 | iex
```

Both scripts read the latest GitHub release, pick the right asset for your machine, and put it where `quiver.core` looks for it: the AppImage under `~/.local/share/Quiver` on Linux, `Quiver.app` in `/Applications` on macOS, and the per-user NSIS install under `%LOCALAPPDATA%\Quiver` on Windows. `ARROW.md` installs to exactly the same places, so it makes no difference which route you take.

---

## Contributing

Quiver is open-source and we welcome contributions from the community — whether that's improving the UI, adding features, or anything in between.

### Build and test

You need a stable Rust toolchain. On Linux install the GPUI system libraries listed in
[docker/native/Dockerfile](docker/native/Dockerfile) (or build everything inside the container:
`scripts/box.sh cargo ...`).

| Command | Description |
|---|---|
| `make dev` | Run the app against a real quiver.core (fetches the sidecar once) |
| `make dev-local` | Run against a sibling quiver.core checkout (`QUIVER_CORE_DEV_PATH`) |
| `cargo run -p quiver-app -- --demo` | Run over seeded in-memory data, no daemon (design review, screenshots) |
| `make build` | Release build of the workspace binaries (no e2e bridge) |
| `make test` | `cargo test --workspace` |
| `make gate` | The PR gate: fmt, clippy `-D warnings`, test, cargo-deny, cargo-audit, machete, dependency direction, file layout |
| `make package` | Assemble the installer/bundle for this OS in `target/bundle/` |
| `make test-e2e-box` | End-to-end scenarios against a real quiver.core in the Linux desktop container (Docker) |

Before opening a PR run `make gate`. Run `make help` for the rest.

The crates below `quiver-ui` never depend on `gpui`; `cargo xtask check-deps` enforces it. Files stay under
300 lines (500 is a hard limit, `cargo xtask check-layout`). See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

Quiver is licensed under the [GPL-3.0](LICENSE).

---

## Stay Connected

- [Rabbyte GitHub](https://github.com/rabbytesoftware)
- [char2cs](https://char2cs.net)
