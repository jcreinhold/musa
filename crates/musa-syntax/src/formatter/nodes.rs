//! How a node lays out its children: blocks, chains, horizontal runs, and
//! the one-line rendering the width questions measure.
//!
//! One concern of the `formatter` module; see its docs for the rules.

use super::bars::{beat_group_starts, drawn_to_scale, wrap_at_beat_groups};
use super::lists::{breakable_list, continues_past_a_branch, list_breaks, opens_a_further_rung};
use super::tokens::format_token;
use super::writer::Writer;
use super::{BarSpacing, Layout, MEASURE};
use crate::language::SyntaxNode;
use crate::{SyntaxElement, SyntaxKind};
use std::collections::HashSet;

pub(super) fn format_node(node: &SyntaxNode, writer: &mut Writer, layout: &Layout) {
    // Set once the first token of a one-word construct has been written, so
    // the rest of it joins on without a space.
    let mut tight = false;
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                writer.blank_line_if_pending();
                // A quote is written as it stands. Its interior is the
                // event track's grammar, whose layout the event track's own printer
                // owns (`docs/rules/language/01-surface.md` §7), and a formatter
                // that re-broke those lines by the host's rules would be a
                // second opinion about a shape this crate has no reading of.
                // Only the anchoring is this crate's: the block moves to the
                // indent the host puts it at, keeping its lines' relative
                // depth.
                if child.kind() == SyntaxKind::EventsQuote {
                    write_quote(&child.text().to_string(), writer);
                    continue;
                }
                // An adapter region, for the same reason: what is inside it is
                // the adapter's language, not this one's, and a formatter that
                // re-spaced it would be deciding a shape this crate cannot
                // read. Written as it stands, anchored at the host's indent.
                if child.kind() == SyntaxKind::SyntaxRegion {
                    write_quote(&child.text().to_string(), writer);
                    continue;
                }
                // A splice stands where one node stands
                // (`docs/rules/language/11-quotation.md` §2), so it is written
                // as one node: `$x`, `${ e }`, `$..xs`, with nothing between
                // the `$` and what it splices and no line break inside it. A
                // splice broken across lines would put the break in the middle
                // of an argument, which is the one place the reader is least
                // able to see that a single value stands there.
                if matches!(child.kind(), SyntaxKind::Splice | SyntaxKind::SequenceSplice) {
                    write_splice(&child, writer, layout);
                    continue;
                }
                if writer.starts_a_beat_group(&child) {
                    writer.widen_next_gap();
                }
                // A bar or a grace note owns its line, so its width is
                // measured from the indent. A block is a body written after
                // what it belongs to — `fn f(x: nat) -> nat` — so its width
                // is measured from where the line has already reached. A rung
                // of an `else if` ladder is written after the `else`, which is
                // the same situation: measured from the indent it would look
                // like it fits, and then be written off the right edge, taking
                // the break out on its condition's arguments instead of on the
                // ladder.
                let start = if child.kind() == SyntaxKind::BlockExpr
                    || (child.kind() == SyntaxKind::IfExpr && node.kind() == SyntaxKind::IfExpr)
                {
                    writer.column()
                } else {
                    writer.indent
                };
                if let Some(lines) = inline_run(&child, start, layout).filter(|_| !opens_a_further_rung(node, &child)) {
                    // A lambda's body is the last thing in an expression, not
                    // the last thing on a line: `map(fn (x) { f(x) }, xs)`
                    // continues with a comma. A conditional's branches are the
                    // same — `else` follows one and the enclosing `,` or `;`
                    // follows the other, so neither may end the line it is
                    // written on. A record pattern is a third: the `->` of its
                    // arm follows it.
                    let inline = node.kind() == SyntaxKind::LambdaExpr
                        || continues_past_a_branch(node)
                        || child.kind() == SyntaxKind::RecordPattern;
                    let last = lines.len().saturating_sub(1);
                    for (index, line) in lines.iter().enumerate() {
                        if inline && index == last {
                            writer.write_run(line);
                        } else {
                            writer.write_line(line);
                        }
                    }
                    continue;
                }
                let wrap = wraps_across_lines(&child);
                if wrap {
                    writer.open_chain();
                }
                // A list decides here, where the column it would start at is
                // known, and tells its own commas and parentheses through the
                // writer — they are the tokens that draw the decision, and
                // they are written too far in to measure anything.
                let list = breakable_list(child.kind());
                if list {
                    // A run has no lines to break at: inside one, a list is
                    // whatever the run is, which is one line.
                    let broken = !writer.in_run() && list_breaks(&child, writer, layout);
                    writer.open_list(broken);
                }
                format_node(&child, writer, layout);
                if list {
                    writer.close_list();
                }
                if wrap {
                    writer.close_chain();
                }
                // An event is self-delimiting, so there is no `;` inside it
                // to end its line the way a context statement's does. Where a
                // bar has not already claimed the line, the events are the
                // lines — which is the layout every unbarred voice has now.
                if matches!(
                    child.kind(),
                    SyntaxKind::NoteStmt
                        | SyntaxKind::RestStmt
                        | SyntaxKind::ChordStmt
                        | SyntaxKind::StackStmt
                        | SyntaxKind::BarStmt
                ) {
                    writer.end_line();
                }
            }
            SyntaxElement::Token(token) => {
                let kind = token.kind();
                if kind == SyntaxKind::Whitespace {
                    writer.note_whitespace(token.text());
                    continue;
                }
                writer.blank_line_if_pending();
                // Some constructs are one word with punctuation in them: a
                // `measure:beat` coordinate is written `3:1` the way a bar
                // number is, a chord symbol is `fmaj7` however many tokens it
                // happens to lex as, a modulation target is one path with
                // dots in it, and `M.member` is one name written in two
                // words. Their insides take no spaces.
                //
                // The two paths §1 adds are the same kind of thing.
                // `Tying::Untied` names one case and `region.anchor` names one
                // place, and a `::` or a `.` written with spaces around it
                // would read as an operator between two names rather than as
                // the inside of one.
                if matches!(
                    node.kind(),
                    SyntaxKind::Position
                        | SyntaxKind::ChordSymbol
                        | SyntaxKind::ParamPath
                        | SyntaxKind::PitchClass
                        | SyntaxKind::NameExpr
                        | SyntaxKind::PathExpr
                        | SyntaxKind::FieldPath
                ) {
                    writer.write_word(kind, token.text(), tight);
                    tight = true;
                    continue;
                }
                format_token(node, &token, writer);
            }
        }
    }
}

/// Write one splice as the single node it stands for.
///
/// The braces of `${ e }` are the splice's own punctuation rather than a
/// block's, so they take one space inside and never break; the interior is
/// rendered by [`one_line`], which is the same formatter everything else uses
/// and not a second set of rules for splices. `$x` and `$..xs` are one word
/// with punctuation in them, like `M.member` and `3:1`.
fn write_splice(node: &SyntaxNode, writer: &mut Writer, layout: &Layout) {
    let inner = node.children().next().map(|held| one_line(&held, layout));
    let text = match (node.kind(), inner) {
        (SyntaxKind::SequenceSplice, held) => format!("$..{}", held.unwrap_or_default()),
        // The braces are written back only where they were written: `$x` is
        // the shorthand for a name, and re-spelling it `${ x }` would be the
        // formatter deciding a spelling the author already decided.
        (_, Some(held))
            if node
                .children_with_tokens()
                .any(|piece| piece.kind() == SyntaxKind::LBrace) =>
        {
            format!("${{ {held} }}")
        }
        (_, Some(held)) => format!("${held}"),
        (_, None) => "$".to_owned(),
    };
    writer.write_run(&text);
}

/// Write a quotation verbatim, re-anchored at the writer's indent.
///
/// The first line joins the line in progress — `let doubled: Music = events
/// EventTrack[WrittenTime, ScoreFact] {` — and the rest keep their depth relative to the
/// shallowest of them, which is what makes reformatting a file that only
/// moved sideways leave the quote's shape alone.
fn write_quote(text: &str, writer: &mut Writer) {
    // The node owns the trivia in front of its first token, and that trivia
    // is the host's business, not the quote's.
    let mut lines = text.trim_start().lines();
    let Some(first) = lines.next() else {
        return;
    };
    let rest: Vec<&str> = lines.collect();
    let base = rest
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len().saturating_sub(line.trim_start().len()))
        .min()
        .unwrap_or_default();
    writer.prep_line();
    if writer.needs_word_space() {
        writer.space();
    }
    writer.write(first.trim_end());
    for line in rest {
        writer.end_line();
        writer.prep_line();
        if line.trim().is_empty() {
            continue;
        }
        let depth = line.len().saturating_sub(line.trim_start().len());
        let relative = " ".repeat(depth.saturating_sub(base));
        writer.write(&format!("{relative}{}", line.trim()));
    }
    // No `end_line`: what follows the quote decides. A `;` joins the closing
    // brace, and a block that ends here breaks the line itself.
    writer.after_significant(SyntaxKind::RBrace);
}

/// A run of events written horizontally, as the lines to write for it.
///
/// A bar and a grace group are the two constructs that are *horizontal*: each
/// holds nothing but events, read left to right, and neither ends its items
/// with anything — so stacking them puts one word on each of several lines and
/// leaves the closing brace stranded after the last.
///
/// The run is rendered by the same [`Writer`] that writes everything else,
/// with line breaks suppressed. There is no second copy of the spacing rules
/// to keep in step: a token that closes up to its neighbour does so once, in
/// [`format_token`], and both layouts read it there.
///
/// Returns `None` for anything else, for a run carrying a comment — a comment
/// wants a line of its own and one line has nowhere to put it — and for a
/// *braced* run too wide for [`MEASURE`], which falls back to the way every
/// other block breaks. A `|` bar has no brace to break at, so an over-wide one
/// wraps at its beat groups instead; with no groups to wrap at, the line runs
/// long, because a long line is better than a wrong one.
fn inline_run(node: &SyntaxNode, indent: usize, layout: &Layout) -> Option<Vec<String>> {
    if !matches!(
        node.kind(),
        SyntaxKind::BarStmt | SyntaxKind::GraceStmt | SyntaxKind::BlockExpr | SyntaxKind::RecordPattern
    ) {
        return None;
    }
    if node
        .descendants_with_tokens()
        .any(|element| matches!(element.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
    {
        return None;
    }
    // A block holds one expression, and a record pattern holds field names
    // read left to right: neither has anything to space by beat group and
    // neither has anything to wrap at, so each is one line when it fits and
    // otherwise breaks at its own braces like every other block.
    //
    // A record pattern is here for the reason this function exists at all. It
    // is followed by the `->` of the arm it opens, so stacking it writes the
    // `}` on a line of its own and leaves the arrow after it — one field per
    // line and a stranded closer, for a construct that is usually two words.
    if matches!(node.kind(), SyntaxKind::BlockExpr | SyntaxKind::RecordPattern) {
        // Notation stays vertical. A block holding a `music` value would
        // otherwise put a voice's notes on one line, which is the one layout
        // this language does not write — only a *bar* is horizontal.
        if node
            .descendants()
            .any(|descendant| descendant.kind() == SyntaxKind::MusicExpr)
        {
            return None;
        }
        let mut writer = Writer::one_line(HashSet::new());
        format_node(node, &mut writer, layout);
        // The run opens with the space its `{` takes after what precedes it,
        // and the caller writes that space itself.
        let line = writer.finish().trim_start().to_owned();
        return (indent.saturating_add(line.chars().count()) <= MEASURE).then_some(vec![line]);
    }
    // The three rungs, in order: to scale if the project asked and the bar
    // fits; else compact if that fits; else wrapped.
    if layout.bars == BarSpacing::Proportional
        && let Some(drawn) = drawn_to_scale(node, layout)
        && indent.saturating_add(drawn.chars().count()) <= MEASURE
    {
        return Some(vec![drawn]);
    }
    let mut writer = Writer::one_line(beat_group_starts(node, &layout.meters));
    format_node(node, &mut writer, layout);
    let line = writer.finish();
    if indent.saturating_add(line.chars().count()) <= MEASURE {
        return Some(vec![line]);
    }
    // A braced run has somewhere to break — its `{` and `}` take the lines a
    // stacked block would give them. A `|` bar has neither, so it wraps.
    let braced = node.children().any(|child| child.kind() == SyntaxKind::Block);
    (!braced).then(|| wrap_at_beat_groups(&line, indent))
}

/// One construct, rendered on a single line by the rules everything else uses.
pub(super) fn one_line(node: &SyntaxNode, layout: &Layout) -> String {
    let mut writer = Writer::one_line(HashSet::new());
    format_node(node, &mut writer, layout);
    writer.finish()
}

/// Whether a signal chain is long enough to be worth stacking.
///
/// Two stages (`oscillator(sine) |> gain(-15 dB)`) read fine on one line;
/// three or more is where a patch stops being a phrase and starts being a
/// signal path, and the roadmap's §7.1 example writes those stacked.
pub(super) fn wraps_across_lines(node: &SyntaxNode) -> bool {
    node.kind() == SyntaxKind::SignalChain
        && node
            .children_with_tokens()
            .filter(|element| element.kind() == SyntaxKind::PipeForward)
            .count()
            >= 2
}
