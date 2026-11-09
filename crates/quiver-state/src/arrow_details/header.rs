//! The arrow page header (new design): icon, name, namespace, version control, status + tags row,
//! description, banner slot. The page has no tabs: header, then README left and Details right.

use super::facts::ArrowFacts;
use super::platform::platform_supported_notice;
use super::status::{compute_status, problem_message, ArrowProblem, ArrowStatus, LastRunFacts};
use super::switch::can_switch;
use crate::i18n::{Key, Params, Translator};

/// What sits at the right of the namespace.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VersionControl {
    /// Not in the library yet and the repository publishes channels (or they are still loading): the
    /// Channel and Version pickers choose the identity Add to Library registers.
    Picker {
        loading: bool,
    },
    /// In the library (or nothing to pick): the resolved ref, read-only. Following something else is the
    /// explicit switch flow.
    Fixed {
        version: String,
        selector: String,
        switch: SwitchAffordance,
    },
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwitchAffordance {
    /// Nothing published to switch to.
    Hidden,
    Enabled,
    /// A switch needs the row at rest (not in flight, running or detached), or one is already running.
    Disabled,
}

/// "Not available for this Mac": the header badge when no target matches this machine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlatformBadge {
    pub label: Key,
    pub os: &'static str,
}

#[derive(Clone, PartialEq, Debug)]
pub struct HeaderModel {
    pub icon: Option<String>,
    pub name: String,
    pub namespace: String,
    pub version: VersionControl,
    pub status: ArrowStatus,
    pub inferred: bool,
    pub platform_badge: Option<PlatformBadge>,
    pub tags: Vec<String>,
    /// A chip in the tags row that opens the problem message.
    pub problem: Option<ArrowProblem>,
    pub description: String,
    /// The banner slot; the header is laid out without one when absent.
    pub banner: Option<String>,
    pub license: Option<String>,
}

/// Inputs that are not part of the arrow itself.
#[derive(Clone, Debug, Default)]
pub struct HeaderContext<'a> {
    /// This machine's `os/arch` key.
    pub platform: &'a str,
    /// `platform` is the native side's answer rather than the user-agent guess. The unsupported badge
    /// waits on this: rendering it off the guess flickers at best and is wrong at worst on Apple Silicon.
    pub platform_resolved: bool,
    pub has_channels: bool,
    pub channels_loading: bool,
    pub last_run: Option<LastRunFacts>,
    /// A selector switch is in flight on this identity.
    pub switching: bool,
    /// The selector the row follows (`stable`, `v1.*`), shown beside the resolved version.
    pub selector: &'a str,
}

pub fn build_header(facts: &ArrowFacts, cx: &HeaderContext<'_>) -> HeaderModel {
    let status = compute_status(facts.state, facts.user_installed, facts.update_ahead());
    let platform_badge = (cx.platform_resolved && !facts.platform_supported)
        .then(|| platform_supported_notice(cx.platform));
    let version = if !facts.user_installed && (cx.has_channels || cx.channels_loading) {
        VersionControl::Picker {
            loading: cx.channels_loading,
        }
    } else if facts.resolved_ref.is_empty() {
        VersionControl::None
    } else {
        let switch = if !facts.user_installed || !cx.has_channels {
            SwitchAffordance::Hidden
        } else if cx.switching || !can_switch(facts.state) {
            SwitchAffordance::Disabled
        } else {
            SwitchAffordance::Enabled
        };
        VersionControl::Fixed {
            version: facts.resolved_ref.clone(),
            selector: cx.selector.to_owned(),
            switch,
        }
    };
    HeaderModel {
        icon: facts.icon.clone(),
        name: facts.name.clone(),
        namespace: facts.namespace.clone(),
        version,
        status,
        inferred: facts.inferred,
        platform_badge,
        tags: facts.tags.clone(),
        problem: problem_message(facts.state, cx.last_run.as_ref()),
        description: facts.description.clone(),
        banner: facts.banner.clone().filter(|b| !b.is_empty()),
        license: (!facts.license.is_empty()).then(|| facts.license.clone()),
    }
}

impl PlatformBadge {
    pub fn text(&self, tr: &Translator) -> String {
        tr.t_with(self.label, &Params::new().with("os", self.os))
    }
}

/// Text of the problem message: the detached note, the failed step's own error verbatim, or a generic
/// sentence when core reported none.
pub fn problem_text(problem: &ArrowProblem, tr: &Translator) -> String {
    use crate::i18n::en::arrow;
    match problem {
        ArrowProblem::Detached => tr.t(arrow::PROBLEM_DETACHED_NOTE),
        ArrowProblem::Failed { detail } => detail
            .clone()
            .unwrap_or_else(|| tr.t(arrow::PROBLEM_FAILED_NOTE)),
    }
}
