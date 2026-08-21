//! One concern of the enclosing module; see its module docs.

use crate::diagnose::Diagnostic;
use crate::origin::SourceSpan;

use super::said;

#[cfg(test)]
use super::{PhaseImports, expand_syntax};

/// What running a transformer over a region produced, or why it did not.
///
/// The three failures are different mistakes and a transformer author reading
/// one should be told which they made, which is why this is a type rather than
/// a `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExpansionFailure {
    /// The adapter read the region and would not have it.
    ///
    /// Not a fault: a transformer that answers `Err` has *worked*, and what it
    /// says is the adapter package's sentence about the composer's text. The
    /// node is how it points — `26-language-design-decision.md` §3.4 gives a
    /// transformer no way to read a source range, so it hands back a node it
    /// was given and the range is read from that.
    Refused {
        /// The adapter's own sentence.
        message: String,
        /// Where it pointed, when it pointed at something the composer wrote.
        /// `None` means the adapter pointed at a node it built itself, which
        /// has no text under it.
        at: Option<SourceSpan>,
    },
    /// A compilation limit was crossed before the run finished.
    ///
    /// Not a fault in the adapter and not a fault in the region: a transformer
    /// is total, so this is the meter stopping a run rather than a run that
    /// would not have stopped. It is its own case because
    /// `docs/rules/language/00-semantics.md` §2 makes the difference matter —
    /// a stop must not read as a file that is not well-typed.
    Stopped,
    /// The transformer did not check as `Syntax -> Syntax`.
    NotATransformer(Vec<Diagnostic>),
    /// It checked and then did not answer — a budget crossed, or the evaluator
    /// and the checker disagreeing, which is a compiler fault rather than a
    /// language effect.
    NoAnswer,
    /// It answered with something that is not a well-formed expression.
    NotAnExpression(crate::syntax::NotAnExpression),
}

/// Run one transformer expression over one region, in the phase environment.
///
/// A test helper, and the one place a bare `expand` expression is still wrapped
/// into a module for the phase to read: a law about the fold or about a builder
/// is about that expression, and making each such test write a whole `library`
/// around it would bury the law in ceremony. Everything else — the compiler's
/// own path, and every test about an adapter *module* — hands the phase a
/// module.
///
/// `region` is source text, read by the fixed reader Musa already has: this
/// driver does not extend the lexer or the grouper.
#[cfg(test)]
pub(crate) fn expand_region(
    transformer: &str,
    region: &str,
    expansion: crate::syntax::ExpansionPath,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let subject = crate::syntax::read_region(&musa_language::parse(region).syntax(), expansion);
    expand_syntax(
        &format!("library {{\n    let level = \"readable\";\n\n    let expand = {transformer};\n}}\n"),
        PhaseImports::bundled(),
        &subject,
    )
    .0
}

/// The refusal an `Err(Both(node, message))` carries.
///
/// The span comes from the node the adapter handed back, never from anything
/// the adapter computed: `SourceInfo` has no eliminator, so an adapter can
/// point at a node it holds and cannot say where a node is.
pub(crate) fn refusal_of(held: &musa_calculus::Datum) -> Option<ExpansionFailure> {
    let musa_calculus::Datum::Case {
        ref constructor,
        ref fields,
    } = *held
    else {
        return None;
    };
    if &**constructor != "Pair.Both" {
        return None;
    }
    let [musa_calculus::Datum::Lit(ref node), ref message] = fields[..] else {
        return None;
    };
    let node = crate::registry::held::<crate::syntax::Syntax>(node)?;
    let at = match node.info() {
        crate::syntax::SourceInfo::Original { span, .. } => Some(*span),
        crate::syntax::SourceInfo::Generated(_) => None,
    };
    Some(ExpansionFailure::Refused {
        message: said(message)?,
        at,
    })
}
