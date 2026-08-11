//! The semantic pipeline's baseline (docs/prompts/38; roadmap §17.7).
//!
//! Seven measurements on seven reference workloads: the historical small,
//! large, and shared columns plus prompt 93's migration-pressure scenarios.
//!
//! | id | what | why it is the right thing to watch |
//! | --- | --- | --- |
//! | P0 | parse only | separates syntax cost from elaboration |
//! | P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
//! | P2 | elaboration only, parse excluded | what prompts 39–41 change |
//! | P3 | the snapshot projection | the stage prompt 39 creates, most likely to regress |
//! | P4 | canonical form of the whole piece | what prompt 43 pays on every edit |
//! | P5 | the semantic hash of the whole piece | what prompt 43 actually asks for |
//! | P6 | the tonal analysis of the whole piece | what prompt 118 adds, and the one stage that grows with the chord vocabulary |
//!
//! P2–P4 report allocation counts as well as time, because the migration's
//! risk is allocation and hashing rather than arithmetic: replacing one
//! `Vec<ScoreEvent>` per voice with one heterogeneous multiset per piece will
//! show up there first, if it shows up at all.
//!
//! Results are recorded in `docs/kernel/09-performance.md`. Run with:
//!
//! ```sh
//! cargo bench -p musa-compiler
//! ```

// Benchmarks index a fixed workload table and unwrap statically valid
// fixtures: a failure is a bug in the benchmark, and panicking is correct.
#![allow(clippy::expect_used)]

use musa_compiler::{AnalysisKind, AnalysisRequest, CompileOptions, SourceDocument, analyze, bench, compile};

/// Divan's allocation profiler; the counts are the point of choosing it.
#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SMALL: &str = include_str!("../../../examples/glass-mountain.musa");
const LARGE: &str = include_str!("../../../tests/fixtures/large-score.musa");
const SHARED: &str = include_str!("../../../tests/fixtures/shared-score.musa");
const OPEN_SHAPE: &str = include_str!("../../../tests/fixtures/open-shape.musa");
const HIGHER_ORDER_SHAPE: &str = include_str!("../../../tests/fixtures/higher-order-shape.musa");
const DECLARATION_HEAVY: &str = include_str!("../../../tests/fixtures/declaration-heavy.musa");
const AUDIO_BRIDGE: &str = include_str!("../../../tests/fixtures/audio-bridge.musa");
const DECLARATION_LIBRARIES: [&str; 4] = [
    include_str!("../../../tests/fixtures/elaboration-libraries/library-0.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-1.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-2.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-3.musa"),
];

/// The reference workloads, named once.
///
/// `small` and `large` are `docs/interface/06-performance.md`'s two; `shared`
/// is prompt 49's, added because neither of the other two contains a `repeat`
/// or a `use` and a prompt whose claim is sharing cannot be measured on
/// material that shares nothing. It denotes the same 1500 notes as `large`
/// minus the coda, written as four motifs repeated 100 times — so `shared`
/// against `large` is the same music at two levels of reuse, and the
/// difference between the columns is what sharing is worth.
const WORKLOADS: [&str; 7] = [
    "small",
    "large",
    "shared",
    "open-shape",
    "higher-order-shape",
    "declaration-heavy",
    "audio-bridge",
];

fn source(workload: &str) -> SourceDocument {
    match workload {
        "small" => SourceDocument::new(SMALL, "examples/glass-mountain.musa"),
        "shared" => SourceDocument::new(SHARED, "tests/fixtures/shared-score.musa"),
        "open-shape" => SourceDocument::new(OPEN_SHAPE, "tests/fixtures/open-shape.musa"),
        "higher-order-shape" => SourceDocument::new(HIGHER_ORDER_SHAPE, "tests/fixtures/higher-order-shape.musa"),
        "declaration-heavy" => SourceDocument::new(DECLARATION_HEAVY, "tests/fixtures/declaration-heavy.musa"),
        "audio-bridge" => SourceDocument::new(AUDIO_BRIDGE, "tests/fixtures/audio-bridge.musa"),
        _ => SourceDocument::new(LARGE, "tests/fixtures/large-score.musa"),
    }
}

fn options(workload: &str) -> CompileOptions {
    let mut options = CompileOptions::default();
    if workload == "declaration-heavy" {
        for (index, source) in DECLARATION_LIBRARIES.into_iter().enumerate() {
            options.imports.insert(
                format!("tests/fixtures/elaboration-libraries/library-{index}.musa"),
                source,
            );
        }
    }
    options
}

/// P0 — parsing only, with source construction outside the timed region.
#[divan::bench(args = WORKLOADS)]
fn p0_parse(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let source = source(workload);
    bencher.bench_local(|| bench::parse(divan::black_box(&source)));
}

/// P1 — `compile` end to end, parse included. The number a keystroke pays.
#[divan::bench(args = WORKLOADS)]
fn p1_compile(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let source = source(workload);
    let options = options(workload);
    bencher.bench_local(|| compile(divan::black_box(&source), &options));
}

/// P2 — everything after parsing: elaboration through the kernel and the
/// snapshot adapter. This is the stage prompts 39–41 rewrite.
#[divan::bench(args = WORKLOADS)]
fn p2_elaborate(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let parsed = bench::parse(&source(workload));
    let options = options(workload);
    bencher.bench_local(|| bench::elaborate(divan::black_box(&parsed), &options));
}

/// P3 — the snapshot projection alone: kernel timelines back into score
/// events, with elaboration hoisted out of the measured region.
#[divan::bench(args = WORKLOADS)]
fn p3_project(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let timelines = bench::timelines(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&timelines).project());
}

/// P4 — canonical form of the whole piece: every voice overlaid into one
/// timeline and normalized. Prompt 43's semantic identity pays this per edit.
#[divan::bench(args = WORKLOADS)]
fn p4_canonical(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let timelines = bench::timelines(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&timelines).canonical());
}

/// P5 — the semantic hash of the whole piece: P4's canonical order plus the
/// digest of its bytes. Prompt 43's session asks for this on every recompile,
/// so the difference between this row and P4's is what identity costs.
#[divan::bench(args = WORKLOADS)]
fn p5_hash(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let timelines = bench::timelines(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&timelines).hash());
}

/// P6 — the tonal reading of a whole piece: segmentation, chord fitting,
/// key regions, numerals, and the boundary readings, with compilation hoisted
/// out of the measured region.
///
/// Watched separately from P0–P5 because it is the one stage whose cost is
/// quadratic in nothing obvious: every slice is fitted against every chord in
/// the vocabulary rooted on every sounding class, and then against every
/// degree of every surviving key. A vocabulary or key-set that grows shows up
/// here and nowhere else.
#[divan::bench(args = WORKLOADS)]
fn p6_analyze(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let compilation = compile(&source(workload), &options(workload));
    let Some(score) = compilation.snapshot() else {
        return;
    };
    let request = AnalysisRequest::new(AnalysisKind::Tonal);
    bencher.bench_local(|| analyze(divan::black_box(score), &request));
}

/// The prompt-96 evaluator curve. Source construction stays outside the
/// timed closure; the count is the exact number of structural fold steps.
#[divan::bench(args = [0u64, 1_000, 10_000, 50_000])]
fn finite_core_fold(bencher: divan::Bencher<'_, '_>, count: u64) {
    let source = SourceDocument::new(
        format!(
            "piece \"Finite core bench\" {{ \
             fn keep(index: Nat, accumulator: Nat) -> Nat {{ accumulator }} \
             let value: Nat = nat_fold(0, keep, {count}); \
             score {{ part p {{ voice v {{ c4/1 }} }} }} \
             }}"
        ),
        "benches/finite-core.musa",
    );
    let options = CompileOptions::default();
    bencher.bench_local(|| compile(divan::black_box(&source), &options));
}

/// A deterministic resource rejection is measured separately from accepted
/// work, so it can never make the successful compilation curve look cheaper.
#[divan::bench]
fn finite_core_rejection(bencher: divan::Bencher<'_, '_>) {
    let source = SourceDocument::new(
        "piece \"Finite core rejection\" { \
         fn keep(index: Nat, accumulator: Nat) -> Nat { accumulator } \
         let value: Nat = nat_fold(0, keep, 200000); \
         score { part p { voice v { c4/1 } } } \
         }",
        "benches/finite-core-rejection.musa",
    );
    let options = CompileOptions::default();
    bencher.bench_local(|| compile(divan::black_box(&source), &options));
}

fn main() {
    // Report what is being measured, so a table row cannot be read without
    // knowing the size of the workload behind it.
    for workload in WORKLOADS {
        let timelines = bench::timelines(&bench::parse(&source(workload)), &options(workload));
        println!("workload {workload}: {} occurrences", timelines.occurrences());
    }
    divan::main();
}
