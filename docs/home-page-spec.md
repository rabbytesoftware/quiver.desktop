# Home Page — Build Spec

Status: design approved, ready for implementation.
Design reference (canvas — Main/Empty/Library/Collections screens + shared ArrowTile component): https://claude.ai/code/artifact/0b5c6dc0-a817-4c37-8931-1aeeff10de83
Design source (`.dc.html` prototype files — reference for exact markup/CSS values, not for copy-paste as production code): session scratchpad, `home-design/`.

This spec captures every product decision made while designing this page, cross-checked against quiver.core's actual source and quiver.desktop's actual current codebase (both verified in-session, not assumed). Anywhere the design mockup and the real app disagree, this doc states the real value to build against.

---

## 1. Purpose

Home (`/`) is the app's landing screen — currently a placeholder (`src/routes/index.tsx`: `<div data-testid="home-page" />`) despite already being wired as the first primary-nav tab. Its job, settled after comparing UX patterns from Steam/Vortex/CurseForge and quiver.core's actual capabilities: a calm, status-first overview of what the user already has — not a fourth place to browse for new arrows (the sidebar's `ArrowList` and `/search` already own compact browsing and discovery, respectively). Every close analog to this app category keeps discovery separate from the library-home and shows status inline rather than in a dedicated feed; quiver.core itself has no featured/trending/recommended concept to build a discovery feed from even if we wanted one.

Non-goals for this revision (see §9): a system-resource dashboard, an update-available banner, an activity/usage log beyond "recently used arrows," configurable Home sections.

---

## 2. Content model

Three sections, each rendered from the shared `ArrowTile` component (§5), each omitted entirely when it has nothing to show (not a collapsed empty box — true progressive disclosure):

| Section | Items shown | Data source | Sort | Grid |
|---|---|---|---|---|
| Recents | top 3 | `ArrowEntry.last_used_at` (**does not exist in quiver.core yet** — §8) | `last_used_at` desc, nulls excluded | fixed `repeat(3, minmax(0,1fr))` — deliberately not `columnCap(3)`'s 2-column wrap (that branch is tuned for organic result counts, not a curated top-N row) |
| Library | 10 of N | `useArrowStore` (already reactive, already app-wide — no new fetch) | name, ascending (§7.3, needs sign-off) | `columnRule(10)` from `src/features/search/lib/columns.ts` |
| Collections | 4 of N | new `useFollowedCollections()` hook (§6.2) | `followed_at` desc (§7.3, needs sign-off) | `columnRule(4)` |

Library and Collections each carry a "View all N" link to a dedicated full-view page (§5); Recents does not (it's inherently capped at 3, there is nothing to view all of that Library doesn't already show).

Status is shown as an inline badge next to the arrow's name inside `ArrowTile`'s caption — not as a separate "Running now" list, and not as a corner overlay on the banner image (an earlier draft of this design used both; both were corrected during design review, see §5.1). The badge reuses `computeStatus`/`STATUS_BADGE_VARIANT`/`STATUS_ICONS`/`FlickerSpinner` from `src/features/arrow-details/lib/status.ts` and `src/components/ui/flicker-spinner.tsx` verbatim — the same pattern already built for the Hero's own status pill (`hero.tsx:163-172`). No badge renders for `ready`/`absent`/`removed` (the default, unremarkable case is silent).

Empty state (zero installed arrows and zero followed collections — a true first run): centered icon, one heading, one line of body copy, one primary CTA button. No section renders behind it.

---

## 3. Layout

- App shell: existing sidebar (unchanged, reused).
- Home itself has no page-level hero (unlike Arrow Details/Collection) — it starts directly with the first section. Section header: `text-[13px] font-semibold tracking-[-0.1px]` + `border-b border-border`, matching the existing convention in `collection-detail-screen`'s `.collection-section-title`/`.collection-section-head`.
- Library/Collections full-view pages: a small "< Home" back-link, then a page title at `text-[24px]/[28px] font-semibold tracking-[-0.4px]`. This is a **new** title scale — no existing page has a bare standalone page title to copy (Search has no title at all; Collection's `.collection-title` at 19px/23px is a hero sub-label sitting next to a banner and icon, the wrong reference point for a full-width page heading, which the design initially borrowed by mistake and then corrected).
- Full-view grids use `columnRule(total)` imported directly from `src/features/search/lib/columns.ts` — do not re-derive this logic; it already exists and is exactly what makes Recents "bigger for free" in §2.

---

## 4. Status badge → action mapping

**Correction from the design mockup, made while writing this spec:** the mockup made both `busy` (installing/updating/stopping/draining) and `problem` (detached) badges clickable, resolving the arrow to `ready` as an interaction demo. Cross-checking against `docs/arrow-details-spec.md` §8 (already verified against core's real source on the arrow-details branch): **core has no cancel/abort capability at all**, and `BeginStop.Validate` explicitly rejects a `stop` call while install/update is in flight. There is nothing to wire a `busy` badge's click to. Only `detached` has a real recovery path — a plain `stop`.

| Badge kind | States | Clickable | Core call |
|---|---|---|---|
| `up` | outdated | No | — (informational; the real Update action lives on the arrow's own detail page) |
| `active` | running | No | — |
| `busy` | installing / updating / stopping / draining | **No** (mockup had this wrong) | — (no cancel capability exists) |
| `problem` | detached | **Yes** | `useStop()` (already exists, `mutations/runtime.ts`) — the same plain Stop call Hero's own Detached recovery uses |

Clicking anywhere else on a tile does nothing on this page beyond a future navigation to `/arrow/$` — Home is an overview surface, not the arrow's action surface.

---

## 5. Component inventory

### 5.1 ArrowTile (new, shared)
The tile used by all three Home sections and both new full-view pages. Same visual shell as the existing `ArrowCard` (`src/features/search/components/card/arrow-card.tsx`) and `CollectionArrowTile` (`src/features/collection/components/collection-arrow-tile.tsx`) — both already share `card.css`'s `data-slot='arrow-card'`/`'card-banner'`/`'card-drawn'` mechanism by convention rather than by shared code (confirmed: `CollectionArrowTile`'s own comment says "Same shape as ArrowCard ... reusing card.css's real data-slot selectors"). `ArrowTile` becomes the third call site of that same convention, not a new pattern.

Props: `namespace`, `title`, `subtitle`, `metaText` (version, or "N arrows" for a collection), `monogram`, `chipColor` (both computed from `ArrowIcon`'s existing hash function — see §5.2), `statusKind` (`''|'up'|'active'|'busy'|'problem'`), `statusLabel`, `onResolve?` (only meaningful when `statusKind === 'problem'`, per §4).

Anatomy: `aspect-[2/1]` banner (real `banner` image or the drawn-monogram fallback, identical to `ArrowCard`), hover-reveal info strip (icon + version, unchanged from `ArrowCard`/`CollectionArrowTile`), then a caption with the name and status badge on one row (flex, name truncates, badge `shrink-0` — matching Hero's own name+badge placement in `hero.tsx:163-172`, not a novel layout) and the description below.

**Recommendation, not a requirement of this spec:** do not touch `ArrowCard`/`CollectionArrowTile` while building this — introduce `ArrowTile` fresh, to avoid regression risk on two already-shipped screens, and file a follow-up ticket to actually consolidate all three into one primitive. See §9, open question 4.

Suggested location: alongside `ArrowIcon`/`ArrowRow` in `src/features/sidebar/components/arrows/arrow-tile.tsx` — not a clean home architecturally, but it's where the codebase already puts cross-feature arrow-display components (`ArrowIcon` is used by sidebar, collection, and arrow-details already), so this keeps one convention instead of starting a second.

### 5.2 HomeScreen (new)
`src/features/home/home-screen.tsx`, replacing the placeholder in `src/routes/index.tsx`. Owns: reading `useArrowStore`, deriving Recents/Library-preview/section-presence, rendering the three sections and the empty state.

### 5.3 LibraryScreen (new)
`src/features/library/library-screen.tsx`, new route `src/routes/library.tsx`. Full grid of every arrow in `useArrowStore`, using real `columnRule`.

### 5.4 CollectionsScreen (new)
`src/features/collections/collections-screen.tsx` (plural — note the existing singular `collection` feature folder already exists for the detail page; keep these visually and structurally distinct in code, not just by name), new route `src/routes/collections.tsx`. Full grid from `useFollowedCollections()`.

### 5.5 EmptyHomeState (new)
Small presentational component: icon, heading, body line, one CTA button (§7.4).

---

## 6. Real codebase — what exists, what's missing

Verified directly against `src/` on `feature/home-page` and `feature/arrow-details`, and against `quiver.core`'s `develop` — not assumed.

### 6.1 Already wired — reuse as-is
- `src/lib/core-store/store/arrows.ts` — `useArrowStore`, the full reactive installed-arrow map, already kept live by the WS `/v0/runtime` listener. Home and Library read this directly; no new fetch.
- `src/features/search/lib/columns.ts` — `columnCap`/`columnRule`. Import directly for both full-view pages.
- `src/features/sidebar/components/arrows/arrow-icon.tsx` — `ArrowIcon`, including its hash-based monogram/chip-color function, reused for every tile's info-strip icon.
- `src/features/arrow-details/lib/status.ts` — `computeStatus`, `STATUS_BADGE_VARIANT`, `STATUS_ICONS` (once that branch merges).
- `src/components/ui/flicker-spinner.tsx`, `src/components/ui/frame.tsx` (once that branch merges).
- `src/lib/core-store/mutations/runtime.ts` — `useStop()`, for the detached badge's click (§4).
- `src/features/search/styles/card.css` — the banner hover/drawn-fallback mechanics, reused verbatim by `ArrowTile`.

### 6.2 Needs new mutation/query hooks
- **`useFollowedCollections()`** — does not exist. `GET /v0/collection?followed=true` is real and already supported by quiver.core (`internal/api/v0/endpoints/collections/`); only `useCollectionDetail(namespace)` (a single collection) exists today in `lib/core-store/queries/collection.ts`. Add a list hook mirroring that file's own pattern, returning `namespace, name, description, tags, media, arrow_count, followed`.
- No new arrow-side hook needed for Library/Recents — `useArrowStore` already covers it.

### 6.3 Needs to be built fresh (exists nowhere yet)
- **`ArrowTile`** (§5.1).
- **`/library`, `/collections` routes** — new TanStack Router file-based routes; neither exists today (confirmed route table: `/`, `/search`, `/collection/$`, `/arrow/$`, `/settings`, `/remote`).
- **Recents' real ordering** — blocked on a quiver.core field that doesn't exist yet (§8).

### 6.4 Domain/DTO type gaps to close
- `ArrowEntry` (`src/domain/arrow.ts`) needs a `last_used_at: string | null` field once the core PR (§8) ships. Extend `ArrowEntry` directly (not a heavier `ArrowDetail`-only field) — Home needs this for every library arrow at once, not one arrow at a time.
- `src/domain/collection.ts` — confirm it already carries `arrow_count`/`followed` on the list shape (core's `CollectionListItemDTO` has both); extend if the frontend type is currently narrower.
- Mock backend (`src/lib/mock/server/handlers/index.ts`, `src/lib/mock/world/*`): verify `GET /v0/collection?followed=true` filtering is actually implemented in the mock (not just the real handler), and add mock `last_used_at` data once the domain field exists.

### 6.5 Design-token gaps
None. Unlike arrow-details (which needed `Frame`/`FlickerSpinner` ported fresh), this feature deliberately reuses existing tokens and components end to end. The one net-new visual element — the inline status badge — is `Badge` plus the icon/spinner components arrow-details is already building; no new CSS variables are needed.

---

## 7. Behavior specs

### 7.1 Section omission
Each section (Recents/Library/Collections) renders nothing — not a header, not an empty-state message — when its source list is empty. This is the load-bearing progressive-disclosure mechanic the whole design rests on; do not replace it with a collapsed placeholder.

### 7.2 Recents — ordering and pre-core-field behavior
Sort by `last_used_at` descending; arrows with a null value are excluded outright, not sorted last. **Until the quiver.core PR in §8 ships, keep the Recents section hidden entirely** rather than substituting `installed_at` as a stand-in — a different signal silently relabeled as "recently used" would misrepresent what the section means and need a correction later once real data exists. This is a recommendation, not yet product-confirmed — see §9, open question 1.

### 7.3 Library / Collections default order
Recommended, not yet signed off (§9, open question 2): Library alphabetical by name ascending; Collections by `followed_at` descending (most-recently-followed first — real, already-available data, unlike arrow-run recency).

### 7.4 Empty-state CTA
The CTA button needs a real destination; the mockup's button is inert. Two viable mechanisms, not decided by the design: (a) navigate to `/search`, or (b) focus the sidebar's search input in place via a small shared store action. Recommend (a) — lower risk, no new cross-component plumbing. See §9, open question 3.

---

## 8. Dependencies (the answer to "is there a blocker")

**No hard blocker.** Two real dependencies, both already scoped:

1. **quiver.core: add `last_used_at` to arrows** — a separate PR against `quiver.core`, already fully scoped in this conversation (exact structs and file:line citations: `internal/domain/arrow.go`'s `Arrow` struct; the stamp site in `internal/app/repositories/runtime/internal/hooks.go`'s `stampCatalog`, parallel to the existing `MethodInstall`/`MethodUninstall` cases; the read-model column addition in `internal/app/repositories/arrow/internal/store/internal/storage/rows.go` plus a migration; four DTOs across `internal/app/models/` and `internal/api/v0/dto/`; `docs/asyncapi/asyncapi.yaml`'s `ArrowEvent` schema; and three prose docs). Only Recents depends on this — Library, Collections, the empty state, and the status-badge work are fully buildable today. Recommend sequencing Recents last, gated on this field's availability (§7.2).
2. **`feature/arrow-details` merge** — `Frame`/`FramePanel`, `FlickerSpinner`, and `status.ts` are introduced on that branch and don't exist on `develop` yet. Branch this feature from (or rebase onto) `arrow-details` once it merges, rather than duplicating those files.

---

## 9. Explicitly deferred (do not build this revision)

- Any curated/editorial "featured" or "trending" content — quiver.core has no such concept (verified: no endpoint, no ranking signal beyond search relevance/stars).
- Real pagination for Library/Collections beyond the responsive grid — full-view pages render everything the API/store returns today.
- A settings surface for configuring which sections appear on Home, or their item counts.
- Cancel-in-progress and reattach-detached as *new* capabilities — Home's badges only surface states core already has; this doesn't change what's actionable, consistent with `docs/arrow-details-spec.md` §8's own non-goals.

---

## 10. Open questions (need product/backend sign-off)

1. Recents sequencing (§7.2/§8): ship it hidden until the core field lands, or hold the whole Home rebuild until then? Recommend the former.
2. Library/Collections default sort order (§7.3): sign off on alphabetical / `followed_at`-desc, or specify a different order.
3. Empty-state CTA mechanism (§7.4): navigate to `/search`, or focus-in-place.
4. `ArrowTile` consolidation (§5.1): build fresh now plus a follow-up ticket (recommended), or take on consolidating `ArrowCard`/`CollectionArrowTile` inside this same feature — bigger scope, touches two already-shipped screens.

---

## 11. Testing

Repo convention (confirmed in `collection`/`search`): co-located `*.test.tsx`/`*.test.ts`, not a separate `__tests__` directory. `npm run doctor` / `npm run test:coverage` enforce a 95% floor across every metric including branches.

Minimum coverage this feature needs beyond line/branch %:
- All four badge kinds (+ none) rendered correctly on `ArrowTile`, including the busy-state `FlickerSpinner`.
- Click-through gating from §4: only `problem` resolves via `useStop()`; `up`/`active`/`busy` do nothing.
- Section presence/omission at zero content, for all three Home sections independently.
- `columnCap`/`columnRule` integration on both full-view pages (not a re-test of the function itself, which already has its own coverage).
- Mock backend updates for `?followed=true` filtering, exercised before component tests depend on it.

---

## 12. Implementation plan

Suggested order — each phase should land buildable/testable on its own:

1. Confirm `feature/arrow-details` is merged, or rebase this feature onto it (§8).
2. Build `ArrowTile` (§5.1), reusing `card.css` mechanics verbatim.
3. Build `useFollowedCollections()` (§6.2) plus any DTO/type extensions (§6.4) and matching mock backend support.
4. Build `HomeScreen`: Library and Collections sections wired to real data; Recents built but rendered hidden pending the core field (§7.2). Wire into the `/` route, replacing the placeholder.
5. Build `/library` route + `LibraryScreen` on real `columnRule`.
6. Build `/collections` route + `CollectionsScreen`.
7. Wire the detached badge's click to `useStop()` (§4).
8. Build `EmptyHomeState` and its CTA (§7.4).
9. Once quiver.core's `last_used_at` PR ships: extend `ArrowEntry`, wire Recents' real sort, unhide the section.
