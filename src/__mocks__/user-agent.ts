/**
 * The user-agent strings of the three webviews Quiver ships in, and the means
 * to pretend to be one of them.
 *
 * Shared rather than inlined per test file because they are a fact about the
 * platforms, not about any one test: `isMacOS` reads them (src/lib/platform.ts)
 * and `Titlebar` renders off the answer, so both suites need the same three,
 * and a typo in a copy would quietly test nothing.
 *
 * jsdom's own default UA is `Mozilla/5.0 (<process.platform>) ...` — neither a
 * real webview's nor stable across the machines this suite runs on, which is
 * exactly why no test may rely on it.
 */
export const USER_AGENTS = {
	/** WKWebView. Frozen at "Intel Mac OS X" even on Apple silicon. */
	macos: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
	/** WebKitGTK. */
	linux: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
	/** WebView2 (Chromium). */
	windows:
		'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0',
};

/** Run the rest of this test as though the webview were on `userAgent`. */
export function runningOn(userAgent: string): void {
	Object.defineProperty(window.navigator, 'userAgent', { value: userAgent, configurable: true });
}

/**
 * Undo `runningOn`. jsdom defines `userAgent` as a getter on
 * `Navigator.prototype`; `runningOn` shadows it with an own data property, and
 * deleting that hands the getter back — so no test leaks its platform into the
 * next one.
 */
export function restoreUserAgent(): void {
	delete (window.navigator as { userAgent?: string }).userAgent;
}
