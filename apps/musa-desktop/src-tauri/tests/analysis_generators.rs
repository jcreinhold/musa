//! Generates the list of readings the findings panel offers.
//!
//! `docs/rules/desktop/08-elaboration.md` §5: an analysis is asked for by name,
//! and the names are the compiler's. A panel carrying its own copy of them
//! would offer a reading this compiler does not run — or, worse, quietly stop
//! offering one it does — the first time the list changed.
//!
//! The method line comes with it, because the panel prints it above the
//! findings and a reader who does not accept the method can stop there.
//!
//! Run `UPDATE_UI_FIXTURES=1 cargo test -p musa-desktop` to refresh.

use std::path::{Path, PathBuf};

use musa_project::AnalysisKind;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn generated(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ui/src/lib/session/generated")
        .join(name)
}

#[test]
fn analysis_kinds_are_current() -> Result {
    let table: Vec<serde_json::Value> = AnalysisKind::ALL
        .into_iter()
        .map(|kind| {
            serde_json::json!({
                "kind": kind.as_str(),
                "method": kind.method(),
            })
        })
        .collect();
    let mut json = serde_json::to_string_pretty(&table)?;
    json.push('\n');

    let path = generated("analysis-kinds.json");
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

/// Every name the panel shows is one the `analyze` command accepts.
///
/// The two sides meet at a string: the panel sends `kind` and the shell reads
/// it back with `AnalysisKind::named`. A name that does not round-trip is a
/// button that always fails.
#[test]
fn every_offered_reading_is_one_the_command_accepts() {
    for kind in AnalysisKind::ALL {
        assert_eq!(
            AnalysisKind::named(kind.as_str()),
            Some(kind),
            "`{}` is offered but not accepted",
            kind.as_str()
        );
    }
}
