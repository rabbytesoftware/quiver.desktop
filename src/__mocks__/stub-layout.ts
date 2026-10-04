/**
 * jsdom lays nothing out, so a virtualized list sees a zero-size viewport and
 * zero-height rows and mounts either nothing or everything. This gives the
 * viewport a size and every row a height, which is all the virtualizer reads.
 * Returns the function that puts jsdom back.
 */
export function stubLayout(viewport = { width: 800, height: 400 }, row = 20): () => void {
	const proto = HTMLElement.prototype;
	const offsetHeight = Object.getOwnPropertyDescriptor(proto, 'offsetHeight');
	const offsetWidth = Object.getOwnPropertyDescriptor(proto, 'offsetWidth');
	const rect = proto.getBoundingClientRect;

	Object.defineProperty(proto, 'offsetHeight', { configurable: true, get: () => viewport.height });
	Object.defineProperty(proto, 'offsetWidth', { configurable: true, get: () => viewport.width });
	proto.getBoundingClientRect = function getBoundingClientRect(this: HTMLElement): DOMRect {
		const height = this.hasAttribute('data-index') ? row : viewport.height;
		return {
			x: 0,
			y: 0,
			top: 0,
			left: 0,
			bottom: height,
			right: viewport.width,
			width: viewport.width,
			height,
			toJSON: () => ({}),
		};
	};

	return () => {
		if (offsetHeight) Object.defineProperty(proto, 'offsetHeight', offsetHeight);
		if (offsetWidth) Object.defineProperty(proto, 'offsetWidth', offsetWidth);
		proto.getBoundingClientRect = rect;
	};
}
