//! Whether Quiver's own components (quiver.desktop, quiver.core) are listed in the catalog.

pub const CATALOG_VISIBILITY_STORAGE_KEY: &str = "quiver.catalogVisibility";

/// Opt-out, not opt-in: anything that is not a real boolean reads as `true` (show), the pre-existing behaviour.
pub fn normalise_show_self_components(stored: Option<bool>) -> bool {
    stored.unwrap_or(true)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CatalogVisibility {
    pub show_self_components: bool,
}

impl Default for CatalogVisibility {
    fn default() -> Self {
        Self {
            show_self_components: true,
        }
    }
}

impl CatalogVisibility {
    pub fn set(&mut self, show: bool) {
        self.show_self_components = show;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_the_self_components_by_default_and_toggles() {
        let mut v = CatalogVisibility::default();
        assert!(v.show_self_components);
        v.set(false);
        assert!(!v.show_self_components);
        v.set(true);
        assert!(v.show_self_components);
    }

    #[test]
    fn a_missing_or_malformed_stored_value_defaults_to_show() {
        assert!(normalise_show_self_components(None));
        assert!(normalise_show_self_components(Some(true)));
        assert!(!normalise_show_self_components(Some(false)));
    }
}
