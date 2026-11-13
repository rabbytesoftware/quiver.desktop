//! Demo data for Remote, the Arrow-app pane and the console: two saved connections, a running app whose
//! page answers, and a few console lines.

use quiver_state::console::{CoreVersions, NewEntry, Note, Stream, Support, Tone};
use quiver_state::domain::{
    local_connection, ActiveRun, ArrowState, ArrowSurface, ConnectionConfig, ConnectionKind,
    ConnectionStatus, RuntimeUpdate, SurfaceMode,
};

use crate::model::AppModel;

/// The namespace the `arrow-app` demo route opens.
pub const APP_NAMESPACE: &str = "github.com/rabbyte/grafana";

fn remote(
    id: &str,
    name: &str,
    url: &str,
) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: name.into(),
        kind: ConnectionKind::Remote,
        url: Some(url.into()),
        api_version: "v0".into(),
    }
}

pub fn seed(model: &AppModel) {
    let mut local = local_connection();
    local.name = "Mock · Normal".into();
    model.services.connections.set_from_event(
        vec![
            local,
            remote(
                "mock-home-lab",
                "Mock · Home Lab",
                "http://mock.home-lab.local:7420",
            ),
        ],
        "local",
    );
    model.services.status.set_status(ConnectionStatus::Ready);
    model.core_versions.set(Some(CoreVersions {
        version: "nightly".into(),
        channel: "nightly".into(),
        built_at: "2026-10-11T09:30:00Z".into(),
        ..CoreVersions::default()
    }));
    model.services.arrows.apply_runtime_update(RuntimeUpdate {
        namespace: APP_NAMESPACE.into(),
        state: ArrowState::Running,
        active_run: Some(ActiveRun {
            method: "run".into(),
            pid: Some(4242),
            variables: Default::default(),
            steps: vec![],
            surface: Some(ArrowSurface {
                mode: SurfaceMode::Listen,
                path: "/".into(),
                ready: true,
            }),
        }),
        last_return: None,
    });
}

/// What the console dock shows in the demo: the old mock daemon's seed log, then a typed command.
pub fn console_entries() -> Vec<NewEntry> {
    use quiver_state::console::frames::{field_kind, LogField, LogLevel, LogRecord};
    use serde_json::json;

    let log = |seq: u64,
               time: &str,
               level: LogLevel,
               component: &str,
               msg: &str,
               fields: serde_json::Value| {
        let fields = fields
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(key, value)| LogField {
                        key: key.clone(),
                        value: match value {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        },
                        kind: field_kind(key, value),
                    })
                    .collect()
            })
            .unwrap_or_default();
        NewEntry::Log(LogRecord {
            seq,
            iso: format!("2026-10-11T09:30:{time}Z"),
            level,
            component: component.into(),
            msg: msg.into(),
            fields,
        })
    };
    let out = |text: &str| NewEntry::Out {
        stream: Stream::Stdout,
        text: text.into(),
    };
    vec![
        log(
            1,
            "01.204",
            LogLevel::Info,
            "daemon",
            "daemon listening",
            json!({"addr": "~/.quiver/quiver.sock", "pid": 48213}),
        ),
        log(
            2,
            "01.531",
            LogLevel::Info,
            "catalog",
            "catalog loaded",
            json!({"arrows": 5, "took": "38ms"}),
        ),
        log(
            3,
            "01.802",
            LogLevel::Debug,
            "topic",
            "subscribed",
            json!({"topic": "quiver-arrow", "peers": 2}),
        ),
        log(
            4,
            "02.116",
            LogLevel::Info,
            "arrow",
            "registered",
            json!({"ns": "github.com/rabbytesoftware/quiver.desktop", "ref": "nightly-rolling", "auto": true}),
        ),
        log(
            5,
            "03.470",
            LogLevel::Warn,
            "release",
            "channel lookup slow",
            json!({"ns": "github.com/char2cs/crowbar", "took": "1.8s", "retry": 1}),
        ),
        log(
            6,
            "04.009",
            LogLevel::Info,
            "arrow",
            "update available",
            json!({"ns": "github.com/example/tidepool", "from": "v1.1.4", "to": "v1.2.0"}),
        ),
        log(
            7,
            "04.388",
            LogLevel::Error,
            "release",
            "checksum mismatch",
            json!({"ns": "github.com/example/wavelength", "want": "sha256:9f2c…e1", "got": "sha256:41ab…07", "err": "asset rejected"}),
        ),
        log(
            8,
            "04.391",
            LogLevel::Info,
            "arrow",
            "install rolled back",
            json!({"ns": "github.com/example/wavelength", "took": "3ms"}),
        ),
        NewEntry::Cmd {
            text: "arrow list".into(),
            at: 1_791_711_005_000,
        },
        out("github.com/rabbyte/grafana   running   v11.4.0"),
        out("github.com/rabbyte/caddy     stopped   v2.8.4"),
        NewEntry::Note {
            tone: Tone::Info,
            note: Note::Text("2 arrows".into()),
        },
    ]
}

pub const SUPPORT: Support = Support::Supported;
