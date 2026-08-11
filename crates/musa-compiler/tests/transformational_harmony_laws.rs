//! What a neo-Riemannian transformation promises (OMT
//! `072-neo-riemannian-triadic-progressions.md`).
//!
//! P, L, and R are defined by which two tones they keep and where the third
//! one goes. That sentence is meaningless for a seventh chord, a suspension,
//! or a diminished triad, so the domain is not `chord_class` but the `triad`
//! refinement — and because the refinement is checked once, by `as_triad`,
//! every transformation below is total and none of them re-checks anything.
//!
//! Spelling is kept, which is the point and the cost. `P` on C major is C
//! minor with an `eb`, never a `d#`; and because spelling is kept, PLR chains
//! do not close. The hexatonic cycle `P L P L P L` returns a triad that sounds
//! like C major and is spelled from D, so a voicing from `c4` does not exist
//! for it. The finite-group statement is true only after `triad_classes`
//! forgets the spelling, and both halves are asserted here.
//!
//! Values become visible the way they do in `serial_laws`: a `nat` sounds as
//! that many overlaid notes, a `list` as one note per member, and a pitch
//! class as its number plus one, so that pitch class zero is still audible.
//! Spelling itself is read through `close_position`, which yields a voicing
//! only when the bass pitch's class is a member *as spelled*.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Code, CompileOptions, ScoreEventKind, ScoreSnapshot, Severity, SourceDocument, compile};

const NEO_RIEMANNIAN: &str = include_str!("../../../examples/neo-riemannian.musa");

/// A minor triad sounds one note through `quality`, a major triad two.
const MINOR: usize = 1;
/// See [`MINOR`].
const MAJOR: usize = 2;

/// The counting apparatus, and the two triads every fixture starts from.
const PRELUDE: &str = r"
    import std::harmony;
    import std::post_tonal::pcset;
    import std::transformational;
    import std::voicing;

    meter 4/4;

    fn tick(one: Music, carried: Music) -> Music { overlay(one, carried) }
    fn beat() -> Music { music { c4/1 } }
    fn tally(count: Nat) -> Music { list_fold(music { rest/1 }, tick, repeat(beat(), count)) }
    fn chorus(voices: List<Music>) -> Music { list_fold(music { rest/1 }, tick, voices) }
    fn beat_for_pc(member: Pc12) -> Music { beat() }
    fn beat_for_voicing(chosen: Voicing) -> Music { beat() }
    fn beat_for_triad(refined: Triad) -> Music { beat() }
    fn numbered(member: Pc12) -> Music { overlay(beat(), tally(number_of(member))) }
    fn quality(refined: Triad) -> Music { match is_major(refined) {
        true -> tally(2),
        false -> tally(1),
    } }

    let c_major: Option<Triad> = as_triad(chord c major);
    let c_minor: Option<Triad> = as_triad(chord c minor);
    let no_steps: List<Triad -> Triad> = [];
";

/// A piece whose one voice sounds `expression`.
fn probe(bindings: &str, expression: &str) -> String {
    format!(
        "piece \"Law\" {{\n{PRELUDE}\n{bindings}\n    score {{ part p {{ voice v {{ use {expression}; }} }} }}\n}}\n"
    )
}

fn compile_named(source: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(source, "test.musa"), &CompileOptions::default())
}

fn errors_of(source: &str) -> Vec<(Code, String)> {
    compile_named(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

fn snapshot_of(source: &str) -> ScoreSnapshot {
    let errors = errors_of(source);
    assert!(errors.is_empty(), "expected a clean compile, got {errors:?}");
    compile_named(source).into_snapshot().expect("compiles")
}

/// How many notes the probe's voice sounds — the value, read as music.
///
/// Simultaneous notes in one voice are one chord event, so what is counted is
/// pitches and not events.
fn counted(bindings: &str, expression: &str) -> usize {
    let snapshot = snapshot_of(&probe(bindings, expression));
    snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .map(|event| match event.kind {
            ScoreEventKind::Note { .. } => 1,
            ScoreEventKind::Chord { ref pitches } => pitches.len(),
            ScoreEventKind::Rest => 0,
        })
        .sum()
}

/// Sound one question about the triad `reached` names, starting from `start`.
///
/// `reached` is a `.musa` expression in a parameter called `refined`, and
/// `question` an expression in a parameter called `probed`: together they are
/// "apply this transformation, then ask this".
fn asked(start: &str, reached: &str, question: &str) -> usize {
    counted(
        &format!(
            "    fn reached(refined: Triad) -> Triad {{ {reached} }}
    fn question(probed: Triad) -> Music {{ {question} }}
    fn combined(refined: Triad) -> Music {{ question(reached(refined)) }}
    let sounded: Music = option_fold(music {{ rest/1 }}, combined, {start});"
        ),
        "sounded",
    )
}

/// The number of `tone` on the triad `reached` names, spelling forgotten.
///
/// A number is a fact about the quotient and not about the spelling, so
/// `unspelled` is applied here rather than hidden: what `ab` and `g#` share
/// is this, and the spelling questions are asked through [`voiceable`].
///
/// `numbered` sounds one note more than the number it reads, so an absent
/// answer is zero notes and pitch class zero is one — the subtraction below
/// is what turns that back into a number, and it panics if nothing sounded.
fn pc_of(start: &str, reached: &str, tone: &str) -> usize {
    asked(start, reached, &format!("numbered(unspelled({tone}(probed)))"))
        .checked_sub(1)
        .expect("the start is a triad, so something sounded")
}

/// Whether a voicing of the triad `reached` names exists over `bass`.
///
/// Present exactly when the bass pitch's class is a member of the triad *as
/// spelled*, which is the only channel in the language that can see spelling.
fn voiceable(start: &str, reached: &str, bass: &str) -> bool {
    asked(
        start,
        reached,
        &format!("option_fold(music {{ rest/1 }}, beat_for_voicing, close_position(triad_content(probed), {bass}))"),
    ) == 1
}

#[test]
fn the_three_transformations_move_one_tone_and_keep_two() {
    assert_eq!(pc_of("c_major", "refined", "triad_root"), 0, "C major is rooted on C");
    assert_eq!(pc_of("c_major", "refined", "triad_third"), 4, "with a major third");
    assert_eq!(pc_of("c_major", "refined", "triad_fifth"), 7, "and a perfect fifth");

    assert_eq!(
        pc_of("c_major", "parallel(refined)", "triad_root"),
        0,
        "P keeps the root"
    );
    assert_eq!(pc_of("c_major", "parallel(refined)", "triad_fifth"), 7, "and the fifth");
    assert_eq!(
        pc_of("c_major", "parallel(refined)", "triad_third"),
        3,
        "moving only the third, by a chromatic semitone"
    );
    assert_eq!(asked("c_major", "parallel(refined)", "quality(probed)"), MINOR);

    assert_eq!(
        pc_of("c_major", "leading_tone(refined)", "triad_root"),
        4,
        "L on C major is E minor: the third became the root"
    );
    assert_eq!(
        pc_of("c_major", "leading_tone(refined)", "triad_third"),
        7,
        "the fifth the third"
    );
    assert_eq!(
        pc_of("c_major", "leading_tone(refined)", "triad_fifth"),
        11,
        "and only the root moved, by a diatonic semitone, down to B"
    );
    assert_eq!(asked("c_major", "leading_tone(refined)", "quality(probed)"), MINOR);

    assert_eq!(
        pc_of("c_major", "relative(refined)", "triad_root"),
        9,
        "R on C major is A minor, the relative pair a key signature already names"
    );
    assert_eq!(pc_of("c_major", "relative(refined)", "triad_third"), 0, "the root kept");
    assert_eq!(
        pc_of("c_major", "relative(refined)", "triad_fifth"),
        4,
        "the third kept"
    );
    assert_eq!(asked("c_major", "relative(refined)", "quality(probed)"), MINOR);
}

#[test]
fn each_transformation_is_its_own_inverse() {
    for operation in [
        "parallel",
        "leading_tone",
        "relative",
        "slide",
        "nebenverwandt",
        "hexatonic_pole",
    ] {
        let twice = format!("{operation}({operation}(refined))");
        for (start, tonic, expected) in [("c_major", "c4", MAJOR), ("c_minor", "c4", MINOR)] {
            assert_eq!(
                pc_of(start, &twice, "triad_root"),
                0,
                "{operation} twice returns the root it started from"
            );
            assert_eq!(
                asked(start, &twice, "quality(probed)"),
                expected,
                "{operation} twice returns the quality it started from"
            );
            assert!(
                voiceable(start, &twice, tonic),
                "{operation} twice returns the spelling too, so `c4` still voices it"
            );
        }
    }
}

#[test]
fn the_named_compositions_are_the_ones_omt_names() {
    assert_eq!(
        pc_of("c_major", "slide(refined)", "triad_root"),
        1,
        "S on C major is a minor triad a chromatic semitone up, keeping the third"
    );
    assert_eq!(asked("c_major", "slide(refined)", "quality(probed)"), MINOR);
    assert_eq!(
        pc_of("c_major", "slide(refined)", "triad_third"),
        4,
        "and E is the tone S keeps"
    );

    assert_eq!(
        pc_of("c_major", "nebenverwandt(refined)", "triad_root"),
        5,
        "N on C major is the minor subdominant, F minor"
    );
    assert_eq!(asked("c_major", "nebenverwandt(refined)", "quality(probed)"), MINOR);

    assert_eq!(
        pc_of("c_major", "hexatonic_pole(refined)", "triad_root"),
        8,
        "H on C major is G sharp minor, which shares no tone with it"
    );
    assert_eq!(asked("c_major", "hexatonic_pole(refined)", "quality(probed)"), MINOR);
}

#[test]
fn a_chain_is_the_composition_it_is_written_as() {
    assert_eq!(
        pc_of("c_major", "chain([parallel, leading_tone], refined)", "triad_root"),
        8,
        "P then L: C minor, then its own third made a root"
    );
    assert_eq!(
        pc_of("c_major", "then(parallel, leading_tone, refined)", "triad_root"),
        8,
        "which `then` says the same way, because both are ordinary composition"
    );
    assert_eq!(
        pc_of("c_major", "chain(no_steps, refined)", "triad_root"),
        0,
        "and an empty chain is the triad it was handed"
    );
    assert_eq!(
        asked("c_major", "chain(no_steps, refined)", "quality(probed)"),
        MAJOR,
        "unchanged in quality as well as in root"
    );
}

#[test]
fn only_a_triad_is_in_the_domain() {
    let refinements = "
    fn present(refined: Triad) -> Music { beat_for_triad(refined) }
    let seventh: Music = option_fold(music { rest/1 }, present, as_triad(chord c major7));
    let suspended: Music = option_fold(music { rest/1 }, present, as_triad(chord c sus4));
    let diminished: Music = option_fold(music { rest/1 }, present, as_triad(chord c dim));
    let minor: Music = option_fold(music { rest/1 }, present, as_triad(chord c minor));
";
    assert_eq!(counted(refinements, "seventh"), 0, "a seventh chord is not a triad");
    assert_eq!(
        counted(refinements, "suspended"),
        0,
        "nor is a suspension: it has no third"
    );
    assert_eq!(counted(refinements, "diminished"), 0, "nor a diminished triad");
    assert_eq!(counted(refinements, "minor"), 1, "the two that are, are");

    let widened = probe(
        "    fn widened(content: ChordClass) -> Triad { parallel(content) }",
        "music { c4/1 }",
    );
    let errors = errors_of(&widened);
    assert!(
        errors.iter().any(|(code, _)| *code == Code::TypeMismatch),
        "a chord class is not a triad and the checker says so: {errors:?}"
    );
}

#[test]
fn spelling_survives_a_transformation() {
    assert!(
        voiceable("c_minor", "leading_tone(refined)", "ab3"),
        "L on C minor is the A flat major triad, so `ab3` voices it"
    );
    assert!(
        !voiceable("c_minor", "leading_tone(refined)", "g#3"),
        "and `g#3` does not, though it sounds the same key: the root fell from `c` by a major third"
    );
    assert!(
        voiceable("c_major", "parallel(refined)", "eb4"),
        "P on C major has an `eb`"
    );
    assert!(
        !voiceable("c_major", "parallel(refined)", "d#4"),
        "and never a `d#`, because the third moved by a chromatic semitone"
    );
}

#[test]
fn the_hexatonic_cycle_closes_only_after_the_spelling_is_forgotten() {
    let cycle = "chain([parallel, leading_tone, parallel, leading_tone, parallel, leading_tone], refined)";
    assert_eq!(
        pc_of("c_major", cycle, "triad_root"),
        0,
        "six steps and the root sounds like C again"
    );
    assert_eq!(
        asked("c_major", cycle, "quality(probed)"),
        MAJOR,
        "and it is a major triad again"
    );
    assert!(
        voiceable("c_major", "refined", "c4"),
        "the triad it started from is voiceable from `c4`"
    );
    assert!(
        !voiceable("c_major", cycle, "c4"),
        "and the one it returned is not: that triad is spelled from D, as `dbb` major"
    );
    assert!(
        voiceable("c_major", cycle, "dbb4"),
        "which is exactly the pitch that does voice it"
    );

    let quotient = "chorus(map(numbered, set_members(triad_classes(probed))))";
    let members = "chorus(map(beat_for_pc, set_members(triad_classes(probed))))";
    assert_eq!(asked("c_major", "refined", members), 3, "a triad is three classes");
    assert_eq!(asked("c_major", cycle, members), 3, "before and after the cycle");
    assert_eq!(
        asked("c_major", "refined", quotient),
        14,
        "and those classes are 0, 4, and 7, which sound as one note more than each number"
    );
    assert_eq!(
        asked("c_major", cycle, quotient),
        14,
        "the very same three, so the cycle closed in the quotient and nowhere else"
    );
}

#[test]
fn the_bundled_example_compiles() {
    let errors = errors_of(NEO_RIEMANNIAN);
    assert!(
        errors.is_empty(),
        "`examples/neo-riemannian.musa` must compile: {errors:?}"
    );
}
