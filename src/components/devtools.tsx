import { lazy, Suspense } from 'react';

/**
 * The TanStack Router devtools panel, in development builds only.
 *
 * `__root.tsx` used to import `TanStackRouterDevtools` statically and render it
 * unconditionally, and got away with it: this version of the package guards
 * ITSELF, exporting `() => null` from its barrel unless
 * `process.env.NODE_ENV === 'development'`, so Rollup already dropped the panel
 * from release builds. That is a third-party implementation detail, not a
 * contract — `TanStackRouterDevtoolsInProd` exists in the same barrel precisely
 * because which one you get is the package's choice to make — and "no debug
 * overlay in the shipped app" is too load-bearing to leave to it.
 *
 * So the decision is made here, on Vite's own build-mode flag:
 *
 *  - `import.meta.env.DEV` is replaced with a literal at build time (`true` for
 *    `vite` and `vite build --mode development`, `false` for a release `vite
 *    build`), so in a release bundle this function reduces to `return null`,
 *    `RouterDevtools` goes unreferenced, and everything the dead branch reached
 *    goes with it — which is why the import is dynamic and why the `lazy` call
 *    is marked pure. Without the annotation Rollup must assume `lazy()` has
 *    side effects, keeps the call, and emits the whole devtools import as a
 *    lazy chunk that nothing will ever load. Measured: with it, `vite build`
 *    transforms 186 modules and emits one JS file; without it, 195 and two.
 *  - it is read inside the component rather than at module scope so that both
 *    branches are reachable from a test — `vi.stubEnv('DEV', …)` can move it,
 *    where a module-scope `const` evaluated at import time could not.
 *
 * `lazy` + `Suspense fallback={null}` because the import is now async: the
 * panel appears a tick after first paint in dev, which is a tick nobody
 * watching a debug overlay will notice.
 */
const RouterDevtools = /*#__PURE__*/ lazy(() =>
	import('@tanstack/react-router-devtools').then((m) => ({ default: m.TanStackRouterDevtools }))
);

export function Devtools() {
	if (!import.meta.env.DEV) return null;

	return (
		<Suspense fallback={null}>
			<RouterDevtools position="bottom-right" />
		</Suspense>
	);
}
