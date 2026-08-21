//! An unknown's lifecycle: created, constrained, resolved, audited.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::refuse::{ElabError, Refusal};
use crate::value::Value;

use super::Elaborator;

impl Elaborator {
    /// The one finish point a declaration shares: refuse the first parameter
    /// nothing in it determined.
    ///
    /// §2.1's discipline, stated where it is enforced: instantiation is one
    /// matching pass, and the report names the *earliest* unsolved parameter
    /// because that is the one the author's next edit is about.
    pub(crate) fn settled(&mut self) -> Result<(), ElabError> {
        // Constraints first: a `Storable` answered here is a hole solved, and
        // the parameter audit below should not call a constructor's own
        // parameters undetermined for having answered one.
        let waiting = core::mem::take(&mut self.constraints);
        for (constraint, scope, env, at, hole) in waiting {
            let cx = scope.cx().clone();
            let term = crate::storable::discharge(self, &cx, &constraint, &env, at)?;
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
}
