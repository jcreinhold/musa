//! What the chromatic quotient promises (docs/prompts/105, and
//! `docs/language/03-musical-domains.md` §1 and §4).
//!
//! `pc12` is `Z/12Z` and a spelled pitch class is not one. Only `χ` is total,
//! and it is not injective; the way back is a policy the author names, which
//! is why it takes a collection and may answer nothing. Every law here holds
//! that boundary at the language surface, where an author would meet it.
//!
//! Counting is how a value becomes visible from outside the compiler. A
//! `nat` is turned into that many overlaid notes and a `list` into one note
//! per member, so `pcset12_vector` and `pcset12_members` can be read off a
//! score snapshot. The algebra itself — the exhaustive `T`/`I` reference,
//! the brute-force prime-form model over all 4096 sets, the interval-class
//! accounting — is checked where it is computed, in `crate::pc12`'s own unit
//! tests, because the domain is crate-private and nothing outside can see a
//! `Pc12` at all.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Code, CompileOptions, ScoreEventKind, ScoreSnapshot, Severity, SourceDocument, compile};

/// The counting apparatus every fixture below is written against.
///
/// `tally` sounds one note per unit of a `nat` and `chorus` one note per
/// member of a list, so a count that has no other way out of the compiler
/// leaves as notes in a voice.
const PRELUDE: &str = r"
    import std::post_tonal::pcset;
    import std::post_tonal::serial;

    meter 4/4;

    fn tick(one: Music, carried: Music) -> Music { overlay(one, carried) }
    fn beat() -> Music { music { c4/1 } }
    fn tally(count: Nat) -> Music { list_fold(music { rest/1 }, tick, repeat(beat(), count)) }
    fn beat_for_pc(member: Pc12) -> Music { beat() }
    fn beat_for_nat(count: Nat) -> Music { beat() }
    fn beat_for_spelling(spelled: NoteName) -> Music { beat() }
    fn chorus(voices: List<Music>) -> Music { list_fold(music { rest/1 }, tick, voices) }
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
/// pitches and not events: `tally(3)` is a chord of three, not three notes.
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

#[test]
fn a_spelled_pitch_class_is_not_an_unspelled_one() {
    let spelled_where_unspelled_belongs = probe("    let wrong: Pc12 = pitchclass_of(c4);", "beat()");
    let errors = errors_of(&spelled_where_unspelled_belongs);
    assert!(
        errors.iter().any(|(code, _)| *code == Code::TypeMismatch),
        "a `pitchclass` must not stand where a `pc12` belongs: {errors:?}"
    );

    let unspelled_where_spelled_belongs = probe("    let wrong: NoteName = pc(0);", "beat()");
    let errors = errors_of(&unspelled_where_spelled_belongs);
    assert!(
        errors.iter().any(|(code, _)| *code == Code::TypeMismatch),
        "and a `pc12` must not stand where a `pitchclass` belongs: {errors:?}"
    );
}

#[test]
fn a_pitch_class_is_not_the_number_that_names_it() {
    let errors = errors_of(&probe("    let wrong: Nat = pc(3);", "beat()"));
    assert!(
        errors.iter().any(|(code, _)| *code == Code::TypeMismatch),
        "reading a residue as a number must be asked for: {errors:?}"
    );
}

#[test]
fn the_quotient_reduces_modulo_twelve() {
    assert_eq!(counted("", "tally(number_of(pc(11)))"), 11);
    assert_eq!(counted("", "tally(number_of(pc(13)))"), 1, "13 and 1 are one residue");
    assert_eq!(counted("", "tally(number_of(pc(25)))"), 1, "and so are 25 and 1");
    assert_eq!(counted("", "tally(number_of(pc(12)))"), 0, "12 is 0");
}

#[test]
fn forgetting_a_spelling_is_total_and_not_injective() {
    let sharp = counted("", "tally(number_of(forget_spelling(pitchclass_of(c#4))))");
    let flat = counted("", "tally(number_of(forget_spelling(pitchclass_of(db4))))");
    assert_eq!(sharp, 1, "c sharp forgets onto one");
    assert_eq!(sharp, flat, "and d flat forgets onto the same one");

    assert_eq!(
        counted("", "tally(number_of(forget_spelling(pitchclass_of(b#3))))"),
        0,
        "b sharp forgets onto zero, which c also forgets onto"
    );
    assert_eq!(
        counted("", "tally(number_of(forget_spelling(pitchclass_of(cbb4))))"),
        10,
        "a deeply altered spelling is an ordinary member of the quotient"
    );
}

#[test]
fn a_spelling_needs_a_collection_and_may_not_exist_in_it() {
    let bindings = "
    fn present(spelled: NoteName) -> Music { beat() }
    let in_c: Music = option_fold(music { rest/1 }, present, spelled_in(pc(1), scale c major));
    let in_d: Music = option_fold(music { rest/1 }, present, spelled_in(pc(1), scale d major));
";
    assert_eq!(
        counted(bindings, "in_c"),
        0,
        "C major has no note of pitch class one, and the projection says so"
    );
    assert_eq!(counted(bindings, "in_d"), 1, "D major spells it `c#`");
}

#[test]
fn a_set_holds_a_repeated_member_once() {
    assert_eq!(
        counted(
            "    let triad: PcSet12 = pcset(pcs([0, 0, 4, 7, 7]));",
            "chorus(map(beat_for_pc, set_members(triad)))"
        ),
        3,
        "a set is what was asked for, so a repetition is not an error and not a member twice"
    );
}

#[test]
fn a_set_reads_out_ascending_and_normal_order_need_not() {
    let bindings = "
    let set: PcSet12 = pcset(pcs([0, 5, 8]));
    let ascending: Music = chorus(map(tally, map(number_of, set_members(set))));
    let normal: Music = chorus(map(tally, map(number_of, normal_order(set))));
";
    assert_eq!(counted(bindings, "ascending"), 13, "0 + 5 + 8 read ascending");
    assert_eq!(
        counted(bindings, "normal"),
        13,
        "the same three members, rotated: 5, 8, 0 is the compact ordering"
    );
}

#[test]
fn the_interval_class_vector_has_six_entries_and_counts_every_pair() {
    let bindings = "    let triad: PcSet12 = pcset(pcs([0, 4, 7]));";
    assert_eq!(
        counted(bindings, "chorus(map(beat_for_nat, interval_class_vector(triad)))"),
        6,
        "six interval classes, because ic 7 is ic 5 heard the other way round"
    );
    assert_eq!(
        counted(bindings, "chorus(map(tally, interval_class_vector(triad)))"),
        3,
        "and three pairs in a three-member set"
    );
}

#[test]
fn a_set_class_survives_transposition_and_inversion() {
    let bindings = "
    let triad: PcSet12 = pcset(pcs([0, 4, 7]));
    let moved: PcSet12 = set_transposed(triad, 3);
    let mirrored: PcSet12 = set_inverted(triad, 0);
";
    let upright = counted(
        bindings,
        "chorus(map(tally, map(number_of, set_members(prime_form(triad)))))",
    );
    let moved = counted(
        bindings,
        "chorus(map(tally, map(number_of, set_members(prime_form(moved)))))",
    );
    let mirrored = counted(
        bindings,
        "chorus(map(tally, map(number_of, set_members(prime_form(mirrored)))))",
    );
    assert_eq!(upright, 10, "the major triad's prime form is 0, 3, 7");
    assert_eq!(moved, upright, "transposition does not change the set class");
    assert_eq!(mirrored, upright, "and neither does inversion");
}

#[test]
fn the_bundled_libraries_import_like_any_other() {
    let source = probe("", "beat()");
    assert!(
        errors_of(&source).is_empty(),
        "`std::post_tonal::pcset` and `std::post_tonal::serial` must compile as bundled sources"
    );
}
