// The search feature's public surface. Named re-exports rather than `export *`,
// so adding a helper to a module does not silently widen what the rest of the
// app may reach for — same rule as `@/lib/core-store` and `@/features/shell`.
//
// There is deliberately no store here. The query is `?q=` on `/search` and
// nothing else; see `components/search-bar.tsx` for what a second copy costs.

export { SearchBar } from './components/search-bar';
export type { SearchBarProps } from './components/search-bar';
