//! What reading a piece promises.
//!
//! Beside the module for [`crate::document::laws`]'s reason: the answer is a
//! [`musa_core::Term`] in a context [`crate::registry::owned`] built, and both
//! are private to this crate.
//!
//! # The survey
//!
//! [`every_example_elaborates`] is the point of the prompt rather than a check
//! on it, exactly as prompt 141o's standard-library survey was. Fifty-four
//! pieces, read through the new structure and checked through the new core, with
//! every remaining fault recorded: each is a spelling prompt 142's Target
//! already owns, and one that appeared without one would be a defect in the
//! reading rather than a migration.
//!
//! The recorded list is exact. A file that starts failing for a new reason fails
//! the build; a reason that stops applying has to be struck from the list in the
//! commit that fixed it.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_language::{SyntaxKind, SyntaxNode};

use crate::document::{Document, Source, elaborate};
use crate::resolve::Resolver;

// ---- reading a written piece back out of a parse ----

/// The `piece … { … }` node `source` writes, with a loud failure when it does
/// not parse.
fn written(source: &str) -> SyntaxNode {
    let document = musa_language::parse(source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    piece_of(&document.syntax()).expect("the source writes a piece")
}

/// The first `piece` under `root`, when there is one.
fn piece_of(root: &SyntaxNode) -> Option<SyntaxNode> {
    root.descendants().find(|node| node.kind() == SyntaxKind::PieceDecl)
}

/// `node` read as a piece and checked, and every distinct fault, sorted.
///
/// Both halves, because neither is the whole promise: the structure has to
/// *read* — every part numbered, every voice folded — and the term it answers
/// has to **check**, which is what says the fold built something that is a
/// written-time track rather than merely a term.
fn checked(node: &SyntaxNode) -> (Option<musa_core::Term>, Vec<String>) {
    checked_with(node, &[])
}

/// The same, with `libraries` in scope beside the piece.
///
/// The piece is its own source: its motifs, fragments, and `let`s are
/// declarations, and a voice that writes `use theme();` needs them bound before
/// the structure around it is a term. What `libraries` adds is what an `import`
/// would — [`every_example_elaborates`] passes the standard library, because a
/// survey that reported `harmonize` missing would be measuring its own harness.
fn checked_with(node: &SyntaxNode, libraries: &[Source]) -> (Option<musa_core::Term>, Vec<String>) {
    let mut resolver = Resolver::new();
    let mut sources = libraries.to_vec();
    sources.push(Source {
        root: node.clone(),
        in_phase: false,
    });
    let elaborated = elaborate(&mut resolver, &sources);
    let answer = read(&mut resolver, elaborated, node);
    let mut said: Vec<String> = resolver
        .diagnostics
        .iter()
        .map(|complaint| format!("{:?}: {}", complaint.code, complaint.message))
        .collect();
    said.sort();
    said.dedup();
    (answer, said)
}

/// The piece's normal form, when the document elaborated and the structure read.
///
/// The structure is read either way. A document that did not elaborate has no
/// context to *check* a term in, but the reading itself needs none — it walks a
/// score and writes raw terms — and a survey that skipped it whenever a
/// signature still said `Music` would be reporting on everything except the
/// thing it is surveying.
fn read(resolver: &mut Resolver, elaborated: Option<Document>, node: &SyntaxNode) -> Option<musa_core::Term> {
    let Some(mut document) = elaborated else {
        let mut sites = crate::lower::Sites::default();
        crate::lower::Lowering::new(resolver, &mut sites).piece(node);
        return None;
    };
    let piece = document.piece(resolver, node)?;
    match document.term(&piece.track) {
        Ok((normal, _)) => Some(normal),
        Err(error) => {
            resolver.report(crate::lower::refusals::restate(document.sites(), &error));
            None
        }
    }
}

/// `source`, which every law that expects success reads its answer out of.
fn piece(source: &str) -> musa_core::Term {
    let (answer, said) = checked(&written(source));
    answer.unwrap_or_else(|| panic!("the piece elaborates: {said:?}"))
}

/// `source`'s parts and voices, without checking the term they fold into.
///
/// Separate from [`piece`] because the numbering is what several laws are
/// about, and a law about numbering should not fail because a voice's body did.
fn structure(source: &str) -> (Option<super::Piece>, Vec<String>) {
    let node = written(source);
    let mut resolver = Resolver::new();
    let mut sites = crate::lower::Sites::default();
    let held = crate::lower::Lowering::new(&mut resolver, &mut sites).piece(&node);
    let mut said: Vec<String> = resolver
        .diagnostics
        .iter()
        .map(|complaint| format!("{:?}: {}", complaint.code, complaint.message))
        .collect();
    said.sort();
    said.dedup();
    (held, said)
}

// ---- the structure ----

#[test]
fn a_voice_is_read_at_its_own_scope() {
    let shown = format!(
        "{:?}",
        piece(
            "piece \"two\" {
                score {
                    part upper { voice one { c5/4 } }
                    part lower { voice two { c3/4 } }
                }
            }"
        )
    );
    assert!(
        shown.contains("Voice { part: 0, voice: 0 }"),
        "the first part's first voice: {shown}"
    );
    assert!(
        shown.contains("Voice { part: 1, voice: 0 }"),
        "and the second part's, numbered within its own part: {shown}"
    );
}

/// An instance site is stated, not skipped.
///
/// The other half of the positional rule — that a written voice keeps its
/// number when the site beside it is expanded — has nothing to assert against
/// yet, because every piece containing a site is refused and no [`super::Piece`]
/// comes back to read ids out of. What the rule buys is that prompt 142 does not
/// have to renumber anything when it stops refusing; what this law protects
/// until then is that the site is *reported* rather than silently dropped.
#[test]
fn an_instance_site_is_refused_rather_than_skipped() {
    let (held, said) = structure(
        "piece \"numbered\" {
            score {
                part strings {
                    voice first { c4/4 }
                    make doubled(first) as second;
                    voice third { e4/4 }
                }
            }
        }",
    );
    assert!(held.is_none(), "the instance site is refused: {said:?}");
    assert_eq!(
        said,
        ["UnsupportedLanguageStage: a voice made from a template has no core spelling yet"],
        "and refused as something prompt 142 owns rather than skipped"
    );
}

#[test]
fn parts_and_voices_are_numbered_by_position() {
    let (held, said) = structure(
        "piece \"numbered\" {
            score {
                part strings { voice first { c4/4 } voice second { e4/4 } }
                part winds { voice only { g4/4 } }
            }
        }",
    );
    let held = held.unwrap_or_else(|| panic!("the piece reads: {said:?}"));
    assert_eq!(
        held.parts
            .iter()
            .map(|part| (part.id, part.name.as_str()))
            .collect::<Vec<_>>(),
        [(0, "strings"), (1, "winds")],
        "parts are numbered by their position in the score"
    );
    let part = held.parts.first().expect("one part");
    assert_eq!(
        part.voices
            .iter()
            .map(|voice| (voice.id, voice.name.as_str()))
            .collect::<Vec<_>>(),
        [(0, "first"), (1, "second")],
        "and voices by their position among the part's items"
    );
}

#[test]
fn two_parts_with_one_name_are_refused() {
    let (held, said) = structure(
        "piece \"twice\" {
            score { part strings { voice a { c4/4 } } part strings { voice b { e4/4 } } }
        }",
    );
    assert!(held.is_none(), "the repeat is refused");
    assert_eq!(said, ["DuplicateName: this score already has a part called `strings`"]);
}

#[test]
fn two_voices_of_one_part_with_one_name_are_refused() {
    let (held, said) = structure(
        "piece \"twice\" {
            score { part strings { voice dux { c4/4 } voice dux { e4/4 } } }
        }",
    );
    assert!(held.is_none(), "the repeat is refused");
    assert_eq!(said, ["DuplicateName: part `strings` already has a voice called `dux`"]);
}

// ---- the context tracks ----

#[test]
fn a_header_fact_covers_the_longest_voice() {
    let shown = format!(
        "{:?}",
        piece(
            "piece \"uneven\" {
                meter 3/4;
                score {
                    part upper { voice one { c5/4 } }
                    part lower { voice two { c3/1 c3/1 } }
                }
            }"
        )
    );
    // D3's max over the parts, not the sum: 2/1 and not 9/4.
    assert!(
        shown.contains("Ratio { numer: 2, denom: 1 }"),
        "the longest voice: {shown}"
    );
    assert!(
        !shown.contains("Ratio { numer: 9, denom: 4 }"),
        "and not the sum of them: {shown}"
    );
}

#[test]
fn a_piece_that_writes_no_meter_still_has_one() {
    let shown = format!(
        "{:?}",
        piece("piece \"bare\" { score { part p { voice v { c4/4 } } } }")
    );
    assert_eq!(shown.matches("kind: Meter").count(), 1, "one meter fact: {shown}");
    assert!(
        shown.contains("scope: Piece, kind: Meter { numerator: 4, denominator: 4 }"),
        "and it is the piece's 4/4, unwritten: {shown}"
    );
}

/// Where a context fact says it came from.
///
/// The fold constructs one `sounded` call per statement and the origin it
/// carries is the statement's, not the piece's: a composer asking where the 7/8
/// came from is asking about a line, and an answer spanning the whole
/// declaration would be true and useless.
#[test]
fn a_context_fact_points_at_the_statement_that_said_it() {
    let source = "piece \"placed\" {
                meter 7/8;
                score { part p { voice v { c4/4 } } }
            }";
    let shown = format!("{:?}", piece(source));
    let wrote = source.find("meter 7/8;").expect("the law writes a meter");
    let end = wrote.saturating_add("meter 7/8;".len());
    assert!(
        shown.contains(&format!("SourceSpan {{ start: {wrote}, end: {end} }}")),
        "the header meter points at its own line: {shown}"
    );
}

#[test]
fn a_key_is_ordinary_in_a_piece_and_misplaced_in_a_music_value() {
    let held = piece(
        "piece \"keyed\" {
            key g major;
            score { part p { voice v { c4/4 } } }
        }",
    );
    let shown = format!("{held:?}");
    assert!(
        shown.contains("kind: Key { tonic: PitchClass { letter: G, accidental: Accidental(0) }, mode: Major }"),
        "a piece says where it starts: {shown}"
    );
    let mut resolver = Resolver::new();
    let mut sites = crate::lower::Sites::default();
    let node = musa_language::parse("let held = music { key g major; c4/4 };");
    let written = node
        .syntax()
        .descendants()
        .find(|child| child.kind() == SyntaxKind::MusicExpr)
        .expect("the law writes a music value");
    assert!(
        crate::lower::Lowering::new(&mut resolver, &mut sites)
            .music(&written)
            .is_none(),
        "and a value usable at several places does not"
    );
    assert_eq!(
        resolver
            .diagnostics
            .iter()
            .map(|complaint| complaint.message.clone())
            .collect::<Vec<_>>(),
        ["a key change cannot stand in a `music` value"]
    );
}

#[test]
fn a_clef_written_in_a_voice_belongs_to_the_part() {
    let shown = format!(
        "{:?}",
        piece(
            "piece \"changing\" {
                score { part p { voice v { c4/4 clef bass; c3/4 } } }
            }"
        )
    );
    assert!(
        shown.contains("scope: Part { part: 0 }, kind: Clef { clef: Bass }"),
        "the clef is a fact at the part's scope rather than the voice's: {shown}"
    );
}

#[test]
fn a_parts_own_meter_is_a_fact_at_the_parts_scope() {
    let shown = format!(
        "{:?}",
        piece(
            "piece \"polymetric\" {
                meter 4/4;
                score { part seven { meter 7/8; voice v { c4/4 } } }
            }"
        )
    );
    assert_eq!(shown.matches("kind: Meter").count(), 2, "two meter facts: {shown}");
    assert!(
        shown.contains("scope: Piece, kind: Meter { numerator: 4, denominator: 4 }"),
        "the piece's: {shown}"
    );
    assert!(
        shown.contains("scope: Part { part: 0 }, kind: Meter { numerator: 7, denominator: 8 }"),
        "and the part's own, which is all polymeter is: {shown}"
    );
}

// ---- the survey ----

/// Every piece in `examples/`, read and checked with the standard library in
/// scope.
///
/// Ten reasons remain, in six classes, and none of them is about the structure
/// this prompt built:
///
/// - **the contextual `music` value** — `Music` in a signature, and the `step`
///   in a free `music { … }` that no `in scale` encloses. One thing twice:
///   prompt 142 deletes contextual `Music`, and a phrase whose meaning depends
///   on where it is used is the thing being deleted.
/// - **a notation statement whose argument is a name** — `root/4` in
///   `motif turn(root: Pitch)`, `key k;` and `in scale mode` in a
///   `template piece`. The reading folds a pitch, a key, and a scale to a
///   *value* while it walks, and a parameter has no value until the site
///   supplies one, so each is reported as though a literal had been misspelled.
///   Prompt 142's Target now names this class; it is the one fault the survey
///   found that no prompt had written down.
/// - **the instance site** — `make` inside a part, which needs the expansion
///   path this prompt's Stop leaves to 142.
/// - **bar structure** — `bar`, `senza`, and `assert`, which are checked against
///   barlines a pass resolves once every voice has been read.
/// - **the anonymous product** — `(A, B)` written as a type, which prompt 136's
///   records replaced and [`crate::lower::types`] refuses by name.
/// - **an argument written by name** — `f(x: 1)`, which
///   [`crate::lower::values`] refuses. Its message reads backwards, naming what
///   the writer should have done rather than what they did; the wording belongs
///   to the prompt that owns that refusal.
/// - **a kernel quote** — `kernel { … }`, whose core spelling 141h's laws
///   already record as 142's.
///
/// Everything else holds on the real corpus: fifty-four pieces' worth of
/// notation statements, motifs, fragments, transformations, part and voice
/// numbering, header and per-part context, and the scope every fact in them is
/// constructed at.
#[test]
fn every_example_elaborates() {
    let libraries = crate::document::laws::library_sources();
    let mut said: Vec<String> = Vec::new();
    for (name, source) in EXAMPLES {
        let parsed = musa_language::parse(source);
        assert!(parsed.errors().is_empty(), "`{name}` parses: {:?}", parsed.errors());
        let Some(node) = piece_of(&parsed.syntax()) else {
            continue;
        };
        let (_, reasons) = checked_with(&node, &libraries);
        said.extend(reasons);
    }
    said.sort();
    said.dedup();
    assert_eq!(
        said,
        [
            "Misplaced: `step` needs a scale to count in",
            "NotAValue: `mode` is not a pitch class",
            "NotAValue: `root` is not a pitch",
            "NotAValue: this key cannot be read",
            "UnknownName: no binder named `Music` is in scope",
            "UnsupportedLanguageStage: a kernel quote has no core spelling yet",
            "UnsupportedLanguageStage: a voice made from a template has no core spelling yet",
            "UnsupportedLanguageStage: an anonymous product has no core spelling",
            "UnsupportedLanguageStage: an argument is passed by position",
            "UnsupportedLanguageStage: this statement has no core spelling yet",
        ],
        "the examples need exactly what 142 owns"
    );
}

/// Every `.musa` file in `examples/`.
///
/// Written out rather than globbed because a test that discovers its own corpus
/// cannot fail when the corpus shrinks, and a file quietly dropped from a survey
/// is the one thing a survey must not allow.
const EXAMPLES: &[(&str, &str)] = &[
    ("annotated", include_str!("../../../../../examples/annotated.musa")),
    (
        "anonymous-functions",
        include_str!("../../../../../examples/anonymous-functions.musa"),
    ),
    ("bulgarian", include_str!("../../../../../examples/bulgarian.musa")),
    ("cadenza", include_str!("../../../../../examples/cadenza.musa")),
    (
        "canon-functions",
        include_str!("../../../../../examples/canon-functions.musa"),
    ),
    ("canon-x", include_str!("../../../../../examples/canon-x.musa")),
    ("canon", include_str!("../../../../../examples/canon.musa")),
    ("changes", include_str!("../../../../../examples/changes.musa")),
    (
        "changing-meter",
        include_str!("../../../../../examples/changing-meter.musa"),
    ),
    ("chant", include_str!("../../../../../examples/chant.musa")),
    (
        "chord-voicings",
        include_str!("../../../../../examples/chord-voicings.musa"),
    ),
    ("clef-change", include_str!("../../../../../examples/clef-change.musa")),
    (
        "contextual-music",
        include_str!("../../../../../examples/contextual-music.musa"),
    ),
    (
        "counterpoint",
        include_str!("../../../../../examples/counterpoint.musa"),
    ),
    (
        "diatonic-sequences",
        include_str!("../../../../../examples/diatonic-sequences.musa"),
    ),
    ("doubled", include_str!("../../../../../examples/doubled.musa")),
    ("drum-chart", include_str!("../../../../../examples/drum-chart.musa")),
    (
        "gesture-data",
        include_str!("../../../../../examples/gesture-data.musa"),
    ),
    (
        "glass-mountain",
        include_str!("../../../../../examples/glass-mountain.musa"),
    ),
    (
        "graces-reordered",
        include_str!("../../../../../examples/graces-reordered.musa"),
    ),
    ("graces", include_str!("../../../../../examples/graces.musa")),
    (
        "harmonize-function",
        include_str!("../../../../../examples/harmonize-function.musa"),
    ),
    ("hemiola", include_str!("../../../../../examples/hemiola.musa")),
    ("house", include_str!("../../../../../examples/house.musa")),
    ("in-c", include_str!("../../../../../examples/in-c.musa")),
    ("invention", include_str!("../../../../../examples/invention.musa")),
    (
        "kernel-splice",
        include_str!("../../../../../examples/kernel-splice.musa"),
    ),
    (
        "loop-lengths",
        include_str!("../../../../../examples/loop-lengths.musa"),
    ),
    ("mobile", include_str!("../../../../../examples/mobile.musa")),
    ("modulation", include_str!("../../../../../examples/modulation.musa")),
    (
        "module-functor-study",
        include_str!("../../../../../examples/module-functor-study.musa"),
    ),
    (
        "named-answer",
        include_str!("../../../../../examples/named-answer.musa"),
    ),
    (
        "neo-riemannian",
        include_str!("../../../../../examples/neo-riemannian.musa"),
    ),
    ("ornaments", include_str!("../../../../../examples/ornaments.musa")),
    (
        "pitch-arithmetic",
        include_str!("../../../../../examples/pitch-arithmetic.musa"),
    ),
    (
        "profile-fixture",
        include_str!("../../../../../examples/profile-fixture.musa"),
    ),
    ("refrain", include_str!("../../../../../examples/refrain.musa")),
    ("repeats", include_str!("../../../../../examples/repeats.musa")),
    ("riser", include_str!("../../../../../examples/riser.musa")),
    ("rubato", include_str!("../../../../../examples/rubato.musa")),
    (
        "rule-of-the-octave",
        include_str!("../../../../../examples/rule-of-the-octave.musa"),
    ),
    (
        "scale-context",
        include_str!("../../../../../examples/scale-context.musa"),
    ),
    (
        "serial-forms",
        include_str!("../../../../../examples/serial-forms.musa"),
    ),
    ("shuffle", include_str!("../../../../../examples/shuffle.musa")),
    ("staff-page", include_str!("../../../../../examples/staff-page.musa")),
    (
        "stdlib-basics",
        include_str!("../../../../../examples/stdlib-basics.musa"),
    ),
    (
        "template-study",
        include_str!("../../../../../examples/template-study.musa"),
    ),
    (
        "tempo-changes",
        include_str!("../../../../../examples/tempo-changes.musa"),
    ),
    (
        "theory-assertions",
        include_str!("../../../../../examples/theory-assertions.musa"),
    ),
    (
        "tonal-construction",
        include_str!("../../../../../examples/tonal-construction.musa"),
    ),
    (
        "tuplet-fixture",
        include_str!("../../../../../examples/tuplet-fixture.musa"),
    ),
    ("twinkle", include_str!("../../../../../examples/twinkle.musa")),
    (
        "unicode-fixture",
        include_str!("../../../../../examples/unicode-fixture.musa"),
    ),
    ("variation", include_str!("../../../../../examples/variation.musa")),
];
