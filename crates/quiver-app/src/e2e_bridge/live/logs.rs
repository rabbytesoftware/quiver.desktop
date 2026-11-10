//! A `log` backend that keeps the most recent lines for `logs.recent` and mirrors them to stderr.

use log::{LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

const CAPACITY: usize = 2000;

static RING: LazyLock<Mutex<VecDeque<String>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(CAPACITY)));

struct RingLogger;

impl Log for RingLogger {
    fn enabled(
        &self,
        metadata: &Metadata,
    ) -> bool {
        metadata.level() <= LevelFilter::Info
    }

    fn log(
        &self,
        record: &Record,
    ) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("{} {}: {}", record.level(), record.target(), record.args());
        eprintln!("{line}");
        if let Ok(mut ring) = RING.lock() {
            if ring.len() == CAPACITY {
                ring.pop_front();
            }
            ring.push_back(line);
        }
    }

    fn flush(&self) {}
}

/// Installs the ring logger. Harmless if a logger is already installed.
pub fn install_logger() {
    if log::set_logger(&RingLogger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

pub fn recent(n: usize) -> Vec<String> {
    RING.lock()
        .map(|r| r.iter().rev().take(n).rev().cloned().collect())
        .unwrap_or_default()
}
