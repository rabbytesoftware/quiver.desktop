//! Picking the locale: match what the system asks for against what ships.

use super::{Locale, LOCALES, SOURCE_LOCALE};

/// Exact tag match first (for every requested tag, in order), then by language subtag, else `fallback`.
pub fn match_locale<'a>(
    requested: &[&str],
    supported: &[&'a str],
    fallback: &'a str,
) -> &'a str {
    for want in requested {
        if let Some(exact) = supported.iter().find(|tag| tag.eq_ignore_ascii_case(want)) {
            return exact;
        }
    }
    for want in requested {
        let wanted = language(want);
        if let Some(found) = supported
            .iter()
            .find(|tag| language(tag).eq_ignore_ascii_case(wanted))
        {
            return found;
        }
    }
    fallback
}

fn language(tag: &str) -> &str {
    tag.split('-').next().unwrap_or(tag)
}

/// The shipped locale best matching the system's ordered language list.
pub fn detect_locale(system: &[&str]) -> Locale {
    let tags: Vec<&str> = LOCALES.iter().map(|l| l.tag()).collect();
    let tag = match_locale(system, &tags, SOURCE_LOCALE.tag());
    Locale::from_tag(tag).unwrap_or(SOURCE_LOCALE)
}

/// `QUIVER_LOCALE`-style override: the shipped locale whose tag matches, if any.
pub fn locale_forced(requested: Option<&str>) -> Option<Locale> {
    let requested = requested.filter(|r| !r.is_empty())?;
    LOCALES
        .iter()
        .copied()
        .find(|l| l.tag().eq_ignore_ascii_case(requested))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTED: [&str; 3] = ["en", "de", "pt-BR"];

    #[test]
    fn an_exact_tag_wins_over_a_language_match() {
        assert_eq!(match_locale(&["pt-BR"], &SUPPORTED, "en"), "pt-BR");
        assert_eq!(match_locale(&["PT-br"], &SUPPORTED, "en"), "pt-BR");
    }

    #[test]
    fn a_later_exact_match_beats_an_earlier_language_match() {
        assert_eq!(match_locale(&["de-AT", "en"], &SUPPORTED, "en"), "en");
    }

    #[test]
    fn falls_back_to_the_language_then_to_the_fallback() {
        assert_eq!(match_locale(&["de-AT"], &SUPPORTED, "en"), "de");
        assert_eq!(match_locale(&["ja"], &SUPPORTED, "en"), "en");
        assert_eq!(match_locale(&[], &SUPPORTED, "en"), "en");
    }

    #[test]
    fn detect_and_force_only_ever_return_shipped_locales() {
        assert_eq!(detect_locale(&["fr-FR", "en-GB"]), Locale::En);
        assert_eq!(locale_forced(Some("EN")), Some(Locale::En));
        assert_eq!(locale_forced(Some("xx")), None);
        assert_eq!(locale_forced(Some("")), None);
        assert_eq!(locale_forced(None), None);
    }
}
