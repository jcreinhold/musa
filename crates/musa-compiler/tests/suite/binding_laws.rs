//! Naming a value inside a block.
//!
//! `docs/rules/language/01-surface.md` §1 admits `let name = value; body` as
//! one expression, and `02-core-calculus.md` §2 has had the term it elaborates
//! to since the beginning — §9.1's path update has been written in terms of it
//! for as long as there has been a path update. So what is checked here is a
//! *spelling*: that it means substitution, that several of them nest
//! rightward, that an inner one shadows an outer binder rather than colliding
//! with it, and that a value naming its own binder is refused rather than
//! quietly meaning something else.
//!
//! The block's own rule is unchanged and is checked here too: `{ e1; e2 }` is
//! still the static error §1 names, because a `let` is not a second
//! expression.

#![allow(clippy::expect_used, clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ScoreEventKind;

fn piece(body: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!(
            "piece \"Binding\" {{ {body} \
             let tune: EventTrack<WrittenTime> = transpose(chosen, music {{ c4/1 }}); \
             tempo 1/4 = 84; meter 4/4; \
             score {{ part p {{ voice v {{ use tune; }} }} }} }}"
        ),
        "binding.musa",
    );
    compile(&source, &CompileOptions::default())
}

/// The one note that piece sounds: `P1` leaves `c4` alone and `P8` takes it to
/// `c5`, so the note is which value the binding carried.
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

/// `let x = v; e` means `e` with `v` written where `x` stands.
///
/// Both spellings of the binding — annotated and inferred — against the body
/// with the value substituted into it by hand.
#[test]
fn a_binding_means_the_body_with_the_value_substituted() {
    let substituted = sounded("fn shifted() -> Interval { P8 } let chosen: Interval = shifted();");
    for binding in ["let held = P8; held", "let held: Interval = P8; held"] {
        let bound = sounded(&format!(
            "fn shifted() -> Interval {{ {binding} }} let chosen: Interval = shifted();"
        ));
        assert_eq!(bound, substituted, "`{binding}` did not mean its substitution");
        assert_eq!(bound, "c5", "`{binding}` carried the wrong value");
    }
}

/// A second binding is written in the scope the first opened, and the body is
/// written in both.
///
/// `let a = …; let b = …; e` is one `let` inside another, so `b`'s value may
/// name `a` — which is the whole of what "nesting rightward" buys, and is
/// checked by making `b` depend on `a` rather than by writing two independent
/// bindings that any order would satisfy.
#[test]
fn a_second_binding_is_written_under_the_first() {
    let sounding = sounded(
        "fn shifted() -> Interval { let first = P8; let second = first; second } \
         let chosen: Interval = shifted();",
    );
    assert_eq!(sounding, "c5", "the second binding did not see the first");
}

/// An inner binding shadows an outer binder of the same name.
///
/// The parameter is `P1` and the binding is `P8`, so the note says which one
/// the body read. Shadowing is the reason a self-naming `let` cannot simply be
/// refused by spelling: where the name *is* in scope, naming it in the value
/// means the outer one, and that is a reading this law depends on.
#[test]
fn an_inner_binding_shadows_an_outer_binder() {
    let sounding = sounded(
        "fn shifted(shift: Interval) -> Interval { let shift = P8; shift } \
         let chosen: Interval = shifted(P1);",
    );
    assert_eq!(sounding, "c5", "the body read the outer binder");
}

/// A value that names the binder it is being bound to is refused.
///
/// Not `unknown-name`: nothing else in scope carries the name, so the only
/// thing the author can have meant is a recursion a `let` does not have. The
/// code is read rather than "it did not compile", because an unknown name
/// would also fail to compile and would be the wrong sentence.
#[test]
fn a_binding_that_names_itself_is_refused() {
    let compilation = piece("fn shifted() -> Interval { let held = held; held } let chosen: Interval = shifted();");
    let refusal = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == musa_score::Code::RecursiveBinding)
        .unwrap_or_else(|| panic!("the self-naming binding was not refused: {:?}", compilation.diagnostics()));
    assert!(
        refusal.message.contains("held"),
        "the refusal names the binder: {}",
        refusal.message
    );
}

/// A match arm's result is an expression, so a binding stands there too.
#[test]
fn a_match_arm_names_a_value() {
    let sounding = sounded(
        "data Clef { Treble, Bass } \
         let written: Clef = Bass; \
         let chosen: Interval = match written { Bass -> let held = P8; held, Treble -> P1 };",
    );
    assert_eq!(sounding, "c5", "the arm's binding did not carry its value");
}

/// The block's own rule survives: two expressions is still the error §1 names,
/// because a `let` is not a second expression.
#[test]
fn two_expressions_in_a_block_are_still_refused() {
    let compilation = piece("fn twice() -> Interval { P8; P8 } let chosen: Interval = twice();");
    let refusal = compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == musa_score::Code::Syntax)
        .unwrap_or_else(|| panic!("the sequence was not refused: {:?}", compilation.diagnostics()));
    assert!(
        refusal.message.contains("one expression"),
        "the refusal is still §1's own: {}",
        refusal.message
    );
}
