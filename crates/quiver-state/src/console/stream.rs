//! The live log stream as a state machine with no sockets or clocks: the owner feeds it what the socket
//! did and carries out what it asks for. It reconnects with backoff from wherever it got to
//! (`since=<cursor>`), so a blip costs nothing and a long outage is replayed from the daemon's ring.
//!
//! It owns no data. What a frame means -- including noticing that the daemon restarted and the cursor no
//! longer applies -- is the state's business.

use super::entries::StreamState;

pub const LOGS_PATH: &str = "/v0/console/logs";
pub const RECONNECT_BASE_MS: u64 = 1000;
pub const RECONNECT_MAX_MS: u64 = 30_000;

pub fn stream_path(cursor: Option<u64>) -> String {
    match cursor {
        Some(since) => format!("{LOGS_PATH}?level=debug&since={since}"),
        None => format!("{LOGS_PATH}?level=debug"),
    }
}

/// What the owner must do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StreamAction {
    /// Open a socket on `path`; report its events with this `attempt` number.
    Open {
        path: String,
        attempt: u64,
    },
    /// Close the socket of `attempt` without reporting its close (a close fires `onclose`, which must not
    /// schedule a reconnect).
    Close {
        attempt: u64,
    },
    /// Call `on_timer(attempt)` after `ms`.
    Wait {
        ms: u64,
    },
    CancelWait,
    /// The state to show.
    State(StreamState),
}

#[derive(Debug, Default)]
pub struct LogStream {
    active: bool,
    /// The attempt whose events count; an event from an older one is ignored.
    generation: u64,
    delay_ms: u64,
    waiting: bool,
    socket_open: bool,
    /// True once a connection has dropped or been replaced: later attempts are reconnects.
    retrying: bool,
}

impl LogStream {
    pub fn running(&self) -> bool {
        self.active
    }

    pub fn start(
        &mut self,
        cursor: Option<u64>,
    ) -> Vec<StreamAction> {
        if self.active {
            return vec![];
        }
        self.active = true;
        self.retrying = false;
        self.delay_ms = RECONNECT_BASE_MS;
        self.connect(cursor)
    }

    pub fn stop(&mut self) -> Vec<StreamAction> {
        if !self.active {
            return vec![];
        }
        self.active = false;
        let mut actions = self.release();
        actions.push(StreamAction::State(StreamState::Idle));
        actions
    }

    fn release(&mut self) -> Vec<StreamAction> {
        let mut actions = Vec::new();
        if self.waiting {
            actions.push(StreamAction::CancelWait);
            self.waiting = false;
        }
        if self.socket_open {
            actions.push(StreamAction::Close {
                attempt: self.generation,
            });
            self.socket_open = false;
        }
        self.generation += 1;
        actions
    }

    fn connect(
        &mut self,
        cursor: Option<u64>,
    ) -> Vec<StreamAction> {
        self.generation += 1;
        self.socket_open = true;
        vec![
            StreamAction::State(if self.retrying {
                StreamState::Reconnecting
            } else {
                StreamState::Connecting
            }),
            StreamAction::Open {
                path: stream_path(cursor),
                attempt: self.generation,
            },
        ]
    }

    pub fn on_open(
        &mut self,
        attempt: u64,
    ) -> Vec<StreamAction> {
        if attempt != self.generation {
            return vec![];
        }
        self.delay_ms = RECONNECT_BASE_MS;
        vec![StreamAction::State(StreamState::Live)]
    }

    /// Whether a message from `attempt` should be handed on.
    pub fn accepts(
        &self,
        attempt: u64,
    ) -> bool {
        attempt == self.generation
    }

    pub fn on_close(
        &mut self,
        attempt: u64,
    ) -> Vec<StreamAction> {
        if attempt != self.generation || !self.active {
            return vec![];
        }
        self.socket_open = false;
        self.retrying = true;
        self.waiting = true;
        let wait = self.delay_ms;
        self.delay_ms = (self.delay_ms * 2).min(RECONNECT_MAX_MS);
        vec![
            StreamAction::State(StreamState::Reconnecting),
            StreamAction::Wait { ms: wait },
        ]
    }

    /// The backoff elapsed; `cursor` is where the log got to meanwhile.
    pub fn on_timer(
        &mut self,
        cursor: Option<u64>,
    ) -> Vec<StreamAction> {
        self.waiting = false;
        if self.active {
            self.connect(cursor)
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_attempt(actions: &[StreamAction]) -> u64 {
        actions
            .iter()
            .find_map(|a| match a {
                StreamAction::Open { attempt, .. } => Some(*attempt),
                _ => None,
            })
            .expect("an Open action")
    }

    fn wait_ms(actions: &[StreamAction]) -> u64 {
        actions
            .iter()
            .find_map(|a| match a {
                StreamAction::Wait { ms } => Some(*ms),
                _ => None,
            })
            .expect("a Wait action")
    }

    #[test]
    fn the_path_asks_for_debug_and_resumes_after_the_cursor() {
        assert_eq!(stream_path(None), "/v0/console/logs?level=debug");
        assert_eq!(
            stream_path(Some(412)),
            "/v0/console/logs?level=debug&since=412"
        );
    }

    #[test]
    fn starting_connects_and_goes_live_on_open_and_start_twice_is_one_socket() {
        let mut s = LogStream::default();
        let started = s.start(None);
        assert_eq!(started[0], StreamAction::State(StreamState::Connecting));
        let attempt = open_attempt(&started);
        assert!(s.start(None).is_empty());
        assert_eq!(s.on_open(attempt), [StreamAction::State(StreamState::Live)]);
        assert!(s.running());
    }

    #[test]
    fn a_dropped_connection_reconnects_from_the_cursor_with_a_doubling_backoff_capped_at_thirty_seconds(
    ) {
        let mut s = LogStream::default();
        let mut attempt = open_attempt(&s.start(None));
        let mut waits = Vec::new();
        for _ in 0..7 {
            let dropped = s.on_close(attempt);
            assert_eq!(dropped[0], StreamAction::State(StreamState::Reconnecting));
            waits.push(wait_ms(&dropped));
            let again = s.on_timer(Some(9));
            assert!(
                matches!(&again[1], StreamAction::Open { path, .. } if path.ends_with("since=9"))
            );
            attempt = open_attempt(&again);
        }
        assert_eq!(waits, [1000, 2000, 4000, 8000, 16_000, 30_000, 30_000]);
    }

    #[test]
    fn a_successful_open_resets_the_backoff() {
        let mut s = LogStream::default();
        let a = open_attempt(&s.start(None));
        s.on_close(a);
        let b = open_attempt(&s.on_timer(None));
        s.on_close(b);
        let c = open_attempt(&s.on_timer(None));
        s.on_open(c);
        assert_eq!(wait_ms(&s.on_close(c)), RECONNECT_BASE_MS);
    }

    #[test]
    fn the_first_attempt_is_connecting_and_later_ones_are_reconnecting() {
        let mut s = LogStream::default();
        let a = open_attempt(&s.start(None));
        s.on_close(a);
        assert_eq!(
            s.on_timer(None)[0],
            StreamAction::State(StreamState::Reconnecting)
        );
    }

    #[test]
    fn stopping_closes_the_socket_silently_and_stale_events_are_ignored() {
        let mut s = LogStream::default();
        let a = open_attempt(&s.start(None));
        let stopped = s.stop();
        assert!(stopped.contains(&StreamAction::Close { attempt: a }));
        assert_eq!(
            stopped.last(),
            Some(&StreamAction::State(StreamState::Idle))
        );
        assert!(
            s.on_close(a).is_empty(),
            "the close we caused must not schedule a reconnect"
        );
        assert!(s.on_open(a).is_empty());
        assert!(!s.accepts(a));
        assert!(s.stop().is_empty());
        assert!(!s.running());
    }

    #[test]
    fn stopping_while_waiting_cancels_the_timer_and_a_late_timer_does_nothing() {
        let mut s = LogStream::default();
        let a = open_attempt(&s.start(None));
        s.on_close(a);
        assert!(s.stop().contains(&StreamAction::CancelWait));
        assert!(s.on_timer(None).is_empty());
    }

    #[test]
    fn a_new_attempt_makes_the_old_sockets_events_stale() {
        let mut s = LogStream::default();
        let a = open_attempt(&s.start(None));
        s.on_close(a);
        let b = open_attempt(&s.on_timer(None));
        assert!(!s.accepts(a) && s.accepts(b));
        assert!(s.on_close(a).is_empty());
    }
}
