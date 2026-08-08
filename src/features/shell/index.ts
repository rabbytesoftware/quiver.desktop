// The shell's public surface. Named re-exports rather than `export *`, so
// adding a helper to a module does not silently widen what the rest of the app
// is allowed to reach for — same rule as `@/lib/core-store`.
//
// `WindowControls` leaves here as the COMPONENT. `geometry.ts` declares an
// interface of the same name — `windowControls()`'s return type — and both
// cannot use one name on this surface. The component is the one anything
// outside the feature has a use for; the interface reaches every caller by
// inference already, so exporting it too would only mean two different things
// answering to `WindowControls` depending on which import a file happened to
// write. It stays exported from `geometry.ts` for use inside the feature.

export { ROW_H, railOwnsControls, windowControls } from './geometry';
export type { SidebarSide } from './geometry';

export { useShellStore, SIDEBAR_MIN, SIDEBAR_MAX, SIDEBAR_DEFAULT } from './store';
export type { ShellState } from './store';

export { AppShell } from './components/app-shell';
export { ChromeRow } from './components/chrome-row';
export { WindowControls } from './components/window-controls';
