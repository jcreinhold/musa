//! Inductive families: what a declaration group is, and the recursor it
//! generates.
//!
//! `docs/rules/language/02-core-calculus.md` §1.1 declares families with
//! **parameters**, fixed across the whole declaration, and **indices**, which a
//! constructor chooses:
//!
//! ```text
//! data Vec (A : Type l) : (n : Nat) → Type l {
//!     Nil  : Vec A zero,
//!     Cons : (n : Nat) → A → Vec A n → Vec A (succ n),
//! }
//! ```
//!
//! The difference is load-bearing rather than cosmetic: the generated recursor's
//! motive quantifies over the *indices* and not over the parameters, so blurring
//! them produces an eliminator that type-checks and proves nothing useful. The
//! rule is enforced by the representation instead of by a check — a constructor
//! does not write its result type at all, only the index arguments it chooses, so
//! "parameters appear uniformly in every constructor's result" cannot be violated
//! by anything a caller writes.
//!
//! # The declaration context, and why terms are read under it
//!
//! A constructor's field type mentions the family it is declaring, and a mutual
//! group's constructors mention each other. Storing that as a [`Constant`]
//! pointing back at the [`Group`] would make the group an [`Arc`] cycle — a leak,
//! and a construction order with no first step.
//!
//! So every [`Term`] a group stores is read under **the declaration context**:
//! `k` binders, one per family in the group, outermost first. A recursive
//! occurrence is then an ordinary variable, which is also what makes strict
//! positivity a question about *variables* rather than about a name table. The
//! constants appear only in terms handed to a caller, which the group does not
//! own, so the cycle cannot form.
//!
//! [`Group::declarations`] is that context as an environment of values, which is
//! how a stored term is read: evaluating in it replaces each declaration variable
//! with the constant it names.
//!
//! # What the recursor is, and why it is assembled rather than stored
//!
//! For family `N_i` of a group `N_1 … N_k` over shared parameters `p⃗`:
//!
//! ```text
//! elim_i : (p⃗ : Params)
//!        → (P_1 : (i⃗ : Indices_1) → N_1 p⃗ i⃗ → Type ℓ) → … → (P_k : …)
//!        → (methods, one per constructor of every family in the group)
//!        → (i⃗ : Indices_i) → (t : N_i p⃗ i⃗) → P_i i⃗ t
//! ```
//!
//! and the method for a constructor `c` of family `N_j` is
//!
//! ```text
//! m_c : (a⃗ : Fields_c) → (ih⃗) → P_j idx_c (c p⃗ a⃗)
//! ```
//!
//! with one induction hypothesis `P_{j'} i⃗' a_ℓ` per recursive field. Motives and
//! methods sit *between* the parameters and the fields, so a field type stored at
//! one depth appears in the method at another — and this crate has no
//! substitution function to shift it with (§3: "reduction is never performed on
//! syntax"). The types are therefore **assembled semantically**: each stored term
//! is evaluated in the environment it was written in and quoted at the depth it
//! now stands at, which is what weakening *is* in a levelled semantic domain.
//!
//! That is also why assembly is handed the motive's universe: `ℓ` is chosen per
//! use site rather than fixed at declaration time, so one family supports both
//! small and large elimination without the universe polymorphism §1.3 refuses.
//!
//! # Recursive fields are direct
//!
//! A field may be an arrow, and it may mention the family — but not both: `sup :
//! (Nat → W) → W` is refused, while `Cons : A → List A → List A` is not. §1.1
//! permits the infinitary constructor, so this is a real narrowing, and §1.2 is
//! what makes it free: "an arrow type is never storable, and neither is any
//! container holding one", so a family with an infinitary constructor could never
//! carry a payload, be a machine port, or cross the kernel boundary. What it buys
//! is that an induction hypothesis is an application rather than a synthesized
//! closure, so ι never builds syntax.

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::CoreError;
use crate::eval::{apply, eval, force};
use crate::level::Level;
use crate::list::List;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::term::{DbLevel, Index, Name, Shape, Term};
use crate::value::{Env, Form, Neutral, Spine, Value};

/// One binder of a telescope: a name and the type it stands at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binder {
    /// The binder's written name, for diagnostics and for what quotation prints.
    pub name: Name,
    /// Its type, read under the declaration context and the binders before it.
    pub ty: Term,
}

/// One constructor of a family.
#[derive(Debug)]
pub struct Constructor {
    /// Its name, which is how a pattern and a diagnostic refer to it.
    pub(crate) name: Name,
    /// Its arguments, a telescope read under the declaration context and the
    /// group's parameters.
    pub(crate) fields: Arc<[Binder]>,
    /// Which fields are recursive occurrences, as `(field, family)` pairs in
    /// field order.
    ///
    /// Computed once by the positivity check rather than recovered at every
    /// reduction: ι needs exactly this list to know which arguments take an
    /// induction hypothesis.
    pub(crate) recursive: Arc<[(u32, u32)]>,
    /// The index arguments its result chooses, read under the declaration
    /// context, the parameters, and its own fields.
    pub(crate) indices: Arc<[Term]>,
}

/// One family of a declaration group.
#[derive(Debug)]
pub struct Declared {
    /// Its name.
    pub(crate) name: Name,
    /// Its indices, a telescope read under the declaration context and the
    /// group's parameters.
    pub(crate) indices: Arc<[Binder]>,
    /// The universe it lands in: the join of its constructors' field levels,
    /// with recursive occurrences contributing nothing.
    pub(crate) level: Level,
    /// Its constructors, in declaration order.
    pub(crate) constructors: Arc<[Constructor]>,
}

/// A declaration group: families declared together, over shared parameters.
///
/// Together rather than one at a time because §1.1 checks strict positivity on
/// the group, and because a mutual group has one recursor per family over *all*
/// the group's motives and methods — neither is a question about a single
/// declaration.
///
/// The parameters belong to the group rather than to each family, which is what
/// makes the mutual recursor statable at all: a motive for `N_j` quantifies over
/// `N_j`'s indices at the *same* parameters the eliminated `N_i` was taken at.
#[derive(Debug)]
pub struct Group {
    /// Where the declaration was written. Every term generated from it says so,
    /// which is §7's preservation clause applied to syntax nobody wrote.
    pub(crate) origin: Origin,
    /// The parameters, read under the declaration context.
    pub(crate) params: Arc<[Binder]>,
    /// The families, in declaration order.
    pub(crate) families: Arc<[Declared]>,
}

/// A type that turned out to be a family applied to its arguments.
///
/// What splitting a `match` subject needs and nothing more: which family, at
/// which parameters, at which indices. Parameters and indices are separated here
/// rather than handed over as one spine, because every rule downstream treats
/// them differently — a motive quantifies over the indices and never over the
/// parameters (§1.1).
pub(crate) struct Element {
    pub(crate) group: Arc<Group>,
    pub(crate) family: u32,
    pub(crate) params: Vec<Value>,
    pub(crate) indices: Vec<Value>,
}

/// The family `ty` is the type of elements of, if it is one.
///
/// # Errors
///
/// As [`force`], from unfolding the type far enough to see its head.
pub(crate) fn element(meter: &mut Meter, ty: &Value) -> Result<Option<Element>, CoreError> {
    let ty = force(meter, ty)?.unwrap_or_else(|| ty.clone());
    let Form::Neutral(neutral) = &ty.form else {
        return Ok(None);
    };
    let Some((constant, mut arguments)) = spine(neutral) else {
        return Ok(None);
    };
    let Role::Family = constant.role else {
        return Ok(None);
    };
    let params = usize::try_from(constant.group.params()).unwrap_or(usize::MAX);
    if arguments.len() < params {
        return Ok(None);
    }
    let indices = arguments.split_off(params);
    Ok(Some(Element {
        group: Arc::clone(&constant.group),
        family: constant.family,
        params: arguments,
        indices,
    }))
}

/// Which of a declaration's three constants this is.
#[derive(Clone, Debug)]
pub(crate) enum Role {
    /// The family itself, `N p⃗ i⃗`.
    Family,
    /// One of its constructors.
    Constructor(u32),
    /// Its generated dependent recursor, at the universe its motives land in.
    ///
    /// The level rides on the *use site* rather than on the declaration: §1.3
    /// admits "no universe polymorphism beyond level metavariables", and a
    /// recursor whose motive universe was fixed when the family was declared
    /// would be small-elimination-only or large-elimination-only forever.
    Recursor(Level),
}

/// A declared constant: a family, a constructor, or a recursor.
///
/// Holds the whole group rather than a name, which is what lets evaluation fire ι
/// and quotation recover a type without a signature threaded through every
/// operation in the crate — `AGENTS.md`'s "hand a consumer what we already
/// computed", applied to the one place it would otherwise cost a parameter on
/// `eval`, `quote`, `apply`, and `Cx` alike.
#[derive(Clone, Debug)]
pub struct Constant {
    pub(crate) group: Arc<Group>,
    pub(crate) family: u32,
    pub(crate) role: Role,
}

/// Two constants are the same when they name the same thing.
///
/// By *name*, not by pointer: a term elaborated twice from one source must
/// compare equal, and two elaborations build two groups. A program cannot declare
/// one name twice, so the name decides.
///
/// A recursor's motive universe is part of its identity, because `Type 0` is not
/// `Type 1` (§1, predicative and not cumulative) and two eliminations into
/// different universes are two terms.
impl PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        if self.name() != other.name() {
            return false;
        }
        match (&self.role, &other.role) {
            (Role::Recursor(one), Role::Recursor(two)) => one.resolved() == two.resolved(),
            (Role::Family, Role::Family) | (Role::Constructor(_), Role::Constructor(_)) => true,
            (Role::Family | Role::Constructor(_) | Role::Recursor(_), _) => false,
        }
    }
}

impl Eq for Constant {}

impl core::fmt::Display for Constant {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(&self.name())
    }
}

/// What a name a declaration group put in scope refers to.
///
/// Two variants rather than one [`Constant`], because a recursor is not finished
/// until its motives have a universe and that is chosen per use site (see
/// [`Role::Recursor`]). Returning a `Constant` with a placeholder level would let
/// a caller forget to replace it and get a term that quietly eliminates into the
/// wrong universe.
pub(crate) enum Found {
    /// A family or a constructor, which needs nothing else to be a constant.
    Rigid(Constant),
    /// A recursor, waiting for the universe its motives land in.
    Recursor(Arc<Group>, u32),
}

impl Found {
    /// What `name` refers to in `group`, or `None` when the group does not
    /// declare it.
    ///
    /// The names a group introduces are its families, each family's constructors
    /// qualified by it, and `N.elim` — the same spellings [`Constant::name`]
    /// prints, so a diagnostic naming a constant names something the author can
    /// write.
    pub(crate) fn named(group: &Arc<Group>, name: &str) -> Option<Self> {
        for (family, declared) in group.families.iter().enumerate() {
            let family = u32::try_from(family).unwrap_or(u32::MAX);
            if *declared.name == *name {
                return Some(Self::Rigid(Constant {
                    group: Arc::clone(group),
                    family,
                    role: Role::Family,
                }));
            }
            let Some(member) = name
                .strip_prefix(&*declared.name)
                .and_then(|rest| rest.strip_prefix('.'))
            else {
                continue;
            };
            if member == "elim" {
                return Some(Self::Recursor(Arc::clone(group), family));
            }
            let which = declared
                .constructors
                .iter()
                .position(|constructor| *constructor.name == *member)?;
            return Some(Self::Rigid(Constant {
                group: Arc::clone(group),
                family,
                role: Role::Constructor(u32::try_from(which).unwrap_or(u32::MAX)),
            }));
        }
        None
    }

    /// The constant this refers to, at the universe a recursor's motives land
    /// in.
    pub(crate) fn at(self, level: Level) -> Constant {
        match self {
            Self::Rigid(constant) => constant,
            Self::Recursor(group, family) => Constant {
                group,
                family,
                role: Role::Recursor(level),
            },
        }
    }

    /// Whether finishing this needs a level the caller has to create.
    pub(crate) const fn is_recursor(&self) -> bool {
        matches!(*self, Self::Recursor(_, _))
    }
}

impl Group {
    /// How many families the group declares.
    pub(crate) fn arity(&self) -> u32 {
        u32::try_from(self.families.len()).unwrap_or(u32::MAX)
    }

    /// How many parameters they share.
    pub(crate) fn params(&self) -> u32 {
        u32::try_from(self.params.len()).unwrap_or(u32::MAX)
    }

    /// How many methods a recursor over this group takes: one per constructor of
    /// every family in it.
    fn methods(&self) -> u32 {
        self.families
            .iter()
            .map(|family| u32::try_from(family.constructors.len()).unwrap_or(u32::MAX))
            .fold(0, u32::saturating_add)
    }

    /// The declaration context as an environment.
    ///
    /// The `k` family constants, ordered so that the variable a stored term uses
    /// for family `i` finds family `i`. Every stored term is read in this
    /// environment, extended by whatever binders it stands under.
    pub(crate) fn declarations(group: &Arc<Self>) -> Env {
        let mut env = List::EMPTY;
        for family in 0..group.arity() {
            env = env.push(
                Constant {
                    group: Arc::clone(group),
                    family,
                    role: Role::Family,
                }
                .value(group.origin),
            );
        }
        env
    }

    /// Where the method handling family `family`'s `constructor`th constructor
    /// sits among the recursor's methods.
    fn method_position(&self, family: u32, constructor: u32) -> u32 {
        let before: u32 = self
            .families
            .iter()
            .take(usize::try_from(family).unwrap_or(usize::MAX))
            .map(|declared| u32::try_from(declared.constructors.len()).unwrap_or(u32::MAX))
            .fold(0, u32::saturating_add);
        before.saturating_add(constructor)
    }

    pub(crate) fn family_at(&self, which: u32) -> Option<&Declared> {
        self.families.get(usize::try_from(which).unwrap_or(usize::MAX))
    }
}

impl Declared {
    pub(crate) fn constructor_at(&self, which: u32) -> Option<&Constructor> {
        self.constructors.get(usize::try_from(which).unwrap_or(usize::MAX))
    }

    fn indices(&self) -> u32 {
        u32::try_from(self.indices.len()).unwrap_or(u32::MAX)
    }
}

impl Constant {
    /// One of a group's families, as the type constructor it is.
    pub(crate) fn family(group: &Arc<Group>, family: u32) -> Self {
        Self {
            group: Arc::clone(group),
            family,
            role: Role::Family,
        }
    }

    /// One of a group's constructors.
    pub(crate) fn constructor(group: &Arc<Group>, family: u32, which: u32) -> Self {
        Self {
            group: Arc::clone(group),
            family,
            role: Role::Constructor(which),
        }
    }

    /// A family's generated recursor, eliminating into `level`.
    pub(crate) fn recursor(group: &Arc<Group>, family: u32, level: Level) -> Self {
        Self {
            group: Arc::clone(group),
            family,
            role: Role::Recursor(level),
        }
    }

    /// Its name, qualified by the family it belongs to.
    ///
    /// A constructor is `Vec.Cons` and a recursor is `Vec.elim`, so two families
    /// may each declare a `Nil` without either shadowing the other and without a
    /// diagnostic that cannot say which one it meant.
    pub(crate) fn name(&self) -> Name {
        let Some(declared) = self.group.family_at(self.family) else {
            return Arc::from("?");
        };
        match &self.role {
            Role::Family => Arc::clone(&declared.name),
            Role::Constructor(which) => match declared.constructor_at(*which) {
                Some(constructor) => Arc::from(format!("{}.{}", declared.name, constructor.name)),
                None => Arc::from("?"),
            },
            Role::Recursor(_) => Arc::from(format!("{}.elim", declared.name)),
        }
    }

    /// This constant as a term.
    pub(crate) fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Const(self.clone()))
    }

    /// This constant as a value: a rigid neutral, which is what a constant is.
    pub(crate) fn value(&self, origin: Origin) -> Value {
        Value::neutral(Neutral {
            origin,
            spine: Spine::Const(self.clone()),
        })
    }

    /// How many arguments saturate it.
    ///
    /// A family takes its parameters and indices, a constructor takes the
    /// parameters and its fields, and a recursor takes everything up to and
    /// including the target. ι fires exactly at this count on a recursor, which
    /// is why it is one number rather than a shape match at every application.
    pub(crate) fn arity(&self) -> u32 {
        let params = self.group.params();
        let Some(declared) = self.group.family_at(self.family) else {
            return 0;
        };
        match &self.role {
            Role::Family => params.saturating_add(declared.indices()),
            Role::Constructor(which) => {
                let fields = declared.constructor_at(*which).map_or(0, |constructor| {
                    u32::try_from(constructor.fields.len()).unwrap_or(u32::MAX)
                });
                params.saturating_add(fields)
            }
            Role::Recursor(_) => params
                .saturating_add(self.group.arity())
                .saturating_add(self.group.methods())
                .saturating_add(declared.indices())
                .saturating_add(1),
        }
    }

    /// The type this constant has.
    ///
    /// Assembled rather than stored, for the reason the module doc gives: the
    /// pieces are read at depths the assembled type does not put them at, and
    /// evaluating and re-quoting is how a levelled semantic domain weakens.
    ///
    /// # Errors
    ///
    /// As [`eval`]: assembly evaluates every stored telescope it walks.
    pub(crate) fn ty(&self, meter: &mut Meter) -> Result<Value, CoreError> {
        let term = self.ty_term(meter)?;
        eval(meter, &Env::EMPTY, &term)
    }

    /// The same type, as a closed term.
    ///
    /// Closed because assembly resolves every declaration variable into the
    /// constant it names, so nothing here is read under the declaration context
    /// even though everything it was built from was.
    ///
    /// # Errors
    ///
    /// As [`eval`].
    pub(crate) fn ty_term(&self, meter: &mut Meter) -> Result<Term, CoreError> {
        let mut builder = Telescope::new(&self.group);
        match &self.role {
            Role::Family => self.family_type(meter, &mut builder),
            Role::Constructor(which) => self.constructor_type(meter, builder, *which),
            Role::Recursor(level) => self.recursor_type(meter, builder, level),
        }
    }

    /// `(p⃗ : Params) → (i⃗ : Indices) → Type l`.
    fn family_type(&self, meter: &mut Meter, builder: &mut Telescope<'_>) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Level::ZERO));
        };
        builder.extend(meter, &self.group.params)?;
        builder.extend(meter, &declared.indices)?;
        let level = declared.level.clone();
        Ok(builder.finish(Term::universe(self.group.origin, level)))
    }

    /// `(p⃗ : Params) → (a⃗ : Fields) → N p⃗ idx⃗`.
    fn constructor_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, which: u32) -> Result<Term, CoreError> {
        let Some(constructor) = self
            .group
            .family_at(self.family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Ok(Term::universe(self.group.origin, Level::ZERO));
        };
        let params = builder.extend(meter, &self.group.params)?;
        builder.extend(meter, &constructor.fields)?;
        let indices = builder.quoted(meter, &constructor.indices)?;
        let arguments = builder.references(&params).into_iter().chain(indices);
        let result = applied(self.group.origin, builder.family(self.family), arguments);
        Ok(builder.close(result))
    }

    /// The recursor's type, at the universe its motives land in.
    fn recursor_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, level: &Level) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Level::ZERO));
        };
        let here = self.group.origin;
        let params = builder.extend(meter, &self.group.params)?;
        let motives = builder.motives(meter, &params, level)?;
        builder.methods(meter, &params, &motives)?;
        let indices = builder.extend(meter, &declared.indices)?;
        let subject = builder.applied_family(self.family, [&params, &indices]);
        let target = builder.assume(meter, "t", subject)?;
        let motive = builder.reference(motives.get(usize::try_from(self.family).unwrap_or(usize::MAX)).copied());
        let at_indices = applied(here, motive, builder.references(&indices));
        let result = Term::app(here, at_indices, builder.reference(Some(target)));
        Ok(builder.close(result))
    }
}

/// Where a binder sits, counted from the start of the telescope being built.
///
/// Absolute rather than relative on purpose: an index counts outward from a use
/// site and so changes with every binder introduced after it, which is exactly
/// the arithmetic that goes wrong by hand. A position never changes, and
/// [`Telescope::reference`] does the one subtraction.
#[derive(Clone, Copy)]
struct At(u32);

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
struct Telescope<'a> {
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
    binders: Vec<(Name, Term)>,
}

impl<'a> Telescope<'a> {
    fn new(group: &'a Arc<Group>) -> Self {
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
    fn nested(&self) -> Self {
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
    fn assume(&mut self, meter: &mut Meter, name: &str, ty: Term) -> Result<At, CoreError> {
        let value = eval(meter, &self.env, &ty)?;
        let at = At(self.depth);
        self.env = self
            .env
            .push(Value::var(self.origin, DbLevel(self.depth), Arc::new(value)));
        self.depth = self.depth.saturating_add(1);
        self.binders.push((Arc::from(name), ty));
        Ok(at)
    }

    /// Introduce every binder of a stored telescope, answering where each sits
    /// and the type it stands at.
    fn extend(&mut self, meter: &mut Meter, binders: &[Binder]) -> Result<Vec<Introduced>, CoreError> {
        let mut introduced = Vec::with_capacity(binders.len());
        for binder in binders {
            let value = eval(meter, &self.reading, &binder.ty)?;
            let ty = quote_type(meter, Depth(self.depth), &value)?;
            let variable = Value::var(self.origin, DbLevel(self.depth), Arc::new(value.clone()));
            self.env = self.env.push(variable.clone());
            self.reading = self.reading.push(variable);
            let at = At(self.depth);
            self.depth = self.depth.saturating_add(1);
            self.binders.push((Arc::clone(&binder.name), ty));
            introduced.push((at, value));
        }
        Ok(introduced)
    }

    /// One motive per family in the group: `P_j : (i⃗ : Indices_j) → N_j p⃗ i⃗ →
    /// Type ℓ`.
    fn motives(&mut self, meter: &mut Meter, params: &[Introduced], level: &Level) -> Result<Vec<At>, CoreError> {
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
        level: &Level,
    ) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(which) else {
            return Ok(Term::universe(self.origin, Level::ZERO));
        };
        let indices = Arc::clone(&declared.indices);
        let mut inner = self.nested();
        let bound = inner.extend(meter, &indices)?;
        let subject = inner.applied_family(which, [params, &bound]);
        inner.assume(meter, "t", subject)?;
        Ok(inner.close(Term::universe(self.origin, level.clone())))
    }

    /// One method per constructor of every family in the group.
    fn methods(&mut self, meter: &mut Meter, params: &[Introduced], motives: &[At]) -> Result<(), CoreError> {
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

    /// `(a⃗ : Fields) → (ih⃗) → P_j idx⃗ (c p⃗ a⃗)`.
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
        let chooses = Arc::clone(&constructor.indices);
        let mut inner = self.nested();
        let fields = inner.extend(meter, &fields_of)?;
        for (field, of_family) in recursive.iter() {
            let Some((at, ty)) = fields.get(usize::try_from(*field).unwrap_or(usize::MAX)).cloned() else {
                continue;
            };
            let hypothesis = inner.hypothesis(meter, motives, *of_family, at, &ty)?;
            inner.assume(meter, "ih", hypothesis)?;
        }
        let indices = inner.quoted(meter, &chooses)?;
        let arguments = inner.references(params).into_iter().chain(inner.references(&fields));
        let built = applied(self.origin, inner.constructor(family, which), arguments);
        let motive = inner.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        let result = Term::app(self.origin, applied(self.origin, motive, indices), built);
        Ok(inner.close(result))
    }

    /// `P_j i⃗ a`, the induction hypothesis for a recursive field.
    ///
    /// The index arguments come from the type the field was *declared* at, which
    /// is the one place they are written; the constructor's own result indices
    /// describe a different value.
    fn hypothesis(
        &self,
        meter: &mut Meter,
        motives: &[At],
        family: u32,
        field: At,
        ty: &Value,
    ) -> Result<Term, CoreError> {
        let written = quote_type(meter, Depth(self.depth), ty)?;
        let indices = index_arguments(&written, self.group.params.len());
        let motive = self.reference(motives.get(usize::try_from(family).unwrap_or(usize::MAX)).copied());
        Ok(Term::app(
            self.origin,
            applied(self.origin, motive, indices),
            self.reference(Some(field)),
        ))
    }

    /// Quote stored terms at the depth this telescope has reached.
    fn quoted(&self, meter: &mut Meter, terms: &[Term]) -> Result<Vec<Term>, CoreError> {
        terms
            .iter()
            .map(|term| {
                let value = eval(meter, &self.reading, term)?;
                quote_type(meter, Depth(self.depth), &value)
            })
            .collect()
    }

    /// The variable naming the binder at `at`, seen from here.
    fn reference(&self, at: Option<At>) -> Term {
        let Some(At(position)) = at else {
            return Term::universe(self.origin, Level::ZERO);
        };
        Term::var(
            self.origin,
            Index(self.depth.saturating_sub(1).saturating_sub(position)),
        )
    }

    fn references(&self, introduced: &[Introduced]) -> Vec<Term> {
        introduced.iter().map(|(at, _)| self.reference(Some(*at))).collect()
    }

    /// The family, as a constant.
    fn family(&self, which: u32) -> Term {
        Constant {
            group: Arc::clone(self.group),
            family: which,
            role: Role::Family,
        }
        .term(self.origin)
    }

    /// `N_which arg…`, for arguments already introduced as binders.
    fn applied_family(&self, which: u32, groups: [&[Introduced]; 2]) -> Term {
        let arguments = groups.into_iter().flat_map(|group| self.references(group));
        applied(self.origin, self.family(which), arguments)
    }

    /// A constructor, as a constant.
    fn constructor(&self, family: u32, which: u32) -> Term {
        Constant {
            group: Arc::clone(self.group),
            family,
            role: Role::Constructor(which),
        }
        .term(self.origin)
    }

    /// Fold the binders back into Π's around `result`.
    fn close(self, result: Term) -> Term {
        self.binders.into_iter().rev().fold(result, |codomain, (name, domain)| {
            Term::pi(self.origin, name, domain, codomain)
        })
    }

    /// [`Self::close`], for a caller holding the telescope by reference.
    fn finish(&mut self, result: Term) -> Term {
        std::mem::take(&mut self.binders)
            .into_iter()
            .rev()
            .fold(result, |codomain, (name, domain)| {
                Term::pi(self.origin, name, domain, codomain)
            })
    }
}

/// `head arg₀ … argₙ₋₁`.
fn applied(origin: Origin, head: Term, args: impl IntoIterator<Item = Term>) -> Term {
    args.into_iter()
        .fold(head, |function, argument| Term::app(origin, function, argument))
}

/// The index arguments of `N p⃗ i⃗`, given how many leading arguments are
/// parameters.
///
/// Reads the spine of a type the positivity check has already established is a
/// family occurrence, so a shape that is not one answers no indices rather than a
/// diagnostic nobody would see.
fn index_arguments(ty: &Term, params: usize) -> Vec<Term> {
    let mut arguments = Vec::new();
    let mut head = ty;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument.clone());
        head = function;
    }
    arguments.reverse();
    if arguments.len() <= params {
        return Vec::new();
    }
    arguments.split_off(params)
}

/// The constant at the head of a blocked spine, and what has been applied to it.
fn spine(neutral: &Neutral) -> Option<(Constant, Vec<Value>)> {
    let mut arguments = Vec::new();
    let mut at = neutral;
    loop {
        match &at.spine {
            Spine::App { function, argument } => {
                arguments.push(Value::clone(argument));
                at = function;
            }
            Spine::Const(constant) => {
                arguments.reverse();
                return Some((constant.clone(), arguments));
            }
            Spine::Var(_, _) | Spine::Meta(_) | Spine::Project { .. } | Spine::J { .. } => return None,
        }
    }
}

/// ι at an inductive family, or `None` when the elimination stays blocked.
///
/// `elim_i p⃗ P⃗ m⃗ i⃗ (c p⃗ a⃗) ⟶ m_c a⃗ ih⃗`. The induction hypothesis for a
/// recursive field is `elim_{j'} p⃗ P⃗ m⃗ i⃗' a_ℓ` — another elimination, which
/// [`apply`] fires in turn, and which is why recursion here is the family's own
/// structure rather than a fixed point. §1.1's strict positivity is what makes
/// that descent terminate; the meter is what makes a violated invariant a refusal
/// rather than a hang.
///
/// # Errors
///
/// As [`apply`].
pub(crate) fn iota(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Value>, CoreError> {
    let Some(reduction) = ready(meter, neutral)? else {
        return Ok(None);
    };
    let here = neutral.origin;
    let mut answer = reduction.method.clone();
    for field in &reduction.fields {
        answer = apply(meter, here, answer, field.clone())?;
    }
    for hypothesis in hypotheses(meter, &reduction)? {
        answer = apply(meter, here, answer, hypothesis)?;
    }
    Ok(Some(answer))
}

/// Everything ι needs once it has decided the elimination fires.
struct Reduction {
    /// The method for the constructor the target was built by.
    method: Value,
    /// The constructor's field arguments.
    fields: Vec<Value>,
    /// The recursor's parameters, motives, and methods — what an induction
    /// hypothesis is the same elimination at.
    prefix: Vec<Value>,
    /// The group, and the constructor within it that the target was built by.
    group: Arc<Group>,
    family: u32,
    which: u32,
    /// The universe the motives land in, which an induction hypothesis inherits.
    level: Level,
}

/// Decide whether `neutral` is a saturated recursor applied to a constructor,
/// and take apart what it is applied to.
fn ready(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Reduction>, CoreError> {
    let Some((recursor, arguments)) = spine(neutral) else {
        return Ok(None);
    };
    let Role::Recursor(level) = recursor.role.clone() else {
        return Ok(None);
    };
    if u32::try_from(arguments.len()).unwrap_or(u32::MAX) != recursor.arity() {
        return Ok(None);
    }
    let group = Arc::clone(&recursor.group);
    let prefix_len = usize::try_from(
        group
            .params()
            .saturating_add(group.arity())
            .saturating_add(group.methods()),
    )
    .unwrap_or(usize::MAX);
    let Some(target) = arguments.last() else {
        return Ok(None);
    };
    let target = force(meter, target)?.unwrap_or_else(|| target.clone());
    let Form::Neutral(target) = &target.form else {
        return Ok(None);
    };
    let Some((constructor, built)) = spine(target) else {
        return Ok(None);
    };
    let Role::Constructor(which) = constructor.role else {
        return Ok(None);
    };
    let params = usize::try_from(group.params()).unwrap_or(usize::MAX);
    let motives = usize::try_from(group.arity()).unwrap_or(usize::MAX);
    let position = usize::try_from(group.method_position(constructor.family, which)).unwrap_or(usize::MAX);
    let Some(method) = arguments.get(params.saturating_add(motives).saturating_add(position)) else {
        return Ok(None);
    };
    Ok(Some(Reduction {
        method: method.clone(),
        fields: built.get(params..).unwrap_or_default().to_vec(),
        prefix: arguments.get(..prefix_len).unwrap_or_default().to_vec(),
        group,
        family: constructor.family,
        which,
        level,
    }))
}

/// One induction hypothesis per recursive field, in field order.
///
/// Which fields are recursive is read off [`Constructor::recursive`] rather than
/// re-derived from the field types here, so that the method the hypothesis is
/// passed to and the hypothesis itself cannot disagree about how many arguments
/// there are: they are the same list.
fn hypotheses(meter: &mut Meter, reduction: &Reduction) -> Result<Vec<Value>, CoreError> {
    let group = &reduction.group;
    let here = group.origin;
    let Some(rule) = group
        .family_at(reduction.family)
        .and_then(|declared| declared.constructor_at(reduction.which))
    else {
        return Ok(Vec::new());
    };
    let params = usize::try_from(group.params()).unwrap_or(usize::MAX);
    // A stored field type is read in the declaration context, then the
    // parameters, then the fields before it — which is exactly the environment
    // this walk builds up.
    let mut reading = Group::declarations(group);
    for param in reduction.prefix.iter().take(params) {
        reading = reading.push(param.clone());
    }
    let mut recursive = rule.recursive.iter().peekable();
    let mut built = Vec::new();
    for (position, field) in reduction.fields.iter().enumerate() {
        let position = u32::try_from(position).unwrap_or(u32::MAX);
        let ty = match rule.fields.get(usize::try_from(position).unwrap_or(usize::MAX)) {
            Some(binder) => eval(meter, &reading, &binder.ty)?,
            None => break,
        };
        if recursive.peek().is_some_and(|(at, _)| *at == position) {
            let of_family = recursive.next().map_or(0, |(_, family)| *family);
            let mut hypothesis = Constant {
                group: Arc::clone(group),
                family: of_family,
                role: Role::Recursor(reduction.level.clone()),
            }
            .value(here);
            let arguments = reduction
                .prefix
                .iter()
                .cloned()
                .chain(index_values(&ty, group.params()));
            for argument in arguments {
                hypothesis = apply(meter, here, hypothesis, argument)?;
            }
            built.push(apply(meter, here, hypothesis, field.clone())?);
        }
        reading = reading.push(field.clone());
    }
    Ok(built)
}

/// The index arguments of a value of family type.
fn index_values(ty: &Value, params: u32) -> Vec<Value> {
    let Form::Neutral(neutral) = &ty.form else {
        return Vec::new();
    };
    let Some((_, arguments)) = spine(neutral) else {
        return Vec::new();
    };
    let params = usize::try_from(params).unwrap_or(usize::MAX);
    arguments.get(params..).map(<[Value]>::to_vec).unwrap_or_default()
}
