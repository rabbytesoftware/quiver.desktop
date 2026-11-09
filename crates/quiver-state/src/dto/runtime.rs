use serde::Deserialize;

use crate::domain::{ActiveRun, ArrowState, LastReturn, RuntimeUpdate};

/// `surface` is absent on a daemon that predates arrow apps, or when the run serves no page; the
/// domain type already reads null and absent alike as `None`.
pub type ActiveRunDto = ActiveRun;

pub fn to_active_run(dto: Option<ActiveRunDto>) -> Option<ActiveRun> {
    dto
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeUpdateDto {
    pub namespace: String,
    pub state: ArrowState,
    #[serde(default)]
    pub active_run: Option<ActiveRunDto>,
    #[serde(default)]
    pub last_return: Option<LastReturn>,
}

pub fn to_runtime_update(dto: RuntimeUpdateDto) -> RuntimeUpdate {
    RuntimeUpdate {
        namespace: dto.namespace,
        state: dto.state,
        active_run: to_active_run(dto.active_run),
        last_return: dto.last_return,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::{Outcome, SurfaceMode};

    fn parse(value: serde_json::Value) -> RuntimeUpdate {
        to_runtime_update(serde_json::from_value(value).unwrap())
    }

    #[test]
    fn maps_namespace_and_state_and_defaults_the_rest_to_none() {
        let u = parse(json!({"namespace": "a@1", "state": "ready"}));
        assert_eq!((u.namespace.as_str(), u.state), ("a@1", ArrowState::Ready));
        assert!(u.active_run.is_none() && u.last_return.is_none());
        let u = parse(json!({"namespace": "a@1", "state": "ready", "active_run": null, "last_return": null}));
        assert!(u.active_run.is_none() && u.last_return.is_none());
    }

    #[test]
    fn preserves_provided_active_run_and_last_return() {
        let u = parse(json!({
            "namespace": "a@1", "state": "running",
            "active_run": {"method": "execute", "variables": {"K": "v"}, "steps": []},
            "last_return": {"method": "install", "outcome": "success"}
        }));
        assert_eq!(u.active_run.unwrap().variables["K"], "v");
        assert_eq!(u.last_return.unwrap().outcome, Outcome::Success);
    }

    #[test]
    fn maps_the_surface_when_present_and_leaves_it_none_when_absent_or_null() {
        let run = |surface: serde_json::Value| {
            parse(json!({"namespace": "a@1", "state": "running",
                "active_run": {"method": "execute", "variables": {}, "steps": [], "surface": surface}}))
                .active_run
                .unwrap()
                .surface
        };
        let s = run(json!({"mode": "listen", "path": "/", "ready": true})).unwrap();
        assert_eq!((s.mode, s.path.as_str(), s.ready), (SurfaceMode::Listen, "/", true));
        assert!(run(json!(null)).is_none());
        let no_surface = parse(json!({"namespace": "a@1", "state": "running", "active_run": {"method": "m"}}));
        assert!(no_surface.active_run.unwrap().surface.is_none());
    }
}
