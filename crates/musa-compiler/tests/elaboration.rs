//! What elaboration through the temporal kernel guarantees, stated without an
//! oracle to compare against. The direct lowerer no longer exists, so the
//! questions a differential suite once answered by comparison are answered
//! here directly:
//!
//! - **fixtures** — positions, durations, spelling, identity, multiplicity,
//!   ordering and provenance are pinned absolutely by the backend goldens
//!   (`musa-render`), the law suites, and the kernel normal forms below;
//! - **the generated corpus** — arbitrary pieces still have to elaborate, and
//!   what a random piece *means* is checkable without a second implementation:
//!   one event per written statement, and a voice as long as the durations
//!   written in it. Checking the text rather than a second path immediately
//!   found that the old generator's chord arm wrote `chord (c4 c4)` — a
//!   syntax error the comparison never noticed, because both paths rejected
//!   it identically;
//! - **error cases** — each has an absolute home (`compiler.rs` for motif
//!   order and unknown motifs, `transform_laws.rs` for double accidentals).
//!
//! Also the falsification fixtures (§33): twinkle, canon, counterpoint normal
//! forms.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// The generators do exact rational arithmetic on small literals; the workspace
// arithmetic lint has nothing to protect here.
#![allow(clippy::arithmetic_side_effects)]

use musa_compiler::{
    CompileOptions, MusicalTime, Realization, Scope, ScoreSnapshot, Severity, SourceDocument, compile,
    kernel_normal_form,
};
use num_rational::Ratio;
use proptest::prelude::*;

const COUNTERPOINT: &str = include_str!("../../../examples/counterpoint.musa");
const TWINKLE: &str = include_str!("../../../examples/twinkle.musa");
const CANON: &str = include_str!("../../../examples/canon.musa");

fn snapshot_of(text: &str) -> Option<ScoreSnapshot> {
    compile(&SourceDocument::new(text, "gen.musa"), &CompileOptions::default()).into_snapshot()
}

fn errors_of(text: &str) -> Vec<String> {
    compile(&SourceDocument::new(text, "gen.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn kernel_normal_forms_snapshot() {
    for (name, source) in [("twinkle", TWINKLE), ("canon", CANON), ("counterpoint", COUNTERPOINT)] {
        let form =
            kernel_normal_form(&SourceDocument::new(source, name), &Realization::deterministic()).expect("elaborates");
        insta::assert_snapshot!(name, form);
    }
}

/// A normal form with the provenance spans taken out.
///
/// Everything about *what a piece is* except where it happens to be written.
/// Moving a statement one line down changes both spans on it and nothing else,
/// so a test about meaning has to say so.
fn without_spans(form: &str) -> String {
    form.lines()
        .map(|line| {
            let fields: Vec<&str> = line.split('|').collect();
            if fields.len() < 7 {
                return line.to_owned();
            }
            let mut kept = fields;
            // `scope|voice|payload|duration|start|end|rest`
            kept.drain(4..6);
            kept.join("|")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Barring a passage changes where its notes are written and nothing else.
///
/// The braces are an assertion, checked and then erased: no occurrence, no
/// payload, no time of its own. A bar that changed the music would be a bar
/// that could not be added to a piece already finished, which is the whole
/// reason to add one.
#[test]
fn bars_are_erased_after_they_are_checked() {
    let piece = |body: &str| format!("piece \"b\" {{ meter 4/4; score {{ part p {{ voice v {{ {body} }} }} }} }}");
    let flat = piece("c4/4 d4/4 e4/4 f4/4 g4/2 a4/2");
    let barred = piece("bar { c4/4 d4/4 e4/4 f4/4 } bar { g4/2 a4/2 }");
    let form = |source: &str| {
        kernel_normal_form(&SourceDocument::new(source, "b"), &Realization::deterministic()).expect("elaborates")
    };
    assert_eq!(without_spans(&form(&flat)), without_spans(&form(&barred)));
}

/// A named bar sounds where it is written *and* wherever it is played, and the
/// two are the same music.
#[test]
fn a_named_bar_plays_the_same_music_it_declared() {
    let source = "piece \"b\" { meter 4/4; score { part p { voice v { \
                  bar head { c4/2 d4/2 } use head; } } } }";
    let form =
        kernel_normal_form(&SourceDocument::new(source, "b"), &Realization::deterministic()).expect("elaborates");
    let pitches: Vec<&str> = form
        .lines()
        .filter_map(|line| line.split('|').nth(2))
        .filter(|payload| payload.starts_with("note:"))
        .collect();
    assert_eq!(pitches, ["note:c4", "note:d4", "note:c4", "note:d4"]);
}

/// The key and the meter are occurrences, and the snapshot's context maps are
/// a reading of them.
#[test]
fn the_key_and_the_meter_are_facts_of_the_timeline() {
    let source = "piece \"x\" { meter 3/4; key bb major; score { part p { voice v { c4/4 } } } }";
    let form =
        kernel_normal_form(&SourceDocument::new(source, "k"), &Realization::deterministic()).expect("elaborates");
    assert!(form.contains("meter:3/4"), "meter is not an occurrence: {form}");
    assert!(form.contains("key:bb:major"), "key is not an occurrence: {form}");

    let snapshot = snapshot_of(source).expect("compiles");
    let meter = snapshot.meter_at(Scope::Piece, MusicalTime::ZERO);
    assert_eq!((meter.numerator(), meter.denominator()), (3, 4));
    assert_eq!(
        snapshot
            .key_at(Scope::Piece, MusicalTime::ZERO)
            .map(|key| key.tonic().to_string()),
        Some("bb".to_owned())
    );
}

/// A repeat sounds its unrolling, note for note.
///
/// It is not *equal* to its unrolling and no longer claims to be: a repeat
/// also states that it is one, so the page can print the body once where the
/// unrolling has to print it three times. That statement is the one extra
/// occurrence, and dropping it is what this test does — everything that
/// sounds has to agree.
#[test]
fn repeat_sounds_the_same_as_its_unrolling() {
    let repeated = "piece \"x\" { score { part p { voice v { repeat 3 { c4/4 d4/4 } } } } }";
    let unrolled = "piece \"x\" { score { part p { voice v { c4/4 d4/4 c4/4 d4/4 c4/4 d4/4 } } } }";
    let a = kernel_normal_form(&SourceDocument::new(repeated, "a"), &Realization::deterministic()).expect("elaborates");
    let b = kernel_normal_form(&SourceDocument::new(unrolled, "b"), &Realization::deterministic()).expect("elaborates");
    // Same temporal facts; provenance (and thus the snapshot) differs, which
    // is exactly the semantic quotient at work (docs/kernel/05-normalization.md).
    assert_ne!(a, b);
    // Payload heads (identity, kind, pitch) and spans agree.
    let heads_and_spans = |form: &str| -> Vec<String> {
        form.lines()
            .filter(|line| !line.contains("repeat:"))
            .map(|line| {
                let head = line.split('|').next().unwrap_or(line);
                let span = line.rsplit("from ").next().unwrap_or(line);
                format!("{head}|{span}")
            })
            .collect()
    };
    assert_eq!(heads_and_spans(&a), heads_and_spans(&b));
}

/// The repeat's own statement: one occurrence over every pass, which is what
/// lets the page print `|:` `:|` instead of three copies.
#[test]
fn a_repeat_says_on_the_timeline_that_it_is_one() {
    let source = "piece \"x\" { score { part p { voice v { repeat 3 { c4/4 d4/4 } } } } }";
    let form =
        kernel_normal_form(&SourceDocument::new(source, "a"), &Realization::deterministic()).expect("elaborates");
    assert!(
        form.contains("repeat:3"),
        "expected the repeat to state its count: {form}"
    );
    let snapshot = snapshot_of(source).expect("compiles");
    let repeats = snapshot.annotations().repeats();
    let repeat = repeats.first().expect("one repeat");
    assert_eq!(repeat.times, 3);
    assert_eq!(repeat.body_end.as_ratio(), num_rational::Ratio::new(1, 2));
    assert_eq!(repeat.end.as_ratio(), num_rational::Ratio::new(3, 2));
    assert!(repeat.endings.is_empty(), "a plain repeat has no endings");
}

/// Endings: every pass plays the body, then its own ending, and the brackets
/// the page draws are the distinct endings rather than the passes.
#[test]
fn endings_play_once_each_and_print_once_each() {
    let source = "piece \"x\" { score { part p { voice v { repeat 3 { \
                  c4/4 ending 1 { d4/4 } ending 2 { e4/4 } } } } } }";
    let snapshot = snapshot_of(source).expect("compiles");
    let voice = snapshot
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().next().map(|(_, voice)| voice.clone()))
        .expect("a voice");
    let pitches: Vec<String> = voice
        .events()
        .iter()
        .map(|event| match &event.kind {
            musa_compiler::ScoreEventKind::Note { pitch } => pitch.to_string(),
            other @ (musa_compiler::ScoreEventKind::Rest | musa_compiler::ScoreEventKind::Chord { .. }) => {
                format!("{other:?}")
            }
        })
        .collect();
    // Pass 3 has no ending of its own, so it plays the last one again.
    assert_eq!(pitches, ["c4", "d4", "c4", "e4", "c4", "e4"]);
    let repeats = snapshot.annotations().repeats();
    let repeat = repeats.first().expect("one repeat");
    let brackets: Vec<Vec<u32>> = repeat.endings.iter().map(|ending| ending.passes.clone()).collect();
    assert_eq!(brackets, [vec![1], vec![2, 3]]);
}

/// The three "error fixtures" the differential suite carried, asserted
/// against what the compiler actually says rather than against a second path.
///
/// Only one of the three was ever an error. The other two — a motif used
/// before its declaration in a piece with no `score`, and a doubly-sharp note
/// carried up two octaves and a semitone — compile cleanly, and the
/// comparison agreed on that silence without anybody noticing. The real
/// coverage for those behaviours is `compiler.rs::motifs_only_see_earlier_motifs`
/// and `transform_laws.rs::a_mirror_image_the_language_cannot_write_is_reported`;
/// what is left here is the one fixture that carried its own weight, plus the
/// two silences stated as the successes they are.
#[test]
fn the_fixtures_the_oracle_used_to_agree_about_still_say_what_they_said() {
    let unknown_motif = "piece \"x\" { score { part p { voice v { use nope(); } } } }";
    let errors = errors_of(unknown_motif);
    assert!(
        errors.iter().any(|message| message.contains("cannot find `nope`")),
        "expected the motif to be named, got {errors:?}"
    );
    assert!(
        snapshot_of(unknown_motif).is_none(),
        "a rejected piece must not produce a score"
    );

    // A motif nobody uses is not resolved, so declaration order is not
    // checked here; `compiler.rs` checks it where a `score` reaches it.
    let unused_forward_reference = "piece \"x\" { motif a() { use b(); } motif b() { c4/4 } }";
    assert!(errors_of(unused_forward_reference).is_empty());

    // `c##4` two octaves and a minor second up is `d##6` — spellable, so no
    // error. Unspellable mirrors are `transform_laws`'s.
    let stacked = "piece \"x\" { score { part p { voice v { transpose up P8 { transpose up P8 { transpose up m2 { c##4/4 } } } } } } }";
    assert!(errors_of(stacked).is_empty());
    let snapshot = snapshot_of(stacked).expect("compiles");
    let (count, _) = measured(&snapshot, 0);
    assert_eq!(count, 1);
}

// --- Generated corpus -------------------------------------------------------
//
// The strategies carry what they wrote — how many statements, and how long —
// so an arbitrary piece can be checked against its own text. That is what the
// oracle used to supply and is strictly more direct: a bug both paths shared
// was invisible to the comparison and is visible here.

/// A generated voice: its text, how many statements it spells, and how long
/// those statements last.
#[derive(Clone, Debug)]
struct GeneratedVoice {
    body: String,
    statements: usize,
    duration: Ratio<i64>,
}

fn pitch_strategy() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["c4", "d4", "e4", "f4", "g4", "a4", "b4", "c5", "eb4", "f#4"])
}

fn duration_strategy() -> impl Strategy<Value = (&'static str, Ratio<i64>)> {
    prop::sample::select(vec![
        ("1/8", Ratio::new(1, 8)),
        ("1/4", Ratio::new(1, 4)),
        ("3/8", Ratio::new(3, 8)),
        ("1/2", Ratio::new(1, 2)),
        ("3/4", Ratio::new(3, 4)),
        ("1", Ratio::new(1, 1)),
    ])
}

/// One statement: a note, a rest, or a two-pitch chord. Each spells exactly
/// one event, whatever its pitch count.
fn item_strategy() -> impl Strategy<Value = (String, Ratio<i64>)> {
    prop::sample::select(vec![0u8, 1, 2]).prop_flat_map(|kind| match kind {
        0 => (pitch_strategy(), duration_strategy())
            .prop_map(|(pitch, (text, value))| (format!("{pitch} {text} "), value))
            .boxed(),
        1 => duration_strategy()
            .prop_map(|(text, value)| (format!("rest {text} "), value))
            .boxed(),
        _ => (pitch_strategy(), pitch_strategy(), duration_strategy())
            .prop_map(|(a, b, (text, value))| (format!("[{a} {b}] {text} "), value))
            .boxed(),
    })
}

fn voice_strategy() -> impl Strategy<Value = GeneratedVoice> {
    prop::collection::vec(item_strategy(), 1..=8).prop_flat_map(|items| {
        let body: String = items.iter().map(|(text, _)| text.as_str()).collect();
        let duration: Ratio<i64> = items.iter().map(|(_, value)| *value).sum();
        let statements = items.len();
        // Optionally wrap the whole body in repeat/transpose. `repeat 2`
        // doubles both counts; `transpose` changes neither.
        prop::sample::select(vec![0u8, 1, 2]).prop_map(move |wrap| match wrap {
            0 => GeneratedVoice {
                body: body.clone(),
                statements,
                duration,
            },
            1 => GeneratedVoice {
                body: format!("repeat 2 {{ {body} }}"),
                statements: statements * 2,
                duration: duration * 2,
            },
            _ => GeneratedVoice {
                body: format!("transpose up P5 {{ {body} }}"),
                statements,
                duration,
            },
        })
    })
}

fn source_strategy() -> impl Strategy<Value = (String, GeneratedVoice, GeneratedVoice)> {
    (voice_strategy(), voice_strategy()).prop_map(|(upper, lower)| {
        let source = format!(
            "piece \"gen\" {{ tempo 1/4 = 96; meter 4/4; key c major; score {{ part p {{ voice a {{ {} }} voice b {{ {} }} }} }} }}",
            upper.body, lower.body
        );
        (source, upper, lower)
    })
}

/// A source with a motif used through repeat and transpose: three uses of a
/// two-statement motif.
fn motif_source_strategy() -> impl Strategy<Value = (String, GeneratedVoice)> {
    (pitch_strategy(), pitch_strategy(), duration_strategy()).prop_map(|(root, other, (text, value))| {
        let source = format!(
            "piece \"gen\" {{ meter 4/4; motif m(root: Pitch = c4) {{ root {text} {other} {text} }} score {{ part p {{ voice v {{ use m({root}); repeat 2 {{ transpose down P5 {{ use m(); }} }} }} }} }} }}"
        );
        let expected = GeneratedVoice {
            body: String::new(),
            statements: 6,
            duration: value * 6,
        };
        (source, expected)
    })
}

/// What the voice at `index` actually contains: how many events, and how far
/// the last one reaches.
fn measured(snapshot: &ScoreSnapshot, index: usize) -> (usize, Ratio<i64>) {
    let voice = snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice))
        .nth(index)
        .expect("the voice exists");
    let end = voice
        .events()
        .iter()
        .map(|event| (event.onset + event.notated_duration.value).as_ratio())
        .max()
        .unwrap_or_default();
    (voice.events().len(), end)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// An arbitrary piece of plain items, with optional `repeat`/`transpose`
    /// wrappers, elaborates to exactly what it wrote: one event per statement,
    /// and a voice as long as the durations in it.
    ///
    /// Bar-length warnings are expected — the generator writes any durations
    /// it likes — so only errors are refused.
    #[test]
    fn generated_sources_elaborate_to_what_they_wrote((source, upper, lower) in source_strategy()) {
        prop_assert!(errors_of(&source).is_empty(), "{source}");
        let snapshot = snapshot_of(&source).expect("compiles");
        prop_assert_eq!(measured(&snapshot, 0), (upper.statements, upper.duration));
        prop_assert_eq!(measured(&snapshot, 1), (lower.statements, lower.duration));
    }

    /// The same, through motifs with positional arguments, defaults, and
    /// nesting: expansion adds events, never time out of nowhere.
    #[test]
    fn motif_expansion_produces_what_the_uses_spell((source, expected) in motif_source_strategy()) {
        prop_assert!(errors_of(&source).is_empty(), "{source}");
        let snapshot = snapshot_of(&source).expect("compiles");
        prop_assert_eq!(measured(&snapshot, 0), (expected.statements, expected.duration));
    }
}
