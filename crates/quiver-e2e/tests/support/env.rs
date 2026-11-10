//! The environment the box exports (the same `QUIVER_E2E_*` names `run-wdio.sh` exported to the WebDriver
//! specs; see `docker/e2e/scenarios/run-native.sh`). Without `QUIVER_E2E_APP_BINARY` a scenario SKIPS, so a plain
//! `cargo test` stays green on a machine that has no app build.

use std::path::PathBuf;

pub struct Env {
    pub app_binary: PathBuf,
    /// Where per-test homes are created (`QUIVER_E2E_TMP`).
    pub tmp: PathBuf,
    /// `QUIVER_E2E_TEST_NAME`, the full test path run-native.sh runs with `--exact`.
    pub test_name: String,
    pub results: Option<PathBuf>,
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// A variable the scenario cannot do without. The box always sets it; a missing one is a harness mistake.
pub fn required(name: &str) -> String {
    var(name).unwrap_or_else(|| {
        panic!("{name} is unset: this scenario runs only in the E2E box (make test-e2e-box)")
    })
}

pub fn optional(name: &str) -> Option<String> {
    var(name)
}

impl Env {
    /// `None` (after printing why) when the app under test is not configured: the caller returns, the test skips.
    pub fn load(scenario: &str) -> Option<Self> {
        let Some(app_binary) = var("QUIVER_E2E_APP_BINARY") else {
            eprintln!(
                "SKIP {scenario}: QUIVER_E2E_APP_BINARY is unset (run it with `make test-e2e-box TESTS={scenario}`)"
            );
            return None;
        };
        let tmp = var("QUIVER_E2E_TMP")
            .map_or_else(|| std::env::temp_dir().join("quiver-e2e"), PathBuf::from);
        let test_name = var("QUIVER_E2E_TEST_NAME").unwrap_or_else(|| scenario.to_string());
        Some(Self {
            app_binary: PathBuf::from(app_binary),
            tmp,
            test_name,
            results: var("QUIVER_E2E_RESULTS").map(PathBuf::from),
        })
    }

    /// `scenarios::bootstrap::x` -> `scenarios__bootstrap__x`, the directory name run-native.sh uses too.
    pub fn safe_name(&self) -> String {
        self.test_name.replace("::", "__")
    }

    /// Where this test leaves screenshots and notes: `<results>/<test>/`.
    pub fn evidence_dir(&self) -> Option<PathBuf> {
        let dir = self.results.as_ref()?.join(self.safe_name());
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }
}
