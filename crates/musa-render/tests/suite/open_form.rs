//! What the three notation backends do with a freedom they have no element
//! for (`docs/rules/kernel/07-backend-contract.md`).
//!
//! The contract is not "it works" — none of the three formats has a mobile
//! form, a free-duration bracket, or an improvised region. It is that each
//! carries the *realized* music plus the instruction as text, and says once,
//! out loud, what it could not say. A silent lossy export is the failure this
//! file exists to prevent.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, SourceDocument, compile};

use musa_render::{NotationOptions, NotationTarget, plan_notation, render_notation};
use musa_score::{Realization, ScoreSnapshot};

const TARGETS: [NotationTarget; 3] = [NotationTarget::Mei, NotationTarget::LilyPond, NotationTarget::MusicXml];

const OPEN: &str = "piece \"Open\" { tempo 1/4 = 60; meter 4/4; key c major;
    fragment alpha { c5/1 }
    fragment beta { d5/1 }
    score { part p { voice v {
        mobile { alpha; beta; }
        improvise 2/1 over \"Dm7 | G7\";
        e5/1 to 4/1
    } } } }";

fn score_of(text: &str) -> ScoreSnapshot {
    compile(
        &SourceDocument::new(text, "open.musa"),
        &CompileOptions {
            realization: Realization::seeded(11),
            ..CompileOptions::default()
        },
    )
    .into_snapshot()
    .expect("compiles")
}

/// Every backend says what it dropped, once per kind and never more.
#[test]
fn each_target_reports_every_loss_exactly_once() {
    let score = score_of(OPEN);
    for target in TARGETS {
        let rendered = render_notation(&score, target, &NotationOptions::default()).expect("renders");
        let warnings = rendered.warnings();
        assert_eq!(warnings.len(), 3, "{target:?}: {warnings:?}");
        for expected in ["mobile form", "improvised region", "free-duration bracket"] {
            assert_eq!(
                warnings.iter().filter(|warning| warning.contains(expected)).count(),
                1,
                "{target:?} said nothing (or said it twice) about {expected}: {warnings:?}"
            );
        }
    }
}

/// A piece that leaves nothing open loses nothing, so the channel stays quiet
/// for every score anyone has written until now.
#[test]
fn a_determinate_piece_warns_about_nothing() {
    let score = score_of(include_str!("../../../../examples/counterpoint.musa"));
    for target in TARGETS {
        let rendered = render_notation(&score, target, &NotationOptions::default()).expect("renders");
        assert!(rendered.warnings().is_empty(), "{target:?}: {:?}", rendered.warnings());
    }
}

/// The instruction reaches the page in every format, in words a reader reads.
#[test]
fn the_instruction_is_printed_even_where_the_format_cannot_draw_it() {
    let score = score_of(OPEN);
    for target in TARGETS {
        let rendered = render_notation(&score, target, &NotationOptions::default()).expect("renders");
        let text = rendered.text();
        assert!(text.contains("any order"), "{target:?} printed no mobile instruction");
        assert!(
            text.contains("improvise over Dm7"),
            "{target:?} printed no improvise instruction"
        );
        assert!(text.contains("hold to 4"), "{target:?} printed no hold instruction");
    }
}

/// And the realized music is there under it: an export is a reading of the
/// work, not a note saying the work was open.
#[test]
fn the_realized_music_is_exported_under_the_instruction() {
    let plan = plan_notation(&score_of(OPEN), &NotationOptions::default()).expect("plans");
    let [mobile, improvise] = plan.open() else {
        panic!("expected two open regions: {:?}", plan.open());
    };
    assert_eq!((mobile.from, mobile.to), (1, 2));
    assert_eq!((improvise.from, improvise.to), (3, 4));
    assert_eq!(plan.holds().len(), 1);
    // Both fragments are on the page, whichever order they were played in.
    let sounded: usize = plan
        .staves()
        .iter()
        .flat_map(|staff| staff.measures())
        .flat_map(|measure| measure.lanes())
        .map(|lane| lane.items().len())
        .sum();
    assert!(sounded >= 5, "the reading was not written out: {sounded} symbols");
}
