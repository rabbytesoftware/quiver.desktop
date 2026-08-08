/**
 * Suite-wide fix, not a design concern of any one test: jsdom implements no
 * part of the pointer-capture API. `setPointerCapture` and
 * `releasePointerCapture` are simply absent from `Element.prototype`, so a
 * component that captures on `pointerdown` — the resize handle does, or the
 * drag dies the moment the cursor leaves the four pixels the handle occupies —
 * throws `TypeError: ... is not a function` on the first press, and every test
 * of the drag fails before it has fired a single `pointermove`.
 *
 * The stubs deliberately track nothing. Capture only decides which element the
 * browser retargets later pointer events at, and `fireEvent` names its own
 * target, so there is no behaviour here for a faithful fake to reproduce — one
 * that kept books would only be inventing an invariant the tests could then
 * assert against instead of asserting against the component.
 *
 * Defined rather than filled in conditionally, so the day jsdom ships a real
 * implementation is not the day the suite starts depending on one.
 */
for (const method of ['setPointerCapture', 'releasePointerCapture'] as const) {
	Object.defineProperty(Element.prototype, method, {
		value: () => {},
		configurable: true,
		writable: true,
	});
}
