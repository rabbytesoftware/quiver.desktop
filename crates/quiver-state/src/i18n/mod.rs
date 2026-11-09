//! Translation: typed keys, `{name}` interpolation and plural selection (port of `src/lib/i18n`).
//!
//! `t(key)` / `t_with(key, params)` resolve against the locale in force; a key missing from a
//! non-source locale falls back to the source locale (English).

pub mod detect;
pub mod en;
pub mod format;
mod message;
mod preference;

pub use detect::{detect_locale, locale_forced, match_locale};
pub use message::{Key, Message, Param, Params};
pub use preference::{normalise_preference, LocalePreference};

/// A shipped locale.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Locale {
    En,
}

pub const SOURCE_LOCALE: Locale = Locale::En;
pub const LOCALES: [Locale; 1] = [Locale::En];

impl Locale {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        LOCALES.iter().copied().find(|l| l.tag() == tag)
    }

    fn lookup(self, key: Key) -> Option<Message> {
        match self {
            Self::En => en::lookup(key),
        }
    }

    /// CLDR plural category for English: `one` only for exactly 1.
    fn is_singular(self, count: f64) -> bool {
        match self {
            Self::En => (count - 1.0).abs() < f64::EPSILON,
        }
    }
}

/// Fills `{name}` holes. A hole with no parameter stays visible; substituted text is not rescanned;
/// numbers are formatted for the locale.
pub fn interpolate(template: &str, params: Option<&Params>) -> String {
    let Some(params) = params.filter(|p| !p.is_empty()) else {
        return template.to_owned();
    };
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let name_len = after
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let is_hole = name_len > 0 && after[name_len..].starts_with('}');
        match (is_hole, params.get(&after[..name_len])) {
            (true, Some(Param::Text(text))) => out.push_str(text),
            (true, Some(Param::Number(n))) => out.push_str(&format::format_number(*n)),
            (true, None) => out.push_str(&rest[open..open + name_len + 2]),
            (false, _) => {
                out.push('{');
                rest = after;
                continue;
            }
        }
        rest = &after[name_len + 1..];
    }
    out.push_str(rest);
    out
}

/// Chooses the plural form for `count` (English: `one` only for exactly 1).
fn select_plural(
    locale: Locale,
    one: &'static str,
    other: &'static str,
    count: f64,
) -> &'static str {
    if locale.is_singular(count) {
        one
    } else {
        other
    }
}

/// Renders a message. A plural message reached without a numeric `count` is treated as zero.
pub fn render_message(locale: Locale, message: Message, params: Option<&Params>) -> String {
    match message {
        Message::Text(text) => interpolate(text, params),
        Message::Plural { one, other } => {
            let count = match params.and_then(|p| p.get("count")) {
                Some(Param::Number(n)) => *n,
                _ => 0.0,
            };
            let form = select_plural(locale, one, other, count);
            let with_count = params.cloned().unwrap_or_default().with("count", count);
            interpolate(form, Some(&with_count))
        }
    }
}

/// Translates `key` in `locale`; the key id itself when no catalogue has it.
pub fn translate_in(locale: Locale, key: Key, params: Option<&Params>) -> String {
    match locale.lookup(key).or_else(|| SOURCE_LOCALE.lookup(key)) {
        Some(message) => render_message(locale, message, params),
        None => key.id().to_owned(),
    }
}

/// A translator bound to one locale.
#[derive(Clone, Copy, Debug)]
pub struct Translator {
    locale: Locale,
}

impl Translator {
    pub const fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub const fn locale(&self) -> Locale {
        self.locale
    }

    pub fn t(&self, key: Key) -> String {
        translate_in(self.locale, key, None)
    }

    pub fn t_with(&self, key: Key, params: &Params) -> String {
        translate_in(self.locale, key, Some(params))
    }

    /// Plural message keyed on `{count}` only.
    pub fn t_count(&self, key: Key, count: usize) -> String {
        self.t_with(key, &Params::count(count))
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new(SOURCE_LOCALE)
    }
}

#[cfg(test)]
mod tests;
