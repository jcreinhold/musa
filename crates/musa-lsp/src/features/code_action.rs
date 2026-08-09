//! Code actions: the certain fixes the diagnostics already carry.
//!
//! The session offers a fix only when it resolves the diagnostic without
//! guesswork (prompt 56); this handler offers exactly those, no more. A fix
//! menu that ranks guesses is an editor feature the session deliberately does
//! not have, so the server does not have one either.

use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Range, TextEdit, Uri, WorkspaceEdit,
};
use musa_project::{Diagnostic, Span};

use crate::convert::LineIndex;
use crate::workspace::Document;

/// The quick fixes for the diagnostics touching `range`.
///
/// Matching is by overlap with the current source's diagnostics rather than
/// by identity with the client's list: the client's context is a hint about
/// diagnostics it may have received before the last keystroke, and the
/// current ones are the truth.
pub(crate) fn code_actions(
    document: &Document,
    uri: &Uri,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommandList> {
    let actions = document
        .snapshot()
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .span
                .is_some_and(|span| overlaps(span, params.range, document.lines()))
        })
        .flat_map(|diagnostic| fixes(document, uri, diagnostic))
        .map(CodeActionOrCommand::CodeAction)
        .collect::<Vec<_>>();
    if actions.is_empty() { None } else { Some(actions) }
}

/// The response type, named once.
type CodeActionOrCommandList = Vec<CodeActionOrCommand>;

/// Whether the diagnostic's span and the requested range share a position.
fn overlaps(span: Span, range: Range, lines: &LineIndex) -> bool {
    let start = lines.byte(range.start);
    let end = lines.byte(range.end);
    span.start < end.max(start.saturating_add(1)) && start < span.end.max(span.start.saturating_add(1))
}

/// One action per certain fix.
fn fixes(document: &Document, uri: &Uri, diagnostic: &Diagnostic) -> Vec<CodeAction> {
    let lsp_diagnostic = super::diagnostics::to_lsp(diagnostic, uri, document.lines());
    let single = diagnostic.fixes.len() == 1;
    diagnostic
        .fixes
        .iter()
        .map(|fix| {
            let edits = fix
                .edits
                .iter()
                .map(|edit| TextEdit {
                    range: document.lines().range(edit.span),
                    new_text: edit.replacement.clone(),
                })
                .collect();
            CodeAction {
                title: fix.title.clone(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![lsp_diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri.clone(), edits)])),
                    document_changes: None,
                    change_annotations: None,
                }),
                // One fix is *the* fix; several are choices, and preferring
                // one of them is how an editor applies the wrong one.
                is_preferred: if single { Some(true) } else { None },
                ..CodeAction::default()
            }
        })
        .collect()
}
