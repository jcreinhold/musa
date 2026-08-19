//! R1 and the identity that makes it usable (`docs/rules/kernel/11-realization.md`).
//!
//! ```text
//! R1.  Same source and same realization ⇒ same term, same normal form,
//!      same semantic hash, byte-identical exports.
//! ```
//!
//! Two halves, and both are tested here because either alone is worthless. R1
//! says a performance is *reproducible*; path identity says it *survives an
//! edit somewhere else*. A design with the first and not the second re-rolls
//! the piece on every keystroke, which is the failure the whole document
//! exists to prevent.

// A failure of these is a bug in the fixture, not in a caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{
    ChoicePath, ChoiceStep, CompileOptions, Decision, Realization, SourceDocument, compile, kernel_normal_form,
    kernel_text,
};

const LOOP_LENGTHS: &str = include_str!("../../../../examples/loop-lengths.musa");

/// Every `.musa` fixture that decides nothing, by name.
const DETERMINATE: &[(&str, &str)] = &[
    ("canon", include_str!("../../../../examples/canon.musa")),
    ("counterpoint", include_str!("../../../../examples/counterpoint.musa")),
    ("invention", include_str!("../../../../examples/invention.musa")),
    ("modulation", include_str!("../../../../examples/modulation.musa")),
    ("twinkle", include_str!("../../../../examples/twinkle.musa")),
    ("variation", include_str!("../../../../examples/variation.musa")),
];

fn under(source: &str, name: &str, realization: &Realization) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(source, name),
        &CompileOptions {
            realization: realization.clone(),
            ..CompileOptions::default()
        },
    )
}

/// R1's first clause, and the one a session's recompilation keys on.
#[test]
fn the_same_seed_is_the_same_performance() {
    let first = under(LOOP_LENGTHS, "loop-lengths", &Realization::seeded(42));
    let second = under(LOOP_LENGTHS, "loop-lengths", &Realization::seeded(42));
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.decisions(), second.decisions());
    // And down to the bytes of the interchange file, which is what a consumer
    // reproducing the file is promised.
    let document = SourceDocument::new(LOOP_LENGTHS, "loop-lengths");
    assert_eq!(
        kernel_text(&document, &Realization::seeded(42), &musa_compiler::ImportSources::default()),
        kernel_text(&document, &Realization::seeded(42), &musa_compiler::ImportSources::default())
    );
}

/// The other side of R1: a different reading is a different piece, and the
/// hash says so. Without this the design would be reproducible and useless —
/// a session keyed on the identity would not recompile when the performance
/// changed.
#[test]
fn a_different_seed_is_a_different_performance() {
    let mut identities = std::collections::BTreeSet::new();
    for seed in 0..64u64 {
        let compiled = under(LOOP_LENGTHS, "loop-lengths", &Realization::seeded(seed));
        assert_eq!(
            compiled.diagnostics().len(),
            0,
            "seed {seed}: {:?}",
            compiled.diagnostics()
        );
        identities.insert(compiled.identity());
    }
    assert!(
        identities.len() > 1,
        "64 seeds produced one performance, so nothing was decided"
    );
}

/// A piece that asks no question never consults the realization, so the seed
/// is not a hidden input to any existing fixture.
///
/// This is what lets the corpus pin a seed (`kernel_interop.rs`) without
/// pinning anything for the pieces that do not need it.
#[test]
fn a_determinate_piece_is_the_same_under_every_seed() {
    for &(name, source) in DETERMINATE {
        let document = SourceDocument::new(source, name);
        let quiet = kernel_normal_form(&document, &Realization::deterministic(), &musa_compiler::ImportSources::default()).expect("elaborates");
        for seed in [1u64, 42, 999, u64::MAX] {
            let realization = Realization::seeded(seed);
            assert_eq!(
                kernel_normal_form(&document, &realization, &musa_compiler::ImportSources::default()).expect("elaborates"),
                quiet,
                "{name}: seed {seed} changed a piece that decides nothing"
            );
            assert_eq!(
                kernel_text(&document, &realization, &musa_compiler::ImportSources::default()),
                kernel_text(&document, &Realization::deterministic(), &musa_compiler::ImportSources::default()),
                "{name}: seed {seed} changed the interchange file"
            );
            assert!(
                under(source, name, &realization).decisions().is_empty(),
                "{name}: a determinate piece recorded a decision"
            );
        }
    }
}

/// A pin beats the seed, which is what makes a realization editable rather
/// than a lottery ticket.
#[test]
fn a_pinned_site_ignores_the_seed() {
    let site = ChoicePath::default().then(ChoiceStep::Ordinal(0));
    for seed in 0..16u64 {
        let mut realization = Realization::seeded(seed);
        realization.pin(site.clone(), Decision::Count(5));
        let compiled = under(LOOP_LENGTHS, "loop-lengths", &realization);
        let [decided] = compiled.decisions() else {
            panic!("seed {seed}: expected one decision, got {}", compiled.decisions().len());
        };
        assert_eq!(decided.path(), &site, "seed {seed} beat the pin");
        assert_eq!(decided.decision(), &Decision::Count(5), "seed {seed} beat the pin");
        // And the record says the answer was kept rather than drawn, which is
        // the difference the Origin view prints.
        assert!(decided.pinned(), "seed {seed}: a pinned site read as drawn");
    }
}

/// Path identity's whole purpose: an edit somewhere else must not re-roll the
/// performance.
///
/// A named bar is inserted *above* the decision site, which is exactly the
/// edit a source span or a `DeclarationId` would fail — the span moves, and
/// the declaration renumbers. The names above the site do not.
#[test]
fn inserting_a_bar_above_a_site_leaves_its_decision_alone() {
    let before = under(LOOP_LENGTHS, "loop-lengths", &Realization::seeded(7));
    // In every voice, because a repeat the page draws is one repeat of the
    // whole system: an intro written into one voice alone would not be an
    // unrelated edit, it would be a different piece.
    let edited = LOOP_LENGTHS
        .replace(
            "voice kick {",
            "voice kick {\n                bar kick_intro { c2/4 c2/4 c2/4 c2/4 }",
        )
        .replace(
            "voice hats {",
            "voice hats {\n                bar hats_intro { rest/1 }",
        )
        .replace("voice line {", "voice line {\n                bar bass_intro { a1/1 }");
    assert_ne!(edited, LOOP_LENGTHS, "the edit did not apply");
    let after = under(&edited, "loop-lengths", &Realization::seeded(7));
    assert_eq!(after.diagnostics().len(), 0, "{:?}", after.diagnostics());
    // Paths and answers, not the whole record: the *spans* moved, which is
    // precisely what a source-span identity would have re-rolled on and what
    // path identity exists to be independent of.
    let answers = |compiled: &musa_compiler::Compilation| -> Vec<(String, String)> {
        compiled
            .decisions()
            .iter()
            .map(|record| (record.path().canonical(), record.decision().to_string()))
            .collect()
    };
    assert_eq!(
        answers(&before),
        answers(&after),
        "an edit above the site changed its decision"
    );
    // The music did change — otherwise the test proves nothing about edits.
    assert_ne!(before.identity(), after.identity());
}

/// One site, one decision, however many voices reach it.
///
/// A repeat barline crosses the system, so a repeat the page can draw takes
/// one count for the whole piece. Three voices each writing the same ranged
/// repeat therefore stay together — which is checked here by the *absence*
/// of the disagreement warning as much as by the count.
#[test]
fn voices_that_write_the_same_ranged_repeat_agree_on_it() {
    for seed in 0..32u64 {
        let compiled = under(LOOP_LENGTHS, "loop-lengths", &Realization::seeded(seed));
        assert_eq!(
            compiled.decisions().len(),
            1,
            "seed {seed}: three voices decided {} times",
            compiled.decisions().len()
        );
        assert!(
            compiled.diagnostics().is_empty(),
            "seed {seed}: {:?}",
            compiled.diagnostics()
        );
    }
}

/// A range that counts downwards is a mistake about the music, and is refused
/// rather than compiled as silence.
#[test]
fn a_backwards_range_is_refused() {
    let source = "piece \"x\" { score { part p { voice v { repeat 6 to 2 { c4/4 } } } } }";
    let compiled = under(source, "backwards", &Realization::seeded(1));
    assert!(
        compiled
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("counts upwards")),
        "{:?}",
        compiled.diagnostics()
    );
}

/// The interchange file says which reading of the work it projects — and says
/// nothing when there is nothing to say.
#[test]
fn the_file_names_the_realization_that_produced_it() {
    let text = kernel_text(
        &SourceDocument::new(LOOP_LENGTHS, "loop-lengths"),
        &Realization::seeded(42),
        &musa_compiler::ImportSources::default(),
    )
    .expect("elaborates");
    let notes: Vec<&str> = musa_kernel::notes(&text).collect();
    assert!(notes.contains(&"realization seed=42 pins=0"), "{notes:?}");
    assert!(
        notes.iter().any(|note| note.starts_with("decision ")),
        "no decision recorded: {notes:?}"
    );
    let report = musa_compiler::check_kernel_text(&text).expect("reads back");
    assert_eq!(report.realization.as_deref(), Some("seed=42 pins=0"));

    let determinate = kernel_text(
        &SourceDocument::new(include_str!("../../../../examples/canon.musa"), "canon"),
        &Realization::seeded(42),
        &musa_compiler::ImportSources::default(),
    )
    .expect("elaborates");
    assert_eq!(musa_kernel::notes(&determinate).count(), 0);
}
