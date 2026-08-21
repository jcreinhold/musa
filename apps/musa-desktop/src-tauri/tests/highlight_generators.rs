//! Generates what the source editor sets its text by.
//!
//! The highlighting is derived from the real token list, not
//! from a pattern set in the interface that drifts the first time a keyword is
//! added. Two artefacts come out of the lexer itself:
//!
//! - `token-classes.json`, every class [`TokenClass`] can produce, which the
//!   editor's style table must cover;
//! - `spellings.json`, every word and mark with its class, so no editor
//!   carries its own keyword list;
//! - `module-names.json`, the keywords a module name may borrow
//!   ([`MODULE_NAME_KEYWORDS`]), so an editor that scopes import paths can
//!   tell `std::harmony` from `harmony { ... }` without owning the list;
//! - `fixtures/lexed/<example>.json`, every token of every example as the
//!   real lexer read it, which the editor's own tokenizer is tested against.
//!
//! Run `UPDATE_UI_FIXTURES=1 cargo test -p musa-desktop` to refresh.

use std::path::{Path, PathBuf};

use musa_syntax::{MODULE_NAME_KEYWORDS, SPELLINGS, SyntaxKind, TokenClass, keyword_doc, lex};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

/// Every class, in the order the enum declares them.
const CLASSES: [TokenClass; 11] = [
    TokenClass::Comment,
    TokenClass::Keyword,
    TokenClass::Use,
    TokenClass::Pitch,
    TokenClass::Duration,
    TokenClass::Number,
    TokenClass::Text,
    TokenClass::Unit,
    TokenClass::Name,
    TokenClass::Punctuation,
    TokenClass::Invalid,
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ui(relative: &str) -> PathBuf {
    root().join("../ui").join(relative)
}

/// Write a generated file, or fail if it is stale and no refresh was asked for.
fn generated(path: &Path, contents: &str) -> Result {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() == Some(contents) {
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
    std::fs::write(path, contents)?;
    Ok(())
}

#[test]
fn token_classes_are_current() -> Result {
    let names: Vec<&str> = CLASSES.iter().map(|class| class.name()).collect();
    let mut json = serde_json::to_string_pretty(&names)?;
    json.push('\n');
    generated(&ui("src/lib/session/generated/token-classes.json"), &json)
}

/// The list has to be the whole enum, or the editor's coverage test is testing
/// a subset of the language and reporting that everything is covered.
#[test]
fn every_class_a_token_can_have_is_listed() {
    for kind in 0_u16..=u16::MAX {
        let Some(class) = TokenClass::of(SyntaxKind::from(kind)) else {
            continue;
        };
        assert!(CLASSES.contains(&class), "{class:?} is missing from CLASSES");
    }
}

/// The words and marks the composer types literally, with what each one is.
/// The editor looks words up here rather than carrying its own keyword list.
#[test]
fn spellings_are_current() -> Result {
    let table: Vec<(&str, &str)> = SPELLINGS
        .iter()
        .filter_map(|&(text, kind)| Some((text, TokenClass::of(kind)?.name())))
        .collect();
    let mut json = serde_json::to_string_pretty(&table)?;
    json.push('\n');
    generated(&ui("src/lib/session/generated/spellings.json"), &json)
}

/// The keywords a module name may borrow, spelled out, so an editor that
/// scopes import paths (`import std::harmony;`, `mod list;`) paints those
/// words as names without carrying its own copy of the list.
#[test]
fn module_names_are_current() -> Result {
    let mut table: Vec<&str> = Vec::new();
    for kind in MODULE_NAME_KEYWORDS {
        let Some(spelling) = SPELLINGS
            .iter()
            .find_map(|&(text, spelled)| (spelled == *kind).then_some(text))
        else {
            return Err(format!("{kind:?} has no spelling in SPELLINGS").into());
        };
        table.push(spelling);
    }
    let mut json = serde_json::to_string_pretty(&table)?;
    json.push('\n');
    generated(&ui("src/lib/session/generated/module-names.json"), &json)
}

/// Every keyword's own documentation, so the source editor's
/// hover teaches the same words the language server serves — written out of
/// the one table, because a tooltip the editor authored itself would drift
/// the first time a keyword's doc changed.
#[test]
fn keyword_docs_are_current() -> Result {
    let table: Vec<serde_json::Value> = SPELLINGS
        .iter()
        .filter_map(|&(_text, kind)| {
            let doc = keyword_doc(kind)?;
            Some(serde_json::json!({
                "spelling": doc.spelling,
                "summary": doc.summary,
                "doc": doc.doc,
            }))
        })
        .collect();
    let mut json = serde_json::to_string_pretty(&table)?;
    json.push('\n');
    generated(&ui("src/lib/session/generated/keyword-docs.json"), &json)
}

#[test]
fn lexed_examples_are_current() -> Result {
    for entry in std::fs::read_dir(root().join("../../../examples"))? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "musa") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let source = std::fs::read_to_string(&path)?;

        // Offsets in the editor's measure, not the lexer's: these are compared
        // against a CodeMirror tokenizer walking a JavaScript string, and the
        // two disagree from the first character above U+007F onwards. The
        // translation is the same one every span in the snapshot gets, so this
        // fixture cannot drift from what the running application sends.
        let offsets = musa_project::Utf16Offsets::new(&source);

        // Trivia is skipped: whitespace carries no ink, and an editor that
        // agreed with the lexer about where the spaces are would still be
        // wrong about the words.
        let tokens: Vec<serde_json::Value> = lex(&source)
            .tokens()
            .iter()
            .filter_map(|token| {
                let class = TokenClass::of(token.kind)?;
                if class == TokenClass::Comment && token.kind == SyntaxKind::Whitespace {
                    return None;
                }
                Some(serde_json::json!({
                    "class": class.name(),
                    "start": offsets.to_utf16(u32::from(token.range.start())),
                    "end": offsets.to_utf16(u32::from(token.range.end())),
                }))
            })
            .collect();

        let mut json = serde_json::to_string_pretty(&tokens)?;
        json.push('\n');
        generated(&ui(&format!("fixtures/lexed/{stem}.json")), &json)?;
    }
    Ok(())
}
