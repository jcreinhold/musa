//! Compiler contract tests: snapshot shape, diagnostics, and the algebraic
//! laws of roadmap §5.1–§5.2 as properties.

// The monoid laws below exercise the time operators directly; those ops are
// total for musa's magnitudes (see `musa-compiler/src/time.rs`).
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{Compilation, CompileOptions, MusicalDuration, SourceDocument, compile};
use num_rational::Ratio;
use proptest::prelude::*;

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");
const INVENTION: &str = include_str!("../../../../examples/invention.musa");
const COUNTERPOINT: &str = include_str!("../../../../examples/counterpoint.musa");

fn compile_source(text: &str) -> Compilation {
    compile(&SourceDocument::new(text, "test.musa"), &CompileOptions::default())
}

fn messages(compilation: &Compilation) -> Vec<String> {
    compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{:?}[{}]: {}", diagnostic.severity, diagnostic.code, diagnostic.message))
        .collect()
}

/// Whether some diagnostic has this code and names `needle`.
///
/// Assertions go through the code rather than the prose wherever the test is
/// about *what was rejected*: a message is writing and gets rewritten, and a
/// test that breaks when a sentence improves is a test that discourages the
/// improvement.
fn reports(compilation: &Compilation, code: musa_compiler::Code, needle: &str) -> bool {
    compilation
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == code && diagnostic.message.contains(needle))
}

#[test]
fn examples_compile_with_snapshots() {
    for source in [GLASS_MOUNTAIN, INVENTION, COUNTERPOINT] {
        let compilation = compile_source(source);
        assert!(
            !compilation.has_errors(),
            "errors: {}",
            messages(&compilation).join("\n")
        );
        assert!(compilation.snapshot().is_some());
    }
}

#[test]
fn counterpoint_snapshot_shape() {
    let compilation = compile_source(COUNTERPOINT);
    let snapshot = compilation.snapshot();
    assert!(snapshot.is_some());
    let Some(snapshot) = snapshot else { return };
    insta::assert_snapshot!(format!("{snapshot:#?}"));
}

#[test]
fn motif_application_expands_with_provenance() {
    let compilation = compile_source(GLASS_MOUNTAIN);
    assert!(
        !compilation.has_errors(),
        "errors: {}",
        messages(&compilation).join("\n")
    );
    let snapshot = compilation.snapshot();
    let Some(snapshot) = snapshot else { return };
    let violin = snapshot
        .parts()
        .iter()
        .find(|(_, part)| part.name() == "violin")
        .map(|(_, part)| part);
    let Some(violin) = violin else { return };
    let lead = violin.voices().map(|(_, voice)| voice).next();
    let Some(lead) = lead else { return };
    // `use sigh();` then `transpose down P5 { use sigh(); }`: two expansions
    // of a 5-item motif.
    assert_eq!(lead.events().len(), 10);
    let Some(first) = lead.events().first() else { return };
    // The default argument bound the `root` parameter to e5.
    assert!(format!("{:?}", first.kind).contains("letter: E"));
    assert!(format!("{:?}", first.kind).contains("octave: 5"));
    assert_eq!(first.origin.expansion_path.len(), 1);
    // The sixth event is the transposed expansion's first note: e5 down a
    // perfect fifth is a4, and its path records both steps in application
    // order (outermost first).
    let sixth = lead.events().get(5);
    let Some(sixth) = sixth else { return };
    assert!(format!("{:?}", sixth.kind).contains("letter: A"));
    assert!(format!("{:?}", sixth.kind).contains("octave: 4"));
    assert_eq!(sixth.origin.expansion_path.len(), 2);
    assert!(format!("{:?}", sixth.origin.expansion_path).starts_with("[Transposition"));
}

#[test]
fn transpose_spells_correctly() {
    let source = "piece \"x\" { score { part p { voice v {
        transpose up P5 { c4/4 }
        transpose up M3 { e4/4 }
        transpose down m3 { d5/4 }
        transpose up m2 { b4/4 }
    } } } }";
    let compilation = compile_source(source);
    assert!(
        !compilation.has_errors(),
        "errors: {}",
        messages(&compilation).join("\n")
    );
    let Some(snapshot) = compilation.snapshot() else { return };
    let Some(voice) = snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
    else {
        return;
    };
    let spellings: Vec<String> = voice
        .events()
        .iter()
        .filter_map(|event| match &event.kind {
            musa_compiler::ScoreEventKind::Note { pitch } => Some(pitch.to_string()),
            musa_compiler::ScoreEventKind::Rest | musa_compiler::ScoreEventKind::Chord { .. } => None,
        })
        .collect();
    assert_eq!(spellings, vec!["g4", "g#4", "b4", "c5"]);
}

#[test]
fn repeat_expands_iterations_with_provenance() {
    let source = "piece \"x\" { score { part p { voice v {
        repeat 3 { c4/4 }
    } } } }";
    let compilation = compile_source(source);
    assert!(
        !compilation.has_errors(),
        "errors: {}",
        messages(&compilation).join("\n")
    );
    let Some(snapshot) = compilation.snapshot() else { return };
    let Some(voice) = snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
    else {
        return;
    };
    assert_eq!(voice.events().len(), 3);
    let onsets: Vec<String> = voice.events().iter().map(|event| event.onset.to_string()).collect();
    assert_eq!(onsets, vec!["0", "1/4", "1/2"]);
    for (index, event) in voice.events().iter().enumerate() {
        assert_eq!(
            event.origin.expansion_path,
            vec![musa_compiler::ExpansionStep::RepeatIteration(
                u32::try_from(index).unwrap_or(0)
            )]
        );
    }
}

#[test]
fn unknown_motif_is_an_error() {
    let compilation = compile_source("piece \"x\" { score { part p { voice v { use missing(); } } } }");
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::UnknownName, "`missing`"),
        "{:?}",
        messages(&compilation)
    );
}

#[test]
fn motifs_only_see_earlier_motifs() {
    let compilation = compile_source(
        "piece \"x\" {
            motif a() { use b(); }
            motif b() { c4/4 }
            score { part p { voice v { use a(); } } }
        }",
    );
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::Misplaced, "`b`"),
        "{:?}",
        messages(&compilation)
    );
}

#[test]
fn nested_motifs_and_duration_parameters_expand() {
    let source = "piece \"x\" {
        motif cell(d: Duration) { c4 d d4 d }
        motif pair(d: Duration) { use cell(d); use cell(d); }
        score { part p { voice v { use pair(1/16); use pair(1/8); } } }
    }";
    let compilation = compile_source(source);
    assert!(
        !compilation.has_errors(),
        "errors: {}",
        messages(&compilation).join("\n")
    );
    let Some(snapshot) = compilation.snapshot() else { return };
    let Some(voice) = snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
    else {
        return;
    };
    let durations: Vec<String> = voice
        .events()
        .iter()
        .map(|event| event.notated_duration.spelling.clone())
        .collect();
    // 1/16 bound through two levels, then the 1/8 default.
    assert_eq!(
        durations,
        vec!["1/16", "1/16", "1/16", "1/16", "1/8", "1/8", "1/8", "1/8"]
    );
    assert_eq!(voice.span().to_string(), "3/4");
}

/// The written durations of the first voice, in order.
fn durations_of(source: &str) -> Vec<String> {
    let compilation = compile_source(source);
    assert!(
        !compilation.has_errors(),
        "errors: {}",
        messages(&compilation).join("\n")
    );
    let Some(snapshot) = compilation.snapshot() else {
        return Vec::new();
    };
    snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
        .map(|voice| {
            voice
                .events()
                .iter()
                .map(|event| event.notated_duration.spelling.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// `c4/4` is not a shorthand that means something like `c4 1/4`. It is the
/// same note: same value, same spelling, same fact.
#[test]
fn the_short_and_long_forms_of_a_duration_are_one_duration() {
    let short = durations_of("piece \"x\" { score { part p { voice v { c4/4 d4/8 e4/1 } } } }");
    let long = durations_of("piece \"x\" { score { part p { voice v { c4/4 d4/8 e4/1 } } } }");
    assert_eq!(short, long);
    assert_eq!(short, vec!["1/4", "1/8", "1"]);
}

/// An augmentation dot multiplies by `2 − 2⁻ᵈ`, and what the score records is
/// the fraction — one duration, one spelling, however it was written.
#[test]
fn an_augmentation_dot_is_half_again() {
    assert_eq!(
        durations_of("piece \"x\" { score { part p { voice v { c4/4. d4/4.. e4/2. f4/3. } } } }"),
        vec!["3/8", "7/16", "3/4", "1/2"]
    );
}

/// The long form already writes what a dot on it would mean, so it does not
/// get one: `3/8.` would be 9/16, which `9/16` says.
#[test]
fn a_dot_on_the_long_form_is_refused() {
    let compilation = compile_source("piece \"x\" { score { part p { voice v { c4 3/8. } } } }");
    assert!(
        compilation.has_errors(),
        "expected a diagnostic, got: {}",
        messages(&compilation).join("\n")
    );
    assert_eq!(
        durations_of("piece \"x\" { score { part p { voice v { c4 9/16 } } } }"),
        vec!["9/16"]
    );
}

#[test]
fn a_motif_use_that_omits_an_argument_is_an_error() {
    let compilation = compile_source(
        "piece \"x\" {
            motif m(root: Pitch) { root 1/4 }
            score { part p { voice v { use m(); } } }
        }",
    );
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::WrongArity, "root"),
        "{:?}",
        messages(&compilation)
    );
}

#[test]
fn unknown_clef_is_an_error() {
    let compilation = compile_source("piece \"x\" { score { part p { clef soprano; voice v { c5/1 } } } }");
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::UnknownWord, "`soprano`"),
        "{:?}",
        messages(&compilation)
    );
    assert!(compilation.snapshot().is_none());
}

#[test]
fn duplicate_part_is_an_error() {
    let compilation =
        compile_source("piece \"x\" { score { part p { voice v { c5/1 } } part p { voice w { d5/1 } } } }");
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::DuplicateName, "`p`"),
        "{:?}",
        messages(&compilation)
    );
}

#[test]
fn duplicate_motif_is_an_error() {
    let compilation =
        compile_source("piece \"x\" { motif m() { c5/1 } motif m() { d5/1 } score { part p { voice v { c5/1 } } } }");
    assert!(compilation.has_errors());
    assert!(
        reports(&compilation, musa_compiler::Code::DuplicateName, "`m`"),
        "{:?}",
        messages(&compilation)
    );
}

/// A front-matter role written twice is an error, not last-wins: two composer
/// lines mean the piece disagrees with itself about who wrote it, and only the
/// person who typed them can say which is true.
#[test]
fn a_front_matter_role_written_twice_is_an_error() {
    let compilation =
        compile_source("piece \"x\" { composer \"a\"; composer \"b\"; score { part p { voice v { c5/1 } } } }");
    assert!(compilation.has_errors());
    assert!(
        messages(&compilation)
            .iter()
            .any(|message| message.contains("already names a composer")),
        "{:?}",
        messages(&compilation)
    );
}

/// The four roles are independent slots: all of them together compile, and
/// each reaches the snapshot as itself.
#[test]
fn all_four_front_matter_roles_reach_the_snapshot() {
    let compilation = compile_source(
        "piece \"x\" { subtitle \"s\"; composer \"c\"; arranger \"a\"; copyright \"©\"; \
         score { part p { voice v { c5/1 } } } }",
    );
    assert!(!compilation.has_errors(), "{:?}", messages(&compilation));
    let Some(snapshot) = compilation.into_snapshot() else {
        return;
    };
    let front = snapshot.front_matter();
    assert_eq!(front.subtitle.as_deref(), Some("s"));
    assert_eq!(front.composer.as_deref(), Some("c"));
    assert_eq!(front.arranger.as_deref(), Some("a"));
    assert_eq!(front.copyright.as_deref(), Some("©"));
}

#[test]
fn provenance_points_back_to_source() {
    let compilation = compile_source(COUNTERPOINT);
    let Some(snapshot) = compilation.snapshot() else { return };
    let violin = snapshot
        .parts()
        .iter()
        .find(|(_, part)| part.name() == "violin")
        .map(|(_, part)| part);
    let Some(violin) = violin else { return };
    let Some(first) = violin
        .voices()
        .map(|(_, voice)| voice)
        .next()
        .and_then(|voice| voice.events().first())
    else {
        return;
    };
    // The first event's origin span covers `d4/2` in the source.
    let span = first.origin.source_span;
    let covered = COUNTERPOINT.get(span.start as usize..span.end as usize);
    assert_eq!(covered, Some("d4/2"));
    assert!(first.origin.expansion_path.is_empty());
}

// --- Laws ------------------------------------------------------------------

fn duration() -> impl Strategy<Value = MusicalDuration> {
    (0i64..=64, 1i64..=16).prop_map(|(numerator, denominator)| MusicalDuration::new(Ratio::new(numerator, denominator)))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn duration_monoid_zero_is_identity(a in duration()) {
        prop_assert_eq!(a + MusicalDuration::ZERO, a);
        prop_assert_eq!(MusicalDuration::ZERO + a, a);
    }

    #[test]
    fn duration_monoid_addition_is_associative(a in duration(), b in duration(), c in duration()) {
        prop_assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn duration_monoid_addition_is_commutative(a in duration(), b in duration()) {
        prop_assert_eq!(a + b, b + a);
    }

    /// §5.2's span law at the snapshot level: lowering a sequence of notes
    /// yields span(a then b) = span(a) + span(b) and cumulative onsets.
    #[test]
    fn lowering_span_is_additive(
        durations in prop::collection::vec(
            prop::sample::select(vec!["1/4", "1/8", "1/2", "3/8", "1"]),
            1..=12
        )
    ) {
        let mut source = String::from("piece \"x\" { score { part p { voice v {\n");
        let mut expected = Ratio::from_integer(0);
        for duration_text in &durations {
            source.push_str("c4 ");
            source.push_str(duration_text);
            source.push('\n');
            let (numerator, denominator) = duration_text.split_once('/').unwrap_or((duration_text, "1"));
            let n: i64 = numerator.parse().unwrap_or(0);
            let d: i64 = denominator.parse().unwrap_or(1);
            expected += Ratio::new(n, d);
        }
        source.push_str("} } } }");
        let compilation = compile_source(&source);
        assert!(!compilation.has_errors(), "errors: {}", messages(&compilation).join("\n"));
        if let Some(snapshot) = compilation.snapshot() {
            if let Some(voice) =
                snapshot.parts().iter().next().and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
            {
                assert_eq!(voice.span().as_ratio(), expected);
                // Onsets are cumulative: each event starts where the previous ended.
                let mut cursor = Ratio::from_integer(0);
                for event in voice.events() {
                    assert_eq!(event.onset.as_ratio(), cursor);
                    cursor += event.notated_duration.value.as_ratio();
                }
            } else {
                prop_assert!(false, "no voice in snapshot");
            }
        } else {
            prop_assert!(false, "no snapshot");
        }
    }

    /// §5.4 composition law at the value level: whenever both evaluation
    /// orders stay spellable, transpose(transpose(p, a), b) == transpose(p, a+b).
    #[test]
    fn transposition_composes(
        pitch in pitch_literal(),
        a in interval_literal(),
        b in interval_literal()
    ) {
        let combined = musa_compiler::Interval {
            diatonic_steps: a.diatonic_steps.saturating_add(b.diatonic_steps),
            semitones: a.semitones.saturating_add(b.semitones),
        };
        let stepwise = pitch.transpose(a).and_then(|moved| moved.transpose(b));
        let direct = pitch.transpose(combined);
        if let (Some(stepwise), Some(direct)) = (stepwise, direct) {
            assert_eq!(stepwise, direct);
        }
    }

    /// Octave transposition preserves letter and accidental exactly.
    #[test]
    fn octave_transposition_preserves_spelling(pitch in pitch_literal()) {
        let up = musa_compiler::Interval { diatonic_steps: 7, semitones: 12 };
        let moved = pitch.transpose(up);
        assert!(moved.is_some());
        if let Some(moved) = moved {
            assert_eq!(moved.letter, pitch.letter);
            assert_eq!(moved.accidental, pitch.accidental);
            assert_eq!(moved.octave, pitch.octave.saturating_add(1));
        }
    }
}

fn pitch_literal() -> impl Strategy<Value = musa_compiler::WrittenPitch> {
    (0i8..7, -2i32..=2, 1i32..7).prop_map(|(steps, accidental, octave)| musa_compiler::WrittenPitch {
        letter: musa_compiler::Letter::from_steps(steps).unwrap_or(musa_compiler::Letter::C),
        accidental: musa_compiler::Accidental(accidental),
        octave,
    })
}

fn interval_literal() -> impl Strategy<Value = musa_compiler::Interval> {
    prop::sample::select(vec![
        (1i64, 1i64),
        (1, 2),
        (2, 3),
        (2, 4),
        (3, 5),
        (4, 7),
        (5, 8),
        (6, 10),
        (-1, -1),
        (-2, -3),
        (-3, -5),
        (-4, -7),
    ])
    .prop_map(|(diatonic_steps, semitones)| musa_compiler::Interval {
        diatonic_steps,
        semitones,
    })
}
