//! One shared stream per endpoint, fanned out to subscribers, reconnecting with exponential
//! backoff and telling subscribers when it did (frames may have been dropped in between).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::watch;

use crate::core_api::{CoreApi, StreamEvent};

const INITIAL_DELAY: Duration = Duration::from_millis(1000);
const MAX_DELAY: Duration = Duration::from_millis(30_000);

/// Delivered to every subscriber after a reconnect, in place of a frame.
pub fn reconnect_sentinel() -> Value {
    json!({ "reconnected": true })
}

pub fn is_reconnect_sentinel(data: &Value) -> bool {
    data.get("reconnected") == Some(&Value::Bool(true))
}

pub type Callback = Arc<dyn Fn(&Value) + Send + Sync>;

#[derive(Default)]
struct Subscribers {
    next_id: u64,
    list: Vec<(u64, Callback)>,
}

struct Channel {
    subscribers: Arc<Mutex<Subscribers>>,
    cancel: watch::Sender<bool>,
}

type Channels = Arc<Mutex<HashMap<String, Channel>>>;

#[derive(Clone)]
pub struct WsManager {
    api: Arc<dyn CoreApi>,
    channels: Channels,
}

/// Dropping (or calling [`Subscription::unsubscribe`]) removes the callback; the last one out
/// closes the endpoint's stream.
pub struct Subscription {
    channels: Channels,
    endpoint: String,
    subscribers: Arc<Mutex<Subscribers>>,
    id: u64,
    done: bool,
}

impl Subscription {
    pub fn unsubscribe(mut self) {
        self.release();
    }

    fn release(&mut self) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        let empty = {
            let mut subs = lock(&self.subscribers);
            subs.list.retain(|(id, _)| *id != self.id);
            subs.list.is_empty()
        };
        if !empty {
            return;
        }
        let mut channels = lock(&self.channels);
        // Only tear down the channel this subscription belongs to: a fresh resubscription to the
        // same endpoint owns its own subscriber set and must survive a stale unsubscribe.
        if let Some(ch) = channels.get(&self.endpoint) {
            if Arc::ptr_eq(&ch.subscribers, &self.subscribers) {
                let _ = ch.cancel.send(true);
                channels.remove(&self.endpoint);
            }
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.release();
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl WsManager {
    pub fn new(api: Arc<dyn CoreApi>) -> Self {
        Self { api, channels: Arc::default() }
    }

    pub fn subscribe(&self, endpoint: &str, callback: impl Fn(&Value) + Send + Sync + 'static) -> Subscription {
        let mut channels = lock(&self.channels);
        let channel = channels.entry(endpoint.to_string()).or_insert_with(|| {
            let subscribers: Arc<Mutex<Subscribers>> = Arc::default();
            let (cancel, cancelled) = watch::channel(false);
            tokio::spawn(run_channel(self.api.clone(), endpoint.to_string(), subscribers.clone(), cancelled));
            Channel { subscribers, cancel }
        });
        let subscribers = channel.subscribers.clone();
        let id = {
            let mut subs = lock(&subscribers);
            subs.next_id += 1;
            let id = subs.next_id;
            subs.list.push((id, Arc::new(callback)));
            id
        };
        Subscription { channels: self.channels.clone(), endpoint: endpoint.to_string(), subscribers, id, done: false }
    }
}

fn dispatch(subscribers: &Mutex<Subscribers>, data: &Value) {
    let callbacks: Vec<Callback> = lock(subscribers).list.iter().map(|(_, cb)| cb.clone()).collect();
    for cb in callbacks {
        cb(data);
    }
}

fn has_subscribers(subscribers: &Mutex<Subscribers>) -> bool {
    !lock(subscribers).list.is_empty()
}

async fn run_channel(
    api: Arc<dyn CoreApi>,
    endpoint: String,
    subscribers: Arc<Mutex<Subscribers>>,
    mut cancelled: watch::Receiver<bool>,
) {
    // `delay` is the wait after the next close; it doubles per consecutive failure and resets once
    // a stream actually opens.
    let mut delay = INITIAL_DELAY;
    let mut first = true;
    loop {
        if !first {
            let wait = delay;
            delay = (delay * 2).min(MAX_DELAY);
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                _ = cancelled.changed() => return,
            }
            if !has_subscribers(&subscribers) {
                return;
            }
        }
        let opened = tokio::select! {
            r = api.open_stream(&endpoint) => r,
            _ = cancelled.changed() => return,
        };
        let mut stream = match opened {
            Ok(stream) => {
                delay = INITIAL_DELAY;
                Some(stream)
            }
            Err(err) => {
                log::debug!("ws-manager: could not open {endpoint}: {err}");
                None
            }
        };
        if !first {
            dispatch(&subscribers, &reconnect_sentinel());
        }
        first = false;
        let Some(stream) = stream.as_mut() else { continue };
        loop {
            let event = tokio::select! {
                e = stream.next() => e,
                _ = cancelled.changed() => return,
            };
            match event {
                StreamEvent::Message(text) => {
                    let parsed = serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text));
                    dispatch(&subscribers, &parsed);
                }
                StreamEvent::Closed(_) => break,
            }
        }
        if !has_subscribers(&subscribers) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;
    use crate::core_api::FakeCoreApi;

    fn collector() -> (impl Fn(&Value) + Send + Sync + 'static, mpsc::UnboundedReceiver<Value>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (move |v: &Value| drop(tx.send(v.clone())), rx)
    }

    #[tokio::test(start_paused = true)]
    async fn delivers_parsed_frames_to_every_subscriber_over_one_socket() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb1, mut rx1) = collector();
        let (cb2, mut rx2) = collector();
        let _s1 = ws.subscribe("/v0/arrow", cb1);
        let _s2 = ws.subscribe("/v0/arrow", cb2);
        api.wait_stream("/v0/arrow", 0).await.send_text(r#"{"a":1}"#);
        assert_eq!(rx1.recv().await.unwrap(), json!({"a":1}));
        assert_eq!(rx2.recv().await.unwrap(), json!({"a":1}));
        assert_eq!(api.open_count("/v0/arrow"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn passes_an_unparseable_frame_through_raw() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb, mut rx) = collector();
        let _s = ws.subscribe("/e", cb);
        api.wait_stream("/e", 0).await.send_text("not json");
        assert_eq!(rx.recv().await.unwrap(), json!("not json"));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_the_channel_open_while_subscribers_remain_and_closes_with_the_last() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb1, _rx1) = collector();
        let (cb2, mut rx2) = collector();
        let s1 = ws.subscribe("/e", cb1);
        let s2 = ws.subscribe("/e", cb2);
        let handle = api.wait_stream("/e", 0).await;
        s1.unsubscribe();
        handle.send_text("1");
        assert_eq!(rx2.recv().await.unwrap(), json!(1));
        s2.unsubscribe();
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        assert!(handle.is_dropped());
    }

    #[tokio::test(start_paused = true)]
    async fn announces_a_reconnect_with_the_sentinel() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb, mut rx) = collector();
        let _s = ws.subscribe("/e", cb);
        api.wait_stream("/e", 0).await.close(None);
        let sentinel = rx.recv().await.unwrap();
        assert!(is_reconnect_sentinel(&sentinel));
        assert_eq!(api.open_count("/e"), 2);
        assert!(!is_reconnect_sentinel(&json!({"reconnected": 1})));
        assert!(!is_reconnect_sentinel(&json!("x")));
    }

    #[tokio::test(start_paused = true)]
    async fn backs_off_exponentially_across_failures_and_resets_after_a_successful_open() {
        let api = FakeCoreApi::new();
        api.fail_opens("/e", 3);
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb, mut rx) = collector();
        let start = tokio::time::Instant::now();
        let _s = ws.subscribe("/e", cb);
        // First open fails at t=0; retries at +1s, +2s (failing), then +4s succeeds.
        assert!(is_reconnect_sentinel(&rx.recv().await.unwrap())); // retry 1 (t=1s, fails)
        assert!(is_reconnect_sentinel(&rx.recv().await.unwrap())); // retry 2 (t=3s, fails)
        assert_eq!(start.elapsed(), Duration::from_secs(3));
        assert!(is_reconnect_sentinel(&rx.recv().await.unwrap())); // retry 3 (t=7s, opens)
        assert_eq!(start.elapsed(), Duration::from_secs(7));
        // Opened: backoff is reset, so the next drop reconnects after 1s again.
        let handle = api.wait_stream("/e", 3).await;
        let before = tokio::time::Instant::now();
        handle.close(None);
        assert!(is_reconnect_sentinel(&rx.recv().await.unwrap()));
        assert_eq!(before.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_resurrect_a_socket_nobody_listens_to() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb, _rx) = collector();
        let s = ws.subscribe("/e", cb);
        let handle = api.wait_stream("/e", 0).await;
        handle.close(None);
        s.unsubscribe();
        tokio::time::sleep(Duration::from_secs(60)).await;
        assert_eq!(api.open_count("/e"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_fresh_resubscription_survives_a_stale_unsubscribe() {
        let api = FakeCoreApi::new();
        let ws = WsManager::new(Arc::new(api.clone()));
        let (cb1, _rx1) = collector();
        let s1 = ws.subscribe("/e", cb1);
        api.wait_stream("/e", 0).await;
        // Simulate the stale handle: a subscription whose channel was replaced.
        let stale = Subscription {
            channels: ws.channels.clone(),
            endpoint: "/e".into(),
            subscribers: Arc::default(),
            id: 1,
            done: false,
        };
        stale.unsubscribe();
        assert!(lock(&ws.channels).contains_key("/e"));
        drop(s1);
        assert!(!lock(&ws.channels).contains_key("/e"));
    }
}
