//! Pattern-fragment unification: when an unknown has exactly one answer, what
//! that answer is, and what to say when it does not have one *yet*.
//!
//! `docs/rules/language/02-core-calculus.md` §2.1 fixes both halves of this
//! module and neither is negotiable.
//!
//! **Solving is Miller's pattern fragment, and only that.** `?m x₀ … x_{n-1} ≟
//! t` has a unique solution when the `xᵢ` are distinct local variables and `t`
//! mentions no variable outside them and does not mention `?m`. Inside the
//! fragment, accepting the solution is not a guess — it is the only one — which
//! is why this is the one place in the crate that writes an answer nobody
//! wrote. Outside it, nothing is guessed and nothing is refused on the spot.
//!
//! **The third answer is the one that makes elaboration order-independent.** A
//! comparison ends *solved*, *not unifiable*, or **blocked** — and collapsing
//! the third into the second is the classic bug, because "I cannot decide this
//! with what I know now" and "these are two different types" are not the same
//! fact. [`Outcome`] is that distinction, and [`Queue`] is where a blocked one
//! waits.
//!
//! # What is here and what is next door
//!
//! Everything in this module answers a [`CoreError`], because everything in it
//! is a fact about two values with no author to address — prompt 150's test,
//! applied. The *diagnostics* are next door in
//! [`crate::elaboration::convert`], which owns the type-directed walk, the
//! path a mismatch is reported under, and the span. This module owns the
//! fragment test, the two checks that make an assignment safe, the abstraction
//! that turns a value into a closed solution, and the queue.
//!
//! # Why the abstraction is a quotation
//!
//! A solution is a *closed* λ-chain (`meta.rs` says why). Building one means
//! turning a value that mentions `x₀ … x_{n-1}` into a term that binds them,
//! and that is exactly what [`quote`] at depth `n` already does. So there is no
//! abstraction function here, no shift, and no substitution — the read-back
//! this crate already owns does the work, and its refusal to name a variable
//! it has no binder for *is* §2.1's scope check. Nothing checks twice at
//! solving time because there is nothing a second walk would catch.
//!
//! [`crate::kernel::recheck`] does check twice, and deliberately: verification
//! that trusts the pass it is verifying verifies nothing.

use std::sync::Arc;

use crate::kernel::budget::Meter;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::eval::{apply, apply_closure, eval, force, opened};
use crate::kernel::meta::Meta;
use crate::kernel::origin::Origin;
use crate::kernel::quote::{At, Mode, quote};
use crate::kernel::term::{Binder, Level, Shape, Term};
use crate::kernel::value::{DefHead, Elim, Env, Form, Head, Neutral, Value};

/// What asking an unknown to take a value answered.
///
/// §2.1's three outcomes, minus the one that is an `Err`: *not unifiable* is a
/// [`CoreError`] and travels the ordinary way, because a caller that has to
/// remember to look at a fourth variant is a caller that will forget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The unknown took the value. Inside the pattern fragment, so unique.
    Solved,
    /// Neither side is a flexible pattern. The comparison belongs to the
    /// caller's structural descent, which is where it goes.
    Rigid,
    /// Flexible, and not decidable yet: an unknown under a spine that is not a
    /// pattern, or a candidate solution still blocked on another unknown.
    /// [`Queue::postpone`] is where this answer is kept.
    Blocked,
}

/// An occurrence of an unknown that §2.1's fragment admits: the unknown, and
/// the distinct variables it is applied to.
///
/// The whole point of the type is that it cannot be built by mistake —
/// [`fragment`] is the only constructor and it applies the test.
pub(crate) struct Fragment {
    /// The unknown at the head.
    pub(crate) meta: Meta,
    /// The variables its spine applies, outermost first.
    levels: Vec<Level>,
}

impl Fragment {
    /// Whether the spine is the identity on the unknown's own scope.
    ///
    /// The shape every occurrence this crate builds already has: a meta of
    /// arity `n` applied to `x₀ … x_{n-1}` in order. It matters because it is
    /// the shape whose inverse is nothing at all, so the solution is the
    /// read-back of the value and no renaming stands between them.
    ///
    /// A permuted or partial spine is inside §2.1's fragment and is *not*
    /// solved here: inverting it needs a read-back that renames as it goes, and
    /// this crate's [`quote`] does not, so the honest answer is the one §2.1
    /// gives outside the fragment — postpone, and let the declaration's end
    /// report it if nothing else settles it. No occurrence the elaborator
    /// builds has that shape, so what this costs is a completeness the language
    /// has never been able to exercise.
    fn is_identity(&self, meta_arity: u32) -> bool {
        self.levels.len() == meta_arity as usize
            && self
                .levels
                .iter()
                .enumerate()
                .all(|(position, level)| u32::try_from(position).is_ok_and(|position| level.0 == position))
    }
}

/// The unsolved unknown at `value`'s head, if there is one.
///
/// The flexibility test on its own, apart from the fragment test: a value with
/// one of these at its head is a value a later solution can change, which is
/// what separates "wait" from "these disagree".
pub(crate) fn flexible_head(value: &Value) -> Option<Meta> {
    let Form::Neutral(neutral) = &value.form else {
        return None;
    };
    let Head::Meta(meta) = &neutral.head else {
        return None;
    };
    meta.solution().is_none().then(|| meta.clone())
}

/// The [`Fragment`] `value` is, if it is one.
///
/// `None` for anything that is not an unsolved unknown applied to distinct bare
/// variables — a rigid head, a solved meta (which should have been forced
/// first), a projection in the spine, an argument that is not a variable, or a
/// variable applied twice. Each of those is §2.1's "outside the fragment", and
/// the caller's answer to all of them is the same.
pub(crate) fn fragment(value: &Value) -> Option<Fragment> {
    let Form::Neutral(neutral) = &value.form else {
        return None;
    };
    let Head::Meta(meta) = &neutral.head else {
        return None;
    };
    if meta.solution().is_some() {
        return None;
    }
    let mut levels = Vec::with_capacity(neutral.spine.len());
    for elimination in &neutral.spine {
        let Elim::App { argument, .. } = elimination else {
            return None;
        };
        let Form::Neutral(applied) = &argument.form else {
            return None;
        };
        // A `let` binder is a local too. Its environment entry is the
        // definition folded rather than a variable (`Cx::defined`), and reading
        // that as "not a variable" would put every occurrence created under a
        // `let` outside the fragment — which is most of them.
        let level = match &applied.head {
            Head::Var(level, _) => *level,
            Head::Def(DefHead::Local(level), _, _) => *level,
            Head::Meta(_) | Head::Const(..) | Head::Base(..) | Head::Builtin(..) | Head::Def(..) => return None,
        };
        if !applied.spine.is_empty() || levels.contains(&level) {
            return None;
        }
        levels.push(level);
    }
    Some(Fragment {
        meta: meta.clone(),
        levels,
    })
}

/// Make `one` stand for `other`, when §2.1 says it uniquely can.
///
/// The whole flexible case, in the order the reasons apply: the reflexive pair
/// first, because `?m ≟ ?m` is already true and neither check below should be
/// asked about it; then the occurs check, which is a refusal and not a
/// postponement — an unknown occurring in its own answer has no solution at any
/// later moment either; then the fragment test; then the read-back, whose
/// failure to name a variable is the scope check and whose answer is the
/// solution.
///
/// A blocked candidate — the solution still mentions an unsolved unknown — is
/// *still assigned*. §2.1's uniqueness argument does not depend on the
/// right-hand side being closed, only on the spine being a pattern, and
/// refusing to write it here would strand the flex-flex chains that
/// `unification_laws.rs` already fixes as the answer.
///
/// # Errors
///
/// [`Malformed::AlreadySolved`] when a second write is attempted, which is a
/// defect in the caller rather than in the program, and whatever the read-back
/// spends.
pub(crate) fn assign(meter: &mut Meter, one: &Value, other: &Value) -> Result<Outcome, CoreError> {
    let Some(pattern) = fragment(one) else {
        // Not a pattern — but the reason matters. An unsolved unknown at the
        // head under a spine the fragment does not admit is §2.1's "outside the
        // fragment", and the answer there is to wait: the spine's arguments may
        // still reduce to variables, or the other side may be solved into
        // something that settles it. A rigid head is a different fact, and
        // sending it to the structural descent is what the descent is for.
        return Ok(match flexible_head(one) {
            Some(_) => Outcome::Blocked,
            None => Outcome::Rigid,
        });
    };
    if let Some(reflexive) = fragment(other)
        && reflexive.meta == pattern.meta
        && reflexive.levels == pattern.levels
    {
        return Ok(Outcome::Solved);
    }
    // The same unknown on both sides under *different* scopes — `?m σ ≟ ?m σ'`
    // — is not a cycle and not a disagreement. Solving it needs the
    // intersection of the two spines, which is outside §2.1's fragment, so the
    // answer there is the fragment's answer: wait, and let the declaration's
    // end call it undetermined if nothing settles it.
    if flexible_head(other).as_ref() == Some(&pattern.meta) {
        return Ok(Outcome::Blocked);
    }
    if !pattern.is_identity(pattern.meta.arity()) {
        return Ok(Outcome::Blocked);
    }
    let (_, goal) = scope_of(meter, &pattern.meta)?;
    let depth = Level(pattern.meta.arity());
    // `Mode::Keep`. `quote`'s header says solutions open, and that sentence was
    // written when a solution was a small implicit type argument stored as a
    // value with no read-back at all. A telescoped unknown is read back, and
    // reading back *opened* normalizes the whole of whatever the argument
    // mentions: elaborating `examples/staff-page.musa` this way spends more
    // than 10⁸ reduction steps inside one `assign`, against 200,000 for the
    // entire declaration when the definitions stay folded. Nothing is lost by
    // keeping them — a folded local is written as its variable and is
    // escape-checked by `Reading::index` exactly as an ordinary variable is,
    // and a folded global is written as the definition itself rather than as a
    // name some later scope has to resolve.
    let Ok(body) = quote(meter, depth, Mode::Keep, &goal, other) else {
        // The one failure this arm swallows is the scope check, and swallowing
        // it into `Blocked` rather than into a refusal is deliberate: the value
        // may still reduce, or the variable it names may belong to a spine
        // another constraint is about to solve away. The declaration's end is
        // where an unknown nothing determined becomes an error.
        return Ok(Outcome::Blocked);
    };
    // The occurs check, on the *term* and not on the value it was read back
    // from. A value carries the environments its closures were built in, and
    // those environments hold every binder that happened to be in scope —
    // including, routinely, other occurrences of the unknown being solved. An
    // occurs check over them answers "yes" for values that do not mention the
    // unknown at all. Quotation has already forced every closure the value
    // actually reaches, so the term is exactly what the solution would be.
    if occurs(&body, &pattern.meta) {
        return Err(Malformed::Cyclic(pattern.meta.id()).into());
    }
    let solution = abstracted(meter, &pattern.meta, body)?;
    pattern.meta.solve(solution)?;
    Ok(Outcome::Solved)
}

/// `λ x₀ … x_{n-1}. body`, evaluated closed — the shape `meta.rs` requires of
/// every solution.
///
/// The binder names come from the telescope so that a solution read back for a
/// diagnostic spells the variables the way the type does.
///
/// `body` must be read at depth `meta.arity()` — under the telescope and
/// nothing else. Every writer of a solution goes through here, because
/// `kernel::meta` requires the stored value to be a closed λ-chain and a value
/// stored at the occurrence's own depth would be applied to the scope it
/// already mentions.
///
/// # Errors
///
/// [`Malformed::LevelArity`] when the telescope is shorter than the arity, and
/// whatever the evaluation spends.
pub(crate) fn abstracted(meter: &mut Meter, meta: &Meta, body: Term) -> Result<Value, CoreError> {
    let mut names = Vec::new();
    let mut ty = meta.ty().clone();
    for _ in 0..meta.arity() {
        let opened_ty = opened(meter, &ty)?;
        let current = opened_ty.as_ref().unwrap_or(&ty);
        let Form::Pi {
            name, domain, codomain, ..
        } = &current.form
        else {
            return Err(Malformed::LevelArity(name_of(meta)).into());
        };
        names.push(Arc::clone(name));
        let level = Level(u32::try_from(names.len().saturating_sub(1)).unwrap_or(0));
        let variable = Value::var(current.origin, level, Arc::clone(domain));
        ty = apply_closure(meter, codomain, variable)?;
    }
    let mut term = body;
    for name in names.into_iter().rev() {
        term = Term::lam(meta.origin(), name, term);
    }
    eval(meter, &Env::under(meta.globals().clone()), &term)
}

/// The unknown's scope opened: the variables `x₀ … x_{n-1}` its occurrences
/// apply, and the goal type its solution's body stands at.
///
/// One walk down the telescope, and the only reader of the shape
/// [`Meta::new`]'s caller promised. A telescope shorter than the arity is that
/// promise broken, so it answers [`Malformed`] rather than assuming.
///
/// # Errors
///
/// [`Malformed::MetaTelescope`] when the type does not have `arity` Π binders,
/// and whatever opening the telescope spends.
pub(crate) fn scope_of(meter: &mut Meter, meta: &Meta) -> Result<(Vec<Value>, Value), CoreError> {
    let mut variables = Vec::with_capacity(meta.arity() as usize);
    let mut ty = meta.ty().clone();
    for position in 0..meta.arity() {
        let opened_ty = opened(meter, &ty)?;
        let current = opened_ty.as_ref().unwrap_or(&ty);
        let Form::Pi { domain, codomain, .. } = &current.form else {
            return Err(Malformed::MetaTelescope(meta.id()).into());
        };
        let variable = Value::var(current.origin, Level(position), Arc::clone(domain));
        variables.push(variable.clone());
        ty = apply_closure(meter, codomain, variable)?;
    }
    Ok((variables, ty))
}

/// An occurrence of `meta`: the unknown applied to the whole of its own scope,
/// as a term and as a value.
///
/// One function for both because they must agree — a value whose spine differs
/// from what the term evaluates to is something other than what the elaborator
/// reasoned about, and that disagreement is invisible until much later.
///
/// The term is [`Term::meta`] and nothing else: the scope is not written
/// (`kernel::meta`), it is read out of the environment where the term is
/// evaluated. `arguments` is what that environment holds, outermost first, and
/// it is a parameter rather than something rebuilt from the telescope because a
/// `let` binder stands for the definition folded and not for a variable — a
/// spine of fresh variables would be a value the term does not evaluate to.
///
/// # Errors
///
/// [`Malformed::MetaTelescope`] when `arguments` is not as long as the arity.
pub(crate) fn occurrence(meta: &Meta, here: Origin, arguments: &[Value]) -> Result<(Term, Value), CoreError> {
    if arguments.len() != meta.arity() as usize {
        return Err(Malformed::MetaTelescope(meta.id()).into());
    }
    let value = Value::neutral(Neutral {
        origin: here,
        head: Head::Meta(meta.clone()),
        spine: arguments
            .iter()
            .map(|argument| Elim::App {
                origin: here,
                argument: Arc::new(argument.clone()),
            })
            .collect(),
    });
    Ok((Term::meta(here, meta.clone()), value))
}

/// `meta`'s solution with its own scope put back: the body the occurrence
/// stands for, at the depth the occurrence sits at.
///
/// What [`crate::elaboration::elab::Elaborator::zonk`] writes into the stored
/// term. Applying the closed λ-chain to the very variables it abstracted is
/// β-reduction back to where it started, which is why the answer has no
/// redex in it and why the stored term reads as though the unknown had been
/// filled in place.
///
/// `None` when the unknown is unsolved.
///
/// # Errors
///
/// As [`scope_of`], plus whatever the applications spend.
pub(crate) fn opened_solution(meter: &mut Meter, meta: &Meta) -> Result<Option<(Value, Value)>, CoreError> {
    let Some(solution) = meta.solution().cloned() else {
        return Ok(None);
    };
    let (variables, goal) = scope_of(meter, meta)?;
    let mut body = solution;
    for variable in variables {
        body = apply(meter, meta.origin(), body, variable)?;
    }
    Ok(Some((body, goal)))
}

/// Whether `target` occurs in `term`, following solutions on the way.
///
/// §2.1's occurs check: an unknown may not occur in its own answer. Refusing
/// rather than postponing is the right answer for it — a cycle is not a fact
/// that a later solution could change.
///
/// Asked of the read-back rather than of the value it came from, and
/// [`assign`] says why: a value carries whole environments, and an occurs check
/// over an environment answers about binders the value never looks at.
pub(crate) fn occurs(term: &Term, target: &Meta) -> bool {
    match term.shape() {
        Shape::Meta(meta) => {
            meta == target
                || meta
                    .solution()
                    .is_some_and(|solution| walk(solution, &mut |inner| inner == target))
        }
        Shape::Var(_) | Shape::Lit(_) | Shape::Universe(_) | Shape::Named { .. } => false,
        Shape::Bind { binder, body, .. } => {
            let in_binder = match binder {
                Binder::Lam => false,
                Binder::Pi { ty, .. } => occurs(ty, target),
                Binder::Let { ty, value } => occurs(ty, target) || occurs(value, target),
            };
            in_binder || occurs(body, target)
        }
        Shape::App { function, argument } => occurs(function, target) || occurs(argument, target),
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().any(|field| occurs(&field.term, target)),
        Shape::Project { record, .. } => occurs(record, target),
    }
}

/// Whether `value` mentions any unsolved unknown.
///
/// The instantiation walk's reading of §2.1's "inferable": a slot typed
/// `Row(?n)` has not been determined by the call, so the argument standing in
/// it is inferred and the match learns the parameter from it rather than the
/// other way round. Over-approximating is the safe direction here — it defers
/// an argument that need not have been — which is why this one asks the value
/// rather than a read-back the caller does not have.
pub(crate) fn mentions_unsolved(value: &Value) -> bool {
    walk(value, &mut |meta| meta.solution().is_none())
}

/// Every unknown reachable from `value`, tested until one says yes.
///
/// Two callers: [`mentions_unsolved`] asks it of a domain the elaborator is
/// deciding what to do with, and [`occurs`] asks it of a *solution*, where the
/// value form is the only form there is. Where it looks is the part that
/// matters:
///
/// - **Through solutions.** `?α := ?β` stores a value that says "blocked on
///   `?β`", so a walk that stopped at the head would miss `?α` sitting inside
///   `?β`'s answer.
/// - **Into environments.** A closure and a telescope carry a term plus the
///   values its free variables stand for; the unknowns a comparison can reach
///   are in those values, and a body that mentions one mentions it through
///   them. That over-approximates — an environment holds binders the body never
///   looks at — which is why [`assign`] asks [`occurs`] of the read-back
///   rather than asking this of the value, and why over-approximating is
///   tolerable for [`mentions_unsolved`], whose wrong answer only defers an
///   argument that need not have been.
fn walk(value: &Value, seen: &mut impl FnMut(&Meta) -> bool) -> bool {
    match &value.form {
        Form::Universe(_) | Form::Lit(_) | Form::Numeral(_) => false,
        Form::Pi { domain, codomain, .. } => walk(domain, seen) || codomain.env.iter().any(|item| walk(item, seen)),
        Form::Lam(closure) => closure.env.iter().any(|item| walk(item, seen)),
        Form::RecordType(telescope) => telescope.env.iter().any(|item| walk(item, seen)),
        Form::Record(fields) => fields.iter().any(|(_, item)| walk(item, seen)),
        Form::Neutral(neutral) => {
            let head = match &neutral.head {
                Head::Meta(meta) => seen(meta) || meta.solution().is_some_and(|solution| walk(solution, seen)),
                Head::Var(_, ty) => walk(ty, seen),
                Head::Const(..) | Head::Base(..) | Head::Builtin(..) => false,
                // A tree body holds *terms*, which are zonked before they are
                // stored, so there is no value behind one for an occurrence to
                // hide in.
                Head::Def(_, ty, crate::kernel::value::Folding::Value(folded)) => walk(ty, seen) || walk(folded, seen),
                Head::Def(_, ty, _) => walk(ty, seen),
            };
            head || neutral.spine.iter().any(|elimination| match elimination {
                Elim::App { argument, .. } => walk(argument, seen),
                Elim::Project { .. } => false,
            })
        }
    }
}

/// The name a malformed telescope is reported under.
///
/// A meta has no name, so the report borrows the only spelling it has. The
/// variant exists for a defect nobody should be able to reach; what it must not
/// do is be silent.
fn name_of(meta: &Meta) -> crate::kernel::term::Name {
    Arc::from(format!("?{}", meta.id()).as_str())
}

/// A comparison that could not be decided yet, kept until something changes.
///
/// It stores *values* and not a resumed walk, because a retry is the same
/// comparison run again from the top: the values are shared, so a meta solved
/// since is already solved in them, and re-descending costs the meter what the
/// retry is worth (§4's conversion work). A suspended continuation would be a
/// second representation of the same comparison, free to disagree with the
/// first.
pub(crate) struct Postponed {
    /// Where the comparison was asked, for the refusal if it never settles.
    pub(crate) origin: Origin,
    /// The binder depth the two values were compared at.
    pub(crate) depth: Level,
    /// What they were compared at: `None` when both sides are types.
    pub(crate) goal: Option<Value>,
    pub(crate) left: Value,
    pub(crate) right: Value,
}

impl Postponed {
    /// The sort this comparison is read back in, for the retry and for the
    /// report.
    pub(crate) fn at(&self) -> At<'_> {
        match &self.goal {
            Some(ty) => At::Term(ty),
            None => At::Type,
        }
    }

    /// The first unsolved unknown this comparison is waiting on, if it is
    /// waiting on one.
    ///
    /// Left rather than right, and the first rather than all of them, because
    /// the report names one place to look and a list of unknowns is not a place.
    pub(crate) fn blocked_on(&self, meter: &mut Meter) -> Result<Option<Meta>, CoreError> {
        for side in [&self.left, &self.right] {
            let forced = force(meter, side)?;
            let side = forced.as_ref().unwrap_or(side);
            if let Form::Neutral(neutral) = &side.form
                && let Head::Meta(meta) = &neutral.head
                && meta.solution().is_none()
            {
                return Ok(Some(meta.clone()));
            }
        }
        Ok(None)
    }
}

/// The comparisons waiting for something to change.
///
/// §2.1 makes the queue part of the judgment rather than an implementation
/// note, so it is a named thing with a named lifecycle: entries go in when a
/// comparison answers [`Outcome::Blocked`], come out in one batch when the
/// driver retries, and whatever is still here when the declaration ends is the
/// error §2.1 requires.
#[derive(Default)]
pub(crate) struct Queue(Vec<Postponed>);

impl Queue {
    /// Keep this comparison for later.
    pub(crate) fn postpone(&mut self, entry: Postponed) {
        self.0.push(entry);
    }

    /// How many comparisons are waiting.
    ///
    /// Read by the driver as its progress measure: a round that left the queue
    /// no shorter than it found it solved nothing, so there is nothing new for
    /// another round to tell any entry.
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Everything waiting, taken for one round of retries.
    ///
    /// Taken rather than borrowed because a retry may postpone again, and an
    /// entry that re-queues itself while the round is walking the same vector
    /// is a round that never ends.
    pub(crate) fn take(&mut self) -> Vec<Postponed> {
        core::mem::take(&mut self.0)
    }
}

#[cfg(test)]
// A malformation that is not reached, or a reflexive pair the kernel refuses,
// is a defect in this module rather than a program error: panicking is the
// correct behaviour there.
#[allow(clippy::panic)]
#[allow(clippy::expect_used)]
mod tests {
    //! The three malformations this module and [`crate::kernel::recheck`] can
    //! answer with, reached.
    //!
    //! In-crate rather than in `tests/suite/`, and `malformed_laws.rs` says why
    //! it has to be: no public constructor builds a [`Term`] holding a
    //! metavariable, so a term that reaches any of these cannot be assembled
    //! from outside. These are the terms nobody should be able to build, built
    //! by the one caller that can.

    use super::{Level, Meta, Origin, Outcome, Term, assign};
    use crate::kernel::context::Cx;
    use crate::kernel::error::{CoreError, Malformed};
    use crate::kernel::sort::Sort;
    use crate::kernel::term::Index;
    use crate::kernel::value::{Closure, Env, Form, Value};
    use std::sync::Arc;

    const HERE: Origin = Origin::node(910);

    /// `{}` as a value, standing at `Type 0`.
    fn unit_type() -> Value {
        Value::new(
            HERE,
            Form::RecordType(crate::kernel::value::Telescope {
                fields: Arc::from([]),
                env: Env::EMPTY,
            }),
        )
    }

    /// `(x : {}) → Type 0` as a value: the telescope of an arity-one unknown
    /// standing for a type. Its goal is a universe rather than `{}` so that a
    /// read-back of its solution is *structural* — at a record type quotation
    /// η-expands, and a solution that names a variable would be replaced by
    /// `{}` before anything could notice the name.
    fn one_binder() -> Value {
        Value::new(
            HERE,
            Form::Pi {
                filling: crate::kernel::term::Filling::Written,
                name: Arc::from("x"),
                domain: Arc::new(unit_type()),
                codomain: Closure {
                    env: Env::EMPTY,
                    body: Term::universe(HERE, Sort::ZERO),
                },
            },
        )
    }

    /// The malformation `outcome` carries, or a panic naming what arrived.
    fn malformed<T>(name: &str, outcome: Result<T, CoreError>) -> Malformed {
        match outcome {
            Ok(_) => panic!("{name}: the kernel accepted what nobody should have built"),
            Err(CoreError::Malformed(fault)) => fault,
            Err(other) => panic!("{name}: reached `{other}` rather than a malformation"),
        }
    }

    /// §2.1's occurs check: an unknown may not occur in its own answer, and the
    /// answer is a refusal rather than a postponement because no later solution
    /// changes it.
    #[test]
    fn an_unknown_that_would_occur_in_its_own_answer_is_refused() {
        let cx = Cx::new();
        let mut meter = cx.meter();
        // Its goal is `Type 0`, so what it stands for is a *type* and the
        // read-back below is structural rather than η-expanding at a record.
        let meta = Meta::new(
            0,
            HERE,
            Value::new(HERE, Form::Universe(Sort::ZERO)),
            0,
            cx.globals().clone(),
        );
        let occurrence = Value::neutral(crate::kernel::value::Neutral::head(
            HERE,
            crate::kernel::value::Head::Meta(meta.clone()),
        ));
        // `?0 ≟ (x : ?0) → {}` — a pattern spine on the left, and the right
        // names the very unknown being solved.
        let cyclic = Value::new(
            HERE,
            Form::Pi {
                filling: crate::kernel::term::Filling::Written,
                name: Arc::from("x"),
                domain: Arc::new(occurrence.clone()),
                codomain: Closure {
                    env: Env::EMPTY,
                    body: Term::record_type(HERE, []),
                },
            },
        );
        let fault = malformed("an unknown in its own answer", assign(&mut meter, &occurrence, &cyclic));
        assert!(matches!(fault, Malformed::Cyclic(0)), "{fault}");
        assert!(!meta.is_solved(), "a refused assignment must write nothing");
    }

    /// The same unknown on both sides under different scopes is a postponement
    /// and not a cycle: `?m σ ≟ ?m σ'` needs the intersection of the two
    /// spines, which is outside the fragment.
    #[test]
    fn the_same_unknown_on_both_sides_waits_rather_than_reporting_a_cycle() {
        let cx = Cx::new();
        let mut meter = cx.meter();
        let meta = Meta::new(1, HERE, unit_type(), 0, cx.globals().clone());
        let occurrence = Value::neutral(crate::kernel::value::Neutral::head(
            HERE,
            crate::kernel::value::Head::Meta(meta),
        ));
        let outcome = assign(&mut meter, &occurrence, &occurrence).expect("a reflexive pair is not a failure");
        assert!(matches!(outcome, Outcome::Solved), "a pair already equal is settled");
    }

    /// The re-checker's own scope check, made over a solution the unifier would
    /// never have written — which is the point of making it twice.
    #[test]
    fn a_stored_solution_that_names_a_variable_outside_its_scope_is_caught_again() {
        let cx = Cx::new();
        let meta = Meta::new(2, HERE, one_binder(), 1, cx.globals().clone());
        // `λ_. y`, where `y` is a variable at a level the unknown's scope does
        // not reach. Applying it to the scope's own binder leaves `y` standing.
        let escapee = Value::var(HERE, Level(7), Arc::new(Value::new(HERE, Form::Universe(Sort::ZERO))));
        meta.solve(Value::new(
            HERE,
            Form::Lam(Closure {
                env: Env::EMPTY.push(escapee),
                body: Term::var(HERE, Index(1)),
            }),
        ))
        .expect("a fresh unknown is unsolved");
        let under = cx.assumed(HERE, Arc::new(unit_type()));
        let goal = Value::new(HERE, Form::Universe(Sort::ZERO));
        let fault = crate::kernel::recheck::disagreement(&under, &goal, &Term::meta(HERE, meta))
            .expect("a solution naming a variable outside its scope is a disagreement");
        assert!(matches!(fault, Malformed::EscapedSolution(2)), "{fault}");
    }

    /// An occurrence standing where its unknown's scope does not reach: the
    /// invariant `meta.rs` states, broken, and reported rather than assumed.
    #[test]
    fn an_occurrence_outside_the_scope_its_unknown_claims_is_refused() {
        let cx = Cx::new();
        let meta = Meta::new(3, HERE, one_binder(), 1, cx.globals().clone());
        meta.solve(Value::new(
            HERE,
            Form::Lam(Closure {
                env: Env::EMPTY,
                body: Term::record_type(HERE, []),
            }),
        ))
        .expect("a fresh unknown is unsolved");
        // The context has no binders at all, and the unknown's scope claims one.
        let goal = Value::new(HERE, Form::Universe(Sort::ZERO));
        let fault = crate::kernel::recheck::disagreement(&cx, &goal, &Term::meta(HERE, meta))
            .expect("an occurrence with no scope to stand in is a disagreement");
        assert!(matches!(fault, Malformed::MetaTelescope(3)), "{fault}");
    }
}
