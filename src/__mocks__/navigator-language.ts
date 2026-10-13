export function speaking(languages: readonly string[], primary: string | undefined = languages[0]): void {
	Object.defineProperty(window.navigator, 'languages', { value: languages, configurable: true });
	Object.defineProperty(window.navigator, 'language', { value: primary, configurable: true });
}

export function restoreLanguages(): void {
	const target = window.navigator as { languages?: readonly string[]; language?: string };
	delete target.languages;
	delete target.language;
}
