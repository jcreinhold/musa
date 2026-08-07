//! The semantic pipeline's baseline (docs/prompts/38; roadmap §17.7).
//!
//! Four measurements on the two reference workloads of
//! `docs/interface/06-performance.md`:
//!
//! | id | what | why it is the right thing to watch |
//! | --- | --- | --- |
//! | P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
//! | P2 | elaboration only, parse excluded | what prompts 39–41 change |
//! | P3 | the snapshot projection | the stage prompt 39 creates, most likely to regress |
//! | P4 | canonical form of the whole piece | what prompt 43 pays on every edit |
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

// Benchmarks index a two-element workload table and unwrap statically valid
// fixtures: a failure is a bug in the benchmark, and panicking is correct.
#![allow(clippy::expect_used)]

use musa_compiler::{CompileOptions, SourceDocument, bench, compile};

/// Divan's allocation profiler; the counts are the point of choosing it.
#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const SMALL: &str = include_str!("../../../examples/glass-mountain.musa");
const LARGE: &str = include_str!("../../../tests/fixtures/large-score.musa");

/// The two reference workloads, named once (`docs/interface/06-performance.md`).
const WORKLOADS: [&str; 2] = ["small", "large"];

fn source(workload: &str) -> SourceDocument {
    match workload {
        "small" => SourceDocument::new(SMALL, "examples/glass-mountain.musa"),
        _ => SourceDocument::new(LARGE, "tests/fixtures/large-score.musa"),
    }
}

/// P1 — `compile` end to end, parse included. The number a keystroke pays.
#[divan::bench(args = WORKLOADS)]
fn p1_compile(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let source = source(workload);
    bencher.bench_local(|| compile(divan::black_box(&source), &CompileOptions::default()));
}

/// P2 — everything after parsing: elaboration through the kernel and the
/// snapshot adapter. This is the stage prompts 39–41 rewrite.
#[divan::bench(args = WORKLOADS)]
fn p2_elaborate(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let parsed = bench::parse(&source(workload));
    bencher.bench_local(|| bench::elaborate(divan::black_box(&parsed), &CompileOptions::default()));
}

/// P3 — the snapshot projection alone: kernel timelines back into score
/// events, with elaboration hoisted out of the measured region.
#[divan::bench(args = WORKLOADS)]
fn p3_project(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let timelines = bench::timelines(&bench::parse(&source(workload)), &CompileOptions::default());
    bencher.bench_local(|| divan::black_box(&timelines).project());
}

/// P4 — canonical form of the whole piece: every voice overlaid into one
/// timeline and normalized. Prompt 43's semantic identity pays this per edit.
#[divan::bench(args = WORKLOADS)]
fn p4_canonical(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let timelines = bench::timelines(&bench::parse(&source(workload)), &CompileOptions::default());
    bencher.bench_local(|| divan::black_box(&timelines).canonical());
}

fn main() {
    // Report what is being measured, so a table row cannot be read without
    // knowing the size of the workload behind it.
    for workload in WORKLOADS {
        let timelines = bench::timelines(&bench::parse(&source(workload)), &CompileOptions::default());
        println!("workload {workload}: {} occurrences", timelines.occurrences());
    }
    divan::main();
}
