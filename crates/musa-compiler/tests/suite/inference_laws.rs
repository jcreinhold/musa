//! Laws for source-level type inference.
//!
//! The unifier's own laws are tested beside it, in `src/infer.rs`, because a
//! substitution is private to the compiler. These are the laws a *writer* can
//! observe: what may be left unwritten, what one declaration may mean at two
//! uses, and what happens where the program genuinely does not decide.

#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

use musa_compiler::{
    CompileOptions, MusicalDuration, MusicalTime, ScoreEvent, ScoreEventKind, ScoreSnapshot, SourceDocument, compile,
};

fn compile_text(source: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, "inference.musa"),
        &CompileOptions::default(),
    )
}

fn snapshot(source: &str) -> ScoreSnapshot {
    let compilation = compile_text(source);
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation.into_snapshot().expect("an inferred piece has a score")
}

fn shape(events: &[ScoreEvent]) -> Vec<(MusicalTime, MusicalDuration, ScoreEventKind)> {
    events
        .iter()
        .map(|event| (event.onset, event.notated_duration.value, event.kind.clone()))
        .collect()
}

fn voices(score: &ScoreSnapshot) -> Vec<Vec<ScoreEvent>> {
    score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| voice.events().to_vec())
        .collect()
}

/// The principal type, stated as a writer meets it: one unannotated `unchanged`
/// serves a use at `Music -> Music` and a use at `Pitch -> Pitch` in the same
/// piece. A checker that inferred *a* type rather than the *principal* one
/// would fix the first use's type onto the declaration and reject the second.
#[test]
fn one_unannotated_declaration_serves_two_types() {
    let score = snapshot(
        "piece \"principal\" {
            let subject: Music = music { c4/4 d4/4 };
            fn unchanged(value) { value }
            score { part p {
                voice direct { use subject; }
                voice music_use { use unchanged(subject); }
                voice pitch_use { use map_note_pitches(unchanged, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(
        shape(&lanes[0]),
        shape(&lanes[1]),
        "unchanged at `Music` changes nothing"
    );
    assert_eq!(
        shape(&lanes[0]),
        shape(&lanes[2]),
        "unchanged at `Pitch` changes nothing"
    );
}

/// A function whose parameter is itself a function needs no annotation either:
/// applying `f` twice says `f` takes and returns one type, and that the value
/// has it. Written out, `twice` is `(a -> a, a) -> a`.
#[test]
fn a_higher_order_parameter_needs_no_annotation() {
    let score = snapshot(
        "piece \"higher order\" {
            let subject: Music = music { c4/4 d4/4 };
            fn twice(f, value) { f(f(value)) }
            fn unchanged(value) { value }
            score { part p {
                voice direct { use subject; }
                voice twice_over { use twice(unchanged, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
}

/// Inference reaches through a chain of unannotated declarations: `held` is a
/// `Pitch` because `unchanged` returns what it is given, and nothing in either
/// declaration says the word.
#[test]
fn an_inferred_type_travels_between_declarations() {
    let compilation = compile_text(
        "piece \"chained\" {
            fn unchanged(value) { value }
            let held = unchanged(c4);
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let item = compilation
        .items()
        .iter()
        .find(|item| item.name == "held")
        .expect("the piece documents its `let`");
    assert_eq!(item.signature, "let held: Pitch");
}

/// What a reader is shown for a declaration that wrote no type: the type it
/// has, spelled the way an annotation spells it, with the quantified variable
/// given a plain name rather than the unifier's number.
#[test]
fn hover_shows_an_inferred_signature_in_written_spelling() {
    let compilation = compile_text(
        "piece \"hover\" {
            fn unchanged(value) { value }
            let held: Pitch = unchanged(c4);
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let item = compilation
        .items()
        .iter()
        .find(|item| item.name == "unchanged")
        .expect("the piece documents its `fn`");
    assert_eq!(item.signature, "fn unchanged(value: a) -> a");
    assert_eq!(item.result.as_ref().map(|result| result.name.as_str()), Some("a"));
    assert_eq!(item.parameters[0].ty.name, "a");
}

/// Where the program really does not decide, inference says so at the place
/// that is undecided rather than choosing a default. `None` alone holds
/// nothing in particular, and no use fixes it.
#[test]
fn an_undetermined_type_is_a_located_error() {
    let compilation = compile_text(
        "piece \"ambiguous\" {
            fn nothing() { None }
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(compilation.has_errors(), "an undetermined type is an error");
    let reported = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message == "the program does not say what this holds")
        .expect("the undetermined type is reported");
    assert!(
        reported.labels.iter().any(|label| label.text.contains("Option<")),
        "the label names the type that is still open: {reported:?}"
    );
    assert!(
        reported
            .help
            .as_deref()
            .is_some_and(|help| help.contains("annotate the declaration")),
        "the error says what to write"
    );
}

/// An annotation is still accepted everywhere it was accepted before, and
/// still constrains: writing a type inference would not have chosen is an
/// error, not a silent widening.
#[test]
fn an_annotation_still_decides_against_inference() {
    let compilation = compile_text(
        "piece \"annotated\" {
            fn unchanged(value) { value }
            let held: Nat = unchanged(c4);
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(compilation.has_errors(), "`Pitch` is not `Nat`");
}
