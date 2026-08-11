//! Folding ranges from the concrete syntax tree.
//!
//! Braces are the language's explicit structure, so a fold is a matched
//! `{ … }` pair on different lines — `piece`, `score`, `part`, `voice`,
//! `motif`, `studio`, and every transform block the grammar grows later fold
//! by virtue of being braced, with no list of keywords to go stale. A run of
//! comment-only lines folds as one comment range. The walk reads the lossless
//! tree and nothing else: no session facts, no compile, so a half-typed
//! document folds whatever braces it has already closed.

use std::collections::BTreeSet;

use lsp_types::{FoldingRange, FoldingRangeKind};
use musa_language::SyntaxKind;

use crate::convert::LineIndex;
use crate::workspace::Document;

/// Every fold the document offers.
pub(crate) fn folding_ranges(document: &Document) -> Option<Vec<FoldingRange>> {
    let snapshot = document.snapshot();
    let source = snapshot.source();
    let tree = musa_language::parse(source);
    let index = LineIndex::new(source);
    let mut ranges = brace_folds(&tree.syntax(), &index);
    ranges.extend(comment_folds(&tree.syntax(), &index));
    ranges.sort_by_key(|range| range.start_line);
    Some(ranges)
}

/// A `region` fold for every matched brace pair whose lines differ. Matching
/// is the walk's own stack, so an unclosed `{` — the composer mid-thought —
/// simply never pairs, and a stray `}` pairs with nothing.
fn brace_folds(tree: &musa_language::SyntaxNode, index: &LineIndex) -> Vec<FoldingRange> {
    let mut open: Vec<u32> = Vec::new();
    let mut ranges = Vec::new();
    for element in tree.descendants_with_tokens() {
        let Some(token) = element.into_token() else {
            continue;
        };
        let kind = token.kind();
        if kind == SyntaxKind::LBrace {
            open.push(u32::from(token.text_range().start()));
        } else if kind == SyntaxKind::RBrace
            && let Some(start) = open.pop()
        {
            let (start_line, end_line) = (
                index.position(start).line,
                index.position(u32::from(token.text_range().start())).line,
            );
            if start_line < end_line {
                ranges.push(FoldingRange {
                    start_line,
                    start_character: None,
                    end_line,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None,
                });
            }
        }
    }
    ranges
}

/// A `comment` fold for every run of two or more comment-only lines.
///
/// "Comment-only" is said from the tree: a line some comment covers and no
/// non-trivia token touches. Line runs are computed on line numbers, so a
/// block comment's middle lines are comment lines too.
fn comment_folds(tree: &musa_language::SyntaxNode, index: &LineIndex) -> Vec<FoldingRange> {
    let mut code_lines: BTreeSet<u32> = BTreeSet::new();
    let mut comment_lines: BTreeSet<u32> = BTreeSet::new();
    for element in tree.descendants_with_tokens() {
        let Some(token) = element.into_token() else {
            continue;
        };
        if token.kind() == SyntaxKind::Whitespace {
            continue;
        }
        let (first, last) = (
            index.position(u32::from(token.text_range().start())).line,
            index
                .position(u32::from(token.text_range().end()).saturating_sub(1))
                .line,
        );
        let lines = if matches!(token.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment) {
            &mut comment_lines
        } else {
            &mut code_lines
        };
        for line in first..=last {
            lines.insert(line);
        }
    }
    let comment_only: Vec<u32> = comment_lines.difference(&code_lines).copied().collect();
    let mut ranges = Vec::new();
    // One pass, flushing each run at its end. The sentinel line after the
    // loop is what flushes the run that reaches the document's end.
    let mut run_start: Option<u32> = None;
    let mut run_end: u32 = 0;
    for line in comment_only.into_iter().chain(std::iter::once(u32::MAX)) {
        match run_start {
            Some(_) if line == run_end.saturating_add(1) => run_end = line,
            Some(start) => {
                if start < run_end {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: run_end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Comment),
                        collapsed_text: None,
                    });
                }
                run_start = (line != u32::MAX).then_some(line);
                run_end = line;
            }
            None => {
                run_start = (line != u32::MAX).then_some(line);
                run_end = line;
            }
        }
    }
    ranges
}
