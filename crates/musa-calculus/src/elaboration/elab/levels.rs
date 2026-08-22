//! Universe levels during elaboration: created, assigned, generalized,
//! defaulted.
//!
//! `docs/rules/language/02-core-calculus.md` §1's hierarchy is
//! universe-polymorphic and nobody writes a level, so every level in a program
//! starts as an unknown. This file is that unknown's lifecycle, and it is the
//! level sort's answer to what [`super::metas`] is for the term sort.
//!
//! # Per declaration, and that is the whole scope
//!
//! An [`Elaborator`] is built per declaration, so its level variables are too:
//! there is no global constraint graph, no inequality set that can go
//! inconsistent halfway through a build, and no way for one declaration's
//! anonymous `Type` to be constrained from another. That is Lean's arrangement
//! rather than Coq's typical ambiguity, and it is the one that composes with
//! separate compilation.
//!
//! Variables are numbered from zero *per elaboration*, which is what keeps a
//! report reading `u0` rather than `u2417`. Two declarations therefore both own
//! a `u0`, and the invariant that makes that safe is stated once here: **a
//! stored level parameter never leaves its [`Defined`](crate::kernel::program)
//! un-instantiated.** Every use of a definition replaces all of its parameters
//! with the use site's own fresh variables ([`Elaborator::instantiate_levels`]),
//! so two declarations' parameters are never in the same comparison.
//!
//! # The defaulting rule, written down rather than emergent
//!
//! At the end of a declaration a level variable is in one of two states, and
//! the rule says which:
//!
//! - **It occurs in the declaration's type.** It is *generalized* — it becomes
//!   a level parameter, and every use site picks its own.
//! - **It does not.** It is *defaulted to `0`*. A level nothing in the type
//!   mentions cannot be chosen by a use site, because a use site sees only the
//!   type; generalizing it would make a parameter no unification could ever
//!   determine, and refusing it would reject a program over an unknown that
//!   provably cannot matter to any caller.
//!
//! Stating it rather than letting it fall out of solver order is `budget.rs`'s
//! argument applied to levels: a program whose acceptance depends on the order
//! constraints happened to arrive in is a program two compilers disagree about.

use std::sync::Arc;

use crate::kernel::origin::Origin;
use crate::kernel::sort::{Levels, Sort, SortVar};
use crate::kernel::term::Term;

use super::Elaborator;

impl Elaborator {
    /// A level nothing has determined yet: `Type`'s level, before use.
    pub(crate) fn fresh_level(&mut self, here: Origin) -> Sort {
        let var = SortVar::new(self.next_level, here);
        self.next_level = self.next_level.saturating_add(1);
        self.created_levels.push(var.clone());
        Sort::var(var)
    }

    /// One fresh level per parameter of a definition being used.
    ///
    /// The use site's half of §1's "generalized per declaration, instantiated
    /// at each use". The levels are unknowns like any other, so the ordinary
    /// conversion at the use site is what determines them, and the defaulting
    /// rule above is what happens when it determines nothing.
    pub(crate) fn instantiate_levels(&mut self, here: Origin, parameters: usize) -> Levels {
        if parameters == 0 {
            return Levels::NONE;
        }
        Levels::of((0..parameters).map(|_| self.fresh_level(here)))
    }

    /// The level parameters `ty` generalizes over, in the order it mentions
    /// them.
    ///
    /// Called once, at the declaration boundary, before [`Self::settled`]
    /// defaults what is left. Written order rather than creation order because
    /// the parameter list a reader sees should follow the type they are
    /// reading.
    ///
    /// [`Self::settled`]: super::Elaborator::settled
    pub(crate) fn generalize_levels(&mut self, ty: &Term) -> Arc<[SortVar]> {
        let mut found = Vec::new();
        ty.level_vars(&mut found);
        found.retain(|var| var.solution().is_none());
        self.generalized_levels.clone_from(&found);
        Arc::from(found)
    }

    /// Solve every level nothing determined and nothing generalized to `0`.
    ///
    /// The second half of the defaulting rule, run from [`Self::settled`] so
    /// that the term audit below it sees levels that are settled rather than
    /// open.
    ///
    /// # Errors
    ///
    /// Never in practice: a variable is written once, and this is the only
    /// writer that runs after assignment has stopped. It answers a
    /// [`crate::kernel::error::Malformed`] rather than asserting, because a
    /// second write would be a solver defect and this crate reports those.
    ///
    /// [`Self::settled`]: super::Elaborator::settled
    pub(crate) fn default_levels(&mut self) -> Result<(), crate::kernel::error::CoreError> {
        let created = core::mem::take(&mut self.created_levels);
        for var in &created {
            if var.solution().is_none() && !self.generalized_levels.contains(var) {
                var.solve(Sort::ZERO)?;
            }
        }
        Ok(())
    }
}
