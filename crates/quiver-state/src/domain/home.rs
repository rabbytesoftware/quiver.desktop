use super::search::SearchEntry;

#[derive(Debug, Clone, PartialEq)]
pub struct HomeShelf {
    pub id: String,
    /// Shown verbatim. A shelf is never labelled with a host name.
    pub title: String,
    /// `None` for a shelf that has never been filled.
    pub refreshed_at: Option<String>,
    pub arrows: Vec<SearchEntry>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Home {
    pub shelves: Vec<HomeShelf>,
    /// A refresh is in flight, so an empty snapshot may still fill.
    pub refreshing: bool,
}
