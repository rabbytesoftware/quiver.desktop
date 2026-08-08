// The rail's public surface. Named re-exports rather than `export *`, so adding
// a helper to a module does not silently widen what the rest of the app is
// allowed to reach for — same rule as `@/features/shell` and `@/lib/core-store`.
//
// `Sidebar` is all of it, and deliberately: the shell mounts the rail whole,
// and every part it is assembled from — the top bar, the nav, the arrow list,
// the drag handle — is reachable only through it. That is what leaves the rail
// free to be rearranged without a grep across the app, and it is the rule
// spec §7.1 states: a feature may import another feature's `index.ts`, never
// its internals.

export { Sidebar } from './components/sidebar';
export type { SidebarProps } from './components/sidebar';
