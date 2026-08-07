//! Every command the application has, with its binding, in one place.
//!
//! `03-interaction.md` §6: the menu, the keyboard sheet, and the command
//! palette are all rendered from this list, so documentation cannot drift
//! from bindings and a command cannot exist in one surface and not another.
//! The palette itself is prompt 23; the list starts here because the native
//! menu already needs it.
//!
//! Commands are named as they resolve — verbs, sentence case, matching the
//! result they report (`05-states.md` §1).

use serde::Serialize;
use ts_rs::TS;

/// Which menu a command appears under, and how the palette groups it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../ui/src/lib/session/generated/")]
pub enum Section {
    File,
    Edit,
    View,
    Help,
}

/// One command: its identity, its words, and its binding.
#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../ui/src/lib/session/generated/")]
pub struct CommandDescriptor {
    /// Stable identity. Menu events and palette entries both use it.
    pub id: &'static str,
    /// What the user sees, in the same words as the result it reports.
    pub title: &'static str,
    pub section: Section,
    /// The binding, in Tauri's accelerator spelling; `None` for mouse-only.
    pub accelerator: Option<&'static str>,
    /// Whether the command is implemented yet. A command that is coming is
    /// shown disabled rather than hidden, so the user learns it exists
    /// (`04-provenance.md` §4 applies the same rule to the edit choice).
    pub available: bool,
}

const fn command(
    id: &'static str,
    title: &'static str,
    section: Section,
    accelerator: Option<&'static str>,
    available: bool,
) -> CommandDescriptor {
    CommandDescriptor {
        id,
        title,
        section,
        accelerator,
        available,
    }
}

/// The registry, in menu order.
pub const COMMANDS: &[CommandDescriptor] = &[
    command("file.new", "New piece", Section::File, Some("CmdOrCtrl+N"), true),
    command("file.open", "Open a piece", Section::File, Some("CmdOrCtrl+O"), true),
    command("file.save", "Save", Section::File, Some("CmdOrCtrl+S"), true),
    command("file.export.mei", "Export MEI", Section::File, None, true),
    command("file.export.lilypond", "Export LilyPond", Section::File, None, true),
    command("file.export.musicxml", "Export MusicXML", Section::File, None, true),
    command("file.export.wav", "Export WAV", Section::File, None, true),
    command("edit.undo", "Undo", Section::Edit, Some("CmdOrCtrl+Z"), true),
    command("edit.redo", "Redo", Section::Edit, Some("CmdOrCtrl+Shift+Z"), true),
    command(
        "edit.format",
        "Format the source",
        Section::Edit,
        Some("CmdOrCtrl+Shift+F"),
        true,
    ),
    // The four workspaces of roadmap §14.4, in the order they are numbered:
    // what the music is, what it sounds like, how it is balanced, and what it
    // says in text.
    command(
        "view.workspace.compose",
        "Compose workspace",
        Section::View,
        Some("CmdOrCtrl+1"),
        true,
    ),
    command(
        "view.workspace.sound",
        "Sound workspace",
        Section::View,
        Some("CmdOrCtrl+2"),
        true,
    ),
    command(
        "view.workspace.mix",
        "Mix workspace",
        Section::View,
        Some("CmdOrCtrl+3"),
        true,
    ),
    command(
        "view.workspace.source",
        "Source workspace",
        Section::View,
        Some("CmdOrCtrl+4"),
        true,
    ),
    command("view.zoom.out", "Zoom out", Section::View, Some("CmdOrCtrl+-"), true),
    command("view.zoom.in", "Zoom in", Section::View, Some("CmdOrCtrl+="), true),
    command(
        "view.zoom.reset",
        "Reset zoom",
        Section::View,
        Some("CmdOrCtrl+0"),
        true,
    ),
    command(
        "view.source",
        "Show the source",
        Section::View,
        Some("CmdOrCtrl+'"),
        true,
    ),
    command("view.theme", "Switch theme", Section::View, None, true),
    command(
        "view.palette",
        "Command palette",
        Section::View,
        Some("CmdOrCtrl+K"),
        true,
    ),
    command("help.keys", "Keyboard sheet", Section::Help, Some("?"), true),
];
