//! English messages for the `remote.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const TITLE: Key = Key::new("remote.title");
pub const SUBTITLE: Key = Key::new("remote.subtitle");
pub const ADD_BUTTON: Key = Key::new("remote.addButton");
pub const LOCAL_SUBTITLE: Key = Key::new("remote.local.subtitle");
pub const STATUS_STARTING: Key = Key::new("remote.status.starting");
pub const STATUS_READY: Key = Key::new("remote.status.ready");
pub const STATUS_DISCONNECTED: Key = Key::new("remote.status.disconnected");
pub const STATUS_DISCONNECTED_REASON: Key = Key::new("remote.status.disconnectedReason");
pub const ACTION_RETRY: Key = Key::new("remote.action.retry");
pub const ACTION_SWITCH_TO_LOCAL: Key = Key::new("remote.action.switchToLocal");
pub const MENU_MORE: Key = Key::new("remote.menu.more");
pub const MENU_SWITCH: Key = Key::new("remote.menu.switch");
pub const MENU_RENAME: Key = Key::new("remote.menu.rename");
pub const MENU_REMOVE: Key = Key::new("remote.menu.remove");
pub const EMPTY_TITLE: Key = Key::new("remote.empty.title");
pub const EMPTY_DESCRIPTION: Key = Key::new("remote.empty.description");
pub const ADD_TITLE: Key = Key::new("remote.add.title");
pub const ADD_DESCRIPTION: Key = Key::new("remote.add.description");
pub const ADD_DESCRIPTION_PAIRING: Key = Key::new("remote.add.descriptionPairing");
pub const ADD_NAME_LABEL: Key = Key::new("remote.add.name.label");
pub const ADD_NAME_PLACEHOLDER: Key = Key::new("remote.add.name.placeholder");
pub const ADD_URL_LABEL: Key = Key::new("remote.add.url.label");
pub const ADD_URL_PLACEHOLDER: Key = Key::new("remote.add.url.placeholder");
pub const ADD_CODE_LABEL: Key = Key::new("remote.add.code.label");
pub const ADD_CODE_HINT: Key = Key::new("remote.add.code.hint");
pub const ADD_CANCEL: Key = Key::new("remote.add.cancel");
pub const ADD_BACK: Key = Key::new("remote.add.back");
pub const ADD_CONTINUE: Key = Key::new("remote.add.continue");
pub const ADD_CHECKING: Key = Key::new("remote.add.checking");
pub const ADD_HEALTH_ERROR: Key = Key::new("remote.add.healthError");
pub const ADD_SUBMIT: Key = Key::new("remote.add.submit");
pub const ADD_SUBMITTING: Key = Key::new("remote.add.submitting");
pub const RENAME_TITLE: Key = Key::new("remote.rename.title");
pub const RENAME_LABEL: Key = Key::new("remote.rename.label");
pub const RENAME_CANCEL: Key = Key::new("remote.rename.cancel");
pub const RENAME_SUBMIT: Key = Key::new("remote.rename.submit");
pub const REMOVE_TITLE: Key = Key::new("remote.remove.title");
pub const REMOVE_DESCRIPTION: Key = Key::new("remote.remove.description");
pub const REMOVE_ACTIVE_WARNING: Key = Key::new("remote.remove.activeWarning");
pub const REMOVE_CANCEL: Key = Key::new("remote.remove.cancel");
pub const REMOVE_SUBMIT: Key = Key::new("remote.remove.submit");
pub const TOAST_CONNECTED: Key = Key::new("remote.toast.connected");
pub const TOAST_DISCONNECTED: Key = Key::new("remote.toast.disconnected");
pub const TOAST_ADDED: Key = Key::new("remote.toast.added");
pub const TOAST_RENAMED: Key = Key::new("remote.toast.renamed");
pub const TOAST_REMOVED: Key = Key::new("remote.toast.removed");
pub const TOAST_CONNECT_FAILED: Key = Key::new("remote.toast.connectFailed");
pub const TOAST_ADD_FAILED: Key = Key::new("remote.toast.addFailed");
pub const TOAST_RENAME_FAILED: Key = Key::new("remote.toast.renameFailed");
pub const TOAST_REMOVE_FAILED: Key = Key::new("remote.toast.removeFailed");
pub const SWITCHER_LABEL: Key = Key::new("remote.switcher.label");
pub const COMMAND_PLACEHOLDER: Key = Key::new("remote.command.placeholder");
pub const COMMAND_GROUP_LABEL: Key = Key::new("remote.command.groupLabel");
pub const COMMAND_CURRENT: Key = Key::new("remote.command.current");
pub const COMMAND_EMPTY: Key = Key::new("remote.command.empty");
pub const COMMAND_NAVIGATE: Key = Key::new("remote.command.navigate");
pub const COMMAND_SELECT: Key = Key::new("remote.command.select");
pub const COMMAND_CLOSE: Key = Key::new("remote.command.close");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (TITLE, Message::Text("Remote Control")),
    (SUBTITLE, Message::Text("Manage your saved connections.")),
    (ADD_BUTTON, Message::Text("Add remote")),
    (LOCAL_SUBTITLE, Message::Text("This device")),
    (STATUS_STARTING, Message::Text("Connecting…")),
    (STATUS_READY, Message::Text("Connected")),
    (STATUS_DISCONNECTED, Message::Text("Disconnected")),
    (STATUS_DISCONNECTED_REASON, Message::Text("Couldn't reach the daemon. Check the URL, that Quiver is running there, and that the pairing hasn't expired.")),
    (ACTION_RETRY, Message::Text("Retry")),
    (ACTION_SWITCH_TO_LOCAL, Message::Text("Switch to Local")),
    (MENU_MORE, Message::Text("More actions")),
    (MENU_SWITCH, Message::Text("Switch to this connection")),
    (MENU_RENAME, Message::Text("Rename")),
    (MENU_REMOVE, Message::Text("Remove")),
    (EMPTY_TITLE, Message::Text("No remote daemons yet")),
    (EMPTY_DESCRIPTION, Message::Text("Add one to connect to Quiver running elsewhere on your network.")),
    (ADD_TITLE, Message::Text("Add a remote connection")),
    (ADD_DESCRIPTION, Message::Text("Point this app at a Quiver daemon running on another machine.")),
    (ADD_DESCRIPTION_PAIRING, Message::Text("Almost there — enter the pairing code from that machine.")),
    (ADD_NAME_LABEL, Message::Text("Name")),
    (ADD_NAME_PLACEHOLDER, Message::Text("Home Lab")),
    (ADD_URL_LABEL, Message::Text("URL")),
    (ADD_URL_PLACEHOLDER, Message::Text("http://10.0.1.8:7420")),
    (ADD_CODE_LABEL, Message::Text("Pairing code")),
    (ADD_CODE_HINT, Message::Text("Run \"quiver auth generate\" on that machine, then enter the code shown. It expires in 5 minutes and works once.")),
    (ADD_CANCEL, Message::Text("Cancel")),
    (ADD_BACK, Message::Text("Back")),
    (ADD_CONTINUE, Message::Text("Continue")),
    (ADD_CHECKING, Message::Text("Checking…")),
    (ADD_HEALTH_ERROR, Message::Text("Couldn't reach that address. Check the URL and try again.")),
    (ADD_SUBMIT, Message::Text("Add connection")),
    (ADD_SUBMITTING, Message::Text("Adding…")),
    (RENAME_TITLE, Message::Text("Rename connection")),
    (RENAME_LABEL, Message::Text("Name")),
    (RENAME_CANCEL, Message::Text("Cancel")),
    (RENAME_SUBMIT, Message::Text("Save")),
    (REMOVE_TITLE, Message::Text("Remove {name}?")),
    (REMOVE_DESCRIPTION, Message::Text("You'll need a new pairing code to reconnect later.")),
    (REMOVE_ACTIVE_WARNING, Message::Text("This is your current connection — removing it switches you back to Local.")),
    (REMOVE_CANCEL, Message::Text("Cancel")),
    (REMOVE_SUBMIT, Message::Text("Remove")),
    (TOAST_CONNECTED, Message::Text("Connected to {name}")),
    (TOAST_DISCONNECTED, Message::Text("Couldn't reach {name}")),
    (TOAST_ADDED, Message::Text("Added {name}")),
    (TOAST_RENAMED, Message::Text("Renamed to {name}")),
    (TOAST_REMOVED, Message::Text("Removed {name}")),
    (TOAST_CONNECT_FAILED, Message::Text("Couldn't switch to {name}")),
    (TOAST_ADD_FAILED, Message::Text("Couldn't add {name}")),
    (TOAST_RENAME_FAILED, Message::Text("Couldn't rename to {name}")),
    (TOAST_REMOVE_FAILED, Message::Text("Couldn't remove {name}")),
    (SWITCHER_LABEL, Message::Text("Switch connection")),
    (COMMAND_PLACEHOLDER, Message::Text("Switch connection…")),
    (COMMAND_GROUP_LABEL, Message::Text("Connections")),
    (COMMAND_CURRENT, Message::Text("Current")),
    (COMMAND_EMPTY, Message::Text("No connections match \"{query}\".")),
    (COMMAND_NAVIGATE, Message::Text("Navigate")),
    (COMMAND_SELECT, Message::Text("Select")),
    (COMMAND_CLOSE, Message::Text("Close")),
];
