//! One instantiation walk: implicits filled, constraints noted, arguments placed.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::eval::{apply_closure, opened};
use crate::origin::Origin;
use crate::raw::Raw;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Filling, Shape, Term};
use crate::value::{Form, Neutral, Value};

use super::{Elaborator, Typed};

impl Elaborator {
    /// §2's constructor rule in a checking position: `C a⃗ ⇐ N p⃗`.
    ///
    /// A bare case name resolves against the family the expected type names —
    /// the whole of what "a constructor is checked" means; [`case_named`]
    /// owns the two spellings and the one coincidence §1.3 allows. The
    /// family's parameters are no longer read off and applied here: they are
    /// the constructor's implicit binders, so the application pass matches
    /// them out of `ty` at the end, which is the same rule §2.1 states for
    /// every other call.
    /// `head` applied to one constructor parameter the expected type carried.
    ///
    /// The parameter is a *value*: it was read off the type, not written, so
    /// there is no [`Raw`] to elaborate and nothing to infer — the term is the
    /// value quoted back at the scope's depth. [`crate::quote::Mode::Keep`]
    /// because the type is the author's and any definition it folds stays
    /// folded in what they read back.
    pub(super) fn given(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        param: &Value,
    ) -> Result<Typed, ElabError> {
        let unfolded = crate::eval::opened(&mut self.meter, &head.ty)?;
        let function_ty = unfolded.as_ref().unwrap_or(&head.ty);
        let Form::Pi { domain, codomain, .. } = &function_ty.form else {
            return Err(Refusal::NotAFunction {
                at: here,
                ty: scope.quote_type(&mut self.meter, &head.ty)?,
            }
            .into());
        };
        let (domain, codomain) = (Arc::clone(domain), codomain.clone());
        let term = crate::quote::quote(&mut self.meter, scope.depth(), crate::quote::Mode::Keep, &domain, param)?;
        Ok(Typed {
            term: Term::app(here, head.term, term),
            ty: apply_closure(&mut self.meter, &codomain, param.clone())?,
        })
    }

    /// `head` applied to `count` fresh metas — a constructor's parameters when
    /// no expected type named the family. The fields solve them through §2.1's
    /// ordinary matching, and [`Elaborator::settled`] audits what they could
    /// not.
    pub(super) fn metas(
        &mut self,
        scope: &Scope,
        here: Origin,
        mut built: Typed,
        count: u32,
    ) -> Result<Typed, ElabError> {
        for _ in 0..count {
            let unfolded = crate::eval::opened(&mut self.meter, &built.ty)?;
            let function_ty = unfolded.as_ref().unwrap_or(&built.ty);
            let Form::Pi { domain, codomain, .. } = &function_ty.form else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &built.ty)?,
                }
                .into());
            };
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let meta = self.fresh_meta(here, &domain);
            let value = Value::neutral(Neutral::head(here, crate::value::Head::Meta(meta.clone())));
            built = Typed {
                term: Term::app(here, built.term, Term::meta(here, meta)),
                ty: apply_closure(&mut self.meter, &codomain, value)?,
            };
        }
        Ok(built)
    }

    /// §2.1's instantiation pass: apply `head` to the written arguments, then
    /// match what remains against `expected` when the call is in a checking
    /// position.
    ///
    /// One left-to-right walk over the spine, and the discipline is the
    /// document's: an implicit parameter becomes a [meta](crate::meta::Meta)
    /// that the first argument to mention it solves; a constraint waits until
    /// the walk has said everything matching can say, and is then resolved
    /// once, by lookup, never postponed. The domain and the argument's written
    /// form together decide what happens to it, in three cases rather than
    /// two:
    ///
    /// - **Settled** — the argument is *checked* against it. So is an argument
    ///   that annotates its own binder, because checking is what makes the
    ///   annotation and the domain agree ([`Raw::annotates_its_binder`]).
    /// - **Still quantified, and the argument can be inferred** — the argument
    ///   teaches the parameter, and what it inferred is matched against the
    ///   domain.
    /// - **Still quantified, and the argument is a checking-only form that
    ///   describes nothing** — the argument is *deferred*. Nothing can infer a
    ///   bare `λ`, and checking one against a domain that is still a meta would
    ///   bind its parameter to that meta; so a placeholder stands in the slot,
    ///   the rest of the spine is walked — which is what solves the domain —
    ///   and the argument is checked afterwards against the type it turned out
    ///   to have. A λ that annotates its own binder is not here: it says what
    ///   its parameter is, so it is one of the arguments the deferred ones are
    ///   waiting for rather than one of the ones waiting.
    ///
    /// Deferral is not postponement. Each argument is elaborated exactly once,
    /// there is no queue and no retry, and both the deferred set and the order
    /// they are revisited in are fixed by the written argument order, so the
    /// answer cannot depend on which branch ran first. A domain that is *still*
    /// unsolved when the walk ends is checked against anyway, which is what
    /// happened before this rule existed: the worst case is the old behaviour,
    /// and the author sees §2.1's [`Refusal::Unsolved`] at declaration end.
    ///
    /// The emitted term is unaffected: the arguments are elaborated in one
    /// order and the spine is built in the written one, so evaluation order is
    /// exactly what was written.
    pub(super) fn apply_spine(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        arguments: &[&Raw],
        expected: Option<&Value>,
    ) -> Result<Typed, ElabError> {
        // A *bare* constructor reference — no written fields — has nothing
        // for its family parameters to be learned from, so each becomes a
        // meta (§2.1): `None` is `None<?>` wherever it stands, and the slot it
        // is checked against solves the meta by ordinary first-order matching.
        // Without this a bare constructor at an undetermined slot would lend
        // the slot its Π-scheme, and the program that then drew a value from
        // the slot would meet a function type where its data was. A written
        // field does the same job later in the walk, so this is the
        // no-arguments case only.
        let head = match head.term.shape() {
            Shape::Const(constant)
                if arguments.is_empty() && matches!(constant.role, crate::family::Role::Constructor(_)) =>
            {
                let params = constant.group.params();
                self.metas(scope, here, head, params)?
            }
            Shape::Meta(_)
            | Shape::Var(_)
            | Shape::Const(_)
            | Shape::Def(_)
            | Shape::Base(_)
            | Shape::Lit(_)
            | Shape::Builtin(_)
            | Shape::Universe(_)
            | Shape::Bind { .. }
            | Shape::App { .. }
            | Shape::RecordType(_)
            | Shape::Record(_)
            | Shape::Project { .. }
            | Shape::Indexed { .. } => head,
        };
        let mut walk = Walk::default();
        let mut waiting: Vec<Waiting<'_>> = Vec::new();
        let mut ty = head.ty.clone();
        self.advance(scope, &mut ty, &mut walk)?;
        for argument in arguments {
            let unfolded = opened(&mut self.meter, &ty)?;
            let current = unfolded.as_ref().unwrap_or(&ty);
            let Form::Pi { domain, codomain, .. } = &current.form else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &ty)?,
                }
                .into());
            };
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let at = argument.origin();
            let (slot, value) = if !crate::convert::mentions_unsolved(&domain)
                || (argument.checks_only() && argument.annotates_its_binder())
            {
                // Checked: the argument is read against the domain, either
                // because the domain is settled or because the argument
                // annotates its own binder and checking is what makes the two
                // agree.
                let term = self.check(scope, argument, &domain)?;
                let value = scope.eval(&mut self.meter, &term)?;
                (Slot::Argument(term), value)
            } else if argument.checks_only() {
                // Deferred: a placeholder holds the slot so the rest of the
                // walk can proceed, and the argument is checked below against
                // whatever the rest of the walk made the domain be. The
                // placeholder is an ordinary meta, so a codomain that reads the
                // argument's *value* gets a value to read, and anything the
                // walk learns about it is a solution the second pass then
                // agrees with rather than overwrites.
                let meta = self.fresh_meta(at, &domain);
                let value = Value::neutral(Neutral::head(at, crate::value::Head::Meta(meta.clone())));
                waiting.push(Waiting {
                    argument,
                    domain: Arc::clone(&domain),
                    meta: meta.clone(),
                    at,
                });
                (Slot::Deferred(meta), value)
            } else {
                // Inferred — but an inferred head can still quantify over
                // parameters the domain determines (`identity` used unapplied):
                // the empty walk peels those into metas and does the matching,
                // which is §2.1's one rule rather than a second path here.
                let inferred = self.infer(scope, argument)?;
                let term = self
                    .apply_spine(scope, argument.origin(), inferred, &[], Some(&domain))?
                    .term;
                let value = scope.eval(&mut self.meter, &term)?;
                (Slot::Argument(term), value)
            };
            walk.slots.push(slot);
            ty = apply_closure(&mut self.meter, &codomain, value)?;
            self.advance(scope, &mut ty, &mut walk)?;
        }
        if let Some(expected) = expected {
            // Checking position: the rest of the type is matched against what
            // the position wants, which is where a bare constructor's family
            // parameters — and any argument's still-unsolved ones — are
            // learned. The position is the mismatch's *expected*: it is the
            // type the author wrote and the walked type the one found.
            //
            // Before the deferred arguments and not after, because this is the
            // last thing that can teach a domain anything: the expected type is
            // where `fold(combine, seed, xs)`'s result type comes from when the
            // seed is the lambda. Idris2 checks the rest of the spine including
            // its target for the same reason.
            self.conversion
                .unify_types(&mut self.meter, scope.depth(), here, expected, &ty)?;
        }
        // The second pass, in written order. `domain` is the same value the
        // walk skipped, and it needs no re-derivation: a meta is shared, so a
        // domain the walk solved is already solved here.
        let mut deferred = Vec::with_capacity(waiting.len());
        for Waiting {
            argument,
            domain,
            meta,
            at,
        } in waiting
        {
            let term = self.check(scope, argument, &domain)?;
            let value = scope.eval(&mut self.meter, &term)?;
            let stood = Value::neutral(Neutral::head(at, crate::value::Head::Meta(meta)));
            // Assignment when the placeholder is still free, conversion when
            // the walk already decided what stood there — one call, because
            // those are the same procedure (see [`crate::convert`]).
            self.conversion
                .unify(&mut self.meter, scope.depth(), at, &domain, &stood, &value)?;
            deferred.push(term);
        }
        Self::finish_walk(here, head.term, ty, &walk, deferred)
    }

    /// Skip the binders §2.1 fills rather than the author: an implicit
    /// parameter becomes a fresh meta, a constraint is noted for the walk's
    /// end.
    pub(super) fn advance(&mut self, scope: &Scope, ty: &mut Value, walk: &mut Walk) -> Result<(), ElabError> {
        let _ = scope;
        loop {
            let unfolded = opened(&mut self.meter, ty)?;
            let current = unfolded.as_ref().unwrap_or(ty);
            let Form::Pi {
                filling,
                domain,
                codomain,
                ..
            } = &current.form
            else {
                return Ok(());
            };
            match filling {
                Filling::Written => return Ok(()),
                Filling::Parameter => {
                    let meta = self.fresh_meta(current.origin, domain);
                    walk.slots.push(Slot::Parameter(meta.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Meta(meta)));
                    *ty = apply_closure(&mut self.meter, codomain, value)?;
                }
                Filling::Constraint(constraint) => {
                    let constraint = Arc::clone(constraint);
                    // The codomain reads the evidence off its binder; a meta
                    // stands for it, and [`Self::settled`] writes the computed
                    // evidence in — the one place a constraint is answered.
                    let meta = self.fresh_meta(current.origin, domain);
                    self.constraints.push((
                        constraint,
                        scope.clone(),
                        codomain.env.clone(),
                        current.origin,
                        meta.clone(),
                    ));
                    walk.slots.push(Slot::Evidence(meta.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Meta(meta)));
                    *ty = apply_closure(&mut self.meter, codomain, value)?;
                }
            }
        }
    }

    /// The walk's end: build the spine, and leave the residual type with the
    /// metas it still mentions — solved or not, which [`Self::settled`]
    /// audits.
    pub(super) fn finish_walk(
        here: Origin,
        head: Term,
        ty: Value,
        walk: &Walk,
        deferred: Vec<Term>,
    ) -> Result<Typed, ElabError> {
        // Consumed in order, which is what makes the emitted spine the written
        // one: the deferred slots stand in `walk.slots` in the order they were
        // written, and the second pass checked them in that same order.
        let mut deferred = deferred.into_iter();
        let mut term = head;
        for slot in &walk.slots {
            let argument = match slot {
                Slot::Parameter(meta) | Slot::Evidence(meta) => Term::meta(here, meta.clone()),
                Slot::Argument(term) => term.clone(),
                // The placeholder is the fallback rather than a panic because
                // it is a *correct* term: the second pass solved it to the
                // argument's value, so a spine built from it says the same
                // thing with the argument read back instead of as written.
                Slot::Deferred(meta) => deferred.next().unwrap_or_else(|| Term::meta(here, meta.clone())),
            };
            term = Term::app(here, term, argument);
        }
        Ok(Typed { term, ty })
    }
}

/// One instantiation pass's local state — see [`Elaborator::apply_spine`].
///
/// Just the spine: constraints are the elaborator's business, resolved at
/// declaration end, and an argument's term is pushed as it is elaborated.
#[derive(Default)]
pub(super) struct Walk {
    /// The spine slots to fill when the walk ends, in walk order.
    pub(super) slots: Vec<Slot>,
}

/// One argument [`Elaborator::apply_spine`] skipped, and where to put it back.
///
/// Not a postponed constraint: it is one argument, one slot, and one pass over
/// the list, all of it inside the call that made it. Nothing outlives the spine.
struct Waiting<'raw> {
    /// The argument, still unelaborated.
    argument: &'raw Raw,
    /// The domain it will be checked against — the same value, whose metas the
    /// rest of the walk may since have solved.
    domain: Arc<Value>,
    /// The placeholder that stood in the slot, so the checked argument can be
    /// made to agree with anything the walk decided about it.
    meta: crate::meta::Meta,
    /// Where the argument was written, for the placeholder and the agreement.
    at: Origin,
}

/// One spine slot of a [`Walk`].
pub(super) enum Slot {
    /// An implicit parameter: the meta stands in the term whether or not the
    /// walk solved it, and [`Elaborator::settled`] audits at declaration end.
    Parameter(crate::meta::Meta),
    /// A constraint's evidence, computed at declaration end.
    Evidence(crate::meta::Meta),
    /// An argument the author wrote, elaborated.
    Argument(Term),
    /// An argument the walk deferred, standing at the placeholder that held
    /// its slot. [`Elaborator::finish_walk`] fills it from the second pass.
    Deferred(crate::meta::Meta),
}
