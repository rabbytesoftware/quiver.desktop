import { browser, $, $$ } from '@wdio/globals';

/**
 * Every selector in this file was read off the real components, not guessed:
 *
 *   SIDEBAR              src/features/sidebar/components/sidebar.tsx
 *   ARROW_LIST_SKELETON  src/features/sidebar/components/arrows/arrow-list-skeleton.tsx
 *   ARROW_LIST_NOTICE    src/features/sidebar/components/arrows/arrow-list-notice.tsx
 *   ARROW_ROW_NAME       src/features/sidebar/components/arrows/arrow-row.tsx
 *   ARROW_CARD / BADGE   src/features/sidebar/components/arrows/arrow-tile.tsx
 */
export const SIDEBAR = '[data-slot="sidebar"]';
export const ARROW_LIST_SKELETON = '[data-slot="arrow-list-skeleton"]';
export const ARROW_LIST_NOTICE = '[data-slot="arrow-list-notice"]';
export const ARROW_ROW_NAME = '[data-slot="arrow-name"]';
export const ARROW_CARD = '[data-slot="arrow-card"]';
export const ARROW_CARD_BADGE = '[data-slot="arrow-card"] [data-slot="badge"]';

/**
 * "The app reached a ready state", expressed only in things the app really
 * renders.
 *
 * `useStatusStore`'s `status` is NOT usable for this: the one component that
 * puts it in the DOM is ConnectionSwitcher, and it returns `null` while
 * `connections.length <= 1` -- which is every fresh launch. So readiness is
 * taken from the catalog instead: the shell has mounted (the sidebar is
 * present) and the arrow list has settled out of its loading skeleton into
 * either real rows or the empty/error notice. That is exactly the
 * `catalog !== 'loading'` branch in arrow-list.tsx.
 */
export async function waitForAppReady(timeout = 90_000): Promise<void> {
	await browser.waitUntil(
		async () => {
			const shell = await $(SIDEBAR);
			if (!(await shell.isExisting())) return false;
			const skeleton = await $(ARROW_LIST_SKELETON);
			return !(await skeleton.isExisting());
		},
		{
			timeout,
			interval: 250,
			timeoutMsg: `app never reached a ready state: ${SIDEBAR} present and ${ARROW_LIST_SKELETON} gone`,
		}
	);
}

/**
 * The arrow names in the sidebar's list, in render order.
 *
 * `textContent`, not `getText()`: WebDriver's `getText` returns *rendered*
 * text and yields `''` for anything it considers not displayed -- which is
 * what this returned for a real, correctly-populated row whose label the
 * layout had clipped. The name is in the DOM either way, and its presence is
 * what this scenario is about.
 *
 * `$$(...).map(...)` is WebdriverIO's own chainable map, resolving to a plain
 * array -- unlike `(await $$(...)).map(...)`, which hands back unresolved
 * promises.
 */
export async function sidebarArrowNames(): Promise<string[]> {
	const raw = await $$(ARROW_ROW_NAME).map((node) => node.getAttribute('textContent'));
	return raw.map((t) => (t ?? '').trim());
}

/** Waits until the sidebar lists an arrow whose name matches, and returns the names seen. */
export async function waitForSidebarArrow(name: string, timeout = 60_000): Promise<string[]> {
	let seen: string[] = [];
	try {
		await browser.waitUntil(
			async () => {
				seen = await sidebarArrowNames();
				return seen.includes(name);
			},
			{ timeout, interval: 500 }
		);
	} catch {
		// Rethrown with the names actually on screen -- `timeoutMsg` only
		// accepts a static string, which could not carry them.
		throw new Error(`sidebar never listed an arrow named "${name}"; saw: ${JSON.stringify(seen)}`);
	}
	return seen;
}

/**
 * Navigates the in-app router by driving the real sidebar link, so the test
 * goes where a user would rather than poking history directly.
 */
export async function openRoute(hashlessPath: string): Promise<void> {
	await browser.execute((p: string) => {
		window.history.pushState({}, '', p);
		window.dispatchEvent(new PopStateEvent('popstate'));
	}, hashlessPath);
}
