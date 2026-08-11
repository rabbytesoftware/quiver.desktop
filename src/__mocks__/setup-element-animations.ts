if (typeof Element.prototype.getAnimations !== 'function') {
	Object.defineProperty(Element.prototype, 'getAnimations', {
		value: (): Animation[] => [],
		configurable: true,
		writable: true,
	});
}
