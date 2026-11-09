//! Plain HTTP message types shared by the handler, the upstream trait and the
//! window IPC. Owned strings and bytes so they serialise without ceremony.

use serde::{Deserialize, Serialize};

/// Header list that keeps order and compares names case-insensitively.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers(Vec<(String, String)>);

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Appends without replacing (repeated headers stay repeated).
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.push((name.into(), value.into()));
    }

    /// Replaces every header of that name with one value.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.remove(name);
        self.0.push((name.to_ascii_lowercase(), value.into()));
    }

    pub fn remove(&mut self, name: &str) {
        self.0.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }
}

impl<N: Into<String>, V: Into<String>> FromIterator<(N, V)> for Headers {
    fn from_iter<T: IntoIterator<Item = (N, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(n, v)| (n.into(), v.into()))
                .collect(),
        )
    }
}

/// A request as the page made it (`uri` absolute) or as sent upstream (`uri`
/// is the daemon path and query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrowRequest {
    pub method: String,
    pub uri: String,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl ArrowRequest {
    pub fn get(uri: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            uri: uri.into(),
            headers: Headers::new(),
            body: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrowResponse {
    pub status: u16,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl ArrowResponse {
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: Headers::new(),
            body,
        }
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}
