/// <reference types="vite/client" />
/// <reference types="@testing-library/jest-dom/vitest" />

interface ImportMetaEnv {
	/** Set by `make dev-mock` and `make dev-web`. A string, because that is what
	 *  the shell hands Vite — `Boolean('0')` is `true`. */
	readonly VITE_QUIVER_MOCK?: string;
	/** Unknown names fall back to `normal`. */
	readonly VITE_QUIVER_SCENARIO?: string;

	/** A BCP-47 tag that FORCES the UI language for one run, over the saved
	 *  preference and over the system. Ignored unless a catalogue for it ships —
	 *  see `localeForcedByEnv` in `@/lib/i18n/detect`. */
	readonly VITE_QUIVER_LOCALE?: string;

	readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
