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
