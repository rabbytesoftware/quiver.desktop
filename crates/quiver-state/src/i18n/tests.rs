use super::en::{app, arrow, home, nav, search, settings};
use super::*;
use std::collections::HashSet;

#[test]
fn answers_with_the_message_behind_the_key() {
    assert_eq!(translate_in(Locale::En, settings::TITLE, None), "Settings");
    assert_eq!(Translator::default().t(nav::HOME), "Home");
}

#[test]
fn renders_the_key_itself_when_no_message_exists() {
    let missing = Key::new("settings.nope.not.a.key");
    assert_eq!(
        translate_in(Locale::En, missing, None),
        "settings.nope.not.a.key"
    );
}

#[test]
fn fills_a_named_hole() {
    let params = Params::new().with("name", "Sample");
    assert_eq!(
        translate_in(Locale::En, arrow::ICON_FALLBACK, Some(&params)),
        "Sample icon"
    );
}

#[test]
fn leaves_a_template_with_no_parameters_alone() {
    assert_eq!(interpolate("nothing to fill", None), "nothing to fill");
}

#[test]
fn formats_numeric_values_for_the_locale() {
    let params = Params::new().with("n", 1234usize);
    assert_eq!(interpolate("{n} arrows", Some(&params)), "1,234 arrows");
}

#[test]
fn does_not_rescan_what_it_just_substituted() {
    let params = Params::new().with("name", "{name} {count}");
    assert_eq!(
        interpolate("{name} is here", Some(&params)),
        "{name} {count} is here"
    );
}

#[test]
fn leaves_an_unfilled_hole_standing_so_the_gap_is_visible() {
    let params = Params::new().with("a", "this");
    assert_eq!(interpolate("{a} and {b}", Some(&params)), "this and {b}");
}

#[test]
fn ignores_braces_that_are_not_a_placeholder() {
    let params = Params::new().with("id", "x");
    assert_eq!(
        interpolate("use { \"id\": 1 } here", Some(&params)),
        "use { \"id\": 1 } here"
    );
}

#[test]
fn picks_the_english_plural_form() {
    let taps = Message::Plural {
        one: "{count} more tap…",
        other: "{count} more taps…",
    };
    let en = |n| render_message(Locale::En, taps, Some(&Params::count(n)));
    assert_eq!(en(1), "1 more tap…");
    assert_eq!(en(3), "3 more taps…");
    assert_eq!(en(0), "0 more taps…");
}

#[test]
fn a_plural_reached_without_a_count_is_zero() {
    let taps = Message::Plural {
        one: "{count} tap",
        other: "{count} taps",
    };
    assert_eq!(render_message(Locale::En, taps, None), "0 taps");
}

#[test]
fn real_plural_keys_pick_the_right_form() {
    let t = Translator::default();
    assert_eq!(t.t_count(search::RESULTS_COUNT, 1), "1 result");
    assert_eq!(t.t_count(search::RESULTS_COUNT, 1234), "1,234 results");
    assert_eq!(t.t_count(home::VIEW_ALL_RECOMMENDED, 1), "View 1 arrow");
}

#[test]
fn every_key_is_unique_and_dotted_and_plurals_carry_their_count() {
    let mut seen = HashSet::new();
    for (key, message) in en::all() {
        assert!(seen.insert(key.id()), "duplicate key {}", key.id());
        assert!(key.id().contains('.'), "{}", key.id());
        if let Message::Plural { one, other } = message {
            for form in [one, other] {
                assert!(form.contains("{count}"), "{} lacks {{count}}", key.id());
            }
        }
    }
    assert!(seen.len() >= 401);
}

#[test]
fn constants_resolve_through_the_catalogue() {
    assert!(en::lookup(app::SETTINGS).is_some() || en::lookup(settings::TITLE).is_some());
    assert!(en::all().all(|(key, _)| en::lookup(*key).is_some()));
}

#[test]
fn preference_degrades_unknown_values_to_system() {
    assert_eq!(
        normalise_preference(Some("system")),
        LocalePreference::System
    );
    assert_eq!(
        normalise_preference(Some("en")),
        LocalePreference::Pinned(Locale::En)
    );
    assert_eq!(normalise_preference(Some("de")), LocalePreference::System);
    assert_eq!(normalise_preference(None), LocalePreference::System);
    assert_eq!(LocalePreference::Pinned(Locale::En).as_stored(), "en");
}

#[test]
fn a_forced_locale_beats_the_preference() {
    let pref = LocalePreference::System;
    assert_eq!(pref.resolve(Locale::En, Some("en")), Locale::En);
    assert_eq!(pref.resolve(Locale::En, Some("zz")), Locale::En);
}
