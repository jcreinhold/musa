//! One concern of the enclosing module; see its module docs.

use indexmap::IndexSet;
use musa_syntax::ast::AstNode as _;

use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};

use super::ModuleFault;
use super::PhaseImports;
use super::Printer;
use super::not_the_operation;
use super::read_adapter_module;

/// Why an adapter's `print` produced no source text.
///
/// [`Self::Loss`] is `26-language-design-decision.md` §4's `PrintLoss`, and it
/// is the one of these that is not a fault at all: the printer was handed a
/// value carrying something it cannot write down, and said so. A printer that
/// quietly dropped that detail would make the round-trip law true by making the
/// value smaller, which is why the operation answers with a `Result` rather
/// than with text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PrintFailure {
    /// The adapter read the value and could not spell it — its own sentence.
    Loss(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `print` did not check against the value it was handed.
    NotAPrinter(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// Run one adapter's `print` over one ordinary value.
///
/// The third of `26-language-design-decision.md` §4's declared operations, and
/// the only one that does **not** run in the phase environment: its input is an
/// ordinary evaluated value rather than syntax, so it is an ordinary total
/// package function and is read under the ordinary reading. A printer that
/// could reach [`SYNTAX_OWNERSHIP`] would be a second way to build syntax, from
/// a value, outside the one place expansion happens.
///
/// `value` is the value as an ordinary expression, because that is the one
/// spelling of a value this crate shares with anything outside it. `A` is what
/// the printer says it takes, and the application against the value is what
/// checks the two agree. A printer *can* say it now: its parameter type is the
/// package's, and the package is in scope where the printer is read. The phase
/// still never learns that type — it is named in the adapter's `print` and read
/// at the use site, never in the module the phase checks.
pub(crate) fn print_value(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    at: &crate::compile::SourceDocument,
    value: &str,
) -> Result<String, PrintFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => PrintFailure::Stopped,
        ModuleFault::Broken(diagnostics) => PrintFailure::NotAPrinter(diagnostics),
    })?;
    let Some(printer) = module.printer() else {
        return Err(PrintFailure::NotAPrinter(vec![not_the_operation("print")]));
    };
    run_printer(printer, adapter_source, imports, at, value)
}

/// The one small piece a printer and its subject are read in.
///
/// Ordinary compilation and nothing else: the same declarations, the same
/// elaborator, the same core, and the same readback that every other document in
/// this crate gets — so [`crate::prelude::phase`]'s vocabulary is out of scope
/// exactly as the empty scope had it out. A printer that could build syntax
/// would be a second way to make an expansion, out of a value, away from the one
/// place expansion happens.
///
/// "Ordinary compilation" is [`crate::document::elaborate`] because that is what
/// the words now mean. The piece below is a document like any other, and a
/// printer read by a second checker would be a printer the language does not
/// agree about: `Ok(text_join([…]))` would have to mean here what it means
/// everywhere, and two checkers is two chances for it not to.
///
/// What the piece holds, and why each part of it is there:
///
/// - **`at`'s ordinary imports, and the adapter's.** `Document(…)` is not a name
///   an empty world holds, so neither the subject nor a `match` over it could be
///   checked without `at`'s: `at` is the document the region will be written
///   into, which makes those the same scope the printed region will itself be
///   read in — the two sides of the round-trip law, read the same way. The
///   adapter's are here for the other half of the same claim: the printer's
///   reached declarations are checked *twice*, once in the phase and once here,
///   and a second reading that could not see the modules the first one saw would
///   be a second program rather than a second reading of one. A syntax import is
///   left out for the reason [`crate::imports::load`] leaves it out: it named a
///   reader, not a module. A line both documents write is written once, because
///   two identical imports in one piece is a collision the composer did not
///   make.
/// - **The module declarations the printer names**, so that a printer may have
///   local definitions. See [`Printer`].
/// - **`printer` as a `fn` declaration, then `printed`, with the value written
///   *at* the call.** A `fn` and not a `let` for the reason [`Printer`] gives:
///   §4 fixes a printer's answer at `Result<Text, Text>`, and writing it is what
///   turns the arms from inferred into checked. `A` is still settled by checking
///   the argument against the printer's own domain rather than by declaring it:
///   what a package's regions produce is the package's business, and a phase
///   that had to be told it would be a phase that knows a type. The value is not
///   bound to a name first, because a bidirectional
///   elaborator has nothing to check a bare `let subject = Alto;` against —
///   `Alto` is a constructor of some family and the binding says of which one
///   only if the type is written, which is the very thing this must not write.
///   In argument position the domain is already known, so the same text needs no
///   annotation at all.
fn run_printer(
    printer: &Printer,
    adapter_source: &str,
    imports: PhaseImports<'_>,
    at: &crate::compile::SourceDocument,
    value: &str,
) -> Result<String, PrintFailure> {
    let mut source = String::from("piece \"print\" {\n");
    let mut written = IndexSet::new();
    written.extend(ordinary_imports(at.text()));
    let mine = written.len();
    written.extend(ordinary_imports(adapter_source));
    for import in &written {
        source.push_str(import);
        source.push('\n');
    }
    for declaration in &printer.reached {
        source.push_str(declaration);
        source.push('\n');
    }
    source.push_str("fn printer");
    source.push_str(&printer.params);
    if !printer.says_answer {
        source.push_str(" -> Result<Text, Text>");
    }
    source.push(' ');
    source.push_str(&printer.answer);
    source.push_str("\nlet printed: Result<Text, Text> = printer(");
    source.push_str(value);
    source.push_str(");\n}\n");

    let parsed = musa_syntax::parse(&source);
    if let Some(error) = parsed.errors().first() {
        return Err(PrintFailure::NotAPrinter(vec![Diagnostic::error(
            Code::Expansion,
            format!("the printer and the value do not parse together: {}", error.message()),
        )]));
    }
    let root = parsed.syntax();
    let Some(piece) = musa_syntax::ast::PieceDecl::from_root(&root) else {
        return Err(PrintFailure::NotAPrinter(vec![not_the_operation("print")]));
    };
    let mut resolver = Resolver::new();
    // The statements come back in the order they were written above, so the
    // split is where `mine` said it was.
    let statements = musa_syntax::ast::ImportStmt::all_at_root(piece.syntax());
    let (here, there) = statements.split_at(mine.min(statements.len()));
    let composers = crate::imports::load(&mut resolver, at.name(), here, imports.sources);
    let adapters = crate::imports::load(&mut resolver, imports.document, there, imports.sources);
    // One library per document, however many closures reached it: a piece that
    // reads both a composer's `std::list` and an adapter's `std::list` reaches
    // the same module twice, and declaring it twice would be a collision
    // neither of them wrote.
    let mut seen = IndexSet::new();
    let mut sources = Vec::new();
    for (importer, libraries) in [(at.name(), &composers), (imports.document, &adapters)] {
        for (from, library) in libraries.each() {
            let document = crate::imports::resolve_import(importer, from.path);
            if seen.insert((document, from.qualifier.map(str::to_owned))) {
                sources.push(crate::document::Source::imported(library.syntax(), from));
            }
        }
    }
    sources.push(crate::document::Source::own(&root));
    sources.push(crate::document::Source::own(piece.syntax()));
    let Some(document) = crate::document::elaborate(&mut resolver, &sources, None) else {
        return Err(print_failure(resolver.diagnostics));
    };
    let printed = match document.value("printed") {
        Ok((printed, _)) => printed,
        Err(musa_calculus::ElabError::Exhausted(_)) => return Err(PrintFailure::Stopped),
        Err(error) => {
            return Err(PrintFailure::NotAPrinter(vec![crate::lower::refusals::restate(
                document.sites(),
                &error,
            )]));
        }
    };
    answered(document.cx(), &printed).ok_or(PrintFailure::NoAnswer)?
}

/// The `Result<Text, Text>` a printer answered, as this module's own outcome.
///
/// [`None`] when the normal form is not one, which is a defect in this crate
/// rather than in an adapter: the term was checked at `Result<Text, Text>` above
/// before it was normalized, so a printer cannot reach it by being wrong.
///
/// [`PrintFailure::Loss`] is what the error arm *means* — the printer read the
/// value and said which part of it it could not spell — so the two arms are not
/// success and failure here. They are the two answers §4 declares.
fn answered(cx: &musa_calculus::Cx, printed: &musa_calculus::Term) -> Option<Result<String, PrintFailure>> {
    let musa_calculus::Datum::Case {
        ref constructor,
        ref fields,
    } = musa_calculus::canonical(cx, printed)?
    else {
        return None;
    };
    let [musa_calculus::Datum::Lit(ref said)] = fields[..] else {
        return None;
    };
    let said = crate::registry::held::<String>(said)?.clone();
    match &**constructor {
        "Result.Ok" => Some(Ok(said)),
        "Result.Err" => Some(Err(PrintFailure::Loss(said))),
        _ => None,
    }
}

/// The import statements of a document that bring a module's declarations in,
/// written back out verbatim.
///
/// Verbatim because an import means what it says: re-spelling `import std::x as
/// y;` from its parts would be this function deciding what the composer wrote.
fn ordinary_imports(text: &str) -> Vec<String> {
    let parsed = musa_syntax::parse(text);
    let root = parsed.syntax();
    root.descendants()
        .filter_map(musa_syntax::ast::ImportStmt::cast)
        .collect::<Vec<_>>()
        .iter()
        .filter(|import| !import.changes_syntax())
        .map(|import| import.syntax().text().to_string().trim().to_owned())
        .collect()
}

/// A stop when a limit was crossed, and the checker's complaints when not.
///
/// Read off the diagnostics for [`module_fault`]'s reason, and separate from it
/// because the two sentences differ: a printer that does not check is not an
/// adapter module that does not check.
fn print_failure(diagnostics: Vec<Diagnostic>) -> PrintFailure {
    if diagnostics.iter().any(|it| it.code == Code::ResourceLimit) {
        PrintFailure::Stopped
    } else {
        PrintFailure::NotAPrinter(diagnostics)
    }
}
