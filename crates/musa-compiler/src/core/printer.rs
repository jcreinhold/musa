//! One concern of the enclosing module; see its module docs.

use indexmap::IndexSet;
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};

use super::phase_type;

/// `print`, and the module declarations it reads.
///
/// The one operation read as text rather than as a value, for the reason
/// [`AdapterModule::printer`] gives — and read *with* its module rather than
/// alone. A musa block holds exactly one expression and the language has no
/// `let` expression, so an operation read as a bare expression cannot bind a
/// single local name: it would be the language minus local definitions, which
/// is the sublanguage by subtraction root `AGENTS.md` forbids and the shape
/// Peyton Jones ch. 3 enriches the calculus to avoid. `expand` and `edit` never
/// felt it because they are checked with the module; this is what gives `print`
/// the same.
///
/// The declarations it *names*, transitively, and not the whole module — which
/// makes a module two parts rather than one. A helper written for the printer
/// names the package's types, and the phase has no such types, so a module
/// checked as one piece could not hold one at all: the phase would refuse the
/// helper before the printer was ever read. So a declaration belongs to
/// whichever operations name it. `level`, `expand` and `edit` are the phase's
/// roots; `print` is the printer's; a declaration both reach is checked twice,
/// once under each reading, which is the honest answer for a helper that is
/// genuinely both. One neither reaches is dead, and stays with the phase so
/// that it is still checked rather than quietly ignored.
///
/// # Why the lambda is kept in two pieces
///
/// [`run_printer`] splices it back as a `fn` declaration rather than as a `let`
/// bound to a lambda, and needs the halves apart to do it. The reason is
/// bidirectional: `let printer = fn (value: Text) { match value { … } };` gives
/// the elaborator a domain and no codomain, so the arms are *inferred*, and
/// `Ok("hello")` on its own infers `Result Text ?e` with nothing to solve `?e`
/// — an unsolved metavariable in a printer that is perfectly well typed. Written
/// as `fn printer(value: Text) -> Result<Text, Text> { … }`, the same arms are
/// *checked*, and the error type comes from the annotation `04-adapters.md` §4
/// already fixes for every printer. Algorithm W did not need this because it
/// solved the whole piece at once; a checker that reads a definition where it
/// stands needs the type written where it stands.
pub(crate) struct Printer {
    /// The parameter list, verbatim — `(value: Text)`.
    pub(crate) params: String,
    /// Everything after it: the body, and the `-> T` before it when the adapter
    /// wrote one.
    pub(crate) answer: String,
    /// Whether it did. When it did not, [`run_printer`] supplies the return type
    /// §4 fixes; when it did, the adapter's own words stand and `printed`'s
    /// annotation is what checks the two agree.
    pub(crate) says_answer: bool,
    /// The module declarations the body reaches, in the order the module writes
    /// them, verbatim.
    pub(crate) reached: Vec<String>,
    /// The names of those the phase must *not* check, being the printer's alone.
    private: IndexSet<String>,
}

impl Printer {
    /// Every name the phase leaves to the printer, `print` included.
    ///
    /// What [`crate::document::Source::without`] is handed. `print` is on the
    /// list because it is the operation itself and the rest because they are
    /// reached from nowhere else, and both halves are the same sentence: a
    /// declaration whose type only the *package* names cannot be read in a phase
    /// that has no package.
    pub(crate) fn left_to_it(&self) -> impl Iterator<Item = String> {
        std::iter::once("print".to_owned()).chain(self.private.iter().cloned())
    }
}

/// The declarations a set of root names reaches, transitively.
///
/// By repetition rather than by recursion: each pass takes the declarations the
/// names so far reach and adds what *they* name, until a pass adds nothing.
fn reached_by(module: &[(Vec<String>, IndexSet<String>, String)], roots: IndexSet<String>) -> Vec<bool> {
    let mut wanted = roots;
    let mut taken = vec![false; module.len()];
    loop {
        let mut grew = false;
        for (reached, (binds, names, _)) in taken.iter_mut().zip(module) {
            if *reached || !binds.iter().any(|name| wanted.contains(name)) {
                continue;
            }
            *reached = true;
            wanted.extend(names.iter().cloned());
            grew = true;
        }
        if !grew {
            return taken;
        }
    }
}

pub(crate) fn printer_source(library: &SyntaxNode) -> Option<Printer> {
    let whole = library.to_string();
    let base = usize::try_from(u32::from(library.text_range().start())).ok()?;
    let written = |from: u32, to: u32| -> Option<String> {
        let from = usize::try_from(from).ok()?.checked_sub(base)?;
        let to = usize::try_from(to).ok()?.checked_sub(base)?;
        Some(whole.get(from..to)?.trim().to_owned())
    };
    // What each declaration binds, what it names, and its own source — in the
    // order the module writes them, because that is the order they have to be
    // spliced back in.
    let mut module: Vec<(Vec<String>, IndexSet<String>, String)> = Vec::new();
    let mut printer = None;
    for node in library.children() {
        let range = node.text_range();
        if matches!(node.kind(), SyntaxKind::LetDecl | SyntaxKind::FnDecl) {
            let Some(name) = declared_name(&node) else { continue };
            if node.kind() == SyntaxKind::LetDecl && name == "print" {
                printer = Some(printer_body(&node, &written)?);
            } else {
                module.push((
                    vec![name],
                    names_in(&node),
                    written(range.start().into(), range.end().into())?,
                ));
            }
        } else if let Some(declaration) = musa_syntax::ast::DataDecl::cast(node.clone()) {
            // A `data` is reached by its type, by any of its constructors, or
            // by the fold generated for it: those are the whole of what naming
            // it can look like from a printer.
            let mut binds: Vec<String> = declaration.variants().iter().filter_map(|it| it.name()).collect();
            if let Some(name) = declaration.name() {
                binds.push(crate::data::fold_name(&name));
                binds.push(name);
            }
            module.push((
                binds,
                names_in(&node),
                written(range.start().into(), range.end().into())?,
            ));
        }
    }
    let (params, answer, says_answer) = printer?;
    let printers = reached_by(&module, names_in_text(&format!("fn {params} {answer}")));
    let phases = reached_by(
        &module,
        ["level", "expand", "edit"].into_iter().map(str::to_owned).collect(),
    );
    let mut reached = Vec::new();
    let mut private = IndexSet::new();
    for ((binds, names, text), (&mine, &theirs)) in module.into_iter().zip(printers.iter().zip(&phases)) {
        if !mine || names_a_phase_type(&names) {
            continue;
        }
        if !theirs {
            private.extend(binds);
        }
        reached.push(text);
    }
    Some(Printer {
        params,
        answer,
        says_answer,
        reached,
        private,
    })
}

/// Whether a declaration writes down one of the phase's own types.
///
/// The one place the printer's reachability may not over-reach. A declaration
/// naming `Syntax`, `NodePath`, `BindingPath` or `SyntaxStep` is the phase's:
/// the ordinary reading has no such type, so splicing it does not cost "a
/// little checking" — it refuses the whole piece, and refuses it with a
/// complaint about a declaration the printer never asked for. So the phase
/// keeps it, and a printer that genuinely reached one is refused for the name
/// it wrote rather than for a type it did not.
///
/// This is the same boundary [`Printer`] states, applied to the declaration
/// rather than to the type: a `data` is reached by any of its constructors, and
/// a constructor name an adapter shares with the package it reads — `Untied` is
/// both `Tying`'s and `Tie`'s — is exactly where the over-approximation
/// otherwise drags the phase's half into the printer's piece.
fn names_a_phase_type(names: &IndexSet<String>) -> bool {
    names
        .iter()
        .any(|name| phase_type(name).is_some() || matches!(name.as_str(), "Syntax" | "SyntaxStep"))
}

/// `let print = fn (…) …;` split where the parameter list ends.
///
/// [`None`] when the declaration's value is not a lambda at all, which refuses
/// the module as "not a printer" — the same answer §4 gives for a `print` that
/// is not `fn (value: T) { … }`, reached one step earlier.
fn printer_body(
    declaration: &SyntaxNode,
    written: &impl Fn(u32, u32) -> Option<String>,
) -> Option<(String, String, bool)> {
    let lambda = declaration
        .children()
        .find(|node| node.kind() == SyntaxKind::LambdaExpr)?;
    let params = lambda
        .children()
        .find(|node| node.kind() == SyntaxKind::ParamList)?
        .text_range();
    Some((
        written(params.start().into(), params.end().into())?,
        written(params.end().into(), lambda.text_range().end().into())?,
        lambda.children_with_tokens().any(|it| it.kind() == SyntaxKind::Arrow),
    ))
}

/// Every name written anywhere inside a declaration.
///
/// Deliberately more than the names it *reads*: a parameter, a field, and a
/// bound pattern variable all land here too. Over-reaching mostly splices a
/// declaration the printer did not need, which costs a little checking;
/// under-reaching would leave a name unbound and refuse a printer that was
/// correct. The one case where over-reaching is not cheap is a declaration
/// written in the phase's language, and [`names_a_phase_type`] is what keeps
/// that one out.
fn names_in(node: &SyntaxNode) -> IndexSet<String> {
    node.descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
        .collect()
}

/// The same over source text, for the printer's own body, which is text by the
/// time anything asks what it names.
fn names_in_text(source: &str) -> IndexSet<String> {
    names_in(&musa_syntax::parse(&format!("library {{ let named = {source}; }}")).syntax())
}

/// The name a `let` declares.
fn declared_name(declaration: &SyntaxNode) -> Option<String> {
    declaration
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}
