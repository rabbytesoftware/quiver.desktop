//! The Settings form's decisions (port of `variable-field.tsx` and the seeding in
//! `arrow-details-screen.tsx`): which control a variable gets, the values it starts with and whether
//! a typed value is acceptable. The values are plain strings, as the daemon takes them.

use std::collections::BTreeMap;

use crate::domain::{ArrowVariable, VariableType};

/// The control a variable is edited with.
#[derive(Clone, PartialEq, Debug)]
pub enum FieldKind {
    Text,
    /// Masked until revealed.
    Secret,
    Switch,
    Select(Vec<String>),
    Number {
        min: Option<f64>,
        max: Option<f64>,
    },
}

/// `sensitive` wins over `type`: a sensitive value is still masked whatever type it is declared as.
pub fn field_kind(variable: &ArrowVariable) -> FieldKind {
    if variable.sensitive == Some(true) {
        return FieldKind::Secret;
    }
    match variable.var_type {
        VariableType::Boolean => FieldKind::Switch,
        VariableType::Select => FieldKind::Select(variable.values.clone().unwrap_or_default()),
        VariableType::Number => FieldKind::Number {
            min: variable.min,
            max: variable.max,
        },
        VariableType::String => FieldKind::Text,
    }
}

/// The manifest's defaults: there is no saved value to read back from Core, so a starting value
/// is the manifest's own default or nothing.
pub fn initial_values(variables: &[ArrowVariable]) -> BTreeMap<String, String> {
    variables
        .iter()
        .filter_map(|v| v.default.clone().map(|d| (v.name.clone(), d)))
        .collect()
}

/// What a field shows: the typed value, else the default, else empty.
pub fn shown_value(
    variable: &ArrowVariable,
    values: &BTreeMap<String, String>,
) -> String {
    values
        .get(&variable.name)
        .cloned()
        .or_else(|| variable.default.clone())
        .unwrap_or_default()
}

/// "1–10", "−∞–10" or "0–∞"; `None` for a number with no bounds.
pub fn range_hint(
    min: Option<f64>,
    max: Option<f64>,
) -> Option<String> {
    if min.is_none() && max.is_none() {
        return None;
    }
    let side = |v: Option<f64>, open: &str| v.map_or_else(|| open.to_owned(), |v| format!("{v}"));
    Some(format!("{}–{}", side(min, "−∞"), side(max, "∞")))
}

#[derive(Clone, PartialEq, Debug)]
pub enum FieldError {
    NotANumber,
    BelowMin(f64),
    AboveMax(f64),
    NotAnOption,
}

impl FieldError {
    pub fn message(&self) -> String {
        match self {
            Self::NotANumber => "Enter a number".into(),
            Self::BelowMin(min) => format!("At least {min}"),
            Self::AboveMax(max) => format!("At most {max}"),
            Self::NotAnOption => "Pick one of the listed values".into(),
        }
    }
}

/// `None` when `value` is acceptable for `variable`. An empty value is acceptable (the daemon
/// applies the manifest default); only a value that is there and wrong is refused.
pub fn validate(
    variable: &ArrowVariable,
    value: &str,
) -> Option<FieldError> {
    if value.is_empty() {
        return None;
    }
    match field_kind(variable) {
        FieldKind::Number { min, max } => {
            let Ok(n) = value.trim().parse::<f64>() else {
                return Some(FieldError::NotANumber);
            };
            if !n.is_finite() {
                return Some(FieldError::NotANumber);
            }
            if let Some(min) = min.filter(|m| n < *m) {
                return Some(FieldError::BelowMin(min));
            }
            max.filter(|m| n > *m).map(FieldError::AboveMax)
        }
        FieldKind::Select(options) => {
            (!options.iter().any(|o| o == value)).then_some(FieldError::NotAnOption)
        }
        _ => None,
    }
}

/// Every invalid variable by name, in declaration order.
pub fn invalid_fields(
    variables: &[ArrowVariable],
    values: &BTreeMap<String, String>,
) -> Vec<(String, FieldError)> {
    variables
        .iter()
        .filter_map(|v| validate(v, &shown_value(v, values)).map(|e| (v.name.clone(), e)))
        .collect()
}

/// Formats a number for the form: whole numbers without a fraction.
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{n:.0}")
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(
        name: &str,
        var_type: VariableType,
    ) -> ArrowVariable {
        ArrowVariable {
            name: name.into(),
            description: String::new(),
            var_type,
            default: None,
            values: None,
            min: None,
            max: None,
            sensitive: None,
        }
    }

    #[test]
    fn each_type_gets_its_control_and_sensitive_wins() {
        assert_eq!(field_kind(&var("a", VariableType::String)), FieldKind::Text);
        assert_eq!(
            field_kind(&var("a", VariableType::Boolean)),
            FieldKind::Switch
        );
        let mut secret = var("a", VariableType::Number);
        secret.sensitive = Some(true);
        assert_eq!(field_kind(&secret), FieldKind::Secret);
        let mut select = var("a", VariableType::Select);
        select.values = Some(vec!["x".into(), "y".into()]);
        assert_eq!(
            field_kind(&select),
            FieldKind::Select(vec!["x".into(), "y".into()])
        );
    }

    #[test]
    fn only_declared_defaults_seed_the_form() {
        let mut with_default = var("port", VariableType::Number);
        with_default.default = Some("25565".into());
        let values = initial_values(&[with_default.clone(), var("motd", VariableType::String)]);
        assert_eq!(values.len(), 1);
        assert_eq!(shown_value(&with_default, &values), "25565");
        assert_eq!(shown_value(&var("motd", VariableType::String), &values), "");
    }

    #[test]
    fn a_typed_value_wins_over_the_default() {
        let mut v = var("port", VariableType::Number);
        v.default = Some("1".into());
        let values = BTreeMap::from([("port".to_owned(), "2".to_owned())]);
        assert_eq!(shown_value(&v, &values), "2");
    }

    #[test]
    fn the_range_hint_marks_open_ends() {
        assert_eq!(range_hint(None, None), None);
        assert_eq!(range_hint(Some(1.0), Some(10.0)).as_deref(), Some("1–10"));
        assert_eq!(range_hint(None, Some(10.0)).as_deref(), Some("−∞–10"));
        assert_eq!(range_hint(Some(0.5), None).as_deref(), Some("0.5–∞"));
    }

    #[test]
    fn numbers_are_checked_against_their_bounds() {
        let mut v = var("n", VariableType::Number);
        v.min = Some(1.0);
        v.max = Some(10.0);
        assert_eq!(validate(&v, "5"), None);
        assert_eq!(validate(&v, "0"), Some(FieldError::BelowMin(1.0)));
        assert_eq!(validate(&v, "11"), Some(FieldError::AboveMax(10.0)));
        assert_eq!(validate(&v, "abc"), Some(FieldError::NotANumber));
        assert_eq!(validate(&v, "NaN"), Some(FieldError::NotANumber));
        assert_eq!(validate(&v, ""), None);
    }

    #[test]
    fn a_select_only_takes_its_listed_values() {
        let mut v = var("mode", VariableType::Select);
        v.values = Some(vec!["a".into(), "b".into()]);
        assert_eq!(validate(&v, "a"), None);
        assert_eq!(validate(&v, "c"), Some(FieldError::NotAnOption));
    }

    #[test]
    fn invalid_fields_lists_them_in_declaration_order() {
        let mut n = var("n", VariableType::Number);
        n.max = Some(1.0);
        let mut m = var("m", VariableType::Number);
        m.min = Some(5.0);
        let values = BTreeMap::from([
            ("n".to_owned(), "3".to_owned()),
            ("m".to_owned(), "1".to_owned()),
        ]);
        let bad = invalid_fields(&[n, m], &values);
        assert_eq!(bad.len(), 2);
        assert_eq!(bad[0].0, "n");
        assert_eq!(bad[1].1, FieldError::BelowMin(5.0));
    }

    #[test]
    fn whole_numbers_print_without_a_fraction() {
        assert_eq!(format_number(3.0), "3");
        assert_eq!(format_number(2.5), "2.5");
    }
}
