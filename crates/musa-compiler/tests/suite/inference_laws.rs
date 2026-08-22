//! Laws for source-level type inference.
//!
//! These are the laws a *writer* can observe: what may be left unwritten, what
//! one declaration may mean at two uses, and what happens where the program
//! genuinely does not decide.
//!
//! # What the dependent core changed, and why these laws moved with it
//!
//! They used to be Hindley–Milner's laws. A `fn` wrote no types at all, the
//! unifier found the *principal* one, and the declaration was generalized so
//! that two uses at two types both went through. `02-core-calculus.md` §2.1 says
//! the opposite in one sentence — elaboration "never defaults and never
//! generalizes" — because a dependent core has no generalization to do: a type
//! may mention a value, so there is no prefix of quantifiers to float out, and a
//! metavariable still unsolved when a declaration ends is a hole the author left
//! rather than a variable the checker may bind.
//!
//! What replaces it is written polymorphism and bidirectional checking, which
//! between them keep every observable claim below except the one that was really
//! a claim about generalization. `fn unchanged<A>(value: A) -> A` serves two
//! uses at two types, because that is what its type says; `let held =
//! unchanged(c4)` still needs no annotation, because the call determines it; and
//! `fn unchanged(value) { value }` — the same declaration with the type
//! parameter *not* written — is now a located refusal naming the binder rather
//! than a generalization. Prompt 142's own Target records the same finding one
//! declaration wider, for `machine_laws`.

// A failure is more useful reported with what actually happened than with an
// assertion message alone.
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::{MusicalDuration, MusicalTime, ScoreEvent, ScoreEventKind, ScoreSnapshot};

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

/// One `unchanged` serves a use at `EventTrack<WrittenTime> ->
/// EventTrack<WrittenTime>` and a use at `Pitch -> Pitch` in the same piece,
/// because its type parameter is written and each call instantiates it.
///
/// The claim that survived generalization's removal. A checker that fixed the
/// first use's type onto the declaration would reject the second, and one that
/// needed a second declaration to serve the second use would make polymorphism
/// a copy-paste. What changed is only that `A` is a word the author writes.
#[test]
fn one_declaration_serves_two_types() {
    let score = snapshot(
        "piece \"principal\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn unchanged<A>(value: A) -> A { value }
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
        "unchanged at `EventTrack<WrittenTime>` changes nothing"
    );
    assert_eq!(
        shape(&lanes[0]),
        shape(&lanes[2]),
        "unchanged at `Pitch` changes nothing"
    );
}

/// A parameter may itself be a function, and the arrow is spelled in the
/// signature: `twice` is `(A -> A, A) -> A`, and applying `f` twice is what
/// makes the two `A`s the same one.
#[test]
fn a_higher_order_parameter_is_written_as_an_arrow() {
    let score = snapshot(
        "piece \"higher order\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn twice<A>(f: A -> A, value: A) -> A { f(f(value)) }
            fn unchanged<A>(value: A) -> A { value }
            score { part p {
                voice direct { use subject; }
                voice twice_over { use twice(unchanged, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
}

/// Inference still reaches through a chain of declarations: `held` writes no
/// type and is a `Pitch`, because `unchanged` returns what it is given and `c4`
/// is what it was given.
///
/// This is the half of the old law that was never generalization. A `let`'s type
/// comes from its value, which is inference in the direction §2 keeps — "a
/// projection, a variable, and a literal infer" — and nothing here is left for a
/// metavariable to hold open.
#[test]
fn an_inferred_type_travels_between_declarations() {
    let compilation = compile_text(
        "piece \"chained\" {
            fn unchanged<A>(value: A) -> A { value }
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

/// What a reader is shown for a declaration, spelled the way an annotation
/// spells it — the type parameter under the name its author gave it rather than
/// a number from inside the checker.
#[test]
fn hover_shows_a_signature_in_written_spelling() {
    let compilation = compile_text(
        "piece \"hover\" {
            fn unchanged<A>(value: A) -> A { value }
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
    assert_eq!(item.signature, "fn unchanged<A>(value: A) -> A");
    assert_eq!(item.result.as_ref().map(|result| result.name.as_str()), Some("A"));
    assert_eq!(item.parameters[0].ty.name, "A");
}

/// Where the program really does not decide, elaboration says so at the place
/// that is undecided rather than choosing a default or binding a variable of its
/// own.
///
/// A parameter with no type is the one place a writer can leave that hole: the
/// body says nothing about `value`, no call is in scope to say it, and §2.1
/// neither defaults nor generalizes. The report is at the binder, which is where
/// the missing word would be written.
#[test]
fn a_parameter_with_no_type_is_a_located_error() {
    let compilation = compile_text(
        "piece \"ambiguous\" {
            fn unchanged(value) { value }
            let held = unchanged(c4);
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(compilation.has_errors(), "an undetermined binder type is an error");
    let reported = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic
                .message
                .contains("this cannot be given a type on its own; write the type it should have")
        })
        .unwrap_or_else(|| panic!("the undetermined binder is reported: {:?}", compilation.diagnostics()));
    assert!(
        reported.labels.iter().any(|label| label.text == "here"),
        "and it is reported at the binder that has no type: {reported:?}"
    );
}

/// An annotation is still accepted everywhere it was accepted before, and still
/// constrains: writing a type the value does not have is an error, not a silent
/// widening.
#[test]
fn an_annotation_still_decides_against_inference() {
    let compilation = compile_text(
        "piece \"annotated\" {
            fn unchanged<A>(value: A) -> A { value }
            let held: Nat = unchanged(c4);
            score { part p { voice v { c4/4 } } }
        }",
    );
    assert!(compilation.has_errors(), "`Pitch` is not `Nat`");
}

/// `01-surface.md` §1's braced type parameter, with no type written, is the
/// bare one: `<{A}>` and `<A>` declare the same inferred binder at `Type 0`.
///
/// Worth a law rather than a comment because the two spellings take different
/// parser paths, and a grammar that admitted the second without meaning the
/// first would be a second way to write a type parameter.
#[test]
fn a_braced_type_parameter_with_no_type_is_the_bare_one() {
    let score = snapshot(
        "piece \"braced\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn unchanged<{A}>(value: A) -> A { value }
            score { part p {
                voice direct { use subject; }
                voice braced { use unchanged(subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
}

/// The braces are what lets a parameter state its *type*, and that is the half
/// the bare form cannot say: `{n : Nat}` abstracts over a natural number rather
/// than over a type. §1's `type` has no universe in it, so `A` is the only
/// spelling for a type parameter and this is the only spelling for anything
/// else.
///
/// `n` is mentioned by nothing, so no argument determines it — which is exactly
/// why the call writes it, and what the next law measures.
#[test]
fn an_inferred_parameter_may_be_a_value_and_is_then_supplied_by_name() {
    let score = snapshot(
        "piece \"named\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn tagged<{n : Nat}, A>(value: A) -> A { value }
            score { part p {
                voice direct { use subject; }
                voice tagged_use { use tagged({n = 0}, subject); }
            } }
        }",
    );
    let lanes = voices(&score);
    assert_eq!(shape(&lanes[0]), shape(&lanes[1]));
}

/// And the same call without the name is refused, which is what makes the law
/// above about the name rather than about the declaration.
#[test]
fn a_parameter_nothing_determines_and_nothing_names_is_refused() {
    let compilation = compile_text(
        "piece \"unnamed\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn tagged<{n : Nat}, A>(value: A) -> A { value }
            score { part p { voice v { use tagged(subject); } } }
        }",
    );
    assert!(
        compilation.has_errors(),
        "a parameter nothing determines is reported, not defaulted"
    );
}

/// A name no inferred binder bears names nothing, and the report says which
/// names there are — the author is almost always remembering a signature.
#[test]
fn a_supplied_name_the_signature_does_not_bear_is_reported_with_the_ones_it_does() {
    let compilation = compile_text(
        "piece \"misnamed\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 };
            fn tagged<{n : Nat}, A>(value: A) -> A { value }
            score { part p { voice v { use tagged({m = 0}, subject); } } }
        }",
    );
    let reported = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.message.contains("has no type parameter named `m`"))
        .unwrap_or_else(|| panic!("the name is reported: {:?}", compilation.diagnostics()));
    assert!(
        reported.message.contains('n') && reported.message.contains('A'),
        "and the report lists the names the signature does bear: {reported:?}"
    );
}
