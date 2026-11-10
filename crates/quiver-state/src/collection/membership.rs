//! Which members of a collection the user already holds: the "Installed" mark on a member tile and the
//! "all installed" line of a collection.

use crate::domain::{collection_arrow_route, CollectionArrow};
use crate::namespace::bare_namespace;

/// A member counts as held when the library has its exact route, or its package at any version (a
/// library row is keyed by what was installed; the collection may pin another selector of the same
/// repository, which is still the arrow the user knows).
pub fn is_member_installed(member: &CollectionArrow, library: &[String]) -> bool {
    let route = collection_arrow_route(member);
    library
        .iter()
        .any(|ns| *ns == route || bare_namespace(ns) == bare_namespace(&member.namespace))
}

/// Resolved members in the library; an unresolved one cannot be held.
pub fn installed_members(arrows: &[CollectionArrow], library: &[String]) -> usize {
    arrows
        .iter()
        .filter(|a| a.resolved && is_member_installed(a, library))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(ns: &str, version: Option<&str>, resolved: bool) -> CollectionArrow {
        CollectionArrow {
            namespace: ns.into(),
            version: version.map(Into::into),
            resolved,
            name: None,
            description: None,
            icon: None,
            banner: None,
        }
    }

    #[test]
    fn a_member_is_held_by_route_or_by_package() {
        let library = vec!["github.com/x/a@v1".to_owned(), "github.com/x/b".to_owned()];
        assert!(is_member_installed(
            &member("github.com/x/a", Some("v1"), true),
            &library
        ));
        assert!(is_member_installed(
            &member("github.com/x/a", Some("v2"), true),
            &library
        ));
        assert!(is_member_installed(
            &member("github.com/x/b", None, true),
            &library
        ));
        assert!(!is_member_installed(
            &member("github.com/x/c", None, true),
            &library
        ));
    }

    #[test]
    fn unresolved_members_are_never_counted() {
        let library = vec!["github.com/x/a".to_owned()];
        let arrows = [
            member("github.com/x/a", None, true),
            member("github.com/x/a", Some("v9"), false),
            member("github.com/x/z", None, true),
        ];
        assert_eq!(installed_members(&arrows, &library), 1);
    }
}
