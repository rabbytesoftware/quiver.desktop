/**
 * jsdom lays nothing out, so a virtualized list sees a zero-size viewport and
 * zero-height rows and mounts either nothing or everything. This gives the
 * viewport a size and every row a height, which is all the virtualizer reads.
 * A scroller also gets a `clientHeight` (the viewport's) and a `scrollHeight`
 * (the height its first child declares), the two numbers tail-following reads.
 * Returns the function that puts jsdom back.
 */
export function stubLayout(viewport = { width: 800, height: 400 }, row = 20): () => void {
	const proto = HTMLElement.prototype;
	const offsetHeight = Object.getOwnPropertyDescriptor(proto, 'offsetHeight');
	const offsetWidth = Object.getOwnPropertyDescriptor(proto, 'offsetWidth');
	const rect = proto.getBoundingClientRect;
	const elementProto = Element.prototype;
	const clientHeight = Object.getOwnPropertyDescriptor(elementProto, 'clientHeight');
	const scrollHeight = Object.getOwnPropertyDescriptor(elementProto, 'scrollHeight');

	// A row's height is what the virtualizer measures it by; everything else is the viewport.
	Object.defineProperty(proto, 'offsetHeight', {
		configurable: true,
		get(this: HTMLElement) {
			return this.hasAttribute('data-index') ? row : viewport.height;
		},
	});
	Object.defineProperty(proto, 'offsetWidth', { configurable: true, get: () => viewport.width });
	Object.defineProperty(elementProto, 'clientHeight', { configurable: true, get: () => viewport.height });
	Object.defineProperty(elementProto, 'scrollHeight', {
		configurable: true,
		get(this: Element) {
			return parseFloat((this.firstElementChild as HTMLElement | null)?.style.height || '0') || 0;
		},
	});
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
		if (clientHeight) Object.defineProperty(elementProto, 'clientHeight', clientHeight);
		if (scrollHeight) Object.defineProperty(elementProto, 'scrollHeight', scrollHeight);
	};
}
