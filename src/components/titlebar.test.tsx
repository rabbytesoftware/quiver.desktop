import { render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';

import { Titlebar } from './titlebar';
import baseConfig from '../../src-tauri/tauri.conf.json';
import macosConfig from '../../src-tauri/tauri.macos.conf.json';

/** The three macOS-only window settings. Named once, asserted from both sides. */
const MACOS_ONLY_WINDOW_KEYS = ['titleBarStyle', 'hiddenTitle', 'trafficLightPosition'];

const baseWindow = baseConfig.app.windows[0] as Record<string, unknown>;
const macosWindow = macosConfig.app.windows[0];

/** A traffic light button, in CSS px. Three of them plus their gaps span ~64. */
const TRAFFIC_LIGHT_DIAMETER_PX = 12;
const TRAFFIC_LIGHTS_WIDTH_PX = 64;

/** Tailwind's spacing scale is 0.25rem — 4px — per unit: `h-12` is 48px. */
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
		expect(macosWindow.trafficLightPosition).toEqual({ x: 12, y: 18 });
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

	it('positions the traffic lights to match the strip the frontend renders', () => {
		// The coupling titlebar.tsx documents: the padding and
		// `trafficLightPosition` describe the same three buttons from two sides,
		// in two files, and until now nothing checked that they agreed.
		const className = renderTitlebar(USER_AGENTS.macos)?.className ?? '';
		const barHeightPx = tailwindPx(/\bh-(\d+)\b/, className);
		const leftPaddingPx = tailwindPx(/\bpl-(\d+)\b/, className);

		// `y` is the buttons' top edge, so this is what centres them in the bar.
		expect(macosWindow.trafficLightPosition.y).toBe((barHeightPx - TRAFFIC_LIGHT_DIAMETER_PX) / 2);
		// ...and the strip's padding has to clear all three of them.
		expect(leftPaddingPx).toBeGreaterThanOrEqual(macosWindow.trafficLightPosition.x + TRAFFIC_LIGHTS_WIDTH_PX);
	});
});
