//! Which arrow apps are kept alive, in which order, and what Reload and Stop
//! do to them. Pure data: the UI calls these and acts on what they return.
//! Ports `src/features/arrow-app/store/open-apps.ts`.

use std::collections::{HashMap, HashSet};

/// Apps kept alive at once; the least recently used one beyond this goes.
pub const MAX_OPEN_APPS: usize = 4;

/// Marks `ns` most recently used and evicts down to `cap`, sparing `visible`.
pub fn admit(order: &[String], ns: &str, visible: Option<&str>, cap: usize) -> Vec<String> {
    let mut next: Vec<String> = order.iter().filter(|n| *n != ns).cloned().collect();
    next.push(ns.to_string());
    while next.len() > cap {
        let victim = next
            .iter()
            .position(|n| Some(n.as_str()) != visible && n != ns);
        match victim {
            Some(i) => {
                next.remove(i);
            }
            None => break,
        }
    }
    next
}

/// What `show` did, so the window layer knows which windows to create or close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// The app had no frame and needs a fresh one.
    pub opened: bool,
    /// Apps dropped by the cap; close their windows and sockets.
    pub evicted: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArrowAppRegistry {
    cap: usize,
    /// Least recently used first. Decides eviction only.
    order: Vec<String>,
    /// In the order opened, which is the order frames render in. It never
    /// follows `order`: moving an iframe in the DOM reloads it.
    frames: Vec<String>,
    visible: Option<String>,
    /// Bumped to remount an app's frame.
    reloads: HashMap<String, u64>,
}

impl Default for ArrowAppRegistry {
    fn default() -> Self {
        Self::with_cap(MAX_OPEN_APPS)
    }
}

impl ArrowAppRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cap(cap: usize) -> Self {
        Self {
            cap,
            order: Vec::new(),
            frames: Vec::new(),
            visible: None,
            reloads: HashMap::new(),
        }
    }

    /// Makes `ns` the visible app, keeping it open and evicting the least
    /// recently used hidden one beyond the cap.
    pub fn show(&mut self, ns: &str) -> Shown {
        let order = admit(&self.order, ns, Some(ns), self.cap);
        let kept: HashSet<&String> = order.iter().collect();
        let evicted: Vec<String> = self
            .order
            .iter()
            .filter(|n| !kept.contains(n))
            .cloned()
            .collect();
        let opened = !self.frames.iter().any(|n| n == ns);
        self.frames.retain(|n| kept.contains(n));
        if opened {
            self.frames.push(ns.to_string());
        }
        self.order = order;
        self.visible = Some(ns.to_string());
        Shown { opened, evicted }
    }

    /// Hides the visible app; it stays alive.
    pub fn hide(&mut self) {
        self.visible = None;
    }

    /// Reload: bumps the key that remounts `ns`'s frame (sockets die with the
    /// old document) and returns the new key.
    pub fn reload(&mut self, ns: &str) -> u64 {
        let key = self.reloads.entry(ns.to_string()).or_insert(0);
        *key += 1;
        *key
    }

    /// Stop (or any other way a surface disappears): drops every app not in
    /// `live`, returning the dropped ones so their windows can be closed. Like
    /// the original, nothing else changes when no open app was dropped.
    pub fn prune(&mut self, live: &HashSet<String>) -> Vec<String> {
        let dropped: Vec<String> = self
            .order
            .iter()
            .filter(|n| !live.contains(*n))
            .cloned()
            .collect();
        if dropped.is_empty() {
            return dropped;
        }
        self.order.retain(|n| live.contains(n));
        self.frames.retain(|n| live.contains(n));
        self.reloads.retain(|n, _| live.contains(n));
        dropped
    }

    /// Stop of one app: it leaves the stack, its frame and its reload key, and
    /// stops being the visible one. Returns whether it was open.
    pub fn stop(&mut self, ns: &str) -> bool {
        let was_open = self.order.iter().any(|n| n == ns);
        self.order.retain(|n| n != ns);
        self.frames.retain(|n| n != ns);
        self.reloads.remove(ns);
        if self.visible.as_deref() == Some(ns) {
            self.visible = None;
        }
        was_open
    }

    /// Open apps, least recently used first.
    pub fn order(&self) -> &[String] {
        &self.order
    }

    /// Open apps in render order.
    pub fn frames(&self) -> &[String] {
        &self.frames
    }

    pub fn visible(&self) -> Option<&str> {
        self.visible.as_deref()
    }

    /// The remount key of `ns` (0 until its first Reload).
    pub fn reload_key(&self, ns: &str) -> u64 {
        self.reloads.get(ns).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn live(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn admit_moves_an_opened_app_to_the_most_recent_end_without_duplicating() {
        assert_eq!(
            admit(&v(&["a", "b", "c"]), "a", None, 4),
            v(&["b", "c", "a"])
        );
        assert_eq!(admit(&v(&["a"]), "b", None, 4), v(&["a", "b"]));
    }

    #[test]
    fn admit_evicts_the_least_recently_used_app_never_the_visible_one() {
        let full = v(&["a", "b", "c", "d"]);
        assert_eq!(MAX_OPEN_APPS, 4);
        assert_eq!(admit(&full, "e", None, 4), v(&["b", "c", "d", "e"]));
        assert_eq!(admit(&full, "e", Some("a"), 4), v(&["a", "c", "d", "e"]));
    }

    #[test]
    fn admit_honours_a_custom_cap() {
        assert_eq!(admit(&v(&["a", "b"]), "c", None, 2), v(&["b", "c"]));
    }

    #[test]
    fn admit_stops_evicting_when_nothing_is_evictable() {
        assert_eq!(admit(&v(&["a"]), "b", Some("a"), 0), v(&["a", "b"]));
    }

    #[test]
    fn show_makes_an_app_visible_and_keeps_it_open_hide_keeps_it_alive() {
        let mut r = ArrowAppRegistry::new();
        assert_eq!(
            r.show("a/b"),
            Shown {
                opened: true,
                evicted: vec![]
            }
        );
        assert_eq!((r.order(), r.visible()), (&v(&["a/b"])[..], Some("a/b")));
        r.hide();
        assert_eq!((r.order(), r.visible()), (&v(&["a/b"])[..], None));
    }

    #[test]
    fn switching_keeps_every_app_in_the_stack() {
        let mut r = ArrowAppRegistry::new();
        r.show("a/b");
        let again = r.show("c/d");
        assert!(again.opened);
        assert_eq!(r.order(), &v(&["a/b", "c/d"])[..]);
        assert_eq!(r.visible(), Some("c/d"));
    }

    #[test]
    fn prune_removes_apps_whose_surface_is_gone() {
        let mut r = ArrowAppRegistry::new();
        r.show("a/b");
        r.show("c/d");
        assert_eq!(r.prune(&live(&["c/d"])), v(&["a/b"]));
        assert_eq!(r.order(), &v(&["c/d"])[..]);
        assert_eq!(r.frames(), &v(&["c/d"])[..]);
    }

    #[test]
    fn showing_an_app_again_reorders_the_lru_but_never_the_frames() {
        let mut r = ArrowAppRegistry::new();
        for ns in ["a", "b", "c", "a", "b"] {
            r.show(ns);
        }
        assert_eq!(r.order(), &v(&["c", "a", "b"])[..]);
        assert_eq!(r.frames(), &v(&["a", "b", "c"])[..]);
        assert!(!r.show("a").opened);
    }

    #[test]
    fn the_fifth_app_evicts_the_lru_hidden_one_and_keeps_other_frames_in_place() {
        let mut r = ArrowAppRegistry::new();
        for ns in ["a", "b", "c", "d"] {
            r.show(ns);
        }
        r.show("a");
        let shown = r.show("e");
        assert_eq!(shown.evicted, v(&["b"]));
        assert_eq!(r.order(), &v(&["c", "d", "a", "e"])[..]);
        assert_eq!(r.frames(), &v(&["a", "c", "d", "e"])[..]);
    }

    #[test]
    fn an_evicted_app_reopens_as_a_fresh_frame_at_the_end() {
        let mut r = ArrowAppRegistry::new();
        for ns in ["a", "b", "c", "d", "e"] {
            r.show(ns);
        }
        assert_eq!(r.frames(), &v(&["b", "c", "d", "e"])[..]);
        let shown = r.show("a");
        assert!(shown.opened);
        assert_eq!(shown.evicted, v(&["b"]));
        assert_eq!(r.frames(), &v(&["c", "d", "e", "a"])[..]);
    }

    #[test]
    fn prune_leaves_the_registry_untouched_when_nothing_changed() {
        let mut r = ArrowAppRegistry::new();
        r.show("a/b");
        r.reload("zzz");
        assert!(r.prune(&live(&["a/b"])).is_empty());
        assert_eq!(r.reload_key("zzz"), 1);
    }

    #[test]
    fn reload_bumps_only_that_apps_key() {
        let mut r = ArrowAppRegistry::new();
        assert_eq!(r.reload("a/b"), 1);
        assert_eq!(r.reload("a/b"), 2);
        assert_eq!(r.reload("c/d"), 1);
        assert_eq!((r.reload_key("a/b"), r.reload_key("c/d")), (2, 1));
        assert_eq!(r.reload_key("never"), 0);
    }

    #[test]
    fn prune_drops_the_reload_keys_of_pruned_apps() {
        let mut r = ArrowAppRegistry::new();
        r.show("a/b");
        r.show("c/d");
        r.reload("a/b");
        r.reload("c/d");
        r.prune(&live(&["c/d"]));
        assert_eq!((r.reload_key("a/b"), r.reload_key("c/d")), (0, 1));
    }

    #[test]
    fn stop_removes_one_app_everywhere_and_unsets_it_as_visible() {
        let mut r = ArrowAppRegistry::new();
        r.show("a");
        r.show("b");
        r.reload("b");
        assert!(r.stop("b"));
        assert_eq!((r.order(), r.frames()), (&v(&["a"])[..], &v(&["a"])[..]));
        assert_eq!((r.visible(), r.reload_key("b")), (None, 0));
        assert!(!r.stop("b"));
    }
}
