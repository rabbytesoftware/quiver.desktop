//! Quiver Core's own subsystems as settings (Developer tab): Fletcher's neighbours in `config.yaml` that the
//! Engine tab does not cover. Every row is one scalar key of `GET /v0/config`, written back through the same
//! `PATCH` (a `null` puts the daemon's default back). Core validates and explains a refusal ("must be a positive
//! duration such as 30s or 5m"), so a typed value is sent as it is and the reason is shown under the row.
//!
//! A key the running daemon does not report (an older core) is left out rather than drawn as a dead control.

use serde_json::{Map, Value};

use super::engine::EngineState;
use crate::i18n::en::settings as keys;
use crate::i18n::Key;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptionKind {
    Toggle,
    /// A whole number of at least `min`.
    Count {
        min: i64,
    },
    /// A Go duration (`30s`, `6h`); `off` allows `0s`, which turns the job off.
    Duration {
        off: bool,
    },
}

#[derive(Debug)]
pub struct CoreOption {
    /// The e2e id: `settings.config.<id>`.
    pub id: &'static str,
    pub path: &'static [&'static str],
    pub label: Key,
    pub description: Key,
    pub kind: OptionKind,
}

impl CoreOption {
    /// The dotted key core uses in `rejected` and `restart_required` (`arrows.auto_retry.retries`).
    pub fn key(&self) -> String {
        self.path.join(".")
    }
}

#[derive(Debug)]
pub struct CoreSection {
    pub title: Key,
    pub options: &'static [CoreOption],
}

pub const CORE_SECTIONS: &[CoreSection] = &[
    CoreSection {
        title: keys::CORE_UPDATES_TITLE,
        options: &[CoreOption {
            id: "version-check-interval",
            path: &["arrows", "version_check_interval"],
            label: keys::CORE_VERSION_CHECK_LABEL,
            description: keys::CORE_VERSION_CHECK_DESCRIPTION,
            kind: OptionKind::Duration { off: true },
        }],
    },
    CoreSection {
        title: keys::CORE_MANIFESTS_TITLE,
        options: &[
            CoreOption {
                id: "fetch-timeout",
                path: &["manifold", "fetch_timeout"],
                label: keys::CORE_FETCH_TIMEOUT_LABEL,
                description: keys::CORE_FETCH_TIMEOUT_DESCRIPTION,
                kind: OptionKind::Duration { off: false },
            },
            CoreOption {
                id: "auto-retry",
                path: &["arrows", "auto_retry", "enabled"],
                label: keys::CORE_AUTO_RETRY_LABEL,
                description: keys::CORE_AUTO_RETRY_DESCRIPTION,
                kind: OptionKind::Toggle,
            },
            CoreOption {
                id: "retries",
                path: &["arrows", "auto_retry", "retries"],
                label: keys::CORE_RETRIES_LABEL,
                description: keys::CORE_RETRIES_DESCRIPTION,
                kind: OptionKind::Count { min: 0 },
            },
        ],
    },
    CoreSection {
        title: keys::CORE_SEARCH_TITLE,
        options: &[
            CoreOption {
                id: "per-provider-limit",
                path: &["search", "per_provider_limit"],
                label: keys::CORE_PER_SOURCE_LABEL,
                description: keys::CORE_PER_SOURCE_DESCRIPTION,
                kind: OptionKind::Count { min: 1 },
            },
            CoreOption {
                id: "provider-timeout",
                path: &["search", "provider_timeout"],
                label: keys::CORE_SOURCE_TIMEOUT_LABEL,
                description: keys::CORE_SOURCE_TIMEOUT_DESCRIPTION,
                kind: OptionKind::Duration { off: false },
            },
            CoreOption {
                id: "unmarked-min-stars",
                path: &["search", "unmarked", "min_stars"],
                label: keys::CORE_MIN_STARS_LABEL,
                description: keys::CORE_MIN_STARS_DESCRIPTION,
                kind: OptionKind::Count { min: 0 },
            },
        ],
    },
    CoreSection {
        title: keys::CORE_RECOMMENDATIONS_TITLE,
        options: &[
            CoreOption {
                id: "recommendations",
                path: &["recommendation", "enabled"],
                label: keys::CORE_RECOMMENDATIONS_LABEL,
                description: keys::CORE_RECOMMENDATIONS_DESCRIPTION,
                kind: OptionKind::Toggle,
            },
            CoreOption {
                id: "recommendations-refresh",
                path: &["recommendation", "refresh_interval"],
                label: keys::CORE_REFRESH_LABEL,
                description: keys::CORE_REFRESH_DESCRIPTION,
                kind: OptionKind::Duration { off: false },
            },
        ],
    },
    CoreSection {
        title: keys::CORE_NETWORK_TITLE,
        options: &[CoreOption {
            id: "port-forwarding",
            path: &["netbridge", "enabled"],
            label: keys::CORE_PORT_FORWARDING_LABEL,
            description: keys::CORE_PORT_FORWARDING_DESCRIPTION,
            kind: OptionKind::Toggle,
        }],
    },
];

#[derive(Clone, PartialEq, Debug)]
pub enum OptionValue {
    Toggle(bool),
    /// A count or a duration, as text for its field.
    Text(String),
}

#[derive(Clone, Debug)]
pub struct CoreOptionRow {
    pub option: &'static CoreOption,
    pub value: OptionValue,
    /// Configured differs from the daemon's default.
    pub can_reset: bool,
    /// Why core refused the last value, verbatim.
    pub problem: Option<String>,
    /// Saved, but the running daemon still uses the old value until it restarts.
    pub pending_restart: bool,
}

fn at<'a>(
    value: &'a Value,
    path: &[&str],
) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(*key))
}

fn shown(
    kind: OptionKind,
    value: &Value,
) -> Option<OptionValue> {
    match kind {
        OptionKind::Toggle => value.as_bool().map(OptionValue::Toggle),
        OptionKind::Count { .. } => value.as_i64().map(|n| OptionValue::Text(n.to_string())),
        OptionKind::Duration { .. } => value.as_str().map(|s| OptionValue::Text(s.to_owned())),
    }
}

/// The rows, by section, of every option the daemon reports; `None` until the config has loaded.
pub fn core_option_rows(state: &EngineState) -> Option<Vec<(Key, Vec<CoreOptionRow>)>> {
    let view = state.view.as_ref()?;
    let sections = CORE_SECTIONS
        .iter()
        .map(|section| {
            let rows = section
                .options
                .iter()
                .filter_map(|option| {
                    let configured = at(&view.configured, option.path)
                        .or_else(|| at(&view.running, option.path))?;
                    let value = shown(option.kind, configured)?;
                    let key = option.key();
                    Some(CoreOptionRow {
                        option,
                        can_reset: at(&view.defaults, option.path).is_some_and(|d| d != configured),
                        problem: state.why(&key).map(str::to_owned),
                        pending_restart: view.restart_required.contains(&key)
                            || at(&view.running, option.path).is_some_and(|r| r != configured),
                        value,
                    })
                })
                .collect::<Vec<_>>();
            (section.title, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    Some(sections)
}

/// `{ "a": { "b": value } }` for the path `a.b`.
pub fn option_patch(
    path: &[&str],
    value: Value,
) -> Value {
    path.iter().rev().fold(value, |inner, key| {
        let mut map = Map::new();
        map.insert((*key).to_owned(), inner);
        Value::Object(map)
    })
}

/// What a field entry sends: a count must be a whole number of at least its minimum (anything else reverts the
/// field without asking core); a duration goes as typed, trimmed, for core to judge. Blank sends nothing.
pub fn parse_option(
    kind: OptionKind,
    raw: &str,
) -> Option<Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match kind {
        OptionKind::Toggle => raw.parse::<bool>().ok().map(Value::Bool),
        OptionKind::Count { min } => raw
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= min)
            .map(Value::from),
        OptionKind::Duration { .. } => Some(Value::from(raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::engine::{ConfigView, Rejection};
    use serde_json::json;

    fn state(
        configured: Value,
        running: Value,
        defaults: Value,
    ) -> EngineState {
        EngineState {
            view: Some(ConfigView {
                running,
                configured,
                defaults,
                restart_required: vec![],
                corrected: vec![],
            }),
            ..EngineState::default()
        }
    }

    fn row<'a>(
        sections: &'a [(Key, Vec<CoreOptionRow>)],
        id: &str,
    ) -> Option<&'a CoreOptionRow> {
        sections
            .iter()
            .flat_map(|(_, rows)| rows)
            .find(|r| r.option.id == id)
    }

    #[test]
    fn every_reported_key_becomes_a_row_and_missing_ones_are_left_out() {
        let config = json!({
            "arrows": {"version_check_interval": "6h", "auto_retry": {"enabled": true, "retries": 3}},
            "netbridge": {"enabled": true},
        });
        let sections = core_option_rows(&state(config.clone(), config.clone(), config)).unwrap();
        assert_eq!(
            row(&sections, "retries").map(|r| r.value.clone()),
            Some(OptionValue::Text("3".into()))
        );
        assert_eq!(
            row(&sections, "port-forwarding").map(|r| r.value.clone()),
            Some(OptionValue::Toggle(true))
        );
        // No `search` or `recommendation` in this daemon's config: no rows, no empty sections.
        assert!(row(&sections, "per-provider-limit").is_none());
        assert!(sections.iter().all(|(_, rows)| !rows.is_empty()));
    }

    #[test]
    fn a_changed_value_can_be_reset_and_its_refusal_is_shown() {
        let configured = json!({"manifold": {"fetch_timeout": "2m"}});
        let defaults = json!({"manifold": {"fetch_timeout": "30s"}});
        let mut s = state(configured.clone(), configured, defaults);
        s.rejected = vec![Rejection {
            key: "manifold.fetch_timeout".into(),
            message: "must be a positive duration such as 30s or 5m, got 2x".into(),
        }];
        let sections = core_option_rows(&s).unwrap();
        let fetch = row(&sections, "fetch-timeout").unwrap();
        assert!(fetch.can_reset);
        assert!(fetch
            .problem
            .as_deref()
            .unwrap()
            .contains("positive duration"));
        assert!(!fetch.pending_restart);
    }

    #[test]
    fn a_value_the_running_daemon_does_not_use_yet_waits_for_a_restart() {
        let configured = json!({"recommendation": {"enabled": false}});
        let running = json!({"recommendation": {"enabled": true}});
        let sections = core_option_rows(&state(configured, running.clone(), running)).unwrap();
        assert!(row(&sections, "recommendations").unwrap().pending_restart);
    }

    #[test]
    fn a_patch_nests_the_value_under_its_path() {
        assert_eq!(
            option_patch(&["arrows", "auto_retry", "retries"], json!(5)),
            json!({"arrows": {"auto_retry": {"retries": 5}}})
        );
        assert_eq!(
            option_patch(&["netbridge", "enabled"], Value::Null),
            json!({"netbridge": {"enabled": null}})
        );
    }

    #[test]
    fn counts_are_checked_here_and_durations_are_left_to_core() {
        let count = OptionKind::Count { min: 1 };
        assert_eq!(parse_option(count, " 25 "), Some(json!(25)));
        assert_eq!(parse_option(count, "0"), None);
        assert_eq!(parse_option(count, "2.5"), None);
        assert_eq!(parse_option(count, ""), None);
        let duration = OptionKind::Duration { off: true };
        assert_eq!(parse_option(duration, " 0s "), Some(json!("0s")));
        assert_eq!(parse_option(duration, "soon"), Some(json!("soon")));
    }
}
