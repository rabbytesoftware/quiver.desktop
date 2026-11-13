//! "At a glance": what the page shows in the README's place when the arrow has none, built from the
//! manifest (methods, settings, platforms, source).

use super::facts::ArrowFacts;
use super::methods::build_methods;
use super::platform::runs_on;

/// What fills the left column of the body.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BodyPlan {
    Readme,
    Glance(Glance),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Glance {
    pub method_count: usize,
    /// Lifecycle verbs and custom methods as written, in order ("install, update, start, ...").
    pub method_names: Vec<String>,
    pub setting_count: usize,
    pub runs_on: String,
    pub source: String,
}

/// A README wins; without one the glance is built from the manifest. `readme` is `None` when core
/// answered 404 for it (the arrow shipped a bare `arrow.yaml`).
pub fn body_plan(
    facts: &ArrowFacts,
    readme: Option<&str>,
    source_url: &str,
) -> BodyPlan {
    if readme.is_some_and(|r| !r.trim().is_empty()) {
        return BodyPlan::Readme;
    }
    let methods = build_methods(facts);
    let verb = |key| crate::i18n::Translator::default().t(key);
    let mut names: Vec<String> = Vec::new();
    for row in methods.lifecycle.iter().chain(methods.custom.iter()) {
        names.push(match &row.name {
            super::methods::MethodName::Lifecycle(key) => verb(*key).to_lowercase(),
            super::methods::MethodName::Custom(name) => name.clone(),
        });
    }
    BodyPlan::Glance(Glance {
        method_count: methods.count(),
        method_names: names,
        setting_count: facts.variable_names.len(),
        runs_on: runs_on(&facts.platforms),
        source: source_url.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_details::facts::fixtures::installed;
    use crate::arrow_details::status::RunState;

    #[test]
    fn a_readme_takes_the_left_column() {
        assert_eq!(
            body_plan(&installed(RunState::Ready), Some("# Hi"), ""),
            BodyPlan::Readme
        );
    }

    #[test]
    fn without_a_readme_the_manifest_is_summarised() {
        let mut facts = installed(RunState::Ready);
        facts.variable_names = vec!["A".into(), "B".into(), "C".into()];
        facts.platforms = vec!["linux/amd64".into(), "windows/amd64".into()];
        for readme in [None, Some("  \n")] {
            let BodyPlan::Glance(glance) = body_plan(&facts, readme, "github.com/rabbyte/terraria")
            else {
                panic!("expected a glance");
            };
            assert_eq!(glance.method_count, 5);
            assert_eq!(
                glance.method_names,
                ["install", "update", "start", "stop", "uninstall"]
            );
            assert_eq!(glance.setting_count, 3);
            assert_eq!(glance.runs_on, "Linux · Windows");
            assert_eq!(glance.source, "github.com/rabbyte/terraria");
        }
    }
}
