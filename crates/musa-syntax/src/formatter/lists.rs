//! Whether a comma-separated list is written across the page or down it,
//! and the width questions that decide it.
//!
//! One concern of the `formatter` module; see its docs for the rules.

use super::nodes::{format_node, one_line, wraps_across_lines};
use super::tokens::format_token;
use super::writer::Writer;
use super::{Layout, MEASURE};
use crate::language::{SyntaxNode, SyntaxToken};
use crate::{SyntaxElement, SyntaxKind};
use std::collections::HashSet;

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
pub(super) fn breakable_list(kind: SyntaxKind) -> bool {
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

/// Whether a list of this kind ends with a trailing comma when it is written
/// down the page — [`breakable_list`] minus the one that does not.
///
/// A product is the exception, and the grammar says so first: `(a, b,)` does
/// not parse, because `paren_or_product_expr` reads an expression after every
/// comma. But it would be the exception anyway. A trailing comma is for a list
/// whose length is open, where the next item is added by writing a line and
/// not by editing the line above it. A product's arity is part of its type:
/// there is no next item to add, so there is nothing for the comma to hold a
/// place for.
pub(super) fn carries_a_trailing_comma(kind: SyntaxKind) -> bool {
    breakable_list(kind) && kind != SyntaxKind::ProductExpr
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
pub(super) fn list_breaks(node: &SyntaxNode, writer: &Writer, layout: &Layout) -> bool {
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

/// Whether a branch of `owner` continues the line it is written on rather than
/// ending it.
///
/// Neither branch of a conditional ends anything: `else` follows the consequent
/// and closes back onto its brace, and whatever the whole `if` was written into
/// — a `,`, a `;`, nothing — follows the alternative. A ladder therefore reads
/// `} else if … {` down one column instead of putting each `else` on a line of
/// its own, and the comma after a conditional arm stays on the arm.
pub(super) fn continues_past_a_branch(owner: &SyntaxNode) -> bool {
    owner.kind() == SyntaxKind::IfExpr
}

/// Whether `child` is a consequent of the conditional `owner` — a branch that
/// `else` follows, rather than the alternative the whole `if` ends with.
///
/// Reaching this question at all means the conditional did not fit on one line:
/// its parent measured it and wrote it out longhand instead. A rung whose body
/// still fits would then stay on the line, and the ladder would keep growing
/// rightwards until some condition's argument list was broken to make room —
/// a break taken in the wrong place, on a call rather than between rungs. So a
/// consequent goes down the page and the next rung opens with `} else if` back
/// at the ladder's own indent. The alternative is exempt: nothing follows it to
/// push anything rightwards, and `} else { None }` reads as one closing line.
pub(super) fn opens_a_further_rung(owner: &SyntaxNode, child: &SyntaxNode) -> bool {
    owner.kind() == SyntaxKind::IfExpr
        && child.kind() == SyntaxKind::BlockExpr
        && owner.children().last().as_ref() != Some(child)
}

/// Whether a local binding's `;` ends the line, or is followed by its body on
/// the same one.
///
/// A `let` in a block is a *step*: the block is read down the page, and the
/// binding, the bindings after it, and the expression they are all for each
/// take a line. Everywhere else a `let` is written into the middle of
/// something — a match arm's result, a `quote at here { … }`'s interior — and
/// breaking there would put the body at the indent of whatever the `let` is
/// nested in, which in an arm is the indent of the *next arm*. An author who
/// wants the stacked reading in one of those places writes the braces that
/// ask for it, and `-> { let … }` gets exactly the block layout above.
///
/// The chain is walked because only its outermost link is the block's child:
/// `let a = …; let b = …; e` is one `let` inside another, and all of its
/// semicolons end their lines or none of them do.
pub(super) fn binding_ends_its_line(binding: &SyntaxNode) -> bool {
    let mut outermost = binding.clone();
    while let Some(owner) = outermost.parent() {
        if owner.kind() != SyntaxKind::LetExpr {
            return owner.kind() == SyntaxKind::BlockExpr;
        }
        outermost = owner;
    }
    false
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
/// editable: with one, every item is a whole line that can be moved, copied or
/// deleted without touching its neighbours, and adding an item after the last
/// one does not edit the last one. That is a property of the *layout*, so the
/// layout writes it — [`format_token`] adds the comma when a list stacks and
/// drops it when the list is joined back onto one line, where `f(a, b,)` is a
/// separator with nothing to separate.
///
/// This is the one token the formatter writes without being given it, and the
/// exception is narrow on purpose: only inside a list that
/// [`carries_a_trailing_comma`], only in the last position, and it says
/// nothing — a list means what it means with the
/// comma or without it, which is why the two spellings were free to drift apart
/// in the first place. Every other token is the program, and the program is
/// read, never written.
/// Whether this `:` is one half of the `::` that spells a namespace.
///
/// Looks both ways: the first half is followed by a colon and the second is
/// preceded by one, and neither may take the space an annotation's colon does.
/// Two adjacent colons mean nothing else in this grammar — a `measure:beat`
/// position is written as one word elsewhere — so the pair is enough to decide
/// it without asking what encloses them.
pub(super) fn halves_a_path_separator(colon: &SyntaxToken) -> bool {
    let neighbour = |mut side: Option<SyntaxElement>, step: fn(&SyntaxElement) -> Option<SyntaxElement>| {
        while let Some(element) = side {
            if element.kind() != SyntaxKind::Whitespace {
                return element.kind() == SyntaxKind::Colon;
            }
            side = step(&element);
        }
        false
    };
    neighbour(colon.next_sibling_or_token(), SyntaxElement::next_sibling_or_token)
        || neighbour(colon.prev_sibling_or_token(), SyntaxElement::prev_sibling_or_token)
}

pub(super) fn ends_its_list(comma: &SyntaxToken) -> bool {
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
