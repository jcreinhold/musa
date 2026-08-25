//! One arm answering for several constructors.
//!
//! `docs/rules/language/01-surface.md` §1 admits `p | q` at every position a
//! pattern stands, and `02-core-calculus.md` §6.2 compiles it by pointing
//! several branches of the case tree at one arm. Two things follow and are
//! checked here. The alternation must *answer* what the arms it replaces
//! answered — for every constructor, not only the first — and coverage must
//! count an alternation as the constructors it names. That it adds nothing to
//! the core is stated where the core is: `musa-calculus`'s `coverage_laws.rs`
//! compares the two elaborated terms.
//!
//! The binding condition is the third. It is Peyton Jones ch. 5's, and it is
//! refused before the tree is built, so the law reads the code and the name
//! rather than whatever the body's own checking would have said.

#![allow(clippy::expect_used, clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ScoreEventKind;

/// The declarations every law here matches against.
const DECLARATIONS: &str = "data Clef { Treble, Bass, Alto, Tenor } \
                            data Inner { Quiet, Loud(count: Nat), Soft(said: Text) } \
                            data Outer { Wrap(held: Inner), Hold(held: Inner) }";

fn piece(body: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!(
            "piece \"Alternation\" {{ {DECLARATIONS} {body} \
             let tune: EventTrack(WrittenTime) = transpose(chosen, music {{ c4/1 }}); \
             tempo 1/4 = 84; meter 4/4; \
             score {{ part p {{ voice v {{ use tune; }} }} }} }}"
        ),
        "alternation.musa",
    );
    compile(&source, &CompileOptions::default())
}

/// The one note that piece sounds, as it is spelled: `P1` leaves `c4` alone
/// and `P8` takes it to `c5`, so the note is which arm ran.
fn sounded(body: &str) -> String {
    let compilation = piece(body);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let event = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .next()
        .expect("the one note");
    match event.kind {
        ScoreEventKind::Note { pitch, .. } => pitch.to_string(),
        other @ (ScoreEventKind::Rest | ScoreEventKind::Chord { .. }) => {
            panic!("a voice holding one note holds a note: {other:?}")
        }
    }
}

/// Four clefs, two answers, one arm each way — and the answer for every clef
/// is the answer the four separate arms gave. Every constructor is run,
/// because an alternation that only ever selected its first alternative would
/// satisfy a law that tried one.
#[test]
fn an_alternation_answers_what_the_several_arms_answered() {
    for (clef, expected) in [("Treble", "c4"), ("Bass", "c5"), ("Alto", "c4"), ("Tenor", "c5")] {
        let alternated = sounded(&format!(
            "let written: Clef = {clef}; \
             let chosen: Interval = match written {{ Bass | Tenor -> P8, Treble | Alto -> P1 }};"
        ));
        let separately = sounded(&format!(
            "let written: Clef = {clef}; \
             let chosen: Interval = \
                 match written {{ Bass -> P8, Tenor -> P8, Treble -> P1, Alto -> P1 }};"
        ));
        assert_eq!(alternated, expected, "the alternation, at {clef}");
        assert_eq!(separately, alternated, "the two readings, at {clef}");
    }
}

/// Peyton Jones ch. 5's condition, refused where it is written.
///
/// Both directions: the alternative that lacks a name its neighbour binds, and
/// the neighbour that lacks a name it binds. The code and the name are read,
/// because "it did not compile" would also be true of a body that failed to
/// check against a type it could not have.
#[test]
fn alternatives_that_bind_different_names_are_refused() {
    for arm in [
        "match held { Loud(count) | Quiet -> 0, Soft(said) -> 1 }",
        "match held { Quiet | Loud(count) -> 0, Soft(said) -> 1 }",
    ] {
        let compilation = piece(&format!(
            "let held: Inner = Quiet; \
             let which: Nat = {arm}; \
             let chosen: Interval = P1;"
        ));
        let refusal = compilation
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code == musa_score::Code::AlternativeBindings)
            .unwrap_or_else(|| panic!("`{arm}` was not refused: {:?}", compilation.diagnostics()));
        assert!(
            refusal.message.contains("count"),
            "the refusal names the name: {}",
            refusal.message
        );
    }
}

/// A name bound at two types is refused where the body cannot be given both.
///
/// The condition §6.2 states about types, and the place it lands: the body is
/// checked once per branch the alternation reaches, so `Loud(n) | Soft(n)` is
/// admitted as far as names go and refused at the arm that reads `n` as a
/// `Nat` it is not.
#[test]
fn a_name_bound_at_two_types_is_refused_where_the_body_reads_it() {
    let compilation = piece(
        "let held: Inner = Quiet; \
         let which: Nat = match held { Loud(n) | Soft(n) -> n, Quiet -> 0 }; \
         let chosen: Interval = P1;",
    );
    assert!(
        compilation.has_errors(),
        "a name bound at `Nat` and at `Text` was admitted"
    );
}

/// An alternation over every constructor of a family is exhaustive, and one
/// that misses a constructor is not — the same coverage rule, unmoved.
#[test]
fn coverage_counts_an_alternation_as_the_constructors_it_names() {
    let whole = piece(
        "let held: Inner = Quiet; \
         let which: Nat = match held { Quiet | Loud(_) | Soft(_) -> 7 }; \
         let chosen: Interval = P1;",
    );
    assert!(!whole.has_errors(), "{:?}", whole.diagnostics());

    let partial = piece(
        "let held: Inner = Quiet; \
         let which: Nat = match held { Quiet | Loud(_) -> 7 }; \
         let chosen: Interval = P1;",
    );
    let refusal = partial
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == musa_score::Code::IncompleteMatch)
        .expect("the incomplete-match refusal");
    assert!(
        refusal.message.contains("Inner.Soft"),
        "the refusal names the constructor with no arm: {}",
        refusal.message
    );
}

/// An alternation nests, in both directions: inside a constructor's
/// sub-position, and around patterns that are themselves nested.
#[test]
fn a_nested_alternation_parses_and_answers() {
    for (subject, expected) in [
        ("Wrap(Loud(1))", "c5"),
        ("Wrap(Soft(\"p\"))", "c5"),
        ("Wrap(Quiet)", "c4"),
        ("Hold(Quiet)", "c4"),
    ] {
        let answer = sounded(&format!(
            "let subject: Outer = {subject}; \
             let chosen: Interval = match subject {{ \
                 Wrap(Loud(_) | Soft(_)) -> P8, Wrap(Quiet) | Hold(_) -> P1, \
             }};"
        ));
        assert_eq!(answer, expected, "the nested alternation, on {subject}");
    }
}
