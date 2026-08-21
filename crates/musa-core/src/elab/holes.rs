//! An unknown's lifecycle: created, constrained, resolved, audited.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::Plicity;
use crate::value::{Env, Value};

use super::Elaborator;

impl Elaborator {
    /// The one finish point a declaration shares: refuse the first parameter
    /// nothing in it determined.
    ///
    /// §2.1's discipline, stated where it is enforced: instantiation is one
    /// matching pass, and the report names the *earliest* unsolved parameter
    /// because that is the one the author's next edit is about.
    pub(crate) fn settled(&mut self) -> Result<(), ElabError> {
        // Constraints first: a dictionary resolved here is a hole solved, and
        // the parameter audit below should not call an instance's own
        // parameters undetermined for having answered one.
        let waiting = core::mem::take(&mut self.constraints);
        for (constraint, scope, env, at, hole) in waiting {
            // Resolved against the *creation* scope: the local dictionaries
            // step 1 looks up are the `where` binders in scope where the
            // constraint was met, and a retry anywhere else would prefer a
            // global instance to the author's own clause.
            let term = crate::dictionary::resolve_at(self, &scope, &constraint, &env, at)?;
            let value = crate::eval::eval(&mut self.meter, &env, &term)?;
            hole.solve(value).map_err(crate::error::CoreError::from)?;
        }
        if let Some(hole) = self.created.iter().find(|hole| !hole.is_solved()) {
            return Err(Refusal::Unsolved {
                site: MetaSource::TypeParameter,
                created: hole.origin(),
                blocked: None,
            }
            .into());
        }
        Ok(())
    }

    /// A hole for a constraint's dictionary, registered for resolution at
    /// [`Self::settled`].
    ///
    /// The registration is the whole of postponement that survives the course
    /// correction: no queue is retried, the constraint is resolved once, when
    /// the declaration's matching has said everything it can.
    pub(crate) fn constrain(
        &mut self,
        scope: &Scope,
        constraint: Arc<crate::class::Constraint>,
        env: Env,
        at: Origin,
        ty: &Value,
    ) -> crate::meta::Hole {
        let hole = self.fresh_hole(at, ty);
        self.constraints
            .push((constraint, scope.clone(), env, at, hole.clone()));
        hole
    }

    /// A placeholder for an implicit argument the walk has not solved yet.
    ///
    /// Creation is where §2.1's discipline is cheap to state: the hole's type
    /// was checked before it was made, and solving is write-once, so the audit
    /// [`Self::settled`] runs is a walk of a list and not a query of a solver.
    pub(crate) fn fresh_hole(&mut self, here: Origin, ty: &Value) -> crate::meta::Hole {
        let hole = crate::meta::Hole::new(self.next_hole, here, ty.clone());
        self.next_hole = self.next_hole.saturating_add(1);
        self.created.push(hole.clone());
        hole
    }

    /// `scope` with a constraint binder's key discharged, for the binder about
    /// to be assumed.
    ///
    /// The second half of the abstracting rule, and not an optimization:
    /// `10-traits.md` §4 step 1 answers a constraint from "a dictionary bound by
    /// an enclosing `where` clause", so a body that writes `x == y` inside
    /// `same` has to reach *this* binder rather than a global instance. Without
    /// it the binder would be bound and unreachable, and a generic definition
    /// would type-check and then resolve to an instance its caller did not
    /// choose. [`crate::dictionary`]'s `requirements` does both for a derived
    /// method's `where`, and this is the same pair one level out.
    ///
    /// A constraint whose head is not yet known discharges nothing, which is
    /// §4's postponement rather than a failure: the key does not exist yet, and
    /// a use inside the body is postponed until it does.
    pub(crate) fn discharging(&mut self, scope: &Scope, plicity: &Plicity, at: &Env) -> Result<Scope, ElabError> {
        let Plicity::Constraint(constraint) = plicity else {
            return Ok(scope.clone());
        };
        let needed = crate::dictionary::instantiated(self, scope, constraint, at)?;
        let Some(key) = crate::dictionary::discharges(&needed, scope) else {
            return Ok(scope.clone());
        };
        let args = crate::dictionary::valued(self, scope, &needed)?;
        Ok(scope.discharging(key, scope.depth(), args))
    }
}
