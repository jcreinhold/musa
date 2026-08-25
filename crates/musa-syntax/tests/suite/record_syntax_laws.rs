//! `01-surface.md` §1.2 and §1.3 as syntax: what a record and an enum look
//! like, what the formatter does with them, and what the grammar refuses.
//!
//! Stated over one program rather than a fragment per law. A record is
//! declared, built, projected, updated along a path, and matched; two enums
//! declare the same case spelling, which is the collision prompt 127dcfb hit
//! and the reason §1.3 namespaces constructors at all. A law that held only for
//! the snippet it was written next to would be a law about that snippet.

use musa_syntax::ast::{AstNode, DataVariant, EnumCase, EnumDecl};
use musa_syntax::{BarSpacing, SyntaxElement, SyntaxKind, format, parse};

/// The program every layout law here is stated over.
///
/// `Pending` is `stdlib/src/adapters/staff.musa`'s eight-field product, cut to
/// the four fields the laws need: prompt 166 measures the real one.
const RECORDS: &str = r#"piece "Records" {
    record Region {
        anchor: Nat;
        span: Nat;
    }

    record Pending {
        read: Nat;
        length: Nat;
        region: Region;
    }

    enum Tying {
        Untied,
        TiedOn(Nat),
        Held { when: Nat; why: Text; },
    }

    enum Slur {
        Untied,
    }

    fn start() -> Pending { Pending { read = 0, length = 1, region = Region { anchor = 0, span = 0 } } }

    fn shift(held: Pending) -> Pending { held with { region.anchor = 1 } }

    fn width(held: Pending) -> Nat { held.region.span }

    fn tied(what: Tying) -> Nat { match what {
        Tying::Untied -> 0,
        Tying::TiedOn(n) -> n,
        Tying::Held -> 1,
    } }

    fn slurred(what: Slur) -> Nat { match what {
        Slur::Untied -> 0,
    } }

    fn reading(held: Pending) -> Nat { match held {
        { read = r } -> r,
    } }
}
"#;

/// The significant tokens of a tree, which formatting may not change.
fn significant_shape(parsed: &musa_syntax::ParsedDocument) -> Vec<(SyntaxKind, String)> {
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

fn count(parsed: &musa_syntax::ParsedDocument, kind: SyntaxKind) -> usize {
    parsed.syntax().descendants().filter(|node| node.kind() == kind).count()
}

/// Every new form parses, and the tree holds the node each one was written as.
#[test]
fn records_and_enums_parse_into_their_own_nodes() {
    let parsed = parse(RECORDS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    assert_eq!(count(&parsed, SyntaxKind::RecordDecl), 2, "Region and Pending");
    assert_eq!(count(&parsed, SyntaxKind::FieldDecl), 7, "2 + 3 declared, 2 in a case");
    assert_eq!(count(&parsed, SyntaxKind::EnumDecl), 2, "Tying and Slur");
    assert_eq!(count(&parsed, SyntaxKind::EnumCase), 4, "three cases and one");
    assert_eq!(count(&parsed, SyntaxKind::RecordLiteralExpr), 2, "Pending and Region");
    assert_eq!(count(&parsed, SyntaxKind::FieldInit), 5, "three fields and two");
    assert_eq!(
        count(&parsed, SyntaxKind::RecordPattern),
        1,
        "the one match on a record"
    );
    assert_eq!(count(&parsed, SyntaxKind::FieldPattern), 1, "`read = r`");
    assert_eq!(count(&parsed, SyntaxKind::FieldPath), 1, "`region.anchor`");
}

/// A `with` path is a path, and the tree keeps its segments.
///
/// The node exists so the compiler reads the segments it was given rather than
/// re-deriving them from the first identifier under a `FieldUpdate` — the same
/// "hand a consumer what we already computed" rule the adapter interface is
/// held to.
#[test]
fn an_update_path_keeps_every_segment() {
    let parsed = parse(RECORDS);
    // Every path in the program, rather than the first one: that the program
    // holds exactly one is half of what is being claimed, and a `find` that
    // silently took the first of several would not say it.
    let paths: Vec<Vec<String>> = parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::FieldPath)
        .map(|path| {
            path.children_with_tokens()
                .filter_map(|element| match element {
                    SyntaxElement::Token(token) if token.kind() == SyntaxKind::Identifier => {
                        Some(token.text().to_owned())
                    }
                    SyntaxElement::Token(_) | SyntaxElement::Node(_) => None,
                })
                .collect()
        })
        .collect();
    assert_eq!(paths, vec![vec!["region".to_owned(), "anchor".to_owned()]]);
}

/// The whole program round-trips losslessly and formats to a fixpoint.
#[test]
fn records_and_enums_round_trip_and_format_to_a_fixpoint() {
    let parsed = parse(RECORDS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let before = significant_shape(&parsed);
    assert_eq!(parsed.syntax().text().to_string(), RECORDS, "parsing is lossless");

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

/// The layout the formatter decided, stated as the lines it must write.
///
/// A declaration is read far more often than it is written, so its fields and
/// its cases go one to a line. A record *pattern* does not: it is followed by
/// the `->` of the arm it opens, and stacking it would strand the `}` before
/// the arrow.
#[test]
fn a_declaration_stacks_and_a_pattern_does_not() {
    let once = format(&parse(RECORDS), BarSpacing::Compact).to_string();
    let lines: Vec<&str> = once.lines().map(str::trim).collect();

    for wanted in [
        "record Region {",
        "anchor: Nat;",
        "enum Tying {",
        "Untied,",
        "TiedOn(Nat),",
    ] {
        assert!(lines.contains(&wanted), "expected a line `{wanted}`:\n{once}");
    }
    assert!(
        once.contains("{ read = r } ->"),
        "a record pattern belongs on the line of its arrow:\n{once}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("Held {")),
        "a case with named fields opens on the case's line:\n{once}"
    );
}

/// The pattern stays with its arrow even when the `match` itself is stacked.
///
/// The law above is read on a program short enough to inline, where every
/// layout question is answered by the enclosing block. This one takes the
/// block away: the arms are written down the page, and the pattern still may
/// not put its `}` on a line of its own.
#[test]
fn a_stacked_match_keeps_its_record_pattern_on_one_line() {
    let source = "piece \"x\" {\n\
         fn reading(held: Pending) -> Nat { match held {\n\
             { read = r, length = n, region = g, taken = t } -> r,\n\
         } }\n\
     }\n";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let once = format(&parsed, BarSpacing::Compact).to_string();
    assert!(
        once.lines()
            .any(|line| line.trim().starts_with('{') && line.contains("} ->")),
        "the pattern and its arrow belong on one line:\n{once}"
    );
    let again = parse(&once);
    assert!(again.errors().is_empty(), "{:?}\n{once}", again.errors());
    assert_eq!(
        format(&again, BarSpacing::Compact).to_string(),
        once,
        "formatting is not a fixpoint:\n{once}"
    );
}

/// Two enums in one piece may spell a case the same way.
///
/// This is the program prompt 127dcfb could not write: the staff adapter's
/// `Untied` and the staff package's `Untied` were one name, and the compiler
/// still carries `names_a_phase_type` as the scar. Here they are two, and the
/// tree says which is which.
#[test]
fn two_enums_may_share_a_case_spelling() {
    let parsed = parse(RECORDS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    // Read through the typed wrappers, which is how every consumer of this
    // crate will read it: a case knows its own name and the declaration it
    // belongs to knows the namespace, and neither has to be recovered from
    // the token stream.
    let piece = parsed.syntax();
    let declarations: Vec<EnumDecl> = piece.descendants().filter_map(EnumDecl::cast).collect::<Vec<_>>();
    let untied: Vec<String> = declarations
        .iter()
        .filter(|declaration| {
            declaration
                .cases()
                .iter()
                .any(|case| case.name().as_deref() == Some("Untied"))
        })
        .filter_map(EnumDecl::name)
        .collect();
    assert_eq!(untied, vec!["Tying".to_owned(), "Slur".to_owned()]);

    // And the whole point: the two `Untied`s are two cases, of two types.
    let cases: Vec<(String, String)> = declarations
        .iter()
        .filter_map(|declaration| Some((declaration.name()?, declaration.cases())))
        .flat_map(|(owner, cases)| {
            cases
                .into_iter()
                .filter_map(move |case| Some((owner.clone(), case.name()?)))
        })
        .filter(|(_, case)| case == "Untied")
        .collect();
    assert_eq!(
        cases,
        vec![
            ("Tying".to_owned(), "Untied".to_owned()),
            ("Slur".to_owned(), "Untied".to_owned())
        ]
    );
}

/// What the grammar refuses, and that it says something about each one.
#[test]
fn the_grammar_refuses_what_section_one_refuses() {
    // §1.2: an update names a *place*, so the left of an `=` is a path and a
    // call computes no place to put the answer back.
    let computed = parse("piece \"x\" {\n    fn f(p: Pending) -> Pending { p with { g(x).h = 1 } }\n}\n");
    assert!(!computed.errors().is_empty(), "`g(x).h` is not a path");

    // A field of a record declaration ends in `;`, so a list written with
    // commas is refused where it is written rather than at the closing brace.
    let commas = parse("piece \"x\" {\n    record R {\n        a: Nat,\n        b: Nat,\n    }\n}\n");
    assert!(!commas.errors().is_empty(), "a record's fields end in `;`");

    // A record literal writes `field = value`; a bare name is not a field.
    let bare = parse("piece \"x\" {\n    fn f() -> R { R { a } }\n}\n");
    assert!(!bare.errors().is_empty(), "§1.2 has no field punning");
}

/// `enum Empty {}` parses, and it is the type `P -> Empty` needs.
#[test]
fn an_enum_with_no_cases_is_admitted() {
    let source = "piece \"x\" {\n    enum Empty {\n    }\n}\n";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::EnumDecl), 1);
    assert_eq!(count(&parsed, SyntaxKind::EnumCase), 0);
}

/// The two capabilities prompt 161 moved onto `data`, written with `data`.
///
/// Before 161 a `private` case and a positional field list were `enum`'s alone,
/// which made `enum` a third declaration form rather than a second spelling of
/// one. They are here so that the general form is general — root `AGENTS.md`'s
/// rule about sublanguages, one level up — and §1.3's rule for each is
/// unchanged: the positional form names types and not fields, and a `private`
/// case leaves the type public.
const VARIANTS: &str = r#"piece "Variants" {
    data Chord {
        private NamedChord(Text, Nat),
        Anonymous(root: Nat, quality: Nat),
        Silence,
    }
}
"#;

#[test]
fn a_data_variant_writes_a_marker_and_a_positional_field() {
    let parsed = parse(VARIANTS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let variants: Vec<DataVariant> = parsed.syntax().descendants().filter_map(DataVariant::cast).collect();
    let read: Vec<(String, bool, usize, usize)> = variants
        .iter()
        .map(|variant| {
            (
                variant.name().unwrap_or_default(),
                variant.is_private(),
                variant.positional().len(),
                variant.fields().len(),
            )
        })
        .collect();
    assert_eq!(
        read,
        vec![
            ("NamedChord".to_owned(), true, 2, 0),
            ("Anonymous".to_owned(), false, 0, 2),
            ("Silence".to_owned(), false, 0, 0),
        ],
        "a `data` case writes a marker, a positional list, or named fields"
    );

    // The same case under the other word, read through the other wrapper: two
    // spellings, one shape.
    let twin = parse("piece \"x\" {\n    enum Chord {\n        private NamedChord(Text, Nat),\n    }\n}\n");
    assert!(twin.errors().is_empty(), "{:?}", twin.errors());
    let cases: Vec<(String, bool, usize)> = twin
        .syntax()
        .descendants()
        .filter_map(EnumCase::cast)
        .map(|case| {
            (
                case.name().unwrap_or_default(),
                case.is_private(),
                case.positional().len(),
            )
        })
        .collect();
    assert_eq!(cases, vec![("NamedChord".to_owned(), true, 2)]);
}

#[test]
fn the_variant_forms_round_trip_and_format_to_a_fixpoint() {
    let parsed = parse(VARIANTS);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let before = significant_shape(&parsed);
    assert_eq!(parsed.syntax().text().to_string(), VARIANTS, "parsing is lossless");

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

/// An index telescope parses under all three words, and only `data` keeps it.
///
/// The grammar admits what it cannot mean so that the refusal can be about
/// indexed families: a parser that stopped at the `(` would report the brace it
/// wanted, and the author would be left to guess. Where the refusal is stated
/// is `crates/musa-compiler/src/lower/items.rs`, and what it says is
/// `lower::laws::an_index_telescope_is_refused_by_the_word_that_wrote_it`.
#[test]
fn an_index_telescope_parses_under_every_word_and_is_refused_later() {
    for source in [
        "piece \"x\" {\n    data Vect(A: Type): (n: Nat) -> Type {\n        Nil : Vect(A, 0),\n    }\n}\n",
        "piece \"x\" {\n    record Pending: (n: Nat) -> Type {\n        read: Nat;\n    }\n}\n",
        "piece \"x\" {\n    enum Reading: (n: Nat) -> Type {\n        Done,\n    }\n}\n",
    ] {
        let parsed = parse(source);
        assert!(parsed.errors().is_empty(), "{source}\n{:?}", parsed.errors());
        assert_eq!(count(&parsed, SyntaxKind::DataIndices), 1, "one telescope in {source}");
    }
}
