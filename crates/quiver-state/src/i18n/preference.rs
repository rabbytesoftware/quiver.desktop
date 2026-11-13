//! The saved language choice: follow the system, or pin a shipped locale.

use super::{detect, Locale, LOCALES};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LocalePreference {
    #[default]
    System,
    Pinned(Locale),
}

impl LocalePreference {
    /// The value to persist: `"system"` or a locale tag.
    pub fn as_stored(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Pinned(locale) => locale.tag(),
        }
    }

    /// The locale in force. A forced locale (env override) beats everything; `detected` is
    /// what the system asked for.
    pub fn resolve(
        self,
        detected: Locale,
        forced: Option<&str>,
    ) -> Locale {
        if let Some(locale) = detect::locale_forced(forced) {
            return locale;
        }
        match self {
            Self::System => detected,
            Self::Pinned(locale) => locale,
        }
    }
}

/// Reads a stored value back; anything that is not "system" or a shipped locale degrades to "system"
/// (a saved language that no longer ships must not break the app).
pub fn normalise_preference(value: Option<&str>) -> LocalePreference {
    match value {
        Some("system") | None => LocalePreference::System,
        Some(tag) => LOCALES
            .iter()
            .copied()
            .find(|l| l.tag() == tag)
            .map_or(LocalePreference::System, LocalePreference::Pinned),
    }
}
