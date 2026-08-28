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

use crate::dto::{
    CommandDto, DawBundleDto, EditDto, ErrorDto, ErrorKindDto, ExportDto, GroupEditDto, ReviewActionDto,
    ReviewAuditionDto, TemplateDto, TransportDto,
};
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

/// Preview the exact barline source action without changing history.
///
/// # Errors
/// If no piece is open or the checked source cannot be rewritten with
/// certainty at direct authored boundaries.
#[tauri::command]
pub fn barline_rewrite(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.barline_rewrite()
}

/// Preview one group transformation: what it would change, what it would
/// leave alone, and the definitions it rewrites.
///
/// Nothing is applied. The reply carries the plan's identity, and accepting
/// it is `CommandDto::ApplyGroupEdit` at the same revision.
///
/// # Errors
/// If no piece is open, it has never compiled, the selection names an event
/// this revision does not have, the command is musically refused, or the
/// result would not compile.
#[tauri::command]
pub fn group_edit_plan(edit: GroupEditDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.group_edit_plan(edit.into())
}

/// Open the take just captured for review.
///
/// Nothing is written: the reply is a reading of the take against this
/// piece's meter, tempo, and key, and the source is untouched until the
/// reviewed phrase is placed.
///
/// # Errors
/// If no piece is open, it has never compiled, nothing has been played, or
/// the take is refused rather than written onto a grid.
#[tauri::command]
pub fn review_begin(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_begin()
}

/// Read the current review, or `null` when nothing is under review.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn review_read(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_read()
}

/// Make one gesture on the review.
///
/// # Errors
/// If no piece is open, nothing is under review, it has been accepted, the
/// gesture names something that is not there, or the site does not admit it.
#[tauri::command]
pub fn review_act(action: ReviewActionDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_act(action.into())
}

/// Take back the last review decision.
///
/// # Errors
/// If no piece is open, nothing is under review, it has been accepted, or
/// there is nothing left to take back.
#[tauri::command]
pub fn review_undo(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_undo()
}

/// Hear the take as played, or the notation as written.
///
/// # Errors
/// If no piece is open or nothing is under review.
#[tauri::command]
pub fn review_audition(mode: ReviewAuditionDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_audition(mode.into())
}

/// Accept the reading. The phrase is settled; placing it is a separate step.
///
/// # Errors
/// If no piece is open, nothing is under review, it was already accepted, or
/// the reading still has a length it cannot write exactly.
#[tauri::command]
pub fn review_accept(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_accept()
}

/// Close the review and drop the take with it.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn review_discard(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_discard()
}

/// What keeping the accepted phrase would write, before it is written.
///
/// `voices` names one destination voice per line the phrase writes; empty
/// asks for the names the project would offer.
///
/// # Errors
/// If no piece is open, nothing is under review, the reading has not been
/// accepted, the piece does not compile, the part the take was played into is
/// gone, or a name is not a name.
#[tauri::command]
pub fn review_placement_plan(voices: Vec<String>, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_placement_plan(voices)
}

/// Keep the accepted phrase: write it into the score as one revision.
///
/// # Errors
/// Everything `review_placement_plan` refuses, plus an unnamed extra line and
/// a document the phrase would stop compiling. A refusal writes nothing and
/// leaves the review open.
#[tauri::command]
pub fn review_place(voices: Vec<String>, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.review_place(voices)
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

/// Package one workstation bundle into a directory and report what it holds.
///
/// The directory is written whole or not at all. What comes back is the
/// project's own report: every file, and everything the target formats could
/// not carry.
///
/// # Errors
/// If no destination was chosen, no piece is open, the piece has never
/// compiled, or the directory cannot be written.
#[tauri::command]
pub fn export_daw_bundle(request: DawBundleDto, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    let path = request
        .path
        .clone()
        .ok_or_else(|| ErrorDto::shell(ErrorKindDto::File, "Choose where to put the bundle first"))?;
    session.export_daw_bundle((&request).into(), PathBuf::from(path))
}

/// Say where in the score the composer is, so a connected keyboard auditions
/// with that part's sound.
///
/// A keyboard is listened to whenever it is safely connected. There is no
/// armed state for it to enter: capture is the only armed state, and it is
/// asked for by name.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn audition_at(caret: Option<String>, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.audition_at(caret)
}

/// Select one stable MIDI input identity.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn select_midi_input(id: String, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.select_midi(id)
}

/// Begin explicit MIDI capture without editing source.
///
/// # Errors
/// If no piece is open or capture cannot begin in the current state.
#[tauri::command]
pub fn start_midi_capture(caret: Option<String>, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.start_midi_capture(caret)
}

/// Freeze the active capture for Review.
///
/// # Errors
/// If no piece is open or no capture is active.
#[tauri::command]
pub fn stop_midi_capture(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.stop_midi_capture()
}

/// Freeze the bounded recent phrase for Review.
///
/// # Errors
/// If no piece is open or no complete recent phrase is available.
#[tauri::command]
pub fn keep_recent_midi(caret: Option<String>, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.keep_recent_midi(caret)
}

/// Clear memory-only recent MIDI evidence.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn clear_recent_midi(session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.clear_recent_midi()
}

/// Enable or disable recent phrase memory.
///
/// # Errors
/// If no piece is open.
#[tauri::command]
pub fn set_recent_midi(enabled: bool, session: State<'_, SessionHandle>) -> Result<Value, ErrorDto> {
    session.set_recent_midi(enabled)
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
/// because a panel has nowhere to put a narrowing and prompt 189 owns the
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
