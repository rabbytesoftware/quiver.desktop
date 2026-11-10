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
    model.services.connections.set_from_event(
        vec![
            local_connection(),
            remote("conn-1", "Studio server", "http://192.168.1.40:7070"),
            remote("conn-2", "Build box", "https://build.example.net:7070"),
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

/// What the console dock shows in the demo.
pub fn console_entries() -> Vec<NewEntry> {
    let out = |text: &str| NewEntry::Out {
        stream: Stream::Stdout,
        text: text.into(),
    };
    vec![
        NewEntry::Raw("$ arrow list".into()),
        out("github.com/rabbyte/grafana   running   v11.4.0"),
        out("github.com/rabbyte/caddy     stopped   v2.8.4"),
        NewEntry::Note {
            tone: Tone::Info,
            note: Note::Text("2 arrows".into()),
        },
    ]
}

pub const SUPPORT: Support = Support::Supported;
