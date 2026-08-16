//! What has to be true of the eight track builtins.
//!
//! Unit tests inside the crate rather than a file in `tests/suite/`, for
//! [`crate::registry::laws`]'s reason: `ScoreFact`, `VoiceTrack`, and `Origin`
//! are private to this crate, and a test outside it links against
//! `parse`/`compile`/`render_notation` and can reach none of them.
//!
//! # Two ways of asking, and why both
//!
//! Seven of the eight are δ-rules, so a law can call the [`musa_core::Rule`]
//! directly and compare its answer with the pure operation it was written from —
//! [`WrittenPitch::transpose`], [`ScoreFact::stretched`],
//! [`musa_kernel::together`]. That is the agreement half, and it is the sharper
//! test of arithmetic because it names the expected value rather than a property
//! of it.
//!
//! It is not enough on its own. A rule that answered the right value at the
//! wrong *shape* — an `Option.Some` where the signature says `Result`, a track
//! literal at the wrong base type — would pass every comparison below and get
//! stuck in front of the first program that used it. So each of the eight is
//! also applied through the core at its registered signature, normalized, and
//! re-checked. `map_note_pitches` has only that half: its reduction is a rewrite
//! and a rewrite exists nowhere but in the evaluator.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use std::sync::Arc;

use musa_core::{Cx, Datum, Refusal, Term};
use musa_kernel::{Duration, Occurrence, Position, Span, WrittenTime};
use num_rational::Ratio;

use super::{
    INVERT, PLAY, RETROGRADE, SET_NOTE_PITCHES, SHIFT, SPELLINGS, STRETCH, TOGETHER, TRACK_BEYOND, TRANSPOSE, built,
    track_type,
};
use crate::Interval;
use crate::core::{BUILTIN_OWNERSHIP, Family};
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use crate::origin::{DeclarationId, ExpansionStep, Origin, SourceSpan};
use crate::pitch::WrittenPitch;
use crate::registry::{HERE, held, literal, origin_literal, owned, plain_type, tagged_type};
use crate::scope::Scope;
use crate::score::NotatedDuration;

// ---- the subject ----

/// The origin every hand-built fact carries.
///
/// Complete in §5.7's sense — two spans and a declaration — because a law about
/// provenance that started from an incomplete origin would be measuring nothing.
fn provenance() -> Origin {
    Origin {
        source_span: SourceSpan { start: 12, end: 20 },
        definition_span: SourceSpan { start: 12, end: 20 },
        declaration: DeclarationId(3),
        expansion_path: Vec::new(),
    }
}

/// A written pitch, by the spelling source would write.
fn pitch(spelling: &str) -> WrittenPitch {
    WrittenPitch::parse(spelling).expect("the law spells its own pitches")
}

/// An exact rational, spelled as a fraction.
fn ratio(numerator: i64, denominator: i64) -> Ratio<i64> {
    Ratio::new(numerator, denominator)
}

/// `[start, start + length)`.
fn over(start: Ratio<i64>, length: Ratio<i64>) -> Span<WrittenTime> {
    Span::new(
        Position::new(start),
        Position::new(start).plus(Duration::new(length).expect("a length is nonnegative")),
    )
    .expect("a nonnegative length is a span")
}

/// One notehead.
fn note(start: Ratio<i64>, length: Ratio<i64>, spelling: &str) -> Occurrence<WrittenTime, ScoreFact> {
    Occurrence::new(
        over(start, length),
        ScoreFact {
            scope: Scope::Piece,
            kind: FactKind::Note {
                pitch: pitch(spelling),
                duration: NotatedDuration::spelled(length),
                articulations: Vec::new(),
                free: None,
            },
            origin: provenance(),
            tied: false,
        },
    )
}

/// One fact that is not a note, which is what makes "leaves the others alone" a
/// claim rather than a vacuous truth.
fn slur(start: Ratio<i64>, length: Ratio<i64>) -> Occurrence<WrittenTime, ScoreFact> {
    Occurrence::new(
        over(start, length),
        ScoreFact {
            scope: Scope::Piece,
            kind: FactKind::Slur,
            origin: provenance(),
            tied: false,
        },
    )
}

/// The track every law below transforms: two noteheads and a slur over both, in
/// a bar of one whole note.
fn subject() -> VoiceTrack {
    let whole = ratio(1, 1);
    musa_kernel::track(
        Duration::new(whole).expect("a whole note is a duration"),
        vec![
            note(ratio(0, 1), ratio(1, 2), "c4"),
            note(ratio(1, 2), ratio(1, 2), "e4"),
            slur(ratio(0, 1), whole),
        ],
    )
    .expect("the occurrences fit the bar")
}

/// The subject, as a rule's argument.
fn given() -> Datum {
    built(subject())
}

/// A literal argument of a plain base type.
fn plain<T>(name: &'static str, value: T) -> Datum
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Datum::Lit(literal(plain_type(name), value))
}

/// A written beat, as a rule's argument.
fn beat(value: Ratio<i64>) -> Datum {
    Datum::Lit(literal(
        tagged_type("Duration", crate::core::Coordinate::WrittenTime),
        value,
    ))
}

/// The track inside a `Result.Ok`, or a panic naming what came back instead.
fn accepted(answer: Option<&Datum>, what: &str) -> VoiceTrack {
    let Some(Datum::Case { constructor, fields }) = answer else {
        panic!("{what} answered {answer:?} rather than a `Result`");
    };
    assert_eq!(&**constructor, "Result.Ok", "{what} refused: {fields:?}");
    let Some(Datum::Lit(value)) = fields.first() else {
        panic!("{what} answered a `Result.Ok` holding no literal");
    };
    held::<VoiceTrack>(value).expect("the answer is a track").clone()
}

/// The track a total rule answered.
fn plainly(answer: Option<&Datum>, what: &str) -> VoiceTrack {
    let Some(Datum::Lit(value)) = answer else {
        panic!("{what} answered {answer:?} rather than a track");
    };
    held::<VoiceTrack>(value).expect("the answer is a track").clone()
}

/// The written pitches of a track, in the order its occurrences stand.
fn pitches(track: &VoiceTrack) -> Vec<WrittenPitch> {
    track
        .occurrences()
        .iter()
        .filter_map(|occurrence| occurrence.payload().pitch_of())
        .collect()
}

/// The last expansion step of every fact, in order.
fn steps(track: &VoiceTrack) -> Vec<Option<ExpansionStep>> {
    track
        .occurrences()
        .iter()
        .map(|occurrence| occurrence.payload().origin.expansion_path.last().cloned())
        .collect()
}

// ---- the registration ----

/// The eight rows of `BUILTIN_OWNERSHIP`'s track family are exactly
/// [`SPELLINGS`], and every one of them is registered and nameable.
///
/// The other half of the accounting law that left [`crate::registry::rules`]'s
/// `UNREGISTERED` in prompt 141h. That array now says nothing about the track,
/// so this says it instead — and says it the harder way round, off the table and
/// off the registry rather than off a number.
#[test]
fn the_eight_track_rows_are_registered_under_the_names_the_table_gives() {
    let named: Vec<&str> = BUILTIN_OWNERSHIP
        .iter()
        .filter(|entry| matches!(entry.family, Family::Track))
        .map(|entry| entry.spelling)
        .collect();
    let mut expected = SPELLINGS.to_vec();
    let mut found = named.clone();
    expected.sort_unstable();
    found.sort_unstable();
    assert_eq!(found, expected, "the track family's rows are the ones registered");

    let cx = owned().expect("the compiler's own context builds");
    for spelling in SPELLINGS.into_iter().chain(TRACK_BEYOND) {
        head(&cx, spelling);
    }
}

/// The head of a registered builtin, by name.
fn head(cx: &Cx, spelling: &str) -> Term {
    let (term, _) = musa_core::infer(cx, &musa_core::Raw::var(HERE, spelling)).unwrap_or_else(|refusal| {
        let unknown = matches!(refusal, musa_core::ElabError::Refused(Refusal::UnknownName { .. }));
        panic!("`{spelling}` is not registered (unknown name: {unknown})");
    });
    term
}

// ---- agreement with the operation each is written from ----

/// `transpose` raises every written pitch by the interval and records that it
/// did.
#[test]
fn transposing_raises_every_written_pitch_and_records_the_step() {
    let up_a_fifth = Interval::parse("P5", false).expect("a perfect fifth is an interval");
    let raised = accepted(
        TRANSPOSE(&[plain("Interval", up_a_fifth), given()]).as_ref(),
        "`transpose`",
    );
    assert_eq!(
        pitches(&raised),
        vec![
            pitch("c4").transpose(up_a_fifth).expect("c4 rises a fifth"),
            pitch("e4").transpose(up_a_fifth).expect("e4 rises a fifth"),
        ],
        "every notehead moved by the interval, and by the same arithmetic"
    );
    assert_eq!(
        steps(&raised),
        vec![Some(ExpansionStep::Transposition(up_a_fifth)); 3],
        "every fact records the transposition, including the slur that has no pitch"
    );
    assert_eq!(raised.duration(), subject().duration(), "a transposition is not a move");
}

/// `stretch` scales the spans and the notation together.
#[test]
fn stretching_scales_the_spans_and_the_written_durations() {
    let twice = ratio(2, 1);
    let longer = accepted(STRETCH(&[plain("Ratio", twice), given()]).as_ref(), "`stretch`");
    assert_eq!(
        longer.duration(),
        Duration::new(ratio(2, 1)).expect("two whole notes is a duration"),
        "the bar is twice as long"
    );
    let ends: Vec<Ratio<i64>> = longer
        .occurrences()
        .iter()
        .map(|occurrence| occurrence.span().end().as_ratio())
        .collect();
    assert_eq!(ends, vec![ratio(1, 1), ratio(2, 1), ratio(2, 1)], "so is every span");
    let written: Vec<ScoreFact> = longer
        .occurrences()
        .iter()
        .map(|occurrence| occurrence.payload().clone())
        .collect();
    for (moved, original) in written.iter().zip(subject().occurrences()) {
        let mut expected = original.payload().stretched(twice);
        expected.origin.expansion_path.push(ExpansionStep::Stretch(twice));
        assert_eq!(*moved, expected, "each fact is what `ScoreFact::stretched` answers");
    }
}

/// `retrograde` mirrors every span about the track's midpoint and touches
/// nothing else.
#[test]
fn retrograde_mirrors_the_spans_and_keeps_the_duration() {
    let backwards = plainly(RETROGRADE(&[given()]).as_ref(), "`retrograde`");
    assert_eq!(
        backwards.duration(),
        subject().duration(),
        "a retrograde does not change how long the music is"
    );
    let spans: Vec<(Ratio<i64>, Ratio<i64>)> = backwards
        .occurrences()
        .iter()
        .map(|occurrence| (occurrence.span().start().as_ratio(), occurrence.span().end().as_ratio()))
        .collect();
    assert_eq!(
        spans,
        vec![
            (ratio(1, 2), ratio(1, 1)),
            (ratio(0, 1), ratio(1, 2)),
            (ratio(0, 1), ratio(1, 1)),
        ],
        "each span is reflected about the bar, and the slur over the whole bar is its own mirror"
    );
    assert_eq!(
        pitches(&backwards),
        vec![pitch("c4"), pitch("e4")],
        "the payloads are untouched: a retrograde is a statement about when"
    );
    assert_eq!(steps(&backwards), vec![Some(ExpansionStep::Retrograde); 3]);
}

/// `invert` mirrors every written pitch about the axis, diatonically.
#[test]
fn inverting_mirrors_every_written_pitch_about_the_axis() {
    let axis = pitch("c4");
    let mirrored = accepted(INVERT(&[plain("Pitch", axis), given()]).as_ref(), "`invert`");
    assert_eq!(
        pitches(&mirrored),
        vec![
            pitch("c4").invert(axis).expect("c4 mirrors about itself"),
            pitch("e4").invert(axis).expect("e4 mirrors about c4"),
        ],
        "every notehead is what `WrittenPitch::invert` answers"
    );
    assert_eq!(
        steps(&mirrored),
        vec![Some(ExpansionStep::Inversion { axis: axis.to_string() }); 3],
        "the step names the axis by the spelling it was written with"
    );
}

/// `shift` moves the music later and lengthens the track by exactly as much.
///
/// Both halves matter. Moving the occurrences without lengthening the track
/// would push the last one outside it, which is the kernel's own bound and the
/// error this rule would otherwise hit.
#[test]
fn shifting_moves_the_music_and_lengthens_the_track_to_hold_it() {
    let by = ratio(1, 4);
    let later = accepted(SHIFT(&[beat(by), given()]).as_ref(), "`shift`");
    assert_eq!(
        later.duration(),
        Duration::new(ratio(5, 4)).expect("a bar and a quarter is a duration"),
        "the track grew by the offset"
    );
    let starts: Vec<Ratio<i64>> = later
        .occurrences()
        .iter()
        .map(|occurrence| occurrence.span().start().as_ratio())
        .collect();
    assert_eq!(starts, vec![ratio(1, 4), ratio(3, 4), ratio(1, 4)], "everything moved");
    assert_eq!(
        steps(&later),
        vec![None; 3],
        "and nothing records it, because the span already does"
    );
}

/// `together` is the kernel's own stacking and not a second one.
#[test]
fn together_is_the_kernels_stacking() {
    let half = musa_kernel::track(
        Duration::new(ratio(1, 2)).expect("half a bar is a duration"),
        vec![note(ratio(0, 1), ratio(1, 2), "g4")],
    )
    .expect("the occurrence fits");
    let stacked = plainly(TOGETHER(&[given(), built(half.clone())]).as_ref(), "`together`");
    assert_eq!(
        stacked,
        musa_kernel::together(vec![subject(), half]),
        "the rule calls the operation §5.7's composition clause is about"
    );
    assert_eq!(
        stacked.duration(),
        subject().duration(),
        "nothing pads the shorter part"
    );
}

/// `set_note_pitches` replaces the *i*th note and keeps everything else.
///
/// Registered in neither ownership table, so this is where its totality is
/// stated: a list shorter than the noteheads is fewer replacements and not a
/// refusal, and a fact that is not a note is never a replacement site.
#[test]
fn set_note_pitches_replaces_each_note_in_turn_and_nothing_else() {
    let one = list(vec![plain("Pitch", pitch("g4"))]);
    let answered = plainly(SET_NOTE_PITCHES(&[given(), one]).as_ref(), "`set_note_pitches`");
    assert_eq!(
        pitches(&answered),
        vec![pitch("g4"), pitch("e4")],
        "the first notehead took the only member and the second kept its own"
    );
    assert_eq!(
        steps(&answered),
        vec![Some(ExpansionStep::MapNotePitches), None, None],
        "only the notehead that changed records that it did"
    );
}

/// A `List` of data, as a rule's argument.
fn list(members: Vec<Datum>) -> Datum {
    let mut built = Datum::Case {
        constructor: Arc::from("List.Empty"),
        fields: Vec::new(),
    };
    for member in members.into_iter().rev() {
        built = Datum::Case {
            constructor: Arc::from("List.Cons"),
            fields: vec![member, built],
        };
    }
    built
}

// ---- §5.7's construction clause ----

/// A `Scope`, as a rule's argument.
fn scope(constructor: &'static str, fields: Vec<Datum>) -> Datum {
    Datum::Case {
        constructor: Arc::from(constructor),
        fields,
    }
}

/// `Nat`, one `Succ` at a time.
fn whole(value: u32) -> Datum {
    let mut built = Datum::Case {
        constructor: Arc::from("Nat.Zero"),
        fields: Vec::new(),
    };
    for _ in 0..value {
        built = Datum::Case {
            constructor: Arc::from("Nat.Succ"),
            fields: vec![built],
        };
    }
    built
}

/// A three-note voicing, spelled by hand.
fn voicing() -> crate::chord::Voicing {
    crate::chord::Voicing::new(
        crate::chord::ChordClass::new(
            crate::pitch::PitchClass::parse("c").expect("`c` is a pitch class"),
            crate::chord::ChordType::Major,
        ),
        vec![pitch("c4"), pitch("e4"), pitch("g4")],
    )
    .expect("a close-position C major triad is a voicing")
}

/// Every fact `play` makes has the requested scope, an exact nonnegative
/// placement inside a track that holds it, and a complete `Origin` — which is
/// §5.7's construction clause, discharged at the one builtin that constructs.
#[test]
fn play_gives_every_fact_it_makes_the_scope_placement_and_origin_it_was_given() {
    let held = ratio(3, 8);
    let requested = Scope::Voice { part: 2, voice: 1 };
    let sounded = accepted(
        PLAY(&[
            Datum::Lit(origin_literal(provenance())),
            scope("Scope.Voice", vec![whole(2), whole(1)]),
            plain("Voicing", voicing()),
            beat(held),
        ])
        .as_ref(),
        "`play`",
    );
    assert_eq!(
        sounded.duration(),
        Duration::new(held).expect("three eighths is a duration"),
        "the track is exactly as long as the chord was asked to sound"
    );
    assert_eq!(sounded.occurrences().len(), 3, "one fact per voiced pitch");
    for occurrence in sounded.occurrences() {
        let fact = occurrence.payload();
        assert_eq!(fact.scope, requested, "the requested scope, on every fact");
        assert_eq!(
            occurrence.span().start().as_ratio(),
            ratio(0, 1),
            "placement starts at the beginning of its own track"
        );
        assert_eq!(
            occurrence.span().end().as_ratio(),
            held,
            "and ends at the length asked for"
        );
        assert_eq!(fact.origin, provenance(), "the origin it was handed, complete");
        assert!(
            matches!(fact.kind, FactKind::Note { .. }),
            "and nothing but noteheads: `play` writes no articulation and no tie"
        );
    }
    assert_eq!(
        pitches(&sounded),
        vec![pitch("c4"), pitch("e4"), pitch("g4")],
        "the voicing's pitches, in the order it voices them"
    );
    // The kernel accepting it is not incidental: `musa_kernel::track` refuses an
    // occurrence past the duration, so answering at all is the bound check.
    musa_kernel::track(sounded.duration(), sounded.occurrences().to_vec())
        .expect("what `play` answers is a track the kernel accepts");
}

// ---- D2, at the family that is not δ ----

/// Each partial track builtin refuses in its own result type rather than getting
/// stuck.
///
/// D2's rule is that a rule answers `None` only where the host's table is wrong,
/// and every one of these is handed data its signature admits. A `None` here
/// would be a `Malformed::BuiltinStuck` in front of a program that had done
/// nothing wrong.
#[test]
fn a_partial_track_builtin_refuses_in_its_result_type() {
    let mut far = pitch("c4");
    far.octave = i32::MAX;
    let unreachable = Interval {
        diatonic_steps: i64::MAX,
        semitones: i64::MAX,
    };
    let refusals = [
        (
            "`transpose` past the coordinate",
            TRANSPOSE(&[plain("Interval", unreachable), given()]),
        ),
        ("`stretch` by nothing", STRETCH(&[plain("Ratio", ratio(0, 1)), given()])),
        (
            "`invert` about an unreachable axis",
            INVERT(&[plain("Pitch", far), given()]),
        ),
        ("`shift` backwards", SHIFT(&[beat(ratio(-1, 4)), given()])),
        (
            "`play` for no time",
            PLAY(&[
                Datum::Lit(origin_literal(provenance())),
                scope("Scope.Piece", Vec::new()),
                plain("Voicing", voicing()),
                beat(ratio(0, 1)),
            ]),
        ),
    ];
    for (what, answer) in refusals {
        let Some(Datum::Case { ref constructor, .. }) = answer else {
            panic!("{what} got stuck rather than refusing: {answer:?}");
        };
        assert_eq!(&**constructor, "Result.Err", "{what} refuses in its result type");
    }
}

// ---- through the core ----

/// The literal term a rule's argument is written as.
fn term(datum: &Datum) -> Term {
    let Datum::Lit(ref value) = *datum else {
        panic!("only a literal argument has a term spelling here");
    };
    value.term(HERE)
}

/// Every track builtin is well typed at its own signature, reduces, and
/// re-checks.
///
/// The other half of this file's opening argument, and the only half
/// `map_note_pitches` has. Three steps that fail on different mistakes: the
/// application is checked at the result type the registration declares, so an
/// argument of the wrong domain is refused before the rule runs;
/// [`musa_core::normalize`] fires the rule and realizes what it answered against
/// that declared result, which is where a rule that answered the wrong shape
/// becomes a misfit; and [`musa_core::well_typed`] re-checks the normal form
/// independently of the evaluator that produced it.
#[test]
fn every_track_application_reduces_and_re_checks_at_its_own_signature() {
    let cx = owned().expect("the compiler's own context builds");
    let track = track_type();
    let fallible =
        crate::registry::applied(&cx, "Result", [track.clone(), plain_type("Text")]).expect("`Result` is declared");
    let up = Interval::parse("m3", false).expect("a minor third is an interval");
    // `fn (pitch) { c4 }` — a mapper that is a function, which is the whole
    // reason `map_note_pitches` cannot be a δ-rule.
    let constant = Term::lam(HERE, "pitch", term(&plain("Pitch", pitch("c4"))));
    let applications: Vec<(&str, Vec<Term>, &Term)> = vec![
        (
            SPELLINGS[0],
            vec![term(&plain("Interval", up)), term(&given())],
            &fallible,
        ),
        (
            SPELLINGS[1],
            vec![term(&plain("Ratio", ratio(3, 2))), term(&given())],
            &fallible,
        ),
        (SPELLINGS[2], vec![term(&given())], &track),
        (
            SPELLINGS[3],
            vec![term(&plain("Pitch", pitch("c4"))), term(&given())],
            &fallible,
        ),
        (SPELLINGS[4], vec![term(&beat(ratio(1, 4))), term(&given())], &fallible),
        (SPELLINGS[5], vec![term(&given()), term(&given())], &track),
        (SPELLINGS[6], vec![constant, term(&given())], &track),
        (
            SPELLINGS[7],
            vec![
                term(&Datum::Lit(origin_literal(provenance()))),
                crate::prelude::constant(&cx, "Scope.Piece").expect("`Scope` is declared"),
                term(&plain("Voicing", voicing())),
                term(&beat(ratio(1, 2))),
            ],
            &fallible,
        ),
    ];
    for (spelling, arguments, ty) in applications {
        let whole = arguments.into_iter().fold(head(&cx, spelling), |function, argument| {
            Term::app(HERE, function, argument)
        });
        musa_core::well_typed(&cx, ty, &whole)
            .unwrap_or_else(|why| panic!("`{spelling}` is not well typed at its own signature: {why}"));
        let normal =
            musa_core::normalize(&cx, ty, &whole).unwrap_or_else(|why| panic!("`{spelling}` does not reduce: {why}"));
        musa_core::well_typed(&cx, ty, &normal)
            .unwrap_or_else(|why| panic!("`{spelling}` reduces to something its own type refuses: {why}"));
    }
}

/// `map_note_pitches` applies the mapper exactly once per notehead and leaves
/// every other fact alone.
///
/// Through the core, because its reduction is a rewrite: there is no rule to
/// call. The mapper counts nothing and returns a constant, so "exactly once per
/// notehead" is read off the answer — two noteheads at the constant, the slur
/// untouched, and one recorded step on each notehead and none on the slur.
#[test]
fn map_note_pitches_applies_the_mapper_once_per_notehead() {
    let cx = owned().expect("the compiler's own context builds");
    let ty = track_type();
    let mapper = Term::lam(HERE, "pitch", term(&plain("Pitch", pitch("g4"))));
    let whole = Term::app(HERE, Term::app(HERE, head(&cx, SPELLINGS[6]), mapper), term(&given()));
    let normal = musa_core::normalize(&cx, &ty, &whole).expect("`map_note_pitches` reduces");
    let musa_core::Shape::Lit(ref answer) = *normal.shape() else {
        panic!("`map_note_pitches` reduced to something that is not a literal: {normal:?}");
    };
    let mapped = held::<VoiceTrack>(answer).expect("the answer is a track");
    assert_eq!(
        pitches(mapped),
        vec![pitch("g4"), pitch("g4")],
        "every notehead took the mapper's answer"
    );
    assert_eq!(
        steps(mapped),
        vec![
            Some(ExpansionStep::MapNotePitches),
            Some(ExpansionStep::MapNotePitches),
            None
        ],
        "and only the noteheads record it"
    );
    assert_eq!(
        mapped.occurrences().len(),
        subject().occurrences().len(),
        "nothing was added and nothing was dropped"
    );
    assert_eq!(mapped.duration(), subject().duration(), "and the bar is the same bar");
}
