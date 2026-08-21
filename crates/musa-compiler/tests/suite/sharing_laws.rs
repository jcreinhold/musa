//! What makes two calls one body.
//!
//! Prompt 142 moved the surface onto a core program, and
//! `docs/rules/events/06-surface-elaboration.md` §Sharing was repaired with it:
//! a motif is one core definition applied at each call site, and the event track
//! term the compiler prints is a *projection* of the evaluated result rather
//! than the shape elaboration was carried in. Counting `let shared` bindings in
//! that text is therefore no longer a measurement of anything, and these laws
//! no longer do it. They state the same claims where a composer can still see
//! them: a body is **read once**, so a body written wrong is one complaint
//! however many calls ask for it, and each call carries **its own site** in the
//! provenance, so the Origin view can still say which `use` produced a note.
//!
//! Two of the five laws that stood here are gone, and neither quietly.
//!
//! `one_body_read_under_two_scales_is_two_bodies` asserted that a motif read
//! under C major and under C dorian must be two bindings, because the sharing
//! key had to keep the two readings apart. There is no sharing key, and there
//! is no second reading either: `scale_context_laws.rs`'s
//! `a_phrase_that_steps_outside_a_scale_is_refused_where_it_is_written` fixes
//! that an `in scale` at a call site does not reach into a saved body, and that
//! a `step` written in one is refused at the definition. The law's premise was
//! the contextual reading prompt 127a deleted.
//!
//! `one_more_call_of_a_shared_body_charges_one_more_body` asserted that the
//! occurrence meter charges every call rather than every body, so that a piece
//! is never accepted for being written in a way the compiler happens to like.
//! It is gone because the meter is: nothing in the new lowering calls
//! `WorkMeter::output`, so the million-occurrence limit
//! `docs/rules/language/06-elaboration-baseline.md` fixes is not charged, and a nullary
//! motif called four hundred times costs about what one call costs. That is a
//! hole, it is recorded as one, and it is not this file's to close — the meter
//! is `resource_validation.rs`'s subject and prompt 165 re-measures it. A law
//! stated here against a meter that does not run would have hidden it.

// A failure is more useful reported with what actually happened than with an
// assertion message alone.
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_score::ExpansionStep;

/// A piece whose one voice holds `body`, with `declarations` above the score.
fn piece(declarations: &str, body: &str) -> String {
    format!(
        "piece \"Sharing\" {{\n    meter 4/4;\n    key c major;\n{declarations}\n    score {{ part p {{ voice v {{\n{body}\n    }} }} }}\n}}\n"
    )
}

/// Every complaint the compiler makes about `source`, by code, span, and
/// message, in a stable order.
///
/// The span is half the measurement: a body elaborated once per call site says
/// the same thing about the same text once per call, and a list that dropped
/// the span could not tell that from one complaint repeated for other reasons.
fn complaints(source: &str) -> Vec<String> {
    let compilation = compile(
        &SourceDocument::new(source, "sharing-laws.musa"),
        &CompileOptions::default(),
    );
    let mut said: Vec<String> = compilation
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            let at = diagnostic
                .labels
                .first()
                .map_or_else(|| "-".to_owned(), |label| format!("{:?}", label.span));
            format!("{:?} {at} {}", diagnostic.code, diagnostic.message)
        })
        .collect();
    said.sort();
    said
}

/// Every distinct `use` site the compiled score's provenance names.
fn call_sites(source: &str) -> BTreeSet<String> {
    let compilation = compile(
        &SourceDocument::new(source, "sharing-laws.musa"),
        &CompileOptions::default(),
    );
    let Some(snapshot) = compilation.snapshot() else {
        panic!("the piece elaborates: {:?}", complaints(source));
    };
    snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice).collect::<Vec<_>>())
        .flat_map(|voice| voice.events().to_vec())
        .flat_map(|event| event.origin.expansion_path)
        .filter_map(|step| match step {
            ExpansionStep::MotifApplication { call_site } => Some(format!("{call_site:?}")),
            ExpansionStep::RepeatIteration(_)
            | ExpansionStep::Transposition(_)
            | ExpansionStep::Stretch(_)
            | ExpansionStep::Retrograde
            | ExpansionStep::Inversion { .. }
            | ExpansionStep::MapNotePitches
            | ExpansionStep::ScaleContext { .. }
            | ExpansionStep::TemplateInstance { .. }
            | ExpansionStep::Assertion { .. }
            | ExpansionStep::EventsSplice { .. }
            | ExpansionStep::Specialization { .. } => None,
        })
        .collect()
}

/// A motif whose body cannot be read, and the calls that ask for it.
///
/// The fault is a `step` with no collection in force, which
/// `scale_context_laws.rs` fixes as a property of the *declaration*: it is
/// refused where it is written, and no call site can supply what it wants. That
/// is what makes it the right probe here — a body elaborated per call would
/// have to say it once per call, and a body elaborated once says it once.
fn unreadable_body(parameter: &str, calls: &str) -> String {
    piece(&format!("    motif cell({parameter}) {{ (c5 step 1)/4 c5/4 }}"), calls)
}

#[test]
fn a_body_written_wrong_is_one_complaint_however_many_calls_ask_for_it() {
    // The call-site gap, closed, and closed at the stage that always owned it.
    // Four `use`s of one motif denote one piece of music four times over, and
    // the body behind them is one core definition — so the text inside it is
    // read, resolved, and checked once, which is what the repaired §Sharing
    // means by "the saving is in elaboration".
    let once = complaints(&unreadable_body("", "        use cell();\n"));
    assert_eq!(once.len(), 1, "one body written wrong is one complaint: {once:?}");
    assert_eq!(
        complaints(&unreadable_body("", &"        use cell();\n".repeat(4))),
        once,
        "and four calls of it are the same one complaint, at the same span"
    );
}

#[test]
fn a_parameterized_body_is_one_body_whatever_it_is_given() {
    // What a body costs is no longer set by the arguments it is given. The
    // replaced elaborator bound one body per distinct argument tuple, and the
    // law that stood here measured that residual duplication as "the shape of
    // the full-laziness gap". A core definition is one definition and takes its
    // arguments, so the gap is closed rather than bounded: eight calls under
    // two arguments read the body once, not twice.
    let calls = ["c5", "d5"]
        .iter()
        .cycle()
        .take(8)
        .fold(String::new(), |mut calls, root| {
            let _ = writeln!(calls, "        use cell({root});");
            calls
        });
    assert_eq!(
        complaints(&unreadable_body("root: Pitch", &calls)),
        complaints(&unreadable_body("root: Pitch", "        use cell(c5);\n")),
        "eight calls and two arguments are one body, so one complaint"
    );
}

#[test]
fn each_call_carries_its_own_site() {
    // Provenance no longer travels on a reference's mark, because there is no
    // shared body whose occurrences would collide: every one is built by the
    // voice's own left fold and carries its own `Origin`. What the Origin view
    // promised is unchanged — three `use`s are three sites, and a note knows
    // which of them made it.
    let sites = call_sites(&piece(
        "    motif cell() { c5/4 d5/4 e5/4 f5/4 }",
        &"        use cell();\n".repeat(3),
    ));
    assert_eq!(sites.len(), 3, "three calls, three call sites: {sites:?}");
}
