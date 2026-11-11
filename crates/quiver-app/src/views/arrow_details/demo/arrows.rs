//! The seeded detail of each demo arrow, matching the old mock's normal scenario.

use super::*;

pub(super) fn minecraft(entry: &ArrowEntry) -> ArrowDetail {
    let methods = vec![
        method(
            "start",
            "Start the server",
            &[ArrowState::Ready],
            start_steps(),
        ),
        method(
            "stop",
            "Stop the server",
            &[ArrowState::Running],
            vec![
                run("Save world", "rcon-cli save-all", "30s"),
                signal("Signal process", SignalKind::Graceful, "10s"),
                run(
                    "Await exit",
                    "while kill -0 {{pid}} 2>/dev/null; do sleep 1; done",
                    "30s",
                ),
            ],
        ),
        method(
            "backup",
            "Snapshot the world directory",
            &[ArrowState::Ready, ArrowState::Running],
            vec![
                run("Freeze world", "rcon-cli save-off", "30s"),
                run(
                    "Copy region files",
                    "cp -r {{workdir}}/world {{backup_dir}}/world-{{timestamp}}",
                    "30s",
                ),
                run(
                    "Compress",
                    "tar -czf {{backup_dir}}/world-{{timestamp}}.tar.gz {{backup_dir}}/world-{{timestamp}}",
                    "30s",
                ),
            ],
        ),
        method(
            "rcon",
            "Open a remote console session",
            &[ArrowState::Running],
            vec![
                run("Connect", "nc {{host}} 25575", "30s"),
                run(
                    "Authenticate",
                    "rcon-cli --password {{RCON_PASSWORD}} login",
                    "30s",
                ),
            ],
        ),
    ];
    let mut d = generic(
        entry,
        vec![target(
            &quiver_platform::platform_key(),
            (4., 8., 40.),
            methods,
        )],
    );
    d.readme = Some(readme_from_environment().unwrap_or_else(|| MINECRAFT_README.into()));
    d.channels = channels();
    d.netbridge = vec![
        port("game", PortProtocol::Tcp, 25565, true),
        port("rcon", PortProtocol::Tcp, 25575, false),
    ];
    d.dependencies = vec![ArrowDependency {
        namespace: "github.com/rabbyte/nats@v2.10.24".into(),
        dep_type: DependencyType::Tool,
    }];
    let mut max_players = variable(
        "MAX_PLAYERS",
        "Maximum concurrent players",
        VariableType::Number,
        Some("20"),
    );
    max_players.min = Some(1.);
    max_players.max = Some(200.);
    let mut difficulty = variable(
        "DIFFICULTY",
        "World difficulty",
        VariableType::Select,
        Some("normal"),
    );
    difficulty.values = Some(
        ["peaceful", "easy", "normal", "hard"]
            .map(String::from)
            .to_vec(),
    );
    d.variables = vec![
        variable(
            "MOTD",
            "Message shown in the server list",
            VariableType::String,
            Some("A Quiver server"),
        ),
        max_players,
        difficulty,
        variable(
            "ONLINE_MODE",
            "Verify players against Mojang auth",
            VariableType::Boolean,
            Some("true"),
        ),
        secret("RCON_PASSWORD", "Password for remote console access"),
    ];
    let mut run = active(
        "start",
        Some(48213),
        &start_steps(),
        &[StepStatus::Completed; 3],
    );
    run.variables = BTreeMap::from([
        ("MOTD".into(), "A Quiver server".into()),
        ("MAX_PLAYERS".into(), "20".into()),
        ("DIFFICULTY".into(), "normal".into()),
        ("ONLINE_MODE".into(), "true".into()),
    ]);
    d.active_run = Some(run);
    d
}

pub(super) fn valheim(entry: &ArrowEntry) -> ArrowDetail {
    let mut d = generic(
        entry,
        vec![target(
            "linux/amd64",
            (4., 8., 20.),
            vec![method(
                "start",
                "Start the server",
                &[ArrowState::Ready],
                start_steps(),
            )],
        )],
    );
    d.netbridge = vec![port("game", PortProtocol::Udp, 2456, true)];
    d.variables = vec![
        variable(
            "WORLD_NAME",
            "Name of the world to load or create",
            VariableType::String,
            Some("Dedicated"),
        ),
        secret("SERVER_PASSWORD", "Password required to join"),
    ];
    d
}

pub(super) fn factorio(entry: &ArrowEntry) -> ArrowDetail {
    let mut d = standard(entry);
    let steps = install_steps();
    let mut failed = progress(
        &steps,
        &[
            StepStatus::Completed,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Pending,
            StepStatus::Pending,
        ],
    );
    failed[2].error = Some("checksum mismatch: expected sha256:9f2c…, got sha256:41ab…".into());
    d.last_return = Some(last_return("install", Outcome::Failed, failed));
    d
}

/// Two platforms whose OS is not this machine's.
fn elsewhere() -> Vec<&'static str> {
    let host = quiver_platform::platform_key();
    let host_os = host.split('/').next().unwrap_or_default().to_owned();
    ["linux/amd64", "windows/amd64", "darwin/arm64"]
        .into_iter()
        .filter(|p| !p.starts_with(&format!("{host_os}/")))
        .take(2)
        .collect()
}

pub(super) fn terraria(entry: &ArrowEntry) -> ArrowDetail {
    let mut d = generic(
        entry,
        // The old mock's "not built for this machine" arrow: two targets, neither for the host's OS (the
        // board draws Linux · Windows on a Mac), so the red badge shows on every platform.
        elsewhere()
            .into_iter()
            .map(|platform| target(platform, (2., 4., 10.), default_methods()))
            .collect(),
    );
    d.netbridge = vec![port("game", PortProtocol::Tcp, 7777, true)];
    d.available = Some(AvailableVersion {
        reference: "v1.4.5.0".into(),
        commit: "7c0de5a1b2c3d4e5f60718293a4b5c6d7e8f9012".into(),
    });
    d.outdated = true;
    d
}

pub(super) fn pixelmon(entry: &ArrowEntry) -> ArrowDetail {
    let mut d = generic(
        entry,
        vec![target(
            &quiver_platform::platform_key(),
            (1., 1., 6.),
            vec![],
        )],
    );
    d.last_return = Some(last_return(
        "install",
        Outcome::Success,
        all_completed(&install_steps()),
    ));
    d
}

pub(super) fn postgres(entry: &ArrowEntry) -> ArrowDetail {
    let mut d = standard(entry);
    d.netbridge = vec![port("sql", PortProtocol::Tcp, 5432, true)];
    d.variables = vec![
        variable(
            "POSTGRES_USER",
            "Superuser name",
            VariableType::String,
            Some("quiver"),
        ),
        secret("POSTGRES_PASSWORD", "Superuser password"),
    ];
    d.last_return = Some(last_return(
        "install",
        Outcome::Success,
        all_completed(&install_steps()),
    ));
    d
}

pub(super) fn redis(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = standard(entry);
    d.active_run = Some(active(
        "install",
        None,
        &install_steps(),
        &[Completed, Running, Pending, Pending, Pending],
    ));
    d
}

pub(super) fn caddy(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = standard(entry);
    d.active_run = Some(active(
        "update",
        None,
        &update_steps(),
        &[Completed, Running, Pending, Pending],
    ));
    d
}

pub(super) fn grafana(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = standard(entry);
    d.active_run = Some(active("stop", None, &stop_steps(), &[Running, Pending]));
    d
}

pub(super) fn nats(entry: &ArrowEntry) -> ArrowDetail {
    use StepStatus::*;
    let mut d = standard(entry);
    d.active_run = Some(active("stop", None, &stop_steps(), &[Completed, Running]));
    d.dependents = vec!["github.com/rabbyte/minecraft@v1.21.4".into()];
    d
}
