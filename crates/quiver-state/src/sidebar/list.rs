use crate::domain::{is_quiver_own_component, ArrowEntry};
use crate::i18n::{en::sidebar, Key};

/// The catalog load, as the list sees it (the store's `CatalogStatus`, without depending on the store).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CatalogPhase {
    Loading,
    Ready,
    Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ListNotice {
    pub title: Key,
    pub body: Key,
    /// A deep link into Settings, Engine tab, when the fix lives there.
    pub action: Option<Key>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ArrowListState {
    Rows(Vec<ArrowEntry>),
    Notice(ListNotice),
    Loading,
}

pub fn build_list(
    arrows: &[ArrowEntry],
    catalog: CatalogPhase,
    show_self_components: bool,
) -> ArrowListState {
    let mut rows: Vec<ArrowEntry> = arrows
        .iter()
        .filter(|a| show_self_components || !is_quiver_own_component(&a.namespace))
        .cloned()
        .collect();
    rows.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    if !rows.is_empty() {
        return ArrowListState::Rows(rows);
    }
    match catalog {
        CatalogPhase::Error => ArrowListState::Notice(ListNotice {
            title: sidebar::ARROWS_ERROR_TITLE,
            body: sidebar::ARROWS_ERROR_BODY,
            action: Some(sidebar::ARROWS_ERROR_ACTION),
        }),
        CatalogPhase::Loading => ArrowListState::Loading,
        CatalogPhase::Ready => ArrowListState::Notice(ListNotice {
            title: sidebar::ARROWS_EMPTY_TITLE,
            body: sidebar::ARROWS_EMPTY_BODY,
            action: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ArrowState;

    fn arrow(
        ns: &str,
        name: &str,
    ) -> ArrowEntry {
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
            last_used_at: None,
            origin: None,
            confidence: None,
        }
    }

    #[test]
    fn rows_sort_by_name_and_win_over_the_catalog_status() {
        let state = build_list(
            &[arrow("b", "beta"), arrow("a", "Alpha")],
            CatalogPhase::Error,
            true,
        );
        let ArrowListState::Rows(rows) = state else {
            panic!("expected rows")
        };
        assert_eq!(
            rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "beta"]
        );
    }

    #[test]
    fn an_empty_list_says_why() {
        assert!(matches!(
            build_list(&[], CatalogPhase::Error, true),
            ArrowListState::Notice(ListNotice {
                action: Some(_),
                ..
            })
        ));
        assert_eq!(
            build_list(&[], CatalogPhase::Loading, true),
            ArrowListState::Loading
        );
        assert!(matches!(
            build_list(&[], CatalogPhase::Ready, true),
            ArrowListState::Notice(ListNotice { action: None, .. })
        ));
    }

    #[test]
    fn quivers_own_components_hide_unless_shown() {
        let own = arrow("github.com/rabbytesoftware/quiver.core@v1", "Core");
        assert_eq!(
            build_list(std::slice::from_ref(&own), CatalogPhase::Ready, true),
            ArrowListState::Rows(vec![own.clone()])
        );
        assert!(matches!(
            build_list(&[own], CatalogPhase::Ready, false),
            ArrowListState::Notice(_)
        ));
    }
}
