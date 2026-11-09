//! The announce itself.
//!
//! 1. Register. A build registers the channel its release pipeline published
//!    it on (`build_channel`: `stable`, `beta`, `hotfix`, `nightly-rolling`) --
//!    a catalog identity's selector never changes, so the row follows that
//!    channel the way quiver.core files its own row. A channel the repository
//!    does not list yet is not registered. Without one, a `stable-*` build tag
//!    means `stable`, and any other build registers the repository's default
//!    channel -- the first one `/channels` lists, which is what a refless add
//!    would pick -- by name, so it knows which row is its own. Only when the
//!    channels cannot be read does it register refless, and then it forgets
//!    nothing. An identity already in the library is not registered again.
//! 2. Adopt. A release build then declares its own tag as what is installed
//!    (`POST /v0/arrow/:ns/adopt`), so the version check offers exactly the
//!    releases newer than the running one -- unless the row already records
//!    that tag, so re-announcing the same build sends nothing. A newer build
//!    (after a self-update relaunch) advances the same row in place. A tag
//!    core will not accept for the channel (400/404) leaves the plain
//!    registration.
//! 3. Forget what earlier announces left. An identity this app registered
//!    itself is recorded; only once the announce fully succeeded, every OTHER
//!    recorded identity still in the library is removed -- the row a build of
//!    another channel filed, e.g. `@nightly-rolling` after the user moved to a
//!    stable build. A row the user added (any pin, any channel) was never
//!    recorded and is never touched.
//!
//! Fire-and-forget-with-logging throughout: a daemon too old for any of this,
//! an unreachable manifest host, or a transient error must never fail or block
//! the connection this rides on.

use std::collections::HashMap;

use futures_util::future::join_all;
use serde::Deserialize;

use super::api::{call, fetch};
use super::namespace::{namespace_segment, with_selector};
use super::{AnnouncedRows, SelfAnnounceConfig, QUIVER_DESKTOP_NAMESPACE};
use crate::transport::Transport;
use crate::Error;

/// The channel a build tag belongs to: the release pipeline only ever stamps `stable-*`.
const STABLE_CHANNEL: &str = "stable";

/// What an announce did that the host may need to react to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnnounceOutcome {
    /// The row was adopted, so the catalog should be asked to re-read what it records.
    pub adopted: bool,
}

#[derive(Deserialize)]
struct ChannelList {
    channels: Vec<Channel>,
}

#[derive(Deserialize)]
struct Channel {
    name: String,
}

#[derive(Deserialize)]
struct ArrowItem {
    namespace: String,
    #[serde(default)]
    versions: Vec<ArrowVersion>,
}

#[derive(Deserialize)]
struct ArrowVersion {
    #[serde(rename = "ref")]
    reference: String,
    resolved_ref: Option<String>,
}

/// Announce quiver.desktop to the daemon behind `transport`. Never fails: every
/// problem is logged and ends the announce where it stands.
pub async fn announce_self(
    transport: &dyn Transport,
    config: &SelfAnnounceConfig,
    record: Option<&dyn AnnouncedRows>,
) -> AnnounceOutcome {
    let mut outcome = AnnounceOutcome::default();
    let tag = config.build_tag.as_deref().filter(|t| !t.is_empty());
    let selector = selector_for(transport, config, tag).await;
    let identity = with_selector(QUIVER_DESKTOP_NAMESPACE, &selector);
    let known = known_rows(transport).await;

    if !known.as_ref().is_some_and(|k| k.contains_key(&identity)) {
        if !register(transport, &identity).await {
            return outcome;
        }
        // Only a row known to be absent is this app's own; a refless register
        // leaves no way to tell which row core filed.
        if !selector.is_empty() && known.is_some() {
            if let Some(record) = record {
                record.add(&identity);
            }
        }
    }
    if let Some(tag) = tag {
        let recorded = known
            .as_ref()
            .and_then(|k| k.get(&identity))
            .map(String::as_str);
        if recorded != Some(tag) {
            if !adopt(transport, &identity, tag).await {
                return outcome;
            }
            outcome.adopted = true;
        }
    }
    if let (false, Some(known), Some(record)) = (selector.is_empty(), &known, record) {
        forget_earlier_announces(transport, record, known, &identity).await;
    }
    outcome
}

async fn register(transport: &dyn Transport, identity: &str) -> bool {
    let path = format!("/v0/arrow/{}", namespace_segment(identity));
    match call(transport, "POST", &path, None).await {
        Ok(_) => true,
        Err(err) => {
            log::error!("core-store: failed to self-announce quiver.desktop: {err}");
            false
        }
    }
}

/// The selector this build files itself under; empty (refless) only when nothing names one.
async fn selector_for(
    transport: &dyn Transport,
    config: &SelfAnnounceConfig,
    tag: Option<&str>,
) -> String {
    let channel = config.build_channel.trim();
    if tag.is_some() && channel.is_empty() {
        return STABLE_CHANNEL.to_string();
    }
    let listed = listed_channels(transport).await;
    if !channel.is_empty() {
        if listed
            .as_ref()
            .is_none_or(|l| l.iter().any(|c| c == channel))
        {
            return channel.to_string();
        }
        log::debug!(
            "core-store: quiver.desktop publishes no {channel} channel yet; announcing its default"
        );
    }
    if tag.is_some() {
        return STABLE_CHANNEL.to_string();
    }
    listed
        .and_then(|l| l.into_iter().next())
        .unwrap_or_default()
}

/// Every channel quiver.desktop publishes, the one a refless add would pick first; `None` when unknown.
async fn listed_channels(transport: &dyn Transport) -> Option<Vec<String>> {
    let path = format!(
        "/v0/arrow/{}/channels",
        namespace_segment(QUIVER_DESKTOP_NAMESPACE)
    );
    match fetch::<ChannelList>(transport, &path).await {
        Ok(list) => Some(list.channels.into_iter().map(|c| c.name).collect()),
        Err(err) => {
            log::error!(
                "core-store: could not read quiver.desktop channels; announcing refless: {err}"
            );
            None
        }
    }
}

/// Whether the declared state now stands: adopted.
async fn adopt(transport: &dyn Transport, identity: &str, tag: &str) -> bool {
    let path = format!("/v0/arrow/{}/adopt", namespace_segment(identity));
    let body = serde_json::json!({ "resolved_ref": tag });
    match call(transport, "POST", &path, Some(body)).await {
        Ok(_) => true,
        Err(Error::Api {
            status: 400 | 404,
            message,
        }) => {
            log::debug!(
				"core-store: {tag} is not a release {identity} can adopt; leaving the plain registration: {message}"
			);
            false
        }
        Err(err) => {
            log::error!("core-store: failed to adopt {tag} for {identity}: {err}");
            false
        }
    }
}

/// Every quiver.desktop identity in the library and the ref each resolved to; `None` when the catalog cannot be read.
async fn known_rows(transport: &dyn Transport) -> Option<HashMap<String, String>> {
    match fetch::<Vec<ArrowItem>>(transport, "/v0/arrow?user_installed=true").await {
        Ok(items) => {
            let rows = items
                .into_iter()
                .find(|item| item.namespace == QUIVER_DESKTOP_NAMESPACE)
                .map(|item| item.versions)
                .unwrap_or_default();
            Some(
                rows.into_iter()
                    .map(|v| {
                        (
                            with_selector(QUIVER_DESKTOP_NAMESPACE, &v.reference),
                            v.resolved_ref.unwrap_or_default(),
                        )
                    })
                    .collect(),
            )
        }
        Err(err) => {
            log::error!(
				"core-store: could not list the catalog; leaving any stale quiver.desktop rows: {err}"
			);
            None
        }
    }
}

/// Removes every recorded identity but `current`; one the library no longer holds, or core no longer has, just leaves the record.
async fn forget_earlier_announces(
    transport: &dyn Transport,
    record: &dyn AnnouncedRows,
    known: &HashMap<String, String>,
    current: &str,
) {
    let stale: Vec<String> = record
        .list()
        .into_iter()
        .filter(|identity| identity != current)
        .collect();
    join_all(stale.iter().map(|identity| async move {
        if !known.contains_key(identity) {
            record.drop_identity(identity);
            return;
        }
        let path = format!("/v0/arrow/{}", namespace_segment(identity));
        match call(transport, "DELETE", &path, None).await {
            Ok(_) | Err(Error::Api { status: 404, .. }) => record.drop_identity(identity),
            Err(err) => log::error!(
				"core-store: could not remove the stale {identity} row; retrying on the next announce: {err}"
			),
        }
    }))
    .await;
}
