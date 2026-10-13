import { lazy, Suspense } from 'react';

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
