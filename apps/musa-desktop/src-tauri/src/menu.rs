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
/// # Errors
/// Propagates whatever Tauri reports if a menu cannot be constructed.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let mut menu = MenuBuilder::new(app);
    for section in [Section::File, Section::Edit, Section::View, Section::Help] {
        let mut submenu = SubmenuBuilder::new(app, title(section));
        for descriptor in COMMANDS.iter().filter(|entry| entry.section == section) {
            let mut item = MenuItemBuilder::with_id(descriptor.id, descriptor.title).enabled(descriptor.available);
            if let Some(accelerator) = descriptor.accelerator {
                item = item.accelerator(accelerator);
            }
            submenu = submenu.item(&item.build(app)?);
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
