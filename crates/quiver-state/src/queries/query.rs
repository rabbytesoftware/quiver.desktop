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
        Self { data: None, error: None, loading: false }
    }
}

pub struct Query<T> {
    state: Store<QueryState<T>>,
}

impl<T> Clone for Query<T> {
    fn clone(&self) -> Self {
        Self { state: self.state.clone() }
    }
}

impl<T> Default for Query<T> {
    fn default() -> Self {
        Self { state: Store::default() }
    }
}

impl<T: Clone> Query<T> {
    pub fn store(&self) -> &Store<QueryState<T>> {
        &self.state
    }

    pub fn snapshot(&self) -> QueryState<T> {
        self.state.snapshot()
    }

    /// Runs `fetch`, publishing loading, then the data or the error. An error keeps the last data.
    pub async fn run<F>(&self, fetch: F)
    where
        F: Future<Output = crate::Result<T>>,
    {
        self.state.update(|s| {
            s.loading = true;
            s.error = None;
        });
        let result = fetch.await;
        self.state.update(|s| {
            s.loading = false;
            match result {
                Ok(data) => s.data = Some(data),
                Err(err) => s.error = Some(err.to_string()),
            }
        });
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
        assert_eq!(q.snapshot(), QueryState { data: Some(1), error: None, loading: false });
        q.run(async { Err(Error::Backend("boom".into())) }).await;
        let s = q.snapshot();
        assert_eq!((s.data, s.error.as_deref(), s.loading), (Some(1), Some("backend: boom"), false));
    }
}
