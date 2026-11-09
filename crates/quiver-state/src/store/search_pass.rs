//! The search controller (`src/lib/core-store/search/pass.ts`): Lane A (the local index, asked as
//! soon as a query is committed) and Lane B (a provider discovery pass, started after stillness or
//! on Enter and streamed back), merged into the [`SearchStore`].

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::task::AbortHandle;

use super::search::{SearchJob, SearchStore};
use crate::core_api::{CloseInfo, CoreApi, CoreApiExt, Method, StreamEvent};
use crate::dto::search::{
    to_discovery_summary, to_search_entry, DiscoveryJobDto, DiscoveryJobStartedDto, SearchResultDto,
};
use crate::namespace::encode_uri_component;

/// Stillness before a provider pass. Measured from the committed query -- spec 2.2.1.
pub const IDLE_BEFORE_PASS: Duration = Duration::from_millis(600);
/// How long a reconnected stream may take to close. The clean close frame is the completion
/// signal; this only bounds a pass whose stream broke abnormally and whose replay never reaches one.
pub const PASS_DEADLINE: Duration = Duration::from_millis(25_000);
/// A dropped stream is replayed by core, so one reconnect recovers it.
const MAX_RECONNECTS: u32 = 1;
/// Core's cap (`maxLimit`), not its default of 25. Spec 1.1.
pub const SEARCH_LIMIT: u32 = 100;

/// This machine's real `"os/arch"` key. Both lanes filter on the same platform, or the stream and
/// the re-query cover different sets.
#[async_trait]
pub trait PlatformSource: Send + Sync {
    async fn platform(&self) -> String;
}

/// Neither lane is ever called with a blank query -- core 400s on Lane A, and the mock trims first.
fn is_blank(q: &str) -> bool {
    q.trim().is_empty()
}

#[derive(Default)]
struct Control {
    query: String,
    idle: Option<AbortHandle>,
    safety: Option<AbortHandle>,
    socket: Option<(u64, AbortHandle)>,
    next_socket: u64,
    // Two lifetimes, two counters: a query change invalidates the local fetch AND the pass, but
    // submit invalidates only the pass -- collapsing these into one counter is what let submit
    // discard a local fetch still wanted.
    query_generation: u64,
    pass_generation: u64,
    disposed: bool,
}

struct Shared {
    api: Arc<dyn CoreApi>,
    store: SearchStore,
    platform_source: Arc<dyn PlatformSource>,
    platform: tokio::sync::OnceCell<String>,
    control: Mutex<Control>,
}

#[derive(Clone)]
pub struct SearchController {
    shared: Arc<Shared>,
}

impl SearchController {
    pub fn new(api: Arc<dyn CoreApi>, store: SearchStore, platform_source: Arc<dyn PlatformSource>) -> Self {
        Self {
            shared: Arc::new(Shared {
                api,
                store,
                platform_source,
                platform: tokio::sync::OnceCell::new(),
                control: Mutex::default(),
            }),
        }
    }

    /// Called with a committed (URL) query -- already debounced by the field. `discover: false`
    /// when the screen is being restored rather than asked for: Lane A still runs, because the
    /// vault holds what the last pass found.
    pub fn set_query(&self, next: &str, discover: bool) {
        let query_generation = {
            let mut c = self.shared.ctl();
            if c.disposed || c.query.as_str() == next {
                return;
            }
            c.query = next.to_string();
            c.cancel_all();
            c.query_generation
        };
        self.shared.store.set_query(next);
        if is_blank(next) {
            return;
        }
        let shared = self.shared.clone();
        tokio::spawn(async move { shared.run_local(query_generation).await });
        if discover {
            self.shared.arm();
        }
    }

    /// Enter: fire the pass now.
    pub fn submit(&self) {
        let generation = {
            let mut c = self.shared.ctl();
            if c.disposed || is_blank(&c.query) {
                return;
            }
            c.cancel_pass();
            c.pass_generation
        };
        let shared = self.shared.clone();
        tokio::spawn(async move { shared.start_pass(generation).await });
    }

    pub fn dispose(&self) {
        {
            let mut c = self.shared.ctl();
            c.cancel_all();
            c.disposed = true;
        }
        self.shared.store.clear();
    }
}

impl Control {
    fn clear_idle(&mut self) {
        if let Some(h) = self.idle.take() {
            h.abort();
        }
    }

    /// Closing the socket is the cancel -- there is no cancel endpoint (spec 1.3).
    fn stop_pass(&mut self) {
        if let Some(h) = self.safety.take() {
            h.abort();
        }
        if let Some((_, h)) = self.socket.take() {
            h.abort();
        }
    }

    fn cancel_pass(&mut self) {
        self.pass_generation += 1;
        self.clear_idle();
        self.stop_pass();
    }

    fn cancel_all(&mut self) {
        self.query_generation += 1;
        self.cancel_pass();
    }

    fn is_current(&self, generation: u64) -> bool {
        !self.disposed && self.pass_generation == generation
    }
}

impl Shared {
    fn ctl(&self) -> MutexGuard<'_, Control> {
        self.control.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn query(&self) -> String {
        self.ctl().query.clone()
    }

    fn is_current(&self, generation: u64) -> bool {
        self.ctl().is_current(generation)
    }

    fn query_is_current(&self, generation: u64) -> bool {
        let c = self.ctl();
        !c.disposed && c.query_generation == generation
    }

    async fn platform(&self) -> &str {
        self.platform.get_or_init(|| self.platform_source.platform()).await
    }

    /// Sorting and narrowing (spec 9.6) act on the answer the client is holding, so the answer has
    /// to be the whole answer. Core defaults `limit` to 25 and caps it at 100; asking for the cap
    /// makes the set complete for any query that does not match more arrows than exist on the
    /// machine plus one pass.
    async fn local_path(&self) -> String {
        let os = encode_uri_component(self.platform().await);
        format!("/v0/search?q={}&limit={SEARCH_LIMIT}&os={os}", encode_uri_component(&self.query()))
    }

    async fn run_local(self: Arc<Self>, generation: u64) {
        if is_blank(&self.query()) {
            return;
        }
        let path = self.local_path().await;
        let result = self.api.get::<Vec<SearchResultDto>>(&path).await;
        if !self.query_is_current(generation) {
            return;
        }
        match result {
            Ok(dtos) => self.store.set_local(dtos.into_iter().map(to_search_entry).collect()),
            Err(_) => self.store.set_local_error(),
        }
    }

    /// Streamed results are unranked (spec 3); Lane A now sees the vault the pass just filled.
    async fn requery(self: Arc<Self>, generation: u64) {
        let path = self.local_path().await;
        let result = self.api.get::<Vec<SearchResultDto>>(&path).await;
        if !self.is_current(generation) {
            return;
        }
        match result {
            Ok(dtos) => self.store.settle(dtos.into_iter().map(to_search_entry).collect()),
            Err(_) => self.store.settle_failed(),
        }
    }

    fn arm(self: &Arc<Self>) {
        let (generation, blank) = {
            let mut c = self.ctl();
            c.clear_idle();
            (c.pass_generation, is_blank(&c.query))
        };
        if blank {
            return;
        }
        let shared = self.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(IDLE_BEFORE_PASS).await;
            if shared.is_current(generation) {
                shared.start_pass(generation).await;
            }
        });
        self.ctl().idle = Some(handle.abort_handle());
    }

    async fn start_pass(self: Arc<Self>, generation: u64) {
        let query = self.query();
        if is_blank(&query) {
            return;
        }
        let started = self.api.request(Method::Post, "/v0/search/discover", Some(json!({ "q": query }))).await;
        let started = started
            .ok()
            .and_then(|r| r.data)
            .and_then(|v| serde_json::from_value::<DiscoveryJobStartedDto>(v).ok());
        let Some(started) = started else { return };
        if !self.is_current(generation) {
            return;
        }
        self.store.begin_pass(SearchJob { id: started.job_id.clone(), expires_at: started.expires_at });

        let path = format!("/v0/search/discover/{}", started.job_id);
        let os = encode_uri_component(self.platform().await);
        if !self.is_current(generation) {
            return;
        }
        self.connect(generation, path, os, 0);
    }

    fn connect(self: &Arc<Self>, generation: u64, path: String, os: String, reconnects: u32) {
        let mut c = self.ctl();
        let id = c.next_socket;
        c.next_socket += 1;
        let shared = self.clone();
        let handle = tokio::spawn(async move { shared.run_socket(generation, id, path, os, reconnects).await });
        c.socket = Some((id, handle.abort_handle()));
    }

    async fn run_socket(self: Arc<Self>, generation: u64, id: u64, path: String, os: String, reconnects: u32) {
        let mut close = None;
        match self.api.open_stream(&format!("{path}?os={os}")).await {
            Err(_) => {}
            Ok(mut stream) => loop {
                match stream.next().await {
                    StreamEvent::Message(text) => {
                        if !self.is_current(generation) {
                            continue;
                        }
                        // A frame we cannot parse is not worth tearing the pass down for.
                        if let Ok(dto) = serde_json::from_str::<SearchResultDto>(&text) {
                            self.store.receive(to_search_entry(dto));
                        }
                    }
                    StreamEvent::Closed(info) => {
                        close = info;
                        break;
                    }
                }
            },
        }
        self.on_close(generation, id, path, os, reconnects, close).await;
    }

    async fn on_close(
        self: Arc<Self>,
        generation: u64,
        id: u64,
        path: String,
        os: String,
        reconnects: u32,
        info: Option<CloseInfo>,
    ) {
        enum Next {
            Ignore,
            Complete,
            Fail,
            Reconnect,
        }
        let next = {
            let mut c = self.ctl();
            if !c.is_current(generation) || c.socket.as_ref().map(|(i, _)| *i) != Some(id) {
                Next::Ignore
            } else if info.as_ref().is_some_and(|i| i.code == 1000 && i.reason == "completed") {
                // The close frame is the end of the stream; the job summary is readable from then on.
                c.socket = None;
                if let Some(h) = c.safety.take() {
                    h.abort();
                }
                Next::Complete
            } else if reconnects >= MAX_RECONNECTS {
                c.socket = None;
                if let Some(h) = c.safety.take() {
                    h.abort();
                }
                Next::Fail
            } else {
                // Core replays everything the pass emitted, and `receive` dedups on the bare
                // namespace, so a reconnect is safe to repeat over what we hold.
                c.socket = None;
                if c.safety.is_none() {
                    let shared = self.clone();
                    let timer = tokio::spawn(async move {
                        tokio::time::sleep(PASS_DEADLINE).await;
                        if shared.ctl().is_current(generation) {
                            shared.ctl().stop_pass();
                            shared.store.settle_failed();
                        }
                    });
                    c.safety = Some(timer.abort_handle());
                }
                Next::Reconnect
            }
        };
        match next {
            Next::Ignore => {}
            Next::Fail => self.store.settle_failed(),
            Next::Reconnect => self.connect(generation, path, os, reconnects + 1),
            Next::Complete => {
                match self.api.get::<DiscoveryJobDto>(&path).await {
                    Ok(job) => {
                        if !self.is_current(generation) {
                            return;
                        }
                        self.store.end_pass(to_discovery_summary(job));
                    }
                    Err(_) => {
                        if !self.is_current(generation) {
                            return;
                        }
                    }
                }
                tokio::spawn(self.requery(generation));
            }
        }
    }
}
