//! The two laws that make a kernel file a Musa document (prompt 120).
//!
//! `docs/rules/language/01-surface.md` claims the kernel is a *sublanguage*, not a
//! sidecar format, and `docs/rules/language/01-surface.md` §7 cashes that claim as
//! a second top-level alternative of the one language. A claim of inclusion
//! is only worth what its laws are:
//!
//! ```text
//! syntactic:  every file of the kernel corpus is a Musa document, byte for
//!             byte, and the unified entry point accepts it
//! semantic:   the unified route's meaning is the kernel's own meaning —
//!             same term, same denotation, same semantic hash
//! ```
//!
//! The second law is the one with teeth. A reader that accepted kernel files
//! and quietly re-elaborated them through the surface pipeline would satisfy
//! the first law and violate the reason for having it: the interchange format
//! exists so that a term travels without being reinterpreted.
//!
//! The corpus is `examples/kernel/`, held current by
//! `kernel_interop::the_kernel_corpus_is_up_to_date` — these laws read the
//! same files, so a corpus that drifts fails there first and here second.

// A failure of these `expect`s is a bug in the fixture corpus, not in a
// caller's input.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
// The failure messages name which fixture broke, which a bare `expect` cannot.
#![allow(clippy::panic)]

use musa_compiler::{
    CompileOptions, DocumentKind, Realization, SourceDocument, check_kernel_text, compile, format_document,
    kernel_text, kernel_text_meaning,
};

/// The realization `examples/kernel/` was printed under, the same seed
/// `kernel_interop` pins.
const FIXTURE_SEED: u64 = 42;

/// Every committed kernel document, as (file name, text).
fn corpus() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kernel");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
        .expect("read examples/kernel")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.to_string_lossy().ends_with(".musa.kernel"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "the kernel corpus is empty");
    files
        .into_iter()
        .map(|path| {
            let name = path.file_name().expect("file name").to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).expect("fixture text"))
        })
        .collect()
}

/// Compile through the one entry point every caller uses.
fn compiled(name: &str, text: &str) -> musa_compiler::Compilation {
    compile(&SourceDocument::new(text, name), &CompileOptions::default())
}

/// The marker is the kernel's own version string.
///
/// Two crates spell this: `musa-kernel` writes `% musa-kernel-2` as the first
/// line of every document it prints, and `musa-language` decides the top-level
/// alternative by matching that line. They cannot depend on each other —
/// `musa-language` is below the kernel and stays there — so the law lives in
/// the one crate that sees both. Bumping the format version without teaching
/// the reader would otherwise make every new document silently surface Musa,
/// which fails as a wall of parse errors rather than as one honest refusal.
#[test]
fn kernel_document_marker_matches_the_kernel() {
    assert_eq!(
        musa_language::KERNEL_MARKER,
        format!("% {}", musa_kernel::FORMAT_VERSION),
        "the alternative's marker and the format's version have drifted",
    );
}

/// Law 1, syntactic: the corpus is Musa, byte for byte and unmodified.
#[test]
fn every_kernel_file_is_a_musa_document() {
    for (name, text) in corpus() {
        assert_eq!(
            musa_language::alternative(&text),
            musa_language::DocumentAlternative::Kernel,
            "{name} is not read as the kernel alternative",
        );
        let compilation = compiled(&name, &text);
        assert_eq!(
            compilation.kind(),
            DocumentKind::Kernel,
            "{name} compiled as another kind"
        );
        assert!(
            !compilation.has_errors(),
            "{name}: {:?}",
            compilation
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
        );
        assert!(compilation.snapshot().is_some(), "{name} produced no score");
    }
}

/// Law 2, semantic: the unified route means what the kernel means.
///
/// [`kernel_text_meaning`] is the kernel's own typed route —
/// `musa_kernel::parse::<ScoreFact>`, evaluated, hashed, and rendered. The
/// unified route reaches the same through `compile`. The hashes agreeing is
/// the statement that reading a kernel file as a Musa document reinterprets
/// nothing; the *denotation* agreeing — every occurrence, in order, with its
/// payload and its exact bounds — is the statement that the hash is agreeing
/// about something. A hash law alone would pass on two empty timelines.
#[test]
fn the_unified_route_preserves_the_kernels_meaning() {
    for (name, text) in corpus() {
        let (denotation, hash) = kernel_text_meaning(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        let check = check_kernel_text(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        let compilation = compiled(&name, &text);
        assert_eq!(compilation.identity(), hash, "{name} means something else as Musa");
        let title = compilation.snapshot().expect("a score").title().to_owned();
        assert_eq!(title, check.name, "{name} is titled after another document");
        assert!(
            denotation.matches("occurrence").count() == check.occurrences,
            "{name}: the denotation and the check disagree on how much music there is",
        );
    }
}

/// The two surfaces of one language meet: printing a piece as kernel text and
/// reading it back through the unified entry point gives the same meaning.
///
/// This is the inclusion the corpus laws above assume — without it, the two
/// alternatives would agree only on files the compiler itself wrote in the
/// same run.
#[test]
fn a_piece_and_its_kernel_printing_have_one_meaning() {
    let sources: &[(&str, &str)] = &[
        ("twinkle", include_str!("../../../../examples/twinkle.musa")),
        ("counterpoint", include_str!("../../../../examples/counterpoint.musa")),
        ("modulation", include_str!("../../../../examples/modulation.musa")),
        (
            "tuplet-fixture",
            include_str!("../../../../examples/tuplet-fixture.musa"),
        ),
    ];
    for (name, source) in sources {
        let document = SourceDocument::new(*source, *name);
        let surface = compile(&document, &CompileOptions::default());
        let printed = kernel_text(&document, &Realization::seeded(FIXTURE_SEED), &musa_compiler::ImportSources::default()).unwrap_or_else(|| panic!("{name}"));
        let kernel = compiled(&format!("{name}.musa.kernel"), &printed);
        assert!(!kernel.has_errors(), "{name}'s own kernel printing did not compile");
        assert_eq!(
            kernel.identity(),
            surface.identity(),
            "{name} means one thing as a piece and another as a term",
        );
    }
}

/// A whole-score document arrives as a whole score: parts, voices, notes,
/// and the contexts they are read under.
///
/// The subset claim would be empty if a kernel document compiled to a
/// technically-valid but musically bare snapshot. `annotated.musa.kernel`
/// carries context facts — a tempo, a meter, a key — alongside its notes, and
/// every one of them has to survive the projection, because a backend asked
/// to engrave this document is given nothing else to read.
#[test]
fn a_whole_score_document_projects_a_whole_score() {
    let text = include_str!("../../../../examples/kernel/annotated.musa.kernel");
    let compilation = compiled("annotated.musa.kernel", text);
    let score = compilation.snapshot().expect("a score");
    assert!(!score.parts().is_empty(), "no parts");
    let (part_id, part) = score.parts().iter().next().expect("a part");
    let notes: usize = part.voices().map(|(_, voice)| voice.events().len()).sum();
    assert!(notes > 0, "a part with no music in it");
    let start = musa_compiler::MusicalTime::default();
    let scope = musa_compiler::Scope::Part { part: part_id.0 };
    assert!(score.tempo_at(scope, start).is_some(), "the tempo fact was lost");
    assert!(score.key_at(scope, start).is_some(), "the key fact was lost");
    assert!(score.meter_at(scope, start).is_measured(), "the meter fact was lost");
}

/// A kernel document formats to itself: the corpus is canonical, and the
/// formatter of the kernel alternative is the kernel's own printer.
///
/// Idempotence is checked on the second pass rather than assumed, because a
/// formatter that is not the identity on its own output is a formatter that
/// fights the editor's save hook.
#[test]
fn formatting_a_kernel_document_is_the_kernels_printing() {
    for (name, text) in corpus() {
        let once = format_document(&text, musa_language::BarSpacing::default())
            .unwrap_or_else(|| panic!("{name} would not format"));
        assert_eq!(once, text, "{name} is not canonical");
        let twice = format_document(&once, musa_language::BarSpacing::default()).expect("format again");
        assert_eq!(twice, once, "formatting {name} is not idempotent");
    }
}

/// A comment in a kernel document survives the formatter.
///
/// The interchange format keeps `%` notes for provenance — which piece, which
/// realization — and a formatter that dropped them would quietly erase the
/// only record of where a term came from.
#[test]
fn formatting_keeps_a_kernel_documents_notes() {
    let text = "% musa-kernel-2\n\
                % generated from examples/twinkle.musa by musa-compiler\n\
                kernel \"twinkle\" {\n  \
                composition main : EventTrack[WrittenTime, ScoreFact] =\n    \
                track 1/4 {\n      \
                occurrence \"voice 0 0 note c4 1/4 [0:0 #4]\" from 0 to 1/4;\n    \
                };\n\
                }\n";
    let formatted = format_document(text, musa_language::BarSpacing::default()).expect("format");
    assert!(
        formatted.contains("% generated from examples/twinkle.musa by musa-compiler"),
        "the note did not survive: {formatted}",
    );
}

/// A payload the compiler has no meaning for is refused by name, not guessed
/// at.
///
/// The kernel is parametric in its payload and this compiler knows exactly
/// one, `ScoreFact`. Reading someone else's payload as a score fact would be
/// the worst kind of success: a plausible score built out of text that meant
/// something else entirely.
#[test]
fn a_payload_the_compiler_does_not_know_is_refused() {
    let text = "% musa-kernel-2\n\
                kernel \"other\" {\n  \
                composition main : EventTrack[WrittenTime, Waveform] =\n    \
                track 1 {\n      \
                occurrence \"sine 440\" from 0 to 1;\n    \
                };\n\
                }\n";
    let compilation = compiled("other.musa.kernel", text);
    assert!(compilation.snapshot().is_none(), "an unknown payload produced a score");
    let codes: Vec<_> = compilation.diagnostics().iter().map(|d| d.code).collect();
    assert!(
        codes.contains(&musa_compiler::Code::UnsupportedPayload),
        "expected an unsupported-payload diagnostic, found {codes:?}",
    );
    let diagnostic = compilation
        .diagnostics()
        .iter()
        .find(|d| d.code == musa_compiler::Code::UnsupportedPayload)
        .expect("the diagnostic");
    assert!(
        diagnostic.message.contains("Waveform"),
        "the refusal does not name the payload: {}",
        diagnostic.message,
    );
    assert!(
        diagnostic.primary_span().is_some(),
        "the refusal points nowhere in the document",
    );
}

/// A document of a version this compiler does not speak is refused, and the
/// refusal lands in the document rather than as a panic or an empty score.
///
/// Both directions are unknown versions, and the retired one is the
/// interesting half: `musa-kernel-1` is refused rather than migrated
/// (`docs/plan/clean-break-ledger.md` §3), so a file written before the
/// coordinate existed is a file this build declines to guess about.
#[test]
fn an_unknown_format_version_is_refused() {
    for version in ["% musa-kernel-1", "% musa-kernel-3"] {
        let text = format!("{version}\nkernel \"later\" {{\n}}\n");
        let compilation = compiled("later.musa.kernel", &text);
        assert!(compilation.snapshot().is_none(), "{version} produced a score");
        assert!(compilation.has_errors(), "{version} compiled clean");
        // The marker is not this crate's, so the file is surface Musa — and
        // surface Musa is where the parse errors come from. Either way it is
        // refused; what must not happen is silent acceptance.
        assert_eq!(
            musa_language::alternative(&text),
            musa_language::DocumentAlternative::Surface,
            "{version} claimed to be this kernel",
        );
    }
}

/// A kernel document that is malformed *as a kernel document* is refused as
/// one, with the kernel's own complaint, not as a page of surface syntax
/// errors.
#[test]
fn a_malformed_kernel_document_is_refused_as_a_kernel_document() {
    let text = "% musa-kernel-2\nkernel \"broken\" {\n  composition main : EventTrack[WrittenTime, ScoreFact] =\n";
    let compilation = compiled("broken.musa.kernel", text);
    assert_eq!(compilation.kind(), DocumentKind::Kernel, "refused as the wrong kind");
    assert!(compilation.snapshot().is_none(), "a truncated term produced a score");
    assert!(compilation.has_errors(), "a truncated term compiled clean");
    assert_eq!(
        compilation.diagnostics().len(),
        1,
        "one refusal, not a cascade: {:?}",
        compilation
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>(),
    );
}

