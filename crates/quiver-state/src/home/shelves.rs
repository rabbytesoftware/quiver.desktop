use crate::domain::{is_held, Home, HomeShelf};

/// The shelves with held arrows dropped, and any shelf left empty removed.
pub fn visible_shelves(home: Option<&Home>) -> Vec<HomeShelf> {
    home.map(|h| h.shelves.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|shelf| {
            let arrows: Vec<_> = shelf
                .arrows
                .iter()
                .filter(|a| !is_held(a))
                .cloned()
                .collect();
            (!arrows.is_empty()).then(|| HomeShelf {
                arrows,
                ..shelf.clone()
            })
        })
        .collect()
}

/// True when the snapshot holds no arrow at all, held or not.
pub fn is_snapshot_empty(home: Option<&Home>) -> bool {
    home.is_none_or(|h| h.shelves.iter().all(|s| s.arrows.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::relevance::fixtures::entry;

    fn shelf(id: &str, arrows: Vec<crate::domain::SearchEntry>) -> HomeShelf {
        HomeShelf {
            id: id.into(),
            title: id.into(),
            refreshed_at: None,
            arrows,
        }
    }

    fn open(ns: &str) -> crate::domain::SearchEntry {
        crate::domain::SearchEntry {
            provenance: None,
            ..entry(ns, ns)
        }
    }

    #[test]
    fn held_arrows_drop_and_a_shelf_left_empty_goes_with_them() {
        let held = crate::domain::SearchEntry {
            installed: true,
            ..entry("h", "h")
        };
        let home = Home {
            shelves: vec![
                shelf("a", vec![held.clone(), open("x")]),
                shelf("b", vec![held]),
            ],
            refreshing: false,
        };
        let shown = visible_shelves(Some(&home));
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].arrows.len(), 1);
        assert!(visible_shelves(None).is_empty());
    }

    #[test]
    fn the_snapshot_is_empty_only_when_no_shelf_holds_an_arrow() {
        assert!(is_snapshot_empty(None));
        assert!(is_snapshot_empty(Some(&Home {
            shelves: vec![shelf("a", vec![])],
            refreshing: false
        })));
        assert!(!is_snapshot_empty(Some(&Home {
            shelves: vec![shelf("a", vec![open("x")])],
            refreshing: false
        })));
    }
}
