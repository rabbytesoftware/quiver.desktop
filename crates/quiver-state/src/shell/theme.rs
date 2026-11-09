//! Theme preference: follow the system or pin light/dark.

pub const THEME_STORAGE_KEY: &str = "quiver.theme";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn as_stored(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Whether the UI is dark: an explicit choice wins, `System` follows the OS.
    pub fn is_dark(self, system_prefers_dark: bool) -> bool {
        match self {
            Self::Dark => true,
            Self::Light => false,
            Self::System => system_prefers_dark,
        }
    }
}

/// A stored value that is no longer valid degrades to "system".
pub fn normalise_theme_preference(value: Option<&str>) -> ThemePreference {
    match value {
        Some("light") => ThemePreference::Light,
        Some("dark") => ThemePreference::Dark,
        _ => ThemePreference::System,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_valid_preferences_round_trip_and_junk_degrades() {
        for p in [
            ThemePreference::System,
            ThemePreference::Light,
            ThemePreference::Dark,
        ] {
            assert_eq!(normalise_theme_preference(Some(p.as_stored())), p);
        }
        assert_eq!(
            normalise_theme_preference(Some("sepia")),
            ThemePreference::System
        );
        assert_eq!(normalise_theme_preference(None), ThemePreference::System);
        assert_eq!(ThemePreference::default(), ThemePreference::System);
    }

    #[test]
    fn an_explicit_choice_ignores_the_system_and_system_follows_it() {
        assert!(ThemePreference::Dark.is_dark(false));
        assert!(!ThemePreference::Light.is_dark(true));
        assert!(ThemePreference::System.is_dark(true));
        assert!(!ThemePreference::System.is_dark(false));
    }
}
