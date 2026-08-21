//! What has to be true of the three words a notated block is built out of.
//!
//! Unit tests inside the crate, for [`crate::registry::track::laws`]'s reason:
//! `FactKind`, `ScoreFact`, and `VoiceTrack` are private to this crate, and a
//! test outside it links against `parse`/`compile`/`render_notation` and can
//! reach none of them.
//!
//! # The mirroring is stated twice, from opposite ends
//!
//! [`samples`] is a list of nineteen `(constructor, datum, kind)` rows written
//! by hand, and it is the only place in this crate where a `Fact` case and a
//! [`FactKind`] case are named together. Two laws read it from opposite ends and
//! neither trusts the list itself:
//!
//! - [`case_of`] is an exhaustive `match` over `FactKind`, so a twentieth kind
//!   added to that enum stops this file *compiling* rather than failing a
//!   comparison it could be excused from.
//! - [`declared`] reads the constructors off [`crate::prelude`]'s own
//!   declaration, so a twentieth `Fact` case with no kind behind it fails too.
//!
//! What is left between them is the field data, and [`sounded_reads_every_fact`]
//! covers that by running the rule: a field read at the wrong offset or out of
//! the wrong domain answers a different kind, or none.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use std::sync::Arc;

use musa_calculus::{Answer, Cx, Datum, Refusal, Term};
use musa_events::{Duration, Occurrence, Position, Span, WrittenTime};
use num_rational::Ratio;

use super::{BEYOND, FOLLOW, SOUNDED, nothing, opaque};
use crate::elaborate::{FactKind, ScoreFact, VoiceTrack};
use crate::registry::track::{built, track_type};
use crate::registry::{HERE, held, literal, origin_literal, owned, plain_type, tagged_type};
use musa_score::harmony::ChordSymbol;
use musa_score::marks::MarkArgument;
use musa_score::origin::{DeclarationId, Origin, SourceSpan};
use musa_score::pitch::{PitchClass, WrittenPitch};
use musa_score::scope::Scope;
use musa_score::score::{Clef, DynamicMark, FreeDuration, Metronome, Mode, NotatedDuration, Ramp};
use musa_score::time::MusicalDuration;

// ---- spelling the arguments ----

/// The origin every fact below is constructed with.
///
/// Complete in §5.7's sense — two spans and a declaration — because a law about
/// provenance that started from an incomplete origin would be measuring nothing.
fn provenance() -> Origin {
    Origin {
        source_span: SourceSpan { start: 40, end: 48 },
        definition_span: SourceSpan { start: 40, end: 48 },
        declaration: DeclarationId(7),
        expansion_path: Vec::new(),
    }
}

/// An exact rational, spelled as a fraction.
fn ratio(numerator: i64, denominator: i64) -> Ratio<i64> {
    Ratio::new(numerator, denominator)
}

/// A written pitch, by the spelling source would write.
fn pitch(spelling: &str) -> WrittenPitch {
    WrittenPitch::parse(spelling).expect("the law spells its own pitches")
}

/// A written beat, as a rule's argument.
fn beat(value: Ratio<i64>) -> Datum {
    Datum::Lit(literal(
        tagged_type("Duration", crate::core::Coordinate::WrittenTime),
        value,
    ))
}

/// A literal argument of a plain base type that has a written spelling.
fn plain<T>(name: &'static str, value: T) -> Datum
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Datum::Lit(literal(plain_type(name), value))
}

/// Text.
fn text(value: &str) -> Datum {
    plain("Text", value.to_owned())
}

/// A constructor of a declared family.
fn case(constructor: &'static str, fields: Vec<Datum>) -> Datum {
    Datum::Case {
        constructor: Arc::from(constructor),
        fields,
    }
}

/// `Nat`, as the count it is.
///
/// The shape a rule is handed, not the shape a source writes: `Nat` is a
/// counting family, so a closed `Nat` reaches a δ-rule as [`Datum::Count`] and
/// never as a tower of `Succ`s. A law that built the tower here would be
/// testing the rules against an argument the core does not produce.
fn whole(value: u64) -> Datum {
    Datum::Count {
        family: Arc::from("Nat"),
        count: value,
    }
}

/// A `List` of data.
fn list(members: Vec<Datum>) -> Datum {
    members
        .into_iter()
        .rev()
        .fold(case("List.Empty", Vec::new()), |built, member| {
            case("List.Cons", vec![member, built])
        })
}

/// `Option.Some`.
fn some(value: Datum) -> Datum {
    case("Option.Some", vec![value])
}

/// `Option.None`.
fn none() -> Datum {
    case("Option.None", Vec::new())
}

/// The first mark the vocabulary lists, which is what this file uses wherever a
/// mark is wanted.
///
/// Read off the table rather than spelled, because `notation_marks.rs` holds
/// this crate to selecting a mark by its row rather than by its name — and a law
/// about facts has no reason to care which mark one carries.
fn any_mark() -> musa_score::Mark {
    let def = musa_score::marks::VOCABULARY.first().expect("the vocabulary has rows");
    musa_score::Mark::parse(def.name).expect("a row of the vocabulary is a mark")
}

/// A quarter note's worth of notation.
fn quarter() -> NotatedDuration {
    NotatedDuration::spelled(ratio(1, 4))
}

/// A held quarter that may run to a half.
fn freedom() -> FreeDuration {
    FreeDuration {
        least: MusicalDuration::new(ratio(1, 4)),
        most: MusicalDuration::new(ratio(1, 2)),
    }
}

/// A *rit.* to 60, over two whole notes, spread evenly.
fn ramp() -> Ramp {
    Ramp {
        to: Some(60),
        over: MusicalDuration::new(ratio(2, 1)),
        shape: musa_events::Progress::linear(),
    }
}

// ---- the nineteen ----

/// Every `Fact` case: its constructor, a datum spelling it, and the
/// [`FactKind`] that datum must read back as.
///
/// Written out rather than generated, because generating it would mean writing
/// the correspondence this file exists to check as a program and then checking
/// the program against itself.
fn samples() -> Vec<(&'static str, Datum, FactKind)> {
    vec![
        (
            "Fact.Note",
            case(
                "Fact.Note",
                vec![
                    plain("Pitch", pitch("c4")),
                    opaque("NotatedDuration", quarter()),
                    list(vec![plain("Mark", any_mark())]),
                    some(opaque("FreeDuration", freedom())),
                ],
            ),
            FactKind::Note {
                pitch: pitch("c4"),
                duration: quarter(),
                articulations: vec![any_mark()],
                free: Some(freedom()),
            },
        ),
        (
            "Fact.Rest",
            case(
                "Fact.Rest",
                vec![opaque("NotatedDuration", quarter()), list(Vec::new()), none()],
            ),
            FactKind::Rest {
                duration: quarter(),
                articulations: Vec::new(),
                free: None,
            },
        ),
        (
            "Fact.Mark",
            case(
                "Fact.Mark",
                vec![
                    plain("Mark", any_mark()),
                    some(plain("MarkArgument", MarkArgument::Number(3))),
                ],
            ),
            FactKind::Mark {
                mark: any_mark(),
                argument: Some(MarkArgument::Number(3)),
            },
        ),
        (
            "Fact.Grace",
            case(
                "Fact.Grace",
                vec![
                    plain("Pitch", pitch("d5")),
                    list(vec![plain("Mark", any_mark())]),
                    whole(2),
                ],
            ),
            FactKind::Grace {
                pitch: pitch("d5"),
                articulations: vec![any_mark()],
                index: 2,
            },
        ),
        ("Fact.Slur", case("Fact.Slur", Vec::new()), FactKind::Slur),
        (
            "Fact.Phrase",
            case("Fact.Phrase", vec![text("antecedent")]),
            FactKind::Phrase {
                name: "antecedent".to_owned(),
            },
        ),
        (
            "Fact.Tuplet",
            case("Fact.Tuplet", vec![whole(3), whole(2)]),
            FactKind::Tuplet { num: 3, den: 2 },
        ),
        (
            "Fact.Dynamic",
            case("Fact.Dynamic", vec![opaque("DynamicMark", DynamicMark::Mf)]),
            FactKind::Dynamic { mark: DynamicMark::Mf },
        ),
        (
            "Fact.Hairpin",
            case(
                "Fact.Hairpin",
                vec![
                    case("Bool.True", Vec::new()),
                    opaque("DynamicMark", DynamicMark::Ff),
                    opaque("Progress", musa_events::Progress::linear()),
                ],
            ),
            FactKind::Hairpin {
                grows: true,
                target: DynamicMark::Ff,
                shape: musa_events::Progress::linear(),
            },
        ),
        (
            "Fact.Key",
            case(
                "Fact.Key",
                vec![plain(
                    "Key",
                    musa_score::Key::new(PitchClass::parse("e").expect("`e` is a pitch class"), Mode::Minor),
                )],
            ),
            FactKind::Key {
                tonic: PitchClass::parse("e").expect("`e` is a pitch class"),
                mode: Mode::Minor,
            },
        ),
        (
            "Fact.Meter",
            case("Fact.Meter", vec![whole(7), whole(8)]),
            FactKind::Meter {
                numerator: 7,
                denominator: 8,
            },
        ),
        (
            "Fact.Clef",
            case("Fact.Clef", vec![opaque("Clef", Clef::Bass)]),
            FactKind::Clef { clef: Clef::Bass },
        ),
        (
            "Fact.Tempo",
            case(
                "Fact.Tempo",
                vec![
                    some(opaque(
                        "Metronome",
                        Metronome {
                            beat: ratio(1, 4),
                            bpm: 92,
                        },
                    )),
                    some(text("Andante")),
                    some(opaque("Ramp", ramp())),
                ],
            ),
            FactKind::Tempo {
                metronome: Some(Metronome {
                    beat: ratio(1, 4),
                    bpm: 92,
                }),
                text: Some("Andante".to_owned()),
                ramp: Some(ramp()),
            },
        ),
        (
            "Fact.Section",
            case("Fact.Section", vec![text("Coda")]),
            FactKind::Section {
                name: "Coda".to_owned(),
            },
        ),
        (
            "Fact.Harmony",
            case(
                "Fact.Harmony",
                vec![opaque(
                    "ChordSymbol",
                    ChordSymbol::parse("cmaj7").expect("`cmaj7` is a chord symbol"),
                )],
            ),
            FactKind::Harmony {
                symbol: ChordSymbol::parse("cmaj7").expect("`cmaj7` is a chord symbol"),
            },
        ),
        (
            "Fact.Repeat",
            case(
                "Fact.Repeat",
                vec![whole(3), some(case("Pair.Both", vec![whole(2), whole(4)]))],
            ),
            FactKind::Repeat {
                times: 3,
                range: Some((2, 4)),
            },
        ),
        (
            "Fact.Mobile",
            case(
                "Fact.Mobile",
                vec![list(vec![text("alpha"), text("beta")]), list(vec![whole(1), whole(0)])],
            ),
            FactKind::Mobile {
                fragments: vec!["alpha".to_owned(), "beta".to_owned()],
                order: vec![1, 0],
            },
        ),
        (
            "Fact.Improvise",
            case("Fact.Improvise", vec![some(text("changes"))]),
            FactKind::Improvise {
                over: Some("changes".to_owned()),
            },
        ),
        (
            "Fact.Ending",
            case("Fact.Ending", vec![whole(2), whole(4)]),
            FactKind::Ending { bracket: 2, pass: 4 },
        ),
    ]
}

/// The `Fact` constructor that mirrors a kind.
///
/// Exhaustive on purpose and never widened with a wildcard: a twentieth
/// [`FactKind`] case has to be given a `Fact` case, and this is the line that
/// stops the crate compiling until it is.
fn case_of(kind: &FactKind) -> &'static str {
    match *kind {
        FactKind::Note { .. } => "Fact.Note",
        FactKind::Rest { .. } => "Fact.Rest",
        FactKind::Mark { .. } => "Fact.Mark",
        FactKind::Grace { .. } => "Fact.Grace",
        FactKind::Slur => "Fact.Slur",
        FactKind::Phrase { .. } => "Fact.Phrase",
        FactKind::Tuplet { .. } => "Fact.Tuplet",
        FactKind::Dynamic { .. } => "Fact.Dynamic",
        FactKind::Hairpin { .. } => "Fact.Hairpin",
        FactKind::Key { .. } => "Fact.Key",
        FactKind::Meter { .. } => "Fact.Meter",
        FactKind::Clef { .. } => "Fact.Clef",
        FactKind::Tempo { .. } => "Fact.Tempo",
        FactKind::Section { .. } => "Fact.Section",
        FactKind::Harmony { .. } => "Fact.Harmony",
        FactKind::Repeat { .. } => "Fact.Repeat",
        FactKind::Mobile { .. } => "Fact.Mobile",
        FactKind::Improvise { .. } => "Fact.Improvise",
        FactKind::Ending { .. } => "Fact.Ending",
    }
}

/// `Fact`'s constructors as [`crate::prelude`] declares them: the qualified name
/// and how many fields each writes.
fn declared() -> Vec<(String, usize)> {
    let family = crate::prelude::musical()
        .into_iter()
        .flat_map(|declaration| declaration.families)
        .find(|family| &*family.name == "Fact")
        .expect("`Fact` is one of the musical declarations");
    family
        .constructors
        .iter()
        .map(|constructor| (format!("Fact.{}", constructor.name), constructor.fields.len()))
        .collect()
}

// ---- the mirroring ----

/// `Fact` and [`FactKind`] have the same cases, walked from both ends.
#[test]
fn every_fact_case_mirrors_a_kind_and_every_kind_a_case() {
    let written = samples();
    for (constructor, _, kind) in &written {
        assert_eq!(
            case_of(kind),
            *constructor,
            "the sample's constructor is the one its kind mirrors"
        );
    }
    let mut mirrored: Vec<(String, usize)> = written
        .iter()
        .map(|(constructor, datum, _)| {
            let Datum::Case { ref fields, .. } = *datum else {
                panic!("`{constructor}` is spelled as a constructor");
            };
            ((*constructor).to_owned(), fields.len())
        })
        .collect();
    let mut declared = declared();
    mirrored.sort_unstable();
    declared.sort_unstable();
    assert_eq!(
        mirrored, declared,
        "every declared `Fact` case is mirrored by a kind, with the fields the declaration writes"
    );
}

// ---- agreement, one fact at a time ----

/// `sounded` applied to one fact for one length.
fn sound(fact: Datum, held: Ratio<i64>) -> Option<Answer> {
    SOUNDED(&[
        Datum::Lit(origin_literal(provenance())),
        case("Scope.Voice", vec![whole(1), whole(0)]),
        fact,
        beat(held),
    ])
}

/// `sounded` puts each of the nineteen facts into a track, unchanged.
///
/// The agreement half, and the one that reads the field data: the expected side
/// of every row is a [`FactKind`] built by hand out of the same values the datum
/// spells, so a field read at the wrong offset, out of the wrong domain, or in
/// the wrong order answers something this comparison names.
#[test]
fn sounded_reads_every_fact() {
    let held = ratio(3, 8);
    for (constructor, datum, expected) in samples() {
        let track = plainly(sound(datum, held).as_ref(), constructor);
        assert_eq!(
            track.occurrences().len(),
            1,
            "`{constructor}` sounds as exactly one occurrence"
        );
        let occurrence = track.occurrences().first().expect("the one occurrence is there");
        assert_eq!(occurrence.payload().kind, expected, "`{constructor}` reads back whole");
        assert_eq!(
            occurrence.span(),
            Span::new(Position::ZERO, Position::new(held)).expect("three eighths is a span"),
            "`{constructor}` covers exactly what it was given"
        );
        assert_eq!(
            track.duration(),
            Duration::new(held).expect("three eighths is a duration"),
            "and the track is exactly that long"
        );
    }
}

/// §5.7's construction clause, at the second builtin that constructs: the scope
/// and the origin are the ones handed in, on every fact, and nothing else is
/// written.
#[test]
fn sounded_gives_the_fact_the_scope_and_origin_it_was_given() {
    let track = plainly(sound(case("Fact.Slur", Vec::new()), ratio(1, 1)).as_ref(), "`sounded`");
    let fact = track
        .occurrences()
        .first()
        .expect("the one occurrence is there")
        .payload();
    assert_eq!(
        fact.scope,
        Scope::Voice { part: 1, voice: 0 },
        "the requested scope, not an ambient one"
    );
    assert_eq!(fact.origin, provenance(), "the origin it was handed, complete");
    assert!(!fact.tied, "`sounded` writes no tie: a tie is a property of two facts");
    // The event track accepting it is not incidental: `musa_events::track` refuses an
    // occurrence past the duration, so answering at all is the bound check.
    musa_events::track(track.duration(), track.occurrences().to_vec())
        .expect("what `sounded` answers is a track the event track accepts");
}

/// A fact of no length is ordinary and a fact of negative length is not.
///
/// The line this module draws differently from `play`'s, and the reason it does:
/// a mark, a grace note, and a dynamic are point occurrences, so refusing zero
/// would refuse three of the nineteen outright.
#[test]
fn sounded_admits_a_point_and_refuses_a_negative_length() {
    let point = plainly(
        sound(case("Fact.Slur", Vec::new()), ratio(0, 1)).as_ref(),
        "`sounded` at a point",
    );
    assert_eq!(
        point.duration(),
        Duration::new(ratio(0, 1)).expect("zero is a duration"),
        "a point occurrence sits in a track of no length"
    );
    assert_eq!(point.occurrences().len(), 1, "and the fact is still there");

    let answer = sound(case("Fact.Slur", Vec::new()), ratio(-1, 4));
    let Some(Answer::Refused(ref because)) = answer else {
        panic!("`sounded` got stuck on a negative length rather than refusing: {answer:?}");
    };
    assert!(
        !because.is_empty(),
        "a negative length is refused, with the sentence the rule wrote"
    );
}

/// A `Fact` this compiler never declared is not a fact.
///
/// D2's other half: a rule answers `None` where the host's table is wrong, and a
/// constructor that is not one of the nineteen can only arrive from a defect in
/// this compiler.
#[test]
fn sounded_answers_nothing_for_a_case_that_is_not_a_fact() {
    assert!(
        sound(case("Fact.Cadenza", Vec::new()), ratio(1, 4)).is_none(),
        "an undeclared constructor is not read as some other fact"
    );
    assert!(
        sound(case("Fact.Note", vec![text("c4")]), ratio(1, 4)).is_none(),
        "a field of the wrong domain is not read as a pitch"
    );
}

// ---- follow ----

/// One notehead, at `start` for `length`.
fn note(start: Ratio<i64>, length: Ratio<i64>, spelling: &str) -> Occurrence<WrittenTime, ScoreFact> {
    let end = Position::new(start).plus(Duration::new(length).expect("a length is nonnegative"));
    Occurrence::new(
        Span::new(Position::new(start), end).expect("a nonnegative length is a span"),
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

/// A one-note track of the given length.
fn bar(length: Ratio<i64>, spelling: &str) -> VoiceTrack {
    musa_events::track(
        Duration::new(length).expect("a length is nonnegative"),
        vec![note(ratio(0, 1), length, spelling)],
    )
    .expect("the notehead fits the bar it fills")
}

/// The track a rule answered, or a panic naming what came back instead.
///
/// One helper for all of them now that the family is total: what a track
/// operation answers is a track, and the two other cases are a refusal — which
/// these laws state on purpose, one test down — and nothing at all, which is
/// this compiler's table disagreeing with itself.
fn plainly(answer: Option<&Answer>, what: &str) -> VoiceTrack {
    let Some(Answer::Reduced(Datum::Lit(value))) = answer else {
        panic!("{what} answered {answer:?} rather than a track");
    };
    held::<VoiceTrack>(value).expect("the answer is a track").clone()
}

/// `follow` adds the durations and starts the second track where the first ends.
#[test]
fn following_adds_the_durations_and_places_the_second_after_the_first() {
    let first = bar(ratio(1, 2), "c4");
    let next = bar(ratio(1, 4), "e4");
    let both = plainly(FOLLOW(&[built(first), built(next)]).as_ref(), "`follow`");
    assert_eq!(
        both.duration(),
        Duration::new(ratio(3, 4)).expect("three quarters is a duration"),
        "the durations add, which is the event track's own law"
    );
    let starts: Vec<Ratio<i64>> = both
        .occurrences()
        .iter()
        .map(|occurrence| occurrence.span().start().as_ratio())
        .collect();
    assert_eq!(
        starts,
        vec![ratio(0, 1), ratio(1, 2)],
        "the second track begins where the first one ends"
    );
}

/// `nothing` is `follow`'s identity on both sides.
#[test]
fn nothing_is_the_identity_of_following() {
    let subject = bar(ratio(1, 2), "c4");
    for (what, arguments) in [
        ("`follow(nothing, t)`", [Datum::Lit(nothing()), built(subject.clone())]),
        ("`follow(t, nothing)`", [built(subject.clone()), Datum::Lit(nothing())]),
    ] {
        let answered = plainly(FOLLOW(&arguments).as_ref(), what);
        assert_eq!(answered.duration(), subject.duration(), "{what} is as long as `t`");
        assert_eq!(
            answered.occurrences(),
            subject.occurrences(),
            "{what} holds exactly what `t` holds, where `t` holds it"
        );
    }
}

/// `nothing` is the empty track and reduces to nothing else.
#[test]
fn nothing_is_the_track_of_no_duration_and_no_occurrences() {
    let empty = held::<VoiceTrack>(&nothing())
        .expect("`nothing` is a track literal")
        .clone();
    assert_eq!(
        empty.duration(),
        Duration::new(ratio(0, 1)).expect("zero is a duration")
    );
    assert!(empty.occurrences().is_empty(), "and holds no occurrence");
    assert_eq!(
        nothing().ty(),
        &track_type(),
        "at the type every signature in this module is written at"
    );
}

// ---- through the core ----

/// The head of a registered builtin, by name.
fn head(cx: &Cx, spelling: &str) -> Term {
    let (term, _) = musa_calculus::infer(cx, &musa_calculus::Raw::var(HERE, spelling)).unwrap_or_else(|refusal| {
        let unknown = matches!(refusal, musa_calculus::ElabError::Refused(Refusal::UnknownName { .. }));
        panic!("`{spelling}` is not registered (unknown name: {unknown})");
    });
    term
}

/// The literal term a rule's argument is written as.
fn term(datum: &Datum) -> Term {
    let Datum::Lit(ref value) = *datum else {
        panic!("only a literal argument has a term spelling here");
    };
    value.term(HERE)
}

/// Both registrations are well typed at their own signatures, reduce, and
/// re-check.
///
/// The shape half of the argument [`crate::registry::track::laws`] opens with. A
/// rule that answered the right value at the wrong shape — an `Option.Some`
/// where the signature says `Result`, a track literal at the wrong base type —
/// passes every comparison above and gets stuck in front of the first program
/// that uses it.
#[test]
fn every_notation_application_reduces_and_re_checks_at_its_own_signature() {
    let cx = owned().expect("the compiler's own context builds");
    let track = track_type();
    let applications: Vec<(&str, Vec<Term>, &Term)> = vec![
        (
            BEYOND[0],
            vec![
                term(&Datum::Lit(origin_literal(provenance()))),
                crate::prelude::constant(&cx, "Scope.Piece").expect("`Scope` is declared"),
                crate::prelude::constant(&cx, "Fact.Slur").expect("`Fact` is declared"),
                term(&beat(ratio(1, 2))),
            ],
            &track,
        ),
        (
            BEYOND[1],
            vec![term(&Datum::Lit(nothing())), term(&built(bar(ratio(1, 4), "c4")))],
            &track,
        ),
    ];
    for (spelling, arguments, ty) in applications {
        let whole = arguments.into_iter().fold(head(&cx, spelling), |function, argument| {
            Term::app(HERE, function, argument)
        });
        let _normal = musa_calculus::normalize(&cx, ty, &whole)
            .unwrap_or_else(|why| panic!("`{spelling}` does not reduce: {why}"));
    }
}
