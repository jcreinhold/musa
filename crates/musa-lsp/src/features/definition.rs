//! Go to definition: from a use to the thing used.
//!
//! Two jumps, both provenance read inward: a `use`
//! statement goes to the motif it expands, and a generated note's statement
//! in the source goes to the statement inside the motif body that spells it —
//! the edit-definition edit's target, so "go to definition" and "edit the
//! definition" point at the same place.

use std::str::FromStr as _;

use lsp_types::{GotoDefinitionResponse, Location, Position, Uri};
use musa_project::ScoreFacts;

use crate::convert::{LineIndex, covers};
use crate::workspace::Document;

/// The definition of the thing at `position`, when it has one.
pub(crate) fn definition(document: &Document, uri: &Uri, position: Position) -> Option<GotoDefinitionResponse> {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    if let Some(answer) = at_named_definition(&snapshot, byte, uri, document.lines()) {
        return Some(answer);
    }
    if let Some(answer) = at_studio_vocabulary(&snapshot, byte) {
        return Some(answer);
    }
    let score = snapshot.score()?;
    let lines = document.lines();
    at_use_site(score, byte, lines, uri).or_else(|| at_generated_event(score, byte, lines, uri))
}

/// A processor, parameter, or studio term is a row of the ordinary checked
/// source value, not a host declaration. Open the declaration that owns those
/// rows; the virtual document then exposes the complete exact contract.
fn at_studio_vocabulary(snapshot: &musa_project::ProjectSnapshot<'_>, byte: u32) -> Option<GotoDefinitionResponse> {
    const URI: &str = "musa-stdlib:/std/sound/catalogue.musa";
    const DECLARATION: &str = "studio_vocabulary";

    let parsed = musa_syntax::parse(snapshot.source());
    let token = parsed.syntax().token_at_offset(byte.into()).find(|token| {
        matches!(
            token.kind(),
            musa_syntax::SyntaxKind::Identifier
                | musa_syntax::SyntaxKind::ScaleKw
                | musa_syntax::SyntaxKind::StudioKw
                | musa_syntax::SyntaxKind::PatchKw
                | musa_syntax::SyntaxKind::ModulateKw
                | musa_syntax::SyntaxKind::BusKw
                | musa_syntax::SyntaxKind::AssignKw
                | musa_syntax::SyntaxKind::SendKw
                | musa_syntax::SyntaxKind::RouteKw
                | musa_syntax::SyntaxKind::MasterKw
                | musa_syntax::SyntaxKind::AtKw
                | musa_syntax::SyntaxKind::OutputKw
                | musa_syntax::SyntaxKind::UnitHz
                | musa_syntax::SyntaxKind::UnitMs
                | musa_syntax::SyntaxKind::UnitS
                | musa_syntax::SyntaxKind::UnitDb
        )
    })?;
    let word = token.text();
    let vocabulary = musa_project::standard_studio_vocabulary().ok()?;
    let is_entry = vocabulary.processor(word).is_some()
        || vocabulary.term(word).is_some()
        || super::call::at(&parsed.syntax(), byte).is_some_and(|call| {
            vocabulary
                .processor(&call.name)
                .is_some_and(|processor| processor.parameters().iter().any(|parameter| parameter.name() == word))
        });
    if !is_entry {
        return None;
    }

    let source = musa_project::standard_library_source(URI)?;
    let start = source.find(DECLARATION)? as u32;
    let uri = Uri::from_str(URI).ok()?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri,
        range: LineIndex::new(source).range(musa_project::Span {
            start,
            end: start.saturating_add(DECLARATION.len() as u32),
        }),
    }))
}

fn at_named_definition(
    snapshot: &musa_project::ProjectSnapshot<'_>,
    byte: u32,
    local_uri: &Uri,
    local_lines: &LineIndex,
) -> Option<GotoDefinitionResponse> {
    let name = snapshot.names().iter().find(|name| {
        name.declaration.is_some_and(|span| covers(span, byte)) || name.uses.iter().any(|span| covers(*span, byte))
    })?;
    if let Some(span) = name.declaration {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: local_uri.clone(),
            range: local_lines.range(span),
        }));
    }
    let external = name.external_declaration.as_ref()?;
    let source = musa_project::standard_library_source(&external.uri)?;
    let uri = Uri::from_str(&external.uri).ok()?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri,
        range: LineIndex::new(source).range(external.span),
    }))
}

fn at_use_site(score: &ScoreFacts, byte: u32, lines: &LineIndex, uri: &Uri) -> Option<GotoDefinitionResponse> {
    let declaration = score
        .occurrences
        .iter()
        .find(|occurrence| covers(occurrence.use_site, byte))
        .and_then(|occurrence| occurrence.declaration)?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: lines.range(declaration),
    }))
}

/// A generated event's `span` is the statement that transitively produced it;
/// when that is where the caret sits, its definition is the spelling
/// statement inside the motif body.
fn at_generated_event(score: &ScoreFacts, byte: u32, lines: &LineIndex, uri: &Uri) -> Option<GotoDefinitionResponse> {
    let event = score.events.iter().find(|event| {
        event.origin.generated && covers(event.origin.span, byte) && event.origin.definition_span != event.origin.span
    })?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: lines.range(event.origin.definition_span),
    }))
}
