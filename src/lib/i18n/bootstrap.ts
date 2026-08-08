import { currentLocale, localeOf, useLocaleStore } from './store';

/**
 * The two things the locale has to drive that live outside React, wired once
 * from `main.tsx`. Returns a teardown so a test can unwire it — nothing in the
 * app calls that, because the app only stops when the window does.
 *
 * 1. `<html lang>`. `index.html` hard-codes `lang="en"`, which is correct today
 *    and becomes a lie the moment a second catalogue ships. It is not
 *    decoration: a screen reader picks its voice and its pronunciation rules
 *    from it, `:lang()` selectors and CSS hyphenation key off it, and a browser
 *    offers to translate a page based on it. Leaving it stale means German text
 *    read aloud by an English synthesiser.
 *
 * 2. `languagechange`. Changing the OS language fires this on `window`, and
 *    without it "follow the system" would only be true at boot. `redetect`
 *    re-reads `navigator.languages`; components subscribed through
 *    `useTranslation` re-render because `detected` is store state.
 *
 *    Not load-bearing — a desktop app is usually restarted after an OS language
 *    change anyway, and a webview is not obliged to fire the event — which is
 *    why it is an improvement rather than the mechanism. The mechanism is the
 *    synchronous read at boot.
 */
export function installLocaleSync(): () => void {
	applyDocumentLanguage(currentLocale());

	// Unconditional, with no "did it actually change?" guard in front of it.
	// Assigning the same string to `lang` is not observable — no mutation
	// record, no style recalculation, nothing an assistive technology reacts to
	// — so the guard would buy nothing and cost a branch that only a second
	// shipped catalogue could ever exercise.
	const unsubscribe = useLocaleStore.subscribe((state) => applyDocumentLanguage(localeOf(state)));

	const onLanguageChange = () => useLocaleStore.getState().redetect();
	window.addEventListener('languagechange', onLanguageChange);

	return () => {
		unsubscribe();
		window.removeEventListener('languagechange', onLanguageChange);
	};
}

function applyDocumentLanguage(locale: string): void {
	document.documentElement.lang = locale;
}
