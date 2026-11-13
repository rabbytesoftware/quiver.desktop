//! The OS process table and the version of the daemon holding the socket.

use std::process::Command;

use super::daemon::Daemon;

pub fn alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

pub fn pgrep(args: &[&str]) -> Vec<u32> {
    Command::new("pgrep")
        .args(args)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// `GET /versions`: the build of the process holding the socket right now, not of any file on disk.
pub async fn daemon_version(d: &Daemon) -> Option<String> {
    let reply = d.get("/versions").await.ok()?;
    reply.json()["version"].as_str().map(String::from)
}

pub fn kill(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
}
