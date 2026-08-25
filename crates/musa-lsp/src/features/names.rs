//! References and rename over the session's name facts.
//!
//! No scanning of our own: the resolver's record *is* the references, and a
//! rename rewrites exactly the spans it recorded. Two callers share the
//! record-lookup — references, and prepare/rename (rename = references plus
//! one byte-level rewrite and one name-legality check) — so both live here.

use lsp_types::{
    Location, Position, PrepareRenameResponse, ReferenceParams, RenameParams, TextEdit, Uri, WorkspaceEdit,
};
use musa_project::NameFact;
use std::str::FromStr as _;

use crate::convert;
use crate::workspace::Document;

/// Find every place a name is spoken: declaration plus resolved uses.
pub(crate) fn references(document: &Document, uri: &Uri, params: &ReferenceParams) -> Option<Vec<Location>> {
    let (entry, _) = entry_at(document, params.text_document_position.position)?;
    let mut locations: Vec<Location> = Vec::new();
    if params.context.include_declaration
        && let Some(span) = entry.declaration
    {
        locations.push(location(uri, document, span));
    }
    if params.context.include_declaration
        && let Some(external) = &entry.external_declaration
        && let Some(source) = document.snapshot().cause_source(&external.uri)
        && let Ok(external_uri) = Uri::from_str(&external.uri)
    {
        locations.push(Location {
            uri: external_uri,
            range: crate::convert::LineIndex::new(source).range(external.span),
        });
    }
    locations.extend(entry.uses.iter().map(|span| location(uri, document, *span)));
    Some(locations)
}

/// Is the cursor on a renameable name, and over what range.
pub(crate) fn prepare_rename(document: &Document, position: Position) -> Option<PrepareRenameResponse> {
    let (entry, span) = entry_at(document, position)?;
    if entry.external_declaration.is_some() {
        return None;
    }
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: document.lines().range(span),
        placeholder: entry.name.clone(),
    })
}

/// Rewrite every recorded spelling of the name — declaration and uses.
///
/// Refuses, rather than guessing, when the new name is not a single legal
/// identifier or would collide with a name in the same namespace: the
/// resolver's duplicates check would fire on the result, and the edit would
/// leave the document worse than it found it.
pub(crate) fn rename(document: &Document, params: &RenameParams) -> Result<Option<WorkspaceEdit>, String> {
    let (entry, _) = entry_at(document, params.text_document_position.position)
        .ok_or_else(|| "there is no named thing at the cursor, or the document has never compiled".to_owned())?;
    if entry.external_declaration.is_some() {
        return Err("imported source is read-only here; define a local wrapper instead".to_owned());
    }
    check_new_name(document, entry, &params.new_name)?;
    let mut edits: Vec<TextEdit> = Vec::new();
    if let Some(span) = entry.declaration {
        edits.push(edit(document, span, &params.new_name));
    }
    edits.extend(entry.uses.iter().map(|span| edit(document, *span, &params.new_name)));
    Ok(Some(WorkspaceEdit::new(std::collections::HashMap::from([(
        params.text_document_position.text_document.uri.clone(),
        edits,
    )]))))
}

/// The fact whose declaration or any use covers the position, and the span
/// it covered — the session's own answer to "which name is this".
fn entry_at(document: &Document, position: Position) -> Option<(&NameFact, musa_project::Span)> {
    let byte = document.lines().byte(position);
    let entry = document.snapshot().names().iter().find(|entry| {
        entry.declaration.is_some_and(|span| convert::covers(span, byte))
            || entry.uses.iter().any(|span| convert::covers(*span, byte))
    })?;
    let span = if entry.declaration.is_some_and(|span| convert::covers(span, byte)) {
        entry.declaration?
    } else {
        *entry.uses.iter().find(|span| convert::covers(**span, byte))?
    };
    Some((entry, span))
}

fn location(uri: &Uri, document: &Document, span: musa_project::Span) -> Location {
    Location {
        uri: uri.clone(),
        range: document.lines().range(span),
    }
}

fn edit(document: &Document, span: musa_project::Span, new_name: &str) -> TextEdit {
    TextEdit {
        range: document.lines().range(span),
        new_text: new_name.to_owned(),
    }
}

/// Would `new_name` still be this name: one identifier token, and free in
/// the name's namespace.
fn check_new_name(document: &Document, entry: &NameFact, new_name: &str) -> Result<(), String> {
    let tokens = musa_syntax::lex(new_name);
    let [token] = tokens.tokens() else {
        return Err(format!("`{new_name}` is not a valid name"));
    };
    if token.kind != musa_syntax::SyntaxKind::Identifier {
        return Err(format!("`{new_name}` is not a valid name"));
    }
    let names = document.snapshot().names();
    let collides = |kind: musa_project::NameKind| {
        names
            .iter()
            .any(|other| other.kind == kind && other.name == new_name && other.name != entry.name)
    };
    // Elaborated values and functions share the value namespace. Motifs,
    // bars, and fragments retain their existing material namespace; the
    // remaining kinds each have their own.
    let collision = match entry.kind {
        musa_project::NameKind::Value | musa_project::NameKind::Function => {
            collides(musa_project::NameKind::Value) || collides(musa_project::NameKind::Function)
        }
        musa_project::NameKind::Motif | musa_project::NameKind::Bar | musa_project::NameKind::Fragment => {
            collides(musa_project::NameKind::Motif)
                || collides(musa_project::NameKind::Bar)
                || collides(musa_project::NameKind::Fragment)
        }
        kind @ (musa_project::NameKind::Part
        | musa_project::NameKind::Voice
        | musa_project::NameKind::Patch
        | musa_project::NameKind::Module
        | musa_project::NameKind::Template) => collides(kind),
    };
    if collision {
        return Err(format!("`{new_name}` already names something here"));
    }
    Ok(())
}
