//! The native menu, built from the command registry.
//!
//! The menu holds no behavior: selecting an item forwards its command id to
//! the webview, which runs the same code path the palette and the keyboard
//! will. That is what keeps three surfaces from drifting into three
//! implementations of "Save".
//!
//! What the menu *does* own is shape. The registry says which section a
//! command belongs to, which submenu it sits in, and where the groups break;
//! this reads those three facts and nothing else.

use tauri::menu::{Menu, MenuBuilder, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

use crate::registry::{COMMANDS, CommandDescriptor, SETTINGS, Section};

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
    // users look for About, Settings, and Quit. There is no equivalent
    // elsewhere: other platforms put Settings and Quit in File and Edit,
    // which is handled below.
    #[cfg(target_os = "macos")]
    {
        menu = menu.item(
            &SubmenuBuilder::new(app, "musa")
                .about(None)
                .separator()
                .item(&settings_item(app)?)
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
        // Items of one submenu are contiguous in the registry, so the nested
        // menu is gathered while they last and flushed when they stop. `apart`
        // on its first item breaks the *section's* group, so it is carried
        // along and spent on the submenu rather than inside it.
        let mut nested: Option<Nested<R>> = None;
        for descriptor in COMMANDS.iter().filter(|entry| entry.section == section) {
            if let Some(finished) = nested.take_if(|gathered| Some(gathered.name) != descriptor.submenu) {
                submenu = flush(app, submenu, &finished)?;
            }
            let item = item_for(app, descriptor)?;
            match descriptor.submenu {
                Some(name) => nested
                    .get_or_insert_with(|| Nested {
                        name,
                        apart: descriptor.apart,
                        items: Vec::new(),
                    })
                    .items
                    .push(item),
                None => {
                    if descriptor.apart {
                        submenu = submenu.separator();
                    }
                    submenu = submenu.item(&item);
                }
            }
        }
        if let Some(finished) = nested {
            submenu = flush(app, submenu, &finished)?;
        }

        // Undo and Redo are musa's own — they walk the document's revisions,
        // not the focused field's edit history. The clipboard is not: it is
        // the platform's, and on macOS the webview will not answer the
        // clipboard keys unless these items exist to route them.
        if section == Section::Edit {
            submenu = submenu.separator().cut().copy().paste().select_all();

            // Windows and Linux have no application submenu, so Settings
            // ends Edit — which is where both platforms put it.
            #[cfg(not(target_os = "macos"))]
            {
                submenu = submenu.separator().item(&settings_item(app)?);
            }
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

/// One item, as its descriptor states it.
///
/// The title inside a submenu drops the submenu's name: the registry says
/// `Export MEI` so that the palette can, and `Export ▸ Export MEI` is the
/// same word twice.
fn item_for<R: Runtime>(app: &AppHandle<R>, descriptor: &CommandDescriptor) -> tauri::Result<MenuItem<R>> {
    let title = descriptor
        .submenu
        .and_then(|name| descriptor.title.strip_prefix(name))
        .map_or(descriptor.title, str::trim_start);
    let mut item = MenuItemBuilder::with_id(descriptor.id, title).enabled(descriptor.available);
    if let Some(accelerator) = descriptor.accelerator {
        item = item.accelerator(accelerator);
    }
    item.build(app)
}

/// A submenu being gathered: its name, whether it begins a group, and what
/// has been put in it so far.
struct Nested<R: Runtime> {
    name: &'static str,
    apart: bool,
    items: Vec<MenuItem<R>>,
}

/// Put a gathered submenu into the section's menu.
fn flush<'a, R: Runtime>(
    app: &AppHandle<R>,
    menu: SubmenuBuilder<'a, R, AppHandle<R>>,
    nested: &Nested<R>,
) -> tauri::Result<SubmenuBuilder<'a, R, AppHandle<R>>> {
    let mut builder = SubmenuBuilder::new(app, nested.name);
    for item in &nested.items {
        builder = builder.item(item);
    }
    let built: Submenu<R> = builder.build()?;
    Ok(if nested.apart {
        menu.separator().item(&built)
    } else {
        menu.item(&built)
    })
}

/// `Settings…`, placed by hand because each platform puts it somewhere else.
///
/// The ellipsis is the platform's convention for an item that opens something
/// rather than doing something, and it belongs to the menu rather than to the
/// registry: the palette's entry is `Settings`, which is what running it says.
fn settings_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    let mut item = MenuItemBuilder::with_id(SETTINGS.id, format!("{}…", SETTINGS.title));
    if let Some(accelerator) = SETTINGS.accelerator {
        item = item.accelerator(accelerator);
    }
    item.build(app)
}

const fn title(section: Section) -> &'static str {
    match section {
        Section::File => "File",
        Section::Edit => "Edit",
        Section::View => "View",
        // Settings has no menu of its own; `settings_item` places its one
        // visible item, and the sheet behind it holds the rest.
        Section::Settings => "Settings",
        Section::Help => "Help",
    }
}

/// Forward a menu selection to the webview by command id.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    // A failed emit means the window is closing; there is nothing to run.
    app.emit(MENU_EVENT, id).ok();
}
