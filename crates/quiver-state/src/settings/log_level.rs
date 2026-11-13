//! The four levels the engine tab offers.

pub const LEVELS: [Level; 4] = [Level::Debug, Level::Info, Level::Warn, Level::Error];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Core accepts a wider vocabulary than this tab writes, and a hand-edited config (or an older core)
/// can hold any of it. With no matching option the trigger would render the raw token with nothing
/// selected, so every alias maps onto the closest canonical level, purely for display.
pub fn normalise_level(raw: &str) -> Level {
    match raw {
        "debug" | "trace" => Level::Debug,
        "info" => Level::Info,
        "warn" | "warning" => Level::Warn,
        "error" | "fatal" | "panic" => Level::Error,
        _ => Level::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_levels_are_untouched() {
        for level in LEVELS {
            assert_eq!(normalise_level(level.as_str()), level);
        }
    }

    #[test]
    fn aliases_map_onto_the_closest_level() {
        assert_eq!(normalise_level("trace"), Level::Debug);
        assert_eq!(normalise_level("warning"), Level::Warn);
        assert_eq!(normalise_level("fatal"), Level::Error);
        assert_eq!(normalise_level("panic"), Level::Error);
    }

    #[test]
    fn anything_else_is_info_rather_than_an_error() {
        assert_eq!(normalise_level("silent"), Level::Info);
        assert_eq!(normalise_level(""), Level::Info);
    }
}
