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
    RightClick {
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
pub fn run(
    cx: &mut App,
    window: AnyWindowHandle,
    mut rx: UnboundedReceiver<Command>,
) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        while let Some(command) = rx.recv().await {
            // Typing into a field clicks it through the window system first; GPUI only sees that click on a later
            // turn of its event loop. Typed at once, the text went in ahead of the click, which then moved the
            // cursor into the middle of it ("daemo" + Enter + "n"). So the click lands before the first key.
            let command = match command {
                Command::Type {
                    id: Some(id),
                    text,
                    reply,
                } => {
                    if let Err(e) = input::click(cx, window, &id, false) {
                        let _ = reply.send(Err(e));
                        continue;
                    }
                    settle_click(cx, &id).await;
                    Command::Type {
                        id: None,
                        text,
                        reply,
                    }
                }
                other => other,
            };
            execute(cx, window, command);
        }
    })
    .detach();
}

/// Waits (up to a second) for the clicked field to report focus, then one more frame for the click's own
/// cursor placement.
async fn settle_click(
    cx: &mut AsyncApp,
    id: &str,
) {
    let timer = |cx: &mut AsyncApp| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(25))
    };
    for _ in 0..40 {
        if quiver_ui::inspect::find(id).is_some_and(|n| n.state.contains("[focused]")) {
            break;
        }
        timer(cx).await;
    }
    timer(cx).await;
}

fn execute(
    cx: &mut AsyncApp,
    window: AnyWindowHandle,
    command: Command,
) {
    let (result, reply) = match command {
        Command::Click { id, double, reply } => (input::click(cx, window, &id, double), reply),
        Command::Hover { id, reply } => (input::hover(cx, window, &id), reply),
        Command::RightClick { id, reply } => (input::right_click(&id), reply),
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
