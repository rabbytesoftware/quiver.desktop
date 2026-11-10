//! Catalog identities: `<namespace>@<selector>` (port of `src/lib/namespace.ts`).

/// The namespace up to the first `@`, the way quiver.core's `BareNamespace()`
/// does: a namespace path never carries one, while a selector may carry
/// slashes or a `*` glob.
pub(super) fn bare_namespace(ns: &str) -> &str {
    ns.split_once('@').map_or(ns, |(head, _)| head)
}

pub(super) fn with_selector(
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

/// The whole namespace as ONE path segment: slashes in the path and in the
/// selector alike are percent-encoded (JavaScript's `encodeURIComponent`).
pub(super) fn namespace_segment(ns: &str) -> String {
    let mut out = String::with_capacity(ns.len());
    for byte in ns.bytes() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selector_is_everything_after_the_first_at() {
        assert_eq!(with_selector("a/b@old/x@y", "stable"), "a/b@stable");
        assert_eq!(with_selector("a/b", ""), "a/b");
    }

    #[test]
    fn the_segment_encodes_like_encode_uri_component() {
        assert_eq!(
            namespace_segment("github.com/rabbytesoftware/quiver.desktop@stable"),
            "github.com%2Frabbytesoftware%2Fquiver.desktop%40stable"
        );
        assert_eq!(namespace_segment("a b*c'(d)~!"), "a%20b*c'(d)~!");
    }
}
