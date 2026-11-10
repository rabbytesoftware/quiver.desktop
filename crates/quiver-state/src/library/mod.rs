//! The full Library view that "View all N arrows" on Home leads to (port of `src/features/library`),
//! with the new design's All / Running / Updates chips.

use crate::arrow_details::RunState;
use crate::domain::{is_quiver_own_component, ArrowEntry};
use crate::i18n::{en::design, Key};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LibraryFilter {
    #[default]
    All,
    Running,
    Updates,
}

pub const FILTERS: [LibraryFilter; 3] = [
    LibraryFilter::All,
    LibraryFilter::Running,
    LibraryFilter::Updates,
];

impl LibraryFilter {
    pub fn label(self) -> Key {
        match self {
            Self::All => design::LIBRARY_FILTER_ALL,
            Self::Running => design::LIBRARY_FILTER_RUNNING,
            Self::Updates => design::LIBRARY_FILTER_UPDATES,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct LibraryModel {
    /// "13 installed arrows" counts every arrow, whatever chip is on.
    pub total: usize,
    pub counts: [(LibraryFilter, usize); 3],
    pub shown: Vec<ArrowEntry>,
}

/// `updates` is the set of namespaces with a newer release known to the client; the entry itself does not
/// carry that, so the view layer passes what it already knows.
pub fn build_library(
    arrows: &[ArrowEntry],
    updates: &[String],
    show_self_components: bool,
    filter: LibraryFilter,
) -> LibraryModel {
    let mut all: Vec<&ArrowEntry> = arrows
        .iter()
        .filter(|a| show_self_components || !is_quiver_own_component(&a.namespace))
        .collect();
    all.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });

    let running = |a: &&ArrowEntry| RunState::from(a.state) == RunState::Running;
    let update = |a: &&ArrowEntry| updates.contains(&a.namespace);
    let counts = [
        (LibraryFilter::All, all.len()),
        (
            LibraryFilter::Running,
            all.iter().copied().filter(running).count(),
        ),
        (
            LibraryFilter::Updates,
            all.iter().copied().filter(update).count(),
        ),
    ];
    let shown = all
        .iter()
        .copied()
        .filter(|a| match filter {
            LibraryFilter::All => true,
            LibraryFilter::Running => running(a),
            LibraryFilter::Updates => update(a),
        })
        .map(Clone::clone)
        .collect();
    LibraryModel {
        total: all.len(),
        counts,
        shown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ArrowState;

    fn arrow(
        ns: &str,
        name: &str,
        state: ArrowState,
    ) -> ArrowEntry {
        ArrowEntry {
            namespace: ns.into(),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            version: "1".into(),
            state,
            active_run: None,
            last_return: None,
            last_used_at: None,
            origin: None,
            confidence: None,
        }
    }

    fn library() -> Vec<ArrowEntry> {
        vec![
            arrow("b", "bravo", ArrowState::Running),
            arrow("a", "Alpha", ArrowState::Ready),
            arrow("c", "Charlie", ArrowState::Ready),
        ]
    }

    #[test]
    fn sorted_by_name_ignoring_case_with_counts_for_every_chip() {
        let model = build_library(&library(), &["c".into()], true, LibraryFilter::All);
        let names: Vec<_> = model.shown.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "bravo", "Charlie"]);
        assert_eq!(
            model.counts,
            [
                (LibraryFilter::All, 3),
                (LibraryFilter::Running, 1),
                (LibraryFilter::Updates, 1)
            ]
        );
    }

    #[test]
    fn a_chip_narrows_the_list_but_not_the_counts() {
        let running = build_library(&library(), &["c".into()], true, LibraryFilter::Running);
        assert_eq!(running.shown.len(), 1);
        assert_eq!(running.shown[0].name, "bravo");
        assert_eq!(running.total, 3);
        let updates = build_library(&library(), &["c".into()], true, LibraryFilter::Updates);
        assert_eq!(updates.shown[0].name, "Charlie");
    }

    #[test]
    fn quivers_own_components_hide_unless_shown() {
        let arrows = [arrow(
            "github.com/rabbytesoftware/quiver.desktop@stable",
            "Desktop",
            ArrowState::Ready,
        )];
        assert_eq!(
            build_library(&arrows, &[], false, LibraryFilter::All).total,
            0
        );
        assert_eq!(
            build_library(&arrows, &[], true, LibraryFilter::All).total,
            1
        );
    }
}
