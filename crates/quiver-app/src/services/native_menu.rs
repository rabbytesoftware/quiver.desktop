//! The OS's own context menu, as the Tauri app showed it (`Menu.popup()`): a GTK menu on Linux, shown
//! from the GTK thread the arrow apps' webviews already use, and muda (the crate behind Tauri's menus) on
//! macOS and Windows. Those two run the menu modally, so it is shown from a foreground task, never from
//! inside an event handler.

use std::sync::Mutex;

#[cfg(not(target_os = "linux"))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The last native menu: how many have been shown, whether it is up and its labels (a separator is "-").
/// The e2e state dump reports it, since the menu is not drawn by the app.
#[derive(Clone, Default, serde::Serialize)]
pub struct MenuRecord {
    pub shown: usize,
    pub open: bool,
    pub items: Vec<String>,
}

static RECORD: Mutex<MenuRecord> = Mutex::new(MenuRecord {
    shown: 0,
    open: false,
    items: Vec::new(),
});

pub fn record() -> MenuRecord {
    RECORD.lock().map(|r| r.clone()).unwrap_or_default()
}

/// Also raises `menu.native.open.<n>` and `menu.native.closed.<n>` (the n-th menu shown) for the e2e bridge.
fn opened(entries: &[MenuEntry]) {
    if let Ok(mut r) = RECORD.lock() {
        r.shown += 1;
        crate::e2e_bridge::signal(&format!("menu.native.open.{}", r.shown));
        r.open = true;
        r.items = entries
            .iter()
            .map(|e| match e {
                MenuEntry::Item(label) => label.clone(),
                MenuEntry::Separator => "-".to_owned(),
            })
            .collect();
    }
}

fn closed() {
    if let Ok(mut r) = RECORD.lock() {
        r.open = false;
        crate::e2e_bridge::signal(&format!("menu.native.closed.{}", r.shown));
    }
}

#[derive(Clone)]
pub enum MenuEntry {
    Item(String),
    Separator,
}

pub enum Picked {
    /// The index of the chosen entry.
    Item(usize),
    Dismissed,
    /// No native menu could be shown (no GTK, no window handle).
    Unavailable,
}

/// The window the menu belongs to. Linux needs none: GTK places the menu at the pointer by itself.
#[derive(Clone, Copy)]
pub struct MenuAnchor {
    #[cfg(not(target_os = "linux"))]
    raw: Option<RawWindowHandle>,
}

// SAFETY: plain data (a pointer or a window id), only used on the UI thread that took it.
unsafe impl Send for MenuAnchor {}

impl MenuAnchor {
    #[cfg(not(target_os = "linux"))]
    pub fn of(window: &gpui::Window) -> Self {
        Self {
            raw: window.window_handle().ok().map(|h| h.as_raw()),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn of(_: &gpui::Window) -> Self {
        Self {}
    }
}

/// Shows `entries` at the pointer and waits for the person's choice.
pub async fn popup(
    anchor: MenuAnchor,
    entries: Vec<MenuEntry>,
    cx: &gpui::AsyncApp,
) -> Picked {
    imp::popup(anchor, entries, cx).await
}

#[cfg(target_os = "linux")]
mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gtk::glib;
    use gtk::prelude::*;
    use tokio::sync::oneshot;

    use super::{MenuAnchor, MenuEntry, Picked};
    use crate::services::gtk_thread;

    thread_local! {
        /// The menu on screen, kept alive until it closes.
        static OPEN: RefCell<Option<gtk::Menu>> = const { RefCell::new(None) };
    }

    pub async fn popup(
        _: MenuAnchor,
        entries: Vec<MenuEntry>,
        _: &gpui::AsyncApp,
    ) -> Picked {
        let (reply, answer) = oneshot::channel();
        if let Err(e) = gtk_thread::post(Box::new(move || show(entries, reply))) {
            log::warn!("no native context menu: {e}");
            return Picked::Unavailable;
        }
        answer.await.unwrap_or(Picked::Unavailable)
    }

    fn show(
        entries: Vec<MenuEntry>,
        reply: oneshot::Sender<Picked>,
    ) {
        let reply = Rc::new(RefCell::new(Some(reply)));
        let answer = {
            let reply = reply.clone();
            move |picked: Picked| {
                if let Some(tx) = reply.borrow_mut().take() {
                    let _ = tx.send(picked);
                }
            }
        };
        let menu = gtk::Menu::new();
        let labels = entries.clone();
        for (ix, entry) in entries.into_iter().enumerate() {
            match entry {
                MenuEntry::Separator => menu.append(&gtk::SeparatorMenuItem::new()),
                MenuEntry::Item(label) => {
                    let item = gtk::MenuItem::with_label(&label);
                    let answer = answer.clone();
                    item.connect_activate(move |_| answer(Picked::Item(ix)));
                    menu.append(&item);
                }
            }
        }
        menu.connect_deactivate(move |_| {
            // GTK closes the menu before it activates the chosen item: "dismissed" waits a turn, so a
            // choice answers first.
            let answer = answer.clone();
            glib::idle_add_local_once(move || {
                answer(Picked::Dismissed);
                OPEN.with(|open| open.borrow_mut().take());
                super::closed();
            });
        });
        menu.show_all();
        // The pointer-anchored popup with no trigger event of GTK's own (the click went to GPUI's window).
        #[allow(deprecated)]
        menu.popup_easy(3, gtk::current_event_time());
        OPEN.with(|open| *open.borrow_mut() = Some(menu));
        super::opened(&labels);
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use std::time::Duration;

    use muda::{ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use raw_window_handle::RawWindowHandle;

    use super::{MenuAnchor, MenuEntry, Picked};

    pub async fn popup(
        anchor: MenuAnchor,
        entries: Vec<MenuEntry>,
        cx: &gpui::AsyncApp,
    ) -> Picked {
        let Some(raw) = anchor.raw else {
            return Picked::Unavailable;
        };
        let menu = Menu::new();
        let mut ids = Vec::new();
        for (ix, entry) in entries.iter().enumerate() {
            match entry {
                MenuEntry::Item(label) => {
                    let item = MenuItem::new(label, true, None);
                    ids.push((item.id().clone(), ix));
                    let _ = menu.append(&item);
                }
                MenuEntry::Separator => {
                    let _ = menu.append(&PredefinedMenuItem::separator());
                }
            }
        }
        // A choice from an earlier menu that nobody read.
        while MenuEvent::receiver().try_recv().is_ok() {}
        super::opened(&entries);
        // SAFETY: the handle is the main window's, taken on this (the UI) thread while it is open.
        let shown = unsafe {
            match raw {
                #[cfg(target_os = "macos")]
                RawWindowHandle::AppKit(h) => {
                    menu.show_context_menu_for_nsview(h.ns_view.as_ptr() as _, None)
                }
                #[cfg(target_os = "windows")]
                RawWindowHandle::Win32(h) => menu.show_context_menu_for_hwnd(h.hwnd.get(), None),
                _ => false,
            }
        };
        super::closed();
        if !shown {
            return Picked::Unavailable;
        }
        // macOS answers while the menu is still up; Windows posts the command to the window, so it lands a
        // moment after the menu closes.
        for _ in 0..20 {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some((_, ix)) = ids.iter().find(|(id, _)| *id == event.id) {
                    return Picked::Item(*ix);
                }
            }
            cx.background_executor()
                .timer(Duration::from_millis(25))
                .await;
        }
        Picked::Dismissed
    }
}
