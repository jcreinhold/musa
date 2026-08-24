//! How one lexeme is spaced against the lexeme before it.
//!
//! One concern of the `formatter` module; see its docs for the rules.

use super::lists::{
    binding_ends_its_line, breakable_list, carries_a_trailing_comma, continues_past_a_body, ends_its_list,
    halves_a_path_separator,
};
use super::writer::Writer;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// Space and write one lexeme.
///
/// A lexeme is a token, or one of the three composite literals — `c#5`, `M3`,
/// `3/8` — which the lexer reads as one token and the parser writes as a node
/// over its parts. Spacing is a question about the lexeme the composer typed,
/// so the parts are not asked: the node's whole text is written as the one
/// word it is, and the two rules that read a *token* (a trailing comma's, a
/// path separator's) ask kinds no literal has.
pub(super) fn format_token(node: &SyntaxNode, lexeme: &SyntaxElement, writer: &mut Writer) {
    let kind = lexeme.kind();
    let whole;
    let text: &str = match lexeme {
        SyntaxElement::Node(literal) => {
            whole = literal.text().to_string();
            &whole
        }
        SyntaxElement::Token(token) => token.text(),
    };
    let parent = node.kind();
    // Whether this token belongs to a list that is being written down the
    // page. Only the list's own punctuation asks, and a token whose parent is
    // the list is directly inside it, so the writer's innermost answer is that
    // list's.
    let stacked = breakable_list(parent) && writer.list_breaks();
    // A lambda's braces close an expression that has more after it, so its
    // `}` does not end the line the way a declaration's body does. A match
    // arm's body and a conditional's branches close the same way — see
    // [`continues_past_a_body`] for what follows each. Either way the brace
    // leaves the line open for it.
    let held = parent == SyntaxKind::BlockExpr && node.parent().is_some_and(|owner| continues_past_a_body(&owner));
    if kind == SyntaxKind::LineComment || kind == SyntaxKind::BlockComment {
        writer.comment(text);
        return;
    }
    // The two halves of the trailing-comma rule, taken before `prep_line`
    // because both are decided on the line the last item ended, not on the
    // line its closer will start. See [`ends_its_list`].
    if stacked
        && carries_a_trailing_comma(parent)
        && matches!(kind, SyntaxKind::RParen | SyntaxKind::RBracket)
        && writer.wants_trailing_comma()
    {
        writer.write(",");
        writer.after_significant(SyntaxKind::Comma);
        writer.end_line();
    }
    if kind == SyntaxKind::Comma
        && !stacked
        && carries_a_trailing_comma(parent)
        && lexeme.as_token().is_some_and(ends_its_list)
    {
        writer.skip_token();
        return;
    }
    writer.prep_line();
    // `01-surface.md` §1's two braced forms — `<{n : Nat}>` and `{A = Nat}` —
    // are the only braces in the language that do not open a block. They hold
    // one name and at most one type or expression, so they are written the way
    // they were read: on the line, closed up to the `<` or the `(` in front of
    // them. Taken before the general brace rule because that rule's whole job
    // is to break the line, which is exactly what these must not do.
    if matches!(parent, SyntaxKind::TypeParam | SyntaxKind::SuppliedArg)
        && matches!(kind, SyntaxKind::LBrace | SyntaxKind::RBrace)
    {
        if kind == SyntaxKind::LBrace && writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
        writer.after_significant(kind);
        if kind == SyntaxKind::LBrace {
            writer.close_up();
        }
        return;
    } else if kind == SyntaxKind::LBrace {
        writer.space();
        writer.write("{");
        writer.indent_more();
        writer.end_line();
    } else if kind == SyntaxKind::RBrace {
        writer.indent_less();
        // A block's one expression ends with no `;` to end its line, so the
        // brace that closes it asks for the line itself. A declaration's last
        // variant may be written without its trailing comma, which leaves the
        // same brace stranded after it — and so does a match whose last arm is
        // a braced body, because that body holds its line open for a comma
        // that was never written.
        if matches!(
            parent,
            SyntaxKind::BlockExpr
                | SyntaxKind::MatchExpr
                | SyntaxKind::QuoteExpr
                | SyntaxKind::QuotePattern
                | SyntaxKind::DataDecl
                | SyntaxKind::RecordUpdateExpr
                | SyntaxKind::RecordDecl
                | SyntaxKind::EnumDecl
                | SyntaxKind::EnumCase
                | SyntaxKind::RecordLiteralExpr
                | SyntaxKind::RecordPattern
        ) {
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
        // A record update closes an *expression*, like a match: a `,` or a
        // `;` may follow it, so the brace leaves the line open for whatever
        // the update was written into. A record literal and a record pattern
        // close an expression and a pattern for the same reason — what follows
        // them is a `,`, a `)`, or the `->` of the arm they open. An enum
        // case's named fields close inside the declaration, and what follows
        // *that* brace is the comma before the next case.
        if !held
            && !matches!(
                parent,
                SyntaxKind::MusicExpr
                    | SyntaxKind::MatchExpr
                    | SyntaxKind::QuoteExpr
                    | SyntaxKind::QuotePattern
                    | SyntaxKind::RecordUpdateExpr
                    | SyntaxKind::RecordLiteralExpr
                    | SyntaxKind::RecordPattern
                    | SyntaxKind::EnumCase
            )
        {
            writer.end_line();
        }
    } else if kind == SyntaxKind::Semicolon {
        writer.write(";");
        if parent != SyntaxKind::LetExpr || binding_ends_its_line(node) {
            writer.end_line();
        }
    } else if kind == SyntaxKind::Comma {
        writer.write(",");
        // A match arm and a constructor are both a case of the same thing, and
        // both read as a list read downwards. A comma *inside* a constructor
        // separates its fields, which are one word's worth of a line, and that
        // comma belongs to the `DataVariant`, not to the declaration.
        //
        // An enum's cases are that same list, and a record's fields are one
        // per line wherever they are written — declared, built, or matched —
        // because a record is read far more often than it is written and its
        // field names are the interface. `EnumCase` is deliberately absent: a
        // comma there separates the *types* of a positional case, which is
        // `TiedOn(Nat, Text)` and one word's worth of a line.
        if matches!(
            parent,
            SyntaxKind::MatchExpr
                | SyntaxKind::DataDecl
                | SyntaxKind::RecordUpdateExpr
                | SyntaxKind::EnumDecl
                | SyntaxKind::RecordLiteralExpr
                | SyntaxKind::RecordPattern
        ) || stacked
        {
            writer.end_line();
        } else if !lexeme.as_token().is_some_and(ends_its_list) {
            writer.space();
        }
    } else if kind == SyntaxKind::Equals {
        writer.space();
        writer.write("=");
        writer.space();
    } else if kind == SyntaxKind::Colon {
        writer.write(":");
        // `::` is one separator the lexer happens to give in two tokens, and
        // §1.5 spells a namespaced case with it. Neither half takes a space,
        // so `Tying::Untied` is written the way it was read rather than as a
        // colon between two names. Asked of the tokens either side rather
        // than of the parent, because a path is spelled the same in a pattern
        // — where it is bumped straight into the arm — as in an expression,
        // where it is a `PathExpr`.
        if parent != SyntaxKind::ImportStmt && !lexeme.as_token().is_some_and(halves_a_path_separator) {
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
        // list. An index closes up to what it indexes, the way a call closes
        // up to what it calls: `xs[i]`, never `xs [i]`.
        if parent != SyntaxKind::IndexExpr && writer.needs_word_space() {
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
    } else if parent == SyntaxKind::MethodCallExpr {
        // `f(x).m(y)` is one expression written in pieces, and neither the `.`
        // nor the method's name takes a space in front of it: a method closes
        // up to its receiver exactly the way a projection does, and the only
        // reason this is said here rather than in the one-word list is that
        // the receiver is a node and the list writes tokens.
        writer.write(text);
    } else if parent == SyntaxKind::BinaryExpr {
        // An operator is a word between two operands and is written like one:
        // `a + b`, `xs / 2`, `x == y`. Three of the six spellings — `/`, `<`,
        // and `-` — close up to their neighbour everywhere else, because
        // everywhere else they are part of a duration, a type parameter, or a
        // sign. What tells the two apart is the node, so the node is what this
        // asks.
        writer.space();
        writer.write(text);
        writer.space();
    } else if matches!(
        kind,
        SyntaxKind::Slash
            | SyntaxKind::Dot
            | SyntaxKind::Greater
            | SyntaxKind::Caret
            | SyntaxKind::Less
            | SyntaxKind::Question
    ) {
        // A short-form duration is part of the note's word: `c4/4.` is one
        // note written one way, not a pitch beside a fraction beside a dot.
        // An accent or a marcato is drawn on its notehead, so it is written
        // on its note: `c4/4>`, never `c4/4 >`. A type parameter binds to its
        // type the same way: `Option<Pitch>`, never `Option <Pitch>`. A
        // question is postfix and closes up for the same reason: `read(here)?`
        // asks about the call, and a space would make the `?` look like a
        // token of the line rather than part of the expression.
        writer.write(text);
    } else {
        if writer.needs_word_space() {
            writer.space();
        }
        writer.write(text);
    }
    writer.after_significant(kind);
}
