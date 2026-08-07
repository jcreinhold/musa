//! The whole bridge. Seven commands, and it stays this small.
//!
//! Nothing here decides anything musical: each command translates a DTO into
//! a `musa-project` request, hands it to the session thread, and returns what
//! came back. Roadmap §15.9 — the shell does command adaptation, window
//! lifecycle, file dialogs, and event delivery, and nothing else.

// Tauri's `#[command]` macro resolves `State` by value; a reference is not a
// valid command argument, so the handles cannot be borrowed here.
#![expect(
    clippy::needless_pass_by_value,
    reason = "Tauri command arguments are resolved by value"
)]

use std::path::PathBuf;

use serde_json::Value;
use tauri::State;

use crate::dto::{CommandDto, EditDto, ErrorDto, ErrorKindDto, ExportDto, TemplateDto, TransportDto};
use crate::session::SessionHandle;

/// Open a `.musa` file and make it the session's project.
///
/// # Errors
/// If the file cannot be read.
#[tauri::command]
pub fn open_project(path: String, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.open(PathBuf::from(path))
}

/// Start a new piece. Without a path it is unsaved but immediately playable.
///
/// # Errors
/// If a path was given and the file cannot be written.
#[tauri::command]
pub fn new_project(
    template: TemplateDto,
    path: Option<String>,
    session: State<'_, SessionHandle>,
) -> Result<Value, ErrorDto> {
    session.new_project(template.into(), path.map(PathBuf::from))
}

/// Apply a document command, or navigate the history.
///
/// # Errors
/// If no piece is open, if there is nothing to undo or redo, or if the
/// document refused the edit.
#[tauri::command]
pub fn apply(command: CommandDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.apply(command.into_request())
}

/// What a score edit would change, before it is made — the counts and the
/// affected notes of `04-provenance.md` §4's inline choice.
///
/// # Errors
/// If no piece is open, it has never compiled, or the edit names an event
/// this revision does not have.
#[tauri::command]
pub fn edit_impact(edit: EditDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.edit_impact(edit.into())
}

/// Play, stop, seek, or set the loop.
///
/// # Errors
/// If no piece is open, or the audio engine refuses the request.
#[tauri::command]
pub fn transport(command: TransportDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.transport(command.into())
}

/// Write an export to disk and report where it landed.
///
/// # Errors
/// If no destination was chosen, no piece is open, the piece has never
/// compiled, or the file cannot be written.
#[tauri::command]
pub fn export(request: ExportDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    let path = request
        .path
        .ok_or_else(|| ErrorDto::shell(ErrorKindDto::File, "Choose where to save the export first"))?;
    session.export(request.target.into(), PathBuf::from(path))
}

/// The current snapshot, for a window that has just opened.
///
/// # Errors
/// If no piece is open yet.
#[tauri::command]
pub fn snapshot(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.snapshot()
}
