//! The native menu, built from the command registry.
//!
//! The menu holds no behavior: selecting an item forwards its command id to
//! the webview, which runs the same code path the palette and the keyboard
//! will. That is what keeps three surfaces from drifting into three
//! implementations of "Save".

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

use crate::registry::{COMMANDS, Section};

/// The event a menu selection raises. The webview runs the command.
pub const MENU_EVENT: &str = "musa://command";

/// Build the application menu.
///
/// The registry supplies musa's own commands. Around them sit the items every
/// desktop application is expected to have and none of which are musa's to
/// invent: quitting, hiding, and the clipboard. They are the platform's
/// predefined items rather than commands of ours, because a composer pressing
/// `⌘Q` is not asking musa for an opinion — and on macOS a webview only gets
/// working `⌘X`/`⌘C`/`⌘V` if the menu says so.
///
/// # Errors
/// Propagates whatever Tauri reports if a menu cannot be constructed.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let mut menu = MenuBuilder::new(app);

    // The application submenu, first and named after the app, is where macOS
    // users look for About and Quit. There is no equivalent elsewhere: other
    // platforms put Quit at the foot of File, which is handled below.
    #[cfg(target_os = "macos")]
    {
        menu = menu.item(
            &SubmenuBuilder::new(app, "musa")
                .about(None)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?,
        );
    }

    for section in [Section::File, Section::Edit, Section::View, Section::Help] {
        let mut submenu = SubmenuBuilder::new(app, title(section));
        for descriptor in COMMANDS.iter().filter(|entry| entry.section == section) {
            let mut item = MenuItemBuilder::with_id(descriptor.id, descriptor.title).enabled(descriptor.available);
            if let Some(accelerator) = descriptor.accelerator {
                item = item.accelerator(accelerator);
            }
            submenu = submenu.item(&item.build(app)?);
        }

        // Undo and Redo are musa's own — they walk the document's revisions,
        // not the focused field's edit history. The clipboard is not: it is
        // the platform's, and on macOS the webview will not answer the
        // clipboard keys unless these items exist to route them.
        if section == Section::Edit {
            submenu = submenu.separator().cut().copy().paste().select_all();
        }

        // Windows and Linux have no application submenu, so File ends with
        // the way out.
        #[cfg(not(target_os = "macos"))]
        if section == Section::File {
            submenu = submenu.separator().quit();
        }

        menu = menu.item(&submenu.build()?);
    }

    menu.build()
}

const fn title(section: Section) -> &'static str {
    match section {
        Section::File => "File",
        Section::Edit => "Edit",
        Section::View => "View",
        Section::Help => "Help",
    }
}

/// Forward a menu selection to the webview by command id.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    // A failed emit means the window is closing; there is nothing to run.
    app.emit(MENU_EVENT, id).ok();
}
