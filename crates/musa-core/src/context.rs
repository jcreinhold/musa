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
use crate::family::{Found, Group};
use crate::list::List;
use crate::origin::Origin;
use crate::quote::Depth;
use crate::term::{DbLevel, Index, Term};
use crate::value::{Env, Value};

/// The binders a term is read under, and the budget its conversions run in.
#[derive(Clone)]
pub struct Cx {
    env: Env,
    /// The type each binder was introduced at, innermost first.
    ///
    /// Kept rather than discarded, and that is not bookkeeping for its own sake:
    /// both [`Self::assume`] and [`Self::define`] evaluate a type already, and a
    /// context that threw the result away would force the elaborator — which
    /// must abstract a metavariable over every binder in scope, at its type — to
    /// evaluate all of them a second time. The environment answers this for an
    /// *assumption*, whose variable value carries its type; it cannot for a
    /// definition, whose value is the definition.
    types: List<Arc<Value>>,
    /// The declaration groups whose constants are in scope, most recent first.
    ///
    /// Beside the binders rather than among them, because a constant is not one:
    /// it has no de Bruijn index, nothing shadows it, and it is in scope in its
    /// own declaration. Keeping the two lists apart is what lets [`Self::closed`]
    /// drop every binder and keep every declaration, which is what a `data`
    /// declaration is elaborated in.
    declared: List<Arc<Group>>,
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
            types: List::EMPTY,
            declared: List::EMPTY,
            depth: 0,
            budget,
        }
    }

    /// This context's declarations, with none of its binders.
    ///
    /// What a `data` declaration is elaborated in: §1.1's parameters, indices,
    /// and constructor types are read under the declaration's own binders and
    /// nothing else, so a group elaborated inside a term would store terms whose
    /// variables named binders the group does not carry. Rather than track an
    /// offset nobody could check, a declaration is closed by construction.
    #[must_use]
    pub fn closed(&self) -> Self {
        Self {
            env: Env::EMPTY,
            types: List::EMPTY,
            declared: self.declared.clone(),
            depth: 0,
            budget: self.budget,
        }
    }

    /// This context with `group`'s families, constructors, and recursors in
    /// scope.
    #[must_use]
    pub fn declaring(&self, group: &Arc<Group>) -> Self {
        Self {
            declared: self.declared.push(Arc::clone(group)),
            ..self.clone()
        }
    }

    /// What a declared name refers to here, most recent declaration first.
    pub(crate) fn declared(&self, name: &str) -> Option<Found> {
        self.declared.iter().find_map(|group| Found::named(group, name))
    }

    /// This context extended by an assumption at type `ty`, written at
    /// `binder`.
    ///
    /// The binder has no name here, and that is not an omission. α-equivalence
    /// is decided by de Bruijn index, quotation takes the names it writes from
    /// the Π and λ values it walks, and a name in the context would be a second
    /// copy free to disagree with those. Prompt 134 attaches names where they
    /// are read — in a diagnostic.
    ///
    /// It does have an [`Origin`], and that *is* load-bearing: an assumption has
    /// no value to take one from, so unless the binder supplies it every
    /// occurrence of the variable in a normal form would point nowhere.
    /// [`Self::define`] needs no such argument, because its value already
    /// carries origins of its own.
    ///
    /// # Errors
    ///
    /// [`CoreError::Exhausted`] or [`CoreError::Malformed`] from evaluating
    /// `ty`, which is read in *this* context and so must be closed under it.
    pub fn assume(&self, binder: Origin, ty: &Term) -> Result<Self, CoreError> {
        let mut meter = Meter::new(self.budget);
        let ty = Arc::new(eval(&mut meter, &self.env, ty)?);
        Ok(self.assumed(binder, ty))
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
        // Nothing in this crate checks that `value` inhabits `ty` — the
        // elaborator does — but a type that cannot be evaluated is a defect
        // worth reporting where it was written rather than at the first
        // conversion that trips over it.
        let ty = Arc::new(eval(&mut meter, &self.env, ty)?);
        let value = eval(&mut meter, &self.env, value)?;
        Ok(self.defined(ty, value))
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

    /// This context extended by an assumption at an *already evaluated* type.
    ///
    /// What [`Self::assume`] is on top of, for the caller that has the type as
    /// a value already. Sharing the [`Arc`] with the variable's own type is why
    /// the two copies cannot disagree.
    pub(crate) fn assumed(&self, binder: Origin, ty: Arc<Value>) -> Self {
        let variable = Value::var(binder, DbLevel(self.depth), Arc::clone(&ty));
        self.pushed(ty, variable)
    }

    /// This context extended by an already-evaluated definition.
    pub(crate) fn defined(&self, ty: Arc<Value>, value: Value) -> Self {
        self.pushed(ty, value)
    }

    /// The type of every binder in scope, innermost first.
    ///
    /// Handed over whole rather than one lookup at a time, because the caller
    /// that wants them — a metavariable abstracting over its context — wants all
    /// of them in that order, and asking by index would make it recover the
    /// depth arithmetic this already knows.
    pub(crate) const fn binder_types(&self) -> &List<Arc<Value>> {
        &self.types
    }

    /// The type binder `index` was introduced at, counting outward from here.
    ///
    /// The other question about the same list, and it earns its own operation
    /// rather than making a caller index [`Self::binder_types`]: a type checker
    /// walking a term asks about exactly one binder at a time, and `None` here
    /// is the unbound variable it must report.
    pub(crate) fn binder_type(&self, index: Index) -> Option<&Arc<Value>> {
        self.types.get(index.0)
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

    fn pushed(&self, ty: Arc<Value>, value: Value) -> Self {
        Self {
            env: self.env.push(value),
            types: self.types.push(ty),
            declared: self.declared.clone(),
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
