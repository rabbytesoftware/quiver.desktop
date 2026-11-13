use crate::i18n::{en::nav, Key};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    Home,
    Remote,
    Settings,
}

impl Destination {
    pub fn path(self) -> &'static str {
        match self {
            Self::Home => "/",
            Self::Remote => "/remote",
            Self::Settings => "/settings",
        }
    }

    pub fn label(self) -> Key {
        match self {
            Self::Home => nav::HOME,
            Self::Remote => nav::REMOTE,
            Self::Settings => nav::SETTINGS,
        }
    }
}

pub const DESTINATIONS: [Destination; 3] = [
    Destination::Home,
    Destination::Remote,
    Destination::Settings,
];

/// Pages that hang off Home keep it lit (the sidebar keeps Home highlighted on them).
const HOME_CHILDREN: [&str; 3] = ["/library", "/collections", "/recommended"];

fn under(
    pathname: &str,
    base: &str,
) -> bool {
    pathname == base
        || pathname
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/'))
}

pub fn active_destination(pathname: &str) -> Option<Destination> {
    if pathname == "/" || HOME_CHILDREN.iter().any(|base| under(pathname, base)) {
        return Some(Destination::Home);
    }
    DESTINATIONS
        .into_iter()
        .find(|d| d.path() != "/" && under(pathname, d.path()))
}

/// Clicking the row that is already active must not push a duplicate history entry.
pub fn blocks_reselect(is_active: bool) -> bool {
    is_active
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_and_each_destination_light_their_own_segment() {
        assert_eq!(active_destination("/"), Some(Destination::Home));
        assert_eq!(active_destination("/remote"), Some(Destination::Remote));
        assert_eq!(active_destination("/settings"), Some(Destination::Settings));
        assert_eq!(
            active_destination("/settings/engine"),
            Some(Destination::Settings)
        );
    }

    #[test]
    fn pages_that_hang_off_home_keep_home_lit() {
        for path in ["/library", "/collections", "/recommended", "/library/x"] {
            assert_eq!(active_destination(path), Some(Destination::Home), "{path}");
        }
    }

    #[test]
    fn a_shared_prefix_is_not_a_match_and_other_routes_light_nothing() {
        assert_eq!(active_destination("/settingsx"), None);
        assert_eq!(active_destination("/search"), None);
        assert_eq!(active_destination("/arrow/github.com/a/b"), None);
    }

    #[test]
    fn reselecting_the_active_row_is_blocked() {
        assert!(blocks_reselect(true));
        assert!(!blocks_reselect(false));
    }
}
