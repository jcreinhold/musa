//! The telescope that assembles a constant's type semantically, which is
//! where the dependent eliminator of §1.1 is generated.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::Constant;
use super::group::{Group, Parameter, Role};
use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::CoreError;
use crate::kernel::eval::eval;
use crate::kernel::origin::Origin;
use crate::kernel::quote::{quote, quote_type};
use crate::kernel::sort::Sort;
use crate::kernel::term::{Filling, Index, Level, Name, Term};
use crate::kernel::value::{Env, Value};
use std::sync::Arc;

/// Where a binder sits, counted from the start of the telescope being built.
///
/// Absolute rather than relative on purpose: an index counts outward from a use
/// site and so changes with every binder introduced after it, which is exactly
/// the arithmetic that goes wrong by hand. A position never changes, and
/// [`Telescope::reference`] does the one subtraction.
#[derive(Clone, Copy)]
pub(super) struct At(Level);

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
    /// The declaration context alone, kept because reading a family's index
    /// telescope means starting from it again rather than from wherever this
    /// walk has reached — see [`Self::index_terms`].
    declarations: Env,
    /// Every binder introduced so far, matching [`Self::depth`].
    env: Env,
    /// Only the binders a stored term is read under.
    reading: Env,
    /// How many binders have been introduced, which is both the quoting depth
    /// and the next position.
    depth: Level,
    /// The binders, in order, to be folded into Π's by [`Self::close`].
    binders: Vec<(Filling, Name, Term)>,
}

impl<'a> Telescope<'a> {
    pub(super) fn new(group: &'a Arc<Group>, globals: &Globals) -> Self {
        let declarations = Group::declarations(group, globals);
        Self {
            group,
            origin: group.origin,
            declarations: declarations.clone(),
            env: declarations.clone(),
            reading: declarations,
            depth: Level::ZERO,
            binders: Vec::new(),
        }
    }

    /// A telescope continuing this one: its environments and depth, its own
    /// binders.
    ///
    /// What makes a method's Π chain nest inside the recursor's without either
    /// one's de Bruijn indices being wrong — both are quoted at the depth they
    /// actually stand at.
    pub(super) fn nested(&self) -> Self {
        Telescope {
            group: self.group,
            origin: self.origin,
            declarations: self.declarations.clone(),
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
        self.env = self.env.push(Value::var(self.origin, self.depth, Arc::new(value)));
        self.depth = self.depth.deeper();
        self.binders.push((Filling::Written, Arc::from(name), ty));
        Ok(at)
    }

    /// Introduce every binder of a stored telescope, answering where each sits
    /// and the type it stands at.
    pub(super) fn extend(&mut self, meter: &mut Meter, binders: &[Parameter]) -> Result<Vec<Introduced>, CoreError> {
        let mut introduced = Vec::with_capacity(binders.len());
        for binder in binders {
            let value = eval(meter, &self.reading, &binder.ty)?;
            let ty = quote_type(meter, self.depth, crate::kernel::quote::Mode::Open, &value)?;
            // A constraint binder's arguments are terms read under exactly the
            // binders its *type* was read under, so they travel by the same
            // eval-then-quote this line already does for the type. Re-indexing
            // one and not the other is how a recursor's parameters would end up
            // naming a motive.
            let filling = self.reindexed(meter, &binder.filling)?;
            let variable = Value::var(self.origin, self.depth, Arc::new(value.clone()));
            self.env = self.env.push(variable.clone());
            self.reading = self.reading.push(variable);
            let at = At(self.depth);
            self.depth = self.depth.deeper();
            self.binders.push((filling, Arc::clone(&binder.name), ty));
            introduced.push((at, value));
        }
        Ok(introduced)
    }

    /// A stored binder's filling, with a constraint's arguments read at the
    /// depth this telescope has reached.
    fn reindexed(&self, meter: &mut Meter, filling: &Filling) -> Result<Filling, CoreError> {
        let Filling::Constraint(constraint) = filling else {
            return Ok(filling.clone());
        };
        let mut args = Vec::with_capacity(constraint.args.len());
        for argument in constraint.args.iter() {
            let value = eval(meter, &self.reading, argument)?;
            args.push(quote_type(meter, self.depth, crate::kernel::quote::Mode::Open, &value)?);
        }
        Ok(Filling::Constraint(Arc::new(constraint.at(Arc::from(args)))))
    }

    /// The index arguments of a family element, read back as terms at the depth
    /// this telescope has reached.
    ///
    /// An index is an ordinary term at an ordinary type, so reading one back
    /// needs its type, and the type of index `k` is the family's own `k`th
    /// index binder — read under the declaration context, the parameters the
    /// element was taken at, and the indices before it. That telescope is
    /// rebuilt here from `params` rather than taken from this walk, because the
    /// element may belong to a *different* family of the group than the one
    /// being assembled, at parameters that are values rather than binders.
    ///
    /// The two vectors are expected to be the family's parameter and index
    /// counts; a short one stops the walk rather than guessing, since a caller
    /// that miscounted has a defect a fabricated argument would hide.
    fn index_terms(
        &self,
        meter: &mut Meter,
        family: u32,
        params: &[Value],
        indices: &[Value],
    ) -> Result<Vec<Term>, CoreError> {
        let Some(declared) = self.group.family_at(family) else {
            return Ok(Vec::new());
        };
        let mut reading = self.declarations.clone();
        for param in params {
            reading = reading.push(param.clone());
        }
        let mut read = Vec::with_capacity(indices.len());
        for (binder, value) in declared.indices.iter().zip(indices) {
            let ty = eval(meter, &reading, &binder.ty)?;
            read.push(quote(meter, self.depth, crate::kernel::quote::Mode::Open, &ty, value)?);
            reading = reading.push(value.clone());
        }
        Ok(read)
    }

    /// The indices constructor `which` of `family` chose, as terms at this
    /// depth.
    ///
    /// The stored terms are read in [`Self::reading`] — the declaration
    /// context, the parameters, and the constructor's own fields, which is
    /// exactly what a chosen index was written under — and then read back at
    /// [`Self::depth`], which is where the method being assembled stands. The
    /// two differ by the recursor's motives and methods, and that difference is
    /// the whole reason this is an evaluation rather than a copy.
    pub(super) fn chosen(
        &self,
        meter: &mut Meter,
        params: &[Introduced],
        family: u32,
        which: u32,
    ) -> Result<Vec<Term>, CoreError> {
        let Some(constructor) = self
            .group
            .family_at(family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Ok(Vec::new());
        };
        let mut chosen = Vec::with_capacity(constructor.chosen.len());
        for term in constructor.chosen.iter() {
            chosen.push(eval(meter, &self.reading, term)?);
        }
        let params = introduced(self.origin, params);
        self.index_terms(meter, family, &params, &chosen)
    }

    /// One motive per family in the group: `R_j : (t : N_j p⃗) → Type ℓ`.
    ///
    /// A **family** of types, not a type. §1.1: "the generated eliminator's
    /// motive is a family … so a method's result type and its induction
    /// hypotheses are the motive *applied to* … the value being eliminated".
    /// That application is the whole of what dependent elimination is, and it
    /// is what lets a branch be checked at the goal *refined by the pattern it
    /// matched*.
    ///
    /// One per family rather than one overall because a mutual recursor
    /// eliminates into a different answer per family, which is what makes it
    /// statable at all.
    ///
    /// `params` are the parameters already introduced, because the family a
    /// motive stands over is the applied `N_j p⃗` and not the bare constant.
    pub(super) fn motives(
        &mut self,
        meter: &mut Meter,
        params: &[Introduced],
        level: &Sort,
    ) -> Result<Vec<At>, CoreError> {
        let mut introduced = Vec::with_capacity(self.group.families.len());
        for which in 0..self.group.arity() {
            let ty = self.motive_family(meter, which, params, level.clone())?;
            introduced.push(self.assume(meter, "R", ty)?);
        }
        Ok(introduced)
    }

    /// `(i⃗ : Indices) → (t : N_which p⃗ i⃗) → Type ℓ`, assembled in a telescope
    /// of its own so that the index and subject binders are inside the motive's
    /// Π and not in the recursor's.
    ///
    /// The indices are bound **here** and not at the recursor, which is what
    /// makes refinement possible: a method for a constructor that chose `0`
    /// answers at the motive applied to `0`, and a method for one that chose
    /// `n + 1` answers at the motive applied to `n + 1`. A motive that took its
    /// indices outside would have to answer at one index for every case, which
    /// is the non-dependent eliminator §1.1 replaced.
    fn motive_family(
        &self,
        meter: &mut Meter,
        which: u32,
        params: &[Introduced],
        level: Sort,
    ) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(which) else {
            return Ok(Term::universe(self.origin, Sort::ZERO));
        };
        let indices = Arc::clone(&declared.indices);
        let mut inner = self.nested();
        let bound = inner.extend(meter, &indices)?;
        let subject = inner.applied_family(which, [params, &bound]);
        inner.assume(meter, "t", subject)?;
        Ok(inner.close(Term::universe(self.origin, level)))
    }

    /// One method per constructor of every family in the group.
    pub(super) fn methods(
        &mut self,
        meter: &mut Meter,
        motives: &[At],
        params: &[Introduced],
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
                let ty = self.method_type(meter, motives, params, family, which)?;
                self.assume(meter, &name, ty)?;
            }
        }
        Ok(())
    }

    /// `(a⃗ : Fields) → (ih⃗) → R_j (c p⃗ a⃗)`.
    fn method_type(
        &self,
        meter: &mut Meter,
        motives: &[At],
        params: &[Introduced],
        family: u32,
        which: u32,
    ) -> Result<Term, CoreError> {
        let Some(constructor) = self
            .group
            .family_at(family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Ok(Term::universe(self.origin, Sort::ZERO));
        };
        let fields_of = Arc::clone(&constructor.fields);
        let recursive = Arc::clone(&constructor.recursive);
        let mut inner = self.nested();
        let fields = inner.extend(meter, &fields_of)?;
        for (field, of_family) in recursive.iter() {
            // A recursive position past the fields would be a defect in the
            // declaration rather than a hypothesis to invent, and skipping it
            // keeps this arity and [`super::iota`]'s in agreement.
            let Some((at, ty)) = fields.get(usize::try_from(*field).unwrap_or(usize::MAX)) else {
                continue;
            };
            // The hypothesis stands at the indices the *field's own type* was
            // written at — `tail : Vec<A>(n)` earns `R n tail`, not `R tail` —
            // and those are read off that type rather than guessed, so the
            // hypothesis and the recursion [`super::iota`] fires agree by
            // construction.
            let element = super::group::element(meter, ty)?;
            let indices = match &element {
                Some(element) => inner.index_terms(meter, element.family, &element.params, &element.indices)?,
                None => Vec::new(),
            };
            let hypothesis = inner.hypothesis(motives, *of_family, *at, indices);
            inner.assume(meter, "ih", hypothesis)?;
        }
        // The subject this method answers *for*, which is what the motive is
        // applied to: not the eliminated value, which no method has, but the
        // constructor form this method is the case of.
        let built = applied(
            self.origin,
            Constant::constructor(self.group, family, which).term(self.origin),
            inner.references(params).into_iter().chain(inner.references(&fields)),
        );
        let motive = inner.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        // The motive at *this constructor's* indices, which is the refinement:
        // `Nil`'s method answers at `R 0` and `Cons`'s at `R (n + 1)`, because
        // that is what each of them chose.
        let chosen = inner.chosen(meter, params, family, which)?;
        let applied_motive = applied(self.origin, motive, chosen);
        Ok(inner.close(Term::app(self.origin, applied_motive, built)))
    }

    /// `R_j i⃗ a`, the induction hypothesis for the recursive field at `at`.
    ///
    /// The field **does** appear in it, which is what dependent means: the
    /// hypothesis is the answer the recursion produced *about that field*, and
    /// not merely an inhabitant of a fixed answer type. The field is also what
    /// [`super::iota`] applies the recursor to when the hypothesis is forced,
    /// so the two agree by construction rather than by a rule that says so.
    fn hypothesis(&self, motives: &[At], family: u32, at: At, indices: Vec<Term>) -> Term {
        let motive = self.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        let at_indices = applied(self.origin, motive, indices);
        Term::app(self.origin, at_indices, self.reference(Some(at)))
    }

    /// The variable naming the binder at `at`, seen from here.
    pub(super) fn reference(&self, at: Option<At>) -> Term {
        let Some(At(position)) = at else {
            return Term::universe(self.origin, Sort::ZERO);
        };
        // The level-to-index conversion, which is what a reference *is*: the
        // binder sits at `position` counting in, and the use site sits at
        // `self.depth` counting in, so the index is the distance between them.
        Term::var(self.origin, position.to_index(self.depth).unwrap_or(Index(0)))
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

    /// Fold the binders back into Π's around `result`.
    pub(super) fn close(self, result: Term) -> Term {
        self.binders
            .into_iter()
            .rev()
            .fold(result, |codomain, (filling, name, domain)| {
                Term::function(self.origin, filling, name, domain, codomain)
            })
    }

    /// Bind the next *stored* binder to a value that is not a binder of this
    /// telescope.
    ///
    /// The one thing [`Self::extend`] cannot do. A field accessor reads field
    /// `i`'s stored type, which was written under the fields before it — and
    /// those fields are not binders here, because the accessor takes the whole
    /// value and not its parts. Each earlier field therefore stands at *its own
    /// accessor applied to that value*, which is the substitution
    /// `01-surface.md` §1.2's dependent record needs and the only reason the
    /// two environments are separate.
    pub(super) fn standing(&mut self, value: Value) {
        self.reading = self.reading.push(value);
    }

    /// The environment every binder introduced so far stands in.
    pub(super) const fn under(&self) -> &Env {
        &self.env
    }

    /// The environment a *stored* term is read in — the declaration context and
    /// the binders that term was written under, and nothing this walk
    /// synthesized.
    pub(super) const fn reading_env(&self) -> &Env {
        &self.reading
    }

    /// How deep this telescope has reached, which is where a term it assembles
    /// is quoted.
    pub(super) const fn reached(&self) -> Level {
        self.depth
    }

    /// [`Self::close`], for a caller holding the telescope by reference.
    pub(super) fn finish(&mut self, result: Term) -> Term {
        std::mem::take(&mut self.binders)
            .into_iter()
            .rev()
            .fold(result, |codomain, (filling, name, domain)| {
                Term::function(self.origin, filling, name, domain, codomain)
            })
    }
}

/// The variables a telescope introduced, as values standing at their own types.
///
/// [`Introduced`] pairs a position with the *type* the binder stands at, which
/// is exactly what a variable value is made of, so this is the pairing read the
/// other way round rather than a second source of truth.
fn introduced(origin: Origin, binders: &[Introduced]) -> Vec<Value> {
    binders
        .iter()
        .map(|(At(position), ty)| Value::var(origin, *position, Arc::new(ty.clone())))
        .collect()
}

/// `head arg₀ … argₙ₋₁`.
pub(super) fn applied(origin: Origin, head: Term, args: impl IntoIterator<Item = Term>) -> Term {
    args.into_iter()
        .fold(head, |function, argument| Term::app(origin, function, argument))
}
