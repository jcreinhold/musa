//! What an `assert` is and is not (docs/prompts/116).
//!
//! `docs/language/05-verification.md` puts explicit assertions in the middle
//! of three strengths: decidable, composer-requested, blocking. Everything
//! this suite pins down follows from that one sentence.
//!
//! - **Identity on success.** A claim that holds gives back exactly the
//!   passage it was written on: the same notes, at the same times, written the
//!   same way, running the same length. The only difference is one
//!   [`ExpansionStep::Assertion`] on the provenance of the notes underneath,
//!   which is Origin and therefore invisible to `≈facts` (law 13).
//! - **Exact witnesses.** A claim that fails names the claim, the smallest
//!   piece of music that refutes it, and both spans — the assertion's and the
//!   offending material's, which are not the same place when the material came
//!   from a motif.
//! - **Per-context rechecking.** The claim is about the passage *as
//!   instantiated*. The same motif under two transpositions is two passages
//!   and gets two verdicts.
//! - **Parity.** Handwritten music and generated music are checked by the same
//!   rule, because by the time the claim is checked there is no difference
//!   between them left to see.
//! - **Nothing becomes a style rule.** Notes outside the scale, chords that
//!   realize nothing, and five-note sonorities are all silent unless someone
//!   wrote an assertion about them. This is the boundary prompt 83 drew
//!   between an error and a lint, and assertions do not move it.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
// Exact rational time through the `MusicalTime` operators, which are total
// for musa's magnitudes (`crates/musa-compiler/src/time.rs`).
#![allow(clippy::arithmetic_side_effects)]
// This suite picks `Assertion` out of `ExpansionStep` and ignores the rest.
// The lint exists so a new step is considered where it matters, and "which of
// these are assertions" is not one of those places.
#![allow(clippy::wildcard_enum_match_arm)]

use musa_compiler::{
    Compilation, CompileOptions, Diagnostic, ExpansionStep, MusicalDuration, MusicalTime, NotatedDuration, ScoreEvent,
    ScoreEventKind, Severity, SourceDocument, compile,
};

const NAME: &str = "assertion.musa";

/// A piece with one voice, in 4/4, with a motif `figure` to use.
fn piece(body: &str) -> String {
    format!(
        "piece \"assert\" {{\n\
         meter 4/4;\n\
         motif figure() {{ c5/4 e5/4 g5/4 c6/4 }}\n\
         score {{ part p {{ voice v {{ {body} }} }} }}\n\
         }}"
    )
}

fn compilation(body: &str) -> Compilation {
    compile(
        &SourceDocument::new(piece(body).as_str(), NAME),
        &CompileOptions::default(),
    )
}

fn errors(body: &str) -> Vec<Diagnostic> {
    compilation(body)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .cloned()
        .collect()
}

/// The one error `body` produces, or a panic naming what came instead.
fn only_error(body: &str) -> Diagnostic {
    let found = errors(body);
    match found.len() {
        1 => found.into_iter().next().expect("one error"),
        _ => panic!(
            "expected exactly one error from `{body}`; got {:?}",
            found.iter().map(|diagnostic| &diagnostic.message).collect::<Vec<_>>()
        ),
    }
}

/// Every event of the piece's one voice, in score order.
fn events(body: &str) -> Vec<ScoreEvent> {
    let compiled = compilation(body);
    let messages: Vec<&str> = compiled
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    assert!(
        !compiled.diagnostics().iter().any(|d| d.severity == Severity::Error),
        "`{body}` did not compile: {messages:?}"
    );
    compiled
        .into_snapshot()
        .expect("a snapshot")
        .parts()
        .iter()
        .next()
        .and_then(|(_, part)| part.voices().map(|(_, voice)| voice).next())
        .map(|voice| voice.events().to_vec())
        .unwrap_or_default()
}

/// What sounds, when, written how. Origin is deliberately not in it: this is
/// the `≈facts` view, and provenance is what an assertion is allowed to
/// change.
type Fact = (MusicalTime, NotatedDuration, ScoreEventKind);

fn fact(event: &ScoreEvent) -> Fact {
    (event.onset, event.notated_duration.clone(), event.kind.clone())
}

/// Where the voice's music ends: the latest point anything is still sounding.
fn extent(body: &str) -> MusicalDuration {
    events(body)
        .iter()
        .map(|event| MusicalDuration::new(event.onset.as_ratio()) + event.notated_duration.value)
        .max()
        .unwrap_or_default()
}

/// The passage every claim below is written about: one 4/4 bar of C major
/// triad tones, ending on the octave.
const PASSAGE: &str = "c5/4 e5/4 g5/4 c6/4";

/// Every claim that holds of [`PASSAGE`], one per family member and one per
/// realization policy.
const HOLDS: [&str; 6] = [
    "fills_meter()",
    "pitches_in(scale c major)",
    "realizes(chord c major, exactly)",
    "realizes(chord c major, may_omit)",
    "realizes(chord c major, may_add)",
    "voices(1)",
];

// --- Identity on success -----------------------------------------------

#[test]
fn a_claim_that_holds_changes_nothing_about_the_music() {
    let plain = events(PASSAGE).iter().map(fact).collect::<Vec<_>>();
    let plain_extent = extent(PASSAGE);
    for claim in HOLDS {
        let asserted = format!("assert {claim} {{ {PASSAGE} }}");
        assert_eq!(
            events(&asserted).iter().map(fact).collect::<Vec<_>>(),
            plain,
            "`{claim}` changed the facts"
        );
        assert_eq!(extent(&asserted), plain_extent, "`{claim}` changed the extent");
    }
}

#[test]
fn nesting_claims_is_still_the_same_music() {
    // Six claims about one passage, each inside the last. The music is the
    // passage; the claims are six lines of provenance.
    let mut nested = PASSAGE.to_owned();
    for claim in HOLDS {
        nested = format!("assert {claim} {{ {nested} }}");
    }
    assert_eq!(
        events(&nested).iter().map(fact).collect::<Vec<_>>(),
        events(PASSAGE).iter().map(fact).collect::<Vec<_>>()
    );
    assert_eq!(extent(&nested), extent(PASSAGE));
}

// --- Provenance ---------------------------------------------------------

/// The claims recorded on an event's expansion path, innermost last.
fn claims_on(event: &ScoreEvent) -> Vec<&str> {
    event
        .origin
        .expansion_path
        .iter()
        .filter_map(|step| match step {
            ExpansionStep::Assertion { claim } => Some(claim.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_claim_is_recorded_on_the_notes_it_covers_with_its_arguments() {
    let asserted = events(&format!("assert pitches_in(scale c major) {{ {PASSAGE} }}"));
    for event in &asserted {
        assert_eq!(
            claims_on(event),
            ["pitches_in(scale c major)"],
            "the arguments belong in the step: `pitches_in` alone is not actionable"
        );
    }
    // And nothing outside the braces carries it.
    let mixed = events(&format!("assert voices(1) {{ c5/4 }} {PASSAGE}"));
    let carried: Vec<usize> = mixed
        .iter()
        .enumerate()
        .filter(|(_, event)| !claims_on(event).is_empty())
        .map(|(at, _)| at)
        .collect();
    assert_eq!(carried, [0], "only the note inside the braces is under the claim");
}

#[test]
fn nested_claims_are_recorded_outermost_first() {
    let asserted = events(&format!("assert fills_meter() {{ assert voices(1) {{ {PASSAGE} }} }}"));
    for event in &asserted {
        assert_eq!(claims_on(event), ["fills_meter()", "voices(1)"]);
    }
}

// --- Exact witnesses ----------------------------------------------------

/// The text of `diagnostic`'s primary label's span in `body`'s source.
fn witness(diagnostic: &Diagnostic, body: &str) -> String {
    let source = piece(body);
    let label = diagnostic
        .labels
        .iter()
        .find(|label| label.primary)
        .expect("a primary label");
    source[label.span.start as usize..label.span.end as usize].to_owned()
}

#[test]
fn a_stray_pitch_is_named_and_never_respelled() {
    let body = "assert pitches_in(scale c major) { c5/4 e5/4 f#5/4 c6/4 }";
    let diagnostic = only_error(body);
    assert_eq!(diagnostic.message, "`f#5` is not in scale c major");
    // The span is the note statement: its duration is part of what was
    // written, and there is no smaller thing to point at.
    assert_eq!(witness(&diagnostic, body), "f#5/4");
    // The whole content of the claim: no enharmonic reading, no repair.
    assert!(!diagnostic.message.contains("gb"), "{}", diagnostic.message);
    assert!(diagnostic.fixes.is_empty(), "an assertion never repairs the source");
}

#[test]
fn the_first_stray_pitch_is_the_only_one_reported() {
    // Four notes out of the scale is one mistake with four instances, and a
    // composer fixes the first one first.
    let diagnostic = only_error("assert pitches_in(scale c major) { c#5/4 d#5/4 f#5/4 g#5/4 }");
    assert_eq!(diagnostic.message, "`c#5` is not in scale c major");
}

#[test]
fn a_short_passage_says_how_short_in_the_bars_own_words() {
    // Prompt 57's sentence, with the noun the page justifies: this is not a
    // bar, so it does not call itself one.
    let diagnostic = only_error("assert fills_meter() { c5/4 e5/4 }");
    assert_eq!(diagnostic.message, "this passage is 1/2 short");
    let bar = only_error("bar { c5/4 e5/4 }");
    assert_eq!(bar.message, "this bar is 1/2 short");
    assert_eq!(bar.code, diagnostic.code, "one obligation, one code");
}

#[test]
fn a_chord_claim_names_the_note_that_does_not_belong() {
    let body = "assert realizes(chord c major, exactly) { c5/4 e5/4 d5/4 g5/4 }";
    let diagnostic = only_error(body);
    assert_eq!(diagnostic.message, "`d5` is not a member of `chord c major`");
    assert_eq!(witness(&diagnostic, body), "d5/4");
}

#[test]
fn a_chord_claim_names_the_member_that_never_sounds() {
    let diagnostic = only_error("assert realizes(chord c major, may_add) { c5/4 d5/4 g5/4 }");
    assert_eq!(diagnostic.message, "`chord c major` is missing its `e`");
}

#[test]
fn the_policies_are_three_different_claims() {
    // One passage, three policies, three verdicts: the third of the chord is
    // absent and a D is present.
    let passage = "[c4 d4 g4]/1";
    let refused = |policy: &str| errors(&format!("assert realizes(chord c major, {policy}) {{ {passage} }}")).len();
    assert_eq!(refused("exactly"), 1, "the added D refutes set equality");
    assert_eq!(refused("may_omit"), 1, "an omission is allowed; an addition is not");
    assert_eq!(
        refused("may_add"),
        1,
        "an addition is allowed; the missing third is not"
    );
    // And each is satisfiable on its own terms.
    assert!(errors("assert realizes(chord c major, exactly) { [c4 e4 g4]/1 }").is_empty());
    assert!(errors("assert realizes(chord c major, may_omit) { [c4 e4]/1 }").is_empty());
    assert!(errors("assert realizes(chord c major, may_add) { [c4 d4 e4 g4]/1 }").is_empty());
}

#[test]
fn a_wrong_voice_count_says_where_and_how_many() {
    let diagnostic = only_error("assert voices(4) { [c3 g3 c4 e4]/2 [c3 g3 c4]/2 }");
    assert_eq!(diagnostic.message, "3 notes sound here, and the passage claims 4");
    assert!(
        diagnostic.help.as_deref() == Some("this is 1/2 into the passage"),
        "{:?}",
        diagnostic.help
    );
}

#[test]
fn silence_is_not_a_wrong_number_of_voices() {
    // A rest in every voice is four voices resting. The claim is about the
    // moments where something sounds.
    assert!(errors("assert voices(2) { [c4 e4]/4 rest/4 [c4 e4]/2 }").is_empty());
}

#[test]
fn a_voice_outside_its_range_says_which_range_it_had() {
    let ranges = "[(f2, d4), (c3, g4), (g3, d5), (c4, g5)]";
    let body = format!("assert within_ranges({ranges}) {{ [c3 g3 c4 c6]/1 }}");
    let diagnostic = only_error(&body);
    assert_eq!(diagnostic.message, "`c6` is above the range for the fourth voice");
}

// --- Per-context rechecking --------------------------------------------

#[test]
fn one_motif_under_two_contexts_is_two_verdicts() {
    // Untransposed, the figure is in C major. Up a minor second it is not,
    // and the claim written around each use is checked against what that use
    // actually sounds.
    assert!(errors("assert pitches_in(scale c major) { use figure(); }").is_empty());
    let diagnostic = only_error("transpose up m2 { assert pitches_in(scale c major) { use figure(); } }");
    assert_eq!(diagnostic.message, "`db5` is not in scale c major");
}

#[test]
fn the_offending_note_is_pointed_at_where_it_is_written() {
    // The note came from a motif, so the two spans are two places: the claim
    // is at the use site and the note is in the motif, which is where a
    // composer goes to change its pitch.
    let body = "transpose up m2 { assert pitches_in(scale c major) { use figure(); } }";
    let diagnostic = only_error(body);
    assert_eq!(witness(&diagnostic, body), "c5/4");
    let secondary: Vec<String> = diagnostic
        .labels
        .iter()
        .filter(|label| !label.primary)
        .map(|label| piece(body)[label.span.start as usize..label.span.end as usize].to_owned())
        .collect();
    assert_eq!(secondary.len(), 1, "the assertion is the other place");
    assert!(secondary[0].starts_with("assert pitches_in"), "{secondary:?}");
}

#[test]
fn the_meter_a_claim_reads_is_the_one_in_force_where_it_stands() {
    // The same three quarters, twice: a measure of 3/4 and two-thirds of a
    // measure of 4/4. `fills_meter` reads the meter, not the notes.
    assert!(errors("meter 3/4; assert fills_meter() { c5/4 e5/4 g5/4 }").is_empty());
    assert_eq!(
        only_error("assert fills_meter() { c5/4 e5/4 g5/4 }").message,
        "this passage is 1/4 short"
    );
}

// --- Parity -------------------------------------------------------------

#[test]
fn handwritten_and_generated_music_get_the_same_verdict() {
    let handwritten = errors(&format!("assert pitches_in(scale c major) {{ {PASSAGE} }}"));
    let generated = errors("assert pitches_in(scale c major) { use figure(); }");
    assert!(handwritten.is_empty() && generated.is_empty());

    let handwritten = only_error("assert voices(2) { c5/4 e5/4 g5/4 c6/4 }");
    let generated = only_error("assert voices(2) { use figure(); }");
    assert_eq!(handwritten.message, generated.message);
    assert_eq!(handwritten.code, generated.code);
}

#[test]
fn a_claim_over_a_repeat_sees_every_pass() {
    // A repeat is expansion, and by the time the claim is checked there is no
    // repeat left — only the notes it produced.
    assert_eq!(
        only_error("assert fills_meter() { repeat 2 { c5/4 e5/4 g5/4 c6/4 } }").message,
        "this passage is 1 too long"
    );
}

// --- Nothing becomes a style rule ---------------------------------------

#[test]
fn music_nobody_asserted_anything_about_is_silent() {
    // Every one of these would fail a claim, and no claim was written.
    for body in ["key c major; c#5/4 d#5/4 f#5/4 g#5/4", "[c4 d4 e4 f4 g4]/1", "c8/1"] {
        assert!(
            errors(body).is_empty(),
            "`{body}` complained without being asked: {:?}",
            errors(body).iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_claim_says_nothing_about_the_music_around_it() {
    // The out-of-scale notes are outside the braces, and the claim is about
    // what is inside them.
    assert!(errors(&format!("f#5/4 assert pitches_in(scale c major) {{ {PASSAGE} }} g#5/4")).is_empty());
}

// --- Well-formedness ----------------------------------------------------

#[test]
fn a_name_that_is_not_a_claim_is_refused_with_the_ones_that_are() {
    let diagnostic = only_error("assert pitches_on(scale c major) { c5/1 }");
    assert_eq!(diagnostic.message, "nothing is claimed by `pitches_on`");
    assert_eq!(diagnostic.help.as_deref(), Some("did you mean `pitches_in`?"));
    let note = diagnostic.note.as_deref().unwrap_or_default();
    for claim in ["fills_meter", "pitches_in", "realizes", "voices", "within_ranges"] {
        assert!(note.contains(claim), "the note lists the family: {note}");
    }
}

#[test]
fn a_claim_written_with_the_wrong_arguments_is_shown_its_shape() {
    let diagnostic = only_error("assert pitches_in() { c5/1 }");
    assert_eq!(
        diagnostic.message,
        "`pitches_in` takes one argument, and none were written"
    );
    assert_eq!(diagnostic.help.as_deref(), Some("`pitches_in(Scale)`"));

    // A policy has no type in the elaboration language, so the shape names
    // the three words instead of a name nobody could write.
    let policy = only_error("assert realizes(chord c major) { [c4 e4 g4]/1 }");
    assert_eq!(
        policy.help.as_deref(),
        Some("`realizes(ChordClass, exactly|may_omit|may_add)`")
    );
}

#[test]
fn a_policy_is_one_of_three_words() {
    let diagnostic = only_error("assert realizes(chord c major, may_ad) { [c4 e4 g4]/1 }");
    assert_eq!(diagnostic.message, "`may_ad` is not a realization policy");
    assert_eq!(diagnostic.help.as_deref(), Some("did you mean `may_add`?"));
}

#[test]
fn a_claim_about_music_that_did_not_resolve_stays_quiet() {
    // "`sigh` is not a motif" is the mistake. "these add up to 0" underneath
    // it would be the compiler complaining about the consequence of its own
    // first complaint.
    let diagnostic = only_error("assert fills_meter() { use sigh(); }");
    assert!(diagnostic.message.contains("sigh"), "{}", diagnostic.message);
}
