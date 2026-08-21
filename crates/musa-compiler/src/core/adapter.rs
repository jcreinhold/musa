//! One concern of the enclosing module; see its module docs.

use musa_calculus::Raw;
use musa_syntax::SyntaxKind;
use musa_syntax::ast::AstNode as _;

use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};

use super::Printer;
use super::printer_source;

/// One adapter module, elaborated in the phase environment.
///
/// **This is the phase environment**, and the only place a [`crate::document::Source`]
/// is read [`in_phase`](crate::document::Source::in_phase). Everything about it
/// is the ordinary machinery: the same declarations, the same bidirectional
/// elaborator, the same core, the same budget. What is phase-local is the
/// *environment* — [`crate::prelude::phase`]'s vocabulary answers a name here and
/// nowhere else — which is what keeps `02-core-calculus.md` §5's closed source
/// type grammar and its "no syntax value" sentence true of the language a
/// composer writes.
///
/// A module and not an expression. An adapter's operations read the module's
/// own `let`, `fn`, and `data`, because they are declarations of the module
/// those operations are declared in, and because a reader written without local
/// definitions is a reader nobody can follow (Peyton Jones ch. 3).
///
/// # What the phase no longer supplies
///
/// The types of `expand` and `edit`. They used to be a table here, matched
/// against what the checker inferred; now the adapter writes them, and the
/// application below is what checks that it wrote the right one. The reason is
/// the reason [`Printer`] already gives for `print`: a bidirectional elaborator
/// settles a `fn` where the `fn` stands, so a definition whose type is held in a
/// table beside the file is a definition the file cannot be read without. Every
/// adapter in `stdlib/src/adapters/` now writes both signatures.
pub(crate) struct AdapterModule {
    /// The module's declarations, elaborated once.
    read: crate::document::Document,
    /// `print`, which is the one declaration not read with the module.
    ///
    /// A printer's argument is the *package's* type — what its regions produce
    /// — and the package a region belongs to is not a module the adapter
    /// imports, so a standalone check of the printer would have nothing to
    /// settle its parameter against. So the printer is read where it is run,
    /// against the value it is handed ([`print_value`]), which is also why it is
    /// the one operation that never sees the phase environment.
    ///
    /// It is still read *with* the module declarations it names, because "not
    /// in the phase environment" is a statement about what is in scope and not
    /// a licence to take the module away. See [`Printer`].
    printer: Option<Printer>,
}

impl AdapterModule {
    /// What checking this module charged.
    ///
    /// `26-language-design-decision.md` §3.5's checking counters, for a run to
    /// add to its own. The module is where an adapter's *checking* actually
    /// happens — the run itself is one application of an operation whose type
    /// the module already fixed — so a phase that reported only the run would
    /// charge nothing for the expensive half and let a file buy unbounded
    /// checking by arranging to be refused.
    pub(crate) const fn spend(&self) -> musa_calculus::Spend {
        self.read.spend()
    }

    /// The text a declaration holds, when it holds one.
    ///
    /// How `level` is read: the declared level is a `Text` the module
    /// evaluates to, so asking for it is asking the module for one of its own
    /// values rather than matching the shape of its source.
    pub(crate) fn text(&self, name: &str) -> Option<String> {
        let (normal, _) = self.read.value(name).ok()?;
        crate::registry::read_back::<String>(&normal).ok().cloned()
    }

    /// Whether the module declares `name` at all.
    pub(crate) fn declares(&self, name: &str) -> bool {
        self.read.names().iter().any(|bound| &**bound == name) || (name == "print" && self.printer.is_some())
    }

    /// The printer, for the one operation read at its use site.
    pub(crate) fn printer(&self) -> Option<&Printer> {
        self.printer.as_ref()
    }

    /// Apply the operation `name` to `arguments`, and normalize the answer.
    ///
    /// One step where there used to be three. The operation's type was compared
    /// against a table, its value was pulled out as a closure, and the closure
    /// was applied by a second evaluator; now the call is written as a term and
    /// elaborated in the module's own context, so "is this an `expand`?" and
    /// "what does it answer here?" are the one question the elaborator already
    /// answers. An adapter whose `expand` takes the wrong thing is refused by
    /// the conversion check at the argument, which says which type it found and
    /// where — the sentence [`not_the_operation`] used to approximate.
    ///
    /// The arguments are terms rather than values because that is the phase's
    /// side of the boundary: a region is a literal at `Syntax ⟨token-tree⟩`, a
    /// command is a literal at `Text`, and an anchor is the prelude's `Nat`.
    pub(crate) fn run(
        &self,
        name: &str,
        arguments: Vec<Raw>,
    ) -> Result<(musa_calculus::Datum, musa_calculus::Spend), Unrun> {
        if !self.declares(name) {
            return Err(Unrun::Undeclared);
        }
        let here = musa_calculus::Origin::UNKNOWN;
        let call = Raw::call(here, Raw::var(here, name), arguments);
        let ((normal, _), spend) = self.read.term_metered(&call).map_err(|error| match error {
            musa_calculus::ElabError::Exhausted(_) => Unrun::Stopped,
            error @ (musa_calculus::ElabError::Refused(_) | musa_calculus::ElabError::Malformed(_)) => {
                Unrun::Refused(vec![crate::lower::refusals::restate(self.read.sites(), &error)])
            }
        })?;
        // Canonicity, and the whole of what it is for: the call was checked
        // before it was normalized, so a closed answer at a declared family *is*
        // one of its constructors. A term that is not one is this crate's defect
        // rather than an adapter's, which is why the two callers report it as
        // "no answer" rather than as a refusal with the adapter's name on it.
        let datum = musa_calculus::canonical(&normal).ok_or(Unrun::NoAnswer)?;
        Ok((datum, spend))
    }
}

/// Why an operation produced no answer.
///
/// Four cases and not one because the two callers restate them into two
/// different vocabularies — a transformer that is not a transformer and an
/// editor that is not an editor are different sentences — and because
/// [`ExpansionFailure`] and [`EditFailure`] each already keep a stop apart from a
/// fault, which this has to be able to tell them.
pub(crate) enum Unrun {
    /// The module declares nothing under that name.
    Undeclared,
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The application did not elaborate.
    Refused(Vec<Diagnostic>),
    /// It elaborated and its normal form was not canonical, which is a defect
    /// in this crate rather than in the adapter.
    NoAnswer,
}

/// Where an adapter module's own imports resolve.
///
/// Two things and not one because an import is resolved *relative to the module
/// that writes it*: `document` is the key the syntax import itself resolved to,
/// and `sources` is the text of every file this compilation may read. A phase
/// that took only the second would resolve `./sibling.musa` against whichever
/// file happened to be compiling.
///
/// Bundled `std::` modules answer out of [`crate::imports::ImportSources`]
/// without having been inserted into it, so a caller that hands over a default
/// one is still handing over the standard library rather than an empty world.
#[derive(Clone, Copy)]
pub(crate) struct PhaseImports<'a> {
    pub(crate) document: &'a str,
    pub(crate) sources: &'a crate::imports::ImportSources,
}

impl<'a> PhaseImports<'a> {
    /// The closure of the module stored under `document`.
    pub(crate) fn at(document: &'a str, sources: &'a crate::imports::ImportSources) -> Self {
        Self { document, sources }
    }
}

#[cfg(test)]
impl PhaseImports<'static> {
    /// The bundled standard library, for a module written in a test rather than
    /// read out of a package.
    ///
    /// A default [`crate::imports::ImportSources`] is not an empty world: it
    /// falls back to the embedded `std::` modules, which is exactly the closure
    /// a test module that writes `import std::list;` should get.
    pub(crate) fn bundled() -> Self {
        static SOURCES: std::sync::LazyLock<crate::imports::ImportSources> =
            std::sync::LazyLock::new(crate::imports::ImportSources::default);
        Self {
            document: "adapter.musa",
            sources: &SOURCES,
        }
    }
}

/// Check and evaluate one adapter module, in the phase environment.
///
/// The diagnostics come back rather than being reported: they are about the
/// adapter package's own document, and publishing a span inside it as a span in
/// the composer's file is exactly what the source map exists to prevent. The
/// caller decides which of its own spans to restate them at.
pub(crate) fn read_adapter_module(source: &str, imports: PhaseImports<'_>) -> Result<AdapterModule, ModuleFault> {
    let parsed = musa_syntax::parse(source);
    if let Some(error) = parsed.errors().first() {
        return Err(ModuleFault::Broken(vec![Diagnostic::error(
            Code::Expansion,
            format!("it does not parse: {}", error.message()),
        )]));
    }
    let root = parsed.syntax();
    let Some(library) = musa_syntax::ast::LibraryDecl::from_root(&root) else {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "an adapter module is a `library`").help(
                "write the module as `library { let level = …; let expand = …; }`, the way `stdlib/src/adapters/` does",
            ),
        ]));
    };
    // The adapter-free bootstrap, checked rather than assumed. An adapter whose
    // own definition needed an adapter would put the expansion order back into
    // a cycle, and this pair of refusals is the whole of what prevents it —
    // which is also why termination is structural and needs no rank arithmetic.
    // An *ordinary* import is not in the cycle and is loaded below: it brings a
    // module's declarations in, which is the one thing `02-core-calculus.md`
    // §5.9 says the phase adds rather than takes away.
    // Root and library both, for the reason [`ordinary_imports`] reads both: an
    // import stands at a document's lexical root or inside its `library`, and
    // which one an author chose is not something the phase should depend on.
    let written: Vec<musa_syntax::ast::ImportStmt> = root
        .descendants()
        .filter_map(musa_syntax::ast::ImportStmt::cast)
        .collect();
    if let Some(reader) = written.iter().find(|import| import.changes_syntax()) {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "it is written with an adapter of its own")
                .at(crate::resolve::trimmed_span(reader.syntax()), "this syntax import")
                .help("an adapter is written in the adapter-free bootstrap: no region, no syntax import"),
        ]));
    }
    if root.descendants().any(|node| node.kind() == SyntaxKind::SyntaxRegion) {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "it is written with an adapter of its own")
                .help("an adapter is written in the adapter-free bootstrap: no region, no syntax import"),
        ]));
    }
    let mut resolver = Resolver::new();
    let libraries = crate::imports::load(&mut resolver, imports.document, &written, imports.sources);
    if resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == musa_score::diagnose::Severity::Error)
    {
        return Err(ModuleFault::Broken(resolver.diagnostics));
    }
    let printer = printer_source(library.syntax());
    let sources: Vec<crate::document::Source> = libraries
        .each()
        .map(|(from, imported)| crate::document::Source::imported(imported.syntax(), from).in_phase())
        .chain(std::iter::once(
            crate::document::Source::own(library.syntax())
                .in_phase()
                .without(printer.iter().flat_map(Printer::left_to_it)),
        ))
        .collect();
    let Some(read) = crate::document::elaborate(&mut resolver, &sources, None) else {
        return Err(module_fault(resolver.diagnostics));
    };
    // A document that came back is not yet a module that checks: `elaborate`
    // answers `Some` for a document whose *declarations* all landed, and the
    // refusals a declaration world reports — a field storing an arrow, a name
    // declared twice — are in the resolver beside it. Answering `Ok` while it
    // holds an error would be an adapter running with a declaration the checker
    // had already refused.
    let refusals: Vec<Diagnostic> = resolver
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::diagnose::Severity::Error)
        .collect();
    if !refusals.is_empty() {
        return Err(module_fault(refusals));
    }
    Ok(AdapterModule { read, printer })
}

/// A stop when a limit was crossed, and the module's own complaints when not.
///
/// Read off the diagnostics rather than off a meter, because the meter is
/// [`musa_calculus`]'s now and it reports exhaustion the way it reports everything
/// else — as a refusal, filed under [`Code::ResourceLimit`] by
/// [`crate::lower::refusals::restate`].
fn module_fault(diagnostics: Vec<Diagnostic>) -> ModuleFault {
    if diagnostics.iter().any(|it| it.code == Code::ResourceLimit) {
        ModuleFault::Stopped
    } else {
        ModuleFault::Broken(diagnostics)
    }
}

/// Why a module could not be read as an adapter module.
///
/// Two cases and not one, for the reason [`ExpansionFailure`] separates the
/// same pair: a compilation that ran out of budget has said nothing about the
/// module, and reporting it as a broken adapter would make a narrowed budget
/// look like a package that does not compile.
pub(crate) enum ModuleFault {
    /// A compilation limit was crossed before the module finished checking.
    Stopped,
    /// It is not an adapter module, and these say why.
    Broken(Vec<Diagnostic>),
}
