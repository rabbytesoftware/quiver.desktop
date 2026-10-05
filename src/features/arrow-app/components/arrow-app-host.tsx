import { useCallback, useEffect, useMemo, useRef, useState, type JSX } from 'react';

import { useArrowStore } from '@/lib/core-store';

import { ArrowAppFrame } from './arrow-app-frame';
import { tauriApi } from '../lib/api';
import { createBridge } from '../lib/bridge';
import { arrowAppOrigin, isWindows, useSurfaceNamespaces } from '../lib/surface';
import { createFrameRegistry, type FrameRegistry } from '../lib/use-frames';
import { useOpenApps } from '../store/open-apps';

/** Each open arrow's host name, asked of the backend once per namespace. */
function useHosts(order: readonly string[]): Record<string, string> {
	const [hosts, setHosts] = useState<Record<string, string>>({});
	const asked = useRef(new Set<string>());

	useEffect(() => {
		for (const ns of order) {
			if (asked.current.has(ns)) continue;
			asked.current.add(ns);
			tauriApi.host(ns).then(
				(host) => setHosts((h) => ({ ...h, [ns]: host })),
				() => asked.current.delete(ns)
			);
		}
	}, [order]);

	return hosts;
}

function useBridge(registry: FrameRegistry) {
	const bridge = useMemo(
		() =>
			createBridge({
				api: tauriApi,
				frameWindow: registry.windowOf,
				hosts: registry.hosts,
				originOf: (host) => arrowAppOrigin(host, isWindows()),
			}),
		[registry]
	);

	useEffect(() => {
		window.addEventListener('message', bridge.onMessage);
		return () => {
			window.removeEventListener('message', bridge.onMessage);
			bridge.dispose();
		};
	}, [bridge]);

	return bridge;
}

interface FrameSlotProps {
	namespace: string;
	host: string | undefined;
	visible: boolean;
	reloadKey: number;
	onRef: (host: string, el: HTMLIFrameElement | null) => void;
	onLoad: (host: string) => void;
}

function FrameSlot({ namespace, host, visible, reloadKey, onRef, onLoad }: FrameSlotProps): JSX.Element | null {
	const surface = useArrowStore((s) => s.arrows.get(namespace)?.active_run?.surface);
	// Stable on purpose: React calls a changed ref callback with null first, and
	// that null is what closes the arrow's sockets.
	const ref = useCallback(
		(el: HTMLIFrameElement | null) => {
			if (host) onRef(host, el);
		},
		[host, onRef]
	);
	const loaded = useCallback(() => {
		if (host) onLoad(host);
	}, [host, onLoad]);
	// Mounted only once the arrow answers: a frame loaded earlier would sit on
	// the daemon's 502 page. Unmounting while not ready also gives a restarted
	// arrow a fresh load when it is ready again.
	if (!host || !surface?.ready) return null;

	return (
		<ArrowAppFrame
			host={host}
			path={surface.path}
			windows={isWindows()}
			visible={visible}
			reloadKey={reloadKey}
			onRef={ref}
			onLoad={loaded}
		/>
	);
}

/**
 * Keeps one iframe per open arrow alive above the router. Mounted once in
 * the shell: routes come and go, the apps do not.
 */
export function ArrowAppHost(): JSX.Element {
	const order = useOpenApps((s) => s.order);
	const visible = useOpenApps((s) => s.visible);
	const reloads = useOpenApps((s) => s.reloads);
	const live = useSurfaceNamespaces();
	const hosts = useHosts(order);
	const registry = useMemo(createFrameRegistry, []);
	const bridge = useBridge(registry);

	useEffect(() => useOpenApps.getState().prune(new Set(live)), [live]);

	// Unmounting a frame (pruned or reloaded) is the moment its sockets die.
	const onRef = useCallback(
		(host: string, el: HTMLIFrameElement | null) => {
			registry.set(host, el);
			if (!el) bridge.closeHost(host);
		},
		[registry, bridge]
	);

	return (
		<div className="pointer-events-none absolute inset-x-0 bottom-0 top-(--arrow-app-header)">
			{order.map((ns) => (
				<FrameSlot
					key={ns}
					namespace={ns}
					host={hosts[ns]}
					visible={ns === visible}
					reloadKey={reloads[ns] ?? 0}
					onRef={onRef}
					onLoad={bridge.frameLoaded}
				/>
			))}
		</div>
	);
}
