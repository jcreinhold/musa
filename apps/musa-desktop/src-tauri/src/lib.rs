//! The desktop shell.
//!
//! Deliberately thin (roadmap §15.9): command adaptation, window lifecycle,
//! file dialogs, and event delivery. Every project semantic lives in
//! `musa-project`, and this crate is not allowed to grow one.

pub mod commands;
pub mod dto;
pub mod menu;
pub mod registry;
pub mod session;

use tauri::Manager as _;

use crate::session::SessionHandle;

/// Run the application until its last window closes.
///
/// # Errors
/// If the webview or the window cannot be created, in which case there is no
/// interface to degrade to and the caller reports it on the terminal.
pub fn run() -> tauri::Result<()> {
    #[expect(clippy::exit, reason = "`generate_context!` expands to an exit on a missing asset")]
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(SessionHandle::spawn(handle.clone()));
            app.set_menu(menu::build(&handle)?)?;
            Ok(())
        })
        .on_menu_event(|app, event| menu::forward(app, event.id().as_ref()))
        .invoke_handler(tauri::generate_handler![
            commands::open_project,
            commands::new_project,
            commands::apply,
            commands::transport,
            commands::export,
            commands::snapshot,
        ])
        .run(context)
}
