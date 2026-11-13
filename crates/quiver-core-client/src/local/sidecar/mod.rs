//! Spawning, supervising and stopping the local quiver.core daemon.

mod home;
mod process;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::process::Child;

use crate::transport::Transport;
use crate::{Error, Result};

use super::config::LocalConfig;
use home::{effective_home, resolve_binary_path};
use process::{health_ok, kill, request_shutdown, spawn_supervisor, take_spawned};

const HEALTH_RETRY_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

pub struct SidecarManager {
    config: LocalConfig,
    /// The daemon THIS app spawned, and the only one it may ever kill.
    ///
    /// A `tokio::process::Child` dropped on the floor leaves the process running
    /// with nothing left in the app that can address it. Holding it here is what
    /// makes [`SidecarManager::reap`] possible at all.
    ///
    /// `None` covers two very different situations, and conflating them is the
    /// mistake this field exists to prevent: nothing has been spawned yet, and
    /// `ensure_running` found a daemon already listening and adopted it. In the
    /// second case there is a live daemon on the address this app is talking to
    /// and it still must not be killed — it belongs to whoever started it.
    ///
    /// A `std::sync::Mutex`, not tokio's, and deliberately: the critical section
    /// is one `Option::take` with no await in it (see `take_spawned`), and an
    /// async lock would invite exactly the "held across the kill" shape that
    /// `teardown`'s `&self` makes tempting.
    child: Arc<Mutex<Option<Child>>>,
}

impl SidecarManager {
    pub fn new(config: LocalConfig) -> Self {
        Self {
            config,
            child: Arc::new(Mutex::new(None)),
        }
    }

    /// Bring the local daemon up, or notice that it already is.
    ///
    /// The probe comes FIRST, and that is the whole point. `LocalConnection::new()`
    /// runs at startup and again on every switch back to local, and each run used
    /// to spawn unconditionally. On unix that was merely wasteful — the second
    /// daemon loses the race for a fixed socket path and exits — and on Windows,
    /// where the port used to be picked fresh each time, it was an orphaned daemon
    /// per switch (see `local::LOCAL_PIPE_NAME`). With a fixed address on both
    /// platforms, one health probe answers "is there already one of these?" and
    /// makes the second spawn unnecessary rather than merely doomed.
    ///
    /// The probe also decides what "ready" means, which is what keeps the fixed
    /// address honest: if something that is not quiver.core holds the address, it
    /// cannot answer `/v0/health`, so this reports failure rather than handing
    /// the app a transport pointed at a stranger.
    ///
    /// The early return is also what makes the ownership rule in [`Self::reap`]
    /// hold: an adopted daemon never reaches `spawn`, so no child handle is
    /// recorded for it, so nothing later kills it.
    pub async fn ensure_running(
        &self,
        transport: &dyn Transport,
    ) -> Result<()> {
        if health_ok(transport).await {
            log::info!("[local] a daemon is already listening — not spawning another");
            return Ok(());
        }
        self.spawn().await?;
        self.wait_for_ready(transport).await
    }

    /// Start the daemon and keep hold of both halves of what that returns.
    ///
    /// Private on purpose: `ensure_running` is the only way in, because it is
    /// the probe that decides whether a spawn is wanted at all, and because a
    /// second spawn on the same manager would displace the child handle the
    /// first one recorded.
    async fn spawn(&self) -> Result<()> {
        // A dev/preview build points the daemon it spawns at this checkout's
        // own `.quiver`, never the user's real one — quiver.core only infers
        // this on its own for an unstamped `go run` build, and this sidecar is
        // always a version-stamped release download, so it never would.
        let dev_home = self.config.dev_home.as_deref();

        // The bundled binary is only ever day-one bootstrap seed: once
        // quiver.core has self-updated at least once (its own Task 1.8), the
        // current binary lives at a stable path outside the bundle and this app
        // must prefer it — otherwise every launch after a self-update would
        // still run the stale seed the app first shipped with. This is checked
        // against `effective_home` — the SAME home `dev_home`, when set, scopes
        // the spawned daemon's *data* directory to below — so a dev build only
        // ever prefers a binary self-installed into THIS checkout's own scoped
        // `.quiver/self/`, never a developer's real, unrelated one. Checking it
        // against the real home instead would silently spawn that unrelated
        // production binary the moment one existed there, while still correctly
        // scoping the child's data directory to the checkout — defeating half of
        // `dev_home`'s isolation guarantee.
        let effective_home = effective_home(dev_home);
        let program = match resolve_binary_path(&effective_home) {
            Some(installed) => {
                log::info!(
                    "[local] using self-installed quiver.core at {}",
                    installed.display()
                );
                installed
            }
            None => self.config.bundled_binary.clone().ok_or_else(|| {
                Error::Daemon(
                    "no quiver.core binary: none self-installed and no bundled sidecar".into(),
                )
            })?,
        };

        let mut command = tokio::process::Command::new(&program);
        command
            .args([
                "daemon",
                "--host",
                &self.config.host.host_arg(dev_home.is_some()),
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        // No console window flashing up next to a GUI app.
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        if let Some(home) = dev_home {
            command.env("QUIVER_HOME", home);
        }

        let mut child = command
            .spawn()
            .map_err(|e| Error::Daemon(format!("spawn {}: {e}", program.display())))?;

        log::info!(
            "[local] spawned quiver.core (pid {})",
            child
                .id()
                .map_or_else(|| "?".to_string(), |p| p.to_string())
        );

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Nothing should be here — `ensure_running` probes before spawning and
        // each `LocalConnection` builds its own manager — but a handle displaced
        // from this slot is a daemon nothing can ever kill, so it is reaped
        // rather than overwritten.
        if let Some(displaced) = take_spawned(&self.child) {
            kill(displaced);
        }
        self.store(child);

        // stdout and stderr are the daemon's own voice. Dropping them throws both
        // away. See `process::pump_events`.
        spawn_supervisor(Arc::clone(&self.child), stdout, stderr);
        Ok(())
    }

    /// Record the child, and let go of the lock. See [`take_spawned`] for why the
    /// critical section is kept to exactly this.
    fn store(
        &self,
        child: Child,
    ) {
        *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
    }

    /// Stop the daemon this app spawned, if this app spawned one.
    ///
    /// Asks it to exit over its own socket (`POST /v0/system/shutdown`) and only
    /// kills the remembered pid if that fails or times out. The socket is the
    /// right address because a self-update swaps the binary and restarts the
    /// daemon under a NEW pid, so the pid recorded at spawn may be gone or
    /// reused by then, while the socket still reaches whichever daemon is live.
    ///
    /// Reached from `LocalConnection::teardown`, which runs when the app
    /// switches away from local and when it quits (`ExitRequested` and `Exit`
    /// both fire, hence idempotent: taking the handle out of the slot makes every
    /// call after the first a no-op).
    ///
    /// It does not stop whatever else holds the local address. The slot is filled
    /// only by `spawn`; a daemon `ensure_running` adopted belongs to whoever
    /// started it. The lock is never held across the request or the kill, which
    /// run on the way out of the app where a block is a hang the user cannot
    /// escape.
    pub async fn reap(
        &self,
        transport: &dyn Transport,
    ) {
        let Some(child) = take_spawned(&self.child) else {
            log::debug!("[local] nothing to reap: this app did not spawn the local daemon");
            return;
        };
        if request_shutdown(transport, SHUTDOWN_TIMEOUT).await {
            log::info!("[local] asked quiver.core to shut down");
            return;
        }
        log::warn!("[local] shutdown request failed, killing the spawned pid instead");
        kill(child);
    }

    /// See [`Self::spawn`]: private for the same reason.
    async fn wait_for_ready(
        &self,
        transport: &dyn Transport,
    ) -> Result<()> {
        for _ in 0..HEALTH_MAX_ATTEMPTS {
            if health_ok(transport).await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(HEALTH_RETRY_MS)).await;
        }
        Err(Error::Daemon(format!(
            "quiver.core did not become ready after {}ms",
            HEALTH_RETRY_MS * HEALTH_MAX_ATTEMPTS as u64,
        )))
    }
}

#[cfg(test)]
mod tests;
