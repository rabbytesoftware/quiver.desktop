//! The Methods modal (inspect only): the five lifecycle actions, then the arrow's own methods.
//! Port of `methods-panel.tsx`'s row building, with the two groups of the new design.

use super::facts::{ArrowFacts, StepSpec};
use crate::i18n::{en::arrow, Key};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MethodName {
    /// A lifecycle verb, named like its action button.
    Lifecycle(Key),
    /// A custom method, shown as written in the manifest.
    Custom(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MethodRow {
    pub name: MethodName,
    pub description: Option<String>,
    pub steps: Vec<StepSpec>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MethodsModel {
    pub lifecycle: Vec<MethodRow>,
    pub custom: Vec<MethodRow>,
}

impl MethodsModel {
    /// The count in the modal title.
    pub fn count(&self) -> usize {
        self.lifecycle.len() + self.custom.len()
    }
}

/// A lifecycle action with no declared steps is left out rather than shown as "0 steps": an empty list
/// means core has nothing declared for it, not that there is a method worth documenting. A custom
/// method is shown regardless, since the manifest named it.
pub fn build_methods(facts: &ArrowFacts) -> MethodsModel {
    let mut lifecycle = Vec::new();
    if let Some(l) = &facts.lifecycle {
        let order = [
            (arrow::ACTION_INSTALL, &l.install),
            (arrow::ACTION_UPDATE, &l.update),
            (arrow::ACTION_START, &l.execute),
            (arrow::ACTION_STOP, &l.stop),
            (arrow::ACTION_UNINSTALL, &l.uninstall),
        ];
        for (key, steps) in order {
            if !steps.is_empty() {
                lifecycle.push(MethodRow {
                    name: MethodName::Lifecycle(key),
                    description: None,
                    steps: steps.clone(),
                });
            }
        }
    }
    let custom = facts
        .methods
        .iter()
        .map(|m| MethodRow {
            name: MethodName::Custom(m.name.clone()),
            description: (!m.description.is_empty()).then(|| m.description.clone()),
            steps: m.steps.clone(),
        })
        .collect();
    MethodsModel { lifecycle, custom }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_details::facts::fixtures::installed;
    use crate::arrow_details::facts::MethodFacts;
    use crate::arrow_details::status::RunState;

    #[test]
    fn lifecycle_rows_come_first_in_the_order_they_run_and_skip_empty_ones() {
        let mut facts = installed(RunState::Ready);
        facts.lifecycle.as_mut().unwrap().update.clear();
        let model = build_methods(&facts);
        let names: Vec<_> = model.lifecycle.iter().map(|r| r.name.clone()).collect();
        assert_eq!(
            names,
            [
                MethodName::Lifecycle(arrow::ACTION_INSTALL),
                MethodName::Lifecycle(arrow::ACTION_START),
                MethodName::Lifecycle(arrow::ACTION_STOP),
                MethodName::Lifecycle(arrow::ACTION_UNINSTALL),
            ]
        );
    }

    #[test]
    fn custom_methods_are_shown_even_without_steps_and_count_with_the_lifecycle() {
        let mut facts = installed(RunState::Ready);
        facts.methods = vec![
            MethodFacts {
                name: "backup".into(),
                description: "Snapshot".into(),
                steps: vec![],
            },
            MethodFacts {
                name: "rcon".into(),
                description: String::new(),
                steps: vec![],
            },
        ];
        let model = build_methods(&facts);
        assert_eq!(model.custom.len(), 2);
        assert_eq!(model.custom[0].description.as_deref(), Some("Snapshot"));
        assert_eq!(model.custom[1].description, None);
        assert_eq!(model.count(), 7);
    }

    #[test]
    fn no_target_means_no_lifecycle_rows() {
        let mut facts = installed(RunState::Ready);
        facts.lifecycle = None;
        assert!(build_methods(&facts).lifecycle.is_empty());
    }
}
