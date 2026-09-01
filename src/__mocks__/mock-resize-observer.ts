/**
 * A controllable stand-in for jsdom's inert `ResizeObserver` -- call
 * `.fire(width)` on the instance to simulate a measurement. Other code in the
 * tree (a popover, a select) may also construct a `ResizeObserver`, so pick
 * the right instance via `MockResizeObserver.for(element)` rather than
 * assuming yours is the only, first, or last one created.
 */
export class MockResizeObserver {
	static instances: MockResizeObserver[] = [];
	callback: ResizeObserverCallback;
	target: Element | null = null;

	constructor(callback: ResizeObserverCallback) {
		this.callback = callback;
		MockResizeObserver.instances.push(this);
	}

	observe(target: Element): void {
		this.target = target;
	}

	unobserve(): void {
		this.target = null;
	}

	disconnect(): void {}

	fire(width: number): void {
		this.callback(
			[{ contentRect: { width }, target: this.target } as ResizeObserverEntry],
			this as unknown as ResizeObserver
		);
	}

	static for(element: Element): MockResizeObserver | undefined {
		return MockResizeObserver.instances.find((instance) => instance.target === element);
	}
}

/** Swaps in `MockResizeObserver` for the global; returns a restore function for `afterEach`. */
export function installMockResizeObserver(): () => void {
	MockResizeObserver.instances = [];
	const original = globalThis.ResizeObserver;
	globalThis.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver;
	return () => {
		globalThis.ResizeObserver = original;
	};
}
