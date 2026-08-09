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
    /// The app's own preferences. The only section with no menu of its own:
    /// its representative is one `Settings…` item, and the sheet behind it is
    /// where the commands are reached by pointer (prompt 59).
    Settings,
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
    ///
    /// Whole, including the submenu's name: `Export MEI`, not `MEI`. The
    /// palette and the keyboard sheet have no nesting to lean on and show
    /// this as it stands; the menu, which does, strips the prefix.
    pub title: &'static str,
    pub section: Section,
    /// The submenu this item sits in, if it is not directly in its section's
    /// menu.
    ///
    /// Always a prefix of [`Self::title`] — `Export ▸ Export MEI` is the same
    /// word twice — which is what lets the menu show the short form without a
    /// second name to keep in step. Asserted by a test.
    pub submenu: Option<&'static str>,
    /// Whether a separator precedes this item: it begins a new group of
    /// related commands. Menus with seams answer questions; flat ones list
    /// functions.
    pub apart: bool,
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
        submenu: None,
        apart: false,
        accelerator,
        available,
    }
}

/// The same command, beginning a new group: a separator goes above it.
const fn apart(descriptor: CommandDescriptor) -> CommandDescriptor {
    CommandDescriptor {
        apart: true,
        ..descriptor
    }
}

/// The same command, inside a submenu of its section's menu.
const fn under(descriptor: CommandDescriptor, submenu: &'static str) -> CommandDescriptor {
    CommandDescriptor {
        submenu: Some(submenu),
        ..descriptor
    }
}

/// The item that opens the settings sheet.
///
/// The one command the menu places by hand — macOS wants it in the application
/// submenu, every other platform at the foot of Edit — so it is named here
/// rather than searched for by id. A registry that stopped declaring it would
/// fail to compile, which is a stronger statement than a test.
pub const SETTINGS: CommandDescriptor = command(
    "settings.open",
    "Settings",
    Section::Settings,
    Some("CmdOrCtrl+,"),
    true,
);

/// The registry, in menu order.
pub const COMMANDS: &[CommandDescriptor] = &[
    command("file.new", "New piece", Section::File, Some("CmdOrCtrl+N"), true),
    command("file.open", "Open a piece", Section::File, Some("CmdOrCtrl+O"), true),
    // A project is a folder of pieces (roadmap §16), and opening one is a
    // different question from opening a file — so it is a different item,
    // rather than a file dialog that guesses from what was chosen.
    command(
        "file.openProject",
        "Open a project",
        Section::File,
        Some("CmdOrCtrl+Shift+O"),
        true,
    ),
    apart(command("file.save", "Save", Section::File, Some("CmdOrCtrl+S"), true)),
    command(
        "file.saveAll",
        "Save every piece",
        Section::File,
        Some("CmdOrCtrl+Alt+S"),
        true,
    ),
    // One verb, four objects. Four sibling `Export …` items read as four
    // different things a composer might do; they are one thing with a choice
    // of format, and a submenu is how a menu says so.
    apart(under(
        command("file.export.mei", "Export MEI", Section::File, None, true),
        "Export",
    )),
    under(
        command("file.export.lilypond", "Export LilyPond", Section::File, None, true),
        "Export",
    ),
    under(
        command("file.export.musicxml", "Export MusicXML", Section::File, None, true),
        "Export",
    ),
    under(
        command("file.export.wav", "Export WAV", Section::File, None, true),
        "Export",
    ),
    command("edit.undo", "Undo", Section::Edit, Some("CmdOrCtrl+Z"), true),
    command("edit.redo", "Redo", Section::Edit, Some("CmdOrCtrl+Shift+Z"), true),
    apart(command(
        "edit.format",
        "Format the source",
        Section::Edit,
        Some("CmdOrCtrl+Shift+F"),
        true,
    )),
    // The volume, before the piece: a contents page is what a bound book puts
    // in front of its first movement, and `⌘0` is the number before the four
    // workspaces for the same reason (prompt 85).
    command(
        "view.workspace.contents",
        "Contents",
        Section::View,
        Some("CmdOrCtrl+0"),
        false,
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
    apart(command(
        "view.source",
        "Show the source",
        Section::View,
        Some("CmdOrCtrl+'"),
        true,
    )),
    apart(command(
        "view.zoom.out",
        "Zoom out",
        Section::View,
        Some("CmdOrCtrl+-"),
        true,
    )),
    command("view.zoom.in", "Zoom in", Section::View, Some("CmdOrCtrl+="), true),
    // `⇧⌘0` rather than `⌘0`: the number keys number the leaf's screens, and
    // `⌘0` is the contents page above them. Reset keeps the shape every other
    // reset in the application has — a modifier and a nought.
    command(
        "view.zoom.reset",
        "Reset zoom",
        Section::View,
        Some("CmdOrCtrl+Shift+0"),
        true,
    ),
    apart(command(
        "view.palette",
        "Command palette",
        Section::View,
        Some("CmdOrCtrl+K"),
        true,
    )),
    // Settings has no menu of its own: this is the item that opens the sheet,
    // and the rest of the section is what the sheet contains.
    SETTINGS,
    // The frame's text, not the score's — deliberately not `⌘=`/`⌘-`, which
    // are zoom and must stay zoom (prompt 55).
    command(
        "settings.text.larger",
        "Larger text",
        Section::Settings,
        Some("CmdOrCtrl+Alt+="),
        true,
    ),
    command(
        "settings.text.smaller",
        "Smaller text",
        Section::Settings,
        Some("CmdOrCtrl+Alt+-"),
        true,
    ),
    command(
        "settings.text.reset",
        "Reset text size",
        Section::Settings,
        Some("CmdOrCtrl+Alt+0"),
        true,
    ),
    command("settings.vim", "Vim mode in the source", Section::Settings, None, true),
    command("settings.theme", "Switch theme", Section::Settings, None, true),
    command("help.keys", "Keyboard sheet", Section::Help, Some("?"), true),
];

#[cfg(test)]
mod registry_laws {
    use super::{COMMANDS, SETTINGS, Section};

    /// The rule the menu leans on to show `MEI` where the registry says
    /// `Export MEI`. If a submenu stops being a prefix of its members'
    /// titles, the menu is stripping the wrong thing and there are two names
    /// for one command again.
    #[test]
    fn every_submenu_is_a_prefix_of_its_items() {
        for command in COMMANDS {
            let Some(submenu) = command.submenu else { continue };
            assert!(
                command.title.starts_with(submenu) && command.title.len() > submenu.len(),
                "`{}` is under `{submenu}` but is not named for it",
                command.title
            );
        }
    }

    /// A section the menu renders and has nothing to put in would build an
    /// empty menu, which is worse than not having one.
    #[test]
    fn every_menu_section_has_items() {
        for section in [Section::File, Section::Edit, Section::View, Section::Help] {
            assert!(
                COMMANDS.iter().any(|command| command.section == section),
                "{section:?} has no commands"
            );
        }
    }

    /// The menu builds `Settings…` from the named descriptor, and the palette
    /// builds its entry from the registry. They are the same command only for
    /// as long as the named one is really in the list.
    #[test]
    fn settings_has_the_item_that_opens_it() {
        assert!(COMMANDS.iter().any(|command| command.id == SETTINGS.id));
        assert_eq!(SETTINGS.section, Section::Settings);
    }
}
