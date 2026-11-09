//! Work that must happen on the GPUI thread, requested by the bridge's I/O threads.

use super::input;
use gpui::{AnyWindowHandle, App, AsyncApp};
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

pub type Reply = oneshot::Sender<Result<(), String>>;

pub enum Command {
    Click {
        id: String,
        double: bool,
        reply: Reply,
    },
    Hover {
        id: String,
        reply: Reply,
    },
    Type {
        id: Option<String>,
        text: String,
        reply: Reply,
    },
    Key {
        keys: String,
        reply: Reply,
    },
    Scroll {
        id: String,
        dx: f32,
        dy: f32,
        reply: Reply,
    },
    Resize {
        width: f32,
        height: f32,
        reply: Reply,
    },
}

/// Spawns the loop that executes commands on the GPUI thread, one at a time, in arrival order.
pub fn run(cx: &mut App, window: AnyWindowHandle, mut rx: UnboundedReceiver<Command>) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        while let Some(command) = rx.recv().await {
            execute(cx, window, command);
        }
    })
    .detach();
}

fn execute(cx: &mut AsyncApp, window: AnyWindowHandle, command: Command) {
    let (result, reply) = match command {
        Command::Click { id, double, reply } => (input::click(cx, window, &id, double), reply),
        Command::Hover { id, reply } => (input::hover(cx, window, &id), reply),
        Command::Type { id, text, reply } => {
            (input::type_text(cx, window, id.as_deref(), &text), reply)
        }
        Command::Key { keys, reply } => (input::press(cx, window, &keys), reply),
        Command::Scroll { id, dx, dy, reply } => (input::scroll(cx, window, &id, dx, dy), reply),
        Command::Resize {
            width,
            height,
            reply,
        } => (input::resize(cx, window, width, height), reply),
    };
    let _ = reply.send(result);
}
