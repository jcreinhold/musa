//! The semantic pipeline's baseline (roadmap §17.7).
//!
//! The pipeline stages on eleven reference workloads: the small, large, and
//! shared columns, the migration-pressure scenarios, and prompt 127's four.
//!
//! | id | what | why it is the right thing to watch |
//! | --- | --- | --- |
//! | P0 | parse only | separates syntax cost from elaboration |
//! | P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
//! | P2 | elaboration only, parse excluded | the stage the kernel migration rewrote |
//! | P3 | the snapshot projection | the stage the migration created, most likely to regress |
//! | P4 | canonical form of the whole piece | what semantic identity pays on every edit |
//! | P5 | the semantic hash of the whole piece | what the session actually asks for |
//! | P6 | the tonal analysis of the whole piece | the one stage that grows with the chord vocabulary |
//! | P7 | every analysis kind at once | what a reader opening the analysis panel pays |
//! | K0 | a `.musa.kernel` document | the interchange alternative, which shares no stage with the surface one |
//! | E0–E4 | format, recover, edit head, edit tail, name under cursor | the interactive path |
//! | S0–S2 | identical calls, distinct arguments, hand-hoisted | the two sharing gaps |
//!
//! P2–P4 report allocation counts as well as time, because the migration's
//! risk is allocation and hashing rather than arithmetic: replacing one
//! `Vec<ScoreEvent>` per voice with one heterogeneous multiset per piece will
//! show up there first, if it shows up at all.
//!
//! Results are recorded in `docs/rules/kernel/09-performance.md` through the kernel
//! migration and in `docs/rules/language/06-performance.md` from prompt 93 on. Run
//! with:
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
const CORE_PRESSURE: &str = include_str!("../../../tests/fixtures/core-pressure.musa");
const TEMPLATE_PRESSURE: &str = include_str!("../../../tests/fixtures/template-pressure.musa");
const ANALYSIS_PRESSURE: &str = include_str!("../../../tests/fixtures/analysis-pressure.musa");
const KERNEL_PRESSURE: &str = include_str!("../../../tests/fixtures/kernel-pressure.musa");
const KERNEL_DOCUMENT: &str = include_str!("../../../tests/fixtures/kernel-pressure.musa.kernel");
const DECLARATION_LIBRARIES: [&str; 4] = [
    include_str!("../../../tests/fixtures/elaboration-libraries/library-0.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-1.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-2.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-3.musa"),
];

/// The reference workloads, named once.
///
/// `small` and `large` are `docs/rules/desktop/06-performance.md`'s two; `shared`
/// exists because neither of the other two contains a `repeat`
/// or a `use`, and a claim about sharing cannot be measured on
/// material that shares nothing. It denotes the same 1500 notes as `large`
/// minus the coda, written as four motifs repeated 100 times — so `shared`
/// against `large` is the same music at two levels of reuse, and the
/// difference between the columns is what sharing is worth.
/// Prompt 127 adds four: the total core, template expansion, the analysis
/// stages, and typed quotation, each written to isolate one of them.
const WORKLOADS: [&str; 11] = [
    "small",
    "large",
    "shared",
    "open-shape",
    "higher-order-shape",
    "declaration-heavy",
    "audio-bridge",
    "core-pressure",
    "template-pressure",
    "analysis-pressure",
    "kernel-pressure",
];

fn source(workload: &str) -> SourceDocument {
    match workload {
        "small" => SourceDocument::new(SMALL, "examples/glass-mountain.musa"),
        "shared" => SourceDocument::new(SHARED, "tests/fixtures/shared-score.musa"),
        "open-shape" => SourceDocument::new(OPEN_SHAPE, "tests/fixtures/open-shape.musa"),
        "higher-order-shape" => SourceDocument::new(HIGHER_ORDER_SHAPE, "tests/fixtures/higher-order-shape.musa"),
        "declaration-heavy" => SourceDocument::new(DECLARATION_HEAVY, "tests/fixtures/declaration-heavy.musa"),
        "audio-bridge" => SourceDocument::new(AUDIO_BRIDGE, "tests/fixtures/audio-bridge.musa"),
        "core-pressure" => SourceDocument::new(CORE_PRESSURE, "tests/fixtures/core-pressure.musa"),
        "template-pressure" => SourceDocument::new(TEMPLATE_PRESSURE, "tests/fixtures/template-pressure.musa"),
        "analysis-pressure" => SourceDocument::new(ANALYSIS_PRESSURE, "tests/fixtures/analysis-pressure.musa"),
        "kernel-pressure" => SourceDocument::new(KERNEL_PRESSURE, "tests/fixtures/kernel-pressure.musa"),
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
    if workload == "template-pressure" {
        options.imports.insert(
            "tests/fixtures/elaboration-libraries/library-0.musa".to_owned(),
            DECLARATION_LIBRARIES[0],
        );
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
/// snapshot adapter. This is the stage the kernel migration rewrote.
#[divan::bench(args = WORKLOADS)]
fn p2_elaborate(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let parsed = bench::parse(&source(workload));
    let options = options(workload);
    bencher.bench_local(|| bench::elaborate(divan::black_box(&parsed), &options));
}

/// P3 — the snapshot projection alone: kernel tracks back into score
/// events, with elaboration hoisted out of the measured region.
#[divan::bench(args = WORKLOADS)]
fn p3_project(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let tracks = bench::tracks(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&tracks).project());
}

/// P4 — canonical form of the whole piece: every voice overlaid into one
/// track and normalized. Semantic identity pays this per edit.
#[divan::bench(args = WORKLOADS)]
fn p4_canonical(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let tracks = bench::tracks(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&tracks).canonical());
}

/// P5 — the semantic hash of the whole piece: P4's canonical order plus the
/// digest of its bytes. The session asks for this on every recompile,
/// so the difference between this row and P4's is what identity costs.
#[divan::bench(args = WORKLOADS)]
fn p5_hash(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let tracks = bench::tracks(&bench::parse(&source(workload)), &options(workload));
    bencher.bench_local(|| divan::black_box(&tracks).hash());
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

/// P7 — every analysis kind on a whole piece, not just the tonal one.
///
/// P6 measures the stage that grows with the chord vocabulary; this measures
/// the stage a reader actually opens, which asks all six questions at once.
/// Counterpoint and voice leading walk pairs of voices, so their cost is
/// quadratic in the part count where P6's is not.
#[divan::bench(args = WORKLOADS)]
fn p7_analyze_all(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let compilation = compile(&source(workload), &options(workload));
    let Some(score) = compilation.snapshot() else {
        return;
    };
    bencher.bench_local(|| {
        let mut findings = 0_usize;
        for kind in AnalysisKind::ALL {
            if let Ok(report) = analyze(divan::black_box(score), &AnalysisRequest::new(kind)) {
                findings = findings.saturating_add(report.findings().len());
            }
        }
        findings
    });
}

/// K0 — a `.musa.kernel` document compiled as itself: the interchange
/// alternative, which shares no stage with the surface one but pays the same
/// budget when an editor opens one.
#[divan::bench]
fn k0_kernel_document(bencher: divan::Bencher<'_, '_>) {
    let document = SourceDocument::new(KERNEL_DOCUMENT, "tests/fixtures/kernel-pressure.musa.kernel");
    let options = CompileOptions::default();
    bencher.bench_local(|| compile(divan::black_box(&document), &options));
}

/// The interactive stages, measured on the large workload because that is the
/// case `docs/rules/desktop/06-performance.md`'s B1 and B2 are stated on.
///
/// E1 and E2 measure a source the composer is in the middle of writing: a
/// piece with an unclosed brace and a bar that does not add up is not an
/// error case in an editor, it is every other keystroke.
mod editing {
    use musa_compiler::{CompileOptions, SourceDocument, compile, format_document};

    use super::{LARGE, options, source};

    /// The large source with one note changed `bytes` in from the given end.
    /// The edit is a note name, so the document stays valid and the compile
    /// being measured is a successful one.
    fn edited(from_start: bool) -> SourceDocument {
        let target = if from_start { "c4/4" } else { "g4/4" };
        let text = if from_start {
            LARGE.replacen(target, "d4/4", 1)
        } else {
            let at = LARGE.rfind(target).unwrap_or(0);
            let mut text = LARGE.to_owned();
            text.replace_range(at..at.saturating_add(target.len()), "d4/4");
            text
        };
        SourceDocument::new(text, "tests/fixtures/large-score.musa")
    }

    /// E0 — laying the whole document out again, which is what the format
    /// command and the editor's format-on-save both pay.
    #[divan::bench]
    fn e0_format(bencher: divan::Bencher<'_, '_>) {
        bencher.bench_local(|| format_document(divan::black_box(LARGE), musa_language::BarSpacing::Compact));
    }

    /// E1 — a document with a brace missing and a bar that does not add up:
    /// parser recovery through elaboration, ending in diagnostics rather than
    /// a score.
    #[divan::bench]
    fn e1_recover(bencher: divan::Bencher<'_, '_>) {
        let text = LARGE.replacen("c4/4", "c4/", 1).replacen("voice", "voice {", 1);
        let document = SourceDocument::new(text, "tests/fixtures/large-score.musa");
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }

    /// E2 — an edit in the first bar of the piece.
    #[divan::bench]
    fn e2_edit_head(bencher: divan::Bencher<'_, '_>) {
        let document = edited(true);
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }

    /// E3 — an edit in the last bar of the piece. Measured beside E2 because
    /// the compiler is not incremental: if these two ever differ, something
    /// has started depending on where an edit landed.
    #[divan::bench]
    fn e3_edit_tail(bencher: divan::Bencher<'_, '_>) {
        let document = edited(false);
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }

    /// E4 — the editor's point query: which name is under the cursor, and
    /// what does the piece say about it. The compilation is hoisted out, so
    /// this is the lookup alone rather than the compile behind it.
    #[divan::bench]
    fn e4_name_under_cursor(bencher: divan::Bencher<'_, '_>) {
        let compilation = compile(&source("large"), &options("large"));
        let at = u32::try_from(LARGE.len() / 2).unwrap_or(0);
        bencher.bench_local(|| {
            let found = compilation
                .references()
                .iter()
                .find(|reference| reference.uses.iter().any(|span| span.start <= at && at < span.end));
            found.map(|reference| compilation.items().iter().any(|item| item.name == reference.name))
        });
    }
}

/// The two sharing gaps, priced (prompt 127).
///
/// `S0` scales identical calls at distinct sites, which is the call-site gap;
/// `S1` and `S2` are the same music written two ways, which is the
/// full-laziness gap. All three report allocations, and the workload's own
/// duplication is printed by `main` so a timing can be read against the size
/// of the term behind it.
mod sharing {
    use musa_compiler::bench::{Sharing, sharing_source};
    use musa_compiler::{CompileOptions, SourceDocument, compile};

    /// Sixteen notes is a phrase; the counts are call sites.
    const BODY: usize = 16;
    const CALLS: [usize; 4] = [8, 32, 128, 512];

    fn document(shape: Sharing, calls: usize) -> SourceDocument {
        SourceDocument::new(sharing_source(shape, calls, BODY), "benches/sharing.musa")
    }

    #[divan::bench(args = CALLS)]
    fn s0_identical_calls(bencher: divan::Bencher<'_, '_>, calls: usize) {
        let document = document(Sharing::Identical, calls);
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }

    #[divan::bench(args = CALLS)]
    fn s1_distinct_arguments(bencher: divan::Bencher<'_, '_>, calls: usize) {
        let document = document(Sharing::Distinct, calls);
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }

    #[divan::bench(args = CALLS)]
    fn s2_hand_hoisted(bencher: divan::Bencher<'_, '_>, calls: usize) {
        let document = document(Sharing::Hoisted, calls);
        let options = CompileOptions::default();
        bencher.bench_local(|| compile(divan::black_box(&document), &options));
    }
}

/// The private evaluator's cost curve. Source construction stays outside the
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

/// How many bodies a piece's term binds, and how big the printed term is.
///
/// Duplication is what the sharing benchmarks are about, and it is visible in
/// the term rather than in a timing: two identical `let shared` bindings are
/// one body elaborated twice. Counted from the printed term because that is
/// the compiler's own account of what it built, and it needs no new API.
///
/// `None` where the interchange helper has no term to print. That helper
/// resolves no imports by construction, so a workload that imports the
/// standard library has no printed term here — which is a missing
/// measurement, not a measurement of zero, and the caller says so.
fn duplication(source: &SourceDocument) -> Option<(usize, usize)> {
    musa_compiler::kernel_text(
        source,
        &musa_compiler::Realization::default(),
        &musa_compiler::ImportSources::default(),
    )
    .map(|text| (text.matches("let shared").count(), text.len()))
}

fn main() {
    // Report what is being measured, so a table row cannot be read without
    // knowing the size of the workload behind it.
    for workload in WORKLOADS {
        let tracks = bench::tracks(&bench::parse(&source(workload)), &options(workload));
        let term = duplication(&source(workload)).map_or_else(
            || "term unavailable (the interchange helper resolves no imports)".to_owned(),
            |(bindings, bytes)| format!("{bindings} shared bindings, {bytes}-byte term"),
        );
        println!("workload {workload}: {} occurrences, {term}", tracks.occurrences());
    }
    for shape in [
        bench::Sharing::Identical,
        bench::Sharing::Distinct,
        bench::Sharing::Hoisted,
    ] {
        for calls in [8_usize, 32, 128, 512] {
            let document = SourceDocument::new(bench::sharing_source(shape, calls, 16), "benches/sharing.musa");
            let (bindings, bytes) = duplication(&document).expect("a generated sharing piece imports nothing");
            println!("sharing {shape:?} calls={calls}: {bindings} shared bindings, {bytes}-byte term");
        }
    }
    divan::main();
}
