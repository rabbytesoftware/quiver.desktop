use super::*;
use crate::console::versions::CONSOLE_FEATURE;
use serde_json::json;

fn supported() -> CoreVersions {
    CoreVersions {
        version: "nightly-latest".into(),
        features: vec![CONSOLE_FEATURE.into()],
        ..CoreVersions::default()
    }
}

fn legacy() -> CoreVersions {
    CoreVersions {
        version: "0.1.0".into(),
        ..CoreVersions::default()
    }
}

fn table() -> Vec<ConsoleCommand> {
    vec![
        ConsoleCommand {
            path: vec!["install".into()],
            short: "install an arrow".into(),
            usage: "install <namespace>".into(),
        },
        ConsoleCommand {
            path: vec!["list".into()],
            short: "list arrows".into(),
            usage: "list".into(),
        },
    ]
}

/// Carries out the effects the way an owner would, recording what matters.
#[derive(Default)]
struct Rig {
    c: ConsoleController,
    sockets: Vec<(u64, String, bool)>,
    execs: Vec<(u64, String, bool)>,
    versions_asked: usize,
    commands_asked: usize,
    timers: Vec<TimerKind>,
}

impl Rig {
    fn apply(&mut self, effects: Vec<ConsoleEffect>) {
        for e in effects {
            match e {
                ConsoleEffect::FetchVersions { .. } => self.versions_asked += 1,
                ConsoleEffect::FetchCommands { .. } => self.commands_asked += 1,
                ConsoleEffect::OpenStream { path, attempt } => {
                    self.sockets.push((attempt, path, false))
                }
                ConsoleEffect::CloseStream { attempt } => {
                    if let Some(s) = self.sockets.iter_mut().find(|s| s.0 == attempt) {
                        s.2 = true;
                    }
                }
                ConsoleEffect::StartTimer { kind, .. } => self.timers.push(kind),
                ConsoleEffect::CancelTimer { kind } => self.timers.retain(|t| *t != kind),
                ConsoleEffect::StartExec { run, line } => self.execs.push((run, line, false)),
                ConsoleEffect::CancelExec { run } => {
                    if let Some(x) = self.execs.iter_mut().find(|x| x.0 == run) {
                        x.2 = true;
                    }
                }
            }
        }
    }

    fn sync(&mut self, id: &str, ready: bool) {
        let fx = self.c.sync(id, ready);
        self.apply(fx);
    }

    /// `/versions` answers, then the command table (when it was asked for).
    fn learn(&mut self, versions: CoreVersions) {
        let epoch = self.c.epoch;
        let fx = self.c.on_versions(epoch, Some(versions));
        let wants_commands = fx
            .iter()
            .any(|e| matches!(e, ConsoleEffect::FetchCommands { .. }));
        self.apply(fx);
        if wants_commands {
            self.c.on_commands(epoch, Some(table()), false);
        }
    }

    fn open(&mut self, open: bool) {
        let fx = self.c.set_open(open);
        self.apply(fx);
    }

    fn deliver(&mut self, socket: usize, frame: serde_json::Value) {
        let attempt = self.sockets[socket].0;
        let fx = self.c.stream_message(attempt, &frame.to_string());
        self.apply(fx);
    }

    fn flush(&mut self) {
        let fx = self.c.timer_elapsed(TimerKind::Flush);
        self.apply(fx);
    }

    fn drop_socket(&mut self, socket: usize) {
        let attempt = self.sockets[socket].0;
        let fx = self.c.stream_closed(attempt);
        self.apply(fx);
    }

    fn backoff(&mut self) {
        let fx = self.c.timer_elapsed(TimerKind::Backoff);
        self.apply(fx);
    }

    fn exec(&mut self, run: usize, frame: serde_json::Value) {
        self.c.exec_frame(self.execs[run].0, &frame.to_string());
    }

    fn submit(&mut self, line: &str) {
        let fx = self.c.submit(line, 1_791_122_529_000);
        self.apply(fx);
    }

    fn texts(&self) -> Vec<String> {
        self.c
            .state
            .entries
            .iter()
            .map(|e| match &e.body {
                NewEntry::Out { text, .. } => text.clone(),
                NewEntry::Cmd { text, .. } => format!("$ {text}"),
                NewEntry::Raw(t) => format!("raw:{t}"),
                NewEntry::Note { note, .. } => format!("{note:?}"),
                NewEntry::Log(r) => r.msg.clone(),
            })
            .collect()
    }
}

fn log(seq: u64, msg: &str) -> serde_json::Value {
    json!({"type":"log","seq":seq,"msg":msg,"level":"info"})
}

fn ready(seq: u64) -> serde_json::Value {
    json!({"type":"ready","seq":seq})
}

fn out(data: &str) -> serde_json::Value {
    json!({"type":"out","stream":"stdout","data":data})
}

fn exit(code: i64) -> serde_json::Value {
    json!({"type":"exit","code":code,"error":""})
}

fn log_msgs(rig: &Rig) -> Vec<String> {
    rig.c
        .state
        .entries
        .iter()
        .filter_map(|e| {
            if let NewEntry::Log(r) = &e.body {
                Some(r.msg.clone())
            } else {
                None
            }
        })
        .collect()
}

fn live() -> Rig {
    let mut rig = Rig::default();
    rig.open(true);
    rig.sync("local", true);
    rig.learn(supported());
    rig
}

mod cancelling;
mod capability;
mod client_side_commands;
mod dispose;
mod log_stream;
mod returning_to_the_newest_line;
mod running_a_command;
mod switching_connection;
