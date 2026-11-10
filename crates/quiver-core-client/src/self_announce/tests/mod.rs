//! The announce against a fake daemon, ported from `self-announce.test.ts`.

mod fake;
mod forgetting;
mod register;

use fake::{Daemon, Failure};

use super::namespace::namespace_segment;
use super::*;

const NS: &str = QUIVER_DESKTOP_NAMESPACE;
const LIST: &str = "/v0/arrow?user_installed=true";

fn enc(s: &str) -> String {
    namespace_segment(s)
}

fn id(selector: &str) -> String {
    format!("{NS}@{selector}")
}

fn post(identity: &str) -> String {
    format!("POST /v0/arrow/{}", enc(identity))
}

fn adopt(identity: &str) -> String {
    format!("POST /v0/arrow/{}/adopt", enc(identity))
}

fn delete(identity: &str) -> String {
    format!("DELETE /v0/arrow/{}", enc(identity))
}

fn config(
    tag: Option<&str>,
    channel: &str,
) -> SelfAnnounceConfig {
    SelfAnnounceConfig {
        build_tag: tag.map(str::to_string),
        build_channel: channel.to_string(),
    }
}

async fn announce(
    daemon: &Daemon,
    config: &SelfAnnounceConfig,
    record: Option<&MemoryAnnouncedRows>,
) -> AnnounceOutcome {
    announce_self(daemon, config, record.map(|r| r as &dyn AnnouncedRows)).await
}
