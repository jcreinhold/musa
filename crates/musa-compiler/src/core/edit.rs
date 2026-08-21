//! One concern of the enclosing module; see its module docs.

use musa_calculus::Raw;

use musa_score::diagnose::Diagnostic;

use super::ModuleFault;
use super::PhaseImports;
use super::Unrun;
use super::{not_the_operation, read_adapter_module};

/// What one expansion charged the phases it used.
///
/// Two of `CompilerLimits`' four counters (`26-language-design-decision.md`
/// §3.5); the other two are the phase's own and are counted by
/// [`crate::expand`]. Read off [`musa_calculus::Spend`], because a transformer is
/// elaborated and normalized by the ordinary machinery and a separate accounting
/// of the same work would be a second opinion about it.
pub(crate) struct PhaseWork {
    pub(crate) evaluation_steps: u64,
}

impl PhaseWork {
    /// §3.5's checking counter, from what the core charged.
    ///
    /// `evaluation_steps` is the core's reduction count, which is the same
    /// quantity under the same name — and the whole of the checking half's
    /// cost: the flat dictionary law postpones no constraint and retries none,
    /// so there is no second quantity to carry. The phase's four counters are
    /// three, and §3.5's determinism-and-growth requirement is met by the one.
    ///
    /// The cost of reading the adapter *module* is in here too, added by
    /// [`AdapterModule::spend`] before the run. It has to be: an adapter whose
    /// operations carry their own signatures is checked without raising a
    /// single metavariable, so a phase that counted only the run's own
    /// elaboration would report zero for the checking half and charge nothing
    /// for a module of any size. The number is the core's own rather than an
    /// estimate on this side, which is what the paragraph above requires.
    pub(crate) const fn of(spent: musa_calculus::Spend) -> Self {
        Self {
            evaluation_steps: spent.steps,
        }
    }
}

/// Why an adapter's `edit` produced no patch.
///
/// The same three-way distinction [`ExpansionFailure`] makes, for the same
/// reasons, minus the two cases an editor cannot reach: an editor answers with
/// replacements rather than with syntax, so there is nothing for the expression
/// gate to reject. It is its own type rather than a reuse because the sentences
/// a reader gets differ — "this is not a transformer" is not what to tell
/// somebody whose `edit` is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditFailure {
    /// The adapter read the command and would not serve it.
    ///
    /// The package's own sentence, exactly as a refusal during expansion is.
    /// It carries no node: a command an adapter does not know is not about a
    /// place in the region, and pointing at one would be inventing a subject.
    Refused(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `edit` did not check as an editor.
    NotAnEditor(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// One replacement an adapter asked for: the anchor of the node to replace, and
/// the text to put there.
///
/// An anchor and not a range. An adapter has no operation for reading a source
/// range and gains none here — it names a node it was given, in the one
/// vocabulary it and an editor share (prompt 127dcc), and the compiler owns the
/// translation to bytes. That makes §4's locality a fact about the type rather
/// than a property the phase has to hope for and check afterwards.
pub(crate) type AdapterPatch = (u64, String);

/// Run one adapter's `edit` over one already-read region.
///
/// The second of `26-language-design-decision.md` §4's declared operations, in
/// the same phase environment [`expand_syntax`] runs in and by the same
/// machinery: one ordinary expression, checked by Algorithm W against a wanted
/// type, evaluated by the total evaluator, metered by the ordinary meter. The
/// command is spelled as its name, the anchor it is about, and one text
/// argument, because the phase is type-blind and may not learn a package's
/// command type.
pub(crate) fn edit_syntax(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
) -> (Result<Vec<AdapterPatch>, EditFailure>, PhaseWork) {
    let mut spent = musa_calculus::Spend::default();
    let answer = run_editor(adapter_source, imports, subject, command, anchor, argument, &mut spent);
    (answer, PhaseWork::of(spent))
}

fn run_editor(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
    spent: &mut musa_calculus::Spend,
) -> Result<Vec<AdapterPatch>, EditFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => EditFailure::Stopped,
        ModuleFault::Broken(diagnostics) => EditFailure::NotAnEditor(diagnostics),
    })?;
    *spent = spent.and(module.spend());
    // `edit(region, command, anchor, argument)`, written as a term and read in
    // the module's own context. The anchor arrives as a number rather than
    // inside the argument text because a command spelled as one string would be
    // one an adapter could not take apart.
    let here = musa_calculus::Origin::UNKNOWN;
    let (answer, spend) = module
        .run(
            "edit",
            vec![
                region(subject),
                text(command),
                Raw::numeral(here, "Nat", anchor),
                text(argument),
            ],
        )
        .map_err(|unrun| match unrun {
            Unrun::Undeclared => EditFailure::NotAnEditor(vec![not_the_operation("edit")]),
            Unrun::Stopped => EditFailure::Stopped,
            Unrun::Refused(diagnostics) => EditFailure::NotAnEditor(diagnostics),
            Unrun::NoAnswer => EditFailure::NoAnswer,
        })?;
    *spent = spent.and(spend);
    patches(&answer).ok_or(EditFailure::NoAnswer)?
}

/// The `Result<List<Pair<Nat, Text>>, Text>` an editor answered.
///
/// [`None`] when the normal form is not one, which is this crate's defect rather
/// than an adapter's for [`answered`]'s reason: the call was checked before it
/// was normalized, so an editor cannot reach it by being wrong.
fn patches(answer: &musa_calculus::Datum) -> Option<Result<Vec<AdapterPatch>, EditFailure>> {
    let musa_calculus::Datum::Case {
        ref constructor,
        ref fields,
    } = *answer
    else {
        return None;
    };
    let [ref held] = fields[..] else { return None };
    match &**constructor {
        "Result.Ok" => Some(Ok(listed(held, patch)?)),
        "Result.Err" => Some(Err(EditFailure::Refused(said(held)?))),
        _ => None,
    }
}

/// One `Pair<Nat, Text>` an editor asked for.
fn patch(field: &musa_calculus::Datum) -> Option<AdapterPatch> {
    let musa_calculus::Datum::Case {
        ref constructor,
        ref fields,
    } = *field
    else {
        return None;
    };
    if &**constructor != "Pair.Both" {
        return None;
    }
    let [musa_calculus::Datum::Count { count, .. }, ref text] = fields[..] else {
        return None;
    };
    Some((count, said(text)?))
}

/// The elements of a `List`, each read by `each`.
///
/// The prelude spelling and not a builtin one: `01-surface.md`'s `List` is
/// `Empty`/`Cons`, so a closed list is that chain and reading it is walking it.
/// [`None`] the moment an element is not what `each` expects, which keeps the
/// whole answer one decision rather than a vector with a hole in it.
fn listed<T>(held: &musa_calculus::Datum, each: impl Fn(&musa_calculus::Datum) -> Option<T>) -> Option<Vec<T>> {
    let mut read = Vec::new();
    let mut rest = held;
    loop {
        let musa_calculus::Datum::Case {
            ref constructor,
            ref fields,
        } = *rest
        else {
            return None;
        };
        match (&**constructor, &fields[..]) {
            ("List.Empty", []) => return Some(read),
            ("List.Cons", [head, tail]) => {
                read.push(each(head)?);
                rest = tail;
            }
            _ => return None,
        }
    }
}

/// The text a literal holds.
pub(crate) fn said(held: &musa_calculus::Datum) -> Option<String> {
    let musa_calculus::Datum::Lit(ref value) = *held else {
        return None;
    };
    crate::registry::held::<String>(value).cloned()
}

/// One region, as the term the phase hands an operation.
pub(crate) fn region(subject: crate::syntax::Syntax) -> Raw {
    Raw::lit(
        musa_calculus::Origin::UNKNOWN,
        crate::registry::literal(crate::registry::syntax_type(crate::syntax::Cat::TokenTree), subject),
    )
}

/// One `Text` argument, the same way.
fn text(said: &str) -> Raw {
    Raw::lit(
        musa_calculus::Origin::UNKNOWN,
        crate::registry::literal(crate::registry::plain_type("Text"), said.to_owned()),
    )
}
