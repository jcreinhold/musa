//! The lossless formatter (roadmap §11).
//!
//! Operates on the concrete syntax tree — never on any semantic model — and
//! rewrites only whitespace. Rules (encoded and reviewed as insta snapshots):
//!
//! - 4-space indent per block level; `{` stays on the declaration line; `}`
//!   on its own line.
//! - One statement per line, terminated by `;`.
//! - Blank lines from the original are preserved, capped at one.
//! - A comment that trailed code on its line stays trailing; an own-line
//!   comment stays attached above the construct it precedes.
//! - A bar that fits the source measure is written on one line. The one
//!   exception, and see [`MEASURE`] for why it earns itself.
//! - A comma-separated list — a call's arguments, a constructor's fields —
//!   is written on one line when it fits [`MEASURE`], and one item per line
//!   when it does not. See [`breakable_list`].
//! - Inside a bar, the gap between beat groups is two spaces instead of one.
//!   A beam is how notation shows which beats a player hears together, and
//!   horizontal space is the only thing text has to draw one with. Which
//!   notes group is [`beat_groups`]' answer, the same one the engraver beams
//!   by, so the two cannot disagree — and a bar the formatter cannot measure
//!   is written with single spaces rather than a guess.
//!
//! Laws (tested as properties): `format` is idempotent, and
//! `parse(format(parse(source)))` equals `parse(source)` up to whitespace.

use std::collections::{HashMap, HashSet};

use text_size::TextRange;

use crate::language::{SyntaxNode, SyntaxToken};
use crate::meter::beat_groups;
use crate::{ParsedDocument, SyntaxElement, SyntaxKind};

/// The result of formatting a document.
pub struct FormattedSource {
    text: String,
}

impl FormattedSource {
    /// The formatted text, ending in exactly one newline.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for FormattedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// How a bar's events are spaced across its line.
///
/// The default is computed rather than asked for. Every file written so far is
/// compact, so defaulting to [`BarSpacing::Proportional`] would rewrite a
/// corpus on upgrade; and compact is the layout that is never actively wrong,
/// because it never claims an alignment. So there is no unset state and no
/// `Option<BarSpacing>` anywhere: a caller that has not been told chooses
/// `Compact`, and that is an answer rather than a gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSpacing {
    /// One space between events, and two between beat groups.
    #[default]
    Compact,
    /// Every event at a column proportional to when it sounds, so beat *n*
    /// falls in the same column on every line of a voice and the page can be
    /// read down as well as across.
    Proportional,
}

/// Format a parsed document. Lossless: every comment and token survives;
/// only whitespace trivia is normalized.
///
/// `bars` is a required parameter and not an overload on purpose. Leaving a
/// one-argument `format` in the API would leave a trap that silently means
/// "compact", so the next call site added — a preview, a build step, a doc
/// test — would quietly ignore the project's setting. Making it mandatory is
/// the type system enforcing what the crate graph cannot: this crate cannot
/// read a manifest, so every caller answers which layout this is.
pub fn format(document: &ParsedDocument, bars: BarSpacing) -> FormattedSource {
    let root = document.syntax();
    let layout = Layout {
        meters: meters_in_force(&root),
        bars,
    };
    let mut writer = Writer::new();
    format_node(&root, &mut writer, &layout);
    FormattedSource { text: writer.finish() }
}

/// What the layout of a bar depends on beyond the tree in front of it.
struct Layout {
    meters: Meters,
    bars: BarSpacing,
}

/// How wide a line a bar — or a comma-separated list — may keep, indent
/// included.
///
/// Everything else in musa is a short statement on its own line. A bar is the
/// one statement that is naturally horizontal, because that is the direction
/// music is read in, and a bar you can select with a double-click and drag
/// into the next voice is the difference between the brace being worth typing
/// and not. So a bar keeps its line, and breaks like any other block only when
/// it is genuinely long.
///
/// 96, not the source column's 48-character measure
/// (`01-visual-language.md` §8). A bar sits four levels in, so sixteen
/// characters are spent before the word `bar`, and a measure of four quarter
/// notes is forty more; budgeting a bar to the column would break every bar
/// there is, which is the same as not having the rule. What 96 buys is that a
/// bar of eight eighths — an ordinary bar — stays one line, and a bar long
/// enough to need scrolling to read is a bar long enough to stack. The cost is
/// that a barred piece has lines past the column, and the column scrolls them,
/// which is what it already does for comment prose and long signal chains.
///
/// A list is budgeted by the same number for the same reason: a list is
/// horizontal until it is long, and one number is one rule to remember.
const MEASURE: usize = 96;

/// Whether this node is a comma-separated list that may be written down the
/// page instead of across it.
///
/// Every list whose items can be arbitrarily large is here, and that is the
/// membership rule rather than a taste: a lambda with a `match` in it is one
/// argument, a function type is one parameter, a constructor call is one
/// element, so none of these lists has a width its spelling bounds. A list
/// left out of this set is a list the formatter would join to whatever length
/// it came to, which is the 200-character line this rule exists to prevent.
///
/// The set must also be *closed downwards* through nesting, and that is the
/// second reason it is wide. A list decides before the lists inside it do, so
/// an outer list that cannot break leaves an inner one to absorb the overflow
/// alone — `[NoteValue(1, 0), …, NoteValue(\n    64,\n    2,\n)]`, which
/// breaks the one list that had nothing to gain by breaking. Whenever a list
/// can hold another, both belong here or neither does.
///
/// Types are the deliberate omission. `Result<Position<WrittenTime>, Text>` is
/// one name for one type and reads as a word however long it runs; a `<` that
/// opened a stack of lines would be the formatter claiming a type has parts a
/// reader looks at separately.
fn breakable_list(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::ExprArgList
            | SyntaxKind::ArgList
            | SyntaxKind::ParamList
            | SyntaxKind::DataVariant
            | SyntaxKind::ListExpr
            | SyntaxKind::ProductExpr
    )
}

/// Whether a list must be written one item per line.
///
/// Two reasons, and both are the same reason: the one-line layout is not
/// available. It does not fit the line it would start at, or it carries a
/// comment — and a comment wants a line of its own, so a list holding one has
/// already been written down the page by the hand that wrote the comment.
///
/// Measured by rendering the list flat with the writer everything else uses,
/// so a list is judged by exactly the text it would produce rather than by a
/// second estimate of it — and measured together with [`line_tail`], because
/// what the budget is about is the line and a list is only part of one.
fn list_breaks(node: &SyntaxNode, writer: &Writer, layout: &Layout) -> bool {
    if node
        .descendants_with_tokens()
        .any(|element| matches!(element.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
    {
        return true;
    }
    let fits = writer
        .column()
        .saturating_add(one_line(node, layout).chars().count())
        .saturating_add(line_tail(node, &writer.lists, layout))
        <= MEASURE;
    !fits && !hugs_its_last(node, writer, layout)
}

/// Whether an over-wide list can stay on its line because the item that
/// overflows it is a list of its own, which will break where it stands.
///
/// `syntax_group(syntax_built(here, 9, 0), "parentheses", [ … ])` is the case:
/// a call whose last argument is a tree written out. Stacking it puts three
/// lines around a bracket that was already going to open one, and because the
/// argument is itself a call with a list in it, every level does that again —
/// the nesting a reader follows becomes a staircase four indents deep before
/// it says anything. Left hugging, the same text reads as what it is, a `[`
/// that opens a body on the line of the call it belongs to.
///
/// The hug is offered only to a bracketed literal in the last position, and
/// only when the call's own line up to its `[` fits. Both halves matter. A
/// prefix that does not fit is a line the hug cannot rescue, which is what
/// keeps a twenty-element list from hugging its last element's arguments; and
/// a bracket is the one closer that says *a collection ends here* on sight, so
/// it can hold a line open the way a brace does. A trailing call cannot:
/// `option_fold(chord c major, same, inversion(chord c major, 1))` hugged at
/// `inversion(` puts two of five arguments down the page and reads as though
/// the inversion were the point, which is why that one stacks instead.
fn hugs_its_last(list: &SyntaxNode, writer: &Writer, layout: &Layout) -> bool {
    let Some(hugged) = trailing_list(list) else {
        return false;
    };
    let Some(opener) = opening_token(&hugged) else {
        return false;
    };
    let mut prefix = Writer::one_line(HashSet::new());
    write_through(list, &opener, &mut prefix, layout);
    writer.column().saturating_add(prefix.written()) <= MEASURE
}

/// The bracketed literal the last item of `list` would break at, following the
/// last child down — an argument is a call is a list, and that spine is the
/// only place a break at the end of the line can come from.
///
/// `None` at a body, whose braces break by their own rule, and `None` at any
/// other list, because the first breakable thing down the spine is where the
/// break would land and only a `[` earns the hug.
fn trailing_list(list: &SyntaxNode) -> Option<SyntaxNode> {
    let mut node = list.children().last()?;
    loop {
        if opens_a_body(node.kind()) {
            return None;
        }
        if breakable_list(node.kind()) {
            return (node.kind() == SyntaxKind::ListExpr).then_some(node);
        }
        node = node.children().last()?;
    }
}

/// Whether this closing delimiter is written on a line of its own rather than
/// on the one the tail is measuring.
fn closes_a_line(kind: SyntaxKind, writer: &Writer) -> bool {
    kind == SyntaxKind::RBrace || (matches!(kind, SyntaxKind::RParen | SyntaxKind::RBracket) && writer.list_breaks())
}

/// The delimiter of the first place inside `node` where the line could be cut
/// — the `(`, `[` or `{` of the first list or body it holds, itself included.
fn next_break(node: &SyntaxNode) -> Option<SyntaxToken> {
    let opportunity = node
        .descendants()
        .find(|inner| breakable_list(inner.kind()) || opens_a_body(inner.kind()))?;
    opening_token(&opportunity)
}

/// The first token of a node that is written rather than skipped. A node owns
/// the trivia in front of it, so its literal first token can be the whitespace
/// the previous line ended with.
fn opening_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
            )
        })
}

/// Write `node` flat, stopping after `stop`. Whether it was reached.
fn write_through(node: &SyntaxNode, stop: &SyntaxToken, writer: &mut Writer, layout: &Layout) -> bool {
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) if child.text_range().contains_range(stop.text_range()) => {
                return write_through(&child, stop, writer, layout);
            }
            SyntaxElement::Node(child) => format_node(&child, writer, layout),
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::Whitespace => {
                writer.note_whitespace(token.text());
            }
            SyntaxElement::Token(token) => {
                let reached = token.text_range() == stop.text_range();
                format_token(node, &token, writer);
                if reached {
                    return true;
                }
            }
        }
    }
    false
}

/// How much of the list's line is spoken for after the list closes.
///
/// A parameter list is the case that makes this necessary and not a
/// refinement: `fn rescaled(factor: Ratio, here: Position<WrittenTime>, point:
/// Position<WrittenTime>)` is 90 columns of list and then 41 more of
/// `-> Result<Position<WrittenTime>, Text> {`, and a rule that weighed only
/// the first would call a 131-column line comfortable.
///
/// Read forward from the list through its siblings and then its parents', in
/// the order the writer will reach them, and stop at the first thing that ends
/// a line or could: a `;`, a stacking comma, or the opening delimiter of the
/// next list or body, which is the next place the line can be cut and so the
/// last column this list is answerable for. Reading past that would charge one
/// list for a length another one is going to break anyway, which is how
/// `syntax_built(here, 9, 0)` came to stack because a bracketed tree three
/// arguments later was long. Stop also once the tail alone has spent the
/// budget, because past that the answer cannot change and the work is a whole
/// subtree's worth of rendering.
///
/// `enclosing` is the stacking decision of every list this one sits inside,
/// outermost first, and it is what makes the walk agree with the writer rather
/// than guess at it: an outer list that has already chosen to stack ends the
/// tail at its next comma, so `Ok(Dotted(base, base))` is measured as the
/// short line it will be written on and not as the whole `nat_fold` call it
/// happens to be an argument of.
fn line_tail(list: &SyntaxNode, enclosing: &[bool], layout: &Layout) -> usize {
    let mut writer = Writer::one_line(HashSet::new());
    let mut outer = enclosing.len();
    let mut width = 0_usize;
    let mut node = list.clone();
    while let Some(parent) = node.parent() {
        // The writer reaches this parent's own tokens under this parent's
        // stacking decision, which was taken before the list being measured
        // was reached and so is already in `enclosing`.
        writer.lists.clear();
        if breakable_list(parent.kind()) {
            outer = outer.saturating_sub(1);
            writer.lists.push(enclosing.get(outer).copied().unwrap_or_default());
        }
        let mut following = node.next_sibling_or_token();
        while let Some(element) = following {
            following = element.next_sibling_or_token();
            match element {
                SyntaxElement::Node(child) => match next_break(&child) {
                    // The line can be cut here, so this is where the tail ends
                    // and the delimiter that cuts it is the last of it.
                    Some(opener) => {
                        write_through(&child, &opener, &mut writer, layout);
                        return writer.written();
                    }
                    None => format_node(&child, &mut writer, layout),
                },
                SyntaxElement::Token(token) if token.kind() == SyntaxKind::Whitespace => {
                    writer.note_whitespace(token.text());
                }
                // A chain long enough to stack puts every stage on its own
                // line, so the `|>` is where this line stops.
                SyntaxElement::Token(token)
                    if token.kind() == SyntaxKind::PipeForward && wraps_across_lines(&parent) =>
                {
                    return width;
                }
                // A closer that takes its own line is not on this one. The
                // `}` of a body always does, and so does the `)` of a list
                // that has already chosen to stack.
                SyntaxElement::Token(token) if closes_a_line(token.kind(), &writer) => {
                    return width;
                }
                SyntaxElement::Token(token) => format_token(&parent, &token, &mut writer),
            }
            width = writer.written();
            if writer.line_ended() || width > MEASURE {
                return width;
            }
        }
        node = parent;
    }
    width
}

/// Whether this node is written as a braced body, so its `{` ends the line the
/// thing it belongs to started.
fn opens_a_body(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::BlockExpr | SyntaxKind::Block | SyntaxKind::MatchExpr | SyntaxKind::MusicExpr
    )
}

/// Whether nothing follows this comma inside its list, so there is nothing for
/// a space after it to separate.
///
/// A trailing comma is how a list written down the page keeps its last item
/// editable, and it survives when the list is joined back up because the
/// formatter rewrites whitespace and never tokens. `f(a, b, )` is that comma
/// with a space it has no use for; `f(a, b,)` is the same list spelled the way
/// a reader would.
fn ends_its_list(comma: &SyntaxToken) -> bool {
    let mut following = comma.next_sibling_or_token();
    while let Some(element) = following {
        if element.kind() != SyntaxKind::Whitespace {
            return matches!(
                element.kind(),
                SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace | SyntaxKind::Greater
            );
        }
        following = element.next_sibling_or_token();
    }
    true
}

fn format_node(node: &SyntaxNode, writer: &mut Writer, layout: &Layout) {
    // Set once the first token of a one-word construct has been written, so
    // the rest of it joins on without a space.
    let mut tight = false;
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                writer.blank_line_if_pending();
                // A quote is written as it stands. Its interior is the
                // kernel's grammar, whose layout the kernel's own printer
                // owns (`docs/rules/language/01-surface.md` §7), and a formatter
                // that re-broke those lines by the host's rules would be a
                // second opinion about a shape this crate has no reading of.
                // Only the anchoring is this crate's: the block moves to the
                // indent the host puts it at, keeping its lines' relative
                // depth.
                if child.kind() == SyntaxKind::KernelQuote {
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
                if writer.starts_a_beat_group(&child) {
                    writer.widen_next_gap();
                }
                // A bar or a grace note owns its line, so its width is
                // measured from the indent. A block is a body written after
                // what it belongs to — `fn f(x: nat) -> nat` — so its width
                // is measured from where the line has already reached.
                let start = if child.kind() == SyntaxKind::BlockExpr {
                    writer.column()
                } else {
                    writer.indent
                };
                if let Some(lines) = inline_run(&child, start, layout) {
                    // A lambda's body is the last thing in an expression, not
                    // the last thing on a line: `map(fn (x) { f(x) }, xs)`
                    // continues with a comma. Every other block ends what it
                    // was written after.
                    let inline = node.kind() == SyntaxKind::LambdaExpr;
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
                if matches!(
                    node.kind(),
                    SyntaxKind::Position
                        | SyntaxKind::ChordSymbol
                        | SyntaxKind::ParamPath
                        | SyntaxKind::PitchClass
                        | SyntaxKind::NameExpr
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

/// Write a quotation verbatim, re-anchored at the writer's indent.
///
/// The first line joins the line in progress — `let doubled: Music = kernel
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

fn format_token(node: &SyntaxNode, token: &SyntaxToken, writer: &mut Writer) {
    let kind = token.kind();
    let text = token.text();
    let parent = node.kind();
    // Whether this token belongs to a list that is being written down the
    // page. Only the list's own punctuation asks, and a token whose parent is
    // the list is directly inside it, so the writer's innermost answer is that
    // list's.
    let stacked = breakable_list(parent) && writer.list_breaks();
    // A lambda's braces close an expression that has more after it, so its
    // `}` does not end the line the way a declaration's body does.
    let held = parent == SyntaxKind::BlockExpr
        && node
            .parent()
            .is_some_and(|owner| owner.kind() == SyntaxKind::LambdaExpr);
    if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
        writer.comment(text);
        return;
    }
    writer.prep_line();
    if kind == SyntaxKind::LBrace {
        writer.space();
        writer.write("{");
        writer.indent_more();
        writer.end_line();
    } else if kind == SyntaxKind::RBrace {
        writer.indent_less();
        // A block's one expression ends with no `;` to end its line, so the
        // brace that closes it asks for the line itself. A declaration's last
        // variant may be written without its trailing comma, which leaves the
        // same brace stranded after it.
        if parent == SyntaxKind::BlockExpr || parent == SyntaxKind::DataDecl {
            writer.break_before_close();
        }
        // On its own line the `}` needs no space in front of it, and
        // `needs_word_space` already knows that: `prep_line` has just put the
        // writer at the start of a line. On a one-line run there is no line
        // start to rely on, and `grace { c5 d5 }` wants its space.
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write("}");
        if !held && !matches!(parent, SyntaxKind::MusicExpr | SyntaxKind::MatchExpr) {
            writer.end_line();
        }
    } else if kind == SyntaxKind::Semicolon {
        writer.write(";");
        writer.end_line();
    } else if kind == SyntaxKind::Comma {
        writer.write(",");
        // A match arm and a constructor are both a case of the same thing, and
        // both read as a list read downwards. A comma *inside* a constructor
        // separates its fields, which are one word's worth of a line, and that
        // comma belongs to the `DataVariant`, not to the declaration.
        if parent == SyntaxKind::MatchExpr || parent == SyntaxKind::DataDecl || stacked {
            writer.end_line();
        } else if !ends_its_list(token) {
            writer.space();
        }
    } else if kind == SyntaxKind::Equals {
        writer.space();
        writer.write("=");
        writer.space();
    } else if kind == SyntaxKind::Colon {
        writer.write(":");
        if parent != SyntaxKind::ImportStmt {
            writer.space();
        }
    } else if kind == SyntaxKind::PipeForward && writer.in_wrapped_chain() {
        // A long chain reads as a stack of stages, which is how a patch is
        // actually thought about.
        writer.indent_continuations();
        writer.newline();
        writer.write("|>");
        writer.space();
    } else if kind == SyntaxKind::Arrow || kind == SyntaxKind::PipeForward {
        writer.space();
        writer.write(text);
        writer.space();
    } else if kind == SyntaxKind::LBracket {
        // `chord [` takes a space; `use sigh(` does not. A type parameter
        // never reaches here — it is written `Option<Pitch>`, and `[` means a
        // list.
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
        if stacked {
            writer.indent_more();
            writer.end_line();
        }
    } else if kind == SyntaxKind::LParen || kind == SyntaxKind::RBracket || kind == SyntaxKind::RParen {
        // A list written down the page opens and closes the way a block does:
        // the opener takes the rest of its line, the items are the lines, and
        // the closer comes back out to the indent the list started at.
        if stacked && kind != SyntaxKind::LParen {
            writer.indent_less();
            writer.break_before_close();
        }
        // `use sigh(` closes up; `fn (line: Music)` does not, because there is
        // no name between the word and the list and `fn(` reads as a call.
        if kind == SyntaxKind::LParen && writer.prev == Some(SyntaxKind::FnKw) {
            writer.space();
        }
        writer.write(text);
        if kind == SyntaxKind::LParen && stacked {
            writer.indent_more();
            writer.end_line();
        }
    } else if matches!(
        kind,
        SyntaxKind::Slash | SyntaxKind::Dot | SyntaxKind::Greater | SyntaxKind::Caret | SyntaxKind::Less
    ) {
        // A short-form duration is part of the note's word: `c4/4.` is one
        // note written one way, not a pitch beside a fraction beside a dot.
        // An accent or a marcato is drawn on its notehead, so it is written
        // on its note: `c4/4>`, never `c4/4 >`. A type parameter binds to its
        // type the same way: `Option<Pitch>`, never `Option <Pitch>`.
        writer.write(text);
    } else {
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
    }
    writer.after_significant(kind);
}

/// Layout state for the formatter.
struct Writer {
    out: String,
    indent: usize,
    at_line_start: bool,
    /// A line break is owed before the next token (after `;`, `{`, `}`,
    /// or a comment). Deferred so a trailing comment can join the line first.
    need_newline: bool,
    /// Newlines seen in the whitespace run just passed.
    pending_newlines: usize,
    /// Kind of the last significant token written.
    prev: Option<SyntaxKind>,
    /// One entry per enclosing stacked chain: whether its continuation
    /// indent has been applied yet.
    chains: Vec<bool>,
    /// One entry per enclosing comma-separated list: whether it is written
    /// one item per line. Innermost last.
    lists: Vec<bool>,
    /// Set while rendering a horizontal run, which is the only time newlines
    /// are swallowed and the only time a gap is ever wider than one space.
    run: Option<Run>,
}

/// What a horizontal run needs to know about itself while it is written.
struct Run {
    /// The items that open a beat group, by range. The gap in front of one is
    /// two spaces wide instead of one.
    beat_groups: HashSet<TextRange>,
    /// The next space written is a beat-group gap.
    wide_gap: bool,
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
        SyntaxKind::BarStmt | SyntaxKind::GraceStmt | SyntaxKind::BlockExpr
    ) {
        return None;
    }
    if node
        .descendants_with_tokens()
        .any(|element| matches!(element.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
    {
        return None;
    }
    // A block holds one expression, so there is nothing in it to space by
    // beat group and nothing to wrap at: it is one line when it fits, and
    // otherwise it breaks at its own braces like every other block.
    if node.kind() == SyntaxKind::BlockExpr {
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

/// Columns to the whole note.
///
/// Sixteen to the quarter. 64 divides by 2, 4, 8, 16 and 32 exactly and by 3
/// closely enough that a triplet lands back on the grid where it ends.
const COLUMNS: u64 = 64;

/// The column an event at `elapsed` whole notes into the bar wants.
///
/// Linear, and that is a proof rather than a taste. If the width given to an
/// event were some function `W` of its own duration and columns accumulated,
/// then "equal elapsed time, equal column" applied to `1/4 + 1/4` against `1/2`
/// forces `W(1/2) = 2·W(1/4)` — and applied at every dyadic split, forces `W`
/// linear. Any concave curve, which is what an engraver actually uses, provably
/// puts beat 3 in a different column on a line of quarters than on a line of
/// halves, and that is the one thing this layout exists to prevent. Real
/// engraving escapes the argument by solving one spacing problem across a whole
/// system; here that would make an edit in bar 5 relay out bars 1 to 8, and
/// diff locality is what a text formatter must not spend.
///
/// Integers throughout, deliberately: `log2` is not bit-identical across libm
/// implementations, and a formatter whose output depended on the platform's
/// would make `musa format --check` fail in CI on a machine other than the one
/// that wrote the file.
fn grid(elapsed: Beat) -> Option<usize> {
    let columns = elapsed
        .numerator
        .checked_mul(COLUMNS)?
        .checked_div(elapsed.denominator)?;
    usize::try_from(columns).ok()
}

/// A `|` bar with every event at the column its onset asks for, or `None` when
/// it cannot be drawn that way.
///
/// Two conditions. The bar must be measurable, which is the same question the
/// beat groups ask and the same answer. And it must be a `|` bar: a named bar's
/// head is its address and is as long as its name, so its events start
/// somewhere no other bar's do, and a column that means one thing per line is
/// no column at all.
///
/// The recurrence is `col(k) = max(grid(t_k), col(k-1) + len(k-1) + gap(k))` —
/// the onset's column, or the compact layout's own gap past the previous event
/// when the music is denser than the grid. Because the second term is always
/// taken into account, a drawn bar is never *narrower* than the compact one,
/// which is what makes the setting safe: turning it on can never push a bar
/// over the line that was not already over it.
///
/// `gap(k)` is the compact writer's gap and not a constant one, which is the
/// whole of that guarantee: a beat-group boundary is written two spaces wide
/// there, so a grid that reserved one would draw a *narrower* line than the
/// compact layout at every group start dense enough to be pushed off its
/// column.
fn drawn_to_scale(bar: &SyntaxNode, layout: &Layout) -> Option<String> {
    if bar.children().any(|child| child.kind() == SyntaxKind::Block) {
        return None;
    }
    let measured = measured_bar(bar, &layout.meters)?;
    let groups = beat_group_starts(bar, &layout.meters);
    let mut body = String::new();
    let mut elapsed = Beat::ZERO;
    let mut next = 0_usize;
    for (item, length) in measured.items {
        let gap = usize::from(groups.contains(&item.text_range()));
        let column = grid(elapsed)?.max(next.saturating_add(gap));
        while body.chars().count() < column {
            body.push(' ');
        }
        let text = one_line(&item, layout);
        next = column.saturating_add(text.chars().count()).saturating_add(1);
        body.push_str(&text);
        elapsed = elapsed.plus(length)?;
    }
    Some(format!("| {body}"))
}

/// One construct, rendered on a single line by the rules everything else uses.
fn one_line(node: &SyntaxNode, layout: &Layout) -> String {
    let mut writer = Writer::one_line(HashSet::new());
    format_node(node, &mut writer, layout);
    writer.finish()
}

/// How far a continuation line is pushed past its bar's own indent.
///
/// Two, which is the width of `| `: a bar that wraps reads as one bar because
/// every line after the first starts under the first event rather than under
/// the barline.
const CONTINUATION: usize = 2;

/// Break an over-wide bar at the gaps between its beat groups.
///
/// The gaps are the only places a bar may be broken — a group is what a player
/// reads at once, and splitting one puts half a beat on the next line. So a
/// bar with no groups, or one group too wide to fit, runs long instead.
///
/// Groups that stay on one line keep the wide gap between them: the wrap
/// decides where the lines end, not what a gap means.
fn wrap_at_beat_groups(line: &str, indent: usize) -> Vec<String> {
    const GAP: &str = "  ";
    let mut lines: Vec<String> = Vec::new();
    for group in line.split(GAP) {
        let room = MEASURE
            .saturating_sub(indent)
            .saturating_sub(if lines.is_empty() { 0 } else { CONTINUATION });
        let joined = |last: &String| {
            last.chars()
                .count()
                .saturating_add(GAP.len())
                .saturating_add(group.chars().count())
        };
        match lines.last_mut() {
            Some(last) if joined(last) <= room => {
                last.push_str(GAP);
                last.push_str(group);
            }
            _ => {
                let mut next = String::new();
                if !lines.is_empty() {
                    next.push_str(&" ".repeat(CONTINUATION));
                }
                next.push_str(group);
                lines.push(next);
            }
        }
    }
    lines
}

// --- Beat groups -----------------------------------------------------------

/// The meter in force at each bar, by where the bar stands in the source.
///
/// Keyed by range because that is the one thing a bar has that is unique to it
/// and stable across the walk. The map holds only bars under a stated meter
/// that is a fraction: `meter none;` and a piece that never says one leave the
/// bar out, and a bar that is not in here is not grouped.
type Meters = HashMap<TextRange, (u32, u32)>;

/// Which meter governs each bar — a pre-walk, because one pass cannot know.
///
/// A part may state its meter *after* the voices it governs (`part_decl`
/// accepts its declarations in any order), so a left-to-right reader would
/// space a part's first bars by the piece's meter and its last ones by the
/// part's. Inside a voice the order does mean what it looks like: a `meter`
/// written among the events is a change written where it happens, and it
/// governs from there.
fn meters_in_force(root: &SyntaxNode) -> Meters {
    let mut meters = Meters::new();
    scan_meters(root, None, &mut meters);
    meters
}

fn scan_meters(node: &SyntaxNode, inherited: Option<(u32, u32)>, meters: &mut Meters) {
    // A part's own meter governs all of it, including the voices written
    // above the line that states it.
    let mut current = if node.kind() == SyntaxKind::PartDecl {
        node.children()
            .find(|child| child.kind() == SyntaxKind::MeterStmt)
            .map_or(inherited, |stated| meter_of(&stated))
    } else {
        inherited
    };
    for child in node.children() {
        if child.kind() == SyntaxKind::MeterStmt {
            current = meter_of(&child);
        }
        if child.kind() == SyntaxKind::BarStmt
            && let Some(meter) = current
        {
            meters.insert(child.text_range(), meter);
        }
        scan_meters(&child, current, meters);
    }
}

/// The fraction a `meter` statement states, or `None` for `meter none;`.
fn meter_of(statement: &SyntaxNode) -> Option<(u32, u32)> {
    let written = statement
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Rational)?;
    let (numerator, denominator) = written.text().split_once('/')?;
    Some((numerator.parse().ok()?, denominator.parse().ok()?))
}

/// The items of a bar that begin a new beat group.
///
/// This is the whole of the layout decision, and it is a whitelist so that a
/// statement kind added next year falls back rather than guesses: a bar is
/// *measurable* when every item is a note, rest or chord contributing its
/// written duration, or a dynamic or a mark contributing nothing; when every
/// duration is a literal; when a meter is in force; and when the durations sum
/// to exactly one measure. Anything else — a `use`, a tuplet, an `improvise`,
/// a duration that is a parameter, a bar that does not add up — is spaced with
/// single spaces throughout, because the grouping would be a claim about music
/// the formatter cannot read.
///
/// One further condition, which is what keeps an ordinary bar ordinary: some
/// group must hold more than one event. A wide gap says *these belong
/// together*, and in a bar of four quarters in 4/4 every group holds one note,
/// so the gaps would separate nothing. A beam that beams one note is not a
/// beam.
fn beat_group_starts(bar: &SyntaxNode, meters: &Meters) -> HashSet<TextRange> {
    let none = HashSet::new();
    let Some(measured) = measured_bar(bar, meters) else {
        return none;
    };
    let (numerator, denominator) = measured.meter;
    let items = measured.items;
    let mut boundaries = Vec::new();
    let mut counted = 0_u32;
    for group in beat_groups(numerator, denominator) {
        counted = counted.saturating_add(group);
        boundaries.push(Beat::new(u64::from(counted), u64::from(denominator)));
    }
    // The downbeat and the barline are boundaries too, and neither is a gap.
    boundaries.pop();

    let mut starts = HashSet::new();
    let mut elapsed = Beat::ZERO;
    let mut next = 0_usize;
    let mut widest = 0_usize;
    let mut in_group = 0_usize;
    for &(ref item, length) in &items {
        let range = item.text_range();
        while boundaries.get(next).is_some_and(|boundary| *boundary < elapsed) {
            next = next.saturating_add(1);
        }
        if boundaries.get(next) == Some(&elapsed) {
            next = next.saturating_add(1);
            starts.insert(range);
            widest = widest.max(in_group);
            in_group = 0;
        }
        in_group = in_group.saturating_add(1);
        elapsed = match elapsed.plus(length) {
            Some(sum) => sum,
            None => return HashSet::new(),
        };
    }
    if widest.max(in_group) > 1 {
        starts
    } else {
        HashSet::new()
    }
}

/// A bar the formatter can read: its meter, and its items with their lengths.
struct MeasuredBar {
    meter: (u32, u32),
    items: Vec<(SyntaxNode, Beat)>,
}

/// The one reader of "can this bar be laid out to the beat".
///
/// Both layouts ask it — the beat groups of every file, and the grid of a
/// project that has asked for one — so a bar that groups is exactly a bar that
/// can be drawn to scale, and neither can drift into measuring something the
/// other does not.
fn measured_bar(bar: &SyntaxNode, meters: &Meters) -> Option<MeasuredBar> {
    if bar.kind() != SyntaxKind::BarStmt {
        return None;
    }
    let &(numerator, denominator) = meters.get(&bar.text_range())?;
    let items = measurable_items(bar)?;
    let total = items.iter().try_fold(Beat::ZERO, |sum, item| sum.plus(item.1))?;
    (total == Beat::new(u64::from(numerator), u64::from(denominator))).then_some(MeasuredBar {
        meter: (numerator, denominator),
        items,
    })
}

/// The bar's items with their written lengths, or `None` if it holds anything
/// the formatter cannot measure.
fn measurable_items(bar: &SyntaxNode) -> Option<Vec<(SyntaxNode, Beat)>> {
    // `bar name { … }` holds a block and `| …` holds its items directly —
    // the same two shapes `ast::voice_items` reads, and the same answer for
    // both, because a name does not change how a bar is counted.
    let body = bar.children().find(|child| child.kind() == SyntaxKind::Block);
    let items = body.as_ref().unwrap_or(bar).children();
    items
        .filter(|item| item.kind() != SyntaxKind::Block)
        .map(|item| {
            let kind = item.kind();
            // A note, a rest, a chord and a stacked chord fill the time they
            // write; a dynamic
            // and a mark fill none. Everything else — a `use`, a tuplet, an
            // `improvise`, a construct added next year — has no length this
            // crate can read, and the whitelist is what makes that the
            // default rather than a case somebody has to remember.
            let length = if matches!(
                kind,
                SyntaxKind::NoteStmt | SyntaxKind::RestStmt | SyntaxKind::ChordStmt | SyntaxKind::StackStmt
            ) {
                Beat::parse(&crate::ast::Duration::of(&item)?.value()?)?
            } else if matches!(kind, SyntaxKind::DynamicStmt | SyntaxKind::MarkStmt) {
                Beat::ZERO
            } else {
                return None;
            };
            Some((item, length))
        })
        .collect()
}

/// A length in whole notes, exactly.
///
/// Reduced on construction so that equality is equality of the fraction, which
/// is what a boundary test needs: 2/8 and 1/4 are the same instant.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Beat {
    numerator: u64,
    denominator: u64,
}

impl Beat {
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    fn new(numerator: u64, denominator: u64) -> Self {
        if denominator == 0 {
            return Self::ZERO;
        }
        let divisor = gcd(numerator, denominator).max(1);
        Self {
            numerator: numerator.checked_div(divisor).unwrap_or(0),
            denominator: denominator.checked_div(divisor).unwrap_or(1),
        }
    }

    /// A duration as `Duration::value` spells it: `3/8`, or `1` for a whole.
    fn parse(spelling: &str) -> Option<Self> {
        let (numerator, denominator) = spelling.split_once('/').unwrap_or((spelling, "1"));
        Some(Self::new(numerator.parse().ok()?, denominator.parse().ok()?))
    }

    /// `None` on overflow, which reads as "not measurable" rather than as a
    /// wrong answer.
    fn plus(self, other: Self) -> Option<Self> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Some(Self::new(
            left.checked_add(right)?,
            self.denominator.checked_mul(other.denominator)?,
        ))
    }
}

impl PartialOrd for Beat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Some(left.cmp(&right))
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let rest = a.checked_rem(b).unwrap_or(0);
        a = b;
        b = rest;
    }
    a
}

/// Whether a signal chain is long enough to be worth stacking.
///
/// Two stages (`oscillator(sine) |> gain(-15 dB)`) read fine on one line;
/// three or more is where a patch stops being a phrase and starts being a
/// signal path, and the roadmap's §7.1 example writes those stacked.
fn wraps_across_lines(node: &SyntaxNode) -> bool {
    node.kind() == SyntaxKind::SignalChain
        && node
            .children_with_tokens()
            .filter(|element| element.kind() == SyntaxKind::PipeForward)
            .count()
            >= 2
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
            at_line_start: true,
            need_newline: false,
            pending_newlines: 0,
            prev: None,
            chains: Vec::new(),
            lists: Vec::new(),
            run: None,
        }
    }

    /// A writer that lays its input out on one line, widening the gap in front
    /// of every item in `beat_groups`.
    fn one_line(beat_groups: HashSet<TextRange>) -> Self {
        Self {
            at_line_start: false,
            run: Some(Run {
                beat_groups,
                wide_gap: false,
            }),
            ..Self::new()
        }
    }

    /// Whether this item opens a beat group, and so takes the wider gap.
    fn starts_a_beat_group(&self, node: &SyntaxNode) -> bool {
        self.run
            .as_ref()
            .is_some_and(|run| run.beat_groups.contains(&node.text_range()))
    }

    fn widen_next_gap(&mut self) {
        if let Some(run) = self.run.as_mut() {
            run.wide_gap = true;
        }
    }

    /// Enter a chain that is written one stage per line.
    fn open_chain(&mut self) {
        self.chains.push(false);
    }

    fn close_chain(&mut self) {
        if self.chains.pop() == Some(true) {
            self.indent_less();
        }
    }

    fn in_wrapped_chain(&self) -> bool {
        !self.chains.is_empty()
    }

    /// Whether this writer is laying its input out on one line.
    fn in_run(&self) -> bool {
        self.run.is_some()
    }

    /// How much has been written, in columns.
    fn written(&self) -> usize {
        self.out.chars().count()
    }

    /// Whether the last thing written asked for the line to end.
    fn line_ended(&self) -> bool {
        self.need_newline
    }

    /// Enter a comma-separated list, recording whether it stacks. Entered for
    /// every list and not only the stacked ones: a short list inside a long
    /// one stays on its line, and the stack is what says so.
    fn open_list(&mut self, broken: bool) {
        self.lists.push(broken);
    }

    fn close_list(&mut self) {
        self.lists.pop();
    }

    /// Whether the innermost enclosing list is written one item per line.
    fn list_breaks(&self) -> bool {
        self.lists.last().copied().unwrap_or_default()
    }

    /// Indent the continuation lines — once per chain, and only when the
    /// first `|>` proves there will be any. The chain's head stays on the
    /// line its statement started.
    fn indent_continuations(&mut self) {
        if let Some(indented) = self.chains.last_mut()
            && !*indented
        {
            *indented = true;
            self.indent_more();
        }
    }

    fn finish(mut self) -> String {
        if self.run.is_some() {
            return self.out;
        }
        while self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out.push('\n');
        self.out
    }

    /// Emit a deferred line break, if one is owed.
    fn prep_line(&mut self) {
        if self.need_newline {
            self.newline();
            self.need_newline = false;
        }
    }

    fn end_line(&mut self) {
        self.need_newline = true;
    }

    /// Put the closing brace of a stacked block on its own line.
    ///
    /// Nothing on a one-line run, where the brace joins its neighbour with a
    /// space, and nothing at the start of a line, which is already what this
    /// is for.
    fn break_before_close(&mut self) {
        if self.run.is_some() || self.at_line_start {
            return;
        }
        self.newline();
    }

    /// Where the next thing written would land on the current line.
    ///
    /// The indent when the line has not been started — or has been ended and
    /// not yet broken — and the width written so far when it has.
    fn column(&self) -> usize {
        if self.at_line_start || self.need_newline {
            return self.indent;
        }
        self.out
            .rsplit('\n')
            .next()
            .map_or(self.indent, |line| line.chars().count())
    }

    fn write(&mut self, text: &str) {
        if self.at_line_start {
            for _ in 0..self.indent {
                self.out.push(' ');
            }
            self.at_line_start = false;
        }
        self.out.push_str(text);
    }

    /// Write one token of a construct that is spelled as a single word:
    /// spaced from what came before unless it is joining a word already
    /// started.
    fn write_word(&mut self, kind: SyntaxKind, text: &str, joining: bool) {
        self.prep_line();
        if !joining && self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.after_significant(kind);
    }

    /// Write a whole construct that was rendered elsewhere, as its own line.
    /// A run written where the line goes on afterwards.
    fn write_run(&mut self, text: &str) {
        self.prep_line();
        if self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.after_significant(SyntaxKind::RBrace);
    }

    fn write_line(&mut self, text: &str) {
        self.prep_line();
        if self.needs_word_space() {
            self.space();
        }
        self.write(text);
        self.end_line();
        self.after_significant(SyntaxKind::RBrace);
    }

    /// One space, or two in front of a beat group — and never one more than
    /// that. On a one-line run the gap between two items can be asked for
    /// twice, by the statement that ended and by the token that follows, and
    /// what is wanted is the gap rather than the count of requests.
    fn space(&mut self) {
        if self.at_line_start {
            return;
        }
        let wide = self.run.as_mut().is_some_and(|run| std::mem::take(&mut run.wide_gap));
        let written = usize::from(self.out.ends_with(' ')).saturating_add(usize::from(self.out.ends_with("  ")));
        for _ in written..if wide { 2 } else { 1 } {
            self.out.push(' ');
        }
    }

    fn newline(&mut self) {
        if self.run.is_some() {
            // One line has no newlines in it, and a statement boundary that
            // wanted one is the space between two items.
            self.space();
            return;
        }
        if !self.at_line_start {
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    /// A comment that trailed code in the original (no newline before it)
    /// stays trailing; an own-line comment writes at the current indent.
    fn comment(&mut self, text: &str) {
        if self.pending_newlines == 0 {
            self.space();
        } else {
            self.prep_line();
        }
        self.pending_newlines = 0;
        self.write(text);
        self.end_line();
    }

    fn note_whitespace(&mut self, text: &str) {
        self.pending_newlines = text.matches('\n').count();
    }

    fn blank_line_if_pending(&mut self) {
        if self.run.is_some() {
            return;
        }
        let starts_line = self.at_line_start || self.need_newline;
        if starts_line && self.pending_newlines >= 2 && !self.out.is_empty() {
            self.prep_line();
            self.out.push('\n');
        }
        // Note: pending_newlines is *not* reset here — the comment logic
        // still needs it; real tokens reset it in after_significant.
    }

    /// Whether the next word-like token needs a space before it.
    ///
    /// The listed kinds are the ones nothing follows with a gap: an opener
    /// (`(`, `[`, `<`), a separator that already wrote its own space, or a
    /// sign that belongs to the number after it. `<` earns its place the way
    /// `[` does — it opens a parameter, and `Option<Pitch>` is one word for
    /// the same reason `[c4, d4]` starts tight.
    fn needs_word_space(&self) -> bool {
        let Some(prev) = self.prev else {
            return false;
        };
        if self.at_line_start {
            return false;
        }
        !matches!(
            prev,
            SyntaxKind::LParen
                | SyntaxKind::LBracket
                | SyntaxKind::Minus
                | SyntaxKind::Comma
                | SyntaxKind::Colon
                | SyntaxKind::Equals
                | SyntaxKind::Arrow
                | SyntaxKind::PipeForward
                | SyntaxKind::Slash
                | SyntaxKind::Less
        )
    }

    fn after_significant(&mut self, kind: SyntaxKind) {
        self.prev = Some(kind);
        self.pending_newlines = 0;
    }

    fn indent_more(&mut self) {
        self.indent = self.indent.saturating_add(4);
    }

    fn indent_less(&mut self) {
        self.indent = self.indent.saturating_sub(4);
    }
}
