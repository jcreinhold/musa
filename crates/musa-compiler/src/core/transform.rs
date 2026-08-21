//! One concern of the enclosing module; see its module docs.

#[cfg(test)]
use musa_language::ast::AstNode as _;
use musa_score::diagnose::{Code, Diagnostic};

#[cfg(test)]
use crate::resolve::Resolver;

use super::ExpansionFailure;
use super::ModuleFault;
use super::PhaseImports;
use super::PhaseWork;
use super::Unrun;
use super::{read_adapter_module, refusal_of, region};

/// The text an ordinary expression evaluates to, for a law that compares two
/// values rather than two spellings of one.
///
/// The round-trip law of §4 is about *values*, and the fixture that carries it
/// states its equality as text equality — which is the fixture's own equality
/// function, not a structural comparison of the printed source, because
/// printing is allowed to normalize.
#[cfg(test)]
pub(crate) fn evaluate_text(expression: &str) -> Option<String> {
    let parsed = musa_language::parse(&format!("library {{\n  let it: Text = {expression};\n}}"));
    if !parsed.errors().is_empty() {
        return None;
    }
    let library = musa_language::ast::LibraryDecl::from_root(&parsed.syntax())?;
    let mut resolver = Resolver::new();
    let document = crate::document::elaborate(&mut resolver, &[crate::document::Source::own(library.syntax())], None)?;
    let (normal, _ty) = document
        .term(&musa_calculus::Raw::var(musa_calculus::Origin::UNKNOWN, "it"))
        .ok()?;
    crate::registry::read_back::<String>(&normal).ok().cloned()
}

/// Run one transformer over one already-read region, in the phase environment.
///
/// The work is reported whichever way the run came out, because what a run
/// cost does not depend on what it answered: an adapter that reads a whole
/// region and then refuses it has read a whole region, and a phase that
/// charged nothing for that would let a file buy unbounded reading by
/// arranging to be refused.
pub(crate) fn expand_syntax(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: &crate::syntax::Syntax,
) -> (Result<crate::syntax::Syntax, ExpansionFailure>, PhaseWork) {
    // Built here and lent inwards rather than made on the other side of
    // `with_room`: what a run charged is reported by the caller that asked for
    // it, and a total kept on a scoped thread would go out of scope with it.
    let mut spent = musa_calculus::Spend::default();
    let answer = with_room(adapter_source, imports, subject, &mut spent);
    (answer, PhaseWork::of(spent))
}

/// Run one transformer with enough stack for the nesting the budget allows.
///
/// The budget is the guard: `Budget::LANGUAGE`'s nesting limit is what turns a
/// region too deep to read into a refusal that names an operation and a place
/// (`../rules/language/02-core-calculus.md` §4). This is what makes that
/// refusal *reachable*. A limit no host can afford to run up to is a limit the
/// process dies before hitting, and not dying is the whole exercise. Neither
/// half stands in for the other: room alone only moves the cliff, and a limit
/// alone only promises a diagnostic the machine may not live to print.
///
/// The room is `NESTING × FRAME_CEILING`, derived rather than picked, so
/// raising the published limit cannot quietly outrun the stack that honours it.
///
/// A host with no threads — the wasm shell — takes the second path and runs on
/// the stack it was linked with, where the room is a link-time setting instead.
/// The budget does not move, so both hosts accept and refuse exactly the same
/// programs; they differ only in what they survive.
fn with_room(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: &crate::syntax::Syntax,
    spent: &mut musa_calculus::Spend,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let room = usize::try_from(crate::core_budget::NESTING.saturating_mul(crate::core_budget::FRAME_CEILING))
        .unwrap_or(usize::MAX);
    let mut answer = None;
    if !cfg!(target_family = "wasm") {
        std::thread::scope(|scope| {
            let run = || answer = Some(run_transformer(adapter_source, imports, subject.clone(), spent));
            if let Ok(running) = std::thread::Builder::new().stack_size(room).spawn_scoped(scope, run)
                && let Err(panic) = running.join()
            {
                // A panic inside is a compiler fault. Resuming it on this side
                // keeps it looking like one, rather than like a phase that
                // quietly answered nothing.
                std::panic::resume_unwind(panic);
            }
        });
    }
    answer.unwrap_or_else(|| run_transformer(adapter_source, imports, subject.clone(), spent))
}

fn run_transformer(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    spent: &mut musa_calculus::Spend,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => ExpansionFailure::Stopped,
        ModuleFault::Broken(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
    })?;
    *spent = spent.and(module.spend());
    let (answer, spend) = module
        .run("expand", vec![region(subject)])
        .map_err(|unrun| match unrun {
            Unrun::Undeclared => ExpansionFailure::NotATransformer(vec![not_the_operation("expand")]),
            Unrun::Stopped => ExpansionFailure::Stopped,
            Unrun::Refused(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
            Unrun::NoAnswer => ExpansionFailure::NoAnswer,
        })?;
    *spent = spent.and(spend);
    let produced = expanded(&answer).ok_or(ExpansionFailure::NoAnswer)??;
    // The gate again, here rather than only in `checked_expression`: a
    // transformer that never called the builtin has still produced output the
    // rest of the compiler will have to anchor diagnostics against.
    crate::syntax::check_expression(&produced).map_err(ExpansionFailure::NotAnExpression)?;
    Ok(produced)
}

/// The `Result<Syntax, Pair<Syntax, Text>>` a transformer answered.
///
/// [`None`] for [`answered`]'s reason, and the error arm is not a fault for
/// [`ExpansionFailure::Refused`]'s: a transformer that answers `Err` has
/// *worked*, and what it says is the adapter package's sentence about the
/// composer's text.
fn expanded(answer: &musa_calculus::Datum) -> Option<Result<crate::syntax::Syntax, ExpansionFailure>> {
    let musa_calculus::Datum::Case {
        ref constructor,
        ref fields,
    } = *answer
    else {
        return None;
    };
    let [ref held] = fields[..] else { return None };
    match &**constructor {
        "Result.Ok" => {
            let musa_calculus::Datum::Lit(ref produced) = *held else {
                return None;
            };
            Some(Ok(crate::registry::held::<crate::syntax::Syntax>(produced)?.clone()))
        }
        "Result.Err" => Some(Err(refusal_of(held)?)),
        _ => None,
    }
}

/// The complaint for a module that declares nothing under a name the phase
/// runs.
///
/// One case where there were two. A module whose `expand` is not a transformer
/// used to be told the type the phase found instead; now the *call* is what
/// checks it, so that author is told which argument did not fit and where, by
/// the conversion check itself. What is left here is the case there is no call
/// to make.
pub(crate) fn not_the_operation(name: &str) -> Diagnostic {
    Diagnostic::error(Code::Expansion, format!("it declares no `{name}`")).help(match name {
        "expand" => {
            "an adapter declares `let expand = fn (region) { … };`, answering `Ok(syntax)` or `Err((node, why))`"
        }
        "edit" => "an adapter declares `let edit = fn (region, command, anchor, argument) { … };`",
        _ => "an adapter declares `let print = fn (value) { … };`, answering `Ok(text)` or `Err(loss)`",
    })
}
