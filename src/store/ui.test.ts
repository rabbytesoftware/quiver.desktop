import { describe, it, expect, beforeEach } from 'vitest';

import { useUIStore } from './ui';

beforeEach(() => {
	useUIStore.setState({
		sidebarWidth: 200,
		selectedNamespace: null,
		navMode: 'home',
	});
});

describe('useUIStore', () => {
	describe('setSidebarWidth', () => {
		it('sets width within bounds', () => {
			useUIStore.getState().setSidebarWidth(250);
			expect(useUIStore.getState().sidebarWidth).toBe(250);
		});

		it('clamps to minimum 120', () => {
			useUIStore.getState().setSidebarWidth(50);
			expect(useUIStore.getState().sidebarWidth).toBe(120);
		});

		it('clamps to maximum 320', () => {
			useUIStore.getState().setSidebarWidth(999);
			expect(useUIStore.getState().sidebarWidth).toBe(320);
		});
	});

	describe('selectArrow', () => {
		it('sets selectedNamespace and switches navMode to arrow', () => {
			useUIStore.getState().selectArrow('github.com/char2cs/quiver');
			expect(useUIStore.getState().selectedNamespace).toBe('github.com/char2cs/quiver');
			expect(useUIStore.getState().navMode).toBe('arrow');
		});
	});

	describe('goHome', () => {
		it('clears selection and switches navMode to home', () => {
			useUIStore.getState().selectArrow('github.com/char2cs/quiver');
			useUIStore.getState().goHome();
			expect(useUIStore.getState().selectedNamespace).toBeNull();
			expect(useUIStore.getState().navMode).toBe('home');
		});
	});

	describe('setNavMode', () => {
		it('sets nav mode to search', () => {
			useUIStore.getState().setNavMode('search');
			expect(useUIStore.getState().navMode).toBe('search');
		});
	});
});
