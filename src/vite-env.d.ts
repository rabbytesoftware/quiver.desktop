/// <reference types="vite/client" />
/// <reference types="@testing-library/jest-dom/vitest" />

interface ImportMetaEnv {
	readonly VITE_QUIVER_MOCK?: string;
	readonly VITE_QUIVER_SCENARIO?: string;

	readonly VITE_QUIVER_LOCALE?: string;

	readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
