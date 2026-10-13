class NoopResizeObserver implements ResizeObserver {
	observe(): void {}
	unobserve(): void {}
	disconnect(): void {}
}

Object.defineProperty(globalThis, 'ResizeObserver', {
	value: NoopResizeObserver,
	configurable: true,
	writable: true,
});
