//! What the Tauri `resolve_release_asset`, `get_build_tag` and `get_build_stamp` commands gave the
//! frontend, as one seam: Quiver's own row installs a release asset nothing in its manifest can
//! name, so the app resolves it at click time and `quiver-state` decides what to do with the answer.

use async_trait::async_trait;
use quiver_release::build_info::{build_stamp, release_tag, BuildStamp};
use quiver_release::ResolvedAsset;
use quiver_state::arrow_details::release_variables::{
    is_self_arrow, release_variables, ReleaseFailure, ReleaseResolveErrorKind,
};

/// What this binary was built from. Every field is `None` for an unstamped (dev, PR) build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    /// The `stable-*` tag the build was cut from (`get_build_tag`).
    pub tag: Option<String>,
    /// Commit, build time and release label for the build indicator (`get_build_stamp`).
    pub stamp: BuildStamp,
    /// The channel the release pipeline published this build on; empty when unknown.
    pub channel: String,
}

impl BuildInfo {
    /// Read the values baked in at compile time.
    pub fn baked() -> Self {
        let stamp = build_stamp();
        let channel = option_env!("QUIVER_DESKTOP_BUILD_CHANNEL")
            .map(str::to_owned)
            .or_else(|| {
                let label = stamp.label.as_deref()?;
                Some(label.split_once('-')?.0.to_owned())
            })
            .unwrap_or_default();
        Self {
            tag: release_tag().map(str::to_owned),
            stamp,
            channel,
        }
    }
}

#[async_trait]
pub trait ReleaseSource: Send + Sync {
    /// This app's own release asset for the platform it runs on, from the release `tag` (the ref
    /// the row is moving to) or the newest one.
    async fn resolve_release_asset(
        &self,
        tag: Option<&str>,
    ) -> Result<ResolvedAsset, quiver_release::Error>;

    fn build_info(&self) -> BuildInfo;
}

/// The real source: the GitHub release page and the compile-time stamp.
pub struct NativeReleaseSource {
    info: BuildInfo,
}

impl NativeReleaseSource {
    pub fn new() -> Self {
        Self {
            info: BuildInfo::baked(),
        }
    }
}

impl Default for NativeReleaseSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReleaseSource for NativeReleaseSource {
    async fn resolve_release_asset(
        &self,
        tag: Option<&str>,
    ) -> Result<ResolvedAsset, quiver_release::Error> {
        quiver_release::resolve_own_release(tag).await
    }

    fn build_info(&self) -> BuildInfo {
        self.info.clone()
    }
}

/// The `QUIVER_RELEASE_*` variables an install or update of `namespace` must carry: none for any
/// other arrow, the resolved asset's URL and checksum for Quiver's own row, or why they cannot be
/// had (no network, no published checksum, ...).
pub async fn resolve_release_variables(
    source: &dyn ReleaseSource,
    namespace: &str,
    tag: Option<&str>,
) -> Result<Vec<(String, String)>, ReleaseFailure> {
    if !is_self_arrow(namespace) {
        return Ok(Vec::new());
    }
    let asset = source
        .resolve_release_asset(tag)
        .await
        .map_err(|e| ReleaseFailure {
            kind: ReleaseResolveErrorKind::parse(e.kind()).unwrap_or(ReleaseResolveErrorKind::Http),
            detail: e.detail().to_owned(),
        })?;
    release_variables(
        &asset.name,
        &asset.tag,
        &asset.url,
        asset.checksum.as_deref().unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct Fake {
        reply: Mutex<Option<Result<ResolvedAsset, quiver_release::Error>>>,
        asked: Mutex<Vec<Option<String>>>,
    }

    impl Fake {
        fn new(reply: Result<ResolvedAsset, quiver_release::Error>) -> Self {
            Self {
                reply: Mutex::new(Some(reply)),
                asked: Mutex::default(),
            }
        }
    }

    #[async_trait]
    impl ReleaseSource for Fake {
        async fn resolve_release_asset(
            &self,
            tag: Option<&str>,
        ) -> Result<ResolvedAsset, quiver_release::Error> {
            self.asked.lock().unwrap().push(tag.map(str::to_owned));
            self.reply.lock().unwrap().take().unwrap()
        }

        fn build_info(&self) -> BuildInfo {
            BuildInfo::baked()
        }
    }

    fn asset(checksum: Option<&str>) -> ResolvedAsset {
        ResolvedAsset {
            tag: "stable-26.5".into(),
            name: "quiver.tar.gz".into(),
            url: "https://example.test/quiver.tar.gz".into(),
            checksum: checksum.map(str::to_owned),
        }
    }

    const SELF_NS: &str = "github.com/rabbytesoftware/quiver.desktop";

    #[tokio::test]
    async fn another_arrow_needs_no_variables_and_asks_nobody() {
        let source = Fake::new(Ok(asset(Some("ab"))));
        let vars = resolve_release_variables(&source, "github.com/other/arrow", None)
            .await
            .unwrap();
        assert!(vars.is_empty());
        assert!(source.asked.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn quivers_own_row_gets_the_url_and_the_checksum_for_the_requested_tag() {
        let source = Fake::new(Ok(asset(Some("ab12"))));
        let vars =
            resolve_release_variables(&source, &format!("{SELF_NS}@stable"), Some("stable-26.5"))
                .await
                .unwrap();
        assert_eq!(
            vars,
            vec![
                (
                    "QUIVER_RELEASE_ASSET_URL".to_string(),
                    "https://example.test/quiver.tar.gz".to_string()
                ),
                ("QUIVER_RELEASE_CHECKSUM".to_string(), "ab12".to_string()),
            ]
        );
        assert_eq!(
            *source.asked.lock().unwrap(),
            vec![Some("stable-26.5".to_string())]
        );
    }

    #[tokio::test]
    async fn an_asset_without_a_published_checksum_is_refused_not_installed_anyway() {
        let source = Fake::new(Ok(asset(None)));
        let failure = resolve_release_variables(&source, SELF_NS, None)
            .await
            .unwrap_err();
        assert_eq!(failure.kind, ReleaseResolveErrorKind::Unverifiable);
    }

    #[tokio::test]
    async fn a_resolver_failure_keeps_its_kind_and_detail() {
        let source = Fake::new(Err(quiver_release::Error::RateLimited("slow down".into())));
        let failure = resolve_release_variables(&source, SELF_NS, None)
            .await
            .unwrap_err();
        assert_eq!(failure.kind, ReleaseResolveErrorKind::RateLimited);
        assert_eq!(failure.detail, "slow down");
    }

    #[test]
    fn an_unstamped_build_reports_no_tag_and_no_label() {
        let info = NativeReleaseSource::new().build_info();
        assert_eq!(info.tag, None);
        assert_eq!(info.stamp.label, None);
    }
}
