//! `11-quotation.md` §2 as syntax: what a quote's body is, where a `$` means
//! something, and what the formatter does with the three splice spellings.
//!
//! The one law worth stating twice, because it is the whole design: **the body
//! is read by the ordinary parser**. So the tests here do not check that a
//! quote body parses into some quote-shaped tree of its own — they check that
//! it parses into the *same* nodes the same text parses into outside a quote.
//! A quotation that did not share the parser would be a second grammar to keep
//! in step with the first, which is what `AGENTS.md` forbids under
//! "no sublanguage by subtraction".
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use musa_language::ast::{AstNode, QuoteExpr, SequenceSplice, Splice};
use musa_language::{BarSpacing, ParsedDocument, SyntaxKind, format, parse};

/// The three spellings of a splice, in the positions the staff adapter's
/// twenty construction sites put them in (note 43 §1.2).
const QUOTES: &str = r"library {
    fn emit(here: NodePath, event: Syntax<Expr>, items: Syntax<Expr>) -> Syntax<Expr> {
        quote at here { Sounded(${ anchored(region, here) }, $event, $items) }
    }

    fn nothing(here: NodePath) -> Syntax<Expr> { quote at here { NoItems } }

    fn every(here: NodePath, xs: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { Items([$..xs]) }
    }
}
";

fn parsed(source: &str) -> ParsedDocument {
    let document = parse(source);
    assert!(
        document.errors().is_empty(),
        "the program did not parse: {:?}",
        document.errors().iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    document
}

fn nodes(document: &ParsedDocument, kind: SyntaxKind) -> Vec<musa_language::SyntaxNode> {
    document
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .collect()
}

/// A node's shape, with every trivia token and every source range dropped.
///
/// What is left is exactly what two texts have to agree on to have parsed the
/// same way, which is the comparison
/// [`the_body_parses_into_what_the_same_text_parses_into_anywhere_else`] makes.
fn shape(node: &musa_language::SyntaxNode) -> String {
    let mut out = String::new();
    write_shape(node, 0, &mut out);
    out
}

fn write_shape(node: &musa_language::SyntaxNode, depth: usize, out: &mut String) {
    use std::fmt::Write as _;

    out.push_str(&"  ".repeat(depth));
    let _ = writeln!(out, "{:?}", node.kind());
    for child in node.children() {
        write_shape(&child, depth.saturating_add(1), out);
    }
}

#[test]
fn a_quote_holds_an_anchor_and_a_body_and_nothing_else() {
    let document = parsed(QUOTES);
    let quotes = nodes(&document, SyntaxKind::QuoteExpr);
    assert_eq!(quotes.len(), 3, "three quotes were written");
    for quote in &quotes {
        let quote = QuoteExpr::cast(quote.clone()).expect("a quote node casts");
        assert!(quote.anchor().is_some(), "a quote names the node it derives from");
        assert!(quote.body().is_some(), "a quote has a body");
    }
}

#[test]
fn the_body_parses_into_what_the_same_text_parses_into_anywhere_else() {
    // The law the whole design rests on. `Sounded(a, b, c)` inside a quote is
    // an `ApplyExpr` over an `ExprArgList`, exactly as it is in a `let`,
    // because it went through `Parser::expr` either way — and the splices are
    // the only nodes in it that a quote adds.
    let quoted =
        parsed("library {\n    fn f(here: NodePath) -> Syntax<Expr> { quote at here { Sounded(one, two) } }\n}\n");
    let plain = parsed("library {\n    let it = Sounded(one, two);\n}\n");
    let inside = QuoteExpr::cast(nodes(&quoted, SyntaxKind::QuoteExpr).remove(0))
        .expect("a quote node casts")
        .body()
        .expect("the quote has a body");
    let outside = nodes(&plain, SyntaxKind::ApplyExpr).remove(0);
    assert_eq!(
        shape(&inside),
        shape(&outside),
        "a quote body and ordinary source disagree about what an expression is"
    );
}

#[test]
fn the_shorthand_and_the_expression_form_are_one_node() {
    // `$x` is `${ x }` with the braces left off, so nothing downstream has to
    // know which was written: both hold one expression, and the shorthand's is
    // the name.
    let document = parsed(
        "library {\n    fn f(here: NodePath, x: Syntax<Expr>) -> Syntax<Expr> { quote at here { P($x, ${ g(x) }) } }\n}\n",
    );
    let splices = nodes(&document, SyntaxKind::Splice);
    assert_eq!(splices.len(), 2, "two splices were written");
    for splice in &splices {
        let splice = Splice::cast(splice.clone()).expect("a splice node casts");
        assert!(splice.expr().is_some(), "a splice holds the expression it splices");
    }
}

#[test]
fn a_sequence_splice_names_the_list_it_spreads() {
    let document = parsed(QUOTES);
    let spreads = nodes(&document, SyntaxKind::SequenceSplice);
    assert_eq!(spreads.len(), 1, "one sequence splice was written");
    let spread = SequenceSplice::cast(spreads[0].clone()).expect("a sequence splice casts");
    assert_eq!(spread.name().as_deref(), Some("xs"));
}

#[test]
fn a_dollar_outside_a_quote_is_not_an_expression() {
    // §2: "`$` is part of the quote's grammar. It is not an operator, it has
    // no meaning outside a quote". The parser is where that is said, so a `$`
    // written in ordinary source is a syntax error rather than a node whose
    // meaning some later pass has to refuse.
    let document = parse("library {\n    let it = $x;\n}\n");
    assert!(
        document.errors().iter().any(|error| error.to_string().contains('$')),
        "`$` outside a quote was read as something: {:?}",
        document.errors().iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    assert!(
        nodes(&document, SyntaxKind::Splice).is_empty(),
        "a splice was built outside a quote"
    );
}

#[test]
fn a_splice_inside_a_splice_belongs_to_the_quote_written_there() {
    // The depth rule, from the outside: the expression inside `${ … }` is host
    // code, so a `$` in it is a stray character unless a quote is written
    // *there*. A nested quote makes it a splice again.
    let nested = parsed(
        "library {\n    fn f(here: NodePath, x: Syntax<Expr>) -> Syntax<Expr> {\n        quote at here { P(${ g(quote at here { Q($x) }) }) }\n    }\n}\n",
    );
    assert_eq!(
        nodes(&nested, SyntaxKind::QuoteExpr).len(),
        2,
        "the inner quote is a quote"
    );
    assert_eq!(
        nodes(&nested, SyntaxKind::Splice).len(),
        2,
        "the outer `${{ … }}` and the inner `$x` are the two splices"
    );

    let stray = parse(
        "library {\n    fn f(here: NodePath, x: Syntax<Expr>) -> Syntax<Expr> { quote at here { P(${ g($x) }) } }\n}\n",
    );
    assert!(
        !stray.errors().is_empty(),
        "a `$` inside a splice's own expression was read as a splice of the enclosing quote"
    );
}

#[test]
fn the_quote_forms_round_trip_and_format_to_a_fixpoint() {
    let once = format(&parsed(QUOTES), BarSpacing::Compact).text().to_owned();
    let twice = format(&parsed(&once), BarSpacing::Compact).text().to_owned();
    assert_eq!(once, twice, "formatting a formatted quote moved it");
    // A splice is written as the one node it stands for: no space after the
    // `$`, one space inside `${ … }`, and no line break anywhere inside it.
    assert!(once.contains("${ anchored(region, here) }"), "{once}");
    assert!(once.contains("$event, $items"), "{once}");
    assert!(once.contains("[$..xs]"), "{once}");
}
