# Contributing to Quiver Desktop

Conventions for the frontend. The gates at the bottom are what CI enforces.

## Feature layout

Everything a feature owns lives under `src/features/<name>/`, split by what the
code _is_ rather than piled into `components/`:

```
src/features/<name>/
  api/          calls to quiver.core, one file per surface   (engine-api.ts)
  components/   React components; sub-folder once a cluster forms
  hooks/        use-*.ts — React-bound, one concern each
  lib/          pure logic, no React, no store                (narrow.ts, geometry.ts)
  stores/       Zustand stores, named for the feature         (shell-store.ts)
  styles/       CSS a component imports                       (card.css)
  index.ts      the feature's public surface
```

Only create the folders a feature actually needs. `search` has no `api/` or
`stores/` because both live in `src/lib/core-store/` — see **Where state lives**.

### When to nest inside `components/`

Nest when a cluster has its own vocabulary, not to hit a file count:

```
search/components/card/      the tile: arrow-card, card-skeleton
search/components/results/   the screen: results-screen, result-grid, results-header, …
search/components/           standalone surfaces: search-bar, search-inspector
```

A component that stands alone stays flat. Mixing nested and flat is fine.

## Imports

- Same folder: relative — `./arrow-card`.
- Anything else, including another folder of the same feature: the alias —
  `@/features/search/lib/narrow`, never `../../lib/narrow`.

Relative paths that climb out of a folder break the moment a file moves, and
this layout exists because files moved. The alias resolves to `src/`, so a path
that has to reach outside `src/` (the repo-root `index.html`, for one) is the
single case that stays relative — say so in a comment where it happens.

## Tests

Tests sit **next to what they test**: `lib/narrow.ts` → `lib/narrow.test.ts`.

This is a deliberate departure from crowbar, which mirrors everything into
`src/__tests__/`. Both work; what does not work is half of each, and every
existing test in this repo — `src/lib/**` included — is co-located.

A test that spans several units is the feature's integration test and stays at
the feature root (`shell.test.tsx`, `settings.test.tsx`). A test named for the
feature but exercising one component is not that — name it for the component and
put it beside it.

## Naming

Files are kebab-case; the exported component stays PascalCase.

```tsx
// file: arrow-card.tsx
export function ArrowCard() { … }
```

Store files carry the feature name (`shell-store.ts`, not `store.ts`) so an open
editor tab says which store it is. Same for `api/` (`engine-api.ts`).

## Where state lives

- Server state and the search pass controller live in `src/lib/core-store/`.
  Rust owns the sidecar, the WebSocket, and HTTP; TypeScript owns the store.
  A feature reads it — it does not re-home it under `features/<name>/stores/`.
- `features/<name>/stores/` is for state that belongs to that feature's UI
  (sidebar width, which settings tab is open, theme preference).

## Vendored UI

`src/components/ui/**` is vendored from CossUI/shadcn registries. It is held out
of `knip` and out of `react-doctor`'s `only-export-components` on purpose, so a
registry file can be dropped in without a reformat. Prune what does not apply
(variants pointing at tokens this theme lacks) and leave a comment saying why.

## Gates

`npm test`, `npx tsc --noEmit`, `npx eslint src`, `npx prettier --check src`,
`npm run doctor`, `npm run test:coverage`. CI runs all of them; `doctor` blocks
on warnings and coverage holds a 95% floor on every metric, branches included.
