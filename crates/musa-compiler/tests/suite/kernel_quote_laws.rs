//! What a kernel quote promises, and what it does not (prompt 121).
//!
//! `kernel EventTrack[WrittenTime, ScoreFact] { … }` is the assembly-level escape: the
//! composer writes the term the compiler would otherwise write for them, and
//! `${e}` splices ordinary Musa music into it. That is a lot of rope, so the
//! promises have to be exact:
//!
//! ```text
//! kept:   payload typing, closure, hygiene, and exact time — the extent and
//!         every onset are read off the term's structure, in rationals
//! given:  bar alignment, scale membership, chord realization — anything
//!         about *where* facts land, because raw placement is the freedom to
//!         land them elsewhere
//! ```
//!
//! The laws below hold both halves. The second half is the one worth writing
//! down: a test suite that only proved the good news would leave a composer
//! believing a quote is safe, and the whole point of an escape hatch is that
//! it is not.
//!
//! The locus law is a reference model rather than a golden: `docs/rules/language`
//! §7 states the quotation locus compositionally — `seq` adds prefix extents,
//! `over` preserves, `shift` translates, a positive `scale` scales the
//! relative offset, `restrict` relocates nothing, and a `let` value begins at
//! its enclosing locus — and each case below is one clause of that statement,
//! read back off the onset of the note the hole put there.

// A failure of these `expect`s is a bug in this file's fixtures.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// The failure messages name which fixture broke, which a bare `expect` cannot.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ScoreEventKind;
use num_rational::Ratio;

/// A piece whose one voice uses `body` as its only material.
fn piece(body: &str) -> String {
    format!(
        "piece \"Q\" {{\n\
         let subject: EventTrack<WrittenTime> = music {{ c4/4 d4/4 e4/4 f4/4 }};\n\
         let assembled: EventTrack<WrittenTime> = {body};\n\
         score {{ part p {{ voice v {{ use assembled; }} }} }}\n\
         }}\n"
    )
}

/// Compile, refusing to continue if anything was reported as an error.
fn compiled(source: &str) -> musa_compiler::Compilation {
    let compilation = compile(&SourceDocument::new(source, "quote.musa"), &CompileOptions::default());
    let errors: Vec<String> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    compilation
}

/// Every note of the compiled piece's one voice: onset, written value, and
/// the pitch as the source spells it.
fn notes(source: &str) -> Vec<(Ratio<i64>, Ratio<i64>, String)> {
    let compilation = compiled(source);
    let snapshot = compilation.snapshot().expect("a score");
    let mut out = Vec::new();
    for (_, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                if let ScoreEventKind::Note { pitch, .. } = &event.kind {
                    out.push((
                        event.onset.as_ratio(),
                        event.notated_duration.value.as_ratio(),
                        pitch.to_string(),
                    ));
                }
            }
        }
    }
    out.sort_by(|left, right| left.partial_cmp(right).expect("total order on rationals"));
    out
}

/// The locus this fact's expansion path records, if a quote put it there.
///
/// Spelled out rather than wildcarded: a step added later is a step this
/// reader has to be taught about, and the compiler saying so is the point of
/// the workspace's lint.
fn splice_locus(origin: &musa_score::Origin) -> Option<Ratio<i64>> {
    origin.expansion_path.iter().find_map(|step| match step {
        musa_score::ExpansionStep::KernelSplice { at } => Some(*at),
        musa_score::ExpansionStep::MotifApplication { .. }
        | musa_score::ExpansionStep::RepeatIteration(_)
        | musa_score::ExpansionStep::Transposition(_)
        | musa_score::ExpansionStep::Stretch(_)
        | musa_score::ExpansionStep::Retrograde
        | musa_score::ExpansionStep::Inversion { .. }
        | musa_score::ExpansionStep::MapNotePitches
        | musa_score::ExpansionStep::ScaleContext { .. }
        | musa_score::ExpansionStep::TemplateInstance { .. }
        | musa_score::ExpansionStep::Assertion { .. }
        | musa_score::ExpansionStep::Specialization { .. } => None,
    })
}

/// Every error a source reports, as messages.
fn errors(source: &str) -> Vec<String> {
    compile(&SourceDocument::new(source, "quote.musa"), &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

/// The splice locus a term records, and where the hole's first note landed.
///
/// The two are different questions and the difference is the point: the locus
/// is where the hole was *instantiated*, and the onset is where a reference
/// put the facts. They agree everywhere except under a `let`, where one
/// instantiation can be placed in several places at once.
fn spliced(term: &str) -> (Ratio<i64>, Ratio<i64>) {
    let source = piece(&format!("kernel EventTrack[WrittenTime, ScoreFact] {term}"));
    let Some(&(onset, _, _)) = notes(&source).iter().find(|(_, _, pitch)| pitch == "c4") else {
        panic!("the hole put no c4 anywhere in {term}");
    };
    let compilation = compiled(&source);
    let snapshot = compilation.snapshot().expect("a score");
    let locus = snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .filter(|event| matches!(&event.kind, ScoreEventKind::Note { pitch, .. } if pitch.to_string() == "c4"))
        .find_map(|event| splice_locus(&event.origin))
        .unwrap_or_else(|| panic!("the hole's material recorded no locus in {term}"));
    (locus, onset)
}

/// The clause-by-clause statement of the quotation locus.
///
/// Each case is one line of `docs/rules/language/01-surface.md` §7, and the value on
/// the right is computed here rather than recorded: `subject` is four quarter
/// notes, so its extent is 1 and its first note is at 0, and every number
/// below follows from that by the rule the case is named after.
#[test]
fn the_quotation_locus_follows_the_terms_structure() {
    // `over` preserves: every branch begins where the overlay does.
    assert_eq!(
        spliced("{ together { ${subject}; } }"),
        (Ratio::new(0, 1), Ratio::new(0, 1))
    );

    // `seq` adds the exact extents of everything before it. The leading
    // half-note gap is written as an empty raw timeline so nothing but the
    // term decides the offset.
    assert_eq!(
        spliced("{ follow { track 1/2 { }; ${subject}; } }"),
        (Ratio::new(1, 2), Ratio::new(1, 2)),
    );

    // `shift` translates.
    assert_eq!(
        spliced("{ shift by 3/4 ${subject} }"),
        (Ratio::new(3, 4), Ratio::new(3, 4))
    );

    // A positive `scale` scales the *relative* offset, and only that: the
    // hole sits 1/2 into a body whose time is then halved.
    assert_eq!(
        spliced("{ scale by 1/2 follow { track 1/2 { }; ${subject}; } }"),
        (Ratio::new(1, 4), Ratio::new(1, 4)),
    );

    // `restrict` relocates nothing: observing a window is not moving what is
    // seen through it.
    assert_eq!(
        spliced("{ restrict from 0 to 2 shift by 1/2 ${subject} }"),
        (Ratio::new(1, 2), Ratio::new(1, 2)),
    );

    // A `let` value begins where its `let` does, and *that* is the locus —
    // the material is instantiated once, there, and the reference below then
    // places the finished facts half a note later. One instantiation, one
    // locus, and as many placements as the term writes references.
    assert_eq!(
        spliced("{ shift by 1/4 let m = ${subject} in shift by 1/2 m }"),
        (Ratio::new(1, 4), Ratio::new(3, 4)),
    );
}

/// A hole cannot be captured, whatever the quote calls its own bindings.
///
/// The fresh name is chosen so that it does not occur in the quote's text at
/// all, so a quote that binds `splice0` — the obvious first guess, and the
/// name a composer reading a printed term would reach for — is not naming the
/// hole. Both bindings survive, and they mean different things.
#[test]
fn a_hole_cannot_be_captured_by_a_name_the_quote_binds() {
    let notes = notes(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            let splice0 = track 1 { occurrence \"note g2 1\" from 0 to 1; } in
            together {
                splice0;
                ${subject};
            }
        }",
    ));
    let pitches: Vec<&str> = notes.iter().map(|(_, _, pitch)| pitch.as_str()).collect();
    assert!(
        pitches.contains(&"g2"),
        "the quote's own `splice0` was overwritten: {notes:?}",
    );
    assert!(
        pitches.contains(&"c4") && pitches.contains(&"f4"),
        "the hole did not survive the collision: {notes:?}",
    );
}

/// A quote stands on its own: every name it uses is one it binds.
#[test]
fn a_quote_with_a_free_name_is_refused() {
    let reported = errors(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] { together { subject; } }",
    ));
    assert!(
        reported
            .iter()
            .any(|message| message.contains("does not stand on its own")),
        "a free name was accepted: {reported:?}",
    );
}

/// Shadowing is capture seen from the inside, and K7 refuses it.
#[test]
fn a_quote_that_rebinds_a_name_is_refused() {
    let reported = errors(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            let m = ${subject} in
            let m = track 1 { occurrence \"note g2 1\" from 0 to 1; } in
            together { m; }
        }",
    ));
    assert!(
        reported
            .iter()
            .any(|message| message.contains("does not stand on its own")),
        "a shadowed name was accepted: {reported:?}",
    );
}

/// The payload type is checked before anything is read, and this build owns
/// exactly one.
#[test]
fn a_payload_this_build_does_not_own_is_refused() {
    let reported = errors(&piece("kernel EventTrack[WrittenTime, Sample] { track 1 { } }"));
    assert!(
        reported.iter().any(|message| message.contains("`Sample` payloads")),
        "an unknown payload was accepted: {reported:?}",
    );
}

/// The coordinate is checked the same way, and before the payload.
///
/// A quote in performed time is not a score whatever its payloads say —
/// nothing converts one coordinate into another, so the mistake has to be
/// caught where it is written rather than absorbed by a conversion that does
/// not exist.
#[test]
fn a_quote_in_another_coordinate_is_refused() {
    let reported = errors(&piece("kernel EventTrack[PerformedTime, ScoreFact] { track 1 { } }"));
    assert!(
        reported
            .iter()
            .any(|message| message.contains("cannot be written in `PerformedTime`")),
        "a performed-time quote was accepted as a score: {reported:?}",
    );
}

/// A quote may not settle what its use is entitled to settle.
///
/// Two refusals with one shape: a context-authoritative fact would change the
/// caller's key or meter from inside a value, and a scope word would choose
/// the voice the material lands in. Both are the use's answer.
#[test]
fn a_quote_may_not_carry_context_or_choose_a_voice() {
    for (raw, expected) in [
        ("occurrence \"key c major\" from 0 to 1;", "a key"),
        ("occurrence \"meter 4/4\" from 0 to 1;", "a meter"),
        (
            "occurrence \"voice 0 0 note c4 1 [0:0]\" from 0 to 1;",
            "a voice of its own",
        ),
    ] {
        let reported = errors(&piece(&format!(
            "kernel EventTrack[WrittenTime, ScoreFact] {{ track 1 {{ {raw} }} }}"
        )));
        assert!(
            reported.iter().any(|message| message.contains(expected)),
            "{raw} was accepted: {reported:?}",
        );
    }
}

/// A hole is instantiated once, and every reference places the same facts.
///
/// The evidence is that the two placements are the *same* notes moved rather
/// than two readings of `subject`: they agree note for note under the shift,
/// which a re-elaboration would also do, and they agree on their definition
/// spans, which a re-elaboration under a different context need not.
#[test]
fn a_hole_is_instantiated_once_and_its_facts_are_frozen() {
    let source = piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            let m = ${subject} in
            together { m; shift by 1 m; }
        }",
    );
    let notes = notes(&source);
    let first: Vec<(Ratio<i64>, Ratio<i64>, String)> = notes
        .iter()
        .filter(|(onset, _, _)| *onset < Ratio::new(1, 1))
        .cloned()
        .collect();
    let second: Vec<(Ratio<i64>, Ratio<i64>, String)> = notes
        .iter()
        .filter(|(onset, _, _)| *onset >= Ratio::new(1, 1))
        .map(|(onset, value, pitch)| (onset - Ratio::new(1, 1), *value, pitch.clone()))
        .collect();
    assert_eq!(first, second, "the two references are not the same material");

    let compilation = compiled(&source);
    let snapshot = compilation.snapshot().expect("a score");
    let mut definitions: Vec<(u32, u32)> = snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events())
        .map(|event| (event.origin.definition_span.start, event.origin.definition_span.end))
        .collect();
    definitions.sort_unstable();
    definitions.dedup();
    assert_eq!(
        definitions.len(),
        4,
        "eight notes from four written ones came from {} places",
        definitions.len(),
    );
}

/// Every fact that leaves a quote says so, and says where it was assembled.
///
/// Raw material is the quote's own writing, at the quote's own zero; material
/// that arrived through a hole records the hole's locus. Without this a note
/// with no surface statement behind it would have no answer to "why is this
/// here", which is the question Origin exists to answer.
#[test]
fn every_spliced_fact_records_its_locus() {
    let compilation = compiled(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            together {
                shift by 1/2 ${subject};
                track 2 { occurrence \"note g2 1\" from 0 to 1; };
            }
        }",
    ));
    let snapshot = compilation.snapshot().expect("a score");
    let mut seen: Vec<(String, Ratio<i64>)> = Vec::new();
    for (_, part) in snapshot.parts().iter() {
        for (_, voice) in part.voices() {
            for event in voice.events() {
                let ScoreEventKind::Note { pitch, .. } = &event.kind else {
                    continue;
                };
                let Some(locus) = splice_locus(&event.origin) else {
                    panic!("{pitch} left a quote without a splice step");
                };
                seen.push((pitch.to_string(), locus));
            }
        }
    }
    assert!(
        seen.iter()
            .any(|(pitch, locus)| pitch == "g2" && *locus == Ratio::new(0, 1)),
        "raw material was not assembled at the quote's own zero: {seen:?}",
    );
    assert!(
        seen.iter()
            .any(|(pitch, locus)| pitch == "c4" && *locus == Ratio::new(1, 2)),
        "the hole's material did not record the hole's locus: {seen:?}",
    );
}

/// A quote denotes what the same term built out of surface constructors does.
///
/// The kernel's `overlay` and `shift` and the surface's are the same two
/// operations — the quote is a second spelling of the term, not a second
/// semantics. Provenance differs by exactly the splice step, so what is
/// compared is the music: onset, written value, and spelling, note for note.
#[test]
fn a_quote_agrees_with_the_term_written_in_the_surface() {
    let quoted = notes(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            let m = ${subject} in
            together { m; shift by 1 m; }
        }",
    ));
    let surfaced = notes(&piece("together(subject, shift(duration_of(1/1), subject))"));
    assert_eq!(quoted, surfaced, "the two spellings of one term differ");
}

/// Raw `scale` is a *time* operation, and renotates nothing.
///
/// This is the sharpest of the bypasses and the easiest to walk into, so it
/// is written down rather than left to be discovered. `scale by 1/2` halves
/// every span; it does not touch the payloads, because payloads are opaque to
/// the kernel and arrive already transformed (`docs/rules/kernel/01-grammar.md`).
/// The surface `stretch` is the operation that does both, which is why
/// augmentation belongs in the host — `${stretch(1/2, subject)}` — and raw
/// `scale` belongs to material whose written values already say what the
/// composer meant.
#[test]
fn raw_scale_moves_facts_without_renotating_them() {
    let scaled = notes(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] { scale by 1/2 ${subject} }",
    ));
    let stretched = notes(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] { ${stretch(1/2, subject)} }",
    ));
    let onsets: Vec<Ratio<i64>> = scaled.iter().map(|(onset, _, _)| *onset).collect();
    assert_eq!(
        onsets,
        stretched.iter().map(|(onset, _, _)| *onset).collect::<Vec<_>>(),
        "the two disagree about time, which is the half raw scale does do",
    );
    assert!(
        scaled.iter().all(|(_, value, _)| *value == Ratio::new(1, 4)),
        "raw scale renotated something: {scaled:?}",
    );
    assert!(
        stretched.iter().all(|(_, value, _)| *value == Ratio::new(1, 8)),
        "the host-side stretch did not renotate: {stretched:?}",
    );
}

/// Exact time survives the escape, in rationals.
///
/// The extent is read off the term's structure — `over` takes the longest
/// branch, `shift` translates it, `scale` multiplies it — and none of that
/// goes through a float. A third of a whole note stays a third.
#[test]
fn a_quotes_time_stays_exact() {
    let notes = notes(&piece(
        "kernel EventTrack[WrittenTime, ScoreFact] {
            scale by 1/3 follow { track 1/2 { }; ${subject}; }
        }",
    ));
    let first = notes.first().expect("a note").0;
    assert_eq!(first, Ratio::new(1, 6), "1/2 scaled by 1/3 is not 1/6");
    let last = notes.last().expect("a note").0;
    assert_eq!(last, Ratio::new(5, 12), "5/4 scaled by 1/3 is not 5/12");
}

/// What a quote gives up: an assertion that passed inside a hole says nothing
/// about the facts after a raw transform has moved them.
///
/// `subject` fills one measure of 4/4 and can be asserted to. Halve it inside
/// a quote and the same claim, made about the result, fails — not because the
/// assertion is unsound but because it is about *placement*, and placement is
/// precisely what raw assembly is free to change. This is the bypass, written
/// down as a test so that it is a documented property rather than a surprise.
#[test]
fn a_raw_transform_can_invalidate_a_placement_claim() {
    let inside = errors(
        "piece \"Q\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 e4/4 f4/4 };
            score { part p { voice v { assert fills_meter() { use subject; } } } }
        }",
    );
    assert!(inside.is_empty(), "the claim should hold of the subject: {inside:?}");

    let outside = errors(
        "piece \"Q\" {
            let subject: EventTrack<WrittenTime> = music { c4/4 d4/4 e4/4 f4/4 };
            let halved: EventTrack<WrittenTime> = kernel EventTrack[WrittenTime, ScoreFact] { scale by 1/2 ${subject} };
            score { part p { voice v { assert fills_meter() { use halved; } } } }
        }",
    );
    assert!(!outside.is_empty(), "a half measure passed a claim to fill a whole one");
}
