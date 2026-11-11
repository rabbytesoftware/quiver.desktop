//! English messages for the `arrow.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const ICON_FALLBACK: Key = Key::new("arrow.icon.fallback");
pub const INFERRED_BADGE: Key = Key::new("arrow.inferred.badge");
pub const INFERRED_TOOLTIP: Key = Key::new("arrow.inferred.tooltip");
pub const ERROR: Key = Key::new("arrow.error");
pub const LOADING: Key = Key::new("arrow.loading");
pub const TAB_OVERVIEW: Key = Key::new("arrow.tab.overview");
pub const TAB_ACTIVITY: Key = Key::new("arrow.tab.activity");
pub const TAB_METHODS: Key = Key::new("arrow.tab.methods");
pub const TAB_SETTINGS: Key = Key::new("arrow.tab.settings");
pub const TAB_GROUP: Key = Key::new("arrow.tab.group");
pub const STATE_DISCOVERED: Key = Key::new("arrow.state.discovered");
pub const STATE_ABSENT: Key = Key::new("arrow.state.absent");
pub const STATE_INSTALLING: Key = Key::new("arrow.state.installing");
pub const STATE_READY: Key = Key::new("arrow.state.ready");
pub const STATE_OUTDATED: Key = Key::new("arrow.state.outdated");
pub const STATE_UPDATING: Key = Key::new("arrow.state.updating");
pub const STATE_RUNNING: Key = Key::new("arrow.state.running");
pub const STATE_STOPPING: Key = Key::new("arrow.state.stopping");
pub const STATE_DRAINING: Key = Key::new("arrow.state.draining");
pub const STATE_DETACHED: Key = Key::new("arrow.state.detached");
pub const STATE_UNINSTALLING: Key = Key::new("arrow.state.uninstalling");
pub const STATE_REMOVED: Key = Key::new("arrow.state.removed");
pub const ACTION_ADD_TO_LIBRARY: Key = Key::new("arrow.action.addToLibrary");
pub const ACTION_INSTALL: Key = Key::new("arrow.action.install");
pub const ACTION_INSTALLING: Key = Key::new("arrow.action.installing");
pub const ACTION_REMOVE_FROM_LIBRARY: Key = Key::new("arrow.action.removeFromLibrary");
pub const MENU_DETAILS: Key = Key::new("arrow.menu.details");
pub const ACTION_START: Key = Key::new("arrow.action.start");
pub const ACTION_OPEN: Key = Key::new("arrow.action.open");
pub const INSTALLED_TOAST: Key = Key::new("arrow.installed.toast");
pub const ACTION_UNINSTALL: Key = Key::new("arrow.action.uninstall");
pub const ACTION_UNINSTALLING: Key = Key::new("arrow.action.uninstalling");
pub const ACTION_UPDATE: Key = Key::new("arrow.action.update");
pub const ACTION_UPDATING: Key = Key::new("arrow.action.updating");
pub const ACTION_STOP: Key = Key::new("arrow.action.stop");
pub const ACTION_STOPPING: Key = Key::new("arrow.action.stopping");
pub const ACTION_DRAINING: Key = Key::new("arrow.action.draining");
pub const ACTION_RESTART: Key = Key::new("arrow.action.restart");
pub const ACTION_REINSTALL: Key = Key::new("arrow.action.reinstall");
pub const ACTION_INFO: Key = Key::new("arrow.action.info");
pub const ACTION_ERROR_TITLE: Key = Key::new("arrow.action.error.title");
pub const UPDATE_CURRENT: Key = Key::new("arrow.update.current");
pub const UPDATE_BUSY: Key = Key::new("arrow.update.busy");
pub const UPDATE_RETRY: Key = Key::new("arrow.update.retry");
pub const UPDATE_DISMISS: Key = Key::new("arrow.update.dismiss");
pub const RELEASE_TITLE: Key = Key::new("arrow.release.title");
pub const RELEASE_OFFLINE: Key = Key::new("arrow.release.offline");
pub const RELEASE_RATE_LIMITED: Key = Key::new("arrow.release.rateLimited");
pub const RELEASE_NO_RELEASE: Key = Key::new("arrow.release.noRelease");
pub const RELEASE_NO_ASSET: Key = Key::new("arrow.release.noAsset");
pub const RELEASE_UNSUPPORTED_PLATFORM: Key = Key::new("arrow.release.unsupportedPlatform");
pub const RELEASE_UNVERIFIABLE: Key = Key::new("arrow.release.unverifiable");
pub const RELEASE_UNAVAILABLE: Key = Key::new("arrow.release.unavailable");
pub const RELEASE_NOTHING_AHEAD: Key = Key::new("arrow.release.nothingAhead");
pub const PLATFORM_UNSUPPORTED: Key = Key::new("arrow.platform.unsupported");
pub const PLATFORM_WARNING_TITLE: Key = Key::new("arrow.platform.warning.title");
pub const PLATFORM_WARNING_MESSAGE: Key = Key::new("arrow.platform.warning.message");
pub const PLATFORM_WARNING_CONFIRM: Key = Key::new("arrow.platform.warning.confirm");
pub const PLATFORM_WARNING_CANCEL: Key = Key::new("arrow.platform.warning.cancel");
pub const PROBLEM_LABEL: Key = Key::new("arrow.problem.label");
pub const PROBLEM_DETACHED_NOTE: Key = Key::new("arrow.problem.detachedNote");
pub const PROBLEM_FAILED_NOTE: Key = Key::new("arrow.problem.failedNote");
pub const STEP_INSPECT: Key = Key::new("arrow.step.inspect");
pub const STEP_MODAL_TITLE: Key = Key::new("arrow.step.modal.title");
pub const STEP_TYPE_RUN: Key = Key::new("arrow.step.type.run");
pub const STEP_TYPE_FETCH: Key = Key::new("arrow.step.type.fetch");
pub const STEP_TYPE_EXTRACT: Key = Key::new("arrow.step.type.extract");
pub const STEP_TYPE_PORTABLE: Key = Key::new("arrow.step.type.portable");
pub const STEP_TYPE_SIGNAL: Key = Key::new("arrow.step.type.signal");
pub const STEP_TYPE_DEPENDENCIES: Key = Key::new("arrow.step.type.dependencies");
pub const STEP_TYPE_EXPOSE: Key = Key::new("arrow.step.type.expose");
pub const STEP_TYPE_UNEXPOSE: Key = Key::new("arrow.step.type.unexpose");
pub const INFERRED_CONFIDENCE: Key = Key::new("arrow.inferred.confidence");
pub const INFERRED_WARNINGS: Key = Key::new("arrow.inferred.warnings");
pub const INFERRED_CONFIDENCE_HIGH: Key = Key::new("arrow.inferred.confidence.high");
pub const INFERRED_CONFIDENCE_MEDIUM: Key = Key::new("arrow.inferred.confidence.medium");
pub const INFERRED_CONFIDENCE_LOW: Key = Key::new("arrow.inferred.confidence.low");
pub const INFERRED_WARNING_ASSUMED_ARCH: Key = Key::new("arrow.inferred.warning.assumed_arch");
pub const INFERRED_WARNING_EMULATED: Key = Key::new("arrow.inferred.warning.emulated");
pub const INFERRED_WARNING_WINDOWS_EXE_UNVERIFIED: Key =
    Key::new("arrow.inferred.warning.windows_exe_unverified");
pub const INFERRED_WARNING_NAME_MISMATCH: Key = Key::new("arrow.inferred.warning.name_mismatch");
pub const INFERRED_WARNING_UNPINNED_ROLLING_TAG: Key =
    Key::new("arrow.inferred.warning.unpinned_rolling_tag");
pub const PREVIEW_SUBTITLE: Key = Key::new("arrow.preview.subtitle");
pub const PREVIEW_USES: Key = Key::new("arrow.preview.uses");
pub const PREVIEW_CONFIGURE: Key = Key::new("arrow.preview.configure");
pub const SETTINGS_TITLE: Key = Key::new("arrow.settings.title");
pub const SETTINGS_SUBTITLE: Key = Key::new("arrow.settings.subtitle");
pub const SETTINGS_DONE: Key = Key::new("arrow.settings.done");
pub const SETTINGS_SUMMARY: Key = Key::new("arrow.settings.summary");
pub const SETTINGS_SUMMARY_SENSITIVE: Key = Key::new("arrow.settings.summarySensitive");
pub const SETTINGS_REVEAL: Key = Key::new("arrow.settings.reveal");
pub const SETTINGS_HIDE: Key = Key::new("arrow.settings.hide");
pub const DETAILS_TITLE: Key = Key::new("arrow.details.title");
pub const DETAILS_REQUIREMENTS: Key = Key::new("arrow.details.requirements");
pub const DETAILS_REQUIREMENTS_CPU: Key = Key::new("arrow.details.requirements.cpu");
pub const DETAILS_REQUIREMENTS_CPU_VALUE: Key = Key::new("arrow.details.requirements.cpu.value");
pub const DETAILS_REQUIREMENTS_MEMORY: Key = Key::new("arrow.details.requirements.memory");
pub const DETAILS_REQUIREMENTS_MEMORY_VALUE: Key =
    Key::new("arrow.details.requirements.memory.value");
pub const DETAILS_REQUIREMENTS_DISK: Key = Key::new("arrow.details.requirements.disk");
pub const DETAILS_REQUIREMENTS_DISK_VALUE: Key = Key::new("arrow.details.requirements.disk.value");
pub const DETAILS_NETWORK: Key = Key::new("arrow.details.network");
pub const DETAILS_NETWORK_REQUIRED: Key = Key::new("arrow.details.network.required");
pub const DETAILS_MAINTAINERS: Key = Key::new("arrow.details.maintainers");
pub const DETAILS_CREDITS: Key = Key::new("arrow.details.credits");
pub const DETAILS_LINKS: Key = Key::new("arrow.details.links");
pub const DETAILS_DEPENDENCIES: Key = Key::new("arrow.details.dependencies");
pub const DETAILS_REQUIRED_BY: Key = Key::new("arrow.details.requiredBy");
pub const README_TASK_DONE: Key = Key::new("arrow.readme.taskDone");
pub const README_TASK_TODO: Key = Key::new("arrow.readme.taskTodo");
pub const ACTIVITY_TITLE: Key = Key::new("arrow.activity.title");
pub const ACTIVITY_EMPTY: Key = Key::new("arrow.activity.empty");
pub const ACTIVITY_EMPTY_NOT_INSTALLED: Key = Key::new("arrow.activity.emptyNotInstalled");
pub const ACTIVITY_OUTCOME_SUCCESS: Key = Key::new("arrow.activity.outcome.success");
pub const ACTIVITY_OUTCOME_FAILED: Key = Key::new("arrow.activity.outcome.failed");
pub const ACTIVITY_OUTCOME_CANCELLED: Key = Key::new("arrow.activity.outcome.cancelled");
pub const METHODS_STEPS: Key = Key::new("arrow.methods.steps");
pub const METHODS_COUNT: Key = Key::new("arrow.methods.count");
pub const VERSION_LABEL: Key = Key::new("arrow.version.label");
pub const CHANNEL_LABEL: Key = Key::new("arrow.channel.label");
pub const CHANNEL_POINTER: Key = Key::new("arrow.channel.pointer");
pub const SELECTOR_KIND_CHANNEL: Key = Key::new("arrow.selector.kind.channel");
pub const SELECTOR_KIND_CONSTRAINT: Key = Key::new("arrow.selector.kind.constraint");
pub const SELECTOR_KIND_PIN: Key = Key::new("arrow.selector.kind.pin");
pub const SELECTOR_KIND_COMMIT: Key = Key::new("arrow.selector.kind.commit");
pub const SELECTOR_INSTALLED: Key = Key::new("arrow.selector.installed");
pub const SELECTOR_RESOLVES: Key = Key::new("arrow.selector.resolves");
pub const SELECTOR_SWITCH: Key = Key::new("arrow.selector.switch");
pub const SELECTOR_SWITCH_TITLE: Key = Key::new("arrow.selector.switchTitle");
pub const SELECTOR_SWITCH_NOTE: Key = Key::new("arrow.selector.switchNote");
pub const SELECTOR_SWITCH_PICK: Key = Key::new("arrow.selector.switchPick");
pub const SELECTOR_SWITCH_CONFIRM: Key = Key::new("arrow.selector.switchConfirm");
pub const SELECTOR_SWITCH_CANCEL: Key = Key::new("arrow.selector.switchCancel");
pub const SELECTOR_SWITCH_FAILED: Key = Key::new("arrow.selector.switchFailed");
pub const SELECTOR_REGISTER_FAILED: Key = Key::new("arrow.selector.registerFailed");
pub const SELECTOR_FAILED_UNINSTALL: Key = Key::new("arrow.selector.failed.uninstall");
pub const SELECTOR_FAILED_REMOVE: Key = Key::new("arrow.selector.failed.remove");
pub const SELECTOR_FAILED_INSTALL: Key = Key::new("arrow.selector.failed.install");
pub const SELECTOR_OPEN_OLD: Key = Key::new("arrow.selector.openOld");
pub const SELECTOR_DISMISS: Key = Key::new("arrow.selector.dismiss");
pub const SELECTOR_SWITCH_TITLE_IDLE: Key = Key::new("arrow.selector.switchTitleIdle");
pub const SELECTOR_SWITCH_NOTE_IDLE: Key = Key::new("arrow.selector.switchNoteIdle");
pub const SELECTOR_SWITCH_CONFIRM_IDLE: Key = Key::new("arrow.selector.switchConfirmIdle");
pub const SELECTOR_CHOOSE_TITLE: Key = Key::new("arrow.selector.chooseTitle");
pub const SELECTOR_CHOOSE_PICK: Key = Key::new("arrow.selector.choosePick");
pub const SELECTOR_CHOOSE_CONFIRM: Key = Key::new("arrow.selector.chooseConfirm");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (ICON_FALLBACK, Message::Text("{name} icon")),
    (INFERRED_BADGE, Message::Text("Auto-made")),
    (INFERRED_TOOLTIP, Message::Text("This repository doesn't publish a Quiver manifest. Quiver built one automatically from its release downloads, so install steps may need a check.")),
    (ERROR, Message::Text("Couldn't load this arrow.")),
    (LOADING, Message::Text("Loading…")),
    (TAB_OVERVIEW, Message::Text("Overview")),
    (TAB_ACTIVITY, Message::Text("Activity")),
    (TAB_METHODS, Message::Text("Methods")),
    (TAB_SETTINGS, Message::Text("Settings")),
    (TAB_GROUP, Message::Text("{a} / {b}")),
    (STATE_DISCOVERED, Message::Text("Not in library")),
    (STATE_ABSENT, Message::Text("Not installed")),
    (STATE_INSTALLING, Message::Text("Installing…")),
    (STATE_READY, Message::Text("Ready")),
    (STATE_OUTDATED, Message::Text("Update available")),
    (STATE_UPDATING, Message::Text("Updating…")),
    (STATE_RUNNING, Message::Text("Running")),
    (STATE_STOPPING, Message::Text("Stopping…")),
    (STATE_DRAINING, Message::Text("Draining…")),
    (STATE_DETACHED, Message::Text("Detached")),
    (STATE_UNINSTALLING, Message::Text("Uninstalling…")),
    (STATE_REMOVED, Message::Text("Removed")),
    (ACTION_ADD_TO_LIBRARY, Message::Text("Add to Library")),
    (ACTION_INSTALL, Message::Text("Install")),
    (ACTION_INSTALLING, Message::Text("Installing…")),
    (ACTION_REMOVE_FROM_LIBRARY, Message::Text("Remove from Library")),
    (MENU_DETAILS, Message::Text("Open details")),
    (ACTION_START, Message::Text("Start")),
    (ACTION_OPEN, Message::Text("Open")),
    (INSTALLED_TOAST, Message::Text("{name} is installed. Open it from here, or find it in the Start menu.")),
    (ACTION_UNINSTALL, Message::Text("Uninstall")),
    (ACTION_UNINSTALLING, Message::Text("Uninstalling…")),
    (ACTION_UPDATE, Message::Text("Update")),
    (ACTION_UPDATING, Message::Text("Updating…")),
    (ACTION_STOP, Message::Text("Stop")),
    (ACTION_STOPPING, Message::Text("Stopping…")),
    (ACTION_DRAINING, Message::Text("Draining…")),
    (ACTION_RESTART, Message::Text("Restart")),
    (ACTION_REINSTALL, Message::Text("Reinstall")),
    (ACTION_INFO, Message::Text("What this does")),
    (ACTION_ERROR_TITLE, Message::Text("Couldn't complete the action")),
    (UPDATE_CURRENT, Message::Text("Already up to date")),
    (UPDATE_BUSY, Message::Text("Another update is underway, or the last one is still being applied. Wait a moment and try again.")),
    (UPDATE_RETRY, Message::Text("Try again")),
    (UPDATE_DISMISS, Message::Text("Dismiss")),
    (RELEASE_TITLE, Message::Text("Couldn't start the update")),
    (RELEASE_OFFLINE, Message::Text("Quiver couldn't reach GitHub to find the download. Check your connection and try again.")),
    (RELEASE_RATE_LIMITED, Message::Text("GitHub is temporarily refusing requests from this network. Try again in an hour.")),
    (RELEASE_NO_RELEASE, Message::Text("There is no published Quiver release to download yet.")),
    (RELEASE_NO_ASSET, Message::Text("This Quiver release doesn't include a download for this computer.")),
    (RELEASE_UNSUPPORTED_PLATFORM, Message::Text("Quiver does not publish a download for this kind of computer.")),
    (RELEASE_UNVERIFIABLE, Message::Text("This Quiver release doesn't publish a checksum, so the download can't be verified. Quiver won't replace itself with a file it can't check.")),
    (RELEASE_UNAVAILABLE, Message::Text("Quiver couldn't work out which file to download. Try again later.")),
    (RELEASE_NOTHING_AHEAD, Message::Text("There is nothing newer to update to. The page now shows what Quiver knows.")),
    (PLATFORM_UNSUPPORTED, Message::Text("Not available for {platform}")),
    (PLATFORM_WARNING_TITLE, Message::Text("This isn't available for your platform")),
    (PLATFORM_WARNING_MESSAGE, Message::Text("Quiver detected this computer as {platform}. This arrow doesn't publish a build for it, so Install will likely fail. Adding it to your library won't install anything yet.")),
    (PLATFORM_WARNING_CONFIRM, Message::Text("Add anyway")),
    (PLATFORM_WARNING_CANCEL, Message::Text("Cancel")),
    (PROBLEM_LABEL, Message::Text("Issue")),
    (PROBLEM_DETACHED_NOTE, Message::Text("Quiver lost track of this process. It may still be running outside Quiver’s supervision — use Stop if you want to kill it.")),
    (PROBLEM_FAILED_NOTE, Message::Text("The last run did not finish successfully.")),
    (STEP_INSPECT, Message::Text("Inspect step definition")),
    (STEP_MODAL_TITLE, Message::Text("Raw step definition")),
    (STEP_TYPE_RUN, Message::Text("run")),
    (STEP_TYPE_FETCH, Message::Text("fetch")),
    (STEP_TYPE_EXTRACT, Message::Text("extract")),
    (STEP_TYPE_PORTABLE, Message::Text("portable")),
    (STEP_TYPE_SIGNAL, Message::Text("signal")),
    (STEP_TYPE_DEPENDENCIES, Message::Text("deps")),
    (STEP_TYPE_EXPOSE, Message::Text("expose")),
    (STEP_TYPE_UNEXPOSE, Message::Text("unexpose")),
    (INFERRED_CONFIDENCE, Message::Text("Confidence: {confidence}")),
    (INFERRED_WARNINGS, Message::Text("Things to check")),
    (INFERRED_CONFIDENCE_HIGH, Message::Text("high")),
    (INFERRED_CONFIDENCE_MEDIUM, Message::Text("medium")),
    (INFERRED_CONFIDENCE_LOW, Message::Text("low")),
    (INFERRED_WARNING_ASSUMED_ARCH, Message::Text("The CPU architecture was assumed, not stated by the release.")),
    (INFERRED_WARNING_EMULATED, Message::Text("This build runs through emulation on your platform.")),
    (INFERRED_WARNING_WINDOWS_EXE_UNVERIFIED, Message::Text("The Windows executable could not be verified.")),
    (INFERRED_WARNING_NAME_MISMATCH, Message::Text("The download is not named after the repository.")),
    (INFERRED_WARNING_UNPINNED_ROLLING_TAG, Message::Text("It tracks a rolling tag, so what you get can change without notice.")),
    (PREVIEW_SUBTITLE, Message::Text("What runs when you choose this")),
    (PREVIEW_USES, Message::Text("Uses")),
    (PREVIEW_CONFIGURE, Message::Text("Configure")),
    (SETTINGS_TITLE, Message::Text("Settings")),
    (SETTINGS_SUBTITLE, Message::Text("Used to configure how this arrow runs. Changes apply the next time a method is called.")),
    (SETTINGS_DONE, Message::Text("Done")),
    (SETTINGS_SUMMARY, Message::Plural { one: "{count} setting", other: "{count} settings" }),
    (SETTINGS_SUMMARY_SENSITIVE, Message::Text("{summary} · {sensitive} sensitive")),
    (SETTINGS_REVEAL, Message::Text("Reveal")),
    (SETTINGS_HIDE, Message::Text("Hide")),
    (DETAILS_TITLE, Message::Text("Details")),
    (DETAILS_REQUIREMENTS, Message::Text("Requirements")),
    (DETAILS_REQUIREMENTS_CPU, Message::Text("CPU")),
    (DETAILS_REQUIREMENTS_CPU_VALUE, Message::Plural { one: "{count} core", other: "{count} cores" }),
    (DETAILS_REQUIREMENTS_MEMORY, Message::Text("Memory")),
    (DETAILS_REQUIREMENTS_MEMORY_VALUE, Message::Text("{count} GB")),
    (DETAILS_REQUIREMENTS_DISK, Message::Text("Disk")),
    (DETAILS_REQUIREMENTS_DISK_VALUE, Message::Text("{count} GB")),
    (DETAILS_NETWORK, Message::Text("Network")),
    (DETAILS_NETWORK_REQUIRED, Message::Text("required")),
    (DETAILS_MAINTAINERS, Message::Text("Maintainers")),
    (DETAILS_CREDITS, Message::Text("Credits")),
    (DETAILS_LINKS, Message::Text("Links")),
    (DETAILS_DEPENDENCIES, Message::Text("Dependencies")),
    (DETAILS_REQUIRED_BY, Message::Text("Required by")),
    (README_TASK_DONE, Message::Text("Done")),
    (README_TASK_TODO, Message::Text("To do")),
    (ACTIVITY_TITLE, Message::Text("Activity")),
    (ACTIVITY_EMPTY, Message::Text("No activity yet.")),
    (ACTIVITY_EMPTY_NOT_INSTALLED, Message::Text("Add this arrow to your library to install and run it.")),
    (ACTIVITY_OUTCOME_SUCCESS, Message::Text("Succeeded")),
    (ACTIVITY_OUTCOME_FAILED, Message::Text("Failed")),
    (ACTIVITY_OUTCOME_CANCELLED, Message::Text("Cancelled")),
    (METHODS_STEPS, Message::Plural { one: "{count} step", other: "{count} steps" }),
    (METHODS_COUNT, Message::Plural { one: "{count} method", other: "{count} methods" }),
    (VERSION_LABEL, Message::Text("Version")),
    (CHANNEL_LABEL, Message::Text("Channel")),
    (CHANNEL_POINTER, Message::Text("rolling")),
    (SELECTOR_KIND_CHANNEL, Message::Text("Channel")),
    (SELECTOR_KIND_CONSTRAINT, Message::Text("Range")),
    (SELECTOR_KIND_PIN, Message::Text("Pinned")),
    (SELECTOR_KIND_COMMIT, Message::Text("Commit")),
    (SELECTOR_INSTALLED, Message::Text("Installed {ref}")),
    (SELECTOR_RESOLVES, Message::Text("Resolves to {ref}")),
    (SELECTOR_SWITCH, Message::Text("Switch…")),
    (SELECTOR_SWITCH_TITLE, Message::Text("Switching means reinstalling")),
    (SELECTOR_SWITCH_NOTE, Message::Text("{from} will be uninstalled and removed from your library, and {to} added in its place. Anything that was installed is installed again from scratch.")),
    (SELECTOR_SWITCH_PICK, Message::Text("Pick what {from} should follow instead.")),
    (SELECTOR_SWITCH_CONFIRM, Message::Text("Uninstall and reinstall")),
    (SELECTOR_SWITCH_CANCEL, Message::Text("Cancel")),
    (SELECTOR_SWITCH_FAILED, Message::Text("Couldn't switch")),
    (SELECTOR_REGISTER_FAILED, Message::Text("Couldn't add that version, so nothing was changed: {reason}")),
    (SELECTOR_FAILED_UNINSTALL, Message::Text("{to} was added to your library, but {from} couldn't be uninstalled. Both are in your library, and {from} is still installed. Uninstall and remove it from its own page.")),
    (SELECTOR_FAILED_REMOVE, Message::Text("{to} was added to your library and {from} was uninstalled, but {from} couldn't be removed from your library. Remove it from its own page.")),
    (SELECTOR_FAILED_INSTALL, Message::Text("{to} replaced {from} in your library, but installing it failed, so nothing is installed. Install it from this page.")),
    (SELECTOR_OPEN_OLD, Message::Text("Open the old version")),
    (SELECTOR_DISMISS, Message::Text("Dismiss")),
    (SELECTOR_SWITCH_TITLE_IDLE, Message::Text("Switch version")),
    (SELECTOR_SWITCH_NOTE_IDLE, Message::Text("{from} will be removed from your library and {to} added in its place. Nothing is installed, so nothing is uninstalled.")),
    (SELECTOR_SWITCH_CONFIRM_IDLE, Message::Text("Switch")),
    (SELECTOR_CHOOSE_TITLE, Message::Text("Choose a version")),
    (SELECTOR_CHOOSE_PICK, Message::Text("Pick what {from} follows when you install it or add it to your library.")),
    (SELECTOR_CHOOSE_CONFIRM, Message::Text("Use this version")),
];
