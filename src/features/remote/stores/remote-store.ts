import { create } from 'zustand';

const TOAST_DURATION_MS = 2600;

export interface RemoteToast {
	id: string;
	message: string;
}

interface RemoteUIState {
	addOpen: boolean;
	openAdd: () => void;
	closeAdd: () => void;

	renameId: string | null;
	openRename: (id: string) => void;
	closeRename: () => void;

	removeId: string | null;
	openRemove: (id: string) => void;
	closeRemove: () => void;

	openMenuId: string | null;
	toggleMenu: (id: string) => void;
	closeMenu: () => void;

	cmdOpen: boolean;
	cmdQuery: string;
	cmdIndex: number;
	openCmd: () => void;
	closeCmd: () => void;
	setCmdQuery: (query: string) => void;
	setCmdIndex: (index: number) => void;

	toasts: RemoteToast[];
	pushToast: (message: string) => void;
	dismissToast: (id: string) => void;
}

/** Every other overlay this feature owns -- opening the command palette (the
 *  global, always-reachable switcher) must not leave a dialog or a row menu
 *  stacked underneath it. */
const CLOSE_OTHER_OVERLAYS = {
	addOpen: false,
	renameId: null,
	removeId: null,
	openMenuId: null,
} as const;

export const useRemoteStore = create<RemoteUIState>((set, get) => ({
	addOpen: false,
	openAdd: () => set({ addOpen: true }),
	closeAdd: () => set({ addOpen: false }),

	renameId: null,
	openRename: (id) => set({ renameId: id }),
	closeRename: () => set({ renameId: null }),

	removeId: null,
	openRemove: (id) => set({ removeId: id }),
	closeRemove: () => set({ removeId: null }),

	openMenuId: null,
	toggleMenu: (id) => set((state) => ({ openMenuId: state.openMenuId === id ? null : id })),
	closeMenu: () => set({ openMenuId: null }),

	cmdOpen: false,
	cmdQuery: '',
	cmdIndex: 0,
	openCmd: () => {
		if (get().cmdOpen) return;
		set({ ...CLOSE_OTHER_OVERLAYS, cmdOpen: true, cmdQuery: '', cmdIndex: 0 });
	},
	closeCmd: () => set({ cmdOpen: false }),
	setCmdQuery: (query) => set({ cmdQuery: query, cmdIndex: 0 }),
	setCmdIndex: (index) => set({ cmdIndex: index }),

	toasts: [],
	pushToast: (message) => {
		const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
		set((state) => ({ toasts: [...state.toasts, { id, message }] }));
		setTimeout(() => get().dismissToast(id), TOAST_DURATION_MS);
	},
	dismissToast: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));
