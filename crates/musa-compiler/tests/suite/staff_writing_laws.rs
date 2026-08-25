//! The writing side of the staff trial (prompt 127dcfb).
//!
//! `staff_expansion_laws.rs` reads a region and asks what it means. This file
//! asks the other two questions: what a structured command does to the text a
//! musician already wrote, and whether a page written *out* of a value says
//! what the value said.
//!
//! The two laws are deliberately different in kind. The **edit law** is about
//! bytes — the patch lands inside the region, replaces the node the command
//! named, and leaves every other byte, comment and blank line exactly where it
//! was. The **round-trip law** is about values, because printing is allowed to
//! normalize: `pickup (1, 4) { … }` prints back as `bar (1, 4) { … }`, the
//! package having no pickup, and comparing text would call that a failure.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{
    AdapterEditError, AdapterPrintError, CompileOptions, SourceDocument, adapter_edits, adapter_print,
};

use musa_score::ScoreEventKind;

/// The bridge from the package's data into music, as `staff_expansion_laws`
/// draws it: one note per realized span, shifted to its onset and stretched to
/// its written value.
const PRELUDE: &str = r"
import std::notation::staff;
import syntax std::adapters::staff as staff;

fn one(start: Position(WrittenTime), held: Duration(WrittenTime)) -> EventTrack(WrittenTime) {
    shift(position_between(position_of(0/1), start), stretch(duration_ratio(held), music { c5/1 }))
}

fn heard(spans: WrittenSpans) -> EventTrack(WrittenTime) {
    written_spans_fold(
        music { rest/1 },
        fn (
            anchor: Nat,
            start: Position(WrittenTime),
            held: Duration(WrittenTime),
            tied: Tie,
            after: EventTrack(WrittenTime),
        ) -> EventTrack(WrittenTime) { together(one(start, held), after) },
        spans,
    )
}

fn shown(answer: Result(Realization, Text)) -> EventTrack(WrittenTime) {
    match answer {
        Ok(reached) -> heard(reached.spans),
        Err(why) -> music { rest/1 },
    }
}
";

/// The page every law here is stated over.
///
/// Small on purpose — each staff compilation costs about a dozen seconds — and
/// still holding one of every shape the printer has to write: a header with a
/// transposition and a spelling, a bar that states its own meter, a plain note,
/// a dotted note, a rest, and a chord.
const TRIAL: &str = "\
        instrument \"bb_clarinet\"
        transposing M2
        clef treble
        key d major
        time (4, 4)
        spelling shortest_readable
        bar (4, 4) { c5/4 d5/4. e5/8 [g4 b4 d5]/4 }";

/// The value `TRIAL` reads to, written out by hand.
///
/// The anchors are the ones that page mints: an anchor is a node's position in
/// the region's own reading order — [`AN_ANCHOR_OF_THE_FIRST_PITCH`] says where
/// the numbers come from — and the adapter takes each item's anchor at the node
/// the item starts with, so a note carries its pitch, a chord carries its
/// bracket, and a bar carries its `bar`. They are stated because the value has
/// to be complete, not because the round-trip law below can see them: see
/// [`a_printed_page_says_what_the_value_said`].
const TRIAL_VALUE: &str = "\
Document(\
\"bb_clarinet\", M2, Treble, key d major, Beats(4, 4), ShortestReadable, \
Bar(32, Beats(4, 4), \
Sounded(42, Note(c5, NoteValue(4, 0), Untied), \
Sounded(46, Note(d5, NoteValue(4, 1), Untied), \
Sounded(51, Note(e5, NoteValue(8, 0), Untied), \
Sounded(55, Chord([g4, b4, d5], NoteValue(4, 0), Untied), NoItems)))), \
NoItems))";

/// The anchor of `TRIAL`'s first written pitch.
///
/// A region's anchors are its nodes in reading order, trivia included, so this
/// is a number about *that* page and not a number about staff notation: 0 is
/// the region's own group, 2 is `instrument`, 42 is the bar's body, and 44 is
/// the `c5` inside it. The test below asserts what it names, so a page edited
/// out from under it fails saying which node it found instead.
///
/// It moved by two at prompt 162hb, and the page did not: `transposing M2`
/// stands before the bar, and a composite literal is a node over its parts
/// now, so `M2` is three nodes where it was one. Reading order is reading
/// order — what it counts is what the reader read.
const AN_ANCHOR_OF_THE_FIRST_PITCH: u64 = 44;

/// A piece holding `region` as a staff page and sounding what it realizes.
fn piece(region: &str) -> String {
    format!(
        "piece \"Staff writing laws\" {{\n{PRELUDE}\n\
         let page: StaffDocument = syntax staff {{\n{region}\n}};\n\
         meter 4/4;\nkey d major;\n\
         score {{ part p {{ voice v {{ use shown(realize(page)); }} }} }}\n}}\n"
    )
}

/// Every note the page realizes to, as `(onset, written duration)` in
/// whole-note fractions, earliest first.
///
/// The whole rest the span fold ends in is dropped: it is the fold's base and
/// says nothing about what was read.
fn realized(region: &str) -> Vec<(String, String)> {
    let source = piece(region);
    let compilation = musa_compiler::compile(
        &SourceDocument::new(&source, "staff-writing-laws.musa"),
        &CompileOptions::default(),
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let score = compilation.snapshot().expect("a score");
    let mut found: Vec<(num_rational::Ratio<i64>, String)> = score
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .flat_map(|(_, voice)| voice.events().to_vec())
        .filter(|event| !matches!(event.kind, ScoreEventKind::Rest))
        .map(|event| (event.onset.as_ratio(), event.notated_duration.spelling))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|(onset, spelling)| (format!("{}/{}", onset.numer(), onset.denom()), spelling))
        .collect()
}

/// The document a printed region is written into: the imports the printer and
/// its subject are read with, and nothing else.
fn writing_into() -> SourceDocument {
    SourceDocument::new(
        "piece \"Staff writing laws\" {\n    import std::notation::staff;\n}\n",
        "staff-writing-laws.musa",
    )
}

// ------------------------------------------------------------------ the edit

#[test]
fn an_edit_changes_the_pitch_it_names_and_no_other_byte() {
    // §2.6 of the trial note, on real notation: one pitch changes and the page
    // around it does not. `replace` puts `a5` where the anchored `c5` stands,
    // and the written value beside it — the `/4` the anchor does not name — is
    // not part of the edit.
    let source = piece(TRIAL);
    let at = u32::try_from(source.find("syntax staff").expect("a region")).expect("a small file");
    let edits = adapter_edits(
        &SourceDocument::new(&source, "staff-writing-laws.musa"),
        &CompileOptions::default(),
        at,
        "replace",
        AN_ANCHOR_OF_THE_FIRST_PITCH,
        "a5",
    )
    .expect("`staff` serves `replace`");
    let [edit] = edits.as_slice() else {
        panic!("one command, one edit: {edits:?}");
    };
    assert_eq!(
        source.get(edit.start as usize..edit.end as usize),
        Some("c5"),
        "the edit replaces the node the anchor named"
    );
    let patched = format!(
        "{}{}{}",
        source.get(..edit.start as usize).unwrap_or_default(),
        edit.text,
        source.get(edit.end as usize..).unwrap_or_default()
    );
    // `c5/4` occurs once in the whole file — the prelude's own note is `c5/1` —
    // so this is the page with that one pitch changed and nothing else, written
    // independently of the offsets the adapter answered with.
    assert_eq!(
        patched,
        source.replacen("c5/4", "a5/4", 1),
        "and every other byte of the page — comments, layout, the header — is where it was"
    );
}

#[test]
fn a_command_the_staff_does_not_serve_is_refused_by_name() {
    // A refusal is the adapter's sentence, not a compiler fault: `staff`
    // read the command, knows it does not serve it, and says which one it
    // does. `AdapterEditError::Refused` is the case that carries that apart
    // from a broken adapter.
    let source = piece(TRIAL);
    let at = u32::try_from(source.find("syntax staff").expect("a region")).expect("a small file");
    let refusal = adapter_edits(
        &SourceDocument::new(&source, "staff-writing-laws.musa"),
        &CompileOptions::default(),
        at,
        "transpose",
        2,
        "M3",
    )
    .expect_err("`staff` serves one command");
    let AdapterEditError::Refused { adapter, message } = refusal else {
        panic!("a refusal, not a fault: {refusal:?}");
    };
    assert_eq!(adapter, "std::adapters::staff");
    assert_eq!(message, "`staff` serves one command, `replace`");
}

// ------------------------------------------------------------ the round trip

// Ignored until prompt 166: the staff adapter's `print` failed
// byte-identically on the checker the course correction replaced and on the
// one that replaced it — its transformer built a core term musa-calculus
// rejects ("a value that is not a type stood in type position"). The rewrite
// at 166 does not build that term, so the law runs in the fast suite.
#[test]
fn a_printed_page_says_what_the_value_said() {
    // The round-trip law, stated on the value rather than on the text because
    // printing is allowed to normalize.
    //
    // What is compared is what the page *realizes to* — every span's onset and
    // written value — which is the package's own answer about the notation and
    // the strongest observation the public surface offers. Anchors are outside
    // it: the reader mints them from the printed text, so a printed page's
    // numbers are the numbers that text earns, and no compiled artifact
    // reports them. That is a limit of what this test can see, not permission
    // for the printer to reorder a page: reordering would move an onset, and
    // the onsets are compared here exactly.
    let printed = adapter_print(
        &writing_into(),
        &CompileOptions::default(),
        "std::adapters::staff",
        TRIAL_VALUE,
    )
    .expect("the staff can write this page");
    assert_eq!(
        realized(&printed),
        realized(TRIAL),
        "the printed page realizes what the page it was printed from realizes"
    );
}

#[test]
fn a_page_the_staff_cannot_spell_is_a_stated_loss_naming_what_it_could_not_write() {
    // The first value a printer has had that it can genuinely fail to spell.
    //
    // A part transposed *down* a major second is a real document — the value
    // holds it, and `interval_inverse` is how an ordinary expression names it —
    // and the reader spells it `down M2`, which the interval literal grammar
    // has no token for. So this page cannot be written down.
    //
    // The answer is a sentence saying which part it could not write, not a page
    // with the transposition quietly left out: a printer that dropped it would
    // satisfy the round-trip law by making the value smaller, which is the
    // failure `26-language-design-decision.md` §4 names.
    let unspellable = TRIAL_VALUE.replace(", M2, ", ", interval_inverse(M2), ");
    assert_ne!(
        unspellable, TRIAL_VALUE,
        "the fixture turned the transposition it meant to turn"
    );
    let loss = adapter_print(
        &writing_into(),
        &CompileOptions::default(),
        "std::adapters::staff",
        &unspellable,
    )
    .expect_err("no written interval names this transposition");
    let AdapterPrintError::Loss { adapter, message } = loss else {
        panic!("a stated loss, not a fault: {loss:?}");
    };
    assert_eq!(adapter, "std::adapters::staff");
    assert!(
        message.contains("transposed"),
        "the loss names what it could not spell: {message}"
    );
}
