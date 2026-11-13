use serde_json::Value;

/// The console capability `GET /versions` advertises in `features`.
pub const CONSOLE_FEATURE: &str = "console.v1";

/// `GET /versions`, as this client reads it. Everything beyond `version` is optional on the wire.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CoreVersions {
    pub version: String,
    pub build_id: String,
    pub commit: String,
    pub built_at: String,
    pub channel: String,
    pub features: Vec<String>,
}

fn text(
    data: &Value,
    key: &str,
) -> String {
    data.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A daemon that predates the console sends no `features`, which reads as none.
pub fn parse_versions(data: &Value) -> Option<CoreVersions> {
    data.is_object().then(|| CoreVersions {
        version: text(data, "version"),
        build_id: text(data, "build_id"),
        commit: text(data, "commit"),
        built_at: text(data, "built_at"),
        channel: text(data, "channel"),
        features: data
            .get("features")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|f| f.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

pub fn supports_console(versions: Option<&CoreVersions>) -> bool {
    versions.is_some_and(|v| v.features.iter().any(|f| f == CONSOLE_FEATURE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_every_field_and_defaults_the_missing_ones() {
        let v = parse_versions(&json!({"version":"26.5.1","build_id":"b","commit":"abc","built_at":"t","channel":"stable","features":["console.v1","x"]})).unwrap();
        assert_eq!(
            (v.version.as_str(), v.channel.as_str(), v.features.len()),
            ("26.5.1", "stable", 2)
        );
        let bare = parse_versions(&json!({"version":"1"})).unwrap();
        assert_eq!((bare.commit.as_str(), bare.features.len()), ("", 0));
    }

    #[test]
    fn non_string_features_are_dropped_and_non_objects_are_unreadable() {
        let v = parse_versions(&json!({"features":["console.v1", 3, null]})).unwrap();
        assert_eq!(v.features, ["console.v1"]);
        for bad in [json!(null), json!("x"), json!([1])] {
            assert_eq!(parse_versions(&bad), None);
        }
    }

    #[test]
    fn the_console_is_offered_only_with_the_flag() {
        let with = parse_versions(&json!({"features":["console.v1"]}));
        let without = parse_versions(&json!({"version":"1"}));
        assert!(supports_console(with.as_ref()));
        assert!(!supports_console(without.as_ref()));
        assert!(!supports_console(None));
    }
}
