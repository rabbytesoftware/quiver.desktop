//! "Take me there": Enter on a typed namespace goes to the arrow or collection if one exists, instead of
//! searching for it. Core has no combined resolve endpoint, so the app asks both
//! `GET /v0/{arrow,collection}/:namespace`; the calls are the data layer's, the decisions are here.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetKind {
    Arrow,
    Collection,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NamespaceTarget {
    pub kind: TargetKind,
    pub namespace: String,
}

/// `domain/user/repo[/auid]`, optionally `@version` on the last segment (core's `Namespace.Validate()`
/// shape). Cheap enough to gate the two network round trips on: an ordinary one- or two-word query never
/// reaches them.
pub fn looks_like_namespace(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() || input.chars().any(char::is_whitespace) {
        return false;
    }
    input.split('/').filter(|part| !part.is_empty()).count() >= 3
        && !input.starts_with('/')
        && !input.ends_with('/')
        && !input.contains("//")
}

/// A collection is never versioned, so a namespace carrying an `@ref` can only name an arrow. Core
/// answers `/v0/collection` for one with a 500 ("internal error"), not a clean 404, so asking would fail
/// every versioned lookup outright: do not ask.
pub fn should_check_collection(namespace: &str) -> bool {
    !namespace.contains('@')
}

/// A collection wins the (rare) case where a namespace names both, since this app is collection-first.
pub fn pick_target(
    namespace: &str,
    is_collection: bool,
    is_arrow: bool,
) -> Option<NamespaceTarget> {
    let namespace = namespace.trim().to_owned();
    if is_collection {
        Some(NamespaceTarget {
            kind: TargetKind::Collection,
            namespace,
        })
    } else if is_arrow {
        Some(NamespaceTarget {
            kind: TargetKind::Arrow,
            namespace,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_namespace_shaped_input_is_worth_two_round_trips() {
        for yes in [
            "github.com/rabbyte/minecraft",
            "github.com/rabbyte/minecraft@v1",
            "github.com/a/b/auid",
            " github.com/a/b ",
        ] {
            assert!(looks_like_namespace(yes), "{yes}");
        }
        for no in [
            "",
            "minecraft",
            "rabbyte/minecraft",
            "two words",
            "github.com/a b/c",
            "/a/b/c",
            "a//b",
            "a/b/",
            "github.com/a/",
        ] {
            assert!(!looks_like_namespace(no), "{no:?}");
        }
    }

    #[test]
    fn a_versioned_namespace_is_never_asked_of_the_collection_endpoint() {
        assert!(should_check_collection("github.com/a/b"));
        assert!(!should_check_collection("github.com/a/b@v1"));
    }

    #[test]
    fn a_collection_wins_a_namespace_that_names_both() {
        let both = pick_target(" github.com/a/b ", true, true).unwrap();
        assert_eq!(
            (both.kind, both.namespace.as_str()),
            (TargetKind::Collection, "github.com/a/b")
        );
        assert_eq!(
            pick_target("x", false, true).unwrap().kind,
            TargetKind::Arrow
        );
        assert_eq!(pick_target("x", false, false), None);
    }
}
