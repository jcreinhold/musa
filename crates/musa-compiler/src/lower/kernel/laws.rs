//! What reading a kernel quote promises.
//!
//! Beside the module rather than in `tests/suite/`, for
//! [`crate::lower::piece::laws`]'s reason: what the reading answers is a
//! [`musa_core::Raw`] and what it assembles is a [`crate::elaborate::VoiceTrack`],
//! and neither leaves this crate.
//!
//! The division of labour with `tests/suite/kernel_quote_laws.rs` is the one the
//! module documentation states. That suite holds the *promises* — the quotation
//! locus clause by clause, hygiene, exactness, and each bypass a raw transform
//! opens — through the whole compiler. What is here is the reading's own share:
//! that the term becomes a `spliced` call over material `instanced` stamped,
//! that the facts written raw in the quote carry the quote's answer to scope and
//! origin, and that each refusal lands where it was written.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use num_rational::Ratio;

use crate::document::{Source, elaborate};
use crate::elaborate::{FactKind, VoiceTrack};
use crate::origin::ExpansionStep;
use crate::resolve::Resolver;

/// A piece whose one voice uses a quote of `body`, assembled and read back.
///
/// Through [`crate::document::Document::track`] rather than through the raw term
/// this module writes, because a law that matched on the application would be
/// checking the spelling rather than the music. What comes back is what a
/// consumer gets.
fn assembled(quote: &str) -> (Option<VoiceTrack>, Vec<String>) {
    let source = format!(
        "piece \"quoted\" {{\n\
         let subject = music {{ c4/4 }};\n\
         let held = {quote};\n\
         score {{ part strings {{ voice line {{ use held; }} }} }}\n\
         }}"
    );
    let document = musa_language::parse(&source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    let node = document
        .syntax()
        .descendants()
        .find(|node| node.kind() == musa_language::SyntaxKind::PieceDecl)
        .expect("the source writes a piece");
    let mut resolver = Resolver::new();
    let sources = [Source::own(&node)];
    let track = elaborate(&mut resolver, &sources, None).and_then(|mut elaborated| {
        let read = elaborated.piece(&mut resolver, &node, "law")?;
        elaborated.track(&read.track).ok()
    });
    let mut said: Vec<String> = resolver
        .diagnostics
        .iter()
        .map(|complaint| complaint.message.clone())
        .collect();
    said.sort();
    said.dedup();
    (track, said)
}

/// The same, at the one instantiation this build has.
fn score(body: &str) -> (Option<VoiceTrack>, Vec<String>) {
    assembled(&format!("kernel EventTrack[WrittenTime, ScoreFact] {body}"))
}

/// Every note the quote assembled: onset, spelling, and the locus its expansion
/// path records.
fn notes(body: &str) -> Vec<(Ratio<i64>, String, Option<Ratio<i64>>)> {
    let (track, said) = score(body);
    let track = track.unwrap_or_else(|| panic!("the quote assembles: {said:?}"));
    let mut seen: Vec<(Ratio<i64>, String, Option<Ratio<i64>>)> = track
        .occurrences()
        .iter()
        .filter_map(|occurrence| {
            let fact = occurrence.payload();
            let FactKind::Note { ref pitch, .. } = fact.kind else {
                return None;
            };
            let locus = fact.origin.expansion_path.iter().find_map(|step| match *step {
                ExpansionStep::KernelSplice { at } => Some(at),
                _ => None,
            });
            Some((occurrence.span().start().as_ratio(), pitch.to_string(), locus))
        })
        .collect();
    seen.sort_by(|left, right| left.partial_cmp(right).expect("rationals are totally ordered"));
    seen
}

/// The messages a quote that cannot be read reports.
fn refused(quote: &str) -> Vec<String> {
    let (track, said) = assembled(quote);
    assert!(track.is_none(), "the quote was accepted: {said:?}");
    said
}

/// The same, at the one instantiation this build has.
fn refuses(body: &str) -> Vec<String> {
    refused(&format!("kernel EventTrack[WrittenTime, ScoreFact] {body}"))
}

// ---- what a quote assembles ----

/// Raw material and spliced material end up in one track, at the exact times the
/// term places them.
///
/// The point of the form, and the thing no surface constructor spells: the `g2`
/// is written as a payload rather than as a note, and the `c4` arrives through a
/// hole from ordinary Musa that knows nothing about being quoted.
#[test]
fn a_quote_assembles_written_payloads_and_spliced_music_into_one_track() {
    let seen = notes(
        "{
            together {
                shift by 1/2 ${subject};
                track 2 { occurrence \"note g2 1\" from 0 to 1; };
            }
        }",
    );
    let placed: Vec<(Ratio<i64>, &str)> = seen.iter().map(|(onset, pitch, _)| (*onset, pitch.as_str())).collect();
    assert_eq!(
        placed,
        [(Ratio::new(0, 1), "g2"), (Ratio::new(1, 2), "c4")],
        "the term's own placement is what the track holds"
    );
}

/// Every fact that leaves a quote records where it was assembled: the quote's
/// own zero for what the quote wrote, and the hole's locus for what a `${…}`
/// brought.
///
/// The reading's half of `tests/suite/kernel_quote_laws.rs`'s locus law. The two
/// steps are stamped by two different things — the raw payloads here, the
/// spliced ones by `instanced` at evaluation — so a law that saw only one of
/// them would pass with the other missing.
#[test]
fn every_assembled_fact_records_the_locus_it_came_in_at() {
    let seen = notes(
        "{
            together {
                shift by 1/2 ${subject};
                track 2 { occurrence \"note g2 1\" from 0 to 1; };
            }
        }",
    );
    let recorded: Vec<(&str, Option<Ratio<i64>>)> =
        seen.iter().map(|(_, pitch, locus)| (pitch.as_str(), *locus)).collect();
    assert_eq!(
        recorded,
        [("g2", Some(Ratio::new(0, 1))), ("c4", Some(Ratio::new(1, 2)))],
        "raw material is assembled at the quote's zero, spliced material at its hole"
    );
}

/// A hole's material keeps the steps it already had, under the splice step —
/// and all of it stands under the `use` that played the quote.
///
/// `instanced` prepends, so a `stretch` written inside the hole is a step the
/// facts accrued *after* they were spliced, and the `use held;` that speaks the
/// whole quote by name is the step they accrued last of all. That is the order
/// they happened in, and the order Origin view reads them in.
#[test]
fn a_holes_own_steps_stand_under_the_splice_step() {
    let (track, said) = score("{ ${stretch(1/2, subject)} }");
    let track = track.unwrap_or_else(|| panic!("the quote assembles: {said:?}"));
    let path: Vec<String> = track
        .occurrences()
        .iter()
        .flat_map(|occurrence| occurrence.payload().origin.expansion_path.iter())
        .map(|step| format!("{step:?}"))
        .collect();
    assert_eq!(
        path.len(),
        3,
        "the use that played it, one splice step, and one stretch: {path:?}"
    );
    assert!(
        path.first().is_some_and(|step| step.starts_with("MotifApplication")),
        "the voice spoke the quote by name first: {path:?}"
    );
    assert!(
        path.get(1).is_some_and(|step| step.starts_with("KernelSplice")),
        "the splice came next: {path:?}"
    );
    assert!(
        path.get(2).is_some_and(|step| step.starts_with("Stretch")),
        "and the stretch happened inside it: {path:?}"
    );
}

// ---- what a quote may not do ----

/// The three type words are this language's, so they are answered before the
/// body is handed to a reader that has never heard of them.
///
/// The coordinate is checked before the payload because it is the stronger
/// claim: a track in performed time is not a score whatever its payloads say.
#[test]
fn a_quote_this_build_cannot_instantiate_is_refused_at_the_word() {
    for (head, expected) in [
        ("Timeline[WrittenTime, ScoreFact]", "not a kernel type constructor"),
        ("EventTrack[PerformedTime, ScoreFact]", "cannot be written in"),
        ("EventTrack[WrittenTime, Sample]", "`Sample` payloads"),
    ] {
        let reported = refused(&format!("kernel {head} {{ track 1 {{ }} }}"));
        assert!(
            reported.iter().any(|message| message.contains(expected)),
            "`{head}` was accepted: {reported:?}"
        );
    }
}

/// A quote may not settle what its use is entitled to settle.
///
/// Both halves of one idea: a context-authoritative payload would change the
/// caller's meter from inside a value, and a payload naming a voice would choose
/// where the material lands. The reading stamps scope and origin a few lines
/// later, so refusing first is what keeps the overwriting from being silent.
#[test]
fn a_quote_that_settles_its_uses_questions_is_refused() {
    for (raw, expected) in [
        ("occurrence \"key c major\" from 0 to 1;", "a key"),
        ("occurrence \"meter 4/4\" from 0 to 1;", "a meter"),
        (
            "occurrence \"voice 0 0 note c4 1 [0:0]\" from 0 to 1;",
            "a voice of its own",
        ),
    ] {
        let reported = refuses(&format!("{{ track 1 {{ {raw} }} }}"));
        assert!(
            reported.iter().any(|message| message.contains(expected)),
            "`{raw}` was accepted: {reported:?}"
        );
    }
}

/// A quote is closed: every name it uses is one it binds, and `${…}` is how the
/// outside gets in.
///
/// Asked of the quote rather than of the use, which is why the reading binds the
/// holes to empty material before checking: a free `subject` is a complaint
/// about this quote even though the piece around it binds one.
#[test]
fn a_quote_that_names_what_it_does_not_bind_is_refused() {
    let reported = refuses("{ together { subject; } }");
    assert!(
        reported
            .iter()
            .any(|message| message.contains("does not stand on its own")),
        "a free name was accepted: {reported:?}"
    );
}
