export function isMacOS(): boolean {
	return /Macintosh|Mac OS X/.test(navigator.userAgent);
}

export type GoOS = 'darwin' | 'linux' | 'windows';
export type GoArch = 'arm64' | 'amd64';

function currentOS(): GoOS {
	const ua = navigator.userAgent;
	if (/Macintosh|Mac OS X/.test(ua)) return 'darwin';
	if (/Windows/.test(ua)) return 'windows';
	return 'linux';
}

/**
 * Best-effort only. There is no Tauri command exposing the sidecar's real
 * `runtime.GOARCH` today (adding one is a Rust-side change, out of scope
 * here) -- this reads the arch hint out of the user agent string, which
 * Apple Silicon Macs commonly omit or misreport (WKWebView has no
 * `navigator.userAgentData`, and Safari's UA string doesn't distinguish
 * Apple Silicon from Intel the way Chromium's does), so this can guess wrong
 * specifically on ARM Macs. `targetForPlatform` already falls back to the
 * first declared target when nothing matches, so a wrong guess degrades
 * gracefully rather than crashing -- but a real `os_arch` Tauri command
 * should replace this the first time an arrow ships genuinely different
 * per-arch requirements/methods and that gap actually bites someone.
 */
function currentArch(): GoArch {
	return /arm64|aarch64|ARM64/.test(navigator.userAgent) ? 'arm64' : 'amd64';
}

/** `"darwin/arm64"`-style string, matching the `platform` key quiver.core's manifest targets are keyed by. */
export function currentPlatform(): `${GoOS}/${GoArch}` {
	return `${currentOS()}/${currentArch()}`;
}
