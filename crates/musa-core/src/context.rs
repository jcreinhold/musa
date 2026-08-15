//! The typing context.
//!
//! A context is the list of binders a term is read under, and this one carries
//! two things per binder: the type it was introduced at, and — for a binder
//! introduced by a definition rather than an assumption — the value it stands
//! for. That second half is δ. Looking a defined variable up finds its value
//! rather than a variable, so unfolding a definition is what the environment
//! already does and there is no separate unfolding rule.
//!
//! **Extending a context evaluates; conversion under it does not.** That split
//! is why [`Cx`] holds a [`Budget`] rather than a meter. Limits are inherited by
//! every operation under a context; a *spend* belongs to the operation that made
//! it, and one meter shared across operations would make a conversion's answer
//! depend on how many ran before it.
//!
//! Extension is persistent — [`Cx::assume`] and [`Cx::define`] answer a new
//! context and leave the old one usable — because an elaborator descends into
//! two branches from one context and neither may see the other's binders.

use std::sync::Arc;

use crate::budget::{Budget, Meter};
use crate::error::CoreError;
use crate::eval::eval;
use crate::quote::Depth;
use crate::term::{DbLevel, Term};
use crate::value::{Env, Value};

/// The binders a term is read under, and the budget its conversions run in.
#[derive(Clone)]
pub struct Cx {
    env: Env,
    depth: u32,
    budget: Budget,
}

impl Cx {
    /// The empty context, at the language budget.
    #[must_use]
    pub fn new() -> Self {
        Self::with_budget(Budget::LANGUAGE)
    }

    /// The empty context, at a stated budget.
    ///
    /// The compiler never calls this with anything but [`Budget::LANGUAGE`] —
    /// see [`Budget::scaled`] for why the other caller exists and what it is
    /// for.
    #[must_use]
    pub const fn with_budget(budget: Budget) -> Self {
        Self {
            env: Env::EMPTY,
            depth: 0,
            budget,
        }
    }

    /// This context extended by an assumption at type `ty`.
    ///
    /// The binder has no name here, and that is not an omission. α-equivalence
    /// is decided by de Bruijn index, quotation takes the names it writes from
    /// the Π and λ values it walks, and a name in the context would be a second
    /// copy free to disagree with those. Prompt 134 attaches names where they
    /// are read — in a diagnostic.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] or [`CoreError::Malformed`] from evaluating
    /// `ty`, which is read in *this* context and so must be closed under it.
    pub fn assume(&self, ty: &Term) -> Result<Self, CoreError> {
        let mut meter = Meter::new(self.budget);
        let ty = eval(&mut meter, &self.env, ty)?;
        Ok(self.pushed(Value::var(DbLevel(self.depth), Arc::new(ty))))
    }

    /// This context extended by a definition of `value` at type `ty`.
    ///
    /// The binder's value goes into the environment, so a later term that names
    /// it sees the definition unfolded. That is δ, and it is the reason this is
    /// a different operation from [`Self::assume`] rather than a flag on it.
    ///
    /// # Errors
    ///
    /// As [`Self::assume`], for either term.
    pub fn define(&self, ty: &Term, value: &Term) -> Result<Self, CoreError> {
        let mut meter = Meter::new(self.budget);
        // The type is evaluated and discarded: nothing in this crate checks
        // that `value` inhabits it — prompt 134's elaborator does — but a type
        // that cannot be evaluated is a defect worth reporting where it was
        // written rather than at the first conversion that trips over it.
        drop(eval(&mut meter, &self.env, ty)?);
        let value = eval(&mut meter, &self.env, value)?;
        Ok(self.pushed(value))
    }

    /// How many binders are in scope.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }

    /// The budget every operation under this context runs in.
    #[must_use]
    pub const fn budget(&self) -> Budget {
        self.budget
    }

    pub(crate) const fn quoting_depth(&self) -> Depth {
        Depth(self.depth)
    }

    pub(crate) const fn env(&self) -> &Env {
        &self.env
    }

    pub(crate) fn meter(&self) -> Meter {
        Meter::new(self.budget)
    }

    fn pushed(&self, value: Value) -> Self {
        Self {
            env: self.env.extend(value),
            depth: self.depth.saturating_add(1),
            budget: self.budget,
        }
    }
}

impl Default for Cx {
    fn default() -> Self {
        Self::new()
    }
}
