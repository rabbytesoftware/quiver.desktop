export const USER_AGENTS = {
	macos: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
	linux: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
	windows:
		'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0',
};

export function runningOn(userAgent: string): void {
	Object.defineProperty(window.navigator, 'userAgent', { value: userAgent, configurable: true });
}

export function restoreUserAgent(): void {
	delete (window.navigator as { userAgent?: string }).userAgent;
}
