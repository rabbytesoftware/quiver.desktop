for (const method of ['setPointerCapture', 'releasePointerCapture'] as const) {
	Object.defineProperty(Element.prototype, method, {
		value: () => {},
		configurable: true,
		writable: true,
	});
}
