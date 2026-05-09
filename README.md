<p align="center">
  <img src=".github/quiver.svg" alt="Quiver" width="450" />
  <br/>
  <em>Software distribution without a middleman.</em>
</p>

Software distribution remains a fragmented and technically demanding process for both developers and end users. Existing solutions are platform-specific, require technical knowledge, and maintain some degree of centralized control — leaving a gap for an open, accessible, and truly cross-platform alternative.

Quiver is a truly decentralized, cross-platform, open-source application store. Developers publish software in under five minutes by hosting a single file (a declarative manifest) on any Git-compatible repository, with no approval process, no fees. End users install any published application in two clicks through an intuitive visual interface, without technical knowledge. The platform runs as a local service on each machine, enabling remote management of multiple hosts from a single desktop interface.

#### What's in this repo

`quiver.desktop` is the visual interface for Quiver — a Tauri application built with React and TypeScript that lets users browse, install, and manage Arrows through a native desktop UI, and remotely manage multiple hosts from a single window. The core engine lives in a separate repository.

---

## Contributing

Quiver is open-source and we welcome contributions from the community — whether that's improving the UI, adding features, or anything in between.

### Common commands

| Command | Description |
|---|---|
| `make deps` | Install all dependencies (frontend + Rust) |
| `make deps-frontend` | Install frontend dependencies (bun) |
| `make deps-rust` | Download Rust dependencies |
| `make build` | Build both frontend and Tauri backend |
| `make fmt-frontend` | Fix frontend formatting with Prettier |
| `make lint-frontend` | Run ESLint |
| `make typecheck-frontend` | Run TypeScript type checking |
| `make fmt-rust` | Fix Rust formatting |
| `make lint-rust` | Run Clippy |
| `make test-rust` | Run Rust tests |
| `make coverage-rust` | Run tests with coverage (≥80%) |

Before opening a PR, always run:

```bash
make pr-checks
```

This runs the full validation suite — formatting, linting, type checking, build, and tests. Run `make help` to see all available targets.

---

## License

Quiver is licensed under the [GPL-3.0](LICENSE).

---

## Stay Connected

- [Rabbyte GitHub](https://github.com/rabbytesoftware)
- [char2cs](https://char2cs.net)
