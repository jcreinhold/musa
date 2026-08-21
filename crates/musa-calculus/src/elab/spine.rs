//! One instantiation walk: implicits filled, constraints noted, arguments placed.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::eval::{apply_closure, opened};
use crate::origin::Origin;
use crate::raw::Raw;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Plicity, Shape, Term};
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
        let term = crate::quote::quote(
            &mut self.meter,
            crate::quote::Depth(scope.depth()),
            crate::quote::Mode::Keep,
            &domain,
            param,
        )?;
        Ok(Typed {
            term: Term::app(here, head.term, term),
            ty: apply_closure(&mut self.meter, &codomain, param.clone())?,
        })
    }

    /// `head` applied to `count` fresh holes — a constructor's parameters when
    /// no expected type named the family. The fields solve them through §2.1's
    /// ordinary matching, and [`Elaborator::settled`] audits what they could
    /// not.
    pub(super) fn holes(
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
            let hole = self.fresh_hole(here, &domain);
            let value = Value::neutral(Neutral::head(here, crate::value::Head::Hole(hole.clone())));
            built = Typed {
                term: Term::app(here, built.term, Term::hole(here, hole)),
                ty: apply_closure(&mut self.meter, &codomain, value)?,
            };
        }
        Ok(built)
    }

    /// §2.1's one instantiation pass: apply `head` to the written arguments,
    /// then match what remains against `expected` when the call is in a
    /// checking position.
    ///
    /// One left-to-right walk, and the discipline is the document's: an
    /// implicit parameter becomes a [hole](crate::meta::Hole) that the first
    /// argument to mention it solves; an argument is *inferred* when its
    /// domain still mentions an unsolved hole and *checked* otherwise, because
    /// those are the two directions in which information can flow; a
    /// constraint waits until the walk has said everything matching can say,
    /// and is then resolved once, by lookup, never postponed. The author sees
    /// the two errors this can raise — a parameter nothing determined
    /// ([`Refusal::Unsolved`], at declaration end) and a call against a
    /// non-function ([`Refusal::NotAFunction`], here) — and neither involves
    /// a mechanism they have to name.
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
        // hole (§2.1): `None` is `None<?>` wherever it stands, and the slot it
        // is checked against solves the hole by ordinary first-order matching.
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
                self.holes(scope, here, head, params)?
            }
            Shape::Hole(_)
            | Shape::Var(_)
            | Shape::Const(_)
            | Shape::Def(_)
            | Shape::Numeral(_)
            | Shape::Base(_)
            | Shape::Lit(_)
            | Shape::Builtin(_)
            | Shape::Universe(_)
            | Shape::Pi { .. }
            | Shape::Lam { .. }
            | Shape::App { .. }
            | Shape::RecordType(_)
            | Shape::Record(_)
            | Shape::Project { .. }
            | Shape::Let { .. } => head,
        };
        let mut walk = Walk::default();
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
            // The domain decides the direction (§2.1): still quantified, the
            // argument teaches the parameter — inferred, and matched;
            // settled, the argument is checked. A checking-only form is checked
            // either way: inference has no rule for it, and the slot's Pi is
            // the type it was always going to be read against, holes included.
            let argument_term = if crate::unify::mentions_unsolved(&domain) && !argument.checks_only() {
                // Inferred — but an inferred head can still quantify over
                // parameters the domain determines (`identity` used unapplied):
                // the empty walk peels those into holes and does the matching,
                // which is §2.1's one rule rather than a second path here.
                let inferred = self.infer(scope, argument)?;
                self.apply_spine(scope, argument.origin(), inferred, &[], Some(&domain))?
                    .term
            } else {
                self.check(scope, argument, &domain)?
            };
            let value = scope.eval(&mut self.meter, &argument_term)?;
            walk.slots.push(Slot::Argument(argument_term));
            ty = apply_closure(&mut self.meter, &codomain, value)?;
            self.advance(scope, &mut ty, &mut walk)?;
        }
        if let Some(expected) = expected {
            // Checking position: the rest of the type is matched against what
            // the position wants, which is where a bare constructor's family
            // parameters — and any argument's still-unsolved ones — are
            // learned. The position is the mismatch's *expected*: it is the
            // type the author wrote and the walked type the one found.
            self.unifier
                .unify_types(&mut self.meter, scope.depth(), here, expected, &ty)?;
        }
        Self::finish_walk(here, head.term, ty, &walk)
    }

    /// Skip the binders §2.1 fills rather than the author: an implicit
    /// parameter becomes a fresh hole, a constraint is noted for the walk's
    /// end.
    pub(super) fn advance(&mut self, scope: &Scope, ty: &mut Value, walk: &mut Walk) -> Result<(), ElabError> {
        let _ = scope;
        loop {
            let unfolded = opened(&mut self.meter, ty)?;
            let current = unfolded.as_ref().unwrap_or(ty);
            let Form::Pi {
                plicity,
                domain,
                codomain,
                ..
            } = &current.form
            else {
                return Ok(());
            };
            match plicity {
                Plicity::Explicit => return Ok(()),
                Plicity::Implicit => {
                    let hole = self.fresh_hole(current.origin, domain);
                    walk.slots.push(Slot::Parameter(hole.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Hole(hole)));
                    *ty = apply_closure(&mut self.meter, codomain, value)?;
                }
                Plicity::Constraint(constraint) => {
                    let constraint = Arc::clone(constraint);
                    // The codomain reads the dictionary off its binder; a hole
                    // stands for it, and [`Self::settled`] writes the resolved
                    // dictionary in — the one place resolution runs, for the
                    // reason the field's doc gives.
                    let hole = self.fresh_hole(current.origin, domain);
                    self.constraints.push((
                        constraint,
                        scope.clone(),
                        codomain.env.clone(),
                        current.origin,
                        hole.clone(),
                    ));
                    walk.slots.push(Slot::Dictionary(hole.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Hole(hole)));
                    *ty = apply_closure(&mut self.meter, codomain, value)?;
                }
            }
        }
    }

    /// The walk's end: build the spine, and leave the residual type with the
    /// holes it still mentions — solved or not, which [`Self::settled`]
    /// audits.
    pub(super) fn finish_walk(here: Origin, head: Term, ty: Value, walk: &Walk) -> Result<Typed, ElabError> {
        let mut term = head;
        for slot in &walk.slots {
            let argument = match slot {
                Slot::Parameter(hole) | Slot::Dictionary(hole) => Term::hole(here, hole.clone()),
                Slot::Argument(term) => term.clone(),
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

/// One spine slot of a [`Walk`].
pub(super) enum Slot {
    /// An implicit parameter: the hole stands in the term whether or not the
    /// walk solved it, and [`Elaborator::settled`] audits at declaration end.
    Parameter(crate::meta::Hole),
    /// A constraint's dictionary, resolved at declaration end.
    Dictionary(crate::meta::Hole),
    /// An argument the author wrote, elaborated.
    Argument(Term),
}
