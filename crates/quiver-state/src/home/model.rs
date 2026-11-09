use super::shelves::{is_snapshot_empty, visible_shelves};
use crate::domain::{is_quiver_own_component, ArrowEntry, CollectionListItem, Home, HomeShelf};

pub const RECENTS_LIMIT: usize = 3;
pub const LIBRARY_PREVIEW_LIMIT: usize = 10;
pub const COLLECTIONS_PREVIEW_LIMIT: usize = 4;
pub const LEAD_SHELF_LIMIT: usize = 3;

pub struct HomeInput<'a> {
    pub arrows: &'a [ArrowEntry],
    pub collections: &'a [CollectionListItem],
    pub home: Option<&'a Home>,
    pub catalog_loading: bool,
    pub collections_loading: bool,
    pub home_loading: bool,
    pub show_self_components: bool,
}

/// A shelf as Home shows it: the lead shelf is capped and links to the full page.
#[derive(Clone, PartialEq, Debug)]
pub struct ShelfView {
    pub shelf: HomeShelf,
    /// "View all N" shows only when the lead shelf was cut.
    pub view_all: Option<usize>,
}

/// Sections in the order the new design stacks them (Library above Popular); each is omitted
/// entirely when it has nothing to show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HomeSection {
    Recents,
    Library,
    LeadShelf,
    Skeleton,
    Collections,
    TrailingShelves,
}

#[derive(Clone, PartialEq, Debug)]
pub struct HomeModel {
    pub empty: bool,
    pub sections: Vec<HomeSection>,
    pub recents: Vec<ArrowEntry>,
    pub library_preview: Vec<ArrowEntry>,
    pub library_total: usize,
    pub collections_preview: Vec<CollectionListItem>,
    pub collections_total: usize,
    pub lead_shelf: Option<ShelfView>,
    pub trailing_shelves: Vec<HomeShelf>,
    pub show_skeleton: bool,
}

fn by_name<T>(name: impl Fn(&T) -> &str) -> impl Fn(&T, &T) -> std::cmp::Ordering {
    move |a, b| {
        name(a)
            .to_lowercase()
            .cmp(&name(b).to_lowercase())
            .then_with(|| name(a).cmp(name(b)))
    }
}

pub fn build_home(input: &HomeInput<'_>) -> HomeModel {
    let all: Vec<&ArrowEntry> = input
        .arrows
        .iter()
        .filter(|a| input.show_self_components || !is_quiver_own_component(&a.namespace))
        .collect();

    let mut used: Vec<&ArrowEntry> = all
        .iter()
        .copied()
        .filter(|a| a.last_used_at.is_some())
        .collect();
    used.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    let recents = used.into_iter().take(RECENTS_LIMIT).cloned().collect();

    let mut by = all.clone();
    by.sort_by(|a, b| by_name(|e: &&ArrowEntry| e.name.as_str())(a, b));
    let library_preview = by
        .into_iter()
        .take(LIBRARY_PREVIEW_LIMIT)
        .cloned()
        .collect();

    let mut cols: Vec<&CollectionListItem> = input.collections.iter().collect();
    cols.sort_by(|a, b| by_name(|c: &&CollectionListItem| c.name.as_str())(a, b));
    let collections_preview = cols
        .into_iter()
        .take(COLLECTIONS_PREVIEW_LIMIT)
        .cloned()
        .collect();

    let mut shelves = visible_shelves(input.home).into_iter();
    let lead_shelf = shelves.next().map(|shelf| {
        let total = shelf.arrows.len();
        let view_all = (total > LEAD_SHELF_LIMIT).then_some(total);
        let arrows = shelf.arrows.into_iter().take(LEAD_SHELF_LIMIT).collect();
        ShelfView {
            shelf: HomeShelf { arrows, ..shelf },
            view_all,
        }
    });
    let trailing_shelves: Vec<HomeShelf> = shelves.collect();

    let show_skeleton = is_snapshot_empty(input.home) && input.home.is_some_and(|h| h.refreshing);
    let empty = !input.catalog_loading
        && !input.collections_loading
        && !input.home_loading
        && all.is_empty()
        && input.collections.is_empty()
        && lead_shelf.is_none()
        && trailing_shelves.is_empty()
        && !show_skeleton;

    let recents: Vec<ArrowEntry> = recents;
    let mut sections = Vec::new();
    if !recents.is_empty() {
        sections.push(HomeSection::Recents);
    }
    if !all.is_empty() {
        sections.push(HomeSection::Library);
    }
    if lead_shelf.is_some() {
        sections.push(HomeSection::LeadShelf);
    }
    if show_skeleton {
        sections.push(HomeSection::Skeleton);
    }
    if !input.collections.is_empty() {
        sections.push(HomeSection::Collections);
    }
    if !trailing_shelves.is_empty() {
        sections.push(HomeSection::TrailingShelves);
    }

    HomeModel {
        empty,
        sections,
        recents,
        library_preview,
        library_total: all.len(),
        collections_preview,
        collections_total: input.collections.len(),
        lead_shelf,
        trailing_shelves,
        show_skeleton,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ArrowState, SearchEntry};
    use crate::search::relevance::fixtures::entry;

    fn arrow(ns: &str, name: &str, used: Option<&str>) -> ArrowEntry {
        ArrowEntry {
            namespace: ns.into(),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            version: "1".into(),
            state: ArrowState::Ready,
            active_run: None,
            last_return: None,
            last_used_at: used.map(Into::into),
            origin: None,
            confidence: None,
        }
    }

    fn input<'a>(
        arrows: &'a [ArrowEntry],
        collections: &'a [CollectionListItem],
        home: Option<&'a Home>,
    ) -> HomeInput<'a> {
        HomeInput {
            arrows,
            collections,
            home,
            catalog_loading: false,
            collections_loading: false,
            home_loading: false,
            show_self_components: true,
        }
    }

    fn collection(name: &str) -> CollectionListItem {
        CollectionListItem {
            namespace: format!("github.com/x/{name}"),
            name: name.into(),
            description: String::new(),
            tags: vec![],
            followed: true,
            arrow_count: 1,
        }
    }

    fn open(ns: &str) -> SearchEntry {
        SearchEntry {
            provenance: None,
            ..entry(ns, ns)
        }
    }

    fn home_of(arrows: usize, refreshing: bool) -> Home {
        let arrows = (0..arrows)
            .map(|i| open(&format!("github.com/x/a{i}")))
            .collect();
        Home {
            shelves: vec![HomeShelf {
                id: "p".into(),
                title: "Popular".into(),
                refreshed_at: None,
                arrows,
            }],
            refreshing,
        }
    }

    #[test]
    fn recents_are_the_three_latest_used_and_never_unused() {
        let arrows = [
            arrow("a", "A", Some("2026-01-01")),
            arrow("b", "B", None),
            arrow("c", "C", Some("2026-03-01")),
            arrow("d", "D", Some("2026-02-01")),
            arrow("e", "E", Some("2026-04-01")),
        ];
        let model = build_home(&input(&arrows, &[], None));
        let names: Vec<_> = model.recents.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["E", "C", "D"]);
    }

    #[test]
    fn the_library_preview_is_ten_by_name_and_the_total_is_all() {
        let arrows: Vec<_> = (0..12)
            .map(|i| arrow(&format!("n{i}"), &format!("arrow {:02}", 11 - i), None))
            .collect();
        let model = build_home(&input(&arrows, &[], None));
        assert_eq!(model.library_preview.len(), 10);
        assert_eq!(model.library_preview[0].name, "arrow 00");
        assert_eq!(model.library_total, 12);
    }

    #[test]
    fn quivers_own_components_hide_unless_shown() {
        let arrows = [
            arrow("github.com/rabbytesoftware/quiver.core", "Core", None),
            arrow("x", "X", None),
        ];
        let mut i = input(&arrows, &[], None);
        assert_eq!(build_home(&i).library_total, 2);
        i.show_self_components = false;
        assert_eq!(build_home(&i).library_total, 1);
    }

    #[test]
    fn collections_preview_is_four_by_name() {
        let cols: Vec<_> = ["e", "d", "c", "b", "a"]
            .iter()
            .map(|n| collection(n))
            .collect();
        let model = build_home(&input(&[], &cols, None));
        assert_eq!(model.collections_preview.len(), 4);
        assert_eq!(model.collections_preview[0].name, "a");
        assert_eq!(model.collections_total, 5);
    }

    #[test]
    fn the_lead_shelf_is_cut_to_three_and_links_to_the_rest() {
        let home = home_of(5, false);
        let model = build_home(&input(&[], &[], Some(&home)));
        let lead = model.lead_shelf.unwrap();
        assert_eq!(lead.shelf.arrows.len(), 3);
        assert_eq!(lead.view_all, Some(5));
        let small = home_of(3, false);
        assert_eq!(
            build_home(&input(&[], &[], Some(&small)))
                .lead_shelf
                .unwrap()
                .view_all,
            None
        );
    }

    #[test]
    fn library_sits_above_popular() {
        let arrows = [arrow("x", "X", Some("2026-01-01"))];
        let cols = [collection("c")];
        let home = home_of(2, false);
        let model = build_home(&input(&arrows, &cols, Some(&home)));
        assert_eq!(
            model.sections,
            [
                HomeSection::Recents,
                HomeSection::Library,
                HomeSection::LeadShelf,
                HomeSection::Collections
            ]
        );
    }

    #[test]
    fn the_skeleton_shows_while_an_empty_snapshot_refreshes() {
        let refreshing = home_of(0, true);
        let model = build_home(&input(&[], &[], Some(&refreshing)));
        assert!(model.show_skeleton);
        assert!(!model.empty);
        assert_eq!(model.sections, [HomeSection::Skeleton]);
    }

    #[test]
    fn empty_only_once_everything_finished_loading_with_nothing() {
        assert!(build_home(&input(&[], &[], None)).empty);
        for (c, k, h) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let mut i = input(&[], &[], None);
            (i.catalog_loading, i.collections_loading, i.home_loading) = (c, k, h);
            assert!(!build_home(&i).empty);
        }
        let arrows = [arrow("x", "X", None)];
        assert!(!build_home(&input(&arrows, &[], None)).empty);
    }
}
