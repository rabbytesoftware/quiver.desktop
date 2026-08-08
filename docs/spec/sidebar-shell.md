# Quiver Desktop — Sidebar & Shell

## Overview

The rail and the window chrome are one grid, not a sidebar sitting next to a title bar. Building
them as two things gets the pixels wrong and collapses entirely once the rail can move to the
right-hand side or the window controls move to the other edge — both of which are requirements.

This document records the decisions behind that layout, including the small ones. **Every numbered
rule below is deliberate and load-bearing.** Several of them look like arbitrary values or
over-specified CSS until you hit the case they exist for; each one names that case. A rule that
gets "simplified" away will not fail a test — it will fail on one platform, at one rail side, at
one window width.

Related specs: [ipc-bridge.md](ipc-bridge.md), and
[sidebar-implementation.md](sidebar-implementation.md) — how this gets built, in what order, and the
ten rules below that were reversed or dissolved while planning it. Those are amended in place here;
the implementation doc's §9 lists them with reasoning.

---

## 1. The grid

```
grid-template-columns: var(--rail) minmax(0, 1fr)    /* reversed when side = right */
grid-template-rows:    var(--row)  1fr
```

| Element | Column | Row |
| --- | --- | --- |
| Rail | 1 | **1 / 3** |
| Chrome row | 2 | 1 |
| Content | 2 | 2 |

**1.1 — The rail always spans the full webview height.** `grid-row: 1 / 3` in every combination.
Its first row holds the history buttons, and on macOS the reserved traffic-light space as well; there
is never a blank band held open above it. The only reason to reserve that space is that something is
actually sitting in it. (Webview, not window: on Windows and Linux the OS draws its title bar above
the webview entirely, which is not ours to span.)

**1.2 — The chrome row belongs to the content column only.** With the search field living there
(§4), the rail is free to reach the top whenever it does not own the controls.

**1.3 — The divider rides the rail**, as `border-right` flipping to `border-left` with the side.
Putting it on the content column was wrong: the content only occupies row 2, so the divider stopped
short of the top.

---

## 2. Window chrome

**macOS is frameless; Windows and Linux keep their native title bar.** This reverses an earlier
position that all three platforms draw their own chrome. Hand-drawn window controls lose hover
glyphs, tiling behaviour, inactive-window states and every OS convention we would then owe forever.
macOS is the only platform offering a third way — `titleBarStyle: "Overlay"` with `hiddenTitle` hides
the bar and keeps the real buttons — so it is the only platform where frameless costs nothing. On
Windows, hiding the bar removes the buttons with it.

**2.1 — The window is opaque.** Crowbar's *layout* pattern comes across — the column owns its
strip, no standalone `Titlebar` component, a spacer element reserves the control space rather than
padding. Its *transparency* stack does not: `transparent: true`, `macos-private-api` and
`window-vibrancy` are all deliberately absent. `src-tauri/Cargo.toml` already says so in as many
words, and the rail's opaque surface would defeat vibrancy anyway.

**2.2 — Which edge holds the controls is a value, not a conditional.**

```ts
// features/shell/geometry.ts
export type SidebarSide = 'left' | 'right'

export function windowControls(): { edge: 'left' | 'right'; kind: 'reserve'; width: number } | null
//   macOS        → { edge: 'left', kind: 'reserve', width: 64 }
//   Win / Linux  → null   (the OS draws its own title bar)

export function railOwnsControls(side: SidebarSide): boolean
```

`kind: 'reserve'` renders an empty spacer — the OS paints the lights over it. We never render window
buttons ourselves. Taking `side` as a parameter rather than closing over a module global keeps this
pure, so every cell below is unit-testable without a window.

| Platform | Rail side | Controls render in | Rail's first row |
| --- | --- | --- | --- |
| macOS | Left | rail | 64px reserve → history |
| macOS | Right | chrome row (inside the field) | **history** |
| Win / Linux | Either | nowhere — the OS owns them | **history** |

`isMacOS()` is called in exactly one place. Anything else that needs to know asks `windowControls()`
or `railOwnsControls(side)`.

**2.3 — There is no frameless configuration.** ~~`decorations: false` in the shared config with
macOS restating true.~~ Dissolved: Windows and Linux keep native decorations, so `decorations` stays
true everywhere and defaults there. The shared `tauri.conf.json` is untouched, and
`tauri.macos.conf.json` remains the only platform overlay file — the invariant `titlebar.test.tsx`
exists to protect. Adding `tauri.windows.conf.json` would silently drop title, size, minimums and
`backgroundThrottling` on that platform, which is exactly the bug that file caused before.

**2.4 — `trafficLightPosition.y` is derived, not chosen.** It is `(ROW_H - 12) / 2` — the lights
are 12px and `y` is their top edge. At `--row: 34` that is **11**. Change the row height and this
moves with it or the buttons sit visibly off-centre.

**2.5 — `titlebar.test.tsx` is retargeted, not inverted.** An earlier draft had its whole
`native window decorations` block reversing. It does not: `decorations` stays true on every platform,
macOS remains the only overlay file, and the comment stating that "custom in-webview window controls
were ruled out" is true again. Only the row-height coupling moves — `h-12` / `y: 18` becomes
`--row: 34` / `y: 11`.

**2.6 — There is nothing to spike.** ~~Spike frameless behaviour before building the rail.~~
Dissolved with §2.3. The resize border, the snap-layouts flyout and the per-compositor Linux
behaviour are all still the OS's to provide, because we never take the title bar away.

---

## 3. Sizing

```css
--rail: 246px;
--row:  34px;
--icon: 20px;         /* content — arrow marks in the list */
--icon-chrome: 17px;  /* chrome glyphs — back / forward */
--inset: calc((var(--row) - var(--icon)) / 2);   /* 7px at 34 / 20 */
```

**3.1 — One row height, no exceptions.** The search field, each nav segment, every arrow row, the
rail's top bar and the chrome row are all `--row`. Committing to one number is what makes the column
read as a single instrument; the alternative is four values nobody chose and everybody maintains.

**3.2 — THREE icon tiers, deliberately.** `--icon` 20 for the arrow rows, `--icon-chrome` 17 for the
back / forward glyphs, `--icon-nav` **14** for the primary nav. Each carries its own token so it
holds independently of the others.

The nav's 14 is the one that gets missed — it was, in the first build, and the segments came out
wearing the arrow rows' 20. A nav segment is mostly icon, so at 20 the glyph fills a collapsed
segment edge to edge and the three of them read as three toolbar buttons rather than as one
segmented control. The design draws the nav at 14 and the list at 20 for exactly that reason.

**3.3 — `--inset` is derived, and governs three distances**: the row's leading and trailing padding,
the gap between icon and label, and the active nav segment's padding. It is not a spacing scale — it
is the value that makes the icon sit the same distance from the row's leading edge as from its top
and bottom. Hardcoding 7px works until `--row` or `--icon` moves, at which point the icon is
silently off-centre again.

**3.4 — Dissolved.** ~~Caption buttons are 46px wide (the Windows convention) by full row
height.~~ There are no caption buttons; see §2.2.

**3.5 — History buttons are square**, `--row × --row`, so they match the rail's rhythm exactly.

**3.6 — `SIDEBAR_MIN` is 160, derived from the nav's collapse point.** With the active segment
capped at 54% and two collapsed segments at their `--row` floor, `0.54W + 2·34 ≤ W` gives `W ≥ 148`.
The subtitle imposes no floor of its own, because it degrades rather than truncating (§5.11). The old
120 dates from a 208px rail with the search field inside it and must not be carried over.
`SIDEBAR_MAX` stays 320.

---

## 4. The search field

**4.1 — It lives in the chrome row, spanning the content column** — not in the rail. It was tried in
the rail; at 208px it had ~130px of text room and collided with the active nav pill (§4.4).

**4.2 — The field *is* the chrome row.** In the one combination that puts the traffic lights in
that column (macOS, rail right), the lights' reserved space sits **inside** the field, on its surface,
rather than beside it. Anything else leaves reserved space on bare `--background` next to a different
surface, and the seam reads as a notch cut out of the bar.

**4.3 — The control set supplies its own inset, and the field drops its padding on that side.**
`cn(leading && 'pl-0')`. Otherwise you get the field's 12px plus the lights' 11px stacked as a double
gap.

**4.4 — Dissolved.** ~~Controls take their host's colour: `color: inherit`, never a fixed
`--foreground`.~~ There are no glyphs in the field to go white-on-white — macOS's reserve is an empty
spacer and no other platform puts anything there. The principle survives for the ⌘K hint, which is
text and inherits by default.

**4.5 — Exactly one of the three combinations puts anything in the field**: macOS with the rail on
the right. In the other two the rail owns the reserve, or there is nothing to own.

**4.6 — The placeholder is "Search".** Not "Search your library".

**4.7 — The rail does not filter.** Search is global — its results belong in the content column.
There is therefore no "no matches" state in the rail, and typing does not disturb the library list.

---

## 5. The rail

**5.1 — One selection for the whole rail, and it lives in the router.** Home / Remote / Settings
and the arrow rows are the same navigation, so exactly one thing is active at any time.

Every rail row is a destination, so each is a TanStack `<Link>` and the router marks the one that
matches with `data-status="active"`. There is no selection store and no derived-selection function —
the invariant stops being something we enforce and becomes something we cannot violate. ~~`type
Active = { kind: 'nav' | 'arrow'; i: number }`~~

Home needs `activeOptions={{ exact: true }}`: TanStack matches by prefix, so `/` otherwise lights up
on every route and this rule is broken on the first click. Nothing matches `/search`, which is
correct — the field's own inversion is what is lit while searching.

**5.2 — The active treatment is identical for a nav segment and an arrow row**: `--foreground`
background, `--background` foreground. They respond the same because they *are* the same thing.

**5.3 — Selecting an arrow collapses every nav segment to an icon.** No segment keeps the wide slot
"to stop the geometry jumping" — that was tried and rejected; it reads as Home still being active.

**5.4 — The nav always spans the full rail width.**

```css
.pnav > [data-status="active"]       { flex: 1 1 auto; max-width: 54%; }
.pnav > :not([data-status="active"]) { flex: 1 1 0;    min-width: var(--row); }
```

The active segment grows with the rail but stops at **54%** — the proportion the design gives it
(112 of 208). Without the cap it eats the whole rail at wide settings; sized to its label alone it is
too tight. The other segments split the remainder, so there is never dead space. With nothing active,
all three match the second rule and share equally — which is why no separate "none active" selector
is needed.

The collapsed floor is `--row` (34), not the design's 44: at 34 the segment is square like a history
button (§3.5) and `--inset` centres its icon on all four sides literally rather than approximately.

**5.5 — Nav segments are flush.** The design's `gap: 4` existed only to make `112 + 4 + 44 + 4 + 44`
come out to exactly 208. Nothing else in the rail is separated, and the active segment absorbs the
difference.

**5.6 — One hover token, `--sidebar-accent`, across arrow rows, nav segments and history buttons.**
The active element is **excluded** from the rule
(`not-data-[status=active]:not-disabled:hover:bg-sidebar-accent`) rather than overridden by a later
one, so it cannot flicker as the cursor crosses it.

**5.7 — Dissolved.** ~~Caption buttons do not share `--row-hover`; they use a translucent tint.~~
There are no caption buttons (§2.2). The technique survives where a tint must work over two surfaces:
`bg-current/10` resolves against whatever `currentColor` is, so it needs no token of its own.

**5.8 — Back / forward live in the rail's top bar, and always face the content.** Right edge of a
left rail, left edge of a right one — never sharing an edge with the traffic lights. The principle:
**the window's edge belongs to the OS, the interior belongs to the app.** One ternary in
`RailTopBar`: the reserve (when there is one) hugs the window edge, the history buttons the interior.

They drive `router.history`. `canGoBack()` exists and `canGoForward()` does not, so back disables
correctly and forward stays enabled — shadow-tracking an index to grey out a button is more state
than a no-op click is worth.

**5.9 — Re-selecting what is already active does not push a history entry.** ~~Or clicking Home
twice leaves you with a back button that appears to do nothing.~~ **That example is wrong, and a test
written from it proves nothing:** router-core's `commitLocation` already short-circuits when the URL
and state are both identical, so clicking Home twice pushes nothing with no code at all.

The reachable case is a row that is active at a location its href does not equal — which is the
rail's normal state, because a rail link carries no search params and the router marks a link active
on a path prefix with a *subset* of the search. `/settings?tab=developer` is that case in this app
today. The guard is `preventDefault()` on the router's own `data-status`, not a `replace` prop:
`replace` is decided at render, so choosing it needs `isActive` outside `<Link>`'s children function
— a second matcher with its own spelling of `exact`/`fuzzy`, free to disagree with the
`activeOptions` on the very link it is attached to.

**5.10 — The namespace appears as a subtitle on the selected row only, without changing row
height.** The label is a centred flex column; the subtitle is `display: none` until selected, and
the name re-centres upward to make room. 13px/1.25 + 10px/1.25 = 28.75px inside 34.

**5.11 — The subtitle shows the WHOLE namespace, and sheds the middle of the path first.** This
reverses an earlier rule that showed only the parent namespace on the grounds that the full versioned
key could not fit. It can: split at the last `@`, let the head truncate and pin the tail.

```tsx
<span className="flex min-w-0">
  <span className="truncate">{head}</span>   {/* github.com/rabbyte/minecraft */}
  <span className="shrink-0">{tail}</span>   {/* @v1.21.4                    */}
</span>
```

```
246px (default)   github.com/rabbyte/minecraft@v1.21.4
~200px            github.com/rabbyte/minecr…@v1.21.4
160px (min)       github.com/rabb…@v1.21.4
```

The version — the useful end — is never what gets dropped. No measurement, no `ResizeObserver`, and
it reflows live while the resize handle is dragged. The full namespace fits above roughly 190px.

**5.12 — The resize handle's drag direction flips with the side.** Dragging right grows a left rail
and shrinks a right one. One sign flip, and exactly the kind of thing that ships broken because
nobody switches the setting while testing. **Test both directions.**

---

## 6. Tokens

**6.1 — `src/index.css` and `docs/pen.dev/design.pen` are not the same palette. Unresolved, but it
does NOT block the shell.**

| | `index.css` | `design.pen` |
| --- | --- | --- |
| `--primary` | `oklch(0.97 0 0)` grey | **`#FF8400`** orange |
| `--destructive` | `oklch(0.65 0 0)` grey | `#FF5C33` |
| `--sidebar` | `oklch(0.18 0 0)` | `#18181b` |
| `--background` | `oklch(0.16 0 0)` | `#111111` |
| semantic colours | none | success / warning / error / info, each with a foreground |
| radius | `0` only | `--radius-none: 0`, `--radius-m: 16`, `--radius-pill: 999` |

`index.css` is committed monochrome and its comments defend grey-as-destructive on principle. The
design has an orange primary and a full semantic set. These are two different positions on whether
Quiver has an accent colour, not drift in a couple of values.

**The rail uses no accent anywhere**, so the shell can be built without settling this — and must be
built without touching `--primary`, or it settles it by accident. What it does gate: run state (§9.4)
and any component restyled after this one.

**6.2 — Values below are read from `design.pen`.** Dark first, light second. They ship as oklch on
shadcn's existing token names — no new colour tokens; see sidebar-implementation.md §5.2 for the
converted values.

| Element | Dark | Light | Ships as |
| --- | --- | --- | --- |
| Rail surface | ~~`#242424 → #1A1A1A`~~ flat midpoint | ~~`#F4F4F4 → #E9E9E9`~~ flat midpoint | `--sidebar` |
| Row label | `#FAFAFA` | `#1A1A1A` | `--sidebar-foreground` |
| Row hover | `#111111` | `#F5F5F5` | `--sidebar-accent` |
| Row / nav **active** | bg `--foreground`, fg `--background` | same | `--sidebar-primary` / `-foreground` |
| Divider | `#2E2E2E` | `#E4E4E4` | `--sidebar-border` |
| Search idle | `#111111D9` | `#FFFFFFD9` | `bg-background/85` |
| Search active | the same inversion as an active row | same | `--foreground` / `--background` |
| Placeholder, ⌘K | `#A8A8A8` | `#8A8A8A` | `--muted-foreground` |

The rail's gradient is not built (§5.7 of sidebar-implementation.md); the flat value is the
perceptual midpoint of the two stops. The search field's focused state is **not** `--primary` — that
is the accent slot §6.1 has not decided, and using it here would settle that by accident.

**6.3 — The HOVERED row is `--background`.** It punches through to the content column's colour, so
a row under the cursor reads as connected to what it would open. It is not an alpha overlay. (This
rule previously said "selected", contradicting §5.2 and §6.2's own table — the selected row is the
`--foreground` / `--background` inversion.)

**6.4 — The idle field needs no token of its own.** `--background-translucent` (`#111111D9`) is
`--background` at 85%, which is `bg-background/85` — plus `backdrop-filter: blur(14px)`, which works
fine over an opaque window because it blurs our own content, not the desktop. Do not invent a
`--toolbar-input`.

**6.5 — The search field inverts in *both* directions**: white on dark, near-black on light. It is
not "goes white on focus".

**6.6 — The traffic lights are never ours to colour.** macOS draws them.

---

## 7. Structure

```
src/features/shell/            window geometry — no data, no queries
  components/{app-shell,chrome-row,window-controls}.tsx
  geometry.ts                  windowControls(), railOwnsControls(side), ROW_H
  store.ts                     sidebarSide, sidebarWidth — persisted

src/features/sidebar/          the rail
  components/{sidebar,rail-top-bar,primary-nav,nav-segment,arrow-list,
              arrow-row,arrow-icon,history-nav,resize-handle}.tsx

src/features/search/           the field
  components/search-bar.tsx
  index.ts                     public surface — no store; the query lives in ?q=
```

**7.1 — A feature may import another feature's `index.ts`, never its internals.**
`src/lib/core-store/index.ts` already works exactly this way, re-exporting a curated set rather than
letting callers reach into `store/` and `mutations/`. ~~The sidebar renders the search field, so this
rule gets its first real test here.~~ It does not — `ChromeRow` renders the field. The rule's first
real test is the rail importing `@/features/shell` to ask which side it is docked to, and that import
is also why the shell and the sidebar form a deliberate cycle (`shell/index → app-shell →
sidebar/index → sidebar → shell/index`). It is safe because `geometry` and `store` are fully
evaluated before `app-shell` in the index's import order and every component is a hoisted function
declaration — but it is inherent to the design, not an accident: the shell mounts the rail, and the
rail asks the shell where it is.

**7.2 — `src/components/titlebar.tsx` moves into `features/shell`.** Its comment block is the best
documentation in the repo of why macOS is special; move the reasoning across and amend it (§2.5),
do not lose it in a rename.

**7.3 — Delete `src/store/ui.ts`, do not migrate it.** Zero consumers outside its own test. Width
belongs to the shell store; selection belongs to the router (every rail row is a route, so the active
row is `data-status="active"` and no selection state exists at all).
`navMode: 'home' | 'arrow' | 'search'` matches neither the design nor §5.1 and must not be carried
forward.

**7.4 — Use shadcn primitives; do not use shadcn's `Sidebar` block.** Confirmed against the
`base-vega` registry rather than assumed:

| | ships on base-vega | npm deps | registry deps |
| --- | --- | --- | --- |
| `tooltip` | yes | none | none |
| `scroll-area` | yes | none | none |
| `sidebar` | yes — **21,958 chars** | none | button, input, separator, **sheet**, skeleton, tooltip, **use-mobile** |

The block brings a `Sheet` and a `use-mobile` hook into a desktop app with a `minWidth: 800` window,
plus a provider, cookie persistence, off-canvas and icon-collapse modes and its own keyboard
shortcut. Hand-roll the rail; spend shadcn on `Tooltip` and `ScrollArea`, both of which are thin
wrappers over `@base-ui/react` and add nothing to `package.json`.

The search field does **not** use `components/ui/input.tsx`. Its `h-9 rounded-md border shadow-xs
focus-visible:ring-3` is cancelled by every rule the field needs (34px, radius 0, `bg-background/85`,
inverts on focus rather than growing a ring), so it would be a primitive imported to be overridden.

**7.5 — i18n is a prerequisite and is already in place.** Every user-facing string goes through
`@/lib/i18n`; a missing key is a compile error. See `src/lib/i18n/locales/en.ts` for how to add one.

---

## 8. Departures from `design.pen`

The design file is now behind the decisions above. Bring it into line before it stops being useful
as a reference.

| | Design | Built |
| --- | --- | --- |
| Row height | 28 rows / 34 strip | **34 throughout** |
| Rail width | 208 | **246** |
| Rail extent | starts below the toolbar | **full webview height** |
| Nav gap | 4 | **0** |
| Collapsed nav segment | 44 wide | **`--row` (34), square** |
| Rail surface | gradient `#242424 → #1A1A1A` | **flat, the perceptual midpoint** |
| Icon size | 17 | **20** (chevrons 17) |
| Namespace subtitle | — | **added** |
| Back / forward | — | **added** |
| Windows / Linux | native decorations | **native decorations** — no departure |

---

## 9. Open questions

Five of the original seven are answered or dissolved. What remains does not block the build.

1. **`index.css` vs `design.pen` palette** (§6.1). Still open — two positions on whether Quiver has an
   accent colour. **No longer blocking:** the rail uses no accent, so the shell is built without
   touching `--primary`. It gates run state (4) and anything restyled after this.
2. ~~**What are Remote and Settings?**~~ **Answered: destinations.** Every rail row is a route,
   including Home, Remote and Settings; arrows route straight to the arrow. The Settings dialog
   retires into `/settings` with its three panels intact. Selection therefore lives in the router and
   nowhere else.
3. **What else fills the chrome row** beside the search field. Still open, and now independent of (2).
4. **Run state in the rail.** `ArrowEntry` carries `state`, `active_run` and `last_return`; the rail
   draws what you *have*, not what is *running*, and the design shows no state at all. Not this
   branch. Whatever answers it wants the accent colour from (1).
5. ~~**Caption button fidelity.**~~ **Dissolved** — Windows and Linux keep native decorations, so
   there are no caption buttons to colour and no Linux button-set question.
6. **The arrow icon's empty state.** `ArrowEntry.icon` is `string | null` and the design draws no
   fallback. A `--icon`-sized tile carrying the name's first letter is a proposal, not a decision.
