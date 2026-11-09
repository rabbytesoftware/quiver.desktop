//! Query keys. An invalidation names a prefix and every key under it is stale.

pub type QueryKey = Vec<String>;

pub const ARROW_DETAIL_PREFIX: &str = "arrow";
pub const COLLECTION_PREFIX: &str = "collection";
pub const HOME_KEY: &str = "home";

fn key(parts: &[&str]) -> QueryKey {
    parts.iter().map(|p| (*p).to_string()).collect()
}

pub fn arrow_detail_prefix() -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX])
}

pub fn arrow_detail_key(namespace: &str) -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX, namespace])
}

pub fn arrow_channels_key(namespace: &str) -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX, namespace, "channels"])
}

pub fn arrow_readme_key(namespace: &str) -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX, namespace, "readme"])
}

pub fn arrow_dependencies_key(namespace: &str) -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX, namespace, "dependencies"])
}

pub fn arrow_dependents_key(namespace: &str) -> QueryKey {
    key(&[ARROW_DETAIL_PREFIX, namespace, "dependents"])
}

pub fn collection_prefix() -> QueryKey {
    key(&[COLLECTION_PREFIX])
}

pub fn collection_key(namespace: &str) -> QueryKey {
    key(&[COLLECTION_PREFIX, namespace])
}

pub fn followed_collections_key() -> QueryKey {
    key(&["collections", "followed"])
}

pub fn home_key() -> QueryKey {
    key(&[HOME_KEY])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_stable_key_from_the_namespace_for_each_of_the_five_arrow_queries() {
        let ns = "github.com/a/b@stable";
        assert_eq!(arrow_detail_key(ns), vec!["arrow", ns]);
        assert_eq!(arrow_channels_key(ns), vec!["arrow", ns, "channels"]);
        assert_eq!(arrow_readme_key(ns), vec!["arrow", ns, "readme"]);
        assert_eq!(arrow_dependencies_key(ns), vec!["arrow", ns, "dependencies"]);
        assert_eq!(arrow_dependents_key(ns), vec!["arrow", ns, "dependents"]);
        assert!(arrow_channels_key(ns).starts_with(&arrow_detail_prefix()));
        assert!(collection_key("c").starts_with(&collection_prefix()));
    }
}
