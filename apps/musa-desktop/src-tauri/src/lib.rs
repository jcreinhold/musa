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
    // The same subscriber the CLI and the language server install, on stderr,
    // asked for the same way: `MUSA_LOG=musa_project=debug` on the app's
    // environment. The desktop has no verbosity flag to pass, so it takes the
    // default — musa's own warnings — and the variable raises it.
    let _installed = musa_project::Logging::new().install();
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
            commands::show_piece,
            commands::save_all,
            commands::apply,
            commands::edit_impact,
            commands::barline_rewrite,
            commands::group_edit_plan,
            commands::review_begin,
            commands::review_read,
            commands::review_act,
            commands::review_undo,
            commands::review_audition,
            commands::review_accept,
            commands::review_discard,
            commands::review_placement_plan,
            commands::review_place,
            commands::transport,
            commands::export,
            commands::snapshot,
            commands::audition_at,
            commands::select_midi_input,
            commands::start_midi_capture,
            commands::stop_midi_capture,
            commands::keep_recent_midi,
            commands::clear_recent_midi,
            commands::set_recent_midi,
            commands::analyze,
            commands::library_document,
        ])
        .run(context)
}
