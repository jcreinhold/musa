//! What reading a piece promises.
//!
//! Beside the module for [`crate::document::laws`]'s reason: the answer is a
//! [`musa_calculus::Term`] in a context [`crate::registry::owned`] built, and both
//! are private to this crate.
//!
//! # The survey
//!
//! [`every_example_elaborates`] is the point of the prompt rather than a check
//! on it, exactly as prompt 141o's standard-library survey was. Fifty-four
//! pieces, read through the new structure and checked through the new core, with
//! every remaining fault recorded and named to the prompt that owns it. A fault
//! that appeared without one would be a defect in the reading rather than a
//! migration.
//!
//! The recorded list is exact. A file that starts failing for a new reason fails
//! the build; a reason that stops applying has to be struck from the list in the
//! commit that fixed it.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::document::{Document, Source, elaborate};
use crate::elaborate::{FactKind, VoiceTrack};
use crate::resolve::Resolver;

/// The document name every law below reads its piece under.
///
/// One name for all of them, because the only thing it decides is what a
/// template instance's generated identity is minted in, and a law that varied
/// it would be varying a digest nothing here reads.
const DOCUMENT: &str = "law";

// ---- reading a written piece back out of a parse ----

/// The `piece … { … }` node `source` writes, with a loud failure when it does
/// not parse.
fn written(source: &str) -> SyntaxNode {
    let document = musa_syntax::parse(source);
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
fn checked(node: &SyntaxNode) -> (Option<musa_calculus::Term>, Vec<String>) {
    checked_with(node, &[])
}

/// The same, with `libraries` in scope beside the piece.
///
/// The piece is its own source: its motifs, fragments, and `let`s are
/// declarations, and a voice that writes `use theme();` needs them bound before
/// the structure around it is a term. What `libraries` adds is what an `import`
/// would — [`every_example_elaborates`] passes the standard library, because a
/// survey that reported `harmonize` missing would be measuring its own harness.
///
/// The document's own root goes in beside the piece, and a root `make` is read
/// before either — the two halves of [`crate::elaborate`]'s `declaring`. Both
/// matter to the same file: `examples/template-study.musa` writes `fn theme()`
/// at the root and its piece is a *template's* body, whose parameters no source
/// binds and the instance does. A harness that read the body without the `make`
/// standing under it would report `subject` unbound — which is a fault of the
/// reading rather than of the file, and the compiler accepts that file.
///
/// Which piece is read follows from the same two: a document's piece is the one
/// written at its root, or the one its root `make` names, and only when it has
/// neither is it the node the caller handed over. The distinction is not
/// pedantic — `examples/module-functor-study.musa` declares two template pieces
/// and makes the *second*, so the first `piece` node under the root is a
/// template nothing in that file instantiates.
fn checked_with(node: &SyntaxNode, libraries: &[Source]) -> (Option<musa_calculus::Term>, Vec<String>) {
    let mut resolver = Resolver::new();
    let root = node.ancestors().last().unwrap_or_else(|| node.clone());
    let made = musa_syntax::ast::MakeStmt::from_root(&root).and_then(|site| {
        crate::template::Templates::collect(&mut resolver, &root).instance(
            &mut resolver,
            &site,
            "piece",
            crate::template::Kind::Piece,
            None,
            DOCUMENT,
        )
    });
    let declared = musa_syntax::ast::PieceDecl::from_root(&root)
        .or_else(|| made.as_ref().and_then(crate::template::Instance::piece))
        .map_or_else(|| node.clone(), |piece| piece.syntax().clone());
    let mut sources = libraries.to_vec();
    if root != declared {
        sources.push(Source::own(&root));
    }
    sources.push(Source::own(&declared));
    let elaborated = elaborate(&mut resolver, &sources, made.as_ref());
    let answer = read(&mut resolver, elaborated, &declared);
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
fn read(resolver: &mut Resolver, elaborated: Option<Document>, node: &SyntaxNode) -> Option<musa_calculus::Term> {
    let Some(mut document) = elaborated else {
        let mut sites = crate::lower::Sites::default();
        crate::lower::Lowering::new(resolver, &mut sites).piece(node, DOCUMENT, None);
        return None;
    };
    let piece = document.piece(resolver, node, DOCUMENT)?;
    match document.term(&piece.track) {
        Ok((normal, _)) => Some(normal),
        Err(error) => {
            resolver.report(crate::lower::refusals::restate(document.sites(), &error));
            None
        }
    }
}

/// `source`, which every law that expects success reads its answer out of.
fn piece(source: &str) -> musa_calculus::Term {
    let (answer, said) = checked(&written(source));
    answer.unwrap_or_else(|| panic!("the piece elaborates: {said:?}"))
}

/// The whole piece and each of its lanes, read the way a consumer reads them.
///
/// Through [`Document::track`] rather than through [`piece`] and a hand-written
/// shape match, because that is the operation the passes use and a law that took
/// the term apart itself would be checking a second readback rather than the
/// one.
fn sounding(source: &str) -> (VoiceTrack, Vec<(String, VoiceTrack)>) {
    let node = written(source);
    let mut resolver = Resolver::new();
    let sources = [Source::own(&node)];
    let mut document = elaborate(&mut resolver, &sources, None).expect("the document elaborates");
    let read = document.piece(&mut resolver, &node, DOCUMENT).expect("the piece reads");
    let lanes = read
        .parts
        .iter()
        .flat_map(|part| {
            part.voices.iter().map(|voice| {
                let track = document
                    .track(&voice.track)
                    .expect("a voice reads back as a track of its own");
                (format!("{}.{}", part.name, voice.name), track)
            })
        })
        .collect();
    let whole = document.track(&read.track).expect("the piece reads back as a track");
    (whole, lanes)
}

/// Every claim written in `source`, placed the way a consumer places it.
///
/// Through [`Document::passage`] for [`sounding`]'s reason: placing a claim by
/// hand here would test a second arithmetic rather than the one the passes use.
fn claimed(source: &str) -> Vec<(Ratio<i64>, Ratio<i64>, String)> {
    let node = written(source);
    let mut resolver = Resolver::new();
    let sources = [Source::own(&node)];
    let mut document = elaborate(&mut resolver, &sources, None).expect("the document elaborates");
    let read = document.piece(&mut resolver, &node, DOCUMENT).expect("the piece reads");
    read.parts
        .iter()
        .flat_map(|part| part.voices.iter())
        .flat_map(|voice| voice.claims.iter())
        .map(|claim| {
            let (made, placed) = document.passage(claim).expect("a claim places against its own voice");
            (placed.at.as_ratio(), placed.extent.as_ratio(), made.describe())
        })
        .collect()
}

/// A bar knows where it stands without a cursor to keep it.
///
/// The claim a `bar` makes is about *this* measure under the meter in force
/// *here*, so a bar that could not place itself would be a true claim about the
/// wrong music. The fold has no cursor; what it has is the term standing before
/// each bar, and how long that lasts is where the bar begins. Three whole
/// measures start at 0, 1, and 2 — and the third one holds a single quarter,
/// which is the case the claim exists to catch and the reason the *extent* is
/// asserted beside the position.
#[test]
fn a_bar_is_placed_by_the_music_before_it() {
    assert_eq!(
        claimed(
            "piece \"bars\" {
                score { part solo { voice one {
                    bar { c4/4 d4/4 e4/4 f4/4 }
                    bar { g4/4 a4/4 b4/4 c5/4 }
                    bar { c5/4 }
                } } }
            }",
        ),
        [
            (Ratio::new(0, 1), Ratio::new(1, 1), "fills_meter()".to_owned()),
            (Ratio::new(1, 1), Ratio::new(1, 1), "fills_meter()".to_owned()),
            (Ratio::new(2, 1), Ratio::new(1, 4), "fills_meter()".to_owned()),
        ],
        "each bar starts where the one before it ended, and says how long it is"
    );
}

/// A bar nested inside a block is still placed absolutely.
///
/// The composition [`super::super::notation::Claimed::before`] performs, and the
/// only law that can tell it apart from a reading that just happened to work at
/// the top level: the bar inside the `repeat` is written second in its own
/// block, and the block itself stands after a whole measure, so the answer is a
/// sum of two prefixes rather than either one of them.
#[test]
fn a_nested_bar_is_placed_through_every_block_it_is_in() {
    assert_eq!(
        claimed(
            "piece \"nested bars\" {
                score { part solo { voice one {
                    bar { c4/4 d4/4 e4/4 f4/4 }
                    repeat 2 {
                        bar { g4/4 a4/4 b4/4 c5/4 }
                        bar { c5/2 }
                    }
                } } }
            }",
        )
        .iter()
        .map(|(at, extent, _)| (*at, *extent))
        .collect::<Vec<_>>(),
        [
            (Ratio::new(0, 1), Ratio::new(1, 1)),
            (Ratio::new(1, 1), Ratio::new(1, 1)),
            (Ratio::new(2, 1), Ratio::new(1, 2)),
        ],
        "the repeat's first bar stands after the measure before the repeat, not at zero"
    );
}

/// An assertion is the claim it names, with the arguments it was written with.
///
/// The law the `assert` reading exists for, and it is stated through
/// [`Document::passage`] for [`claimed`]'s reason. Every one of
/// `musa_score::assert::ParamType`'s six shapes is here, because the six are read
/// three different ways and a law that exercised one would prove the least
/// interesting of them: a scale and a chord are base literals, a count is the
/// prelude's unary `Nat`, a list of ranges is `List.Cons` over `Pair.Both`, and
/// a policy and a rule id are *words* that never become terms at all.
///
/// The positions are asserted beside the claims for the same reason a bar's
/// are: an assertion places exactly the way a bar does, because it is the same
/// [`super::super::notation::Claimed`] with a different claim in it.
#[test]
fn an_assertion_carries_the_claim_and_the_arguments_it_was_written_with() {
    assert_eq!(
        claimed(
            "piece \"claims\" {
                score { part choir { voice one {
                    assert pitches_in(scale c major) { c4/4 d4/4 e4/4 f4/4 }
                    assert voices(1) { g4/4 }
                    assert realizes(chord c major, exactly) { [c4 e4 g4]/4 }
                    assert within_ranges([(c3, c5), (g3, g5)]) { c4/4 }
                    assert follows(satb_spacing) { c4/4 }
                    assert fills_meter() { c4/1 }
                } } }
            }",
        ),
        [
            (
                Ratio::new(0, 1),
                Ratio::new(1, 1),
                "pitches_in(scale c major)".to_owned()
            ),
            (Ratio::new(1, 1), Ratio::new(1, 4), "voices(1)".to_owned()),
            (
                Ratio::new(5, 4),
                Ratio::new(1, 4),
                "realizes(chord c major, exactly)".to_owned()
            ),
            (Ratio::new(3, 2), Ratio::new(1, 4), "within_ranges(2 ranges)".to_owned()),
            (Ratio::new(7, 4), Ratio::new(1, 4), "follows(satb_spacing)".to_owned()),
            (Ratio::new(2, 1), Ratio::new(1, 1), "fills_meter()".to_owned()),
        ],
        "each assertion carries what was written on it, and stands where the music before it ends"
    );
}

/// A claim nobody registered, an arity nobody declared, and a word nobody
/// spells — refused where they are written.
///
/// Four refusals in one law because they are one boundary: the registry is the
/// authority on what an `assert` may say, and all four are mistakes about *its*
/// vocabulary rather than about a type. They are read while the block is read,
/// which is what lets each one point at the source it is about — a pass that
/// met them after elaboration would have nothing to point at, because none of
/// the four ever becomes a term.
#[test]
fn an_assertion_is_refused_against_the_registrys_own_vocabulary() {
    let refused = |claim: &str| {
        let source = format!("piece \"bad\" {{ score {{ part p {{ voice v {{ assert {claim} {{ c4/4 }} }} }} }} }}");
        checked(&written(&source)).1
    };
    assert_eq!(
        refused("pitches_at(scale c major)"),
        ["UnknownName: nothing is claimed by `pitches_at`"],
        "a claim the registry does not have is not a claim"
    );
    assert_eq!(
        refused("voices(4, 5)"),
        ["WrongArity: `voices` takes one argument, and 2 were written"],
        "and one it does have is written with what it takes"
    );
    assert_eq!(
        refused("realizes(chord c major, may_drop)"),
        ["UnknownWord: `may_drop` is not a realization policy"],
        "a policy is one of three words"
    );
    assert_eq!(
        refused("follows(satb_tessitura)"),
        ["UnknownWord: `satb_tessitura` is not a rule this claim can check"],
        "and a rule id is one of the rules an assertion may name"
    );
}

/// A value argument is checked at the type the registry declares for it.
///
/// And checked *when it is evaluated*, which is the arrangement rather than a
/// gap: an assertion's arguments are terms, and a term has no type until the
/// document it stands in has a context to check it in. The reading records the
/// annotation; [`Document::passage`] is where it is checked, the same call that
/// turns the two track terms into a position and an extent.
///
/// `voices` takes a `Nat`, so a scale written where the count goes is a
/// conversion the core refuses against the registry's own declaration — one
/// answer to "what may stand here", from the row that declares it, rather than
/// a table of expected types beside it.
#[test]
fn a_value_argument_is_checked_at_the_shape_the_claim_declares() {
    let node = written(
        "piece \"mistyped\" {
            score { part p { voice v {
                assert voices(scale c major) { c4/4 }
            } } }
        }",
    );
    let mut resolver = Resolver::new();
    let sources = [Source::own(&node)];
    let mut document = elaborate(&mut resolver, &sources, None).expect("the document elaborates");
    let read = document.piece(&mut resolver, &node, DOCUMENT).expect("the piece reads");
    let claim = read
        .parts
        .iter()
        .flat_map(|part| part.voices.iter())
        .flat_map(|voice| voice.claims.iter())
        .next()
        .expect("the assertion is recorded");
    let Err(error) = document.passage(claim) else {
        panic!("a scale is not a count")
    };
    let restated = crate::lower::refusals::restate(document.sites(), &error);
    assert_eq!(
        restated.code,
        musa_score::diagnose::Code::ConversionMismatch,
        "the count's own type is what refuses a scale: {}",
        restated.message
    );
}

/// `senza` stops the barlines and puts back the meter that was in force.
///
/// The two halves are what makes it sugar rather than a mechanism, and the
/// second half is the one worth a law: the meter that resumes is `3/4` because
/// the piece's header says so, and a reading that restored a default would have
/// silently rebarred everything after the cadenza. Nothing here consults a
/// cursor — the meter travels down with the reading, and the two facts are two
/// ordinary statements of the fold.
#[test]
fn senza_stops_the_meter_and_restores_the_one_in_force() {
    let (whole, _) = sounding(
        "piece \"cadenza\" {
            meter 3/4;
            score { part solo { voice one {
                c4/4
                senza { d4/8 e4/8 }
                f4/4
            } } }
        }",
    );
    assert_eq!(
        placed(&whole),
        ["0 c4", "1/4 0/4", "1/4 d4", "3/8 e4", "1/2 3/4", "1/2 f4", "0 3/4"],
        "the braces open unmeasured at 1/4 and close back into three-four at 1/2"
    );
}

/// The readback is the music, not a term that resembles it.
///
/// What a consumer receives is a `VoiceTrack`: the occurrences a voice folded,
/// at the exact written positions the fold placed them, in a track exactly as
/// long as what was written. Asserting the *positions* is the point — a term
/// that held the right facts in the wrong order would print the same way and
/// sound like a different piece.
///
/// The meter is in the list and is not an accident of the fixture: a piece that
/// writes no `meter` is in four-four at its own start, and a readback that
/// dropped the fact saying so would hand a consumer a piece with no time
/// signature.
#[test]
fn a_piece_reads_back_as_the_track_it_folded() {
    let (whole, _) = sounding(
        "piece \"read back\" {
            score { part solo { voice one { c4/4 d4/4 e4/2 } } }
        }",
    );
    assert_eq!(
        whole.duration().as_ratio(),
        Ratio::new(1, 1),
        "two quarters and a half is one whole"
    );
    assert_eq!(
        placed(&whole),
        ["0 c4", "1/4 d4", "1/2 e4", "0 4/4"],
        "each note starts where the one before it ended, under the meter the piece is in"
    );
}

/// A voice reads back on its own, and holds only what it sounds.
///
/// The lane a projection draws, and the reason [`super::Voice`] keeps a term of
/// its own: two parts sounding at once are one simultaneity in the piece and two
/// staves on a page, and a consumer that had only the piece's track would have to
/// take a normal form apart by scope to get them back. The header facts are
/// *absent* from a lane and present in the whole, which is the same split
/// `resolve::lower_header` and the lane loop already make.
#[test]
fn each_voice_reads_back_as_the_lane_it_is() {
    let (whole, lanes) = sounding(
        "piece \"two staves\" {
            score {
                part upper { voice one { c5/2 } }
                part lower { voice two { c3/4 g3/4 } }
            }
        }",
    );
    let shown: Vec<_> = lanes
        .iter()
        .map(|(name, track)| (name.as_str(), placed(track)))
        .collect();
    assert_eq!(
        shown,
        [
            ("upper.one", vec!["0 c5".to_owned()]),
            ("lower.two", vec!["0 c3".to_owned(), "1/4 g3".to_owned()]),
        ],
        "each lane is its own voice and nothing else"
    );
    assert_eq!(
        whole.duration(),
        lanes.first().expect("the upper part is a lane").1.duration(),
        "the piece is as long as its longest lane, not as long as their sum"
    );
}

/// Where each occurrence starts and what it is, compactly enough to read.
fn placed(track: &VoiceTrack) -> Vec<String> {
    track
        .occurrences()
        .iter()
        .map(|occurrence| {
            let at = occurrence.span().start().as_ratio();
            let what = match occurrence.payload().kind {
                FactKind::Note { ref pitch, .. } => pitch.to_string(),
                FactKind::Meter { numerator, denominator } => format!("{numerator}/{denominator}"),
                ref other => format!("{other:?}"),
            };
            format!("{at} {what}")
        })
        .collect()
}

/// `source`'s parts and voices, without checking the term they fold into.
///
/// Separate from [`piece`] because the numbering is what several laws are
/// about, and a law about numbering should not fail because a voice's body did.
fn structure(source: &str) -> (Option<super::Piece>, Vec<String>) {
    let node = written(source);
    let mut resolver = Resolver::new();
    let mut sites = crate::lower::Sites::default();
    let held = crate::lower::Lowering::new(&mut resolver, &mut sites).piece(&node, DOCUMENT, None);
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

/// An instance site makes a voice where it stands, named by its `as` name.
///
/// The other half of the positional rule this module's header states: the
/// written voice *after* a site keeps the number it would have had, which is
/// what "the numbering counts every item" was for. The site's own voice is
/// numbered where it was written, so a `make` between two voices makes a voice
/// there and nothing renumbers.
#[test]
fn an_instance_site_makes_a_voice_where_it_stands() {
    let (held, said) = structure(
        "template voice echo(root: Pitch) { root/4 }
        piece \"numbered\" {
            score {
                part strings {
                    voice first { c4/4 }
                    make echo(e4) as second;
                    voice third { g4/4 }
                }
            }
        }",
    );
    let held = held.unwrap_or_else(|| panic!("the piece reads: {said:?}"));
    let voices: Vec<(u32, &str)> = held
        .parts
        .first()
        .expect("the score writes one part")
        .voices
        .iter()
        .map(|voice| (voice.id, voice.name.as_str()))
        .collect();
    assert_eq!(
        voices,
        [(0, "first"), (1, "second"), (2, "third")],
        "the instance is the part's second voice, and the written one after it is still its third"
    );
}

/// What an instance sounds is the template's body with the site's arguments
/// bound, and every fact of it says which site made it.
///
/// Both halves of what a `make` is, in one piece. The binding half is what makes
/// two instances of one template two different tracks; the provenance half is
/// what makes Origin view able to tell either of them from a note a composer
/// wrote, and it is *first* in the path because everything a transform appends
/// happened inside the instance.
#[test]
fn an_instance_sounds_the_template_bound_to_its_own_arguments() {
    let (_, lanes) = sounding(
        "template voice echo(root: Pitch) { root/4 }
        piece \"twice\" {
            score {
                part strings {
                    make echo(e4) as high;
                    make echo(c4) as low;
                }
            }
        }",
    );
    let named: Vec<&str> = lanes.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(named, ["strings.high", "strings.low"], "two instances, two voices");
    let shown = format!("{:?}", lanes.first().expect("the part writes two voices").1);
    assert!(
        shown.contains("letter: E") && !shown.contains("letter: C"),
        "the argument the site gave, not the one the other site gave: {shown}"
    );
    assert!(
        shown.contains("expansion_path: [TemplateInstance { template: \"echo\", alias: \"high\""),
        "and every fact of it came from this site: {shown}"
    );
}

/// Two tied noteheads are one sound, and the tie is gone by the time anything
/// reads the voice.
///
/// Roadmap §6.3: a tie is duration structure rather than an annotation, so the
/// merged occurrence spans both noteheads and is spelled as the compound the
/// composer wrote. `~` is the only surface mark that means something about *two*
/// statements, which is why it takes two words — `tied` on the notehead, applied
/// where the note is read, and `joined` on the voice, applied where the whole of
/// it exists.
#[test]
fn a_tie_is_one_occurrence_spanning_both_noteheads() {
    let (_, lanes) = sounding(
        "piece \"held\" {
            score { part strings { voice line { c4/4 ~ c4/4 g4/4 } } }
        }",
    );
    let track = &lanes.first().expect("the part writes one voice").1;
    let sounded: Vec<(Ratio<i64>, Ratio<i64>, String)> = track
        .occurrences()
        .iter()
        .filter_map(|occurrence| {
            let pitch = occurrence.payload().pitch_of()?;
            Some((
                occurrence.span().start().as_ratio(),
                occurrence.span().end().as_ratio(),
                pitch.to_string(),
            ))
        })
        .collect();
    assert_eq!(
        sounded,
        [
            (Ratio::new(0, 1), Ratio::new(1, 2), "c4".to_owned()),
            (Ratio::new(1, 2), Ratio::new(3, 4), "g4".to_owned()),
        ],
        "the two quarters are one half, and the note after them is where it was"
    );
    assert!(
        track.occurrences().iter().all(|occurrence| !occurrence.payload().tied),
        "no fact leaves a voice still claiming to continue into something"
    );
}

/// A tie onto a different note, and a tie onto nothing, are both refused.
///
/// The two ways `~` can be written and mean nothing, and the reason `joined` is
/// applied at a voice rather than at each block: "nothing follows it" is only
/// answerable where there is nothing after, and a merge inside a `repeat` body
/// would report every tie that continues past the block as dangling.
#[test]
fn a_tie_that_joins_nothing_is_refused() {
    for (line, expected) in [
        ("c4/4 ~ g4/4", "a tie joins two of the same note"),
        ("c4/4 ~", "this tie has nothing to tie to"),
    ] {
        let (held, said) = checked(&written(&format!(
            "piece \"dangling\" {{ score {{ part strings {{ voice line {{ {line} }} }} }} }}"
        )));
        assert!(held.is_none(), "`{line}` was accepted: {said:?}");
        assert!(
            said.iter().any(|complaint| complaint.contains(expected)),
            "`{line}` reported something else: {said:?}"
        );
    }
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
    let node = musa_syntax::parse("let held = music { key g major; c4/4 };");
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
        ["a key change belongs to the piece, not to material"]
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
/// **No reason left here is a language fault.** Two remain and they are one
/// fact twice: `diatonic-sequences` and `rule-of-the-octave` build chord
/// voicings by index arithmetic over a scale, and doing that costs more
/// reduction steps than §4's default of 200,000 allows.
///
/// The size of it, measured by bisecting `Budget::LANGUAGE`'s step limit
/// against this survey and reading which examples still say anything:
///
/// | limit | examples that exhaust |
/// | --- | --- |
/// | 50,000 | `diatonic-sequences`, `in-c`, `rule-of-the-octave`, `shuffle` |
/// | 100,000 | `diatonic-sequences`, `rule-of-the-octave` |
/// | 215,000 | `diatonic-sequences`, `rule-of-the-octave` |
/// | 220,000 | `rule-of-the-octave` |
/// | 255,000 | `rule-of-the-octave` |
/// | 260,000 | none |
///
/// So it is a tail of two and not a slope. Every other example fits inside half
/// the limit, `examples/in-c.musa` — 590 lines, the longest in the corpus —
/// among them, which is what says there is no base cost the whole corpus pays
/// and nothing here that is quadratic in the length of the music. The two that
/// do not fit want 1.1× and 1.3× of it.
///
/// **This is the one thing standing between the survey and 142's Check**, and
/// it is recorded rather than answered because both ways of answering it belong
/// to prompt 164. Making the checker spend fewer steps on this shape is 144's
/// performance half, which already names the left-nested `follow` spine
/// ([`crate::lower::notation`]) and the re-normalization conversion does.
/// Raising 200,000 is a cost-table version bump: `02-core-calculus.md` §4 calls
/// it a language-version constant, and 144 says in as many words that it is
/// "never a threshold quietly raised to make the suite pass". A migration may
/// not do the second and cannot do the first in passing, so the honest record
/// is this table.
///
/// Everything else holds on the real corpus: fifty-four pieces' worth of
/// notation statements, motifs, fragments, transformations, part and voice
/// numbering, header and per-part context, and the scope every fact in them is
/// constructed at.
///
/// Three language faults were here and went with the migration itself. **A
/// scale the phrase does not name** — a `step` in a free `music { … }` — went
/// when the contextual `music` value did: every `step` in the corpus now stands
/// inside the `in scale` that gives it a coordinate system, which is what
/// `scale-context.musa` was always saying. **An argument written by name** —
/// `Against(first: …, second: …)` at a *use* in `gesture-data` — went by
/// dropping the labels: a field's name is written at its declaration, and
/// repeating it at a use was a second way to pass an argument. And **a trait a
/// document cannot use in itself** — `transposed` on the receiver of
/// `triad_root(refined) up M3` in `stdlib/src/transformational.musa` — went at
/// prompt 141r, which declares a document's instances with its definitions
/// rather than after them, so `10-traits.md` §6's exact-receiver lookup finds
/// the instance at the head where the reading always said it should.
///
/// Five classes were here and are gone. The **events quote** —
/// `events EventTrack[WrittenTime, ScoreFact] { … }`, which
/// [`crate::lower::events`] reads: the reading answers everything the quote can
/// be wrong about, the term rides in a literal, and one builtin binds the holes
/// and evaluates. The **instance site** — a `make` among
/// a part's items — went with the whole of what a `make` is: the template's
/// body read once at the site's own scope, its parameters λ-bound to the
/// argument expressions the site wrote, and the result wrapped in `instanced`
/// so that every fact it produces carries the site's step first. The expansion
/// path 141k's Stop left to this prompt turned out not to need a number `Sites`
/// hands out, because provenance is stamped where the facts are *made* rather
/// than where the term is read.
///
/// A notation statement whose argument is
/// a *name* — `root/4` in `motif turn(root: Pitch)`, `key k;` and `in scale
/// mode` in a `template piece` — which 141k's fold reported as though a literal
/// had been misspelled, because it folded to a value while it walked and a
/// parameter has no value until an instance site supplies one; 142 answers it by
/// reading those three positions as terms (see [`crate::lower::notation`]). The
/// anonymous product, where only the *type* half was ever missing: `(a, b)` has
/// lowered since 141g, and both halves now read the `Pair` the prelude declares,
/// so a written product is canonical data rather than a structural record.
///
/// And the **assertion**. `assert`'s claim, its arity, and its two *word*
/// arguments are read where they are written, and its value arguments are
/// annotated with the type the registry declares and left as terms;
/// [`crate::document::Document::passage`] evaluates them and builds the claim.
/// `bar` and `senza` were beside it in that class and went earlier: a bar's
/// braces erase and its claim is placed by the term standing before it, and a
/// `senza` is `meter none`, the body, and the meter the reading carried down.
#[test]
fn every_example_elaborates() {
    let libraries = crate::document::laws::library_sources();
    let mut said: Vec<String> = Vec::new();
    for (name, source) in EXAMPLES {
        let parsed = musa_syntax::parse(source);
        assert!(parsed.errors().is_empty(), "`{name}` parses: {:?}", parsed.errors());
        let Some(node) = piece_of(&parsed.syntax()) else {
            continue;
        };
        let (_, reasons) = checked_with(&node, &libraries);
        let _ = &libraries;
        said.extend(reasons);
    }
    said.sort();
    said.dedup();
    // The charge site moves with the engine: under the course correction's
    // checker the 200001st step lands at the evaluation walk itself rather
    // than inside an application of the staff adapter. One ResourceLimit
    // remains the corpus's only failure either way.
    assert_eq!(
        said,
        ["ResourceLimit: evaluation exceeded the budget for reduction steps at 200001 of 200000"],
        "the corpus reads and checks; what is left is the step budget, and the adapter's rewrite owns it"
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
        "events-splice",
        include_str!("../../../../../examples/events-splice.musa"),
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
