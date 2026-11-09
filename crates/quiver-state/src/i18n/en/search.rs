//! English messages for the `search.*` keys. Generated from src/lib/i18n/locales/en.ts, then maintained by hand.

use crate::i18n::message::{Key, Message};

pub const PLACEHOLDER: Key = Key::new("search.placeholder");
pub const LABEL: Key = Key::new("search.label");
pub const RESULTS_COUNT: Key = Key::new("search.results.count");
pub const RESULTS_SEARCHING: Key = Key::new("search.results.searching");
pub const RESULTS_PASS_FAILED: Key = Key::new("search.results.passFailed");
pub const RESULTS_REFUSED: Key = Key::new("search.results.refused");
pub const RESULTS_RETRY: Key = Key::new("search.results.retry");
pub const RESULTS_INSPECT: Key = Key::new("search.results.inspect");
pub const RESULTS_UNREACHABLE: Key = Key::new("search.results.unreachable");
pub const RESULTS_IDLE: Key = Key::new("search.results.idle");
pub const RESULTS_EMPTY_EVERYWHERE: Key = Key::new("search.results.emptyEverywhere");
pub const RESULTS_EMPTY_WITH_REFUSAL: Key = Key::new("search.results.emptyWithRefusal");
pub const CARD_STARS: Key = Key::new("search.card.stars");
pub const PROVENANCE_INSTALLED: Key = Key::new("search.provenance.installed");
pub const PROVENANCE_DEPENDENCY: Key = Key::new("search.provenance.dependency");
pub const PROVENANCE_COLLECTION: Key = Key::new("search.provenance.collection");
pub const PROVENANCE_SEEN: Key = Key::new("search.provenance.seen");
pub const SHELF_VAULT: Key = Key::new("search.shelf.vault");
pub const SHELF_NETWORK: Key = Key::new("search.shelf.network");
pub const SHELF_SO_FAR: Key = Key::new("search.shelf.soFar");
pub const RESULTS_HOSTS: Key = Key::new("search.results.hosts");
pub const RESULTS_REFUSED_COUNT: Key = Key::new("search.results.refusedCount");
pub const SORT_LABEL: Key = Key::new("search.sort.label");
pub const SORT_RELEVANCE: Key = Key::new("search.sort.relevance");
pub const SORT_NAME: Key = Key::new("search.sort.name");
pub const SORT_STARS: Key = Key::new("search.sort.stars");
pub const NARROW_LABEL: Key = Key::new("search.narrow.label");
pub const NARROW_FACET: Key = Key::new("search.narrow.facet");
pub const NARROW_CLEAR: Key = Key::new("search.narrow.clear");
pub const NARROW_EMPTY: Key = Key::new("search.narrow.empty");
pub const INSPECTOR_TITLE: Key = Key::new("search.inspector.title");
pub const INSPECTOR_DESCRIPTION: Key = Key::new("search.inspector.description");
pub const INSPECTOR_PASS: Key = Key::new("search.inspector.pass");
pub const INSPECTOR_QUERY: Key = Key::new("search.inspector.query");
pub const INSPECTOR_JOB: Key = Key::new("search.inspector.job");
pub const INSPECTOR_COUNTS: Key = Key::new("search.inspector.counts");
pub const INSPECTOR_EXPIRES: Key = Key::new("search.inspector.expires");
pub const INSPECTOR_HOSTS: Key = Key::new("search.inspector.hosts");
pub const INSPECTOR_SETTINGS: Key = Key::new("search.inspector.settings");
pub const INSPECTOR_ASKED: Key = Key::new("search.inspector.asked");
pub const INSPECTOR_RUNNING: Key = Key::new("search.inspector.running");
pub const INSPECTOR_CLOSE: Key = Key::new("search.inspector.close");

pub(super) const MESSAGES: &[(Key, Message)] = &[
    (PLACEHOLDER, Message::Text("Search Arrows")),
    (LABEL, Message::Text("Search")),
    (
        RESULTS_COUNT,
        Message::Plural {
            one: "{count} result",
            other: "{count} results",
        },
    ),
    (RESULTS_SEARCHING, Message::Text("Searching the network…")),
    (
        RESULTS_PASS_FAILED,
        Message::Text("The network search did not finish."),
    ),
    (RESULTS_REFUSED, Message::Text("{host} refused")),
    (RESULTS_RETRY, Message::Text("Retry in {seconds}s")),
    (RESULTS_INSPECT, Message::Text("Inspect")),
    (RESULTS_UNREACHABLE, Message::Text("Quiver is not running")),
    (RESULTS_IDLE, Message::Text("Type to search.")),
    (
        RESULTS_EMPTY_EVERYWHERE,
        Message::Text("Nothing matched, and every host answered."),
    ),
    (
        RESULTS_EMPTY_WITH_REFUSAL,
        Message::Text("Nothing matched on the hosts that answered."),
    ),
    (
        CARD_STARS,
        Message::Plural {
            one: "{count} star",
            other: "{count} stars",
        },
    ),
    (PROVENANCE_INSTALLED, Message::Text("installed")),
    (PROVENANCE_DEPENDENCY, Message::Text("dependency")),
    (PROVENANCE_COLLECTION, Message::Text("collection")),
    (PROVENANCE_SEEN, Message::Text("seen")),
    (SHELF_VAULT, Message::Text("In your vault")),
    (SHELF_NETWORK, Message::Text("From the network")),
    (SHELF_SO_FAR, Message::Text("{count} so far")),
    (
        RESULTS_HOSTS,
        Message::Plural {
            one: "{count} host",
            other: "{count} hosts",
        },
    ),
    (
        RESULTS_REFUSED_COUNT,
        Message::Plural {
            one: "{count} refused",
            other: "{count} refused",
        },
    ),
    (SORT_LABEL, Message::Text("Sort results")),
    (SORT_RELEVANCE, Message::Text("Relevance")),
    (SORT_NAME, Message::Text("Name")),
    (SORT_STARS, Message::Text("Stars")),
    (NARROW_LABEL, Message::Text("Narrow results")),
    (
        NARROW_FACET,
        Message::Plural {
            one: "{value}, {count} result",
            other: "{value}, {count} results",
        },
    ),
    (NARROW_CLEAR, Message::Text("Clear")),
    (
        NARROW_EMPTY,
        Message::Text("Nothing matches that combination."),
    ),
    (INSPECTOR_TITLE, Message::Text("This search")),
    (
        INSPECTOR_DESCRIPTION,
        Message::Text("What the pass did, and what it was allowed to do."),
    ),
    (INSPECTOR_PASS, Message::Text("The pass")),
    (INSPECTOR_QUERY, Message::Text("Query")),
    (INSPECTOR_JOB, Message::Text("Job ID")),
    (
        INSPECTOR_COUNTS,
        Message::Text("Found / verified / skipped"),
    ),
    (INSPECTOR_EXPIRES, Message::Text("Expires at")),
    (INSPECTOR_HOSTS, Message::Text("Hosts")),
    (INSPECTOR_SETTINGS, Message::Text("Settings")),
    (INSPECTOR_ASKED, Message::Text("{count} asked")),
    (
        INSPECTOR_RUNNING,
        Message::Text("The job carries no providers until the pass ends."),
    ),
    (INSPECTOR_CLOSE, Message::Text("Close")),
];
