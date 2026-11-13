use super::*;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

// ── Preferring a self-installed Core ─────────────────────────────────────

#[test]
fn resolve_binary_path_prefers_installed_core_when_present() {
    let home = TempDir::new().unwrap();
    let self_dir = home.path().join("self");
    fs::create_dir_all(&self_dir).unwrap();
    let installed_bin = self_dir.join(if cfg!(windows) {
        "quiver.exe"
    } else {
        "quiver"
    });
    fs::write(&installed_bin, b"fake binary").unwrap();

    let resolved = resolve_binary_path(home.path());

    assert_eq!(resolved, Some(installed_bin));
}

#[test]
fn resolve_binary_path_none_when_not_installed() {
    let home = TempDir::new().unwrap();

    let resolved = resolve_binary_path(home.path());

    assert_eq!(resolved, None);
}

/// Guards every test below that touches `QUIVER_HOME` (and, further down,
/// `HOME`/`USERNAME`): `std::env::set_var` and `remove_var` mutate
/// process-global state, and two tests racing on the same variable could
/// each observe the other's value. Serializing the tests below against
/// EACH OTHER is what this lock actually buys. It is a narrower guarantee
/// than full soundness, and this comment used to claim otherwise: per
/// `set_var`/`remove_var`'s own safety docs, they race against a
/// concurrent env-var READ on any other thread, not just a concurrent
/// write to the same key, because most platform implementations (glibc's
/// `environ` included) back the whole environment table with one shared
/// allocation. `cargo test`'s default multi-threaded runner can run one of
/// these `set_var` calls concurrently with, say, `tempfile::TempDir::new()`
/// reading `TMPDIR` in an unrelated test elsewhere in this file — a
/// different key, still a documented race. In practice, two full
/// `make test-rust` runs back to back showed no failures (128/128 both
/// times), so the real-world blast radius appears small — but that is an
/// empirical observation about this suite's current shape, not a
/// soundness argument this lock provides on its own.
fn quiver_home_env_lock() -> &'static Mutex<()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// The override quiver.core itself honors first — see
/// `resolveHome()`/`homeOverrideEnv` in its own `metadata` package. This
/// app has to agree on that env var, or it would look for a self-installed
/// Core in a directory quiver.core was never told to use.
#[test]
fn quiver_home_dir_prefers_the_env_var_override_when_set() {
    let _guard = quiver_home_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("QUIVER_HOME", "/tmp/quiver-home-dir-override-test");
    }

    let resolved = quiver_home_dir();

    unsafe {
        std::env::remove_var("QUIVER_HOME");
    }
    assert_eq!(
        resolved,
        PathBuf::from("/tmp/quiver-home-dir-override-test")
    );
}

/// With no override, this has to land on exactly the same default
/// quiver.core's own `metadata.yaml` (`paths.home`) documents — not
/// merely a plausible-looking `~/.quiver`. Unix and Windows are asserted
/// separately because the underlying env var (`HOME` vs `USERNAME`) and
/// the shape of the default path differ per platform.
#[test]
fn quiver_home_dir_falls_back_to_the_platform_default_when_unset() {
    let _guard = quiver_home_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("QUIVER_HOME");
    }

    let resolved = quiver_home_dir();

    #[cfg(unix)]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        assert_eq!(resolved, PathBuf::from(home).join(".quiver"));
    }
    #[cfg(windows)]
    {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into());
        assert_eq!(
            resolved,
            PathBuf::from(format!(r"C:\Users\{user}\Documents\.quiver"))
        );
    }
}

/// Exercises `platform_default_home()`'s own INNER fallback, reached only
/// when the platform env var it reads (`HOME` on Unix, `USERNAME` on
/// Windows) is unset too — not just `QUIVER_HOME`. The test above only
/// ever clears `QUIVER_HOME`, so it never reaches `/tmp/.quiver` (Unix) or
/// `"unknown"` (Windows): exactly the branches documented as this app's
/// deliberate divergence from quiver.core's Go implementation, which
/// returns the raw, unexpanded `~/.quiver` template instead. A regression
/// here — say, an `.unwrap()` that panics instead of falling back — would
/// pass the test above and still be invisible without this one.
#[test]
fn quiver_home_dir_falls_back_further_when_the_platform_env_var_is_also_unset() {
    let _guard = quiver_home_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("QUIVER_HOME");
    }

    #[cfg(unix)]
    {
        let prior = std::env::var("HOME").ok();
        unsafe {
            std::env::remove_var("HOME");
        }

        let resolved = quiver_home_dir();

        unsafe {
            match &prior {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(resolved, PathBuf::from("/tmp/.quiver"));
    }
    #[cfg(windows)]
    {
        let prior = std::env::var("USERNAME").ok();
        unsafe {
            std::env::remove_var("USERNAME");
        }

        let resolved = quiver_home_dir();

        unsafe {
            match &prior {
                Some(value) => std::env::set_var("USERNAME", value),
                None => std::env::remove_var("USERNAME"),
            }
        }
        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\unknown\Documents\.quiver")
        );
    }
}

// ── Scoping the self-install check to `dev_home` ─────────────────────────

/// The bug Finding 1 of the Task 2.2 review describes: without this,
/// `spawn` checked for a self-installed Core against the REAL production
/// home regardless of `dev_home`, so the moment a developer's own,
/// unrelated `quiver.core` self-updated once, every `make dev`/
/// `make dev-bundle` afterwards would silently spawn that production
/// binary — while still correctly scoping its *data* directory to the
/// checkout via `dev_home` — defeating half of `dev_home`'s isolation
/// guarantee. This proves the fix: with `dev_home` set (as in any debug
/// build) and a binary self-installed under the checkout-scoped home's
/// `self/`, resolution must land on THAT binary even when the real
/// production home (here, `QUIVER_HOME`) has an entirely different
/// self-installed binary of its own.
#[test]
fn effective_home_prefers_dev_home_over_a_real_self_installed_core() {
    let _guard = quiver_home_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let bin_name = if cfg!(windows) {
        "quiver.exe"
    } else {
        "quiver"
    };

    // The checkout-scoped dev home, with its OWN self-installed binary.
    let dev_home = TempDir::new().unwrap();
    let dev_self_dir = dev_home.path().join("self");
    fs::create_dir_all(&dev_self_dir).unwrap();
    let dev_installed_bin = dev_self_dir.join(bin_name);
    fs::write(&dev_installed_bin, b"dev-scoped binary").unwrap();

    // The real production home -- what `quiver_home_dir()` resolves to
    // without a `dev_home` override -- which ALSO has a self-installed
    // binary: an unrelated one that must never be picked while `dev_home`
    // is set, even though it exists and predates this fix's check.
    let real_home = TempDir::new().unwrap();
    let real_self_dir = real_home.path().join("self");
    fs::create_dir_all(&real_self_dir).unwrap();
    fs::write(real_self_dir.join(bin_name), b"real production binary").unwrap();
    unsafe {
        std::env::set_var("QUIVER_HOME", real_home.path());
    }

    let effective = effective_home(Some(dev_home.path()));

    unsafe {
        std::env::remove_var("QUIVER_HOME");
    }

    assert_eq!(
        effective,
        dev_home.path(),
        "a dev build must resolve the self-install check against its own \
			 checkout-scoped home, never the real one -- even when the real \
			 home also has a self-installed binary"
    );

    let resolved = resolve_binary_path(&effective);
    assert_eq!(
        resolved,
        Some(dev_installed_bin),
        "spawn must prefer the binary self-installed into the checkout's \
			 own home, not the real, unrelated production install"
    );
}

/// The other half of `effective_home`: a release build (`dev_home: None`,
/// since `quiver_home()` only ever returns `Some` in a debug build) must
/// fall back to `quiver_home_dir()`'s real resolution unchanged -- this
/// fix must not touch production behavior at all.
#[test]
fn effective_home_falls_back_to_quiver_home_dir_when_dev_home_is_none() {
    let _guard = quiver_home_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("QUIVER_HOME", "/tmp/quiver-effective-home-fallback-test");
    }

    let effective = effective_home(None);

    unsafe {
        std::env::remove_var("QUIVER_HOME");
    }

    assert_eq!(
        effective,
        PathBuf::from("/tmp/quiver-effective-home-fallback-test")
    );
}
