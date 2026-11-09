//! Wire helpers for `GET /v0/config`.
//!
//! A section is not flat: `arrows.auto_retry` and `search.unmarked` are objects. Anything that
//! lists a section's settings goes through here, so a nested object becomes dotted rows
//! (`unmarked.min_stars`) instead of an opaque blob.

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigRow {
    /// Dotted wire path relative to the section, e.g. `unmarked.min_stars`.
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDisplayRow {
    pub key: String,
    pub value: String,
}

/// Walks a config object down to its leaves. An empty object is kept as a leaf, so a group with
/// nothing in it still shows up rather than vanishing.
pub fn flatten_config(section: &Map<String, Value>) -> Vec<ConfigRow> {
    let mut rows = Vec::new();
    flatten_into(section, "", &mut rows);
    rows
}

fn flatten_into(section: &Map<String, Value>, prefix: &str, rows: &mut Vec<ConfigRow>) {
    for (name, value) in section {
        let key = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            Value::Object(inner) if !inner.is_empty() => flatten_into(inner, &key, rows),
            _ => rows.push(ConfigRow {
                key,
                value: value.clone(),
            }),
        }
    }
}

/// JS `String(x)` for a scalar.
fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e15 => format!("{f:.0}"),
            _ => n.to_string(),
        },
        other => other.to_string(),
    }
}

/// Renders one leaf the way the settings list shows it: scalars as-is, arrays comma-joined.
pub fn format_config_value(value: &Value) -> String {
    match value {
        Value::Array(items) if items.is_empty() => "[]".into(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Object(_) | Value::Array(_) => item.to_string(),
                other => scalar_text(other),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(_) => "{}".into(),
        other => scalar_text(other),
    }
}

pub fn to_config_display_rows(section: &Map<String, Value>) -> Vec<ConfigDisplayRow> {
    flatten_config(section)
        .into_iter()
        .map(|ConfigRow { key, value }| ConfigDisplayRow {
            key,
            value: format_config_value(&value),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn rows(value: Value) -> Vec<(String, Value)> {
        flatten_config(value.as_object().unwrap())
            .into_iter()
            .map(|r| (r.key, r.value))
            .collect()
    }

    fn pair(k: &str, v: Value) -> (String, Value) {
        (k.to_string(), v)
    }

    #[test]
    fn keeps_a_flat_section_in_wire_order() {
        assert_eq!(
            rows(json!({"per_provider_limit": 25, "provider_timeout": "10s"})),
            vec![
                pair("per_provider_limit", json!(25)),
                pair("provider_timeout", json!("10s"))
            ]
        );
    }

    #[test]
    fn flattens_nested_objects_to_dotted_keys_at_any_depth() {
        assert_eq!(
            rows(json!({"fetch_concurrency": 8, "unmarked": {"enabled": true, "min_stars": 50}})),
            vec![
                pair("fetch_concurrency", json!(8)),
                pair("unmarked.enabled", json!(true)),
                pair("unmarked.min_stars", json!(50))
            ]
        );
        assert_eq!(
            rows(json!({"a": {"b": {"c": {"d": 1}}, "e": false}})),
            vec![pair("a.b.c.d", json!(1)), pair("a.e", json!(false))]
        );
    }

    #[test]
    fn keeps_arrays_null_and_empty_objects_as_leaves() {
        assert_eq!(
            rows(json!({"hosts": ["a", "b"]})),
            vec![pair("hosts", json!(["a", "b"]))]
        );
        assert_eq!(
            rows(json!({"unmarked": {"min_stars": null}, "channel": null})),
            vec![
                pair("unmarked.min_stars", Value::Null),
                pair("channel", Value::Null)
            ]
        );
        assert_eq!(
            rows(json!({"unmarked": {}})),
            vec![pair("unmarked", json!({}))]
        );
        assert!(rows(json!({})).is_empty());
    }

    #[test]
    fn formats_each_leaf_kind() {
        for (value, expected) in [
            (json!(25), "25"),
            (json!("10s"), "10s"),
            (json!(true), "true"),
            (json!(false), "false"),
            (json!(null), "null"),
            (json!({}), "{}"),
            (json!([]), "[]"),
            (json!(["a", 2, true]), "a, 2, true"),
            (json!(["a", null]), "a, null"),
            (json!([{"a": 1}, {"b": 2}]), r#"{"a":1}, {"b":2}"#),
            (json!(2.0), "2"),
            (json!(1.5), "1.5"),
        ] {
            assert_eq!(format_config_value(&value), expected, "{value}");
        }
    }

    #[test]
    fn display_rows_never_render_an_opaque_object() {
        let section = json!({
            "per_provider_limit": 25, "fetch_concurrency": 8, "provider_timeout": "10s",
            "unmarked": {"min_stars": 50, "probe_limit": 10}, "auto_retry": {"enabled": true, "retries": 3}
        });
        let rows = to_config_display_rows(section.as_object().unwrap());
        let flat: Vec<_> = rows
            .iter()
            .map(|r| (r.key.as_str(), r.value.as_str()))
            .collect();
        assert_eq!(
            flat,
            vec![
                ("per_provider_limit", "25"),
                ("fetch_concurrency", "8"),
                ("provider_timeout", "10s"),
                ("unmarked.min_stars", "50"),
                ("unmarked.probe_limit", "10"),
                ("auto_retry.enabled", "true"),
                ("auto_retry.retries", "3")
            ]
        );
    }
}
