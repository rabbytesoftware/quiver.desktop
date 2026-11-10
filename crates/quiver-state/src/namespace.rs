//! Namespace identity helpers (`bare@selector`), shared by every layer.

/// A namespace split at its first `@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceParts<'a> {
    pub head: &'a str,
    /// Empty, or the selector including its leading `@`.
    pub tail: &'a str,
}

/// Splits at the FIRST `@`, the way quiver.core's `BareNamespace()`/`Ref()`
/// do: a namespace path never carries one, while a selector may carry slashes
/// or a `*` glob, so everything after the first `@` is the selector, whole.
pub fn split_namespace(ns: &str) -> NamespaceParts<'_> {
    match ns.find('@') {
        Some(at) => NamespaceParts {
            head: &ns[..at],
            tail: &ns[at..],
        },
        None => NamespaceParts { head: ns, tail: "" },
    }
}

pub fn bare_namespace(ns: &str) -> &str {
    split_namespace(ns).head
}

/// The selector a catalog identity follows -- a channel, constraint, pinned ref or commit -- or `""` when refless.
pub fn selector_of(ns: &str) -> &str {
    split_namespace(ns).tail.get(1..).unwrap_or("")
}

pub fn with_selector(
    ns: &str,
    selector: &str,
) -> String {
    let bare = bare_namespace(ns);
    if selector.is_empty() {
        bare.to_string()
    } else {
        format!("{bare}@{selector}")
    }
}

/// `encodeURIComponent`: everything but `A-Za-z0-9 - _ . ! ~ * ' ( )` is percent-encoded.
pub fn encode_uri_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The whole namespace as ONE path segment: slashes in the path and in the selector alike are percent-encoded.
pub fn namespace_segment(ns: &str) -> String {
    encode_uri_component(ns)
}

/// `github.com/rabbyte/minecraft` -> `rabbyte`; the host is already the source.
pub fn owner_of(namespace: &str) -> &str {
    let parts: Vec<&str> = namespace.split('/').collect();
    if parts.len() > 2 {
        parts[parts.len() - 2]
    } else {
        parts[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_the_first_at() {
        assert_eq!(
            split_namespace("a/b@feat/x@y"),
            NamespaceParts {
                head: "a/b",
                tail: "@feat/x@y"
            }
        );
        assert_eq!(
            split_namespace("a/b"),
            NamespaceParts {
                head: "a/b",
                tail: ""
            }
        );
    }

    #[test]
    fn selector_and_with_selector() {
        assert_eq!(selector_of("a/b@stable"), "stable");
        assert_eq!(selector_of("a/b"), "");
        assert_eq!(with_selector("a/b@old", "new"), "a/b@new");
        assert_eq!(with_selector("a/b@old", ""), "a/b");
    }

    #[test]
    fn segment_encodes_slashes_and_at_but_keeps_glob() {
        assert_eq!(
            namespace_segment("github.com/x/y@beta"),
            "github.com%2Fx%2Fy%40beta"
        );
        assert_eq!(
            namespace_segment("github.com/x/y@v1.*"),
            "github.com%2Fx%2Fy%40v1.*"
        );
        assert_eq!(
            namespace_segment("github.com/x/y@feat/x"),
            "github.com%2Fx%2Fy%40feat%2Fx"
        );
    }

    #[test]
    fn owner() {
        assert_eq!(owner_of("github.com/rabbyte/minecraft"), "rabbyte");
        assert_eq!(owner_of("solo"), "solo");
    }
}
