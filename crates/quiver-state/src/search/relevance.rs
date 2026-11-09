//! Whether an entry is what the query names.

use crate::domain::SearchEntry;

fn is_separator(c: char) -> bool {
    matches!(c, '.' | '-' | '_' | '/') || c.is_whitespace()
}

fn squash(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| !is_separator(*c))
        .collect()
}

fn tokens(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(is_separator)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether every word of the query appears in what names the arrow. Separators are ignored on both
/// sides, so `quiver.chat`, `quiver-chat` and `quiver chat` all find `quiver.chat`.
///
/// Descriptions are left out on purpose: core's text index matches them, and that is how a followed
/// collection answered "Quiver.chat" with every arrow whose blurb mentions Quiver.
pub fn matches_query(entry: &SearchEntry, query: &str) -> bool {
    let words = tokens(query);
    if words.is_empty() {
        return true;
    }
    let mut named = vec![entry.namespace.as_str(), entry.name.as_str()];
    named.extend(entry.tags.iter().map(String::as_str));
    let haystack = squash(&named.join(" "));
    words.iter().all(|word| haystack.contains(word.as_str()))
}

/// Narrows the local lane to what the query names. Core matches that lane loosely (its text index reads
/// descriptions, and a followed collection would otherwise fill the screen with arrows nobody asked
/// for), so it is filtered here. Only the local lane: a streamed entry is a network result, ranked by
/// core's discovery and kept whole.
pub fn relevant_local(local: &[SearchEntry], query: &str) -> Vec<SearchEntry> {
    local
        .iter()
        .filter(|e| matches_query(e, query))
        .cloned()
        .collect()
}

#[cfg(test)]
pub(crate) mod fixtures {
    use crate::domain::{SearchEntry, SearchProvenance};

    pub fn entry(namespace: &str, name: &str) -> SearchEntry {
        SearchEntry {
            namespace: namespace.into(),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            versions: vec![],
            compatible_os: vec![],
            provenance: Some(SearchProvenance::Collection),
            installed: false,
            known: true,
            stars: 0,
            source: None,
            origin: None,
            confidence: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::entry;
    use super::*;

    fn chat() -> SearchEntry {
        entry("github.com/rabbytesoftware/quiver.chat", "Quiver Chat")
    }

    #[test]
    fn finds_quiver_chat_however_the_separators_are_written() {
        for q in [
            "quiver.chat",
            "quiver-chat",
            "quiver_chat",
            "Quiver Chat",
            "quiverchat",
            "quiver/chat",
            " quiver . chat ",
        ] {
            assert!(matches_query(&chat(), q), "{q}");
        }
    }

    #[test]
    fn ignores_separators_on_the_entry_side_too() {
        for (name, q) in [
            ("quiver_chat", "quiver.chat"),
            ("quiver-chat", "quiver_chat"),
            ("Quiver Chat", "quiver-chat"),
        ] {
            assert!(
                matches_query(&entry("github.com/x/other", name), q),
                "{name} / {q}"
            );
        }
    }

    #[test]
    fn does_not_match_on_the_description() {
        let mut other = entry("github.com/rabbytesoftware/quiver.essentials", "Essentials");
        other.description = "Works with Quiver chat too".into();
        assert!(!matches_query(&other, "quiver.chat"));
    }

    #[test]
    fn a_blank_query_matches_everything_and_tags_count() {
        let mut other = entry("github.com/x/y", "Y");
        assert!(matches_query(&other, "  "));
        other.tags = vec!["database".into()];
        assert!(matches_query(&other, "database"));
    }

    #[test]
    fn keeps_only_the_local_entries_the_query_names() {
        let other = entry("github.com/rabbytesoftware/quiver.essentials", "Essentials");
        assert_eq!(relevant_local(&[chat(), other], "quiver.chat"), [chat()]);
    }
}
