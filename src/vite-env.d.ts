/// <reference types="vite/client" />
/// <reference types="@testing-library/jest-dom/vitest" />

interface ImportMetaEnv {
	/** Set by `make dev-mock` and `make dev-web`. A string, because that is what
	 *  the shell hands Vite — `Boolean('0')` is `true`. */
	readonly VITE_QUIVER_MOCK?: string;
	/** Unknown names fall back to `normal`. */
	readonly VITE_QUIVER_SCENARIO?: string;

	readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
