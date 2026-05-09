# CI Overhaul & quiver.core Sidecar Infrastructure

**Date:** 2026-05-09
**Branch:** `enchancement/ci-improvements`

---

## Overview

Two tightly coupled workstreams:

1. **CI overhaul** — replace the current `release/*` / `vYY.MM.XX` pipeline with the `beta/YY.M` / `stable-YY.M` model defined in the shared branch spec, mirroring the structure already implemented in `rabbytesoftware/quiver.core`.
2. **Sidecar infrastructure** — update all references from the renamed `rabbytesoftware/quiver` → `rabbytesoftware/quiver.core`, rename the version tracking file, and add core version validation to release builds.

The reference implementation for both flows is the quiver.core repo. Where logic is identical, the desktop repo copies it directly; where the desktop differs (Tauri multi-platform builds instead of Go cross-compilation), it adapts.

---

## Branch Model

Unchanged from the shared spec. Desktop-specific notes:

| Branch | Targets | Notes |
|---|---|---|
| `feature/*`, `enhancement/*` | `develop` | `enhancement/*` is a kept alias for `feature/*` |
| `fix/*`, `refactor/*` | `develop` | |
| `dependabot/*` | `develop` | |
| `beta/YY.M` or `beta/YY.M.P` | `master` | Triggers pre-release on every push |
| `hotfix/*` | `master` | Triggers pre-release on every push |
| `backport/*` | `develop` | Auto-created by CI after every stable release |

`release/*` branches are removed from the model. The current CI validation that allows them is dropped.

---

## Workflow File Structure

Four files, mirroring quiver.core exactly:

| File | Replaces | Trigger |
|---|---|---|
| `.github/workflows/ci.yml` | (updated in place) | PR → `master` or `develop` |
| `.github/workflows/prerelease.yml` | `release.yml` | Push to `beta/**` or `hotfix/**` |
| `.github/workflows/stable-release.yml` | `merge-to-master.yml` | PR closed on `master` (merged) |
| `.github/workflows/backport.yml` | (new) | `workflow_call` from `stable-release.yml` |

`release.yml` and `merge-to-master.yml` are deleted.

The shared action `.github/actions/build-tauri/action.yml` is kept and updated (repo reference only).

---

## `ci.yml` — PR Validation

### Branch validation changes

- **Add** `beta/*` → must target `master`
- **Add** `backport/*` → must target `develop`
- **Remove** `release/*` (no longer a valid branch type)
- All other rules unchanged

### Jobs (no structural changes)

- `draft-reminder` — unchanged
- `validate-branch` — updated rules above
- `code-quality-frontend` — unchanged
- `code-quality-rust` — unchanged
- `build-frontend` — unchanged
- `build-tauri` — unchanged (continues using placeholder binaries; no real sidecar download during PR validation)

---

## `prerelease.yml` — Beta & Hotfix Pre-releases

**Trigger:** push to `beta/**` or `hotfix/**`

**Concurrency:** `prerelease-${{ github.ref }}`, cancel-in-progress (same as quiver.core).

### Steps

1. **Validate `CORE_VERSION`** — read `CORE_VERSION` from repo root, call `gh release view <version> --repo rabbytesoftware/quiver.core`. Fail loudly if the release does not exist.
2. **Derive pre-release tag** — identical logic to quiver.core:
   - `beta/*`: count existing `beta-YY.M` and `beta-YY.M-*` tags → first push produces `beta-YY.M`, subsequent pushes produce `beta-YY.M-1`, `beta-YY.M-2`, etc.
   - `hotfix/*`: find latest `stable-*` tag, increment patch → count existing `hotfix-*` tags for that version → produce `hotfix-YY.M.P` or `hotfix-YY.M.P-N`.
3. **Multi-platform Tauri build** — 3-platform matrix (ubuntu-latest, macos-latest, windows-latest) calling `.github/actions/build-tauri` with the `CORE_VERSION` value.
4. **Download artifacts** and publish a GitHub pre-release tagged with the derived tag.
5. **Comment on open PR** (if one exists for the branch) with the pre-release link.

---

## `stable-release.yml` — Stable Release

**Trigger:** `pull_request` on `master`, type `closed`, filtered to `merged == true` and source branch starting with `beta/` or `hotfix/`.

**Concurrency:** `stable-release`, cancel-in-progress: false (never interrupt a release in flight).

### Steps

1. **Validate `CORE_VERSION`** — same check as prerelease.
2. **Derive stable tag** — identical logic to quiver.core:
   - `beta/*` source: derive series from branch name (`beta/26.5` → series `26.5`), find latest `stable-26.5` or `stable-26.5.*`, increment patch or create fresh.
   - `hotfix/*` source: find latest `stable-*` tag, derive series, apply same increment logic.
3. **Create and push the stable tag.**
4. **Multi-platform Tauri build** — same 3-platform matrix via `build-tauri` action.
5. **Publish stable GitHub Release** (not draft, not pre-release).
6. **Call `backport.yml`** as a reusable workflow, passing the stable version.

---

## `backport.yml` — Backport to Develop

**Trigger:** `workflow_call` with input `version` (the stable tag, e.g. `stable-26.5`).

Near-direct copy from quiver.core. Steps:

1. Checkout `master` with full history.
2. Create branch `backport/{DATE}-{VERSION}` (strips `stable-` prefix from version).
3. Push branch and open PR targeting `develop`.
4. Attempt `gh pr merge --merge`. On failure, post a warning comment on the PR asking for manual resolution.

---

## Sidecar Infrastructure

### File rename

`.quiver-version` → `CORE_VERSION`

Same format (plain text, e.g. `v0.1.0`). All workflow references updated accordingly.

### Repo reference update

Every occurrence of `rabbytesoftware/quiver` in CI and the build action is updated to `rabbytesoftware/quiver.core`. Affects:

- `.github/actions/build-tauri/action.yml` — `gh release download` call
- Any workflow that references the repo directly

### Core version validation

Added as the **first step** in both `prerelease.yml` and `stable-release.yml`:

```bash
CORE_VERSION=$(cat CORE_VERSION)
gh release view "$CORE_VERSION" --repo rabbytesoftware/quiver.core || {
  echo "❌ quiver.core release $CORE_VERSION not found. Core must be released before desktop."
  exit 1
}
```

Not added to `ci.yml` (PR validation uses placeholder binaries and doesn't download the real core).

### `src/lib/sidecar.ts`

No changes needed. It references the local binary path (`binaries/quiver`), not the upstream repo.

### `src-tauri/tauri.conf.json`

No changes needed. `externalBin` path is correct.

---

## Error Handling

| Scenario | Behavior |
|---|---|
| `CORE_VERSION` release missing on pre/stable build | Hard fail, loud error message |
| Beta tag already exists (race) | Tag derivation counts existing tags, naturally increments |
| Backport merge conflict | PR stays open, bot comments asking for manual resolution |
| CI draft PR | Job exits 1, comment posted with reminder to run `make pr-checks` |

---

## Out of Scope

- Nightly builds (explicitly dropped)
- UI sidebar component (separate story)
- Changes to `src/lib/sidecar.ts` or API client
- Changes to `tauri.conf.json`
