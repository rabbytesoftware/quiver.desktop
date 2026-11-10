//! Dock keys: Escape closes, Ctrl+C cancels, Up and Down walk the history, Tab completes a command word.

use gpui::{Context, KeyDownEvent, Window};
use quiver_state::console::{complete_line, Direction};
use quiver_ui::InputEvent;

use crate::views::console::ConsoleDock;

impl ConsoleDock {
    pub(in crate::views::console) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = &event.keystroke;
        let handled = match key.key.as_str() {
            "escape" => {
                self.close(cx);
                true
            }
            "c" if key.modifiers.control => {
                self.cancel(cx);
                true
            }
            "up" => self.step(Direction::Up, window, cx),
            "down" => self.step(Direction::Down, window, cx),
            "tab" => self.complete(window, cx),
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }

    fn step(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let typed = self.input.read(cx).value().to_string();
        if typed != self.controller.state.draft
            && self.controller.state.history_cursor.index.is_none()
        {
            self.controller.state.set_draft(&typed);
        }
        self.controller.state.step_history(direction);
        let text = self.controller.state.draft.clone();
        self.input.update(cx, |i, cx| i.set_value(text, window, cx));
        true
    }

    fn complete(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let typed = self.input.read(cx).value().to_string();
        let done = complete_line(&typed, &self.controller.state.commands);
        if done.line != typed {
            self.input
                .update(cx, |i, cx| i.set_value(done.line, window, cx));
        }
        true
    }

    pub(in crate::views::console) fn input_event(
        &mut self,
        input: &gpui::Entity<quiver_ui::InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => self.run_line(window, cx),
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                if text != self.controller.state.draft
                    && self.controller.state.history_cursor.index.is_none()
                {
                    self.controller.state.set_draft(&text);
                }
            }
            _ => {}
        }
    }

    /// Keeps the log on its newest line while new lines arrive and after a command is run.
    pub(in crate::views::console) fn follow_tail(&mut self) {
        let state = &self.controller.state;
        if state.tail != self.tail_seen || state.entries.len() != self.entries_seen {
            self.tail_seen = state.tail;
            self.entries_seen = state.entries.len();
            self.scroll.scroll_to_bottom();
        }
    }
}
