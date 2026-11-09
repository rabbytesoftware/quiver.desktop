//! What the engine tab shows, derived from the config view: values, "can reset" (configured differs
//! from the default), restart and correction notices, and the inline rejection reasons.

use super::auto_register::{auto_register_rejection, is_auto_register_on};
use super::engine::{EngineState, Rejection};
use super::log_level::{normalise_level, Level};
use crate::i18n::{en::settings, Key};
use serde_json::Value;

fn int(config: &Value, section: &str, key: &str) -> Option<i64> {
    config.get(section)?.get(key)?.as_i64()
}

fn text<'a>(config: &'a Value, section: &str, key: &str) -> Option<&'a str> {
    config.get(section)?.get(key)?.as_str()
}

fn flag(config: &Value, section: &str, key: &str) -> Option<bool> {
    config.get(section)?.get(key)?.as_bool()
}

/// Which port field a rejection is about. Each field is patched on its own, but the row has one shared
/// description for both, so a rejection must name the field or it would sit under two fields with no clue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PortField {
    Lowest,
    Highest,
}

impl PortField {
    pub fn label(self) -> Key {
        match self {
            Self::Lowest => settings::ENGINE_PORTS_LOWEST,
            Self::Highest => settings::ENGINE_PORTS_HIGHEST,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PortsRow {
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub can_reset: bool,
    pub problem: Option<(PortField, String)>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogsRows {
    pub disk_enabled: bool,
    pub disk_can_reset: bool,
    pub level: Level,
    pub level_can_reset: bool,
    pub level_problem: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelfUpdateRow {
    pub channel: Option<String>,
    pub can_reset: bool,
    pub problem: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AutoRegisterNote {
    /// The daemon refused the switch; its own reason.
    Rejected(String),
    /// Configured differs from running: takes effect at the next restart.
    Pending,
    Normal,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AutoRegisterRow {
    pub on: bool,
    pub can_reset: bool,
    pub note: AutoRegisterNote,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EngineModel {
    pub patch_error: Option<String>,
    /// "key (message), key (message)" for settings the daemon corrected on disk.
    pub corrected: Option<String>,
    pub restart_required: bool,
    pub ports: PortsRow,
    pub logs: LogsRows,
    pub self_update: SelfUpdateRow,
    pub auto_register: AutoRegisterRow,
}

pub fn corrected_list(corrected: &[Rejection]) -> Option<String> {
    (!corrected.is_empty()).then(|| {
        corrected
            .iter()
            .map(|c| format!("{} ({})", c.key, c.message))
            .collect::<Vec<_>>()
            .join(", ")
    })
}

/// `None` until a view has loaded.
pub fn build_engine_model(state: &EngineState) -> Option<EngineModel> {
    let view = state.view.as_ref()?;
    let (cfg, running, defaults) = (&view.configured, &view.running, &view.defaults);

    let problem = |field: PortField, key: &str| state.why(key).map(|m| (field, m.to_owned()));
    let ports = PortsRow {
        start: int(cfg, "netbridge", "ephemeral_port_start"),
        end: int(cfg, "netbridge", "ephemeral_port_end"),
        can_reset: int(cfg, "netbridge", "ephemeral_port_start")
            != int(defaults, "netbridge", "ephemeral_port_start")
            || int(cfg, "netbridge", "ephemeral_port_end")
                != int(defaults, "netbridge", "ephemeral_port_end"),
        problem: problem(PortField::Lowest, "netbridge.ephemeral_port_start")
            .or_else(|| problem(PortField::Highest, "netbridge.ephemeral_port_end")),
    };

    let level_raw = text(cfg, "logger", "level").unwrap_or("info");
    let logs = LogsRows {
        disk_enabled: flag(cfg, "logger", "enabled").unwrap_or(false),
        disk_can_reset: flag(cfg, "logger", "enabled") != flag(defaults, "logger", "enabled"),
        level: normalise_level(level_raw),
        level_can_reset: text(cfg, "logger", "level") != text(defaults, "logger", "level"),
        level_problem: state.why("logger.level").map(str::to_owned),
    };

    let channel = text(cfg, "arrows", "self_update_channel").filter(|c| !c.is_empty());
    let self_update = SelfUpdateRow {
        channel: channel.map(str::to_owned),
        can_reset: text(cfg, "arrows", "self_update_channel")
            != text(defaults, "arrows", "self_update_channel"),
        problem: state.why("arrows.self_update_channel").map(str::to_owned),
    };

    let on = is_auto_register_on(cfg);
    let note = if let Some(reason) = auto_register_rejection(&state.rejected) {
        AutoRegisterNote::Rejected(reason.to_owned())
    } else if on != is_auto_register_on(running) {
        AutoRegisterNote::Pending
    } else {
        AutoRegisterNote::Normal
    };

    Some(EngineModel {
        patch_error: state.patch_error.clone(),
        corrected: corrected_list(&view.corrected),
        restart_required: !view.restart_required.is_empty(),
        ports,
        logs,
        self_update,
        auto_register: AutoRegisterRow {
            on,
            can_reset: on != is_auto_register_on(defaults),
            note,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::engine::ConfigView;
    use serde_json::json;

    fn state(configured: Value, running: Value, rejected: Vec<Rejection>) -> EngineState {
        let defaults = json!({
            "netbridge": {"ephemeral_port_start": 49152, "ephemeral_port_end": 65535},
            "logger": {"enabled": true, "level": "info"},
            "arrows": {"self_update_channel": "stable"},
            "manifold": {"fletcher": {"enabled": false}},
        });
        EngineState {
            view: Some(ConfigView {
                running,
                configured,
                defaults,
                restart_required: vec![],
                corrected: vec![],
            }),
            rejected,
            loading: false,
            error: None,
            patch_error: None,
        }
    }

    fn pristine() -> Value {
        json!({
            "netbridge": {"ephemeral_port_start": 49152, "ephemeral_port_end": 65535},
            "logger": {"enabled": true, "level": "info"},
            "arrows": {"self_update_channel": "stable"},
            "manifold": {"fletcher": {"enabled": false}},
        })
    }

    fn model(configured: Value, running: Value, rejected: Vec<Rejection>) -> EngineModel {
        build_engine_model(&state(configured, running, rejected)).unwrap()
    }

    #[test]
    fn nothing_to_show_until_the_config_loads() {
        assert!(build_engine_model(&EngineState::default()).is_none());
    }

    #[test]
    fn a_pristine_config_has_nothing_to_reset() {
        let m = model(pristine(), pristine(), vec![]);
        assert!(!m.ports.can_reset && !m.logs.disk_can_reset && !m.logs.level_can_reset);
        assert!(!m.self_update.can_reset && !m.auto_register.can_reset);
        assert_eq!(m.auto_register.note, AutoRegisterNote::Normal);
        assert_eq!((m.ports.start, m.ports.end), (Some(49152), Some(65535)));
    }

    #[test]
    fn a_changed_value_can_be_reset_to_the_default() {
        let mut cfg = pristine();
        cfg["netbridge"]["ephemeral_port_end"] = json!(60000);
        cfg["logger"]["level"] = json!("debug");
        cfg["logger"]["enabled"] = json!(false);
        cfg["arrows"]["self_update_channel"] = json!("nightly");
        cfg["manifold"]["fletcher"]["enabled"] = json!(true);
        let m = model(cfg.clone(), cfg, vec![]);
        assert!(m.ports.can_reset && m.logs.disk_can_reset && m.logs.level_can_reset);
        assert!(m.self_update.can_reset && m.auto_register.can_reset);
        assert_eq!(m.logs.level, Level::Debug);
        assert_eq!(m.self_update.channel.as_deref(), Some("nightly"));
    }

    #[test]
    fn a_log_level_alias_is_shown_as_its_canonical_level() {
        let mut cfg = pristine();
        cfg["logger"]["level"] = json!("fatal");
        assert_eq!(model(cfg.clone(), cfg, vec![]).logs.level, Level::Error);
    }

    #[test]
    fn a_port_rejection_names_the_field_it_is_about() {
        let rejected = vec![Rejection {
            key: "netbridge.ephemeral_port_end".into(),
            message: "too low".into(),
        }];
        let m = model(pristine(), pristine(), rejected);
        assert_eq!(
            m.ports.problem,
            Some((PortField::Highest, "too low".into()))
        );
        let rejected = vec![Rejection {
            key: "netbridge.ephemeral_port_start".into(),
            message: "x".into(),
        }];
        assert_eq!(
            model(pristine(), pristine(), rejected)
                .ports
                .problem
                .unwrap()
                .0,
            PortField::Lowest
        );
    }

    #[test]
    fn auto_register_says_pending_when_configured_differs_from_running_and_rejected_wins() {
        let mut cfg = pristine();
        cfg["manifold"]["fletcher"]["enabled"] = json!(true);
        assert_eq!(
            model(cfg.clone(), pristine(), vec![]).auto_register.note,
            AutoRegisterNote::Pending
        );
        let rejected = vec![Rejection {
            key: "manifold.fletcher.enabled".into(),
            message: "unknown setting".into(),
        }];
        assert_eq!(
            model(cfg, pristine(), rejected).auto_register.note,
            AutoRegisterNote::Rejected("unknown setting".into())
        );
    }

    #[test]
    fn corrections_and_the_restart_notice_come_from_the_daemon() {
        let mut s = state(pristine(), pristine(), vec![]);
        let view = s.view.as_mut().unwrap();
        view.restart_required = vec!["netbridge.ephemeral_port_start".into()];
        view.corrected = vec![
            Rejection {
                key: "logger.level".into(),
                message: "was trace".into(),
            },
            Rejection {
                key: "a.b".into(),
                message: "x".into(),
            },
        ];
        let m = build_engine_model(&s).unwrap();
        assert!(m.restart_required);
        assert_eq!(
            m.corrected.as_deref(),
            Some("logger.level (was trace), a.b (x)")
        );
    }

    #[test]
    fn a_log_level_rejection_is_shown_under_the_row() {
        let rejected = vec![Rejection {
            key: "logger.level".into(),
            message: "unusable log level".into(),
        }];
        assert_eq!(
            model(pristine(), pristine(), rejected)
                .logs
                .level_problem
                .as_deref(),
            Some("unusable log level")
        );
    }
}
