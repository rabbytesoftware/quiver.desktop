use std::time::Duration;

use crate::core_api::{CoreApi, CoreApiExt};
use crate::domain::Home;
use crate::dto::home::{to_home, HomeDto};
use crate::Result;

/// How often to re-read while core says a refresh is in flight.
pub const POLL: Duration = Duration::from_millis(3000);

/// `GET /v0/home` -- the recommendation snapshot. A core that predates the endpoint answers 404,
/// which reads as "no shelves" rather than an error.
pub async fn fetch_home(api: &dyn CoreApi) -> Result<Home> {
    match api.get::<HomeDto>("/v0/home").await {
        Ok(dto) => Ok(to_home(dto)),
        Err(err) if err.is_not_found() => Ok(Home::default()),
        Err(err) => Err(err),
    }
}

/// Polls only while core says a refresh is in flight.
pub fn refetch_interval(current: Option<&Home>) -> Option<Duration> {
    current.is_some_and(|h| h.refreshing).then_some(POLL)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::core_api::{FakeCoreApi, Method};

    #[tokio::test]
    async fn maps_the_snapshot() {
        let api = FakeCoreApi::new();
        api.route(Method::Get, "/v0/home", json!({"shelves": [{"id": "s", "title": "T", "refreshed_at": null, "arrows": []}], "refreshing": true}));
        let home = fetch_home(&api).await.unwrap();
        assert_eq!(home.shelves.len(), 1);
        assert!(home.refreshing);
    }

    #[tokio::test]
    async fn reads_a_404_as_no_shelves_and_surfaces_any_other_failure() {
        let api = FakeCoreApi::new();
        assert_eq!(fetch_home(&api).await.unwrap(), Home::default());
        api.route_error(Method::Get, "/v0/home", 500, "boom");
        assert!(fetch_home(&api).await.is_err());
    }

    #[test]
    fn polls_every_three_seconds_while_refreshing_and_not_once_idle() {
        let refreshing = Home {
            shelves: vec![],
            refreshing: true,
        };
        assert_eq!(
            refetch_interval(Some(&refreshing)),
            Some(Duration::from_secs(3))
        );
        assert_eq!(refetch_interval(Some(&Home::default())), None);
        assert_eq!(refetch_interval(None), None);
    }
}
