//! A literal pattern matches exactly the value it spells.
//!
//! `docs/rules/language/02-core-calculus.md` §5.5 makes a pattern's meaning
//! the value it names, and §5.7 makes evaluation total — so the arm that runs
//! is decided by the value and not by which base type the value happens to
//! have. That is one law, and this suite runs it once per literal pattern the
//! checker admits, because the defect it exists to catch was a law that held
//! for six value kinds and silently failed for the seventh: a text pattern
//! never matched, the arm below it ran, and nothing was reported.
//!
//! Every law here reads the *answer*, not the verdict. The subject varies and
//! the observable does not: each piece transposes one note by the interval the
//! match chose, so `c4` means the pattern matched and `c5` means it did not.
//! A test that only asked whether the file compiled would have passed for as
//! long as the defect stood.

#![allow(clippy::expect_used, clippy::panic)]

use musa_compiler::{CompileOptions, ScoreEventKind, SourceDocument, compile};

/// The one note a piece built by [`answer`] sounds, as it is spelled.
///
/// `P1` leaves `c4` alone and `P8` takes it to `c5`, so the pitch *is* which
/// arm ran.
fn answer(annotation: &str, subject: &str, pattern: &str) -> String {
    let source = SourceDocument::new(
        format!(
            "piece \"Literal patterns\" {{\n\
             \x20   let subject: {annotation} = {subject};\n\
             \x20   let chosen: Interval = match subject {{ {pattern} -> P1, _ -> P8 }};\n\
             \x20   let tune: EventTrack<WrittenTime> = transpose(chosen, music {{ c4/1 }});\n\n\
             \x20   tempo 1/4 = 84;\n\
             \x20   meter 4/4;\n\n\
             \x20   score {{ part p {{ voice v {{ use tune; }} }} }}\n\
             }}\n"
        ),
        "literal-patterns.musa",
    );
    let compilation = compile(&source, &CompileOptions::default());
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

/// Every literal pattern the lowering admits: the annotation to write, the
/// subject the arm should match, a subject it should not, and the pattern
/// itself.
///
/// **A literal has one type**, and this table used to say otherwise. Two rows
/// stood here reading `2` and `3/8` at `Duration<WrittenTime>`, on the replaced
/// checker's rule that an integer and a rational each stood at two types. The
/// core `02-core-calculus.md` §2 fixes has no such rule — "a projection, a
/// variable, and a literal infer", so a rational infers `Ratio` and `Switch`
/// asks for `Ratio ≡ Duration WrittenTime`, which is false. A duration is
/// written `duration_of(3/8)` and matched by matching what it is *made of*:
/// `match duration_ratio(held) { 3/8 -> …, _ -> … }`. That is one call, and it
/// is the alternative to return-type-directed literal overloading, which
/// `01-surface.md` §1.6's table refuses by name.
///
/// `Bool` and `Nat` stay because they are *declared* families: `true` and `7`
/// are constructor patterns split by the case tree, not equality tests. The
/// other four are the base types a token spells — and the four the prelude's
/// `Eq` instances answer for.
const ADMITTED: [(&str, &str, &str, &str); 6] = [
    ("Bool", "true", "false", "true"),
    ("Nat", "7", "8", "7"),
    ("Text", "\"staff\"", "\"studio\"", "\"staff\""),
    ("Ratio", "3/8", "5/8", "3/8"),
    ("Pitch", "e5", "f5", "e5"),
    ("Interval", "M3", "m3", "M3"),
];

#[test]
fn a_literal_pattern_matches_the_value_it_spells() {
    for (annotation, subject, _, pattern) in ADMITTED {
        assert_eq!(
            answer(annotation, subject, pattern),
            "c4",
            "`{pattern}` at `{annotation}` did not match `{subject}`, so the arm below it ran"
        );
    }
}

#[test]
fn a_literal_pattern_matches_nothing_else() {
    for (annotation, _, other, pattern) in ADMITTED {
        assert_eq!(
            answer(annotation, other, pattern),
            "c5",
            "`{pattern}` at `{annotation}` matched `{other}`, which is a different value"
        );
    }
}

/// A duration is matched by matching the rational it is made of.
///
/// The positive half of the two rows [`ADMITTED`] lost. Nothing about durations
/// became unaskable when the literal stopped standing at two types: the question
/// "is this an eighth" is `duration_ratio(held) == 3/8`, and a `match` over it
/// is the same three lines with one call in front. What went away is the
/// coincidence that made `3/8` mean two things depending on where it was read.
#[test]
fn a_duration_is_matched_through_the_ratio_it_is_made_of() {
    let source = SourceDocument::new(
        "piece \"Durations\" {\n\
         \x20   let held: Duration<WrittenTime> = duration_of(3/8);\n\
         \x20   let chosen: Interval = match duration_ratio(held) { 3/8 -> P1, _ -> P8 };\n\
         \x20   let tune: EventTrack<WrittenTime> = transpose(chosen, music { c4/1 });\n\n\
         \x20   score { part p { voice v { use tune; } } }\n\
         }\n"
        .to_owned(),
        "durations.musa",
    );
    let compilation = compile(&source, &CompileOptions::default());
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

/// The law the defect broke, written on its own so that a regression names
/// itself.
///
/// Text is the kind the comparison forgot. It is also the kind an adapter is
/// handed — a token's kind and its text both arrive as `Text` — so a text
/// pattern that falls through is not a corner of the language but the whole
/// of what reading a region depends on.
#[test]
fn a_text_pattern_is_not_a_hole_in_the_language() {
    assert_eq!(answer("Text", "\"staff\"", "\"staff\""), "c4");
    assert_eq!(answer("Text", "\"\"", "\"\""), "c4", "the empty text is a text");
    assert_eq!(
        answer("Text", "\"staff \"", "\"staff\""),
        "c5",
        "two texts are one text only when they are the same bytes"
    );
}
