//! The whole bridge. Twelve commands, and it stays this small.
//!
//! Nothing here decides anything musical: each command translates a DTO into
//! a `musa-project` request, hands it to the session thread, and returns what
//! came back. The shell does command adaptation, window
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

/// Open a `.musa` file, or a folder of them, as the session's project.
///
/// # Errors
/// If the file cannot be read, or the folder holds no piece.
#[tauri::command]
pub fn open_project(path: String, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.open(PathBuf::from(path))
}

/// Turn to another file of the project already open.
///
/// `file` is a name from the snapshot's contents, which is the only place the
/// webview learns one: the shell never composes a path.
///
/// # Errors
/// If nothing is open, or the file cannot be read.
#[tauri::command]
pub fn show_piece(file: String, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.show(file)
}

/// Write every piece of the project that has unsaved edits.
///
/// # Errors
/// If nothing is open, or a piece cannot be written.
#[tauri::command]
pub fn save_all(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.save_all()
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

/// Start or stop reading a MIDI keyboard.
///
/// Turned on with note entry and off with it: a keyboard is read while the
/// composer is entering notes, and at every other moment the session is
/// asleep. Answering with the snapshot is how the interface learns which
/// keyboard it got, if any.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn listen_to_midi(
    listening: bool,
    caret: Option<String>,
    session: State<'_, SessionHandle>,
) -> Result<Value, ErrorDto> {
    session.listen_to_midi(listening, caret)
}

/// The current snapshot, for a window that has just opened.
///
/// # Errors
/// If no piece is open yet.
#[tauri::command]
pub fn snapshot(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.snapshot()
}

/// Read the last valid score and report what one analysis saw.
///
/// `kind` is the analysis's own command-line spelling, which is what the
/// snapshot's findings and the language server's lenses both name it by. The
/// request is the default one — the whole score, segmented at attacks —
/// because a panel has nowhere to put a narrowing and prompt 142 owns the
/// screen that would.
///
/// # Errors
/// If no piece is open, it has never compiled, or the kind is not one this
/// compiler runs. Refused rather than answered empty: a reader shown zero
/// findings would conclude the music is clean
/// (`docs/rules/desktop/08-elaboration.md` §8).
#[tauri::command]
pub fn analyze(kind: String, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.analyze(kind)
}

/// The bundled library module `uri` names, with `start..end` restated in its
/// own measure.
///
/// The handle is the one the snapshot handed out beside the URI: a term
/// declared outside the open document travels as a module and an opaque byte
/// range, because a span on the wire indexes the document the wire carried
/// (`docs/rules/desktop/08-elaboration.md` §3). No session state is touched — a
/// bundled module is compiled into the binary and is the same in every
/// window.
///
/// # Errors
/// If no bundled module answers to that URI.
#[tauri::command]
pub fn library_document(uri: String, start: Option<u32>, end: Option<u32>) -> Result<Value, ErrorDto> {
    let at = start.zip(end);
    let document = musa_project::library_document(&uri, at)
        .ok_or_else(|| ErrorDto::shell(ErrorKindDto::Nothing, format!("`{uri}` is not a bundled Musa module")))?;
    serde_json::to_value(document).map_err(|error| ErrorDto::shell(ErrorKindDto::Backend, error.to_string()))
}
