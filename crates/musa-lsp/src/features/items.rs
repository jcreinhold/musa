//! The declaration under the caret, and what an editor says about it.
//!
//! One lookup and one rendering, shared by hover, signature help, completion,
//! and the outline — because the four are four presentations of the same five
//! sentences, and a shell that wrote them four times would disagree with
//! itself the first time one changed.
//!
//! Nothing here reads Musa source. The session's item facts already carry the
//! signature the checker settled on, the summary written above the
//! declaration, and where it is written; this module turns them into markdown.
//! That is the whole of the "no second opinion" rule, applied to the one
//! feature most tempted to break it: before these facts existed, hover
//! answered for an imported name by scanning the bundled source for a line
//! starting with `fn`.

use musa_project::{ItemFact, NameFact, ProjectSnapshot, Span};

use crate::convert::{LineIndex, covers};

/// The declaration named at `byte`, and the name token that named it.
///
/// Found through the session's own name record: a use, or the declaration
/// itself. A word that resolved to nothing has no record and no answer here,
/// which is the honest result — the alternative is to guess from spelling.
pub(crate) fn at<'a>(snapshot: &'a ProjectSnapshot<'a>, byte: u32) -> Option<(&'a ItemFact, Span)> {
    let name = snapshot.names().iter().find(|name| {
        name.declaration.is_some_and(|span| covers(span, byte)) || name.uses.iter().any(|span| covers(*span, byte))
    })?;
    let span = name
        .declaration
        .filter(|span| covers(*span, byte))
        .or_else(|| name.uses.iter().copied().find(|span| covers(*span, byte)))?;
    Some((of(snapshot, name)?, span))
}

/// The record for one recorded name, matched by what it is called and what it
/// names — the pair item facts are unique by.
pub(crate) fn of<'a>(snapshot: &'a ProjectSnapshot<'_>, name: &NameFact) -> Option<&'a ItemFact> {
    snapshot
        .items()
        .iter()
        .find(|item| item.name == name.name && item.kind == name.kind)
}

/// The record for a name written as text — what a call site has to go on.
pub(crate) fn named<'a>(snapshot: &'a ProjectSnapshot<'_>, name: &str) -> Option<&'a ItemFact> {
    snapshot.items().iter().find(|item| item.name == name)
}

/// Everything the editor says about one declaration, as markdown.
///
/// The order is the order a reader needs it in: what to write instead, if the
/// declaration says it is superseded; then the signature, because that is the
/// question; then the prose; then the one line that tells this type from the
/// one it is confused with; then where it is written.
pub(crate) fn markdown(item: &ItemFact) -> String {
    let mut text = String::new();
    if let Some(instead) = &item.deprecation {
        text.push_str("**Deprecated** — ");
        text.push_str(instead);
        text.push_str("\n\n");
    }
    text.push_str("```musa\n");
    text.push_str(&item.signature);
    text.push_str("\n```");
    if let Some(summary) = &item.summary {
        text.push_str("\n\n");
        text.push_str(summary);
    }
    // The distinction, and only when there is one to draw. `NoteName` against
    // `Pc12` and `ChordClass` against `Voicing` are the confusions this line
    // exists for; a `Music` needs no disambiguation and gets no sentence.
    if let Some(result) = &item.result
        && let Some(distinction) = &result.distinction
    {
        text.push_str("\n\n`");
        text.push_str(&result.name);
        text.push_str("` — ");
        text.push_str(distinction);
    }
    if let Some(where_from) = origin_line(item) {
        text.push_str("\n\n");
        text.push_str(&where_from);
    }
    text
}

/// Where the declaration is written, as one italic line.
///
/// A bundled module says so and links to its virtual document, at the line the
/// declaration is on: the text is compiled into the binary, so the link is the
/// only way to reach it and an edit has nowhere to land. An ordinary import
/// names its path. A local declaration says nothing — the reader is looking at
/// it.
fn origin_line(item: &ItemFact) -> Option<String> {
    let uri = item.uri.as_deref()?;
    if !item.read_only {
        return Some(format!("*Imported from `{uri}`*"));
    }
    let source = musa_project::standard_library_source(uri)?;
    let line = LineIndex::new(source).range(item.span).start.line.saturating_add(1);
    Some(format!("*Bundled Musa source · read-only* — [{uri}]({uri}#L{line})"))
}
