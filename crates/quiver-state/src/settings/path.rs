//! Putting the CLI's directory on PATH (the engine tab's "PATH" row).

use crate::arrow_details::invoke::ActionFailure;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PathStatus {
    pub bin_dir: String,
    pub on_path: bool,
    pub configured: bool,
    pub files: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathState {
    /// The daemon already sees the directory on PATH.
    Ready,
    /// Configured, but only terminals opened from now on will have it.
    Restart,
    /// Neither: the user has to ask for it.
    Setup,
}

pub fn path_state(status: &PathStatus) -> PathState {
    if status.on_path {
        PathState::Ready
    } else if status.configured {
        PathState::Restart
    } else {
        PathState::Setup
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PathStore {
    pub status: Option<PathStatus>,
    pub loading: bool,
    pub setting_up: bool,
    /// A daemon that predates `/v0/system/path` answers 404; the panel hides the entry rather than
    /// showing a failure the user cannot act on.
    pub unavailable: bool,
    /// Owned by `load`: there is no status to show, so the row offers a retry.
    pub error: Option<String>,
    /// Owned by `setup`: a status is already on screen, so this renders inline.
    pub setup_error: Option<String>,
}

impl Default for PathStore {
    fn default() -> Self {
        Self {
            status: None,
            loading: true,
            setting_up: false,
            unavailable: false,
            error: None,
            setup_error: None,
        }
    }
}

impl PathStore {
    pub fn begin_load(&mut self) {
        self.loading = true;
        self.error = None;
        self.setup_error = None;
        self.unavailable = false;
    }

    pub fn loaded(&mut self, result: Result<PathStatus, ActionFailure>) {
        self.loading = false;
        match result {
            Ok(status) => self.status = Some(status),
            Err(failure) if failure.status == Some(404) => self.unavailable = true,
            Err(failure) => self.error = Some(failure.message),
        }
    }

    pub fn begin_setup(&mut self) {
        self.setting_up = true;
        self.setup_error = None;
    }

    /// The response is the updated status, used as-is (no re-read).
    pub fn setup_done(&mut self, result: Result<PathStatus, ActionFailure>) {
        self.setting_up = false;
        match result {
            Ok(status) => self.status = Some(status),
            Err(failure) => self.setup_error = Some(failure.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(on_path: bool, configured: bool) -> PathStatus {
        PathStatus {
            bin_dir: "/b".into(),
            on_path,
            configured,
            files: vec![],
        }
    }

    fn fail(code: Option<u16>) -> ActionFailure {
        ActionFailure {
            status: code,
            message: "boom".into(),
        }
    }

    #[test]
    fn the_state_follows_what_the_daemon_sees() {
        assert_eq!(path_state(&status(true, true)), PathState::Ready);
        assert_eq!(path_state(&status(true, false)), PathState::Ready);
        assert_eq!(path_state(&status(false, true)), PathState::Restart);
        assert_eq!(path_state(&status(false, false)), PathState::Setup);
    }

    #[test]
    fn loading_stores_the_status() {
        let mut store = PathStore::default();
        assert!(store.loading);
        store.begin_load();
        store.loaded(Ok(status(false, false)));
        assert!(!store.loading);
        assert!(store.status.is_some());
    }

    #[test]
    fn a_404_hides_the_row_any_other_failure_offers_a_retry() {
        let mut store = PathStore::default();
        store.loaded(Err(fail(Some(404))));
        assert!(store.unavailable && store.error.is_none());
        store.begin_load();
        assert!(!store.unavailable);
        store.loaded(Err(fail(Some(500))));
        assert_eq!(store.error.as_deref(), Some("boom"));
        assert!(!store.unavailable);
    }

    #[test]
    fn setup_failure_is_inline_and_keeps_the_status() {
        let mut store = PathStore::default();
        store.loaded(Ok(status(false, false)));
        store.begin_setup();
        assert!(store.setting_up);
        store.setup_done(Err(fail(None)));
        assert!(!store.setting_up);
        assert_eq!(store.setup_error.as_deref(), Some("boom"));
        assert!(store.status.is_some());
        store.begin_setup();
        assert!(store.setup_error.is_none());
        store.setup_done(Ok(status(false, true)));
        assert_eq!(
            path_state(store.status.as_ref().unwrap()),
            PathState::Restart
        );
    }
}
