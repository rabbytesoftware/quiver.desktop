# Contributing to Quiver Desktop

The app is a Cargo workspace. [docs/architecture.md](docs/architecture.md) has the crate graph; this page is
the working agreement. `make gate` is what CI enforces.

## Where code goes

- **A decision** (what a status means, which buttons show, when an update is offered) lives in
  `crates/quiver-state/src/<feature>/` as plain Rust with unit tests. Views call it.
- **A view** lives in `crates/quiver-app/src/views/<feature>/` (`mod.rs` plus `parts/`). It reads state, renders and
  sends an action; it holds no business logic. How to write one: `crates/quiver-app/src/views/README.md`.
- **A reusable widget** lives in `crates/quiver-ui/src/<component>.rs`, one file per component, named after its
  Coss UI counterpart, and is added to the gallery (`cargo run -p quiver-ui --example gallery`).
- **Colours, radii, sizes** come from `quiver-theme` tokens. Never hard-code a colour in a view.
- **Talking to quiver.core, secrets, settings, paths** live in `quiver-core-client` / `quiver-platform`.
  Arrow-app rules live in `quiver-arrow-host`.

## Rules the gate checks

- Dependencies point one way. Nothing below `quiver-ui` imports `gpui` (`cargo xtask check-deps`).
- Files under 300 lines, 500 is a hard limit; no `utils`/`helpers`/`common`/`misc` modules (`cargo xtask check-layout`).
- `cargo fmt --check`, `cargo clippy -D warnings` with `unwrap_used` and `expect_used` denied.
- One `thiserror` error enum per library crate; `anyhow` only in `quiver-app` and `xtask`.
- `cargo deny`, `cargo audit`, `cargo machete`.
- Tests sit next to the code (`#[cfg(test)]`); integration tests in each crate's `tests/`.
  No sleeps: wait on a signal.

## End-to-end tests

Every interactive element and screen has a stable id (`<screen>.<element>`). The debug-only `e2e-bridge`
feature serves them over JSON-RPC, and `quiver-mcp` exposes them to an agent. See [docs/e2e.md](docs/e2e.md).
`make test-e2e-box` runs the scenarios in the Linux container against a real quiver.core. The bridge is never
compiled into release artifacts (`cargo xtask check-release-artifact`).

## Commits

Small, focused, imperative (`feat(state): ...`, `fix(app): ...`). Decisions that deviate from the structure
rules go in `docs/decision-log.md`.
