//! `--demo` data for the arrow page: a detail for every library row of `crate::demo`, with the three
//! boards' arrows spelled out (Minecraft running with a README, Caddy mid-update, Terraria with an
//! update and no banner or README). Used instead of the daemon reads when the app runs `--demo`.

use std::collections::BTreeMap;

use quiver_state::domain::{
    ActiveRun, ArrowCredit, ArrowDetail, ArrowEntry, ArrowLifecycle, ArrowMedia, ArrowMethod,
    ArrowPort, ArrowRequirement, ArrowState, ArrowStepDefinition, ArrowTarget, ArrowVariable,
    AvailableVersion, Overridable, PortProtocol, SelectorKind, StepBase, StepProgress, StepStatus,
    VariableType,
};
use quiver_state::namespace::bare_namespace;

const MINECRAFT_README: &str = "# About Minecraft Server\n\nA vanilla Minecraft dedicated server with automatic world backups, packaged as a Quiver arrow.\n\n## Getting started\n\nStart the arrow, then connect to your machine’s address on the port listed under Network. Worlds are stored in the arrow’s data folder and backed up on every stop.\n\n## Backups\n\nRun the backup method from Methods at any time. Restoring is a manual copy for now.\n";

fn run(title: &str, command: &str) -> ArrowStepDefinition {
    ArrowStepDefinition::Run {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        command: Overridable::Plain(command.into()),
        elevated: Overridable::Plain(false),
        timeout: Overridable::Plain(String::new()),
    }
}

fn fetch(title: &str, url: &str) -> ArrowStepDefinition {
    ArrowStepDefinition::Fetch {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        url: Overridable::Plain(url.into()),
        to: Overridable::Plain("${ARROW_DIR}/download".into()),
        checksum: Overridable::Plain(String::new()),
        timeout: Overridable::Plain(String::new()),
    }
}

fn simple(kind: &str, title: &str) -> ArrowStepDefinition {
    let base = StepBase {
        title: title.into(),
        exit_on_failure: None,
    };
    match kind {
        "extract" => ArrowStepDefinition::Extract { base },
        _ => ArrowStepDefinition::Dependencies { base },
    }
}

fn four_steps() -> Vec<ArrowStepDefinition> {
    vec![
        fetch("Download", "https://example.test/release.tgz"),
        run("Verify checksum", "sha256sum -c release.sha256"),
        simple("extract", "Extract"),
        run("Restart", "systemctl restart service"),
    ]
}

fn target(platform: &str, lifecycle: ArrowLifecycle, methods: Vec<ArrowMethod>) -> ArrowTarget {
    ArrowTarget {
        platform: platform.into(),
        requirement: ArrowRequirement {
            cpu_cores: 2.0,
            memory_gb: 4.0,
            disk_gb: 10.0,
        },
        lifecycle,
        methods,
    }
}

fn variable(name: &str, description: &str, var_type: VariableType, default: &str) -> ArrowVariable {
    ArrowVariable {
        name: name.into(),
        description: description.into(),
        var_type,
        default: Some(default.into()),
        values: None,
        min: None,
        max: None,
        sensitive: None,
    }
}

fn progress(statuses: &[StepStatus]) -> Vec<StepProgress> {
    let titles = ["Download", "Verify checksum", "Extract", "Restart"];
    statuses
        .iter()
        .enumerate()
        .map(|(i, status)| StepProgress {
            index: i as u32,
            title: titles[i % 4].into(),
            status: *status,
            step_type: if i == 0 { "fetch" } else { "run" }.into(),
            error: None,
            note: None,
        })
        .collect()
}

fn active(method: &str, statuses: &[StepStatus]) -> ActiveRun {
    ActiveRun {
        method: method.into(),
        pid: None,
        variables: BTreeMap::new(),
        steps: progress(statuses),
        surface: None,
    }
}

fn generic(entry: &ArrowEntry, targets: Vec<ArrowTarget>) -> ArrowDetail {
    ArrowDetail {
        origin: None,
        confidence: None,
        warnings: vec![],
        namespace: entry.namespace.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        license: "MIT".into(),
        url: bare_namespace(&entry.namespace).into(),
        tags: entry.tags.clone(),
        media: ArrowMedia {
            icon: None,
            banner: entry.banner.clone(),
        },
        maintainers: vec![ArrowCredit {
            name: "Rabbyte".into(),
            email: None,
            url: None,
        }],
        credits: vec![],
        netbridge: vec![ArrowPort {
            name: "game".into(),
            protocol: PortProtocol::Tcp,
            default: 7777,
            required: true,
        }],
        variables: vec![],
        targets,
        state: entry.state,
        user_installed: true,
        installed_at: None,
        selector: "stable".into(),
        selector_kind: SelectorKind::Channel,
        resolved_ref: entry.version.clone(),
        installed_commit: String::new(),
        available: None,
        outdated: false,
        active_run: entry.active_run.clone(),
        last_return: None,
        channels: vec![],
        readme: None,
        dependencies: vec![],
        dependents: vec![],
    }
}

fn standard_lifecycle() -> ArrowLifecycle {
    ArrowLifecycle {
        install: four_steps(),
        update: four_steps(),
        execute: vec![run("Run", "./server --port ${PORT}")],
        stop: vec![ArrowStepDefinition::Signal {
            base: StepBase {
                title: "Signal".into(),
                exit_on_failure: None,
            },
            signal: Overridable::Plain(quiver_state::domain::SignalKind::Graceful),
            timeout: Overridable::Plain(String::new()),
        }],
        uninstall: vec![
            run("Remove files", "rm -rf ${ARROW_DIR}"),
            simple("deps", "Release dependencies"),
        ],
    }
}

fn minecraft(entry: &ArrowEntry) -> ArrowDetail {
    let methods = vec![
        ArrowMethod {
            name: "backup".into(),
            description: "Snapshot the world folder".into(),
            available_in: vec![ArrowState::Running],
            steps: vec![
                run("stop-saving", "save-off"),
                run("archive", "tar -czf worlds.tgz world/"),
                run("resume", "save-on"),
            ],
        },
        ArrowMethod {
            name: "rcon".into(),
            description: "Open a remote console".into(),
            available_in: vec![ArrowState::Running],
            steps: vec![run("connect", "rcon-cli"), run("prompt", "rcon-cli --repl")],
        },
    ];
    let mut d = generic(
        entry,
        vec![target("linux/amd64", standard_lifecycle(), methods)],
    );
    d.readme = Some(MINECRAFT_README.into());
    d.variables = vec![
        variable(
            "PORT",
            "Port the server listens on.",
            VariableType::Number,
            "7777",
        ),
        variable(
            "MOTD",
            "Message shown in the server list.",
            VariableType::String,
            "A Quiver server",
        ),
        variable(
            "HARDCORE",
            "Permanent death.",
            VariableType::Boolean,
            "false",
        ),
    ];
    d
}

fn caddy(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = generic(
        entry,
        vec![target("linux/amd64", standard_lifecycle(), vec![])],
    );
    d.tags = vec!["network".into(), "service".into()];
    d.variables = vec![
        variable("HTTP_PORT", "Plain HTTP port.", VariableType::Number, "80"),
        variable(
            "DOMAIN",
            "Site address to serve.",
            VariableType::String,
            "localhost",
        ),
    ];
    d.active_run = Some(active("update", &[Completed, Running, Pending, Pending]));
    d
}

fn terraria(entry: &ArrowEntry) -> ArrowDetail {
    let mut lifecycle = standard_lifecycle();
    lifecycle.uninstall.clear();
    let mut d = generic(
        entry,
        vec![
            target("linux/amd64", lifecycle.clone(), vec![]),
            target("windows/amd64", lifecycle.clone(), vec![]),
            target("darwin/arm64", lifecycle, vec![]),
        ],
    );
    d.tags = vec!["game".into(), "server".into()];
    d.variables = vec![
        variable("WORLD", "World file name.", VariableType::String, "world1"),
        variable(
            "PORT",
            "Port the server listens on.",
            VariableType::Number,
            "7777",
        ),
        variable("PASSWORD", "Join password.", VariableType::String, ""),
    ];
    d.available = Some(AvailableVersion {
        reference: "v1.4.5.0".into(),
        commit: String::new(),
    });
    d.outdated = true;
    d
}

fn redis(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = generic(
        entry,
        vec![target("linux/amd64", standard_lifecycle(), vec![])],
    );
    d.active_run = Some(active("install", &[Completed, Completed, Running, Pending]));
    d
}

/// The detail the demo shows for a library row.
pub fn detail(entry: &ArrowEntry) -> ArrowDetail {
    match bare_namespace(&entry.namespace).rsplit('/').next() {
        Some("minecraft") => minecraft(entry),
        Some("caddy") => caddy(entry),
        Some("terraria") => terraria(entry),
        Some("redis") => redis(entry),
        _ => generic(
            entry,
            vec![target("linux/amd64", standard_lifecycle(), vec![])],
        ),
    }
}
