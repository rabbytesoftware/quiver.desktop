import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export const CATALOG_VISIBILITY_STORAGE_KEY = 'quiver.catalogVisibility';

interface CatalogVisibilityState {
	/** Opt-out, not opt-in: `true` (show, the current/pre-existing behaviour) is the default. */
	showSelfComponents: boolean;
	setShowSelfComponents: (value: boolean) => void;
}

export function normaliseShowSelfComponents(value: unknown): boolean {
	return typeof value === 'boolean' ? value : true;
}

export const useCatalogVisibilityStore = create<CatalogVisibilityState>()(
	persist(
		(set) => ({
			showSelfComponents: true,
			setShowSelfComponents: (showSelfComponents) => set({ showSelfComponents }),
		}),
		{
			name: CATALOG_VISIBILITY_STORAGE_KEY,
			partialize: (s) => ({ showSelfComponents: s.showSelfComponents }),
			merge: (persisted, current) => ({
				...current,
				showSelfComponents: normaliseShowSelfComponents(
					(persisted as { showSelfComponents?: unknown } | null)?.showSelfComponents
				),
			}),
		}
	)
);
