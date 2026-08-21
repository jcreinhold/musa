//! The WebAssembly shell: one small module that carries the
//! whole semantic pipeline — parse, compile, notation plan, MEI render — into
//! the browser for `@musa/web`. A shell like `musa` and `musa-lsp`: it
//! depends on `musa-compiler` and `musa-notation`, never the reverse, and adds
//! no semantics of its own.
//!
//! Owns: the wasm-crossing diagnostic type and the three functions of the
//! boundary. Must never contain: musical semantics, notation planning, DOM or
//! worker code (that is the TypeScript layer's job), audio, or filesystem
//! access — a snippet is one self-contained string.
//!
//! Invariants: [`TypesetResult::mei`] is present exactly when the source
//! compiles to a score with no error-severity diagnostics; a material
//! document (`library { … }`) yields `mei: None` with no error, because "no
//! score ever" is not a failure; spans are byte offsets — line/column is the
//! display layer's job, computed from source the page already holds.

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_notation::{NotationOptions, NotationTarget, render_notation};
use musa_score::{Diagnostic, Severity};
use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// The document name used in diagnostics. Snippets have no meaningful path;
/// a stable placeholder keeps diagnostic text honest about that.
const SNIPPET_NAME: &str = "snippet.musa";

/// What one snippet compiles to.
///
/// `mei` is present exactly when the source compiles to a score with no
/// error-severity diagnostics; warnings travel alongside the MEI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypesetResult {
    /// The Verovio-compatible MEI, or `None` when there is nothing to
    /// engrave (compile failure, or a material document).
    pub mei: Option<String>,
    /// Every diagnostic, parse and semantic, in source order.
    pub diagnostics: Vec<WebDiagnostic>,
}

/// A diagnostic that can cross the wasm boundary.
///
/// Fixes are not serialized: no web caller can apply them (the web package
/// renders; it does not edit).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WebDiagnostic {
    /// `"error"` or `"warning"`.
    pub severity: String,
    /// The stable diagnostic code, or `"render"` for backend failures.
    pub code: String,
    /// What is wrong, in the compiler's own words.
    pub message: String,
    /// Where. The primary label, when there is one, is first.
    pub labels: Vec<WebLabel>,
    /// What to do about it.
    pub help: Option<String>,
    /// The rule behind it.
    pub note: Option<String>,
}

/// One labelled place in the snippet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WebLabel {
    /// First byte of the span.
    pub start: u32,
    /// One past the last byte of the span.
    pub end: u32,
    /// What is wrong *here*.
    pub text: String,
    /// Whether this is the place to look first.
    pub primary: bool,
}

impl WebDiagnostic {
    fn from_compiler(diagnostic: &Diagnostic) -> Self {
        Self {
            severity: match diagnostic.severity {
                Severity::Error => "error".to_owned(),
                Severity::Warning => "warning".to_owned(),
            },
            code: diagnostic.code.as_str().to_owned(),
            message: diagnostic.message.clone(),
            labels: diagnostic
                .labels
                .iter()
                .map(|label| WebLabel {
                    start: label.span.start,
                    end: label.span.end,
                    text: label.text.clone(),
                    primary: label.primary,
                })
                .collect(),
            help: diagnostic.help.clone(),
            note: diagnostic.note.clone(),
        }
    }

    fn render_failure(message: String) -> Self {
        Self {
            severity: "error".to_owned(),
            code: "render".to_owned(),
            message,
            labels: Vec::new(),
            help: None,
            note: None,
        }
    }
}

/// Typeset one snippet: the native-testable core of [`typeset`].
/// `#[doc(hidden)]`: public so the integration tests reach it, not part of
/// the crate's documented surface.
#[doc(hidden)]
pub fn typeset_impl(source: &str) -> TypesetResult {
    let compilation = compile(&SourceDocument::new(source, SNIPPET_NAME), &CompileOptions::default());
    let mut diagnostics: Vec<WebDiagnostic> = compilation
        .diagnostics()
        .iter()
        .map(WebDiagnostic::from_compiler)
        .collect();
    let mei = if compilation.has_errors() {
        None
    } else {
        compilation.snapshot().and_then(|snapshot| {
            match render_notation(snapshot, NotationTarget::Mei, &NotationOptions::default()) {
                Ok(rendered) => Some(rendered.text().to_owned()),
                Err(error) => {
                    diagnostics.push(WebDiagnostic::render_failure(error.to_string()));
                    None
                }
            }
        })
    };
    TypesetResult { mei, diagnostics }
}

/// Validate one snippet: the native-testable core of [`validate`].
#[doc(hidden)]
pub fn validate_impl(source: &str) -> Vec<WebDiagnostic> {
    compile(&SourceDocument::new(source, SNIPPET_NAME), &CompileOptions::default())
        .diagnostics()
        .iter()
        .map(WebDiagnostic::from_compiler)
        .collect()
}

/// Install the panic hook once, so a panic is legible in the console rather
/// than an opaque `unreachable executed`.
#[wasm_bindgen(start)]
pub fn install_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Compile and engrave one self-contained snippet. Returns a
/// [`TypesetResult`] as a plain JS object.
///
/// # Errors
/// Only an internal serialization failure rejects; an invalid score is a
/// *result* (diagnostics, no MEI), never an exception.
#[wasm_bindgen]
pub fn typeset(source: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&typeset_impl(source)).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Validate one snippet without engraving. Returns a list of
/// [`WebDiagnostic`] as plain JS objects.
///
/// # Errors
/// Only an internal serialization failure rejects.
#[wasm_bindgen]
pub fn validate(source: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&validate_impl(source)).map_err(|error| JsValue::from_str(&error.to_string()))
}
