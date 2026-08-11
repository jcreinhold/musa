//! Analysis on request: a lens to ask with, and a command that answers.
//!
//! An analysis is an *observation*, not a verdict. `docs/language/07-analysis.md`
//! is emphatic about it: a cadence with two of its three kinds of evidence is a
//! finding with an absence recorded, not a mistake, and a chord reading that
//! two keys support is two candidates rather than one error. So the answer
//! travels as the command's own result, typed, and never through
//! `publishDiagnostics` — the problems pane means "this is wrong", and nothing
//! an analysis sees is.
//!
//! Asking is explicit for the same reason. Analysis costs real work and states
//! assumptions the reader has to agree to; running all six kinds on every
//! keystroke would spend that work on a question nobody asked, and would put
//! its assumptions where nobody reads them. A lens per kind, at the piece it
//! reads, is the smallest surface that keeps the asking deliberate.

use lsp_types::{CodeLens, Command, Uri};
use musa_project::AnalysisKind;
use serde_json::Value;

use crate::workspace::Document;

/// The command an analysis lens invokes. Clients that offer analysis outside
/// the lens — a palette entry, a keybinding — send this too.
pub(crate) const ANALYZE: &str = "musa.analyze";

/// One lens per analysis kind, on the line the piece is declared.
///
/// Nothing for a document that has never compiled: an analysis reads the last
/// valid score, and offering to read a score that does not exist would put the
/// failure in the result of a command rather than in the problems pane where
/// the reader is already looking.
pub(crate) fn code_lenses(document: &Document, uri: &Uri) -> Option<Vec<CodeLens>> {
    let snapshot = document.snapshot();
    // A score, because there is nothing to analyze without one — and the
    // `piece` keyword, because the facts carry no span for the declaration
    // itself and a lens has to sit somewhere a reader is looking. Finding the
    // keyword is reading, which is what the lossless tree is for.
    snapshot.score()?;
    let parsed = musa_language::parse(snapshot.source());
    let keyword = parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == musa_language::SyntaxKind::PieceKw)?;
    let range = document.lines().range(musa_project::Span {
        start: u32::from(keyword.text_range().start()),
        end: u32::from(keyword.text_range().end()),
    });
    Some(
        AnalysisKind::ALL
            .into_iter()
            .map(|kind| CodeLens {
                range,
                command: Some(Command {
                    title: format!("Analyze: {}", kind.as_str()),
                    command: ANALYZE.to_owned(),
                    arguments: Some(vec![
                        Value::String(uri.to_string()),
                        Value::String(kind.as_str().to_owned()),
                    ]),
                }),
                data: None,
            })
            .collect(),
    )
}

/// Run one analysis over the document's last valid score.
///
/// The arguments are the lens's own: the document's URI and the kind's
/// command-line spelling. Everything else is the default request — the whole
/// score, segmented at attacks — because a lens has nowhere to put a
/// narrowing and a client that wants one sends the command itself.
///
/// # Errors
/// The message the session gave, when the piece has never compiled or the
/// request does not fit the score. A refused analysis is a refusal, not an
/// empty report: a reader shown zero findings would conclude the music is
/// clean.
pub(crate) fn execute(document: &Document, kind: &str) -> Result<Value, String> {
    let kind = AnalysisKind::named(kind).ok_or_else(|| format!("`{kind}` is not an analysis this compiler runs"))?;
    let facts = document
        .session()
        .analyze(&musa_project::AnalysisRequest::new(kind))
        .map_err(|error| error.to_string())?;
    serde_json::to_value(facts).map_err(|error| error.to_string())
}
