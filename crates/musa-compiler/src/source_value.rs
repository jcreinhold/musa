//! Checked ordinary source data for host consumers.
//!
//! This is a second facade operation, not a second semantic path: it performs
//! the same expansion, import loading, declaration elaboration, inference, and
//! normalization as [`crate::compile`], then asks `musa-calculus` to freeze one
//! named value as canonical data. It produces no score and knows no DSP schema.

use musa_score::diagnose::{Code, Diagnostic, Severity};
use musa_score::origin::SourceSpan;
use musa_syntax::ast::AstNode as _;

use crate::{CompileOptions, SourceDocument};

/// Check and freeze one named ordinary source value.
///
/// The binding may be declared at the document root or inside its piece. The
/// returned artifact has no public constructor and contains exact framed data,
/// not evaluator state or printed source.
///
/// # Errors
///
/// Returns every syntax, import, declaration, elaboration, normalization, or
/// artifact-schema diagnostic encountered. No artifact is returned beside an
/// error-severity diagnostic.
pub fn checked_source_value(
    source: &SourceDocument,
    options: &CompileOptions,
    binding: &str,
    schema: &musa_calculus::SourceSchema,
) -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    musa_calculus::with_stack_room(|| checked_source_value_in_room(source, options, binding, schema))
}

fn checked_source_value_in_room(
    source: &SourceDocument,
    options: &CompileOptions,
    binding: &str,
    schema: &musa_calculus::SourceSchema,
) -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    let expansion = crate::expand::expand(source, options);
    let mut resolver = crate::resolve::Resolver::new();
    resolver.diagnostics.extend(expansion.diagnostics.iter().cloned());
    let parsed = musa_syntax::parse(expansion.document.text());
    for error in parsed.errors() {
        let range = error.range();
        let span = SourceSpan::new(u32::from(range.start()), u32::from(range.end()));
        let mut diagnostic = Diagnostic::error(Code::Syntax, error.message()).at(span, error.label());
        if let Some(help) = error.help() {
            diagnostic = diagnostic.help(help);
        }
        resolver.report(diagnostic);
    }
    if has_errors(&resolver.diagnostics) {
        return Err(resolver.diagnostics);
    }
    let root = parsed.syntax();
    let Some(file) = musa_syntax::ast::Document::of_root(&root) else {
        return Err(vec![Diagnostic::error(
            Code::Syntax,
            "this source has no document root",
        )]);
    };
    let mut imports = musa_syntax::ast::ImportStmt::all_at_root(&root);
    if let Some(piece) = file.piece() {
        imports.extend(piece.imports());
    }
    let libraries = crate::imports::load(&mut resolver, source.name(), &imports, &options.imports);
    if has_errors(&resolver.diagnostics) {
        return Err(resolver.diagnostics);
    }
    let mut sources: Vec<_> = libraries
        .each()
        .map(|(from, library)| crate::document::Source::imported(library.syntax(), from))
        .chain([crate::document::Source::own(&root)])
        .collect();
    if let Some(piece) = file.piece() {
        sources.push(crate::document::Source::own(piece.syntax()));
    }
    let Some(document) = crate::document::elaborate(&mut resolver, &sources) else {
        return Err(resolver.diagnostics);
    };
    match document.checked_source(binding, schema) {
        Ok(value) if !has_errors(&resolver.diagnostics) => Ok(value),
        Ok(_) => Err(resolver.diagnostics),
        Err(error) => {
            resolver.report(Diagnostic::error(Code::NotAValue, error.to_string()));
            Err(resolver.diagnostics)
        }
    }
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// Check the edition-pinned standard studio vocabulary as ordinary source.
///
/// This is the one bridge used by documentation, tooling, and host agreement
/// checks. It deliberately returns the generic checked artifact: the compiler
/// does not own or interpret the sound schema.
///
/// # Errors
///
/// Returns the standard-library import, checking, normalization, or artifact
/// diagnostics. A bundled vocabulary that fails here is a build defect.
pub fn checked_standard_studio_vocabulary() -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    const PROBE: &str = r#"import std::sound::catalogue;
piece "Studio vocabulary" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    checked_source_value(
        &SourceDocument::new(PROBE, "musa-stdlib:/studio-vocabulary.musa"),
        &CompileOptions::default(),
        "studio_vocabulary",
        &musa_calculus::SourceSchema::new(
            "std.sound.catalogue.StudioVocabularyArtifact",
            "StudioVocabularyArtifact",
            1,
        ),
    )
}

/// Check the edition-pinned performance vocabulary as ordinary source.
///
/// Gesture and control meanings live in `std::performance`; this bridge only
/// requests its finite, normalized artifact from the ordinary checker.
///
/// # Errors
///
/// Returns standard-library import, checking, normalization, or artifact
/// diagnostics. A bundled vocabulary that fails here is a build defect.
pub fn checked_standard_performance_vocabulary() -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    const PROBE: &str = r#"import std::performance;
piece "Performance vocabulary" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    checked_source_value(
        &SourceDocument::new(PROBE, "musa-stdlib:/performance-vocabulary.musa"),
        &CompileOptions::default(),
        "performance_vocabulary",
        &musa_calculus::SourceSchema::new(
            "std.performance.PerformanceVocabularyArtifact",
            "PerformanceVocabularyArtifact",
            1,
        ),
    )
}

/// Check the edition-pinned standard instruments and their private machines.
///
/// The compiler returns only the generic checked artifact. Instrument schema,
/// mapping, and implementation policy remain declarations in `std::sound`;
/// the DSP boundary may derive a read-only preparation projection from this
/// exact value.
///
/// # Errors
///
/// Returns standard-library import, checking, normalization, or artifact
/// diagnostics. A bundled instrument that fails here is a build defect.
pub fn checked_standard_instruments() -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    const PROBE: &str = r#"import std::sound::instrument;
piece "Standard instruments" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    checked_source_value(
        &SourceDocument::new(PROBE, "musa-stdlib:/standard-instruments.musa"),
        &CompileOptions::default(),
        "standard_instruments",
        &musa_calculus::SourceSchema::new(
            "std.sound.instrument.InstrumentExecutionArtifact",
            "InstrumentExecutionArtifact",
            2,
        ),
    )
}

/// Check and project the private registered-machine component of the bundled
/// basic instrument.
///
/// Machine values deliberately use their existing distinct compiler
/// projection rather than pretending to be canonical record data. The binding
/// remains private to `std::sound::instrument`; this operation is the narrow
/// preparation seam that may name it.
///
/// # Errors
///
/// Returns diagnostics from checking the declaring module, or a build-defect
/// diagnostic when the edition-pinned private binding is absent.
pub fn checked_standard_instrument_machine() -> Result<musa_score::MachineSpec, Vec<Diagnostic>> {
    const URI: &str = "musa-stdlib:/std/sound/instrument.musa";
    let source = crate::standard_library_source(URI).ok_or_else(|| {
        vec![Diagnostic::error(
            Code::Import,
            "the bundled instrument module is absent",
        )]
    })?;
    let compilation = crate::compile(&SourceDocument::new(source, URI), &CompileOptions::default());
    if compilation.has_errors() {
        return Err(compilation.diagnostics().to_vec());
    }
    compilation.machine("basic_sine_machine").cloned().ok_or_else(|| {
        vec![Diagnostic::error(
            Code::NotAValue,
            "the bundled basic instrument has no checked machine component",
        )]
    })
}

/// Check the edition-pinned transcription policies as ordinary source.
///
/// Subdivision, tuplet, weight, window, and search-bound policy live in
/// `std::transcription`. The compiler returns only the generic checked
/// artifact; the host optimizer projects one named policy from this exact
/// value and never constructs one.
///
/// # Errors
///
/// Returns standard-library import, checking, normalization, or artifact
/// diagnostics. A bundled transcription policy that fails here is a build
/// defect.
pub fn checked_standard_transcription_policies() -> Result<musa_calculus::CheckedSource, Vec<Diagnostic>> {
    const PROBE: &str = r#"import std::transcription;
piece "Transcription policy" {
    meter 4/4;
    key c major;
    score { part proof { voice observed { rest/1 } } }
}
"#;
    checked_source_value(
        &SourceDocument::new(PROBE, "musa-stdlib:/transcription-policy.musa"),
        &CompileOptions::default(),
        "transcription_policies",
        &musa_calculus::SourceSchema::new(
            "std.transcription.TranscriptionPolicyArtifact",
            "TranscriptionPolicyArtifact",
            1,
        ),
    )
}
