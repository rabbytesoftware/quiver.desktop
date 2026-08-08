// The shell's public surface. Named re-exports rather than `export *`, so
// adding a helper to a module does not silently widen what the rest of the app
// is allowed to reach for — same rule as `@/lib/core-store`.
//
// `AppShell`, `ChromeRow` and the `WindowControls` component join this list as
// Tasks 7 and 8 add them.

export { ROW_H, railOwnsControls, windowControls } from './geometry';
export type { SidebarSide, WindowControls } from './geometry';

export { useShellStore, SIDEBAR_MIN, SIDEBAR_MAX, SIDEBAR_DEFAULT } from './store';
export type { ShellState } from './store';
