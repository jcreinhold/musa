//! The two things `syntax` writes, read by the fixed reader
//! (`docs/notes/research/language-design-closure/26-language-design-decision.md`
//! §3.1–§3.2).
//!
//! What this crate owns of adapter expansion is exactly the surface: a header
//! statement that names which package reads a region, and a named delimited
//! region whose interior is grouped and not otherwise read. The meaning is the
//! compiler's, and these laws are about the reader refusing to have an opinion
//! on it.

#![allow(clippy::expect_used, clippy::panic)]

use musa_syntax::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

const REGION: &str = r#"piece "Adapters" {
    import syntax std::adapters::doubled as doubled;

    let total = syntax doubled { 3 (4 [5]) c#4/4. };

    score { part piano { voice one { c4/1 } } }
}
"#;

/// Every node kind under `root`, in tree order.
fn nodes(node: &musa_syntax::SyntaxNode) -> Vec<SyntaxKind> {
    node.descendants().map(|it| it.kind()).collect()
}

/// The one region in `REGION`.
fn region(document: &musa_syntax::ParsedDocument) -> musa_syntax::SyntaxNode {
    document
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::SyntaxRegion)
        .expect("the piece writes one region")
}

#[test]
fn a_syntax_import_and_a_region_are_both_read_without_complaint() {
    let document = parse(REGION);
    assert!(
        document.errors().is_empty(),
        "the header form and the region are grammar: {:?}",
        document
            .errors()
            .iter()
            .map(musa_syntax::SyntaxError::message)
            .collect::<Vec<_>>()
    );
    let import = document
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::ImportStmt)
        .expect("the piece writes one import");
    assert!(
        import
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::SyntaxKw),
        "a syntax import is told from an ordinary one by the word in it, not by lookahead"
    );
}

#[test]
fn the_interior_of_a_region_is_grouped_and_nothing_more() {
    let document = parse(REGION);
    let region = region(&document);
    let inside = nodes(&region);
    assert!(
        inside
            .iter()
            .all(|kind| matches!(kind, SyntaxKind::SyntaxRegion | SyntaxKind::SyntaxGroup)),
        "the fixed grouper forms a node per matched delimiter pair and no other node: {inside:?}"
    );
    // `{ 3 (4 [5]) … }` — the braces, the parentheses, and the brackets.
    assert_eq!(
        inside.iter().filter(|kind| **kind == SyntaxKind::SyntaxGroup).count(),
        3,
        "one group per matched pair"
    );
}

#[test]
fn a_region_keeps_every_character_it_was_written_with() {
    let document = parse(REGION);
    let region = region(&document);
    assert!(
        region.text().to_string().contains("c#4/4."),
        "a spelling only the adapter can read still has to survive the reader"
    );
    assert_eq!(
        document.syntax().text().to_string(),
        REGION,
        "the reader is lossless over a region as it is over everything else"
    );
}

#[test]
fn the_formatter_has_no_opinion_about_the_inside_of_a_region() {
    let document = parse(REGION);
    let written = format(&document, BarSpacing::default()).text().to_owned();
    assert!(
        written.contains("syntax doubled { 3 (4 [5]) c#4/4. }"),
        "the interior is the adapter's language, so it is written as it stands:\n{written}"
    );
    let again = format(&parse(&written), BarSpacing::default()).text().to_owned();
    assert_eq!(again, written, "formatting is idempotent over a region too");
}

#[test]
fn a_syntax_import_that_names_nothing_is_refused() {
    let document = parse("piece \"x\" {\n    import syntax std::adapters::doubled;\n}\n");
    assert!(
        document.errors().iter().any(|error| error.message().contains("as")),
        "a region names its adapter, so an import that gives it no name has nothing to be named by"
    );
}
