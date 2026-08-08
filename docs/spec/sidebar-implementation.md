# Quiver Desktop — Sidebar & Shell: Implementation

How the decisions in [sidebar-shell.md](sidebar-shell.md) get built. That document records _what_ the
shell looks like and why; this one records _how_ it is assembled, what it costs, and in what order.

Six of that document's rules were reversed or dissolved while planning this. They have been amended
in place rather than left to contradict — see [§9](#9-what-changed-in-sidebar-shellmd) for the list
and the reasoning, so nobody re-derives a decision that was already made and unmade.

**Scope.** This branch builds the shell: the grid, the rail, the search field, and five navigable
routes. It does not build what those routes _show_. `/`, `/remote` and `/arrow/$` render placeholders;
`/settings` renders the panels that already exist. The content column is separate work.

---

## 1. Routes and selection

**1.1 — Every row in the rail is a destination.** Home, Remote, Settings and each arrow are all
routes. Clicking one navigates.

```
src/routes/
  __root.tsx     AppShell wraps <Outlet/>
  index.tsx      Home              — needs activeOptions={{ exact: true }}
  remote.tsx     Remote
  settings.tsx   the three existing panels, Dialog shell removed
  search.tsx     results, ?q=
  arrow.$.tsx    splat
```

**1.2 — There is no selection state, and no derived-selection function.** Every row is a TanStack
`<Link>`; the router sets `data-status="active"` on the one that matches. §5.1's "exactly one thing
is active" stops being an invariant we enforce and becomes one we cannot violate.

Three of the original rules evaporate as a result:

- **§5.3** — an arrow is open, so no nav link matches, so all three segments hit the collapsed rule.
  No code.
- **§5.4** — "nothing active" needs no separate selector, exactly as predicted, because it is just
  three inactive links.
- **§5.9** — re-selecting the active row must not push history. TanStack pushes duplicates by
  default, so this is one `replace: true` guard, not a comparison against stored state.

**1.3 — Home needs `activeOptions={{ exact: true }}`.** TanStack's link matching is prefix-based, so
`/` matches every route in the app. Without this, Home is lit on top of whatever else is lit, and
§5.1 is broken on the first click.

**1.4 — Arrow namespaces need a splat, not a param.** The key is
`github.com/rabbyte/minecraft@v1.21.4` — slashes and an `@` inside a single identifier. `/arrow/$`
carries it verbatim; `/arrow/$namespace` would need encoding on the way in and decoding on the way
out of every link, and would show up percent-mangled in devtools.

**1.5 — The Settings dialog retires into `/settings`.** `general.tsx`, `connections.tsx` and
`developer.tsx` move to the content column unchanged — they do not know what wraps them.
`settings-dialog.tsx` and its `Dialog` shell go; `useSettingsUI` keeps `tab` and `query` and loses
`open`. `MockIndicator`'s "Turn off" link and `__root.tsx`'s temporary corner button both become
navigations. `components/ui/dialog.tsx` stays — other features will want it.

**1.6 — The search query lives in the URL, not a store.** `?q=` on `/search` is the single source of
truth, which is what makes a search result linkable and the back button meaningful.

**1.7 — The first keystroke pushes; the rest replace.** Navigating to `/search` from wherever you
were is a push, so back returns you there. Every subsequent keystroke is
`navigate({ replace: true })`. Pushing on each one buries the previous page under twenty history
entries and makes the back button useless — which is the same failure §5.9 guards against, arrived at
from the other direction.

---

## 2. Module layout

```
src/features/shell/          window geometry — no data, no queries
  geometry.ts                ROW_H, windowControls(), railOwnsControls(side)
  store.ts                   sidebarSide, sidebarWidth — persisted
  components/app-shell.tsx   the grid; owns the geometry custom properties
  components/chrome-row.tsx
  components/window-controls.tsx
  index.ts

src/features/sidebar/        the rail
  components/{sidebar,rail-top-bar,primary-nav,nav-segment,
              arrow-list,arrow-row,arrow-icon,history-nav,resize-handle}.tsx
  index.ts

src/features/search/         the field
  components/search-bar.tsx
  index.ts
```

**2.1 — A feature may import another feature's `index.ts`, never its internals.** The sidebar renders
the search field, which is the first real test of this rule (§7.1). `src/lib/core-store/index.ts`
already works this way and is the pattern to copy.

**2.2 — `features/search` has no store.** Per §1.6 the query is a URL search param. A store here
would be a second copy of it, and the two would disagree the moment someone navigates with the back
button.

**2.3 — `src/store/ui.ts` is deleted, not migrated.** It has zero consumers outside its own test.
Its `sidebarWidth` becomes the shell store's; its `selectedNamespace` and `navMode` are answered by
the router (§1.2) and should not be carried forward — `navMode: 'home' | 'arrow' | 'search'` matches
neither the design nor §5.1.

**2.4 — `src/components/titlebar.tsx` is deleted, and its reasoning is not.** The component's job is
absorbed by `app-shell.tsx` and `window-controls.tsx`. Its comment block is the best explanation in
the repo of why macOS is special; it moves to `window-controls.tsx` intact. Rename the file and the
reasoning survives; delete the file and it does not.

---

## 3. Component tree

```
<AppShell>                    grid; owns --rail --row --icon --icon-chrome --inset
  <Sidebar>                   grid-row 1 / 3; --sidebar; border on the content edge
    <RailTopBar>              row 1 — never blank (§1.1)
      <WindowControls/>         only when railOwnsControls(side)
      <HistoryNav/>             always, on the content-facing edge
    <PrimaryNav/>             Home | Remote | Settings
    <ArrowList/>              ScrollArea of <ArrowRow>
    <ResizeHandle/>
  <ChromeRow>                 col 2, row 1
    <SearchBar/>                is the row (§4.2)
  <main>                      col 2, row 2
    <MockIndicator/>
    <Outlet/>
```

**3.1 — `MockIndicator` goes inside the content column, below the chrome row.** Not a third grid
row. It describes the data, and the data lives there; a dev-only band has no business in the window
chrome. This also keeps the grid at 2×2 in every combination.

**3.2 — `TooltipProvider` mounts once, in `AppShell`.**

---

## 4. Window chrome

**macOS is frameless; Windows and Linux keep their native title bar.** This reverses §2.2's premise
that all three platforms draw their own chrome.

The reasoning is the one that kept macOS's real traffic lights: hand-drawn window controls lose hover
glyphs, tiling behaviour, inactive-window states and every OS convention we would then owe forever.
macOS is the only platform that offers a third option — `titleBarStyle: "Overlay"` with `hiddenTitle`
hides the bar and keeps the real buttons — so it is the only platform where frameless costs nothing.
On Windows, hiding the title bar removes the buttons with it, and drawing replacements means matching
Segoe Fluent glyphs, Windows 11 hover geometry and inactive states by hand, while losing the
snap-layouts flyout unless someone writes a `WM_NCHITTEST` handler in Rust.

```ts
// features/shell/geometry.ts
export const ROW_H = 34;

export type SidebarSide = 'left' | 'right';

export interface WindowControls {
	edge: 'left' | 'right';
	kind: 'reserve'; // the OS paints over our spacer; we never render buttons
	width: number;
}

export function windowControls(): WindowControls | null {
	return isMacOS() ? { edge: 'left', kind: 'reserve', width: 64 } : null;
}

export function railOwnsControls(side: SidebarSide): boolean {
	const controls = windowControls();
	return controls !== null && controls.edge === side;
}
```

| Platform    | Rail   | Rail's row 1           | Chrome row              |
| ----------- | ------ | ---------------------- | ----------------------- |
| macOS       | left   | 64px reserve → history | search                  |
| macOS       | right  | history                | lights reserve → search |
| Win / Linux | either | history                | search                  |

**4.1 — `railOwnsControls` takes the side as a parameter.** §2.2 had it closing over a module-level
`sidebarSide`. As a parameter it is a pure function and every cell of the table above is unit-testable
without a window.

**4.2 — `isMacOS()` is called in exactly one place.** Anything else in the tree that needs to know
asks `windowControls()` or `railOwnsControls(side)`. Scattering the platform test through JSX cannot
express the table above without contradicting itself somewhere.

**4.3 — The only configuration change in the project is `trafficLightPosition.y: 18 → 11`.** The
shared `tauri.conf.json` is untouched; `decorations` stays true everywhere and defaults there.
`tauri.macos.conf.json` remains the only platform overlay file, which is the invariant
`titlebar.test.tsx` exists to protect.

**4.4 — `y` is derived, not chosen.** `(ROW_H - 12) / 2` — the lights are 12px and `y` is their top
edge. At `--row: 34` that is 11. It lives in a JSON file that cannot see a stylesheet, so a test
reads both sides and fails when they disagree.

**4.5 — The whole of row 1 is a window drag handle, on every platform.**
~~Drag regions on macOS only.~~ macOS hides its title bar under
`titleBarStyle: "Overlay"` and takes every draggable surface with it, so without
this the top of the window is dead and the only way to move it is the 64px the
lights sit on. Nothing about the page looks wrong — the window simply will not
move.

`data-tauri-drag-region` goes on the rail's top bar, on its spacer (most of the
row's width, and a click lands on whichever element is actually under it), and
on the search plate. Tauri dispatches on the event TARGET, so the two history
buttons and the search input stay interactive by being their own targets.

**The input drags too, but on a threshold rather than on the attribute.** Tauri's
own handler fires on `mousedown` with no slop, so `data-tauri-drag-region` on the
field would take the press away from focusing it and from selecting the text in
it — and that failure reads as "the search box is broken", not as "the drag
region is too greedy". `dragWindowOrFocus` resolves the same gesture by what the
pointer does next: press and release focuses, press and move past 4px calls
`startDragging()`. That is what makes the whole 828px row grabbable rather than
the ~70px the field leaves over.

**A focused field is the exception, and it is the one that matters.** Once the
input has focus the handler returns immediately, so drag-to-select works. Miss
that and dragging across a query to select it throws the window across the
desktop.

Not on `<body>` — `index.html` already records why — and not on any button.

The reserve carries `data-slot="window-controls"` as well, because
`data-tauri-drag-region` stopped identifying it the moment the rest of the row
became a drag surface too.

**4.6 — Four capability permissions become unused.** `allow-minimize`, `allow-toggle-maximize`,
`allow-close` and `allow-is-maximized` were granted for caption buttons that will not exist. Only
`allow-start-dragging` is still needed. Trim them; an unused permission in a capability file is the
first thing a reviewer asks about.

---

## 5. Tokens

**5.1 — Geometry is CSS custom properties, consumed as `h-(--row)` and `p-(--inset)`.**

```css
--rail: 246px;
--row: 34px;
--icon: 20px; /* content — arrow marks in the list */
--icon-chrome: 17px; /* chrome glyphs — back / forward */
--inset: calc((var(--row) - var(--icon)) / 2); /* 7px at 34 / 20 */
```

`--inset` governs three distances: the row's leading and trailing padding, the gap between icon and
label, and the collapsed segment's padding. Write `p-[7px]` anywhere and that stops being true
silently — no test fails, the icon just drifts off-centre the next time `--row` or `--icon` moves.

This is idiomatic to shadcn, not a departure from it: their own Sidebar block defines
`--sidebar-width` and `--sidebar-width-icon` as plain custom properties driven from TS constants.

**5.2 — No new colour tokens. Eleven existing ones are retuned to `design.pen`'s values, in oklch.**

`index.css` already declares this policy in its own header — _"shadcn's token NAMES with this
design's values"_. Every role the design names already had a token waiting for it; `--sidebar-border`
was already exactly `#2E2E2E`.

```
                              .dark               :root
--background                  oklch(0.178 0 0)    oklch(1.000 0 0)
--foreground                  oklch(1.000 0 0)    oklch(0.145 0 0)
--border                      oklch(0.301 0 0)    oklch(0.919 0 0)
--muted-foreground            oklch(0.784 0.004 121.6)  oklch(0.633 0 0)
--sidebar                     oklch(0.239 0 0)    oklch(0.951 0 0)
--sidebar-foreground          oklch(0.985 0 0)    oklch(0.218 0 0)
--sidebar-accent              oklch(0.178 0 0)    oklch(0.970 0 0)
--sidebar-accent-foreground   oklch(0.985 0 0)    oklch(0.218 0 0)
--sidebar-primary             oklch(1.000 0 0)    oklch(0.145 0 0)
--sidebar-primary-foreground  oklch(0.178 0 0)    oklch(1.000 0 0)
--sidebar-border              oklch(0.301 0 0)    oklch(0.919 0 0)
```

**5.3 — `--primary` is not touched, deliberately.** It is the accent slot, and whether Quiver has an
accent is exactly what §6.1 has not decided. An earlier draft of this plan mapped the search field's
focused state onto `--primary`, which would have quietly pre-empted that decision. The focused field
uses `--foreground` / `--background` — the same inversion as an active row, which is what §5.2 says
it is.

**5.4 — Two proposed tokens are redundant; Tailwind's alpha modifier covers them.**
`--background-translucent` (`#111111D9`) is `bg-background/85`. `--wctl-hover` is `bg-current/10`,
which resolves against whatever `currentColor` happens to be and therefore works on both surfaces
§5.7 worried about.

**5.5 — `--sidebar-accent` equals `--background` in both modes.** Not a coincidence — it is §6.2's
hover punching through to the content column's colour.

**5.6 — `--muted-foreground` keeps `design.pen`'s `#B8B9B6` verbatim**, chroma and all
(`oklch(0.784 0.004 121.6)` — a whisper of green). Flattening it to `0` would be invisible, and would
also be us editing the design instead of implementing it.

**5.7 — The rail is flat, not a gradient.** `design.pen` draws `#242424 → #1A1A1A`; the built rail is
the perceptual midpoint, `oklch(0.239 0 0)`. Taking either endpoint would change the rail's weight
against the content column.

**5.8 — Three comments in `index.css` describe a shell that does not exist**, and the block is being
rewritten anyway:

- `background: transparent` "or macOS vibrancy dies" — nothing enables vibrancy; there is no
  `window-vibrancy` dependency and `Cargo.toml` explicitly declines `macos-private-api`;
- "26px buttons, 30px rail rows" — the rows are 34;
- "every hue flattened to a grey" — no longer true after 5.6.

---

## 6. The rail

**6.1 — The collapsed segment's floor is `--row`, not the design's 44px.** At 34 it is square like a
history button (§3.5) and `--inset` centres the icon on all four sides literally rather than
approximately. The design's arithmetic (`112 + 4 + 44 + 4 + 44 = 208`) no longer applies anyway: the
rail is 246 and the gap is gone.

```
active     flex: 1 1 auto;  max-width: 54%     /* the design's 112/208 */
collapsed  flex: 1 1 0;     min-width: var(--row)
```

**6.2 — `SIDEBAR_MIN` is 160, derived from the nav's collapse point.**

```
0.54W + 2·34 ≤ W   →   W ≥ 148   →   160
```

The old 120 dates from a 208px rail with the search field inside it and must not be carried over.
`SIDEBAR_MAX` stays 320.

**6.3 — The namespace subtitle shows the whole namespace, and sheds the middle of the path first.**
This inverts §5.11, which said to show the parent namespace _because_ the full key could not fit. The
rule is to show as much as fits and give up the least useful part first — and the version is the
useful end.

```tsx
const at = ns.lastIndexOf('@');
const head = at === -1 ? ns : ns.slice(0, at); // github.com/rabbyte/minecraft
const tail = at === -1 ? '' : ns.slice(at); // @v1.21.4

<span className="flex min-w-0">
	<span className="truncate">{head}</span>
	<span className="shrink-0">{tail}</span>
</span>;
```

```
246px (default)   github.com/rabbyte/minecraft@v1.21.4
~200px            github.com/rabbyte/minecr…@v1.21.4
160px (min)       github.com/rabb…@v1.21.4
```

No measurement, no `ResizeObserver`, and it reflows live while the resize handle is being dragged.
The full namespace fits at every width above roughly 190px.

**6.4 — The subtitle does not change the row's height** (§5.10). The label is a centred flex column;
the subtitle is `hidden group-data-[status=active]:block` and the name re-centres upward around it.
13px/1.25 + 10px/1.25 = 28.75px inside 34.

**6.5 — Arrow rows sort by `name` through `localeCompare` with the active locale.** `useArrowStore`
hands back a `Map`, whose order is insertion order — which is to say, whatever the catalog stream
happened to do.

**6.6 — Back disables; forward does not.** TanStack's history exposes `canGoBack()` and no
`canGoForward()`. Shadow-tracking an index to grey out the forward button is more state than a
no-op click is worth.

**6.7 — One hover rule, with active and disabled excluded rather than overridden.**
`not-data-[status=active]:not-disabled:hover:bg-sidebar-accent` across arrow rows, nav segments and
history buttons. Excluding is what keeps it from flickering as the cursor crosses the active row;
a later rule that merely overrides it does not.

**6.8 — Tooltips only on collapsed segments.** The active segment shows its label, so it suppresses
its own.

**6.9 — The resize handle's drag direction flips with the side.**
`delta = side === 'left' ? dx : -dx`. The width drives a CSS custom property live during the drag and
commits to the persisted store on pointer-up; committing on every `pointermove` writes to
localStorage sixty times a second.

---

## 7. Testing

**7.1 — What earns a test.**

|                                                                 | Why                                                                                                        |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `geometry.ts`, all platform × side combinations                 | The whole matrix is one function. `runningOn()` already exists in `src/__mocks__/user-agent.ts`            |
| `trafficLightPosition.y === (ROW_H - 12) / 2`                   | Two files that cannot see each other. `titlebar.test.tsx` already does this for the 48px bar — retarget it |
| Exactly one `data-status="active"` per route; zero at `/search` | The invariant that replaced the store, and the only thing that catches §1.3's prefix-match trap            |
| Resize sign flip, **both directions**                           | Ships broken otherwise — nobody switches the setting while testing                                         |
| Width clamps to [160, 320]                                      |                                                                                                            |
| The head/tail namespace split                                   | With `@ref`, without, and `@` inside a path                                                                |
| §5.9's `replace: true` guard                                    | Clicking the active row must not grow history                                                              |

**7.2 — Every pixel detail in this document is verified by eye, not by CI.** jsdom has no layout
engine; `getBoundingClientRect` returns zeros. The 54% cap, the flush nav, `--inset` centring the
icon and the subtitle's truncation ladder are all invisible to the suite. Asserting `--row: 34px` in
a test is the stylesheet agreeing with itself. The Tauri MCP tools — screenshot, computed styles,
resize — drive the real window during development; that is verification, not a committed test.

**7.3 — Branch coverage is the constraint, not line coverage.** The gate is 95% on all four metrics
and branches currently sit at 95.71%, a 0.71-point cushion. The shell is branchy by nature —
platform × side × active × disabled — so every branch needs a test or the gate trips on the first
commit. Build the geometry tests first for that reason alone.

**7.4 — Two exclusions already work in our favour.** `src/routes/**` is excluded from coverage, so
route files cost nothing; `src/components/ui/**` is excluded as vendored, so Tooltip and ScrollArea
need no tests at all.

**7.5 — jsdom does not implement `Element.prototype.setPointerCapture`.** The resize handle's tests
need a stub alongside the existing `setup-local-storage.ts`.

---

## 8. Build order

Three units. All of them are testable on macOS, which is what removing the frameless work bought.

**Unit 1 — Foundation.** No visible change.

- `features/shell/geometry.ts` and its tests
- `features/shell/store.ts` — `sidebarSide`, `sidebarWidth`, persisted (copy the `partialize` /
  `merge` pattern from `lib/i18n/store.ts`)
- `index.css` — geometry properties, the eleven retuned values, the three stale comments
- delete `src/store/ui.ts` and `src/store/ui.test.ts`
- `tauri.macos.conf.json` — `y: 11`; retarget `titlebar.test.tsx`
- trim the four unused capability permissions
- `shadcn add tooltip scroll-area` (both confirmed on `base-vega`; no npm dependencies, no registry
  dependencies)
- message keys for every new string

**Unit 2 — Shell.** Visible.

- `app-shell.tsx`, `chrome-row.tsx`, `window-controls.tsx`, `search-bar.tsx`
- routes `remote.tsx`, `settings.tsx`, `search.tsx`, `arrow.$.tsx`
- Settings dialog → route (§1.5)
- `__root.tsx` rewritten; `titlebar.tsx` deleted and its reasoning relocated (§2.4)
- `MockIndicator` into the content column

**Unit 3 — Rail.**

- `sidebar.tsx`, `rail-top-bar.tsx`, `primary-nav.tsx`, `nav-segment.tsx`, `history-nav.tsx`,
  `arrow-list.tsx`, `arrow-row.tsx`, `arrow-icon.tsx`, `resize-handle.tsx`

---

## 9. What changed in sidebar-shell.md

Amended in place. Recorded here so a decision that was made and unmade is not re-derived.

| Rule  | Was                                              | Now                                                               | Why                                                              |
| ----- | ------------------------------------------------ | ----------------------------------------------------------------- | ---------------------------------------------------------------- |
| §2.2  | four cells, `kind: 'reserve' \| 'render'`        | three cells, returns `null` off macOS                             | Windows/Linux keep native decorations (§4)                       |
| §2.3  | `decorations: false` shared, macOS restates true | no config change at all                                           | nothing to make frameless                                        |
| §2.5  | `titlebar.test.tsx` inverts                      | its config assertions stand; only the row-height coupling moves   | "custom in-webview controls were ruled out" is true again        |
| §2.6  | spike frameless before building the rail         | dissolved                                                         | nothing to spike                                                 |
| §3.4  | caption buttons are 46px wide                    | dissolved                                                         | there are no caption buttons                                     |
| §4.4  | `color: inherit` on the controls                 | dissolved                                                         | macOS's reserve is an empty spacer; no glyphs in the field       |
| §5.7  | caption hover needs its own token                | dissolved                                                         | `bg-current/10` (§5.4), and nothing to hover anyway              |
| §5.11 | show the parent namespace                        | show the whole namespace, shed the path's middle first            | the version is the useful end, and it can be kept (§6.3)         |
| §6.3  | "the **selected** row is `--background`"         | hover is `--background`; selected is the `--foreground` inversion | §5.2 and §6.2's table always said so; §6.3 named the wrong state |
| §9.5  | red close-hover, Linux button set                | dissolved                                                         | no buttons to colour                                             |

---

## 10. Open items

None of these block the three units above.

1. **`index.css` vs `design.pen`'s `--primary`** (§6.1). Still two positions on whether Quiver has an
   accent colour. It does not gate the shell — the rail uses no accent anywhere — but it gates run
   state (§9.4) and any component restyled after this.
2. **What `/`, `/remote` and `/arrow/$` show.** Out of scope here; they render placeholders.
3. ~~**The arrow icon's empty state.**~~ **Answered by the CossUI rebuild (§11).** `ArrowIcon` is
   Base UI's `Avatar`: the manifest's image when one resolves, a two-glyph monogram otherwise, in an
   18px box. Base UI holds the `<img>` out of the DOM until it loads, so a URL that 404s stays on the
   monogram rather than leaving a broken-image glyph in the rail.
4. **What else sits in the chrome row** beside the search field (§9.3).
5. **Run state in the rail** (§9.4). The rail lists what you have, not what is running, and
   `ArrowEntry` carries `state`, `active_run` and `last_return` that nothing displays.

---

## 11. Amendment — the CossUI rebuild (2026-08-08)

The rail was rebuilt on the `@coss` registry (style `base-nova`), the component set Crowbar uses.
Plan: `docs/superpowers/plans/2026-08-08-cossui-sidebar-migration.md`. What changed against the
three units above:

|              | Before                                    | After                                                         | Why                                                                                                                                                                                  |
| ------------ | ----------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| row chrome   | per-component class strings               | `src/features/sidebar/row-base.ts`                            | Ported from crowbar's `workspace-row-base.ts`. Several call sites draw rows; a change has to reach all of them.                                                                      |
| row height   | `h-(--row)`, 34px                         | `h-9`, 36px                                                   | ROW_BASE's own height. The window chrome row keeps `--row`, because it aligns with the content column and the traffic lights.                                                        |
| radius       | `0`                                       | `rounded-lg` = `--radius` = 10px                              | §6.1a of the shell spec.                                                                                                                                                             |
| selected row | `--sidebar-primary` fill                  | `bg-foreground` / `text-background`, border matching the fill | Same inversion, now stated once in `ROW_ACTIVE`. Crowbar's own `ROW_ACTIVE` _raises_ from `--background`; that was deliberately not taken — see the file's comment.                  |
| arrow icon   | hand-rolled `<img>` / tile                | `Avatar` + `AvatarFallback`, 18px                             | Handles a broken URL, which the `<img>` did not. Needs an explicit `role="img"` on the fallback: Base UI renders it as a bare span and `aria-label` on a generic element is ignored. |
| history nav  | bare `<button>`                           | `Button variant="ghost"`                                      | One control vocabulary with the rest of the app.                                                                                                                                     |
| arrow list   | `overflow-y-auto` + `::-webkit-scrollbar` | `ScrollArea`                                                  | Those pseudo-elements are ignored the moment anything sets `scrollbar-width`.                                                                                                        |
| subtitle     | `text-[10px]` UI sans                     | `ROW_SUBLABEL`, JetBrains Mono 10.5px                         | Mono is for identifiers only. The head/tail split of §5.11 is unchanged.                                                                                                             |

**Not changed:** the search bar (§4), except to collapse a `cn` workaround that only existed because
`tailwind-merge` 1.14.0 could not parse Tailwind v4 syntax. The namespace split (§5.11), the
reselect guard, the resize handle and the shell grid are untouched.

**Two defects found while verifying on the live app.** The switch had no `rounded-*` at all — stripped
when the app was pinned at radius 0 — and was fixed. The slider's `data-horizontal:` variants have
never matched: Base UI emits `data-orientation="horizontal"`, so its track has been 0-height since
`daa8c46`. That one is **pre-existing and unfixed** — out of scope for the rail, but it is a real
broken control on `/settings`.
