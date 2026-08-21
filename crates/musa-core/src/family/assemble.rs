//! The telescope that assembles a constant's type semantically, which is
//! where the recursor of §1.1 is generated.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::Constant;
use super::group::{Binder, Group, Role};
use crate::budget::Meter;
use crate::error::CoreError;
use crate::eval::eval;
use crate::level::Level;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::term::{DbLevel, Index, Name, Plicity, Term};
use crate::value::{Env, Value};
use std::sync::Arc;

/// Where a binder sits, counted from the start of the telescope being built.
///
/// Absolute rather than relative on purpose: an index counts outward from a use
/// site and so changes with every binder introduced after it, which is exactly
/// the arithmetic that goes wrong by hand. A position never changes, and
/// [`Telescope::reference`] does the one subtraction.
#[derive(Clone, Copy)]
pub(super) struct At(u32);

/// A binder that has been introduced: where it sits, and the type it stands at.
type Introduced = (At, Value);

/// A telescope under construction, and the two environments it stands in.
///
/// Built forward and folded back at the end: `Term::pi` does not care what depth
/// it is at, because its domain and codomain were each quoted at the depth they
/// stand at. That is what makes the forward walk legal, and why nothing here
/// shifts an index.
///
/// **Two environments, because a stored term is not read in the one being
/// built.** A recursor introduces motives and methods between the parameters and
/// the fields, and a constructor's stored field type knows nothing about them. So
/// [`Self::env`] tracks every binder — it is what an assembled term is evaluated
/// in — while [`Self::reading`] tracks only the binders a stored term was written
/// under. One line keeps them right: [`Self::extend`] introduces a *stored*
/// telescope and pushes to both, [`Self::assume`] introduces a *synthesized*
/// binder and pushes to `env` alone.
pub(super) struct Telescope<'a> {
    group: &'a Arc<Group>,
    origin: Origin,
    /// Every binder introduced so far, matching [`Self::depth`].
    env: Env,
    /// Only the binders a stored term is read under.
    reading: Env,
    /// How many binders have been introduced, which is both the quoting depth
    /// and the next position.
    depth: u32,
    /// The binders, in order, to be folded into Π's by [`Self::close`].
    binders: Vec<(Plicity, Name, Term)>,
}

impl<'a> Telescope<'a> {
    pub(super) fn new(group: &'a Arc<Group>) -> Self {
        let declarations = Group::declarations(group);
        Self {
            group,
            origin: group.origin,
            env: declarations.clone(),
            reading: declarations,
            depth: 0,
            binders: Vec::new(),
        }
    }

    /// A telescope continuing this one: its environments and depth, its own
    /// binders.
    ///
    /// What makes a method's or a motive's Π chain nest inside the recursor's
    /// without either one's indices being wrong — both are quoted at the depth
    /// they actually stand at.
    pub(super) fn nested(&self) -> Self {
        Telescope {
            group: self.group,
            origin: self.origin,
            env: self.env.clone(),
            reading: self.reading.clone(),
            depth: self.depth,
            binders: Vec::new(),
        }
    }

    /// Introduce a synthesized binder at an already-assembled type.
    pub(super) fn assume(&mut self, meter: &mut Meter, name: &str, ty: Term) -> Result<At, CoreError> {
        let value = eval(meter, &self.env, &ty)?;
        let at = At(self.depth);
        self.env = self
            .env
            .push(Value::var(self.origin, DbLevel(self.depth), Arc::new(value)));
        self.depth = self.depth.saturating_add(1);
        self.binders.push((Plicity::Explicit, Arc::from(name), ty));
        Ok(at)
    }

    /// Introduce every binder of a stored telescope, answering where each sits
    /// and the type it stands at.
    pub(super) fn extend(&mut self, meter: &mut Meter, binders: &[Binder]) -> Result<Vec<Introduced>, CoreError> {
        let mut introduced = Vec::with_capacity(binders.len());
        for binder in binders {
            let value = eval(meter, &self.reading, &binder.ty)?;
            let ty = quote_type(meter, Depth(self.depth), crate::quote::Mode::Open, &value)?;
            // A constraint binder's arguments are terms read under exactly the
            // binders its *type* was read under, so they travel by the same
            // eval-then-quote this line already does for the type. Re-indexing
            // one and not the other is how a recursor's parameters would end up
            // naming a motive.
            let plicity = self.reindexed(meter, &binder.plicity)?;
            let variable = Value::var(self.origin, DbLevel(self.depth), Arc::new(value.clone()));
            self.env = self.env.push(variable.clone());
            self.reading = self.reading.push(variable);
            let at = At(self.depth);
            self.depth = self.depth.saturating_add(1);
            self.binders.push((plicity, Arc::clone(&binder.name), ty));
            introduced.push((at, value));
        }
        Ok(introduced)
    }

    /// A stored binder's plicity, with a constraint's arguments read at the
    /// depth this telescope has reached.
    fn reindexed(&self, meter: &mut Meter, plicity: &Plicity) -> Result<Plicity, CoreError> {
        let Plicity::Constraint(constraint) = plicity else {
            return Ok(plicity.clone());
        };
        let mut args = Vec::with_capacity(constraint.args.len());
        for argument in constraint.args.iter() {
            let value = eval(meter, &self.reading, argument)?;
            args.push(quote_type(meter, Depth(self.depth), crate::quote::Mode::Open, &value)?);
        }
        Ok(Plicity::Constraint(Arc::new(constraint.at(Arc::from(args)))))
    }

    /// One motive per family in the group: `P_j : N_j p⃗ → Type ℓ`.
    pub(super) fn motives(
        &mut self,
        meter: &mut Meter,
        params: &[Introduced],
        level: Level,
    ) -> Result<Vec<At>, CoreError> {
        let mut introduced = Vec::with_capacity(self.group.families.len());
        for which in 0..self.group.arity() {
            let ty = self.motive_type(meter, params, which, level)?;
            introduced.push(self.assume(meter, "P", ty)?);
        }
        Ok(introduced)
    }

    fn motive_type(
        &self,
        meter: &mut Meter,
        params: &[Introduced],
        which: u32,
        level: Level,
    ) -> Result<Term, CoreError> {
        if self.group.family_at(which).is_none() {
            return Ok(Term::universe(self.origin, Level::ZERO));
        }
        let mut inner = self.nested();
        let subject = inner.applied_family(which, [params, &[]]);
        inner.assume(meter, "t", subject)?;
        Ok(inner.close(Term::universe(self.origin, level)))
    }

    /// One method per constructor of every family in the group.
    pub(super) fn methods(
        &mut self,
        meter: &mut Meter,
        params: &[Introduced],
        motives: &[At],
    ) -> Result<(), CoreError> {
        for family in 0..self.group.arity() {
            let names: Vec<Name> = self.group.family_at(family).map_or_else(Vec::new, |declared| {
                declared
                    .constructors
                    .iter()
                    .map(|constructor| Arc::clone(&constructor.name))
                    .collect()
            });
            for (which, name) in names.into_iter().enumerate() {
                let which = u32::try_from(which).unwrap_or(u32::MAX);
                let ty = self.method_type(meter, params, motives, family, which)?;
                self.assume(meter, &name, ty)?;
            }
        }
        Ok(())
    }

    /// `(a⃗ : Fields) → (ih⃗) → P_j (c p⃗ a⃗)`.
    fn method_type(
        &self,
        meter: &mut Meter,
        params: &[Introduced],
        motives: &[At],
        family: u32,
        which: u32,
    ) -> Result<Term, CoreError> {
        let Some(constructor) = self
            .group
            .family_at(family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Ok(Term::universe(self.origin, Level::ZERO));
        };
        let fields_of = Arc::clone(&constructor.fields);
        let recursive = Arc::clone(&constructor.recursive);
        let mut inner = self.nested();
        let fields = inner.extend(meter, &fields_of)?;
        for (field, of_family) in recursive.iter() {
            let Some((at, _)) = fields.get(usize::try_from(*field).unwrap_or(usize::MAX)).cloned() else {
                continue;
            };
            let hypothesis = inner.hypothesis(motives, *of_family, at);
            inner.assume(meter, "ih", hypothesis)?;
        }
        let arguments = inner.references(params).into_iter().chain(inner.references(&fields));
        let built = applied(self.origin, inner.constructor(family, which), arguments);
        let motive = inner.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        let result = Term::app(self.origin, motive, built);
        Ok(inner.close(result))
    }

    /// `P_j a`, the induction hypothesis for a recursive field.
    fn hypothesis(&self, motives: &[At], family: u32, field: At) -> Term {
        let motive = self.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        Term::app(self.origin, motive, self.reference(Some(field)))
    }

    /// The variable naming the binder at `at`, seen from here.
    pub(super) fn reference(&self, at: Option<At>) -> Term {
        let Some(At(position)) = at else {
            return Term::universe(self.origin, Level::ZERO);
        };
        Term::var(
            self.origin,
            Index(self.depth.saturating_sub(1).saturating_sub(position)),
        )
    }

    pub(super) fn references(&self, introduced: &[Introduced]) -> Vec<Term> {
        introduced.iter().map(|(at, _)| self.reference(Some(*at))).collect()
    }

    /// The family, as a constant.
    pub(super) fn family(&self, which: u32) -> Term {
        Constant {
            group: Arc::clone(self.group),
            family: which,
            role: Role::Family,
        }
        .term(self.origin)
    }

    /// `N_which arg…`, for arguments already introduced as binders.
    pub(super) fn applied_family(&self, which: u32, groups: [&[Introduced]; 2]) -> Term {
        let arguments = groups.into_iter().flat_map(|group| self.references(group));
        applied(self.origin, self.family(which), arguments)
    }

    /// A constructor, as a constant.
    pub(super) fn constructor(&self, family: u32, which: u32) -> Term {
        Constant {
            group: Arc::clone(self.group),
            family,
            role: Role::Constructor(which),
        }
        .term(self.origin)
    }

    /// Fold the binders back into Π's around `result`.
    pub(super) fn close(self, result: Term) -> Term {
        self.binders
            .into_iter()
            .rev()
            .fold(result, |codomain, (plicity, name, domain)| {
                Term::function(self.origin, plicity, name, domain, codomain)
            })
    }

    /// [`Self::close`], for a caller holding the telescope by reference.
    pub(super) fn finish(&mut self, result: Term) -> Term {
        std::mem::take(&mut self.binders)
            .into_iter()
            .rev()
            .fold(result, |codomain, (plicity, name, domain)| {
                Term::function(self.origin, plicity, name, domain, codomain)
            })
    }
}

/// `head arg₀ … argₙ₋₁`.
pub(super) fn applied(origin: Origin, head: Term, args: impl IntoIterator<Item = Term>) -> Term {
    args.into_iter()
        .fold(head, |function, argument| Term::app(origin, function, argument))
}
