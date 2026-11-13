//! Message and key types. A [`Key`] can only be made inside this crate (the catalogue constants),
//! so a key that is used always exists: there is no way to spell one wrongly.

use std::collections::HashMap;

/// A catalogue key such as `settings.title`. Constants live next to their message in `en/`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key(&'static str);

impl Key {
    pub(crate) const fn new(id: &'static str) -> Self {
        Self(id)
    }

    /// The dotted id, e.g. `nav.home`.
    pub const fn id(self) -> &'static str {
        self.0
    }
}

/// A catalogue message: plain text or a plural pair. Only `one`/`other` exist because the
/// source locale is English; a locale with more categories extends this enum.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Message {
    Text(&'static str),
    Plural {
        one: &'static str,
        other: &'static str,
    },
}

/// Values for `{name}` holes. Numbers are formatted for the locale, text is inserted as is.
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Text(String),
    Number(f64),
}

impl From<&str> for Param {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for Param {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<f64> for Param {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<usize> for Param {
    fn from(value: usize) -> Self {
        Self::Number(value as f64)
    }
}

impl From<u32> for Param {
    fn from(value: u32) -> Self {
        Self::Number(f64::from(value))
    }
}

/// Named parameters of one translation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params(HashMap<&'static str, Param>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one parameter (builder style).
    pub fn with(
        mut self,
        name: &'static str,
        value: impl Into<Param>,
    ) -> Self {
        self.0.insert(name, value.into());
        self
    }

    /// `{count}` only: the common case of a plural message.
    pub fn count(n: usize) -> Self {
        Self::new().with("count", n)
    }

    pub(crate) fn get(
        &self,
        name: &str,
    ) -> Option<&Param> {
        self.0.get(name)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
