//! Argument order is not semantically significant for an un-annotated lambda.
//!
//! `02-core-calculus.md` §2.1 walks an application spine left to right, and an
//! argument whose domain still mentions an unsolved hole used to be checked
//! against that hole — which, for a bare `fn (x) { … }`, meant binding `x` to
//! it and refusing whatever the body then asked of `x`. The library was shaped
//! around the weakness: a fold takes its seed before its combining function so
//! that the accumulator's type is decided before the lambda is reached.
//!
//! The rule now defers such an argument to the end of the spine — Idris2's
//! `checkRtoL` without its fallback, since Musa has no ambiguity in a spine to
//! fall back from. These are the laws of what that buys and what it does not:
//! a lambda types the same in every slot, the *emitted* order is still the
//! written one, and a parameter nothing determines is still refused rather
//! than guessed.

// A failure is more useful reported with what actually happened.
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{Code, ScoreEventKind};

/// The declarations, inside a piece that sounds `sounded`.
fn piece(declarations: &str, sounded: &str) -> String {
    format!(
        "piece \"Argument order\" {{ import std::core; import std::list; import std::pitch; {declarations} \
         score {{ part p {{ voice v {{ {sounded} }} }} }} }}"
    )
}

fn compile_core(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(piece(declarations, "c4/1"), "argument-order-laws.musa"),
        &CompileOptions::default(),
    )
}

fn errors(declarations: &str) -> Vec<(Code, String)> {
    compile_core(declarations)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// What actually sounded, so a law about *order* can observe order.
fn sounding(declarations: &str, sounded: &str) -> Vec<String> {
    let compilation = compile(
        &SourceDocument::new(piece(declarations, sounded), "argument-order-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    compilation
        .into_snapshot()
        .map(|snapshot| {
            snapshot
                .parts()
                .iter()
                .flat_map(|(_, part)| part.voices())
                .flat_map(|(_, voice)| voice.events().to_vec())
                .map(|event| match event.kind {
                    ScoreEventKind::Note { pitch } => pitch.to_string(),
                    ScoreEventKind::Rest => "rest".to_owned(),
                    ScoreEventKind::Chord { pitches } => format!("chord:{pitches:?}"),
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

/// A generic application whose function argument comes *first*, which is the
/// order the workaround forbade.
const APPLIED: &str = "fn applied({A: Type}, {B: Type}, by: A -> B, value: A) -> B { by(value) }";

/// The whole point: the lambda stands before the argument that says what its
/// parameter is, and `p` is a `Pitch` anyway.
///
/// `p.act(P8)` is the observation that makes this a real law rather
/// than a compile check — `10-traits.md` §6 resolves a method by its *exact*
/// receiver, so a `p` still standing at a hole has no method at all.
#[test]
fn an_un_annotated_lambda_is_typed_by_an_argument_written_after_it() {
    let compilation = compile_core(&format!(
        "{APPLIED} let raised: Pitch = applied(fn (p) {{ p.act(P8) }}, c4);"
    ));
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// The same lambda in the other slot, so the law is about the lambda and not
/// about a position that happens to work.
#[test]
fn the_same_lambda_types_in_either_slot() {
    let compilation = compile_core(
        "fn onto({A: Type}, {B: Type}, value: A, by: A -> B) -> B { by(value) } \
         let raised: Pitch = onto(c4, fn (p) { p.act(P8) });",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// Three parameters, the lambda in each of them in turn. A rule that only
/// looked one argument ahead would pass the last of these and fail the first.
#[test]
fn a_lambda_types_in_every_slot_of_a_three_argument_call() {
    let both = "fn both({A: Type}, {B: Type}, by: A -> B, first: A, second: A) -> B { by(second) }";
    let middle = "fn middle({A: Type}, {B: Type}, first: A, by: A -> B, second: A) -> B { by(second) }";
    let last = "fn last({A: Type}, {B: Type}, first: A, second: A, by: A -> B) -> B { by(second) }";
    for (declaration, call) in [
        (both, "both(fn (p) { p.act(P8) }, c4, e4)"),
        (middle, "middle(c4, fn (p) { p.act(P8) }, e4)"),
        (last, "last(c4, e4, fn (p) { p.act(P8) })"),
    ] {
        let compilation = compile_core(&format!("{declaration} let raised: Pitch = {call};"));
        assert!(!compilation.has_errors(), "{call}: {:?}", compilation.diagnostics());
    }
}

/// Two lambdas in one spine, neither of which the other could type.
#[test]
fn two_deferred_arguments_are_both_typed_by_the_one_that_is_not() {
    let compilation = compile_core(
        "fn twice({A: Type}, first: A -> A, second: A -> A, value: A) -> A { second(first(value)) } \
         let raised: Pitch = twice(fn (p) { p.act(P8) }, fn (q) { q.act(P8) }, c4);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// The *expected* type is part of "the rest of the spine": it is matched
/// before the deferred arguments are checked, so a call whose only argument is
/// the lambda still knows what the lambda is.
#[test]
fn the_position_a_call_stands_in_types_a_lambda_no_argument_could() {
    let compilation = compile_core(
        "fn kept({A: Type}, by: A -> A) -> A -> A { by } \
         let raised: Pitch -> Pitch = kept(fn (p) { p.act(P8) });",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// Deferral is not defaulting. Nothing in this program says what `A` is, and
/// §2.1's refusal is what the author gets — the rule buys information that was
/// already written down, and invents none.
#[test]
fn a_parameter_nothing_determines_is_still_refused() {
    let reported = errors(
        "fn kept({A: Type}, by: A -> A) -> A -> A { by } \
         let raised = kept(fn (p) { p });",
    );
    assert!(!reported.is_empty(), "nothing was refused");
}

/// Arguments are elaborated in one order and *evaluated* in the written one.
///
/// `first` and `second` are applied in the order the body writes, and the
/// notes come out in that order whether or not the spine deferred anything.
#[test]
fn deferring_an_argument_does_not_reorder_what_the_call_does() {
    let sounded = sounding(
        "fn line({A: Type}, by: A -> A, from: A) -> A { by(from) } \
         let subject: EventTrack(WrittenTime) = music { c4/4 e4/4 };",
        "use line(retrograde, subject);",
    );
    assert_eq!(sounded, ["e4", "c4"]);
}

/// The library's fold no longer has to put its seed first to be usable, which
/// is the ergonomic claim this whole rule was made for.
#[test]
fn a_fold_may_take_its_combining_function_first() {
    let compilation = compile_core(
        "fn folded({A: Type}, {B: Type}, combine: B -> A -> B, seed: B, values: List(A)) -> B { \
             values.fold_from_start(seed, combine) \
         } \
         let total: Nat = folded(fn (carried, each) { carried }, 0, range(4));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// `compose` is `core.musa`'s, and it is the shape the whole prompt was for:
/// three distinct types in one signature, a function-valued result, and the
/// composed function named rather than applied.
#[test]
fn compose_names_a_function_at_three_types() {
    let compilation = compile_core(
        "fn width(values: List(Nat)) -> Nat { 0 } \
         fn note_at(n: Nat) -> Pitch { c4 } \
         let sounded: List(Nat) -> Pitch = compose(note_at, width); \
         let raised: Pitch = sounded(range(4));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// The same composition with its first argument un-annotated. Only the
/// *second* argument says what the middle type is, so this is the reordering
/// working inside the library's own function.
#[test]
fn compose_types_an_un_annotated_lambda_from_the_argument_after_it() {
    let compilation = compile_core(
        "fn width(values: List(Nat)) -> Nat { 0 } \
         fn note_at(n: Nat) -> Pitch { c4 } \
         let sounded: List(Nat) -> Pitch = compose(fn (n) { note_at(n) }, width); \
         let raised: Pitch = sounded(range(4));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// And what it does *not* buy, said as a law so nobody reads the rule as
/// guessing: `compose` at `Pitch -> Pitch` leaves the middle type open, and two
/// bare lambdas leave it open too. The refusal is §2.1's, and the repair is an
/// annotation on either lambda.
#[test]
fn a_middle_type_no_argument_names_is_refused_rather_than_guessed() {
    let reported = errors("let higher: Pitch -> Pitch = compose(fn (p) { p.act(P8) }, fn (q) { q.act(P8) });");
    assert!(!reported.is_empty(), "nothing was refused");
    let repaired = compile_core(
        "let higher: Pitch -> Pitch = compose(fn (p) { p.act(P8) }, fn (q: Pitch) -> Pitch { q.act(P8) });",
    );
    assert!(!repaired.has_errors(), "{:?}", repaired.diagnostics());
}
