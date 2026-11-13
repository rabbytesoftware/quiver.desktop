//! Automatic arrow registration is quiver.core's Fletcher: it builds a manifest for a repository that
//! ships none. It is the only switch; the search pass that surfaces such repositories is not flagged.

use super::engine::Rejection;
use serde_json::{json, Value};

const FLETCHER_KEY: &str = "manifold.fletcher";

/// On only when `manifold.fletcher.enabled` is `true`. A daemon that predates the key omits it, and
/// that reads as off, as does every malformed shape.
pub fn is_auto_register_on(config: &Value) -> bool {
    config
        .get("manifold")
        .and_then(|m| m.get("fletcher"))
        .and_then(|f| f.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The config patch that turns Fletcher on or off.
pub fn auto_register_patch(on: bool) -> Value {
    json!({ "manifold": { "fletcher": { "enabled": on } } })
}

/// The patch that puts Fletcher back to whatever the daemon defaults to.
pub fn auto_register_reset_patch() -> Value {
    json!({ "manifold": { "fletcher": { "enabled": null } } })
}

/// Why the daemon refused the switch, if it did. An older daemon does not know the key and rejects it
/// by name, at whatever depth it stopped reading.
pub fn auto_register_rejection(rejected: &[Rejection]) -> Option<&str> {
    rejected
        .iter()
        .find(|r| {
            r.key == FLETCHER_KEY
                || r.key.starts_with(&format!("{FLETCHER_KEY}."))
                || FLETCHER_KEY.starts_with(&format!("{}.", r.key))
        })
        .map(|r| r.message.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_switch_and_treats_every_odd_shape_as_off() {
        let cases = [
            (json!({"manifold": {"fletcher": {"enabled": true}}}), true),
            (json!({"manifold": {"fletcher": {"enabled": false}}}), false),
            (json!({}), false),
            (json!({"manifold": {"fetch_timeout": "30s"}}), false),
            (json!({"manifold": {"fletcher": {}}}), false),
            (json!({"manifold": null}), false),
            (json!({"manifold": "nope"}), false),
            (json!({"manifold": {"fletcher": null}}), false),
            (json!({"manifold": {"fletcher": true}}), false),
            (json!({"manifold": {"fletcher": {"enabled": "yes"}}}), false),
            (json!({"search": {"unmarked": {"enabled": true}}}), false),
        ];
        for (config, expected) in cases {
            assert_eq!(is_auto_register_on(&config), expected, "{config}");
        }
    }

    #[test]
    fn patches_only_the_fletcher_switch_and_round_trips() {
        assert_eq!(
            auto_register_patch(true),
            json!({"manifold": {"fletcher": {"enabled": true}}})
        );
        assert_eq!(
            auto_register_patch(false),
            json!({"manifold": {"fletcher": {"enabled": false}}})
        );
        assert!(auto_register_patch(true).get("search").is_none());
        assert!(auto_register_reset_patch().get("search").is_none());
        assert!(is_auto_register_on(&auto_register_patch(true)));
        assert!(!is_auto_register_on(&auto_register_patch(false)));
    }

    #[test]
    fn reset_sends_null_so_the_daemon_restores_its_default() {
        assert_eq!(
            auto_register_reset_patch(),
            json!({"manifold": {"fletcher": {"enabled": null}}})
        );
    }

    fn rejection(key: &str) -> Rejection {
        Rejection {
            key: key.into(),
            message: "unknown setting".into(),
        }
    }

    #[test]
    fn reports_a_rejection_of_the_leaf_the_group_or_a_section_an_older_daemon_stopped_at() {
        for key in ["manifold.fletcher.enabled", "manifold.fletcher", "manifold"] {
            assert_eq!(
                auto_register_rejection(&[rejection(key)]),
                Some("unknown setting"),
                "{key}"
            );
        }
    }

    #[test]
    fn ignores_rejections_about_other_settings() {
        let others = [
            rejection("logger.level"),
            rejection("manifold.fetch_timeout"),
            rejection("search.unmarked.min_stars"),
        ];
        assert_eq!(auto_register_rejection(&others), None);
        assert_eq!(auto_register_rejection(&[]), None);
    }
}
