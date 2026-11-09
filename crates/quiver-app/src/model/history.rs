use super::route::Route;

/// Back/forward history. Navigating to the page you are on adds nothing (the sidebar rule
/// `blocks_reselect`); navigating from the middle drops the forward entries, like a browser.
#[derive(Debug)]
pub struct History {
    entries: Vec<Route>,
    index: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: vec![Route::Home],
            index: 0,
        }
    }
}

impl History {
    pub fn current(&self) -> &Route {
        &self.entries[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// Returns whether the current route changed.
    pub fn push(&mut self, route: Route) -> bool {
        if *self.current() == route {
            return false;
        }
        self.entries.truncate(self.index + 1);
        self.entries.push(route);
        self.index += 1;
        true
    }

    /// Replaces the current entry (typing in search must not pile up history).
    pub fn replace(&mut self, route: Route) -> bool {
        if *self.current() == route {
            return false;
        }
        self.entries[self.index] = route;
        true
    }

    pub fn back(&mut self) -> bool {
        let moved = self.can_go_back();
        if moved {
            self.index -= 1;
        }
        moved
    }

    pub fn forward(&mut self) -> bool {
        let moved = self.can_go_forward();
        if moved {
            self.index += 1;
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_dedupes_and_truncates_the_forward_entries() {
        let mut h = History::default();
        assert!(!h.push(Route::Home));
        assert!(h.push(Route::Library));
        assert!(h.push(Route::Remote));
        assert!(h.back());
        assert_eq!(h.current(), &Route::Library);
        assert!(h.push(Route::Collections));
        assert!(!h.can_go_forward());
        assert!(h.back() && h.back());
        assert!(!h.back());
        assert_eq!(h.current(), &Route::Home);
    }

    #[test]
    fn replace_keeps_the_depth() {
        let mut h = History::default();
        h.push(Route::Search { query: "a".into() });
        assert!(h.replace(Route::Search { query: "ab".into() }));
        assert!(h.back());
        assert_eq!(h.current(), &Route::Home);
    }
}
