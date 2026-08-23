//! The semantic pipeline's baseline (roadmap §17.7).
//!
//! The pipeline stages on eleven reference workloads: the small, large, and
//! shared columns, the migration-pressure scenarios, and prompt 127's four.
//!
//! | id | what | why it is the right thing to watch |
//! | --- | --- | --- |
//! | P0 | parse only | separates syntax cost from elaboration |
//! | P1 | `compile` end to end | B1's server-side share: a keystroke costs a compile |
//! | P2 | elaboration only, parse excluded | the stage the event track migration rewrote |
//! | P3 | the snapshot projection | the stage the migration created, most likely to regress |
//! | P4 | canonical form of the whole piece | what semantic identity pays on every edit |
//! | P5 | the semantic hash of the whole piece | what the session actually asks for |
//! | P6 | the tonal analysis of the whole piece | the one stage that grows with the chord vocabulary |
//! | P7 | every analysis kind at once | what a reader opening the analysis panel pays |
//! | K0 | a `.musa.events` document | the interchange alternative, which shares no stage with the surface one |
//! | E0–E4 | format, recover, edit head, edit tail, name under cursor | the interactive path |
//! | S0–S2 | identical calls, distinct arguments, hand-hoisted | the two sharing gaps |
//!
//! P2–P4 report allocation counts as well as time, because the migration's
//! risk is allocation and hashing rather than arithmetic: replacing one
//! `Vec<ScoreEvent>` per voice with one heterogeneous multiset per piece will
//! show up there first, if it shows up at all.
//!
//! **A workload the compiler refuses is still timed, and the preamble says so.**
//! Four of the eleven currently are — `large`, `core-pressure`,
//! `template-pressure`, and `analysis-pressure` — so their P0–P7 rows price a
//! refusal rather than a compilation, the way `finite_core_rejection` is kept
//! apart from `finite_core_fold` for the same reason. Dropping them from
//! `WORKLOADS` would lose the coverage silently; printing the compiler's own
//! error beside the row does not. The preamble line is the one that says which
//! rows those are, and it is not optional reading.
//!
//! Results are recorded in `docs/rules/events/09-pipeline-baseline.md` through the event track
//! migration and in `docs/rules/language/06-elaboration-baseline.md` from prompt 93 on. Run
//! with:
//!
//! ```sh
//! cargo bench -p musa-compiler
//! ```

use musa_compiler::{CompileOptions, SourceDocument, bench, compile};

use musa_score::{AnalysisKind, AnalysisRequest, analyze};

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
const EVENTS_PRESSURE: &str = include_str!("../../../tests/fixtures/events-pressure.musa");
const EVENTS_DOCUMENT: &str = include_str!("../../../tests/fixtures/events-pressure.musa.events");
const DECLARATION_LIBRARIES: [&str; 4] = [
    include_str!("../../../tests/fixtures/elaboration-libraries/library-0.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-1.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-2.musa"),
    include_str!("../../../tests/fixtures/elaboration-libraries/library-3.musa"),
];

/// The reference workloads, named once.
///
/// `small` and `large` are `docs/rules/desktop/06-frame-budgets.md`'s two; `shared`
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
    "events-pressure",
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
        "events-pressure" => SourceDocument::new(EVENTS_PRESSURE, "tests/fixtures/events-pressure.musa"),
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

/// P2 — everything after parsing: elaboration through the event track and the
/// snapshot adapter. This is the stage the event track migration rewrote.
#[divan::bench(args = WORKLOADS)]
fn p2_elaborate(bencher: divan::Bencher<'_, '_>, workload: &str) {
    let parsed = bench::parse(&source(workload));
    let options = options(workload);
    bencher.bench_local(|| bench::elaborate(divan::black_box(&parsed), &options));
}

/// P3 — the snapshot projection alone: events tracks back into score
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

/// K0 — a `.musa.events` document compiled as itself: the interchange
/// alternative, which shares no stage with the surface one but pays the same
/// budget when an editor opens one.
#[divan::bench]
fn k0_events_document(bencher: divan::Bencher<'_, '_>) {
    let document = SourceDocument::new(EVENTS_DOCUMENT, "tests/fixtures/events-pressure.musa.events");
    let options = CompileOptions::default();
    bencher.bench_local(|| compile(divan::black_box(&document), &options));
}

/// The interactive stages, measured on the large workload because that is the
/// case `docs/rules/desktop/06-frame-budgets.md`'s B1 and B2 are stated on.
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
        bencher.bench_local(|| format_document(divan::black_box(LARGE), musa_syntax::BarSpacing::Compact));
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
/// full-laziness gap. All three report allocations, and `main` prints each
/// workload's term size so a timing can be read against the size of the term
/// behind it.
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

/// How big the piece's printed term is.
///
/// Term size is what a timing has to be read against: the same music written
/// twice as large is not the same measurement, and the byte count is the
/// compiler's own account of what it built, needing no new API.
///
/// **The `let shared` count that stood beside it is gone, and it was not
/// dropped for tidiness.** Prompt 142 put the surface on a core program, and
/// the events text became a *projection* of the evaluated result rather than
/// the shape elaboration was carried in — `piece_term` prints one literal per
/// voice and binds nothing. `sharing_laws.rs` retired the same count for the
/// same reason at the time; this benchmark did not, so every row it printed
/// read `0 shared bindings`, which is not a measurement of no duplication but
/// the absence of a measurement wearing its clothes.
///
/// Read under the workload's *own* import sources rather than an empty set.
/// The helper used to pass `ImportSources::default()`, so `declaration-heavy`
/// reported no term for a reason that was the benchmark's own doing; the
/// options the timed benchmarks compile under are the ones this reads under
/// too, and the two now agree about what the workload is.
///
/// `Err` where there is no term to print, carrying the compiler's own words
/// for why. A refused workload has no term, and so does one whose imports
/// were still not supplied; both are missing measurements rather than
/// measurements of zero, and printing one cause for the other would record a
/// reading nobody took.
fn term_bytes(source: &SourceDocument, options: &CompileOptions) -> Result<usize, String> {
    musa_compiler::events_text(source, &musa_score::Realization::default(), &options.imports)
        .map(|text| text.len())
        .ok_or_else(|| refusal(source, options))
}

/// Why a workload has no printed term, in the compiler's own words.
///
/// The first error it reported, and how many followed it. A benchmark cannot
/// diagnose the compiler and should not try: it reports what the compiler
/// said and leaves the judgement to whoever reads the row.
fn refusal(source: &SourceDocument, options: &CompileOptions) -> String {
    let compilation = compile(source, options);
    let mut errors = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| diagnostic.message.as_str());
    let Some(first) = errors.next() else {
        return "the piece compiles and still prints no term".to_owned();
    };
    match errors.count() {
        0 => first.to_owned(),
        rest => format!("{first} (and {rest} more)"),
    }
}

/// One workload's term as a preamble cell: its size, or why it has none.
fn term_cell(source: &SourceDocument, options: &CompileOptions) -> String {
    match term_bytes(source, options) {
        Ok(bytes) => format!("{bytes}-byte term"),
        Err(reason) => format!("term unavailable ({reason})"),
    }
}

fn main() {
    // Report what is being measured, so a table row cannot be read without
    // knowing the size of the workload behind it.
    for workload in WORKLOADS {
        let tracks = bench::tracks(&bench::parse(&source(workload)), &options(workload));
        let term = term_cell(&source(workload), &options(workload));
        println!("workload {workload}: {} occurrences, {term}", tracks.occurrences());
    }
    for shape in [
        bench::Sharing::Identical,
        bench::Sharing::Distinct,
        bench::Sharing::Hoisted,
    ] {
        for calls in [8_usize, 32, 128, 512] {
            let document = SourceDocument::new(bench::sharing_source(shape, calls, 16), "benches/sharing.musa");
            let term = term_cell(&document, &CompileOptions::default());
            println!("sharing {shape:?} calls={calls}: {term}");
        }
    }
    divan::main();
}
