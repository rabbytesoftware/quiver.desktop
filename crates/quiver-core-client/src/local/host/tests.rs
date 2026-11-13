use super::*;

const DEV_CHECKOUT: &str = "/Users/dev/quiver.desktop";

#[test]
fn a_debug_build_scopes_home_under_the_checkout_it_was_built_from() {
    assert_eq!(
        dev_quiver_home("/Users/dev/quiver.desktop/src-tauri", true),
        Some(std::path::PathBuf::from(
            "/Users/dev/quiver.desktop/src-tauri/.quiver"
        ))
    );
}

/// The property production ships on: a real `tauri build` (not `--debug`)
/// must never override quiver.core's own default, or a shipped app would
/// start scoping a user's real installed arrows and config away from
/// their actual `~/.quiver`.
#[test]
fn a_release_build_never_overrides_the_real_home() {
    assert_eq!(
        dev_quiver_home("/Users/dev/quiver.desktop/src-tauri", false),
        None
    );
}

#[cfg(unix)]
#[test]
fn unix_host_arg_is_the_bare_default_scheme_in_production() {
    // `unix://` with no path means "quiver.core's default", which is
    // ~/.quiver/quiver.sock — the same path default_socket_path() builds.
    // Passing an explicit path would silently diverge if core ever moved it.
    assert_eq!(
        LocalHost::Unix("/x/y.sock".into()).host_arg(false),
        "unix://"
    );
}

/// The other half of the fix in this module: a dev build's own default
/// resolves under QUIVER_HOME (its own long checkout path) if left bare,
/// which is exactly what breaks the bind (see `dev_socket_override`). The
/// explicit path must be passed instead, not core's default.
#[cfg(unix)]
#[test]
fn unix_host_arg_is_the_explicit_short_override_in_a_dev_build() {
    assert_eq!(
        LocalHost::Unix("/tmp/quiver-dev-abc123.sock".into()).host_arg(true),
        "unix:///tmp/quiver-dev-abc123.sock"
    );
}

/// The name reaches core verbatim, as an `npipe://` URI: core has no
/// per-home pipe default to derive, and the dev name in particular must not
/// be replaced by core's own.
#[cfg(windows)]
#[test]
fn pipe_host_arg_names_the_pipe_explicitly() {
    assert_eq!(
        LocalHost::Pipe("quiver".into()).host_arg(false),
        "npipe://quiver"
    );
    assert_eq!(
        LocalHost::Pipe("quiver-dev-abc123".into()).host_arg(true),
        "npipe://quiver-dev-abc123"
    );
}

#[cfg(windows)]
#[test]
fn dev_pipe_name_is_none_in_a_release_build() {
    assert_eq!(
        dev_pipe_name("C:\\dev\\quiver.desktop\\src-tauri", false),
        None
    );
}

#[cfg(windows)]
#[test]
fn dev_pipe_name_is_stable_per_checkout_and_distinct_between_them() {
    let a = "C:\\dev\\quiver.desktop-a\\src-tauri";
    let b = "C:\\dev\\quiver.desktop-b\\src-tauri";
    assert_eq!(dev_pipe_name(a, true), dev_pipe_name(a, true));
    assert_ne!(dev_pipe_name(a, true), dev_pipe_name(b, true));
}

/// A dev build must never share the shipped build's pipe, or `tauri dev`
/// would adopt (and then fail to isolate from) the installed app's daemon.
#[cfg(windows)]
#[test]
fn dev_pipe_name_never_collides_with_the_shipped_name() {
    let name = dev_pipe_name("C:\\dev\\quiver.desktop\\src-tauri", true).unwrap();
    assert_ne!(name, LOCAL_PIPE_NAME);
    assert!(name.starts_with("quiver-dev-"), "got {name:?}");
}

#[cfg(unix)]
#[test]
fn dev_socket_override_is_none_in_a_release_build() {
    assert_eq!(
        dev_socket_override("/Users/dev/quiver.desktop/src-tauri", false),
        None
    );
}

/// The property that matters for `default_socket_path()`: the same
/// checkout must always get the same socket path, or every restart of
/// the same worktree would leave the previous run's daemon unreachable
/// and orphaned — the orphaned-daemon failure `LOCAL_PIPE_NAME`'s
/// own doc describes.
#[cfg(unix)]
#[test]
fn dev_socket_override_is_the_same_path_for_the_same_checkout() {
    let manifest_dir = "/Users/dev/quiver.desktop/src-tauri";
    assert_eq!(
        dev_socket_override(manifest_dir, true),
        dev_socket_override(manifest_dir, true)
    );
}

#[cfg(unix)]
#[test]
fn dev_socket_override_differs_between_checkouts() {
    assert_ne!(
        dev_socket_override("/Users/dev/quiver.desktop-a/src-tauri", true),
        dev_socket_override("/Users/dev/quiver.desktop-b/src-tauri", true)
    );
}

/// The whole reason this override exists: a checkout under this
/// project's own workspace tooling is long enough that the real
/// `.quiver`-anchored path (173 bytes, measured) exceeds `sockaddr_un`'s
/// ~104-byte limit and fails to bind with EINVAL. The override must stay
/// short regardless of how long the checkout path itself is.
#[cfg(unix)]
#[test]
fn dev_socket_override_stays_well_under_the_unix_socket_path_limit() {
    let long_manifest_dir = "/Users/char2cs/.crowbar/projects/cb81ea03-54b7-4093-8bdd-fdd6b183e91d/github.com/rabbytesoftware/quiver.desktop/feature/remote-control/worktree/src-tauri";
    let path = dev_socket_override(long_manifest_dir, true).expect("must override in a dev build");
    assert!(
        path.to_string_lossy().len() < 100,
        "got a {}-byte path, too close to sockaddr_un's ~104-byte limit: {path:?}",
        path.to_string_lossy().len()
    );
}

/// The defect this guards: `local_host(Some(DEV_CHECKOUT))` used to call `pick_free_port()` on
/// Windows, so every `LocalConnection::new()` — startup, and every switch
/// back to local — addressed a DIFFERENT daemon and spawned one, leaving the
/// previous ones running and unreachable.
///
/// Two calls, one assertion: the local daemon's address must not depend on
/// when it was asked for. It goes red on Windows the moment a free-port pick
/// comes back, and red on unix if the socket path ever picks up a nonce.
#[test]
fn the_local_daemon_address_is_the_same_on_every_construction() {
    assert_eq!(
        local_host(Some(DEV_CHECKOUT)),
        local_host(Some(DEV_CHECKOUT)),
        "a local address that changes per construction spawns a daemon per \
			 construction and orphans the last one"
    );
}

/// And it is the address this platform is documented to use — the pairing a
/// compiler cannot check, written out per platform in literals rather than
/// derived from the thing under test.
///
/// `cargo test` is itself a debug build, so this can only ever observe the
/// dev-scoped override (see `dev_socket_override`) on unix, never the real
/// production default — that half is covered directly by
/// `unix_host_arg_is_the_bare_default_scheme_in_production` and the
/// `dev_quiver_home`/`dev_socket_override` tests instead.
#[test]
fn the_local_daemon_address_is_the_documented_one_for_this_platform() {
    let host = local_host(Some(DEV_CHECKOUT));
    #[cfg(unix)]
    assert!(
        matches!(&host, LocalHost::Unix(p) if p.contains("quiver-dev-") && p.ends_with(".sock")),
        "unix addresses the dev-scoped override under test; got {host:?}"
    );
    // The literal, not `LOCAL_PIPE_NAME`: comparing the constant to itself
    // would pass whatever it were changed to.
    #[cfg(windows)]
    assert!(
        matches!(&host, LocalHost::Pipe(n) if n.starts_with("quiver-dev-")),
        "windows addresses the dev-scoped pipe under test; got {host:?}"
    );
}
