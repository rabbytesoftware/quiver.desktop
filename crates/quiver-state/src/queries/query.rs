use std::future::Future;

use crate::observable::Store;

/// What a view renders for one read: the last data (kept while a refetch runs), the last error,
/// and whether a fetch is in flight. `data == None && loading` is the first load.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryState<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub loading: bool,
}

impl<T> Default for QueryState<T> {
    fn default() -> Self {
        Self {
            data: None,
            error: None,
            loading: false,
        }
    }
}

pub struct Query<T> {
    state: Store<QueryState<T>>,
}

impl<T> Clone for Query<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T> Default for Query<T> {
    fn default() -> Self {
        Self {
            state: Store::default(),
        }
    }
}

impl<T: Clone + PartialEq> Query<T> {
    pub fn store(&self) -> &Store<QueryState<T>> {
        &self.state
    }

    pub fn snapshot(&self) -> QueryState<T> {
        self.state.snapshot()
    }

    /// Runs `fetch`, publishing loading, then the data or the error. An error keeps the last data.
    ///
    /// Subscribers are woken only when the state a view can see changed: the first load always publishes
    /// (the skeleton is up while `loading`), but a refetch of data already on screen is silent and publishes
    /// once, at the end, and only if the result differs. Home polls every 3 s while core refreshes; without
    /// this every poll repainted the whole window (a 200 ms software-rendered frame) for identical data.
    pub async fn run<F>(
        &self,
        fetch: F,
    ) where
        F: Future<Output = crate::Result<T>>,
    {
        let before = self.state.snapshot();
        let first_load = before.data.is_none();
        let start = |s: &mut QueryState<T>| {
            s.loading = true;
            s.error = None;
        };
        if first_load {
            self.state.update(start);
        } else {
            self.state.update_quiet(start);
        }
        let result = fetch.await;
        let changed = self.state.update_quiet(|s| {
            s.loading = false;
            match result {
                Ok(data) => s.data = Some(data),
                Err(err) => s.error = Some(err.to_string()),
            }
            *s != before
        });
        if first_load || changed {
            self.state.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[tokio::test]
    async fn publishes_data_then_keeps_it_through_a_failing_refetch() {
        let q: Query<u32> = Query::default();
        q.run(async { Ok(1) }).await;
        assert_eq!(
            q.snapshot(),
            QueryState {
                data: Some(1),
                error: None,
                loading: false
            }
        );
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        let s = q.snapshot();
        assert_eq!(
            (s.data, s.error.as_deref(), s.loading),
            (Some(1), Some("backend: boom"), false)
        );
    }

    #[tokio::test]
    async fn a_refetch_returning_the_same_data_wakes_nobody() {
        let q: Query<u32> = Query::default();
        q.run(async { Ok(1) }).await;
        let revision = q.store().revision();
        q.run(async { Ok(1) }).await;
        assert_eq!(q.store().revision(), revision);
        q.run(async { Ok(2) }).await;
        assert_eq!(q.store().revision(), revision + 1);
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        assert_eq!(q.store().revision(), revision + 2);
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        assert_eq!(q.store().revision(), revision + 2);
    }

    #[tokio::test]
    async fn the_first_load_publishes_even_when_it_fails_twice_the_same_way() {
        let q: Query<u32> = Query::default();
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        let revision = q.store().revision();
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        assert!(q.store().revision() > revision);
    }
}
