//! What elaborating a document promises.
//!
//! Beside the driver rather than in `tests/suite/`, for [`crate::lower::laws`]'s
//! reason: the context is [`crate::registry::owned`] and the answer is a
//! [`musa_core::Term`], and both are private to this crate.
//!
//! # The survey
//!
//! Half of this module is [`the_standard_library_elaborates`] and its two
//! adapter laws, and they are the point of the prompt rather than a check on it.
//! Elaborating `stdlib/` through the new checker and recording exactly what it
//! still refuses is what tells prompt 142 which of its failures will be
//! migrations and which would be defects: every entry below is a spelling 142's
//! Target already owns, and an entry that appeared without one would be a defect
//! in one of the thirteen prompts that built the reading.
//!
//! The recorded lists are exact, the way `registry::rules::UNREGISTERED` is
//! exact. A corpus file that starts failing for a new reason fails the build; a
//! reason that stops applying has to be struck from the list in the commit that
//! fixed it, which is what keeps the survey a record rather than a wish.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_language::{SyntaxKind, SyntaxNode};

use super::{Document, Source, elaborate};
use crate::resolve::Resolver;

// ---- reading a written document back out of a parse ----

/// The `library { … }` node `source` writes, with a loud failure when it does
/// not parse.
fn library(source: &str) -> SyntaxNode {
    let document = musa_language::parse(source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    written_library(&document.syntax()).expect("the source writes a library")
}

/// The first `library` under `root`, when there is one.
fn written_library(root: &SyntaxNode) -> Option<SyntaxNode> {
    root.descendants().find(|node| node.kind() == SyntaxKind::LibraryDecl)
}

/// `source` elaborated on its own, and every distinct fault, sorted.
///
/// The fault is the code and the message and never the span, because a law that
/// pinned spans would fail on a blank line and the span is
/// [`crate::lower::refusals`]'s to get right.
fn elaborated(source: &str) -> (Option<Document>, Vec<String>) {
    faults(&[Source {
        root: library(source),
        in_phase: false,
    }])
}

/// The same for a document already assembled out of several sources.
fn faults(sources: &[Source]) -> (Option<Document>, Vec<String>) {
    let mut resolver = Resolver::new();
    let document = elaborate(&mut resolver, sources);
    let mut said: Vec<String> = resolver
        .diagnostics
        .iter()
        .map(|complaint| format!("{:?}: {}", complaint.code, complaint.message))
        .collect();
    said.sort();
    said.dedup();
    (document, said)
}

/// `source`, which every law that expects success reads its answer out of.
fn document(source: &str) -> Document {
    let (document, said) = elaborated(source);
    document.unwrap_or_else(|| panic!("the document elaborates: {said:?}"))
}

// ---- the walk ----

#[test]
fn ordinary_definitions_elaborate_and_read_back() {
    let document = document(
        "library {
            fn twice(n: Nat) -> Nat { Succ(Succ(n)) }
            let three: Nat = twice(1);
            let also: Nat = 3;
        }",
    );
    assert_eq!(
        document.names().iter().map(|name| &**name).collect::<Vec<_>>(),
        ["twice", "three", "also"],
        "the names are the ones the document wrote, in written order"
    );
    let (computed, _) = document.value("three").expect("`three` is bound");
    let (written, _) = document.value("also").expect("`also` is bound");
    assert_eq!(
        computed, written,
        "a definition reads back as the normal form its body computes"
    );
}

#[test]
fn a_body_may_name_a_definition_written_after_it() {
    let document = document(
        "library {
            let answer: Nat = later(1);
            fn later(n: Nat) -> Nat { Succ(n) }
            let written: Nat = 2;
        }",
    );
    let (answer, _) = document.value("answer").expect("`answer` is bound");
    let (written, _) = document.value("written").expect("`written` is bound");
    assert_eq!(answer, written, "and it means what it names");
}

#[test]
fn a_data_declaration_may_be_written_after_the_one_that_names_it() {
    let document = document(
        "library {
            data Held { Wrap(inner: Inner) }
            data Inner { Only }
            let held: Held = Wrap(Only);
        }",
    );
    let (value, _) = document.value("held").expect("`held` is bound");
    let shown = format!("{value:?}");
    assert!(shown.contains("\"Wrap\""), "the outer constructor: {shown}");
    assert!(shown.contains("\"Only\""), "and the inner one: {shown}");
}

#[test]
fn two_data_declarations_that_name_each_other_are_refused() {
    let (document, said) = elaborated(
        "library {
            data Left { Wrap(inner: Right) }
            data Right { Wrap(inner: Left) }
        }",
    );
    assert!(document.is_none(), "the cycle is refused");
    assert_eq!(
        said,
        ["DependencyCycle: `Left`, `Right` name each other"],
        "and the refusal names both declarations"
    );
}

#[test]
fn the_phase_vocabulary_is_readable_only_in_a_phase_source() {
    const WRITTEN: &str = "library { fn probe(held: Syntax<Expr>) -> Syntax<Expr> { held } }";
    let (ordinary, said) = elaborated(WRITTEN);
    assert!(ordinary.is_none(), "ordinary source cannot name `Syntax`: {said:?}");
    let (phase, said) = faults(&[Source {
        root: library(WRITTEN),
        in_phase: true,
    }]);
    assert!(phase.is_some(), "an adapter phase can: {said:?}");
}

/// A structured answer reads back as the data it is.
///
/// The compiler's half of prompt 141q, and the reason that prompt exists:
/// [`crate::registry::read_back`] answers a *base literal*, and a count and a
/// list are neither. `3` is `Nat.Succ` three deep over `Nat.Zero`, `[1, 2]` is
/// `List.Cons` over `List.Empty`, and both are constructor spines that
/// `read_back` refuses.
///
/// Stated over source rather than over hand-written terms because that is the
/// claim worth making. The shapes below are what `let n: Nat = 3;` *actually*
/// elaborates to through the prelude's declarations and 141g's lowering, so a
/// change to either that broke a consumer's reading would fail here rather than
/// in whatever pass first tried to count something.
///
/// The parameter is what `List` adds over `Nat`. A value of `List Nat` is
/// `List.Cons Nat 1 (…)`, and every field below is a field: the reading drops
/// the parameter because the declaration says where the fields begin.
#[test]
fn a_count_and_a_list_read_back_as_canonical_data() {
    let document = document(
        "library {
            let n: Nat = 3;
            let xs: List<Nat> = [1, 2];
        }",
    );
    let read = |name: &str| {
        let (normal, _) = document.value(name).expect("the definition is bound");
        musa_core::canonical(&normal).unwrap_or_else(|| panic!("`{name}` reads back as data"))
    };
    let case = |name: &str, fields: Vec<musa_core::Datum>| musa_core::Datum::Case {
        constructor: std::sync::Arc::from(name),
        fields,
    };
    let zero = case("Nat.Zero", Vec::new());
    let succ = |inner| case("Nat.Succ", vec![inner]);
    let three = succ(succ(succ(zero.clone())));
    assert_eq!(read("n"), three, "a count is the `Succ`s it is deep");
    assert_eq!(
        read("xs"),
        case(
            "List.Cons",
            vec![
                succ(zero.clone()),
                case("List.Cons", vec![succ(succ(zero)), case("List.Empty", Vec::new())]),
            ],
        ),
        "and a list is its members, with the element type left out"
    );
}

/// The two questions a [`Document`] answers are one interface, not two.
///
/// [`Document::value`] hands back an [`musa_core::ElabError`] and nothing else
/// about place, so the only way a caller turns one into a diagnostic is through
/// the site table the same document holds — which is the arrangement
/// `02-core-calculus.md` §7 asks for, with the core a leaf that never learns
/// what a file is. A name nobody wrote has no span, and the restatement says so
/// rather than pointing at node one.
#[test]
fn a_refusal_about_a_name_is_restated_through_the_documents_own_sites() {
    let document = document("library { let held: Nat = 1; }");
    let error = document.value("absent").expect_err("`absent` is not bound");
    let restated = crate::lower::refusals::restate(document.sites(), &error);
    assert_eq!(restated.code, crate::diagnose::Code::UnknownName);
    assert!(
        restated.labels.is_empty(),
        "a name the reading never numbered has nowhere to point: {:?}",
        restated.labels
    );
}

// ---- the survey ----

/// [`STANDARD_LIBRARY`], parsed, as the sources an importing document sees.
///
/// `pub(crate)` for one caller outside this module: prompt 141p's example survey
/// in [`crate::lower::piece::laws`] reads the same corpus, because an example
/// that writes `use theory::harmony;` sees exactly these and surveying it
/// without them would report missing names that are not missing. One list rather
/// than two, so a library added to `stdlib/` cannot appear in one survey and not
/// the other.
pub(crate) fn library_sources() -> Vec<Source> {
    STANDARD_LIBRARY
        .iter()
        .map(|&(name, source)| {
            let held = musa_language::parse(source);
            assert!(held.errors().is_empty(), "`{name}` parses: {:?}", held.errors());
            Source {
                root: written_library(&held.syntax()).expect("every standard library file writes a library"),
                in_phase: false,
            }
        })
        .collect()
}

/// Every library `stdlib/` writes that is not an adapter, in import order.
///
/// One document rather than sixteen, because that is what an importing file
/// sees: `option_fold` is written in `option.musa` and read in four others, and
/// surveying each file alone would report fifteen missing names that are not
/// missing at all.
const STANDARD_LIBRARY: &[(&str, &str)] = &[
    ("core", include_str!("../../../../stdlib/src/core.musa")),
    ("collections", include_str!("../../../../stdlib/src/collections.musa")),
    ("list", include_str!("../../../../stdlib/src/list.musa")),
    ("option", include_str!("../../../../stdlib/src/option.musa")),
    ("pitch", include_str!("../../../../stdlib/src/pitch.musa")),
    ("scale", include_str!("../../../../stdlib/src/scale.musa")),
    ("harmony", include_str!("../../../../stdlib/src/harmony.musa")),
    ("voicing", include_str!("../../../../stdlib/src/voicing.musa")),
    ("context", include_str!("../../../../stdlib/src/context.musa")),
    (
        "transformational",
        include_str!("../../../../stdlib/src/transformational.musa"),
    ),
    (
        "post_tonal/pcset",
        include_str!("../../../../stdlib/src/post_tonal/pcset.musa"),
    ),
    (
        "post_tonal/serial",
        include_str!("../../../../stdlib/src/post_tonal/serial.musa"),
    ),
    (
        "tonal/harmony",
        include_str!("../../../../stdlib/src/tonal/harmony.musa"),
    ),
    (
        "tonal/sequences",
        include_str!("../../../../stdlib/src/tonal/sequences.musa"),
    ),
    (
        "tonal/schemas",
        include_str!("../../../../stdlib/src/tonal/schemas.musa"),
    ),
    (
        "notation/staff",
        include_str!("../../../../stdlib/src/notation/staff.musa"),
    ),
];

/// The whole standard library, elaborated as one document.
///
/// One fault, and it is a row in prompt 142's Target rather than a defect here:
/// **`Music`**, the contextual type 142 deletes. `list.musa` and `voicing.musa`
/// write it in a signature, and after the migration a fragment is an
/// `EventTrack ⟨written⟩` and a motif is a function to one, which is what
/// `00-semantics.md` §3 already says.
///
/// The anonymous product was the second, until 142 found that only *half* of it
/// was missing: `(a, b)` had lowered here since 141g and it was the type
/// `(A, B)` alone that refused, which is one construct disagreeing with itself
/// rather than a stage. Both halves now read `Pair`, the family 141ha declared
/// for the machine calculus's wiring — a written product is a constructor
/// application and therefore canonical data, which a structural record is not.
///
/// Everything else the thirteen prompts built holds on the standard library's
/// real Musa: every `data` declaration, every generic signature, the qualified
/// paths 141l reads, and the notation vocabulary 141j and 141k registered.
#[test]
fn the_standard_library_elaborates() {
    let (_, said) = faults(&library_sources());
    assert_eq!(
        said,
        ["UnknownName: no binder named `Music` is in scope"],
        "the standard library needs exactly what 142 already owns"
    );
}

/// `stdlib/src/adapters/doubled.musa`, elaborated in phase scope.
///
/// One fault: something the reading cannot give a type on its own. 142's
/// migration writes the annotation, which is the same repair `01-surface.md` §1
/// asks for anywhere else a declaration is left open.
#[test]
fn the_doubled_adapter_elaborates() {
    let (_, said) = adapter(include_str!("../../../../stdlib/src/adapters/doubled.musa"));
    assert_eq!(
        said,
        ["UnsolvedMetavariable: this cannot be given a type on its own; write the type it should have"],
        "the doubled adapter needs exactly what 142 already owns"
    );
}

/// `stdlib/src/adapters/staff.musa`, elaborated in phase scope.
///
/// One reason, and it is 142's: a conversion mismatch where the file branches on
/// a rule 141m left answering `Result τ Text` — one of the twenty sites 141m's
/// survey table lists and 142's Target moves onto the refusal channel.
///
/// This file is prompt 145's benchmark and 142's Stop forbids rewriting it, so
/// the reason here is the one a *migration* has to answer and not the ones a
/// rewrite would.
#[test]
fn the_staff_adapter_elaborates() {
    let (_, said) = adapter(include_str!("../../../../stdlib/src/adapters/staff.musa"));
    assert_eq!(
        said,
        ["ConversionMismatch: type mismatch"],
        "the staff adapter needs exactly what 142 already owns"
    );
}

/// One adapter, elaborated alone in the scope `02-core-calculus.md` §5.9 gives
/// a phase.
///
/// Alone, and that is the arrangement rather than a shortcut: an adapter module
/// is checked as its own document, which is the whole of what the replaced
/// checker's `Reading::Expansion` meant.
fn adapter(source: &str) -> (Option<Document>, Vec<String>) {
    let held = musa_language::parse(source);
    assert!(held.errors().is_empty(), "the adapter parses: {:?}", held.errors());
    faults(&[Source {
        root: written_library(&held.syntax()).expect("an adapter writes a library"),
        in_phase: true,
    }])
}
