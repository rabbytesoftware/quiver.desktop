// Which desktop OS this webview is running on.
//
// Quiver ships on macOS, Linux and Windows, and exactly one piece of chrome
// differs between them: macOS hides its title bar and lets the frontend own
// those pixels, while Linux and Windows keep the one the OS draws. The window
// side of that difference is expressed in `src-tauri/tauri.macos.conf.json`
// (see `Titlebar`); this is the frontend side.

/**
 * Is this a macOS webview?
 *
 * WHY THE USER-AGENT STRING, and not something better-behaved:
 *
 *  - the shell injects `window.__QUIVER__` with `mode` and `api`, and neither
 *    identifies the OS — `quiver://localhost` is the API origin on macOS AND
 *    on Linux, so it cannot be derived from what we already have;
 *  - `@tauri-apps/plugin-os` answers properly, but costs a Rust-side plugin
 *    registration and is `async`, which turns a static layout decision into a
 *    render-after-await — the title bar would flash in and out on every mount;
 *  - `navigator.userAgentData` is Chromium-only, so it exists in WebView2 and
 *    in neither of the two WebKit webviews.
 *
 * The UA string is synchronous, available before first paint, and unambiguous
 * across the three webviews Quiver actually runs in:
 *
 *   macOS    WKWebView    Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ...
 *   Linux    WebKitGTK    Mozilla/5.0 (X11; Linux x86_64) ...
 *   Windows  WebView2     Mozilla/5.0 (Windows NT 10.0; Win64; x64) ...
 *
 * The usual trap with this test — an iPad asking for the desktop site and
 * claiming to be a Macintosh — is unreachable: Quiver is a desktop app and does
 * not run in a mobile browser.
 */
export function isMacOS(): boolean {
	return /Macintosh|Mac OS X/.test(navigator.userAgent);
}
