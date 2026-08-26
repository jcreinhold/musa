//! Elaboration of the surface language through the event-track
//! (docs/rules/events/06-surface-elaboration.md, docs/rules/events/05-normalization.md).
//!
//! This is *the* semantic path: name resolution, motif registration and unit
//! checks come from `resolve.rs`, voice content elaborates into
//! `VoiceTrack` values built from events `follow`/`together`, and
//! `project.rs` reads a `ScoreSnapshot` back out of the result (§27).
//!
//! Design decisions recorded in docs/rules/events/06 and 08:
//! - a `rest` statement elaborates to a `Rest` payload occurrence — notation
//!   intent, a typed fact; the event track has no silence object (§2);
//! - transposition applies eagerly during elaboration via the shared
//!   interval stack (§19 evaluation strategy); semantically it is a payload
//!   map (§13) and composition is commutative, so eager application is
//!   observably equal by the composition law);
//! - voice/part identity rides in the payload (§32 Q3 working stance);
//! - provenance (`Origin`) rides in the payload, above the event track (§20).
//!
//! In the score adapter the event track's unit is the whole note (1 = semibreve),
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
use musa_syntax::ast::AstNode as _;

mod bars;
mod canonical;
mod fact;
mod normal;
mod place;
mod score;

#[cfg(test)]
pub(crate) use fact::map_note_pitch_fact;
pub(crate) use fact::{FactKind, ScoreFact, VoiceTrack};
pub use normal::events_normal_form;
pub(crate) use normal::piece_term;
use place::MediaDeclarations;
pub(crate) use place::{SHARED_ORIGIN, SHARED_SCOPE, instantiate};

use bars::{check_tuplets, reported_an_error};
use score::{elaborate_libraries, elaborate_material, elaborate_score};

/// Elaborate `source` through the event-track and adapt the result into
/// a `ScoreSnapshot` (docs/rules/events/06).
pub(crate) fn elaborate(source: &SourceDocument, options: &crate::CompileOptions) -> Compilation {
    let document = musa_syntax::parse(source.text());
    // Parsing is the one phase with its own timing question — a large file
    // that is slow to *parse* and a large file that is slow to *elaborate*
    // are different bugs — and the boundary between them is this line.
    tracing::debug!(phase = "parse", "parsed");
    let mut resolver = Resolver::new();
    elaborate_parsed(&document, source.name(), options, &mut resolver)
}

/// Everything after parsing (docs/rules/events/06): elaborate, adapt, check.
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
    let Some(file) = musa_syntax::ast::Document::of_root(&root) else {
        // A parse produces a `Root` and `Document` is the reader for one, so
        // nothing reaches this. It is a fallthrough rather than a panic
        // because the workspace denies the panicking macros and because there
        // is a truthful answer to hand: a file whose root cannot be read
        // declares nothing, which is what an empty compilation says.
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    // `document := declaration* piece?` and there is no third thing a file can
    // be (`docs/rules/language/01-surface.md` §1). The five statements a piece
    // owns are read at the root by the parser so that standing in the wrong
    // place can be *said*; this is where it is said.
    report_misplaced_piece_statements(resolver, &file);
    // A file is one piece however the piece got there, so the piece is the one
    // `PieceDecl` at the root and there is nothing else it could be. Without
    // one, the file exports what it declared — and a file that declares
    // modules is a package's own bookkeeping, which is the same absence read
    // one step further.
    let Some(piece) = file.piece() else {
        if !file.mods().is_empty() {
            return elaborate_module_file(resolver, &root);
        }
        return elaborate_material(resolver, &file, name, options);
    };

    resolver.realization = options.realization.clone();
    let mut snapshot = ScoreSnapshot::default();
    let mut imports = musa_syntax::ast::ImportStmt::all_at_root(&root);
    imports.extend(piece.imports());
    let libraries = crate::imports::load(resolver, name, &imports, &options.imports);
    let sources = declaring(&root, &libraries, piece.syntax());
    let Some(mut elaborated) = crate::document::elaborate(resolver, &sources) else {
        tracing::debug!(
            phase = "check",
            diagnostics = resolver.diagnostics.len(),
            "stopped at the core"
        );
        return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
    };
    // The kernel's own reading of the whole document, in test builds. The
    // per-declaration audit `declare_program` runs is a debug assertion about
    // one member; this is prompt 158's closed pass over the finished program,
    // and running it here is what puts `examples/` and every fixture the crate
    // compiles behind the same gate as the standard library.
    #[cfg(test)]
    crate::document::audit(&elaborated);
    tracing::debug!(phase = "check", diagnostics = resolver.diagnostics.len(), "checked");
    // Before the piece is read, because reading it mutates the site table and
    // a machine is a *declaration*: what this answers is the same either way,
    // and asking first is what keeps the two readings independent.
    let machines = elaborated.machines();
    elaborate_libraries(resolver, &libraries, &mut snapshot);
    resolve::lower_header(resolver, &piece, &mut snapshot);
    let media = media_declarations(resolver, &libraries, &file, Some(&piece));
    let identity = elaborate_score(resolver, &mut elaborated, &piece, &media, &mut snapshot);
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
    let imported_instruments: Vec<musa_syntax::ast::InstrumentDecl> = libraries
        .each()
        .flat_map(|(_, library)| library.instruments())
        .collect();
    let studio = resolve::lower_studio(resolver, &piece, &snapshot, &imported_studios, &imported_instruments);
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
    let declared = piece.studio().is_some()
        || !piece.instruments().is_empty()
        || piece
            .score()
            .is_some_and(|score| score.parts().iter().any(|part| part.sound().is_some()));
    let (studio_source, studio_spans) = match crate::studio::checked_source(&studio, declared) {
        Ok(source) => source,
        Err(mut diagnostics) => {
            resolver.diagnostics.append(&mut diagnostics);
            return Compilation::new(None, std::mem::take(&mut resolver.diagnostics));
        }
    };
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
        .with_studio_source(studio_source, studio_spans)
        .with_machines(machines)
        .with_identity(identity)
        .with_decisions(std::mem::take(&mut resolver.decisions))
        .with_references(references)
}

/// The five statements a piece owns, reported wherever one stands at a file
/// root.
///
/// `tempo`, `meter`, `key`, front matter, and `score` are the piece's ambient
/// state and its music, and a file root has no piece for them to belong to.
/// The parser reads them there anyway, and this is why: a grammar that simply
/// lacks the arm can only say *expected a declaration*, where a grammar that
/// reads them can name the piece each one wanted. Prompt 164a's Design calls
/// that the enrichment direction, and `AGENTS.md` calls the alternative a
/// sublanguage by subtraction.
///
/// Reported and not fatal. Every one of the five is *misplaced* rather than
/// wrong, so the declarations around it are still worth checking, and a file
/// that also declares a piece gets the rest of the piece's diagnostics in the
/// same run.
fn report_misplaced_piece_statements(resolver: &mut Resolver, file: &musa_syntax::ast::Document) {
    let has_piece = file.piece().is_some();
    let mut misplaced: Vec<(SyntaxNode, &str, &str)> = Vec::new();
    for tempo in file.tempos() {
        misplaced.push((tempo.syntax().clone(), "tempo", "sets the tempo a piece starts in"));
    }
    for meter in file.meters() {
        misplaced.push((meter.syntax().clone(), "meter", "sets the meter a piece starts in"));
    }
    for key in file.keys() {
        misplaced.push((key.syntax().clone(), "key", "sets the key a piece starts in"));
    }
    for front in file.front_matter() {
        misplaced.push((front.syntax().clone(), "front matter", "names who a piece is by"));
    }
    if let Some(score) = file.score() {
        misplaced.push((score.syntax().clone(), "score", "is a piece's music"));
    }
    for (node, what, does) in misplaced {
        let help = if has_piece {
            "move it inside the `piece { … }` block below"
        } else {
            "wrap it in `piece \"…\" { … }`, or move it into the piece that wants it"
        };
        resolver.report(
            Diagnostic::error(Code::Misplaced, format!("`{what}` belongs inside a piece"))
                .at(resolve::trimmed_span(&node), format!("{what} {does}"))
                .help(help),
        );
    }
}

/// A module file: `mod …;` declarations and nothing else.
///
/// The third document shape (`docs/rules/language/01-surface.md` §1's
/// `module-file`), and the one that owes no piece. `stdlib/src/lib.musa` is
/// the worked example: a package's root file names its children and declares
/// nothing of its own, and a directory module's `mod.musa` does the same one
/// level down (`docs/rules/language/04-templates-and-modules.md`).
///
/// It elaborates to the module tree and no exports, so there is nothing here
/// to check: which files those names reach is a question about the package,
/// and [`crate::package`] answers it against the file listing rather than
/// against one file read alone.
///
/// What *is* checked is that the file says only that. A `let` or an `import`
/// beside the `mod`s is unreachable — a module file is a path segment and not
/// a module of its own, so nothing can import what it binds — and the same
/// reasoning that makes an undeclared file "declared nowhere" makes an
/// undeclarable binding an error rather than dead weight.
fn elaborate_module_file(resolver: &mut Resolver, root: &SyntaxNode) -> Compilation {
    for child in root.children() {
        if child.kind() == musa_syntax::SyntaxKind::ModDecl {
            continue;
        }
        resolver.report(
            Diagnostic::error(Code::Misplaced, "a module file declares modules and nothing else")
                .at(resolve::trimmed_span(&child), "this is not a `mod` declaration")
                .help("move it into one of the modules this file names, or drop the `mod` declarations from this file"),
        );
    }
    Compilation::new(None, std::mem::take(&mut resolver.diagnostics)).into_modules()
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

/// Project the source-owned recorded-media declarations visible at a piece.
///
/// This follows the same closure and shadow-free order as [`declaring`]:
/// imported modules, file root, then piece. Keeping the environment beside
/// that source closure prevents full compilation and normal-form export from
/// inventing different name-resolution rules for media.
fn media_declarations(
    resolver: &mut Resolver,
    libraries: &crate::imports::Libraries,
    file: &musa_syntax::ast::Document,
    piece: Option<&musa_syntax::ast::PieceDecl>,
) -> MediaDeclarations {
    let mut media = MediaDeclarations::default();
    for (_, library) in libraries.each() {
        media.extend(resolver, library.clips(), library.fixed_media());
    }
    media.extend(resolver, file.clips(), file.fixed_media());
    if let Some(piece) = piece {
        media.extend(resolver, piece.clips(), piece.fixed_media());
    }
    media
}
