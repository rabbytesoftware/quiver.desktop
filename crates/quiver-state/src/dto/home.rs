use serde::Deserialize;

use super::search::{to_search_entry, SearchResultDto};
use crate::domain::{Home, HomeShelf};

#[derive(Debug, Clone, Deserialize)]
pub struct HomeShelfDto {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub refreshed_at: Option<String>,
    #[serde(default)]
    pub arrows: Option<Vec<SearchResultDto>>,
}

/// `GET /v0/home`. Each arrow is the same DTO `GET /v0/search` returns.
#[derive(Debug, Clone, Deserialize)]
pub struct HomeDto {
    #[serde(default)]
    pub shelves: Option<Vec<HomeShelfDto>>,
    #[serde(default)]
    pub refreshing: Option<bool>,
}

pub fn to_home(dto: HomeDto) -> Home {
    Home {
        shelves: dto
            .shelves
            .unwrap_or_default()
            .into_iter()
            .map(|shelf| HomeShelf {
                id: shelf.id,
                title: shelf.title,
                refreshed_at: shelf.refreshed_at,
                arrows: shelf.arrows.unwrap_or_default().into_iter().map(to_search_entry).collect(),
            })
            .collect(),
        refreshing: dto.refreshing == Some(true),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn home(value: serde_json::Value) -> Home {
        to_home(serde_json::from_value(value).unwrap())
    }

    #[test]
    fn maps_shelves_and_their_arrows_through_the_search_mapper() {
        let h = home(json!({"shelves": [{"id": "s", "title": "Popular", "refreshed_at": "2026-01-01T00:00:00Z",
            "arrows": [{"namespace": "n", "name": "n", "installed": false, "known": true, "media": {"icon": ""}}]}],
            "refreshing": true}));
        assert!(h.refreshing);
        assert_eq!(h.shelves[0].arrows[0].icon, None);
        assert_eq!(h.shelves[0].refreshed_at.as_deref(), Some("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn keeps_a_never_filled_shelf_as_a_null_timestamp_with_no_arrows() {
        let h = home(json!({"shelves": [{"id": "s", "title": "T", "refreshed_at": null, "arrows": null}], "refreshing": false}));
        assert_eq!(h.shelves[0].refreshed_at, None);
        assert!(h.shelves[0].arrows.is_empty());
    }

    #[test]
    fn treats_null_shelves_as_none_and_a_missing_flag_as_false() {
        let h = home(json!({"shelves": null}));
        assert!(h.shelves.is_empty() && !h.refreshing);
    }
}
