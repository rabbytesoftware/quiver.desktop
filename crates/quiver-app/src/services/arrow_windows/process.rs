//! The real launcher: one `quiver-arrow-window` process per arrow app, JSON lines on its pipes.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;

use quiver_arrow_host::{decode_line, encode_line, HostToWindow, WindowToHost};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::JoinHandle;

use super::launcher::{HelperArgs, HelperLauncher, HelperLink};
use crate::services::{Error, Result};

/// File name of the helper, which ships next to the main binary.
pub const HELPER_BINARY: &str = if cfg!(windows) {
    "quiver-arrow-window.exe"
} else {
    "quiver-arrow-window"
};

pub struct ProcessLauncher {
    program: PathBuf,
    /// Passed before the URL and the title; empty for the real helper.
    leading_args: Vec<OsString>,
}

impl ProcessLauncher {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            leading_args: Vec::new(),
        }
    }

    /// For running the helper through an interpreter (tests do).
    pub fn with_leading_args(
        mut self,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        self.leading_args = args.into_iter().map(Into::into).collect();
        self
    }

    /// The helper next to the running executable, if it is there.
    pub fn next_to_current_exe() -> Option<Self> {
        let path = std::env::current_exe().ok()?.parent()?.join(HELPER_BINARY);
        path.exists().then(|| Self::new(path))
    }
}

/// Owns the child and its two pumps. Dropping it kills the process (`kill_on_drop`) and stops the
/// pumps, which closes the helper's stdin: a helper that somehow survives the kill exits on EOF.
struct ProcessGuard {
    _child: tokio::process::Child,
    pumps: [JoinHandle<()>; 2],
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        for pump in &self.pumps {
            pump.abort();
        }
    }
}

impl HelperLauncher for ProcessLauncher {
    fn launch(&self, args: &HelperArgs) -> Result<HelperLink> {
        let mut child = Command::new(&self.program)
            .args(&self.leading_args)
            .arg(&args.url)
            .arg(&args.title)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Helper(format!("{}: {e}", self.program.display())))?;
        let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(Error::Helper("the helper has no stdio".into()));
        };

        let (to_helper, mut commands) = unbounded_channel::<HostToWindow>();
        let writer = tokio::spawn(async move {
            while let Some(command) = commands.recv().await {
                let Ok(mut line) = encode_line(&command) else {
                    continue;
                };
                line.push('\n');
                if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        let (events, from_helper) = unbounded_channel::<WindowToHost>();
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match decode_line::<WindowToHost>(&line) {
                    Ok(event) => {
                        if events.send(event).is_err() {
                            break;
                        }
                    }
                    Err(e) => log::warn!("arrow window: bad message from helper: {e}"),
                }
            }
            // Dropping `events` is how the supervisor learns the helper is gone.
        });

        Ok(HelperLink {
            to_helper,
            from_helper,
            guard: Box::new(ProcessGuard {
                _child: child,
                pumps: [writer, reader],
            }),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// `sh -c <script> helper <url> <title>`: the script is a stand-in for the helper binary.
    fn shell(script: &str) -> ProcessLauncher {
        ProcessLauncher::new("/bin/sh").with_leading_args(["-c", script, "helper"])
    }

    fn args() -> HelperArgs {
        HelperArgs {
            url: "arrow-app://abc.localhost/".into(),
            title: "Chat".into(),
        }
    }

    #[tokio::test]
    async fn lines_flow_both_ways_and_the_helper_gets_the_url_and_title() {
        // Reports its arguments as a protocol request (method = title, uri = url), then answers
        // a Close with Closed.
        let script = r#"printf '{"type":"protocol_request","id":1,"method":"%s","uri":"%s","headers":[],"body":""}\n' "$2" "$1"; read line; printf '{"type":"closed"}\n'"#;
        let mut link = shell(script).launch(&args()).unwrap();
        let Some(WindowToHost::ProtocolRequest(req)) = link.from_helper.recv().await else {
            panic!("expected the helper's request");
        };
        assert_eq!(
            (req.method.as_str(), req.uri.as_str()),
            ("Chat", "arrow-app://abc.localhost/")
        );
        link.to_helper.send(HostToWindow::Close).unwrap();
        assert_eq!(link.from_helper.recv().await, Some(WindowToHost::Closed));
        assert_eq!(link.from_helper.recv().await, None);
    }

    #[tokio::test]
    async fn garbage_on_stdout_is_skipped_not_fatal() {
        let script = r#"echo garbage; echo; printf '{"type":"ready"}\n'"#;
        let mut link = shell(script).launch(&args()).unwrap();
        assert_eq!(link.from_helper.recv().await, Some(WindowToHost::Ready));
    }

    /// The helper would sleep for ever and holds a FIFO's write end open; the test reads the other
    /// end, which reaches EOF only when the process is really gone.
    #[tokio::test]
    async fn dropping_the_guard_kills_a_helper_that_would_never_exit() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("alive");
        assert!(std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        let reader_path = fifo.clone();
        let eof = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut sink = Vec::new();
            std::fs::File::open(reader_path)
                .and_then(|mut f| f.read_to_end(&mut sink))
                .is_ok()
        });

        let script = r#"exec 3>"$2"; printf '{"type":"ready"}\n'; exec sleep 1000"#;
        let mut link = shell(script)
            .launch(&HelperArgs {
                url: "arrow-app://x.localhost/".into(),
                title: fifo.to_string_lossy().into_owned(),
            })
            .unwrap();
        assert_eq!(link.from_helper.recv().await, Some(WindowToHost::Ready));
        drop(link.guard);
        assert!(eof.await.unwrap());
    }

    #[test]
    fn a_missing_binary_is_a_helper_error() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();
        let err = ProcessLauncher::new("/nonexistent/quiver-arrow-window")
            .launch(&args())
            .err()
            .unwrap();
        assert!(matches!(err, Error::Helper(_)));
    }
}
