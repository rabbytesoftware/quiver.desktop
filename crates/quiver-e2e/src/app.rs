//! Launching the real app with the bridge enabled.

use crate::client::Bridge;
use crate::protocol::{PORT_ENV, PORT_LINE_PREFIX};
use crate::{Error, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// A running app. Dropping it kills the process; scenarios that check a clean shutdown call
/// [`AppProcess::wait_exit`] after asking the app to quit.
pub struct AppProcess {
    child: Child,
    port: u16,
}

impl AppProcess {
    /// Starts `binary` (built with `--features e2e-bridge`) with `env` and waits for the line
    /// `QUIVER_BRIDGE_PORT=<port>` on its stdout: the app's own "bridge is listening" signal.
    pub async fn launch(binary: &Path, env: &[(String, String)]) -> Result<Self> {
        let mut cmd = Command::new(binary);
        cmd.env(PORT_ENV, "0").envs(env.iter().map(|(k, v)| (k, v)));
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Launch(format!("{}: {e}", binary.display())))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Launch("no stdout".into()))?;
        let mut lines = BufReader::new(stdout).lines();
        let port = loop {
            match lines.next_line().await? {
                Some(line) => {
                    if let Some(rest) = line.strip_prefix(PORT_LINE_PREFIX) {
                        break rest
                            .trim()
                            .parse()
                            .map_err(|_| Error::Launch(format!("bad port line: {line}")))?;
                    }
                }
                None => return Err(Error::Launch("app exited before the bridge came up".into())),
            }
        };
        // Keep draining stdout so the app never blocks on a full pipe.
        tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
        Ok(Self { child, port })
    }

    pub async fn bridge(&self) -> Result<Bridge> {
        Bridge::connect(self.port).await
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Kills the process outright (a crash, not a quit).
    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }

    /// Waits for the process to exit on its own (after the scenario asked the app to quit).
    pub async fn wait_exit(&mut self) -> Result<std::process::ExitStatus> {
        Ok(self.child.wait().await?)
    }
}
