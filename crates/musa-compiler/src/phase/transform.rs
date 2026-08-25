//! One concern of the enclosing module; see its module docs.

use musa_score::diagnose::{Code, Diagnostic};
#[cfg(test)]
use musa_syntax::ast::AstNode as _;

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
    let parsed = musa_syntax::parse(&format!("\n  let it: Text = {expression};\n"));
    if !parsed.errors().is_empty() {
        return None;
    }
    let library = musa_syntax::ast::Document::of_root(&parsed.syntax())?;
    let mut resolver = Resolver::new();
    let document = crate::document::elaborate(&mut resolver, &[crate::document::Source::own(library.syntax())])?;
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
    subject: &crate::quote::Syntax,
) -> (Result<crate::quote::Syntax, ExpansionFailure>, PhaseWork) {
    let mut spent = musa_calculus::Spend::default();
    let answer = run_transformer(adapter_source, imports, subject.clone(), &mut spent);
    (answer, PhaseWork::of(spent))
}

fn run_transformer(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::quote::Syntax,
    spent: &mut musa_calculus::Spend,
) -> Result<crate::quote::Syntax, ExpansionFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped(limit) => ExpansionFailure::Stopped(limit),
        ModuleFault::Broken(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
    })?;
    *spent = spent.and(module.spend());
    let (answer, spend) = module
        .run("expand", vec![region(subject)])
        .map_err(|unrun| match unrun {
            Unrun::Undeclared => ExpansionFailure::NotATransformer(vec![not_the_operation("expand")]),
            Unrun::Stopped(limit) => ExpansionFailure::Stopped(limit),
            Unrun::Refused(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
            Unrun::NoAnswer => ExpansionFailure::NoAnswer,
        })?;
    *spent = spent.and(spend);
    let produced = expanded(&answer).ok_or(ExpansionFailure::NoAnswer)??;
    // The gate again, here rather than only in `checked_expression`: a
    // transformer that never called the builtin has still produced output the
    // rest of the compiler will have to anchor diagnostics against.
    crate::quote::check_expression(&produced).map_err(ExpansionFailure::NotAnExpression)?;
    Ok(produced)
}

/// The `Result<Syntax, Pair<Syntax, Text>>` a transformer answered.
///
/// [`None`] for [`answered`]'s reason, and the error arm is not a fault for
/// [`ExpansionFailure::Refused`]'s: a transformer that answers `Err` has
/// *worked*, and what it says is the adapter package's sentence about the
/// composer's text.
fn expanded(answer: &musa_calculus::Datum) -> Option<Result<crate::quote::Syntax, ExpansionFailure>> {
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
            Some(Ok(crate::registry::held::<crate::quote::Syntax>(produced)?.clone()))
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
