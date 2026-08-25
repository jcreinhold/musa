//! What a record literal names, and what the name buys.
//!
//! `01-surface.md` §1.2 says parameters "are allowed and are ordinary" and that
//! a literal elaborates to the family's one constructor applied to its fields.
//! Both halves were true of the *declaration* and neither was true of the
//! *term*: the head rode in an annotation, an annotation is elaborated as a
//! type, and `Cell : Type 0 → Type 0` is not one. So every parameterized
//! `record` in the library was a declaration nothing could construct.
//!
//! The head rides on the literal now. With a goal it says which family the
//! literal claims and is checked against the one the goal names; with no goal
//! it *is* the family, and the parameters are metavariables the fields solve.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::Severity;

fn compiled(source: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, "record-literal.musa"),
        &CompileOptions::default(),
    )
}

fn errors(source: &str) -> Vec<String> {
    compiled(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// A library holding `declarations`, with nothing else in it.
fn library(declarations: &str) -> String {
    format!(" {declarations} ")
}

/// The defect, gone: a parameterized record is declared, constructed against
/// its type, and projected out of.
#[test]
fn a_parameterized_record_is_constructed_and_projected() {
    let refused = errors(&library(
        "record Cell(A: Type) { spot: Nat; value: A; } \
         let one: Cell(Nat) = Cell { spot = 0, value = 1 }; \
         let held: Nat = one.value;",
    ));
    assert!(refused.is_empty(), "{refused:?}");
}

/// And with no expected type at all. The head names the family, the family's
/// parameter is a metavariable, and the second field solves it — which is the
/// half an annotation could never have done, because an annotation says the
/// answer rather than asking for it.
#[test]
fn a_headed_literal_solves_its_parameters_from_its_fields() {
    let refused = errors(&library(
        "record Cell(A: Type) { spot: Nat; value: A; } \
         let one = Cell { spot = 0, value = 1 }; \
         let held: Nat = one.value;",
    ));
    assert!(refused.is_empty(), "{refused:?}");
}

/// A parameter no field mentions is an unsolved metavariable, refused where
/// every other unsolved one is rather than guessed at.
#[test]
fn a_parameter_no_field_mentions_is_not_guessed() {
    let refused = errors(&library(
        "record Tagged(A: Type) { spot: Nat; } let one = Tagged { spot = 0 };",
    ));
    assert!(!refused.is_empty(), "an unsolved parameter must be refused");
}

/// Two names for one value, and they disagree. Before the head rode on the
/// term this came out of the annotation as a conversion mismatch; dropping the
/// annotation without saying anything would have answered a real mistake with
/// silence.
#[test]
fn the_literal_and_its_goal_must_name_one_family() {
    let refused = errors(&library(
        "record Cell(A: Type) { spot: Nat; value: A; } \
         record Other(A: Type) { spot: Nat; value: A; } \
         let one: Cell(Nat) = Other { spot = 0, value = 1 };",
    ));
    assert_eq!(
        refused,
        vec!["this record literal names `Other`, but `Cell` is what stands here".to_owned()]
    );
}

/// A record *pattern* is the same question with an answer of its own: the
/// scrutinee's type supplies the family through §6.2's case tree, and nothing
/// here changed it.
#[test]
fn a_record_pattern_still_finds_its_family_from_the_subject() {
    let refused = errors(&library(
        "record Cell(A: Type) { spot: Nat; value: A; } \
         fn spot_of(cell: Cell(Nat)) -> Nat { match cell { Cell(spot, value) -> spot, } } \
         let one: Nat = spot_of(Cell { spot = 3, value = 1 });",
    ));
    assert!(refused.is_empty(), "{refused:?}");
}

/// The tempting fix — insert metas in `check_type` until a Π reaches a
/// universe — would have turned this deliberate diagnostic into a silent solve
/// for the coordinate. The parameters this rule solves are solved at a *term*,
/// from its fields, and nowhere else.
#[test]
fn a_family_that_takes_an_argument_still_says_so() {
    let refused = errors(&library("let held: Duration = duration_of(1/4);"));
    assert!(
        refused.iter().any(|message| message.contains("Duration")),
        "{refused:?}"
    );
}

/// The three structures `std::algebra` declares, now inhabited at written
/// pitch — and inhabited *usefully*, so the law is a rendered voice rather
/// than a compilation that did not complain.
///
/// Two voices, note for note: one built by asking the records, one written
/// out. `compose(M3, m3)` is a fifth, the action carries `c4` to `e4`, the
/// torsor's difference of `c4` and `g4` carries `c4` to `g4`, and the unit
/// moves nothing.
#[test]
fn the_structures_at_written_pitch_are_values_that_compute() {
    let compilation = compiled(
        "piece \"Algebra\" { \
         import std::algebra; \
         import std::pitch; \
         let fifth: Interval = (interval_group.compose)(M3, m3); \
         let nothing: Interval = (interval_group.unit)(P5); \
         let moved: Pitch = (pitch_action.act)(c4, M3); \
         let apart: Interval = (pitch_torsor.difference)(c4, g4); \
         fn by_record() -> EventTrack(WrittenTime) { \
             music { (c4 up fifth)/4 moved/4 (c4 up apart)/4 (c4 up nothing)/4 } \
         } \
         fn by_hand() -> EventTrack(WrittenTime) { music { g4/4 e4/4 g4/4 c4/4 } } \
         score { part p { voice a { use by_record(); } voice b { use by_hand(); } } } }",
    );
    let errors: Vec<&musa_score::Diagnostic> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let score = compilation.snapshot().expect("a score");
    let voices: Vec<Vec<musa_score::ScoreEventKind>> = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| voice.events().iter().map(|event| event.kind.clone()).collect())
        .collect();
    let [ref by_record, ref by_hand] = voices[..] else {
        panic!("two voices, found {}", voices.len());
    };
    assert_eq!(by_record, by_hand);
}
