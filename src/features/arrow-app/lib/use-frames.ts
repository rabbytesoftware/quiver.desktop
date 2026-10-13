/** Which iframe belongs to which arrow host. A plain object so it can be unit
 *  tested and shared with the bridge without re-rendering. */
export function createFrameRegistry() {
	const frames = new Map<string, HTMLIFrameElement>();
	return {
		set(host: string, el: HTMLIFrameElement | null) {
			if (el) frames.set(host, el);
			else frames.delete(host);
		},
		windowOf: (host: string): Window | undefined => frames.get(host)?.contentWindow ?? undefined,
		hosts: () => [...frames.keys()],
	};
}

export type FrameRegistry = ReturnType<typeof createFrameRegistry>;
