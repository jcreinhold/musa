//! The explicit, compiler-proved barline source rewrite.

use musa_compiler::{BarlineSourceItem, BarlineSourceRole};
use musa_score::MusicalTime;

use crate::{Revision, Span, TextEdit};

/// Why a barline rewrite has no certain transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarlineBlockerReason {
    /// The current text has no successful checked score at this revision.
    InvalidDocument,
    /// The preview belongs to an earlier revision.
    StaleRevision,
    /// A measured loose run starts between barlines.
    StartsBetweenBarlines,
    /// A measured loose run ends between barlines.
    EndsBetweenBarlines,
    /// One direct item contains a barline and would need to be opened or split.
    ItemCrossesBoundary,
    /// An adapter-produced item owns an insertion boundary rather than source.
    GeneratedBoundary,
}

/// A certain refusal of the semantic barline rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarlineBlocker {
    reason: BarlineBlockerReason,
    span: Option<Span>,
}

impl BarlineBlocker {
    pub(crate) const fn new(reason: BarlineBlockerReason, span: Option<Span>) -> Self {
        Self { reason, span }
    }

    /// The machine-readable reason.
    pub const fn reason(&self) -> BarlineBlockerReason {
        self.reason
    }

    /// The first relevant source range, when the refusal belongs to an item.
    pub const fn span(&self) -> Option<Span> {
        self.span
    }
}

impl std::fmt::Display for BarlineBlocker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.reason {
            BarlineBlockerReason::InvalidDocument => "the current document does not have a valid checked score",
            BarlineBlockerReason::StaleRevision => "the barline preview is stale; recompute it for this revision",
            BarlineBlockerReason::StartsBetweenBarlines => {
                "a loose passage begins between barlines; Musa will not invent a pickup"
            }
            BarlineBlockerReason::EndsBetweenBarlines => {
                "a loose passage ends between barlines; the closing measure is incomplete"
            }
            BarlineBlockerReason::ItemCrossesBoundary => {
                "a direct source item crosses a required barline and cannot be split automatically"
            }
            BarlineBlockerReason::GeneratedBoundary => {
                "generated material hides a required barline; edit the authored construct instead"
            }
        })
    }
}

impl std::error::Error for BarlineBlocker {}

/// One immutable preview of the proved, canonically formatted rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarlineRewrite {
    revision: Revision,
    edits: Vec<TextEdit>,
    source: String,
    inserted: usize,
}

impl BarlineRewrite {
    /// The revision whose byte ranges and checked facts this plan describes.
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    /// The exact edits every shell applies. Applying these to the previewed
    /// source produces [`Self::source`].
    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    /// The complete canonically formatted source after insertion.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Number of new `|` assertions, and therefore measures made explicit.
    pub const fn inserted(&self) -> usize {
        self.inserted
    }

    /// A short musical summary for command palettes and terminal previews.
    pub fn summary(&self) -> String {
        let noun = if self.inserted == 1 { "bar line" } else { "bar lines" };
        let measures = if self.inserted == 1 { "measure" } else { "measures" };
        format!("Insert {} {noun} in {} {measures}", self.inserted, self.inserted)
    }
}

pub(crate) fn plan(
    source: &str,
    revision: Revision,
    items: &[BarlineSourceItem],
    spacing: musa_syntax::BarSpacing,
) -> Result<Option<BarlineRewrite>, BarlineBlocker> {
    let mut insertions = Vec::new();
    if let Some(item) = items
        .iter()
        .find(|item| !item.is_authored() && item.is_measured() && item.internal_boundary().is_some())
    {
        return Err(blocked(BarlineBlockerReason::GeneratedBoundary, item));
    }
    let mut cursor: usize = 0;
    while let Some(item) = items.get(cursor) {
        if item.role() != BarlineSourceRole::Loose {
            cursor = cursor.saturating_add(1);
            continue;
        }
        let scope = item.scope();
        let start = cursor;
        while items
            .get(cursor)
            .is_some_and(|next| next.scope() == scope && next.role() == BarlineSourceRole::Loose)
        {
            cursor = cursor.saturating_add(1);
        }
        if let Some(run) = items.get(start..cursor) {
            plan_run(run, &mut insertions)?;
        }
    }
    if insertions.is_empty() {
        return Ok(None);
    }
    insertions.sort_unstable();
    insertions.dedup();
    let inserted = insertions.len();
    let mut insertion_edits: Vec<musa_syntax::TextEdit> = insertions
        .into_iter()
        .map(|at| musa_syntax::TextEdit::new(text_size::TextRange::empty(at.into()), "| "))
        .collect();
    insertion_edits.sort_by_key(|edit| std::cmp::Reverse(edit.range.start()));
    let with_bars = musa_syntax::apply_edits(source, &insertion_edits);
    let formatted = musa_compiler::format_document(&with_bars, spacing).unwrap_or(with_bars);
    let edits = vec![TextEdit::new(
        Span {
            start: 0,
            end: u32::try_from(source.len()).unwrap_or(u32::MAX),
        },
        formatted.clone(),
    )];
    Ok(Some(BarlineRewrite {
        revision,
        edits,
        source: formatted,
        inserted,
    }))
}

fn plan_run(run: &[BarlineSourceItem], insertions: &mut Vec<u32>) -> Result<(), BarlineBlocker> {
    let Some(first) = run.first() else { return Ok(()) };
    if !run.iter().any(BarlineSourceItem::is_measured) {
        return Ok(());
    }
    if !first.is_measured() {
        return Ok(());
    }
    if !first.starts_on_barline() {
        return Err(blocked(BarlineBlockerReason::StartsBetweenBarlines, first));
    }
    let last = run.last().unwrap_or(first);
    if !last.ends_on_barline() {
        return Err(blocked(BarlineBlockerReason::EndsBetweenBarlines, last));
    }
    let mut last_boundary: Option<MusicalTime> = None;
    for item in run {
        if item.internal_boundary().is_some() {
            let reason = if !item.is_authored() || item.contains_generated_content() {
                BarlineBlockerReason::GeneratedBoundary
            } else {
                BarlineBlockerReason::ItemCrossesBoundary
            };
            return Err(blocked(reason, item));
        }
        if item.starts_on_barline() && item.start() < last.end() && last_boundary != Some(item.start()) {
            if !item.is_authored() {
                return Err(blocked(BarlineBlockerReason::GeneratedBoundary, item));
            }
            insertions.push(item.span().start);
            last_boundary = Some(item.start());
        }
    }
    Ok(())
}

fn blocked(reason: BarlineBlockerReason, item: &BarlineSourceItem) -> BarlineBlocker {
    let span = item.span();
    BarlineBlocker::new(
        reason,
        Some(Span {
            start: span.start,
            end: span.end,
        }),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn summary_pluralizes() {
        let rewrite = super::BarlineRewrite {
            revision: crate::Revision(1),
            edits: Vec::new(),
            source: String::new(),
            inserted: 1,
        };
        assert_eq!(rewrite.summary(), "Insert 1 bar line in 1 measure");
    }
}
