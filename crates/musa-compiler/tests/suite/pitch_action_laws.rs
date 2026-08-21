use musa_compiler::{CompileOptions, SourceDocument, compile};
use musa_score::{Accidental, Interval, Letter, WrittenPitch};
use proptest::prelude::*;

fn pitch_strategy() -> impl Strategy<Value = WrittenPitch> {
    (0_i8..7, -100_000_i32..=100_000, -100_000_i32..=100_000).prop_map(|(letter, accidental, octave)| WrittenPitch {
        letter: Letter::from_steps(letter).unwrap_or(Letter::C),
        accidental: Accidental(accidental),
        octave,
    })
}

fn interval_strategy() -> impl Strategy<Value = Interval> {
    (-1_000_000_i64..=1_000_000, -1_000_000_i64..=1_000_000).prop_map(|(diatonic_steps, semitones)| Interval {
        diatonic_steps,
        semitones,
    })
}

proptest! {
    #[test]
    fn zero_is_the_identity(pitch in pitch_strategy()) {
        prop_assert_eq!(pitch.transpose(Interval::ZERO), Some(pitch));
    }

    #[test]
    fn componentwise_addition_acts_compatibly(
        pitch in pitch_strategy(),
        first in interval_strategy(),
        second in interval_strategy(),
    ) {
        let composed = first.compose(second);
        let stepwise = pitch.transpose(first).and_then(|moved| moved.transpose(second));
        let direct = composed.and_then(|interval| pitch.transpose(interval));
        prop_assert_eq!(stepwise, direct);
    }

    #[test]
    fn inverse_undoes_an_interval(pitch in pitch_strategy(), interval in interval_strategy()) {
        let restored = pitch
            .transpose(interval)
            .and_then(|moved| interval.inverse().and_then(|inverse| moved.transpose(inverse)));
        prop_assert_eq!(restored, Some(pitch));
    }

    #[test]
    fn octave_translation_is_the_pitchclass_quotient(pitch in pitch_strategy()) {
        let translated = pitch.transpose(Interval { diatonic_steps: 7, semitones: 12 });
        prop_assert_eq!(translated.map(WrittenPitch::pitch_class), Some(pitch.pitch_class()));
    }

    // ---- the torsor law, and the carrier that does not satisfy it ----------
    //
    // `05-verification.md` §4's law 7 asks for cancellation, and cancellation is
    // what `Torsor<Pitch, Interval>` asserts: exactly one interval joins any
    // ordered pair of written pitches. The two tests below are its two halves —
    // that the difference lands, and that nothing else does — and the third is
    // why `NoteName` carries `Action` and no torsor.

    /// The difference of two pitches carries the first to the second exactly.
    #[test]
    fn a_difference_carries_the_first_pitch_to_the_second(
        from in pitch_strategy(),
        to in pitch_strategy(),
    ) {
        let arrived = from.between(to).and_then(|span| from.transpose(span));
        prop_assert_eq!(arrived, Some(to));
    }

    /// And it is the *only* interval that does, which is the half that makes
    /// this a torsor rather than a transitive action. Stated from the other
    /// direction — every interval that lands is the difference — because that
    /// is the form a counterexample would take.
    #[test]
    fn no_second_interval_carries_one_pitch_to_another(
        from in pitch_strategy(),
        interval in interval_strategy(),
    ) {
        if let Some(to) = from.transpose(interval) {
            prop_assert_eq!(from.between(to), Some(interval));
        }
    }

    /// Why `NoteName` has no torsor: the octave fixes every spelled class, so
    /// two classes are joined by infinitely many intervals rather than by one.
    /// `03-musical-domains.md` §1.1 states the absence as part of law 7, and
    /// this is the counterexample it rests on.
    #[test]
    fn an_octave_fixes_every_spelled_class_so_the_difference_is_not_unique(
        pitch in pitch_strategy(),
        octaves in -1_000_i64..=1_000,
    ) {
        let interval = Interval {
            diatonic_steps: octaves.saturating_mul(7),
            semitones: octaves.saturating_mul(12),
        };
        let moved = pitch.pitch_class().transpose(interval);
        prop_assert_eq!(moved, Some(pitch.pitch_class()));
    }

    #[test]
    fn arbitrary_alterations_round_trip(
        letter in 0_i8..7,
        accidental in -10_000_i32..=10_000,
        octave in -10_000_i32..=10_000,
    ) {
        let pitch = WrittenPitch {
            letter: Letter::from_steps(letter).unwrap_or(Letter::C),
            accidental: Accidental(accidental),
            octave,
        };
        prop_assert_eq!(WrittenPitch::parse(&pitch.to_string()), Some(pitch));
    }
}

#[test]
fn enharmonic_pitches_remain_different_written_values() {
    assert_ne!(WrittenPitch::parse("c#4"), WrittenPitch::parse("db4"));
}

#[test]
fn named_intervals_cover_compound_direction_and_multiple_alteration() {
    assert_eq!(
        Interval::parse("M10", false),
        Some(Interval {
            diatonic_steps: 9,
            semitones: 16
        })
    );
    assert_eq!(
        Interval::parse("m2", true),
        Some(Interval {
            diatonic_steps: -1,
            semitones: -1
        })
    );
    assert_eq!(
        Interval::parse("AAA4", false),
        Some(Interval {
            diatonic_steps: 3,
            semitones: 8
        })
    );
    assert_eq!(
        Interval::parse("dim7", false),
        Some(Interval {
            diatonic_steps: 6,
            semitones: 9
        })
    );
    assert_eq!(
        Interval::parse("ddd10", false),
        Some(Interval {
            diatonic_steps: 9,
            semitones: 12
        })
    );
    assert_eq!(
        Interval::parse("M10", false).map(|value| value.to_string()),
        Some("M10".to_owned())
    );
    assert_eq!(
        Interval::parse("m2", true).map(|value| value.to_string()),
        Some("down m2".to_owned())
    );
    assert_eq!(
        Interval::parse("dim7", false).map(|value| value.to_string()),
        Some("dim7".to_owned())
    );
}

#[test]
fn a_pitch_parameter_can_determine_every_note_of_contextual_music() {
    let source = r#"
piece "Computed notes" {
    fn turn(root: Pitch) -> EventTrack<WrittenTime> { music {
        root/4
        (root up M2)/4
        (root up M10)/4
        (root down m2)/4
    } }
    score { part p { voice v { use turn(c4); } } }
}
"#;
    let compilation = compile(
        &SourceDocument::new(source, "pitch-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:#?}", compilation.diagnostics());
    let pitches = compilation.snapshot().map(|snapshot| {
        snapshot
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices())
            .flat_map(|(_, voice)| voice.events())
            .filter_map(|event| match &event.kind {
                musa_score::ScoreEventKind::Note { pitch } => Some(pitch.to_string()),
                musa_score::ScoreEventKind::Rest | musa_score::ScoreEventKind::Chord { .. } => None,
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(
        pitches,
        Some(vec!["c4".to_owned(), "d4".to_owned(), "e5".to_owned(), "b3".to_owned()])
    );
}
