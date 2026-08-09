//! Go to definition: from a use to the thing used.
//!
//! Two jumps, both provenance read inward (prompt 24's vocabulary): a `use`
//! statement goes to the motif it expands, and a generated note's statement
//! in the source goes to the statement inside the motif body that spells it —
//! the edit-definition edit's target, so "go to definition" and "edit the
//! definition" point at the same place.

use lsp_types::{GotoDefinitionResponse, Location, Position, Uri};
use musa_project::ScoreFacts;

use crate::convert::{LineIndex, covers};
use crate::workspace::Document;

/// The definition of the thing at `position`, when it has one.
pub(crate) fn definition(document: &Document, uri: &Uri, position: Position) -> Option<GotoDefinitionResponse> {
    let byte = document.lines().byte(position);
    let snapshot = document.snapshot();
    let score = snapshot.score()?;
    let lines = document.lines();
    at_use_site(score, byte, lines, uri).or_else(|| at_generated_event(score, byte, lines, uri))
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
