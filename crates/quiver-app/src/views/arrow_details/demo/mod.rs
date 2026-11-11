//! `--demo` data for the arrow page: a detail for every library row of `crate::demo`, with the three
//! boards' arrows spelled out (Minecraft running with a README, Caddy mid-update, Terraria with an
//! update and no banner or README). Used instead of the daemon reads when the app runs `--demo`.

use std::collections::BTreeMap;

use quiver_state::domain::{
    ActiveRun, ArrowChannel, ArrowCredit, ArrowDependency, ArrowDetail, ArrowEntry, ArrowLifecycle,
    ArrowMedia, ArrowMethod, ArrowPort, ArrowRequirement, ArrowState, ArrowStepDefinition,
    ArrowTarget, ArrowVariable, AvailableVersion, ChannelKind, DependencyType, LastReturnDetail,
    Outcome, Overridable, PortProtocol, SelectorKind, SignalKind, StepBase, StepProgress,
    StepStatus, VariableType,
};
use quiver_state::namespace::bare_namespace;

mod arrows;

const MINECRAFT_README: &str = "## About\n\nMinecraft Server packages a vanilla Java Edition server with sane defaults, automatic world backups, and RCON wired up out of the box. It targets Java 21 and runs on macOS, Linux and Windows.\n\n## What you get\n\n- Vanilla server jar, pinned to this arrow's exact ref\n- World auto-backup before every stop\n- RCON enabled on a local port unless overridden\n- Clean shutdown via a graceful stop signal, not a hard kill\n\n## Lifecycle\n\n```mermaid\nstateDiagram-v2\n    [*] --> Ready: install\n    Ready --> Starting: start\n    Starting --> Running\n    Running --> Stopping: stop\n    Stopping --> Ready\n    Running --> Running: backup\n```\n\n## Getting started\n\n- [ ] Set `MOTD` and `DIFFICULTY` in Settings\n- [ ] Run **Start** -- first boot generates the world and can take a minute or two\n- [ ] Watch the Activity tab for progress\n";

/// `QUIVER_DEMO_README_KB=<kb>`: the Minecraft README replaced by about that many KB of mixed Markdown (headings,
/// paragraphs, lists, code, a table), to measure rendering of a huge README.
fn readme_from_environment() -> Option<String> {
    let kb: usize = std::env::var("QUIVER_DEMO_README_KB").ok()?.parse().ok()?;
    let mut out = String::from("# Huge README\n\n");
    let mut n = 0;
    while out.len() < kb * 1024 {
        n += 1;
        out.push_str(&format!(
            "## Section {n}\n\nParagraph with **bold**, *italic*, `code` and a [link](https://example.test/{n}). Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n- first item of {n}\n- second item of {n}\n- third item of {n}\n\n```sh\nquiver arrow run section-{n}\n```\n\n| key | value |\n| --- | --- |\n| section | {n} |\n\n"
        ));
    }
    Some(out)
}

fn run(
    title: &str,
    command: &str,
    timeout: &str,
) -> ArrowStepDefinition {
    ArrowStepDefinition::Run {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        command: Overridable::Plain(command.into()),
        elevated: Overridable::Plain(false),
        timeout: Overridable::Plain(timeout.into()),
    }
}

fn fetch(
    title: &str,
    url: &str,
    to: &str,
    timeout: &str,
) -> ArrowStepDefinition {
    ArrowStepDefinition::Fetch {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        url: Overridable::Plain(url.into()),
        to: Overridable::Plain(to.into()),
        checksum: Overridable::Plain(String::new()),
        timeout: Overridable::Plain(timeout.into()),
    }
}

fn signal(
    title: &str,
    signal: SignalKind,
    timeout: &str,
) -> ArrowStepDefinition {
    ArrowStepDefinition::Signal {
        base: StepBase {
            title: title.into(),
            exit_on_failure: None,
        },
        signal: Overridable::Plain(signal),
        timeout: Overridable::Plain(timeout.into()),
    }
}

const ARCHIVE: &str =
    "https://github.com/{{namespace}}/releases/download/{{ref}}/{{name}}-{{os}}-{{arch}}.tar.gz";

fn install_steps() -> Vec<ArrowStepDefinition> {
    vec![
        fetch(
            "Resolve manifest",
            "https://index.quiver.dev/v0/resolve/{{namespace}}",
            "{{workdir}}/manifest.json",
            "15s",
        ),
        fetch(
            "Fetch archive",
            ARCHIVE,
            "{{workdir}}/archive.tar.gz",
            "120s",
        ),
        run(
            "Verify checksum",
            "sha256sum -c archive.tar.gz.sha256",
            "10s",
        ),
        run(
            "Unpack to workdir",
            "tar -xzf archive.tar.gz -C {{workdir}}",
            "30s",
        ),
        run(
            "Write runtime config",
            "quiver-init --workdir {{workdir}} --vars {{variables}}",
            "5s",
        ),
    ]
}

fn start_steps() -> Vec<ArrowStepDefinition> {
    vec![
        run(
            "Bind ports",
            "{{workdir}}/bin/{{name}} --bind {{ports}}",
            "10s",
        ),
        run(
            "Start process",
            "{{workdir}}/bin/{{name}} --config {{workdir}}/config.yaml",
            "30s",
        ),
        run(
            "Await readiness",
            "curl --retry 10 --retry-delay 2 -sf http://localhost:{{port}}/healthz",
            "30s",
        ),
    ]
}

fn update_steps() -> Vec<ArrowStepDefinition> {
    vec![
        fetch(
            "Fetch new version",
            ARCHIVE,
            "{{workdir}}/update.tar.gz",
            "120s",
        ),
        run(
            "Verify checksum",
            "sha256sum -c update.tar.gz.sha256",
            "10s",
        ),
        signal("Stop previous version", SignalKind::Graceful, "15s"),
        run(
            "Migrate config",
            "quiver-migrate --workdir {{workdir}} --to {{ref}}",
            "10s",
        ),
    ]
}

fn stop_steps() -> Vec<ArrowStepDefinition> {
    vec![
        signal("Signal process", SignalKind::Graceful, "10s"),
        run(
            "Await exit",
            "while kill -0 {{pid}} 2>/dev/null; do sleep 1; done",
            "30s",
        ),
    ]
}

fn uninstall_steps() -> Vec<ArrowStepDefinition> {
    vec![
        signal("Stop process", SignalKind::Kill, "5s"),
        run("Remove workdir", "rm -rf {{workdir}}", "10s"),
        run("Prune runtime config", "rm -rf {{config_dir}}", "5s"),
    ]
}

fn lifecycle() -> ArrowLifecycle {
    ArrowLifecycle {
        install: install_steps(),
        update: update_steps(),
        execute: start_steps(),
        stop: stop_steps(),
        uninstall: uninstall_steps(),
    }
}

fn method(
    name: &str,
    description: &str,
    available_in: &[ArrowState],
    steps: Vec<ArrowStepDefinition>,
) -> ArrowMethod {
    ArrowMethod {
        name: name.into(),
        description: description.into(),
        available_in: available_in.to_vec(),
        steps,
    }
}

fn default_methods() -> Vec<ArrowMethod> {
    vec![
        method(
            "start",
            "Start the service",
            &[ArrowState::Ready],
            start_steps(),
        ),
        method(
            "stop",
            "Stop the service",
            &[ArrowState::Running],
            stop_steps(),
        ),
    ]
}

fn target(
    platform: &str,
    requirement: (f64, f64, f64),
    methods: Vec<ArrowMethod>,
) -> ArrowTarget {
    ArrowTarget {
        platform: platform.into(),
        requirement: ArrowRequirement {
            cpu_cores: requirement.0,
            memory_gb: requirement.1,
            disk_gb: requirement.2,
        },
        lifecycle: lifecycle(),
        methods,
    }
}

fn variable(
    name: &str,
    description: &str,
    var_type: VariableType,
    default: Option<&str>,
) -> ArrowVariable {
    ArrowVariable {
        name: name.into(),
        description: description.into(),
        var_type,
        default: default.map(Into::into),
        values: None,
        min: None,
        max: None,
        sensitive: None,
    }
}

fn secret(
    name: &str,
    description: &str,
) -> ArrowVariable {
    ArrowVariable {
        sensitive: Some(true),
        ..variable(name, description, VariableType::String, None)
    }
}

fn port(
    name: &str,
    protocol: PortProtocol,
    default: u32,
    required: bool,
) -> ArrowPort {
    ArrowPort {
        name: name.into(),
        protocol,
        default,
        required,
    }
}

fn progress(
    steps: &[ArrowStepDefinition],
    statuses: &[StepStatus],
) -> Vec<StepProgress> {
    statuses
        .iter()
        .zip(steps)
        .enumerate()
        .map(|(i, (status, def))| {
            let (title, step_type) = match def {
                ArrowStepDefinition::Run { base, .. } => (&base.title, "run"),
                ArrowStepDefinition::Fetch { base, .. } => (&base.title, "fetch"),
                ArrowStepDefinition::Signal { base, .. } => (&base.title, "signal"),
                ArrowStepDefinition::Dependencies { base }
                | ArrowStepDefinition::Extract { base }
                | ArrowStepDefinition::Portable { base }
                | ArrowStepDefinition::Expose { base }
                | ArrowStepDefinition::Unexpose { base } => (&base.title, "exec"),
            };
            StepProgress {
                index: i as u32,
                title: title.clone(),
                status: *status,
                step_type: step_type.into(),
                error: None,
                note: None,
            }
        })
        .collect()
}

fn active(
    method: &str,
    pid: Option<u32>,
    steps: &[ArrowStepDefinition],
    statuses: &[StepStatus],
) -> ActiveRun {
    ActiveRun {
        method: method.into(),
        pid,
        variables: BTreeMap::new(),
        steps: progress(steps, statuses),
        surface: None,
    }
}

fn credit(name: &str) -> ArrowCredit {
    ArrowCredit {
        name: name.into(),
        email: None,
        url: None,
    }
}

fn generic(
    entry: &ArrowEntry,
    targets: Vec<ArrowTarget>,
) -> ArrowDetail {
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
        maintainers: vec![credit("rabbyte")],
        credits: vec![],
        netbridge: vec![],
        variables: vec![],
        targets,
        state: entry.state,
        user_installed: true,
        installed_at: Some("2026-07-14T09:20:00Z".into()),
        selector: entry.version.clone(),
        selector_kind: SelectorKind::Pin,
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

fn standard(entry: &ArrowEntry) -> ArrowDetail {
    generic(
        entry,
        vec![target(
            &quiver_platform::platform_key(),
            (2., 4., 10.),
            default_methods(),
        )],
    )
}

fn last_return(
    method: &str,
    outcome: Outcome,
    steps: Vec<StepProgress>,
) -> LastReturnDetail {
    LastReturnDetail {
        method: method.into(),
        outcome,
        variables: BTreeMap::new(),
        steps,
    }
}

fn all_completed(steps: &[ArrowStepDefinition]) -> Vec<StepProgress> {
    progress(steps, &vec![StepStatus::Completed; steps.len()])
}

fn channels() -> Vec<ArrowChannel> {
    vec![
        ArrowChannel {
            name: "stable".into(),
            kind: ChannelKind::Ordered,
            latest: "v1.21.4".into(),
            count: Some(3),
            members: Some(vec!["v1.21.4".into(), "v1.21.1".into(), "v1.20.6".into()]),
        },
        ArrowChannel {
            name: "nightly".into(),
            kind: ChannelKind::Pointer,
            latest: "nightly-latest".into(),
            count: None,
            members: None,
        },
    ]
}

/// The detail the demo shows for a library row: the old mock world's arrow of the same slug.
pub fn detail(entry: &ArrowEntry) -> ArrowDetail {
    match bare_namespace(&entry.namespace).rsplit('/').next() {
        Some("minecraft") => arrows::minecraft(entry),
        Some("valheim") => arrows::valheim(entry),
        Some("factorio") => arrows::factorio(entry),
        Some("terraria") => arrows::terraria(entry),
        Some("pixelmon-assets") => arrows::pixelmon(entry),
        Some("postgres") => arrows::postgres(entry),
        Some("redis") => arrows::redis(entry),
        Some("caddy") => arrows::caddy(entry),
        Some("grafana") => arrows::grafana(entry),
        Some("nats") => arrows::nats(entry),
        _ => standard(entry),
    }
}
