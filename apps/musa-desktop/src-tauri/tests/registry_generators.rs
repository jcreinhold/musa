//! Generates the command registry the interface renders its bindings from.
//!
//! `03-interaction.md` §6: the native menu, the keyboard sheet, and the
//! command palette are one list. The menu reads it from Rust; the webview
//! reads this JSON. Writing it from `COMMANDS` means a binding cannot be
//! documented in one place and bound in another.
//!
//! Run `UPDATE_UI_FIXTURES=1 cargo test -p musa-desktop` to refresh.

use std::path::{Path, PathBuf};

use musa_desktop::registry::COMMANDS;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn generated(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ui/src/lib/session/generated")
        .join(name)
}

#[test]
fn command_registry_is_current() -> Result {
    let mut json = serde_json::to_string_pretty(COMMANDS)?;
    json.push('\n');

    let path = generated("commands.json");
    let current = std::fs::read_to_string(&path).ok();
    if current.as_deref() == Some(&json) {
        return Ok(());
    }
    if current.is_some() && std::env::var_os("UPDATE_UI_FIXTURES").is_none() {
        return Err(format!(
            "{} is stale — rerun with UPDATE_UI_FIXTURES=1 and commit the result",
            path.display()
        )
        .into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, json)?;
    Ok(())
}

/// Ids are how the menu, the palette, and the webview refer to the same
/// command; two commands sharing one would make a menu item ambiguous.
#[test]
fn command_ids_are_unique() {
    let mut ids: Vec<&str> = COMMANDS.iter().map(|command| command.id).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "two commands share an id");
}

/// Same for accelerators: a duplicate binding is a keystroke whose result
/// depends on menu order.
#[test]
fn accelerators_are_unique() {
    let mut keys: Vec<&str> = COMMANDS.iter().filter_map(|command| command.accelerator).collect();
    keys.sort_unstable();
    let count = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), count, "two commands share an accelerator");
}
