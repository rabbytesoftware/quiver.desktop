//! The sidebar search field's rules (port of `search-bar.tsx`). The URL stays the source of truth; the
//! debounce sits in front of it (spec 2.5).

use super::resolve::{looks_like_namespace, NamespaceTarget};

pub const LOCAL_DEBOUNCE_MS: u64 = 150;

/// Where committing the draft goes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CommitTarget {
    /// An empty query has nothing to show: go Home instead of a blank results page.
    Home {
        replace: bool,
    },
    Results {
        query: String,
        replace: bool,
    },
}

pub fn commit_target(next: &str, showing_results: bool) -> CommitTarget {
    if next.is_empty() {
        CommitTarget::Home {
            replace: showing_results,
        }
    } else {
        CommitTarget::Results {
            query: next.to_owned(),
            replace: showing_results,
        }
    }
}

/// What Enter does once the lookup (if any) has answered.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SubmitAction {
    GoTo(NamespaceTarget),
    /// Fire the pass now, not after the debounce (spec 2.2). `request_submit` carries the exact draft
    /// through the store: the mounted controller is a sibling of the field. Only for a non-blank draft.
    Search {
        request_submit: Option<String>,
        then: CommitTarget,
    },
}

/// Enter is also "take me there": if what is typed names a real arrow or collection, that beats a search
/// for it. Only on Enter -- resolving on every keystroke would fire two lookups per character.
pub fn needs_namespace_lookup(draft: &str) -> bool {
    looks_like_namespace(draft)
}

pub fn submit_action(
    draft: &str,
    resolved: Option<NamespaceTarget>,
    showing_results: bool,
) -> SubmitAction {
    if let Some(target) = resolved.filter(|_| !draft.trim().is_empty()) {
        return SubmitAction::GoTo(target);
    }
    SubmitAction::Search {
        request_submit: (!draft.trim().is_empty()).then(|| draft.to_owned()),
        then: commit_target(draft, showing_results),
    }
}

/// The field's draft. Authoritative from the URL while it names a query: a back button or a deep link
/// overwrites the draft. Off the results route the URL names nothing, and syncing there would blank the
/// field on the way out; the field lives in the sidebar and outlives the route, so does the query.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SearchField {
    pub draft: String,
}

impl SearchField {
    pub fn sync_from_url(&mut self, url_query: &str, showing_results: bool) {
        if showing_results {
            self.draft = url_query.to_owned();
        }
    }

    pub fn change(&mut self, next: &str) {
        self.draft = next.to_owned();
    }

    /// The clear button of the new design's field.
    pub fn clear(&mut self) {
        self.draft.clear();
    }

    pub fn has_text(&self) -> bool {
        !self.draft.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::resolve::TargetKind;
    use super::*;

    #[test]
    fn an_empty_query_goes_home_replacing_the_results_entry() {
        assert_eq!(
            commit_target("", true),
            CommitTarget::Home { replace: true }
        );
        assert_eq!(
            commit_target("", false),
            CommitTarget::Home { replace: false }
        );
        assert_eq!(
            commit_target("rust", true),
            CommitTarget::Results {
                query: "rust".into(),
                replace: true
            }
        );
    }

    #[test]
    fn the_url_wins_only_while_it_names_a_query() {
        let mut field = SearchField {
            draft: "typed".into(),
        };
        field.sync_from_url("", false);
        assert_eq!(
            field.draft, "typed",
            "leaving the route must not blank the field"
        );
        field.sync_from_url("from-url", true);
        assert_eq!(field.draft, "from-url");
    }

    #[test]
    fn enter_on_a_resolved_namespace_navigates_there() {
        let target = NamespaceTarget {
            kind: TargetKind::Arrow,
            namespace: "github.com/a/b".into(),
        };
        assert_eq!(
            submit_action("github.com/a/b", Some(target.clone()), false),
            SubmitAction::GoTo(target)
        );
    }

    #[test]
    fn enter_otherwise_fires_the_pass_now_and_commits() {
        assert_eq!(
            submit_action("rust", None, true),
            SubmitAction::Search {
                request_submit: Some("rust".into()),
                then: CommitTarget::Results {
                    query: "rust".into(),
                    replace: true
                },
            }
        );
        assert_eq!(
            submit_action("   ", None, false),
            SubmitAction::Search {
                request_submit: None,
                then: CommitTarget::Results {
                    query: "   ".into(),
                    replace: false
                },
            }
        );
    }

    #[test]
    fn only_namespace_shaped_drafts_are_looked_up() {
        assert!(needs_namespace_lookup("github.com/a/b"));
        assert!(!needs_namespace_lookup("rust"));
    }

    #[test]
    fn the_clear_button_empties_the_field() {
        let mut field = SearchField::default();
        assert!(!field.has_text());
        field.change("x");
        assert!(field.has_text());
        field.clear();
        assert!(!field.has_text());
    }
}
