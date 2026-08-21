//! Elaboration of the surface language through the temporal kernel
//! (docs/rules/kernel/06-surface-elaboration.md, docs/rules/kernel/05-normalization.md).
//!
//! This is *the* semantic path: name resolution, motif registration and unit
//! checks come from `resolve.rs`, voice content elaborates into
//! `VoiceTrack` values built from kernel `follow`/`together`, and
//! `project.rs` reads a `ScoreSnapshot` back out of the result (§27).
//!
//! Design decisions recorded in docs/rules/kernel/06 and 08:
//! - a `rest` statement elaborates to a `Rest` payload occurrence — notation
//!   intent, a typed fact; the kernel has no silence object (§2);
//! - transposition applies eagerly during elaboration via the shared
//!   interval stack (§19 evaluation strategy); semantically it is a payload
//!   map (§13) and composition is commutative, so eager application is
//!   observably equal by the composition law);
//! - voice/part identity rides in the payload (§32 Q3 working stance);
//! - provenance (`Origin`) rides in the payload, above the kernel (§20).
//!
//! In the score adapter the kernel's unit is the whole note (1 = semibreve),
//! matching `MusicalTime`.
//!
//! Rational arithmetic here is exact and total for musa's magnitudes (see
//! `time.rs`); the workspace arithmetic lint is allowed at module scope.
//!
//! # By file
//!
//! - [`fact`] — what a score fact is: the kinds, the payload, and the maps
//!   over it that a transform performs.
//! - [`canonical`] — the admitted equality: the deterministic key a fact
//!   normalizes by.
//! - [`score`] — elaborating a piece into the snapshot the projection reads.
//! - [`place`] — placing a fact in time, and the shared coordinates a
//!   template's instances are written at.
//! - [`bars`] — barlines: the meters that decide them, and the claims measured
//!   against them.
//! - [`normal`] — the piece term and its normal form, for golden snapshots.

use crate::compile::{Compilation, SourceDocument};
use crate::resolve::{self, Resolver};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::score::ScoreSnapshot;
use musa_syntax::SyntaxNode;
use musa_syntax::ast::{AstNode as _, PieceDecl};

mod bars;
mod canonical;
mod fact;
mod normal;
mod place;
mod score;

#[cfg(test)]
pub(crate) use fact::map_note_pitch_fact;
pub(crate) use fact::{FactKind, ScoreFact, VoiceTrack};
pub use normal::kernel_normal_form;
pub(crate) use normal::piece_term;
pub(crate) use place::{SHARED_ORIGIN, SHARED_SCOPE, instantiate};

use bars::{check_tuplets, reported_an_error};
use score::{elaborate_libraries, elaborate_material, elaborate_score};

/// Elaborate `source` through the temporal kernel and adapt the result into
/// a `ScoreSnapshot` (docs/rules/kernel/06).
pub(crate) fn elaborate(source: &SourceDocument, options: &crate::CompileOptions) -> Compilation {
    let document = musa_syntax::parse(source.text());
    // Parsing is the one phase with its own timing question — a large file
    // that is slow to *parse* and a large file that is slow to *elaborate*
    // are different bugs — and the boundary between them is this line.
    tracing::debug!(phase = "parse", "parsed");
    let mut resolver = Resolver::new();
    elaborate_parsed(&document, source.name(), options, &mut resolver)
}

/// Everything after parsing (docs/rules/kernel/06): elaborate, adapt, check.
///
/// Split out of [`elaborate`] so the parse and the semantic work can be
/// measured apart; `resolver` arrives from the caller for the same reason
/// (see `crate::bench`). The production path passes a fresh one.
pub(crate) fn elaborate_parsed(
    document: &musa_syntax::ParsedDocument,
    name: &str,
    options: &crate::CompileOptions,
    resolver: &mut Resolver,
) -> Compilation {
    for error in document.errors() {
        let range = error.range();
        let span = SourceSpan::new(u32::from(range.start()), u32::from(range.end()));
        let mut diagnostic = Diagnostic::error(Code::Syntax, error.message()).at(span, error.label());
        if let Some(help) = error.help() {
            diagnostic = diagnostic.help(help);
        }
        if let Some((title, replacement)) = error.fix() {
            diagnostic = diagnostic.fix(title, span, replacement);
        }
        resolver.report(diagnostic);
    }
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == musa_score::diagnose::Severity::Error)
    {
        tracing::debug!(
            phase = "syntax",
            errors = resolver.diagnostics.len(),
            "stopped at syntax"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let root = document.syntax();
    let mut templates = crate::template::Templates::collect(resolver, &root);
    // The piece a document declares: written out, or made by an instance
    // standing where it would be. Both are one piece, and everything after
    // this line reads the same `PieceDecl` either way.
    let made = musa_syntax::ast::MakeStmt::from_root(&root)
        .and_then(|site| templates.instance(resolver, &site, "piece", crate::template::Kind::Piece, None, name));
    let Some(piece) = PieceDecl::from_root(&root).or_else(|| made.as_ref().and_then(crate::template::Instance::piece))
    else {
        if let Some(library) = musa_syntax::ast::LibraryDecl::from_root(&root) {
            return elaborate_material(resolver, &library, name, options);
        }
        if musa_syntax::ast::MakeStmt::from_root(&root).is_none() {
            resolver.report(
                Diagnostic::error(Code::Misplaced, "this file declares no piece")
                    .at(SourceSpan::new(0, 0), "expected `piece \"…\" { … }`")
                    .help("every musa file is one piece, or a `library { … }` for others to import"),
            );
        }
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    if piece.is_template() && made.is_none() {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "a piece with parameters needs `template`")
                .at(resolve::trimmed_span(piece.syntax()), "this piece takes parameters")
                .help("write `template piece …` and a `make … as …;` for each instance"),
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }

    resolver.realization = options.realization.clone();
    let mut snapshot = ScoreSnapshot::default();
    let mut imports = musa_syntax::ast::ImportStmt::all_at_root(&root);
    imports.extend(piece.imports());
    let libraries = crate::imports::load(resolver, name, &imports, &options.imports);
    let sources = declaring(&root, &libraries, piece.syntax());
    let Some(mut elaborated) = crate::document::elaborate(resolver, &sources, made.as_ref()) else {
        tracing::debug!(
            phase = "check",
            diagnostics = resolver.diagnostics.len(),
            "stopped at the core"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    tracing::debug!(phase = "check", diagnostics = resolver.diagnostics.len(), "checked");
    // Before the piece is read, because reading it mutates the site table and
    // a machine is a *declaration*: what this answers is the same either way,
    // and asking first is what keeps the two readings independent.
    let machines = elaborated.machines();
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot);
    let identity = elaborate_score(resolver, &mut elaborated, &piece, name, &mut snapshot);
    // The identity hash is the one fact that says *which* piece was produced,
    // and it is what two runs that should agree are compared on.
    tracing::debug!(phase = "elaborate", %identity, "elaborated");
    snapshot.set_annotations(std::mem::take(&mut resolver.annotations));
    // Advice about a piece that does not compile is advice about a piece that
    // does not exist. A bar reported as a quarter too long already makes every
    // later barline wrong, and "this voice stops part-way through measure 3"
    // is that same quarter, said again from further away.
    if !reported_an_error(resolver) {
        resolve::check_measure_sanity(resolver, &snapshot);
        resolve::check_groove_has_a_meter(resolver, &snapshot);
        check_tuplets(resolver, &snapshot);
    }
    if reported_an_error(resolver) {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    let imported_studios: Vec<musa_syntax::ast::StudioDecl> =
        libraries.each().filter_map(|(_, library)| library.studio()).collect();
    let studio = resolve::lower_studio(resolver, &piece, &snapshot, &imported_studios);
    if resolver
        .diagnostics
        .iter()
        .any(|d| d.severity == musa_score::diagnose::Severity::Error)
    {
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    }
    // The lint pass reads the resolver's reference index and the lowered
    // studio, so it runs before either leaves the resolver — and after every
    // error check above, because advice about a piece that does not compile
    // is advice about a piece that does not exist.
    let references = std::mem::take(&mut resolver.references);
    let lints = crate::lint::lint(document, &piece, &references, &studio);
    resolver.diagnostics.extend(lints);
    // A piece that asked nothing was not realized, it was compiled, and the
    // score says so by having no performance at all. Everything downstream —
    // the seed field, the Origin step, the export note — appears and vanishes
    // on this one value.
    let mut snapshot = snapshot;
    if !resolver.decisions.is_empty() {
        snapshot.set_performance(resolver.realization.seed());
    }
    Compilation::new(Some(snapshot), std::mem::take(&mut resolver.diagnostics))
        .with_studio(studio)
        .with_machines(machines)
        .with_identity(identity)
        .with_decisions(std::mem::take(&mut resolver.decisions))
        .with_references(references)
}

/// Every node whose declarations this piece is read against, in reading order.
///
/// The import closure first, then the document's own root, then the piece —
/// which is a source of its own because a `piece` *declares*: its motifs, its
/// fragments and its `let`s are the names its voices write, and a walk that
/// stopped at the root would leave every one of them unbound. A template's body
/// arrives the same way, since the piece a root `make` names is a node like any
/// other.
///
/// None of them is in phase: [`crate::imports::load`] leaves an
/// `import … changes syntax` out of the closure, and a phase module is
/// elaborated by [`crate::expand`] under vocabulary of its own.
fn declaring(
    root: &SyntaxNode,
    libraries: &crate::imports::Libraries,
    piece: &SyntaxNode,
) -> Vec<crate::document::Source> {
    libraries
        .each()
        .map(|(from, library)| crate::document::Source::imported(library.syntax(), from))
        .chain([crate::document::Source::own(root), crate::document::Source::own(piece)])
        .collect()
}
