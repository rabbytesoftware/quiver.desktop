import { render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';
import { ROW_H } from '@/features/shell/geometry';

import { Titlebar } from './titlebar';
import baseConfig from '../../src-tauri/tauri.conf.json';
import macosConfig from '../../src-tauri/tauri.macos.conf.json';

/** The three macOS-only window settings. Named once, asserted from both sides. */
const MACOS_ONLY_WINDOW_KEYS = ['titleBarStyle', 'hiddenTitle', 'trafficLightPosition'];

const baseWindow = baseConfig.app.windows[0] as Record<string, unknown>;
const macosWindow = macosConfig.app.windows[0];

/**
 * Every `tauri.<platform>.conf.json` that exists, keyed by the path written
 * below. Discovered rather than imported one by one, so a new platform overlay
 * shows up here without anyone remembering to add it — which is the point: an
 * overlay nobody notices is how Windows lost its window controls.
 *
 * `import.meta.glob` rather than `readdirSync`, because this project carries no
 * `@types/node` and `node:fs` therefore does not type-check. Vite resolves the
 * pattern at transform time, so the set is as fresh as the run. The pattern
 * cannot match `tauri.conf.json` itself: it requires a second `.` before
 * `conf`.
 */
const PLATFORM_OVERLAYS = import.meta.glob<{ app: { windows: Record<string, unknown>[] } }>(
	'../../src-tauri/tauri.*.conf.json',
	{ eager: true, import: 'default' }
);

/**
 * The window settings a given platform actually starts with.
 *
 * RFC 7396 again: an overlay's `app.windows` REPLACES the shared array whole,
 * so a platform that has an overlay gets that overlay's window and NOTHING of
 * the shared one, while a platform without one gets the shared window verbatim.
 */
function effectiveWindow(platform: string): Record<string, unknown> {
	return PLATFORM_OVERLAYS[`../../src-tauri/tauri.${platform}.conf.json`]?.app.windows[0] ?? baseWindow;
}

/** A traffic light button, in CSS px. Three of them plus their gaps span ~64. */
const TRAFFIC_LIGHT_DIAMETER_PX = 12;
const TRAFFIC_LIGHTS_WIDTH_PX = 64;

/** Tailwind's spacing scale is 0.25rem — 4px — per unit: `pl-20` is 80px. */
function tailwindPx(pattern: RegExp, className: string): number {
	const match = pattern.exec(className);
	if (!match) throw new Error(`expected a ${pattern.source} class, got: ${className}`);
	return Number(match[1]) * 4;
}

function renderTitlebar(userAgent: string): HTMLElement | null {
	runningOn(userAgent);
	return render(<Titlebar />).container.querySelector('header');
}

afterEach(() => {
	restoreUserAgent();
});

describe('Titlebar on macOS', () => {
	it('marks itself as a drag region', () => {
		// Without this attribute the window has no draggable surface at all —
		// "Overlay" title bar style means macOS no longer provides one.
		const header = renderTitlebar(USER_AGENTS.macos);
		expect(header?.hasAttribute('data-tauri-drag-region')).toBe(true);
	});

	it('reserves room for the traffic lights', () => {
		// pl-20 (80px) clears the ~64px the three buttons occupy at x: 12.
		// If this padding is dropped, app chrome renders underneath them.
		const header = renderTitlebar(USER_AGENTS.macos);
		expect(header?.className).toContain('pl-20');
		expect(header?.className).toContain('h-12');
	});
});

// Finding B: those settings are macOS-only, so Linux and Windows keep the title
// bar the OS draws — with its own controls and its own drag surface. A strip
// rendered there is a second, empty 48px band stacked under the real one, with
// 80px of padding held open for buttons that live somewhere else. The user
// ruled out replacing it with custom window controls: native decorations are
// the answer on both.
describe('Titlebar off macOS', () => {
	it('renders nothing on Linux, where the OS draws the title bar', () => {
		expect(renderTitlebar(USER_AGENTS.linux)).toBeNull();
	});

	it('renders nothing on Windows, where the OS draws the title bar', () => {
		expect(renderTitlebar(USER_AGENTS.windows)).toBeNull();
	});
});

// The other half of finding B. Nothing in a Tauri build fails when a macOS-only
// key is set on Linux or Windows — tao simply ignores it — so a shared config
// that carries all three reads as though every platform got an overlay title
// bar, which is the assumption that put an unconditional strip in the frontend
// in the first place. Splitting them into tauri.macos.conf.json is what makes
// the platform difference something a reader (and this suite) can see. JSON
// takes no comments, so these assertions are the documentation.
describe('window chrome configuration', () => {
	it('keeps the macOS-only settings out of the shared config', () => {
		for (const key of MACOS_ONLY_WINDOW_KEYS) {
			expect(baseWindow).not.toHaveProperty(key);
		}
	});

	it('applies them in the macOS overlay', () => {
		expect(macosWindow.titleBarStyle).toBe('Overlay');
		expect(macosWindow.hiddenTitle).toBe(true);
		expect(macosWindow.trafficLightPosition).toEqual({ x: 12, y: 11 });
	});

	it('leaves every shared window setting exactly as the shared config has it', () => {
		// Tauri merges a platform config with JSON Merge Patch (RFC 7396), which
		// REPLACES arrays rather than merging them element-wise. So this overlay's
		// `app.windows` stands in for the shared one wholesale, and any key it
		// omits or contradicts silently takes a Tauri default — or a different
		// value — on macOS alone. Duplication is forced; divergence is not, and
		// this is what catches it.
		const shared: Record<string, unknown> = { ...macosWindow };
		for (const key of MACOS_ONLY_WINDOW_KEYS) {
			delete shared[key];
		}
		expect(shared).toEqual(baseWindow);
	});

	it('centres the traffic lights in the row height the shell is built on', () => {
		// `y` is derived, not chosen: it is the buttons' top edge, so centring
		// them takes (ROW_H - 12) / 2. The row lives in `--row` and in `ROW_H`,
		// and a JSON file can read neither — retune the scale without this
		// assertion and the lights sit visibly high in the row, on macOS only,
		// with nothing else failing.
		expect(macosWindow.trafficLightPosition.y).toBe((ROW_H - TRAFFIC_LIGHT_DIAMETER_PX) / 2);

		// ...and whatever the frontend puts on that edge still has to clear all
		// three of them, or app chrome renders underneath the buttons.
		const leftPaddingPx = tailwindPx(/\bpl-(\d+)\b/, renderTitlebar(USER_AGENTS.macos)?.className ?? '');
		expect(leftPaddingPx).toBeGreaterThanOrEqual(macosWindow.trafficLightPosition.x + TRAFFIC_LIGHTS_WIDTH_PX);
	});
});

// Every window in this app gets its controls from exactly one of two places: the
// OS, or `Titlebar`. macOS is the only platform where the second is true, and
// `Titlebar` returning `null` everywhere else is only correct while the first
// holds — so `decorations: false` off macOS produces a window with no title bar,
// no close button, no minimise, and no drag surface. Alt+F4 and nothing else.
// That is precisely what `tauri.windows.conf.json` used to do, and the reason
// these assertions exist: custom in-webview window controls were ruled out, so
// native decorations are the only chrome Windows and Linux can have.
describe('native window decorations', () => {
	it.each([
		['linux', USER_AGENTS.linux],
		['windows', USER_AGENTS.windows],
	])('leaves %s the title bar the OS draws, because the frontend draws none', (platform, userAgent) => {
		expect(renderTitlebar(userAgent)).toBeNull();
		// `decorations` defaults to true, so unset is the right way to keep it.
		expect(effectiveWindow(platform).decorations ?? true).toBe(true);
	});

	it('keeps them on macOS too, where the traffic lights are the whole of the chrome', () => {
		// `titleBarStyle: "Overlay"` hides the bar and keeps the buttons —
		// `decorations: false` would take the buttons with it, and then the
		// `pl-20` above would be reserving space for nothing.
		expect(effectiveWindow('macos').decorations ?? true).toBe(true);
	});

	it('gives only macOS a platform config at all', () => {
		// An overlay replaces `app.windows` wholesale (see `effectiveWindow`), so
		// one that mentions a single key silently drops title, size, minimums and
		// `backgroundThrottling` on that platform alone — which is what the old
		// `tauri.windows.conf.json` did on top of removing the decorations.
		// macOS is the only platform whose window chrome genuinely differs, so it
		// is the only one that should be paying that price. Anything else needs
		// to restate every shared key deliberately, and this is what makes that
		// decision impossible to make by accident.
		expect(Object.keys(PLATFORM_OVERLAYS)).toEqual(['../../src-tauri/tauri.macos.conf.json']);
	});
});
