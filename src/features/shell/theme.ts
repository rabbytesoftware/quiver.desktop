import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ThemePreference = 'system' | 'light' | 'dark';

export const THEME_STORAGE_KEY = 'quiver.theme';

interface ThemeState {
	preference: ThemePreference;
	setPreference: (preference: ThemePreference) => void;
}

export function normalisePreference(value: unknown): ThemePreference {
	return value === 'light' || value === 'dark' ? value : 'system';
}

export const useThemeStore = create<ThemeState>()(
	persist(
		(set) => ({
			preference: 'system',
			setPreference: (preference) => set({ preference }),
		}),
		{
			name: THEME_STORAGE_KEY,
			partialize: (s) => ({ preference: s.preference }),
			merge: (persisted, current) => ({
				...current,
				preference: normalisePreference((persisted as { preference?: unknown } | undefined)?.preference),
			}),
		}
	)
);

const DARK_QUERY = '(prefers-color-scheme: dark)';

function prefersDark(): boolean {
	return typeof matchMedia === 'function' && matchMedia(DARK_QUERY).matches;
}

function apply(preference: ThemePreference): void {
	const dark = preference === 'dark' || (preference === 'system' && prefersDark());
	document.documentElement.classList.toggle('dark', dark);
}

/**
 * Mirrors `installLocaleSync`: apply once, then keep applying. The media
 * listener stays attached whatever the preference is and simply re-derives —
 * `apply` already ignores the system while an explicit choice is in force, so
 * there is no subscribe/unsubscribe dance to get wrong.
 *
 * This is the sole owner of the OS `change` listener. `index.html`'s
 * pre-paint script reads the persisted preference to avoid a flash of the
 * wrong theme before this module ever runs, but it does not attach a
 * `change` listener of its own — a second one there would race this one.
 */
export function installThemeSync(): () => void {
	apply(useThemeStore.getState().preference);

	const unsubscribe = useThemeStore.subscribe((state) => apply(state.preference));

	const onSystemChange = () => apply(useThemeStore.getState().preference);
	const media = typeof matchMedia === 'function' ? matchMedia(DARK_QUERY) : null;
	media?.addEventListener('change', onSystemChange);

	return () => {
		unsubscribe();
		media?.removeEventListener('change', onSystemChange);
	};
}
