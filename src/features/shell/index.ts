export { ROW_H, railOwnsControls, windowControls } from './lib/geometry';
export type { SidebarSide } from './lib/geometry';

export { useShellStore, SIDEBAR_MIN, SIDEBAR_MAX, SIDEBAR_DEFAULT } from './stores/shell-store';
export type { ShellState } from './stores/shell-store';

export { installThemeSync, useThemeStore } from './lib/theme';
export type { ThemePreference } from './lib/theme';

export { AppShell } from './components/app-shell';
export { ChromeRow } from './components/chrome-row';
export { WindowControls } from './components/window-controls';
