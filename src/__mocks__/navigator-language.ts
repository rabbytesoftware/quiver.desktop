/**
 * Pretend the machine is set to a different language.
 *
 * The same shape, and for the same reason, as `runningOn` in `./user-agent.ts`:
 * `navigator.language` and `navigator.languages` are getters on
 * `Navigator.prototype`, jsdom answers `en-US` for both regardless of the host,
 * and locale detection is the one module whose whole job is to give a different
 * answer per machine. A suite that leaned on jsdom's default would assert
 * nothing — it would pass on a German developer's laptop and on an English CI
 * box for entirely unrelated reasons.
 *
 * Shared rather than inlined because two suites need it (detection, and the
 * store's "follow the system" path), and a divergent copy would quietly test a
 * different scenario than it claims to.
 *
 * `primary` is separate from the list so the degenerate webview can be
 * described: one that populates `navigator.language` and leaves `languages`
 * empty is exactly the case `systemLocales` falls back for, and it cannot be
 * expressed by a list alone.
 */
export function speaking(languages: readonly string[], primary: string | undefined = languages[0]): void {
	Object.defineProperty(window.navigator, 'languages', { value: languages, configurable: true });
	Object.defineProperty(window.navigator, 'language', { value: primary, configurable: true });
}

/**
 * Undo `speaking`. Deleting the own data properties hands the prototype's
 * getters back, so no case leaks its language into the next one.
 */
export function restoreLanguages(): void {
	const target = window.navigator as { languages?: readonly string[]; language?: string };
	delete target.languages;
	delete target.language;
}
