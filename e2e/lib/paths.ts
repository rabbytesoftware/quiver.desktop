import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));

/** The repo root (`e2e/lib/..` -> `e2e/..`). */
export const REPO_ROOT = path.resolve(HERE, '..', '..');

/**
 * The built application binary `tauri:options.application` points at.
 *
 * A RELEASE build, deliberately, and the choice is load-bearing rather than
 * cosmetic. `dev_quiver_home` (src-tauri/src/connection/local/mod.rs) pins a
 * debug build's QUIVER_HOME to `<CARGO_MANIFEST_DIR>/.quiver` at COMPILE time
 * and its unix socket to a hash of the same path, so a debug binary's home is
 * not controllable per run -- every scenario here would share one home and
 * scenario 1's "clean QUIVER_HOME" precondition would be unstatable. In a
 * release build `quiver_home()` returns `None`, so the app resolves the home
 * from the environment instead (`SidecarManager::quiver_home_dir`) and the
 * harness can hand each spec its own.
 *
 * `cargo build --release` / `tauri build --no-bundle` both write here; the
 * binary name is the `[package] name` in src-tauri/Cargo.toml (`quiverdesktop`).
 */
export const APP_BINARY =
	process.env.QUIVER_E2E_APP_BINARY ??
	path.join(
		REPO_ROOT,
		'src-tauri',
		'target',
		'release',
		process.platform === 'win32' ? 'quiverdesktop.exe' : 'quiverdesktop'
	);

/** Where per-spec throwaway homes are created. */
export const E2E_TMP = process.env.QUIVER_E2E_TMP ?? path.join(os.tmpdir(), 'quiver-e2e');

/**
 * The scratch HOME handed to one spec's app process.
 *
 * HOME, not QUIVER_HOME, and that is not interchangeable here. In a release
 * build the app passes core a bare `unix://` host argument
 * (`LocalHost::host_arg(false)`), which means "quiver.core's own default
 * socket" -- and core resolves that default under QUIVER_HOME. The app's own
 * transport, meanwhile, dials `default_socket_path()`, which is built from
 * `$HOME` (`format!("{}/.quiver/quiver.sock", home)`). Those two agree only
 * while QUIVER_HOME is left at its platform default of `$HOME/.quiver`.
 * Setting QUIVER_HOME directly to some third directory would move core's
 * socket out from under the app and the app would never connect.
 *
 * So: override HOME, let QUIVER_HOME default beneath it, and both halves stay
 * pointed at the same place.
 */
export function homeForSpec(specName: string): string {
	return path.join(E2E_TMP, specName);
}

/** QUIVER_HOME as core resolves it on unix, given the HOME above. */
export function quiverHome(home: string): string {
	return path.join(home, '.quiver');
}

/**
 * The unix socket both sides settle on. Windows uses a fixed loopback port
 * (`LOCAL_TCP_PORT` = 40257) instead, because Rust's async stack has no
 * AF_UNIX support there.
 */
export function socketPath(home: string): string {
	return path.join(quiverHome(home), 'quiver.sock');
}

/**
 * quiver.core's stable self-install path -- `paths.self` in
 * internal/core/metadata/metadata.yaml is `{{home}}/self`, and
 * `selfarrow.PromoteRunningBinary` writes the running binary there on every
 * boot. This is the exact path `SidecarManager::resolve_binary_path` probes
 * before falling back to the bundled sidecar.
 */
export function selfInstalledCore(home: string): string {
	return path.join(quiverHome(home), 'self', process.platform === 'win32' ? 'quiver.exe' : 'quiver');
}

/** The two self-arrow namespaces, from metadata.yaml's `namespaces` block. */
export const NS_CORE = 'github.com/rabbytesoftware/quiver.core';
export const NS_DESKTOP = 'github.com/rabbytesoftware/quiver.desktop';
