//! The macOS application menu.
//!
//! Without this Quiver ships Tauri's stock menu, which is close to right but
//! captures Cmd+Z natively: AppKit resolves menu key-equivalents *before* the
//! web content sees them, and the native Undo targets the WKWebView's own undo
//! stack rather than anything the app knows about. Omitting Edit ▸ Undo/Redo
//! lets those keystrokes reach the webview, where the frontend can own them.
//!
//! Window ▸ Close is deliberately KEPT, which is where this diverges from
//! Crowbar's menu. Crowbar frees Cmd+W because it has in-app tabs to close;
//! Quiver has no such binding yet, so omitting it here would only make Cmd+W do
//! nothing at all. Revisit when there is something in-app for it to close.

use tauri::menu::{AboutMetadata, Menu, MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Wry};

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
	let app_menu = SubmenuBuilder::new(app, "Quiver")
		.about(Some(AboutMetadata::default()))
		.separator()
		.services()
		.separator()
		.hide()
		.hide_others()
		.show_all()
		.separator()
		.quit()
		.build()?;

	// No undo()/redo(): freed for the webview (see module docs).
	let edit_menu = SubmenuBuilder::new(app, "Edit")
		.cut()
		.copy()
		.paste()
		.select_all()
		.build()?;

	let window_menu = SubmenuBuilder::new(app, "Window")
		.minimize()
		.close_window()
		.build()?;

	MenuBuilder::new(app)
		.items(&[&app_menu, &edit_menu, &window_menu])
		.build()
}
