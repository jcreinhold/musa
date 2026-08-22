//! An unknown's lifecycle: created, constrained, resolved, audited.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::meta::MetaSource;
use crate::kernel::origin::Origin;
use crate::kernel::value::Value;

use super::Elaborator;

impl Elaborator {
    /// The one finish point a declaration shares: refuse the first parameter
    /// nothing in it determined.
    ///
    /// §2.1's discipline, stated where it is enforced: instantiation is one
    /// matching pass, and the report names the *earliest* unsolved parameter
    /// because that is the one the author's next edit is about.
    pub(crate) fn settled(&mut self) -> Result<(), ElabError> {
        // Levels first, and before the constraints below: a `Storable`
        // discharged here evaluates a term, and a term whose levels are still
        // open would carry an unknown into a stored value. See
        // [`super::levels`] for the rule this runs.
        self.default_levels()?;
        // The level equations conversion could not answer on the spot, read
        // once more now that defaulting has closed everything open. One place,
        // one message: postponing and then reporting from two places would be
        // two sentences about one disagreement.
        for equation in self.conversion.postponed_levels() {
            equation.settled()?;
        }
        // Constraints next: a `Storable` answered here is a meta solved, and
        // the parameter audit below should not call a constructor's own
        // parameters undetermined for having answered one.
        let waiting = core::mem::take(&mut self.constraints);
        for (constraint, scope, env, at, meta) in waiting {
            let cx = scope.cx().clone();
            let term = crate::elaboration::storable::discharge(self, &cx, &constraint, &env, at)?;
            let value = crate::kernel::eval::eval(&mut self.meter, &env, &term)?;
            meta.solve(value).map_err(crate::kernel::error::CoreError::from)?;
        }
        if let Some(meta) = self.created.iter().find(|meta| !meta.is_solved()) {
            return Err(Refusal::Unsolved {
                site: MetaSource::TypeParameter,
                created: meta.origin(),
                blocked: None,
            }
            .into());
        }
        Ok(())
    }

    /// A placeholder for an implicit argument the walk has not solved yet.
    ///
    /// Creation is where §2.1's discipline is cheap to state: the meta's type
    /// was checked before it was made, and solving is write-once, so the audit
    /// [`Self::settled`] runs is a walk of a list and not a query of a solver.
    pub(crate) fn fresh_meta(&mut self, here: Origin, ty: &Value) -> crate::kernel::meta::Meta {
        let meta = crate::kernel::meta::Meta::new(self.next_meta, here, ty.clone());
        self.next_meta = self.next_meta.saturating_add(1);
        self.created.push(meta.clone());
        meta
    }
}
