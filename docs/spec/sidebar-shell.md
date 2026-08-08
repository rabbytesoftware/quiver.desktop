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

Related specs: [ipc-bridge.md](ipc-bridge.md).

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

**1.1 — The rail always spans the full window height.** `grid-row: 1 / 3` in every combination.
Its first row is either the window controls or the primary nav; there is never a blank band held
open above it. The only reason to reserve that space is that something is actually sitting in it.

**1.2 — The chrome row belongs to the content column only.** With the search field living there
(§4), the rail is free to reach the top whenever it does not own the controls.

**1.3 — The divider rides the rail**, as `border-right` flipping to `border-left` with the side.
Putting it on the content column was wrong: the content only occupies row 2, so the divider stopped
short of the top.

---

## 2. Window chrome

Quiver draws its own chrome on every platform. macOS keeps its real traffic lights, positioned by
the OS over space we reserve; Windows and Linux go frameless and get caption buttons we render.

**2.1 — The window is opaque.** Crowbar's *layout* pattern comes across — the column owns its
strip, no standalone `Titlebar` component, a spacer element reserves the control space rather than
padding. Its *transparency* stack does not: `transparent: true`, `macos-private-api` and
`window-vibrancy` are all deliberately absent. `src-tauri/Cargo.toml` already says so in as many
words, and the sidebar's opaque gradient would defeat vibrancy anyway.

**2.2 — Which edge holds the controls is a value, not a conditional.**

```ts
// features/shell/geometry.ts
export type SidebarSide = 'left' | 'right'
export type ControlsKind = 'reserve' | 'render'

export function windowControls(): { edge: 'left' | 'right'; kind: ControlsKind; width: number }
//   macOS        → { edge: 'left',  kind: 'reserve', width: 64  }
//   Win / Linux  → { edge: 'right', kind: 'render',  width: 138 }

export const railOwnsControls = () => windowControls().edge === sidebarSide
```

`kind: 'reserve'` renders an empty spacer — the OS paints the lights over it. `kind: 'render'`
renders three real buttons wired to `getCurrentWindow().minimize() / toggleMaximize() / close()`.
Same seam, two implementations, one place to look. Scattering `isMacOS()` through the JSX cannot
express the four combinations without contradicting itself somewhere.

| Platform | Rail side | Controls render in | Rail's first row |
| --- | --- | --- | --- |
| macOS | Left | rail | 64px reserve |
| macOS | Right | chrome row | **nav** |
| Win / Linux | Left | chrome row | **nav** |
| Win / Linux | Right | rail | 3 caption buttons |

**2.3 — Frameless is configured from the shared config, not a new overlay.** Put
`decorations: false` in `tauri.conf.json` and have the existing `tauri.macos.conf.json` restate
`decorations: true`. macOS needs it true — `titleBarStyle: "Overlay"` hides the bar but keeps the
buttons, and `decorations: false` would take the buttons with it. Doing it this way preserves the
invariant `titlebar.test.tsx` already guards: **macOS remains the only platform with an overlay
file.** Adding `tauri.windows.conf.json` would silently drop title, size, minimums and
`backgroundThrottling` on that platform, which is exactly the bug that file caused before.

**2.4 — `trafficLightPosition.y` is derived, not chosen.** It is `(ROW_H - 12) / 2` — the lights
are 12px and `y` is their top edge. At `--row: 34` that is **11**. Change the row height and this
moves with it or the buttons sit visibly off-centre.

**2.5 — This reverses a recorded decision.** `titlebar.test.tsx` currently asserts that `Titlebar`
renders `null` off macOS, that `decorations` stays true everywhere, and states in a comment that
"custom in-webview window controls were ruled out". Its whole `native window decorations` block
inverts. **Rewrite its comments, not only its expectations** — a reader who finds the old reasoning
next to new assertions learns something false.

**2.6 — Spike frameless behaviour before building the rail.** `decorations: false` on Windows
historically costs the resize border and the snap-layouts flyout on the maximize button; on Linux it
varies by compositor. Tauri v2 handles more of this than v1 did, not all of it. Throwaway window,
drag, resize from all eight edges, double-click-to-maximise, snap layouts. Cheap on day one,
miserable at the end.

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

**3.2 — Two icon tiers, deliberately.** Content icons at 20, chrome glyphs one step down at 17. The
chevrons carry their own token so they hold at 17 independently of the list.

**3.3 — `--inset` is derived, and governs three distances**: the row's leading and trailing padding,
the gap between icon and label, and the active nav segment's padding. It is not a spacing scale — it
is the value that makes the icon sit the same distance from the row's leading edge as from its top
and bottom. Hardcoding 7px works until `--row` or `--icon` moves, at which point the icon is
silently off-centre again.

**3.4 — Caption buttons are 46px wide** (the Windows convention) by full row height. Width and
height are on different axes here; only the height follows `--row`.

**3.5 — History buttons are square**, `--row × --row`, so they match the rail's rhythm exactly.

**3.6 — `SIDEBAR_MIN` needs re-deriving.** The thresholds in `src/store/ui.ts` (120 min) date from a
208px rail with the search field inside it. With the field on top, the constraints are the nav's
collapse point and the namespace subtitle's legibility. Do not carry the old numbers over.

---

## 4. The search field

**4.1 — It lives in the chrome row, spanning the content column** — not in the rail. It was tried in
the rail; at 208px it had ~130px of text room and collided with the active nav pill (§4.4).

**4.2 — The field *is* the chrome row.** When the controls land in that column they render **inside**
the field, on its surface, rather than beside it. Anything else leaves the controls on bare
`--background` next to a different surface, and the seam reads as a notch cut out of the bar.

**4.3 — Each control set supplies its own inset, and the field drops its padding on that side.**
Otherwise you get the field's 12px plus the lights' 11px stacked as a double gap.

**4.4 — Controls take their host's colour: `color: inherit`, never a fixed `--foreground`.** This is
load-bearing. The field inverts on focus (§5.4); a caption glyph pinned to `--foreground` goes
white-on-white the instant someone clicks the search box.

**4.5 — Only two of the four combinations put controls in the field**: macOS + rail right (lights
lead) and Windows/Linux + rail left (buttons trail). In the other two the rail owns them and paints
them with its own gradient.

**4.6 — The placeholder is "Search".** Not "Search your library".

**4.7 — The rail does not filter.** Search is global — its results belong in the content column.
There is therefore no "no matches" state in the rail, and typing does not disturb the library list.

---

## 5. The rail

**5.1 — One selection for the whole rail.** Home / Remote / Settings and the arrow rows are the same
navigation, so exactly one thing is active at any time:

```ts
type Active = { kind: 'nav' | 'arrow'; i: number }
```

Selecting an arrow clears the nav segment and vice versa. You cannot be on Home *and* have an arrow
selected. This is also what gives back/forward (§5.8) an unambiguous meaning — it would not have one
if the nav and the list held separate selections.

**5.2 — The active treatment is identical for a nav segment and an arrow row**: `--foreground`
background, `--background` foreground. They respond the same because they *are* the same thing.

**5.3 — Selecting an arrow collapses every nav segment to an icon.** No segment keeps the wide slot
"to stop the geometry jumping" — that was tried and rejected; it reads as Home still being active.

**5.4 — The nav always spans the full rail width.**

```css
.pnav button[data-wide="true"]       { flex: 1 1 auto; max-width: 54%; }
.pnav button:not([data-wide="true"]) { flex: 1 1 0; min-width: 44px; }
```

The active segment grows with the rail but stops at **54%** — the proportion the design gives it
(112 of 208). Without the cap it eats the whole rail at wide settings; sized to its label alone it
is too tight. The other segments split the remainder, so there is never dead space. With nothing
active, all three match the second rule and share equally — which is why no separate "none active"
selector is needed.

**5.5 — Nav segments are flush.** The design's `gap: 4` existed only to make `112 + 4 + 44 + 4 + 44`
come out to exactly 208. Nothing else in the rail is separated, and the active segment absorbs the
difference.

**5.6 — One hover token, `--row-hover`, across arrow rows, nav segments and history buttons.** The
active element is **excluded** from the rule (`:not([aria-current="true"])`, `:not(:disabled)`)
rather than overridden by a later one, so it cannot flicker as the cursor crosses it.

**5.7 — Caption buttons deliberately do *not* share `--row-hover`.** They sit on two different
surfaces depending on placement — the rail's gradient or the search field's plate — and a solid fill
only works against one of them. They use a translucent tint instead.

**5.8 — Back / forward live in the rail's top bar, and always face the content.** Right edge of a
left rail, left edge of a right one — never sharing an edge with the traffic lights or the caption
buttons. The principle: **the window's edge belongs to the OS, the interior belongs to the app.**
DOM order (`lights, histnav, wctl`) already achieves this when the rail is on the right; only the
mirror case needs `margin-left: auto`.

**5.9 — Re-selecting what is already active does not push a history entry**, or clicking Home twice
leaves you with a back button that appears to do nothing.

**5.10 — The namespace appears as a subtitle on the selected row only, without changing row
height.** The label is a centred flex column; the subtitle is `display: none` until selected, and
the name re-centres upward to make room. 13px/1.25 + 10px/1.25 = 28.75px inside 34.

**5.11 — The subtitle shows the parent namespace**, e.g. `github.com/rabbyte`, not the full
versioned key. The row already names the arrow, so `…/minecraft` under "Minecraft Server" is
redundant, and the full `github.com/rabbyte/minecraft@v1.21.4` cannot fit at this width — truncating
it drops the version, which is the useful end.

**5.12 — The resize handle's drag direction flips with the side.** Dragging right grows a left rail
and shrinks a right one. One sign flip, and exactly the kind of thing that ships broken because
nobody switches the setting while testing. **Test both directions.**

---

## 6. Tokens

**6.1 — `src/index.css` and `docs/pen.dev/design.pen` are not the same palette. This is unresolved
and blocks the restyling work.**

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
Quiver has an accent colour, not drift in a couple of values. **Pick one and delete the other before
building.** It is upstream of §6.5 and of how run state is eventually shown (§8).

**6.2 — Values below are read from `design.pen`.** Dark first, light second.

| Element | Dark | Light |
| --- | --- | --- |
| Rail gradient | `#242424 → #1A1A1A` | `#F4F4F4 → #E9E9E9` |
| Row label | `#FAFAFA` | `#1A1A1A` |
| Row hover | `#111111` | `#F5F5F5` |
| Row / nav **active** | bg `--foreground`, fg `--background` | same |
| Search idle | `--background-translucent` `#111111D9` | `#FFFFFFD9` |
| Search active | `#FFFFFF` | `#151515` |
| Placeholder, ⌘K | `#A8A8A8` | `#8A8A8A` |

**6.3 — The selected row is `--background`.** It punches through to the content column's colour, so
a selected row reads as connected to what it opened. It is not an alpha overlay.

**6.4 — `--background-translucent` already exists in the design.** Do not invent a
`--toolbar-input`. The idle field is that token plus `backdrop-filter: blur(14px)` — which works
fine over an opaque window because it blurs our own content, not the desktop.

**6.5 — The search field inverts in *both* directions**: white on dark, near-black on light. It is
not "goes white on focus".

**6.6 — The traffic lights are never ours to colour.** macOS draws them.

---

## 7. Structure

```
src/features/shell/            window geometry — no data, no queries
  components/{app-shell,chrome-row,window-controls,caption-buttons}.tsx
  geometry.ts                  windowControls(), railOwnsControls(), ROW_H
  store.ts                     sidebarSide, sidebarWidth — persisted

src/features/sidebar/          the rail
  components/{sidebar,primary-nav,nav-segment,arrow-list,arrow-row,
              arrow-icon,history-nav,resize-handle}.tsx
  store.ts                     the single `active` selection

src/features/search/           the query
  components/search-bar.tsx
  store.ts
  index.ts                     public surface
```

**7.1 — A feature may import another feature's `index.ts`, never its internals.**
`src/lib/core-store/index.ts` already works exactly this way, re-exporting a curated set rather than
letting callers reach into `store/` and `mutations/`. The sidebar renders the search field, so this
rule gets its first real test here.

**7.2 — `src/components/titlebar.tsx` moves into `features/shell`.** Its comment block is the best
documentation in the repo of why macOS is special; move the reasoning across and amend it (§2.5),
do not lose it in a rename.

**7.3 — Retire `src/store/ui.ts`.** It holds three unrelated things (`sidebarWidth`,
`selectedNamespace`, `navMode`) and has no production consumers — only its own test imports it.
Width belongs to the shell, selection to the sidebar. `navMode: 'home' | 'arrow' | 'search'` matches
neither the design nor §5.1 and should not be carried forward.

**7.4 — Use shadcn primitives; do not use shadcn's `Sidebar` block.** The block brings a provider,
off-canvas and icon collapse modes, a mobile `Sheet`, cookie persistence and its own keyboard
shortcut. The rail is a fixed grid column with a custom segmented nav, a capped active segment,
unified selection and a side-aware resize handle — the machinery does not apply and the parts that
do would be fought. Hand-roll the rail; spend shadcn on `Tooltip` (icon-only segments need hover
labels) and `ScrollArea`. Note this project is on **Base UI** (`base-vega`), not Radix — confirm
what that registry actually ships before planning around any component.

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
| Rail extent | starts below the toolbar | **full window height** |
| Nav gap | 4 | **0** |
| Windows / Linux | native decorations | **frameless, our caption buttons** |
| Icon size | 17 | **20** (chevrons 17) |
| Namespace subtitle | — | **added** |
| Back / forward | — | **added** |

---

## 9. Open questions

1. **`index.css` vs `design.pen` palette** (§6.1). Blocks restyling. Everything else here is decided.
2. **What are Remote and Settings?** The design has `Window Toolbar / Host Menu` and
   `Window Toolbar / Settings` variants, implying all three nav items are real destinations. But
   Settings is already a dialog. Proposal: Remote is a popover anchored to its segment (it is a host
   switcher, not a place); Settings stays the dialog. Only Home participates in §5.1.
3. **What fills the chrome row** beside the search field, if anything. The design's `Host Control`
   would sit there naturally — but that overlaps question 2.
4. **Run state in the rail.** `ArrowEntry` carries `running`, `installing`, `active_run` with steps,
   and the rail draws a list of things you *have* rather than things you are *running*. The design
   shows no state at all. Not this branch, but it is the gap the design has not answered — and
   whatever answers it will want the accent colour from question 1.
5. **Caption button fidelity.** Windows convention is a red `#C42B1B` close hover, which contradicts
   §6.1's monochrome position. Linux has no single convention (GNOME often close-only, KDE all
   three). Proposal: keep the red — it is OS chrome we are impersonating, not a Quiver statement —
   and ship the trio on both.
