//! A dependency-graph namespace enriched with whatever the reactive catalog knows about it. Core's
//! `/dependencies` and `/dependents` return bare `namespace@ref` strings only, so this fills in name,
//! icon and live state when possible (port of `dependency-rows.ts`).

use super::status::RunState;
use crate::namespace::{bare_namespace, split_namespace};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CatalogEntryFacts {
    pub namespace: String,
    pub name: String,
    pub icon: Option<String>,
    pub state: RunState,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DependencyRow {
    /// Full `namespace@ref`, exactly as the graph endpoint reported it (for navigation).
    pub namespace: String,
    pub name: String,
    pub icon: Option<String>,
    pub reference: String,
    pub state: RunState,
    /// False when the namespace is not in the catalog at all: `name` falls back to the bare namespace,
    /// no icon, and `state` is a filler callers ignore in favour of "Discovered".
    pub user_installed: bool,
}

pub fn build_dependency_rows(
    namespaces: &[String],
    catalog: &[CatalogEntryFacts],
) -> Vec<DependencyRow> {
    namespaces
        .iter()
        .map(|namespace| {
            let parts = split_namespace(namespace);
            let (head, reference) = (parts.head, parts.tail.get(1..).unwrap_or(""));
            let found = catalog
                .iter()
                .find(|e| &e.namespace == namespace)
                .or_else(|| {
                    catalog
                        .iter()
                        .find(|e| bare_namespace(&e.namespace) == head)
                });
            match found {
                Some(e) => DependencyRow {
                    namespace: namespace.clone(),
                    name: e.name.clone(),
                    icon: e.icon.clone(),
                    reference: reference.to_owned(),
                    state: e.state,
                    user_installed: true,
                },
                None => DependencyRow {
                    namespace: namespace.clone(),
                    name: head.to_owned(),
                    icon: None,
                    reference: reference.to_owned(),
                    state: RunState::Absent,
                    user_installed: false,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(namespace: &str, name: &str, state: RunState) -> CatalogEntryFacts {
        CatalogEntryFacts {
            namespace: namespace.into(),
            name: name.into(),
            icon: Some("i.png".into()),
            state,
        }
    }

    #[test]
    fn an_exact_catalog_match_supplies_name_icon_and_state() {
        let rows = build_dependency_rows(
            &["github.com/a/b@v1".into()],
            &[entry("github.com/a/b@v1", "B", RunState::Running)],
        );
        assert_eq!(rows[0].name, "B");
        assert_eq!(rows[0].state, RunState::Running);
        assert_eq!(rows[0].reference, "v1");
        assert!(rows[0].user_installed);
    }

    #[test]
    fn another_version_of_the_same_arrow_still_identifies_it() {
        let rows = build_dependency_rows(
            &["github.com/a/b@v2".into()],
            &[entry("github.com/a/b@stable", "B", RunState::Ready)],
        );
        assert_eq!(rows[0].name, "B");
        assert_eq!(rows[0].namespace, "github.com/a/b@v2");
    }

    #[test]
    fn an_unknown_namespace_falls_back_to_the_bare_one() {
        let rows = build_dependency_rows(&["github.com/x/y@v3".into()], &[]);
        assert_eq!(rows[0].name, "github.com/x/y");
        assert_eq!(rows[0].icon, None);
        assert!(!rows[0].user_installed);
        assert_eq!(rows[0].reference, "v3");
    }
}
