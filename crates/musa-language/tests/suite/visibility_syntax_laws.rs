//! `01-surface.md` §1.3 as syntax: where `private` may stand, what the tree
//! keeps of it, what the formatter does with it, and what the grammar refuses.
//!
//! One program rather than a fragment per law, for the reason
//! `record_syntax_laws.rs` gives: a marker that only worked in the snippet it
//! was written next to would be a marker about that snippet. This one is note 43
//! §5.1's `Chord` beside the other four declarations the word may mark, so the
//! layout laws see it in every position at once.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use musa_language::ast::{AstNode, EnumDecl, FnDecl, LetDecl, RecordDecl, StructureDecl};
use musa_language::{BarSpacing, ParsedDocument, SyntaxElement, SyntaxKind, format, parse};

/// The program every law here is stated over.
const HIDDEN: &str = r"library {
    signature Chords {
        let unison: Nat;
    }

    private let concert_a = 440;

    private fn dotted_factor(dots: Nat) -> Nat { dots }

    private record Pending {
        read: Nat;
    }

    enum Chord {
        private NamedChord(Symbol),
        private AnonymousChord(Nat),
    }

    private enum Tuning {
        Equal,
    }

    private structure Common : Chords {
        let unison = 0;

        fn double(n: Nat) -> Nat { n }
    }

    fn build(sym: Symbol) -> Chord { Chord::NamedChord(sym) }
}
";

/// The significant tokens of a tree, which formatting may not change.
fn significant_shape(parsed: &ParsedDocument) -> Vec<(SyntaxKind, String)> {
    parsed
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| match element {
            SyntaxElement::Token(token) => (!matches!(
                token.kind(),
                SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
            ))
            .then(|| (token.kind(), token.text().to_owned())),
            SyntaxElement::Node(_) => None,
        })
        .collect()
}

/// The root a `library` declares into, which is where the declarations are.
fn library(parsed: &ParsedDocument) -> musa_language::SyntaxNode {
    parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == SyntaxKind::LibraryDecl)
        .expect("the program is a library")
}

/// §1.3: the marker is a token of the declaration it marks, so every accessor
/// that declaration had still reads the same node.
#[test]
fn a_marked_declaration_is_the_same_node_and_says_it_is_private() {
    let parsed = parse(HIDDEN);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let root = library(&parsed);

    let lets: Vec<LetDecl> = root.children().filter_map(LetDecl::cast).collect();
    let [concert_a] = lets.as_slice() else {
        panic!("expected the one marked `let`, found {}", lets.len());
    };
    assert_eq!(concert_a.name().as_deref(), Some("concert_a"));
    assert!(concert_a.is_private());

    let fns: Vec<FnDecl> = root.children().filter_map(FnDecl::cast).collect();
    let [dotted, build] = fns.as_slice() else {
        panic!("expected two functions, found {}", fns.len());
    };
    assert_eq!(dotted.name().as_deref(), Some("dotted_factor"));
    assert!(dotted.is_private(), "the marked one is private");
    assert_eq!(build.name().as_deref(), Some("build"));
    assert!(!build.is_private(), "and the unmarked one is not");

    let records = RecordDecl::all_at_root(&root);
    let [pending] = records.as_slice() else {
        panic!("expected one record, found {}", records.len());
    };
    assert_eq!(pending.name().as_deref(), Some("Pending"));
    assert!(pending.is_private());
    assert_eq!(pending.fields().len(), 1, "the marker is not read as a field");

    let structures = StructureDecl::all_at_root(&root);
    let [common] = structures.as_slice() else {
        panic!("expected one structure, found {}", structures.len());
    };
    assert_eq!(common.name().as_deref(), Some("Common"));
    assert_eq!(
        common.signature().as_deref(),
        Some("Chords"),
        "the marker is not read as the structure's name"
    );
    assert!(common.is_private());
}

/// §1.3: the two positions are independent. An enum may hide its cases and keep
/// its type public, or hide the type, and the tree tells them apart.
#[test]
fn a_case_and_its_type_are_marked_separately() {
    let parsed = parse(HIDDEN);
    let root = library(&parsed);
    let enums = EnumDecl::all_at_root(&root);
    let [chord, tuning] = enums.as_slice() else {
        panic!("expected two enums, found {}", enums.len());
    };

    assert_eq!(chord.name().as_deref(), Some("Chord"));
    assert!(!chord.is_private(), "the type stays public");
    let cases = chord.cases();
    assert_eq!(cases.len(), 2, "the marker is not read as a case");
    for case in &cases {
        assert!(case.is_private(), "`{:?}` is package-maintained", case.name());
    }
    assert_eq!(cases[0].name().as_deref(), Some("NamedChord"));
    assert!(
        cases[0]
            .syntax()
            .descendants()
            .any(|node| node.kind() == SyntaxKind::TypeName),
        "`Symbol` is still the case's payload"
    );

    assert_eq!(tuning.name().as_deref(), Some("Tuning"));
    assert!(tuning.is_private(), "and a whole type may be hidden instead");
    let cases = tuning.cases();
    let [equal] = cases.as_slice() else {
        panic!("expected one case, found {}", cases.len());
    };
    assert!(!equal.is_private(), "which is not the same as hiding its cases");
}

/// The program round-trips losslessly and formats to a fixpoint.
#[test]
fn a_marked_program_round_trips_and_formats_to_a_fixpoint() {
    let parsed = parse(HIDDEN);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let before = significant_shape(&parsed);
    assert_eq!(parsed.syntax().text().to_string(), HIDDEN, "parsing is lossless");

    let once = format(&parsed, BarSpacing::Compact).to_string();
    let reparsed = parse(&once);
    assert!(reparsed.errors().is_empty(), "{:?}\n{once}", reparsed.errors());
    assert_eq!(
        significant_shape(&reparsed),
        before,
        "formatting changed the tree:\n{once}"
    );
    assert_eq!(
        format(&reparsed, BarSpacing::Compact).to_string(),
        once,
        "formatting is not a fixpoint:\n{once}"
    );
}

/// The layout: the marker leads its declaration's line, one space before the
/// word it marks, and a marked case is still one case to a line.
#[test]
fn the_marker_leads_the_line_it_marks() {
    let once = format(&parse(HIDDEN), BarSpacing::Compact).to_string();
    let lines: Vec<&str> = once.lines().map(str::trim).collect();
    for wanted in [
        "private let concert_a = 440;",
        "private record Pending {",
        "private enum Tuning {",
        "private NamedChord(Symbol),",
        "private AnonymousChord(Nat),",
    ] {
        assert!(lines.contains(&wanted), "expected a line `{wanted}`:\n{once}");
    }
    for wanted in [
        "private fn dotted_factor(dots: Nat)",
        "private structure Common: Chords",
    ] {
        assert!(
            lines.iter().any(|line| line.starts_with(wanted)),
            "expected a line starting `{wanted}`:\n{once}"
        );
    }
}

/// §1.3: `private` marks a declaration, and an `import` or a `make` is not one.
/// The complaint points at the marker rather than at the word after it.
#[test]
fn a_marker_on_something_that_is_not_a_declaration_is_refused() {
    let parsed = parse("library {\n    private use x;\n}\n");
    let messages: Vec<String> = parsed.errors().iter().map(ToString::to_string).collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`private` does not mark this")),
        "expected the misplaced-marker complaint, got {messages:?}"
    );
    assert_eq!(
        parsed.syntax().text().to_string(),
        "library {\n    private use x;\n}\n",
        "a refused program is still read losslessly"
    );
}

/// The non-overlap with sealing, said where a reader would otherwise believe
/// the marker was doing something: a structure's signature already hides
/// everything it does not list.
#[test]
fn a_marker_inside_a_structure_says_the_signature_already_hides_it() {
    let source = "library {\n    signature S {\n        let a: Nat;\n    }\n\n    structure M : S {\n        private \
                  let b = 1;\n    }\n}\n";
    let parsed = parse(source);
    let messages: Vec<String> = parsed.errors().iter().map(ToString::to_string).collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("this is already private")),
        "expected the redundant-marker complaint, got {messages:?}"
    );
    assert_eq!(
        parsed.syntax().text().to_string(),
        source,
        "a refused program is still read losslessly"
    );
}

/// Marking a declaration changes nothing about the program that did not name
/// it: the same source without the markers parses to the same significant
/// tokens, minus exactly the markers.
#[test]
fn the_marker_is_the_only_difference_it_makes() {
    let marked = significant_shape(&parse(HIDDEN));
    let plain = significant_shape(&parse(&HIDDEN.replace("private ", "")));
    let stripped: Vec<_> = marked
        .into_iter()
        .filter(|(kind, _)| *kind != SyntaxKind::PrivateKw)
        .collect();
    assert_eq!(stripped, plain, "the marker changed how the rest was read");
}
