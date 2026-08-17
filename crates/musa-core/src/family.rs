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

use crate::base::Datum;
use crate::budget::Meter;
use crate::class::PackageId;
use crate::error::{CoreError, Malformed};
use crate::eval::{apply, eval, force};
use crate::level::Level;
use crate::list::List;
use crate::origin::Origin;
use crate::quote::{Depth, quote, quote_type};
use crate::term::{DbLevel, Index, Name, Plicity, Shape, Term};
use crate::value::{Elim, Env, Form, Head, Neutral, Value};
use crate::visibility::{ModuleId, Visibility};

/// One binder of a telescope: a name, the type it stands at, and how a use site
/// fills it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binder {
    /// The binder's written name, for diagnostics and for what quotation prints.
    pub name: Name,
    /// Its type, read under the declaration context and the binders before it.
    pub ty: Term,
    /// How a use site supplies its argument.
    ///
    /// [`Plicity::Explicit`] for everything a declaration writes — §1.1's
    /// parameters and indices are written at every use, which is what
    /// `List<Nat>` is. A group's `where` clause appends
    /// [`Plicity::Constraint`] parameters after the written ones, and those are
    /// answered by `10-traits.md` §4 instead of written; see
    /// [`RawData::context`](crate::RawData).
    pub plicity: Plicity,
}

impl Binder {
    /// A binder a use site writes: `(name : ty)`.
    #[must_use]
    pub fn explicit(name: Name, ty: Term) -> Self {
        Self {
            name,
            ty,
            plicity: Plicity::Explicit,
        }
    }
}

/// One constructor of a family.
#[derive(Debug)]
pub struct Constructor {
    /// Its name, which is how a pattern and a diagnostic refer to it.
    pub(crate) name: Name,
    /// Whether it may be named outside the module its group was declared in.
    pub(crate) visibility: Visibility,
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

/// The two constructors that make a family *count*.
///
/// A family counts when a closed value of it is completely described by how many
/// steps it stands above a floor: the group takes no parameters, the family takes
/// no indices, and it has exactly two constructors, one with no fields and one
/// with a single field that is a recursive occurrence of the family itself. That
/// sentence is both the recognition rule and the soundness argument for
/// [`Shape::Numeral`](crate::term::Shape::Numeral), which is why it is one
/// sentence and not two.
///
/// Derived at [`crate::declare`] from the declaration's own shape rather than
/// nominated by the host. A `Registry` row would have to be filled *after*
/// `declare_data` had built the group, and a call order enforced by convention is
/// coupling this crate does not have anywhere else; deriving costs no parameter,
/// no ordering, and no host knowledge, and any family of this shape gets the
/// representation without asking for it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Counting {
    /// The constructor with no fields — what a count of zero denotes.
    pub(crate) floor: u32,
    /// The constructor with one self-typed field — one more than its argument.
    pub(crate) step: u32,
}

/// One family of a declaration group.
#[derive(Debug)]
pub struct Declared {
    /// Its name.
    pub(crate) name: Name,
    /// Whether the *type* may be named outside the module its group was
    /// declared in. Independent of its constructors': `01-surface.md` §1.3's
    /// whole point is a public type whose cases are package-maintained.
    pub(crate) visibility: Visibility,
    /// Its indices, a telescope read under the declaration context and the
    /// group's parameters.
    pub(crate) indices: Arc<[Binder]>,
    /// The universe it lands in: the join of its constructors' field levels,
    /// with recursive occurrences contributing nothing.
    pub(crate) level: Level,
    /// Its constructors, in declaration order.
    pub(crate) constructors: Arc<[Constructor]>,
    /// Which of them make it count, when its shape says it does — see
    /// [`Counting`].
    pub(crate) counting: Option<Counting>,
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
    /// The module the declaration was written in, when the declaring context
    /// named one. Stamped once at [`crate::declare`] rather than asked for
    /// again, because a group is immutable and a second answer could disagree.
    pub(crate) module: Option<ModuleId>,
    /// The package it was written in, stamped the same way and for §3's orphan
    /// rule: an `impl` is at home if it shares a package with the *declaration*
    /// of its head type, and this is where that package is recorded.
    pub(crate) package: Option<PackageId>,
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

impl Element {
    /// The family's name, for a diagnostic that has to say which type it is.
    pub(crate) fn name(&self) -> Name {
        self.group
            .family_at(self.family)
            .map_or_else(|| Arc::from("?"), |declared| Arc::clone(&declared.name))
    }

    /// The module this family's cases are private to, when `viewer` may not
    /// take one apart.
    ///
    /// Asked at a split rather than at the `match`, because the split is the
    /// case analysis. [`declare`](crate::declare) has already refused a family
    /// whose cases disagree, so the first case answers for all of them, and a
    /// family with no cases hides nothing — there is no elimination to refuse.
    pub(crate) fn abstract_from(&self, viewer: Option<ModuleId>) -> Option<ModuleId> {
        let home = self.group.module?;
        let declared = self.group.family_at(self.family)?;
        let open = declared
            .constructors
            .first()
            .is_none_or(|case| case.visibility.visible_from(self.group.module, viewer));
        (!open).then_some(home)
    }
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

/// A closed value of a counting family, as how far above the floor it stands.
///
/// The representation [`Counting`] licenses, and the reason `repeat 384` is one
/// node rather than 384. It is canonical: evaluation collapses the step
/// constructor applied to a numeral into a numeral one higher and the floor into
/// zero, so at a counting family no *value* ever holds a constructor spine and
/// conversion is [`u64`] equality. The spine reappears one level at a time, and
/// only where an elimination asks for it — [`iota`] and [`crate::case`].
///
/// Definitionally the tower it stands for, which is what makes the
/// representation a conservative extension rather than a new form of value: a
/// numeral of `n` and `n` applications of the step constructor to the floor are
/// the same value, and `suite::numeral_laws` is where that is checked rather
/// than asserted.
#[derive(Clone, Debug)]
pub struct Numeral {
    /// The family, as its own [`Role::Family`] constant. Carried rather than
    /// implied, because the core does not know which family is `Nat` and must
    /// not guess: two counting families are two types, and a numeral has to be
    /// able to say which one it inhabits.
    pub(crate) family: Constant,
    /// How many steps above the floor.
    pub(crate) count: u64,
}

/// Two numerals are the same when they count the same far at the same family.
impl PartialEq for Numeral {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && self.family == other.family
    }
}

impl Eq for Numeral {}

impl core::fmt::Display for Numeral {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "{}", self.count)
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

    /// The module this name is private to, when `viewer` may not name it.
    ///
    /// Three rules and each earns its place. A family answers for itself. A
    /// constructor is hidden by its own marker *and* by its family's, because a
    /// case of a type nobody outside can name is not reachable either way. And
    /// a recursor is hidden when the constructors are, because eliminating a
    /// family is exactly the case analysis `private` cases exist to prevent —
    /// hiding the pattern spelling while leaving `Chord.elim` in scope would
    /// hide nothing at all.
    pub(crate) fn hidden_from(&self, viewer: Option<ModuleId>) -> Option<ModuleId> {
        let (group, family) = match self {
            Self::Rigid(constant) => (&constant.group, constant.family),
            Self::Recursor(group, family) => (group, *family),
        };
        let home = group.module?;
        let declared = group.family_at(family)?;
        let sees = |visibility: Visibility| visibility.visible_from(group.module, viewer);
        // `declare` has already refused a family whose cases disagree, so the
        // first case answers for all of them.
        let sees_cases = || declared.constructors.first().is_none_or(|case| sees(case.visibility));
        let visible = sees(declared.visibility)
            && match self {
                Self::Rigid(Constant {
                    role: Role::Constructor(which),
                    ..
                }) => declared
                    .constructor_at(*which)
                    .is_some_and(|case| sees(case.visibility)),
                Self::Rigid(Constant {
                    role: Role::Recursor(_),
                    ..
                })
                | Self::Recursor(_, _) => sees_cases(),
                Self::Rigid(Constant { role: Role::Family, .. }) => true,
            };
        (!visible).then_some(home)
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

    /// Whether this is the family itself rather than one of its members.
    pub(crate) const fn is_family(&self) -> bool {
        matches!(self.role, Role::Family)
    }

    /// The cases this family declares, in declaration order.
    ///
    /// What a diagnostic lists when a name reached into the namespace and found
    /// nothing: the answer to "no such case" is the cases there are.
    pub(crate) fn cases(&self) -> Vec<Name> {
        let Some(declared) = self.group.family_at(self.family) else {
            return Vec::new();
        };
        declared
            .constructors
            .iter()
            .map(|constructor| Arc::from(format!("{}.{}", declared.name, constructor.name)))
            .collect()
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

    /// The two constructors that make this a counting family, when it is one and
    /// this constant names the family itself.
    ///
    /// Asked of the [`Role::Family`] constant rather than of a constructor,
    /// because a numeral names the type it inhabits — `384` is a `Nat`, not a
    /// `Nat.Succ`.
    pub(crate) fn counting(&self) -> Option<Counting> {
        matches!(self.role, Role::Family)
            .then(|| self.group.family_at(self.family)?.counting)
            .flatten()
    }

    /// Why this constant is *not* a counting family, as the first condition of
    /// [`Counting`]'s rule that it fails.
    ///
    /// `None` when it counts, so a caller already answered by [`Self::counting`]
    /// never reaches it. The conditions are asked in the order [`Counting`]
    /// states them, which is the order an author reads a declaration in: what
    /// the name is, what the declaration stands over, and what its cases are.
    /// Naming the first failure rather than all of them is deliberate — a
    /// declaration that fails two conditions is repaired at the first one.
    pub(crate) fn uncounted(&self) -> Option<&'static str> {
        if self.counting().is_some() {
            return None;
        }
        if !self.is_family() {
            return Some("it names a case rather than the type itself");
        }
        let Some(declared) = self.group.family_at(self.family) else {
            return Some("it is not a declared type");
        };
        if !self.group.params.is_empty() {
            return Some("its declaration takes parameters, and a count does not say what they are");
        }
        if !declared.indices.is_empty() {
            return Some("it is indexed, and a count does not say which index the value lands at");
        }
        if declared.constructors.len() != 2 {
            return Some("it does not have exactly two cases");
        }
        Some("neither case is one with no fields whose partner takes exactly one value of the same type")
    }

    /// This family's numeral for `count`, taken apart one level.
    ///
    /// `None` at zero — the floor takes no argument, so there is nothing below
    /// it — and otherwise the argument the step constructor was applied to.
    /// [`Self::case_of`] says which constructor it is either way; the two are
    /// separate because a caller that only needs to know *which* branch to take
    /// should not have to build the value under it.
    pub(crate) fn below(&self, count: u64) -> Option<Numeral> {
        count.checked_sub(1).map(|below| Numeral {
            family: self.clone(),
            count: below,
        })
    }

    /// Which constructor a numeral of `count` was built by, as its index.
    pub(crate) fn case_of(&self, counting: Counting, count: u64) -> u32 {
        if count == 0 { counting.floor } else { counting.step }
    }

    /// This constant as a value: a rigid neutral, which is what a constant is —
    /// unless it is a counting family's floor, which is the numeral zero.
    ///
    /// The collapse lives here rather than in [`crate::eval::eval`]'s arm for
    /// [`Shape::Const`] so that there is one answer to "what value is this
    /// constant": every path that turns the floor into a value gets the numeral,
    /// and no second path can produce the spine form [`Numeral`]'s canonicity
    /// says does not exist.
    pub(crate) fn value(&self, origin: Origin) -> Value {
        self.floor().map_or_else(
            || Value::neutral(Neutral::head(origin, Head::Const(self.clone()))),
            |zero| Value::new(origin, Form::Numeral(zero)),
        )
    }

    /// The numeral zero, when this constant is a counting family's floor.
    fn floor(&self) -> Option<Numeral> {
        let Role::Constructor(which) = self.role else {
            return None;
        };
        let family = Self {
            group: Arc::clone(&self.group),
            family: self.family,
            role: Role::Family,
        };
        let counting = family.counting()?;
        (which == counting.floor).then_some(Numeral { family, count: 0 })
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
        let over_params = builder.reading.clone();
        builder.extend(meter, &constructor.fields)?;
        let indices = builder.chosen(meter, &over_params, self.family, &constructor.indices)?;
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
    binders: Vec<(Plicity, Name, Term)>,
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
        self.binders.push((Plicity::Explicit, Arc::from(name), ty));
        Ok(at)
    }

    /// Introduce every binder of a stored telescope, answering where each sits
    /// and the type it stands at.
    fn extend(&mut self, meter: &mut Meter, binders: &[Binder]) -> Result<Vec<Introduced>, CoreError> {
        let mut introduced = Vec::with_capacity(binders.len());
        for binder in binders {
            let value = eval(meter, &self.reading, &binder.ty)?;
            let ty = quote_type(meter, Depth(self.depth), &value)?;
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
            args.push(quote_type(meter, Depth(self.depth), &value)?);
        }
        Ok(Plicity::Constraint(Arc::new(constraint.at(Arc::from(args)))))
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
        let over_params = self.reading.clone();
        let mut inner = self.nested();
        let fields = inner.extend(meter, &fields_of)?;
        for (field, of_family) in recursive.iter() {
            let Some((at, ty)) = fields.get(usize::try_from(*field).unwrap_or(usize::MAX)).cloned() else {
                continue;
            };
            let hypothesis = inner.hypothesis(meter, motives, *of_family, at, &ty)?;
            inner.assume(meter, "ih", hypothesis)?;
        }
        let indices = inner.chosen(meter, &over_params, family, &chooses)?;
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

    /// A constructor's chosen index arguments, re-read at the depth this
    /// telescope has reached.
    ///
    /// `over_params` is the environment the family's *index binders* were
    /// written in — the parameters alone — because index `k`'s type may mention
    /// indices `0..k`, and the values those stand at here are the ones this
    /// constructor chose.
    ///
    /// Read with [`quote`] at that type rather than with [`quote_type`], because
    /// an index is a term: `Vec A Zero` chooses a `Nat` and does not choose a
    /// type. §5.9's numeral is the value that makes the difference visible —
    /// `Zero` evaluates to one node that inhabits a type and is not one, so
    /// reading it as a type is [`Malformed::NotAType`] for a program the
    /// elaborator had just accepted.
    fn chosen(
        &self,
        meter: &mut Meter,
        over_params: &Env,
        family: u32,
        terms: &[Term],
    ) -> Result<Vec<Term>, CoreError> {
        let declared = self.group.family_at(family);
        let mut env = over_params.clone();
        let mut read = Vec::with_capacity(terms.len());
        for (position, term) in terms.iter().enumerate() {
            let value = eval(meter, &self.reading, term)?;
            let binder = declared.and_then(|declared| declared.indices.get(position));
            read.push(match binder {
                Some(binder) => {
                    let ty = eval(meter, &env, &binder.ty)?;
                    quote(meter, Depth(self.depth), &ty, &value)?
                }
                // A constructor choosing more indices than its family declares
                // is refused where it is declared, so this arm is unreachable
                // from an accepted group; reading as a type keeps it total
                // without inventing a type nothing wrote.
                None => quote_type(meter, Depth(self.depth), &value)?,
            });
            env = env.push(value);
        }
        Ok(read)
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
        self.binders
            .into_iter()
            .rev()
            .fold(result, |codomain, (plicity, name, domain)| {
                Term::function(self.origin, plicity, name, domain, codomain)
            })
    }

    /// [`Self::close`], for a caller holding the telescope by reference.
    fn finish(&mut self, result: Term) -> Term {
        std::mem::take(&mut self.binders)
            .into_iter()
            .rev()
            .fold(result, |codomain, (plicity, name, domain)| {
                Term::function(self.origin, plicity, name, domain, codomain)
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
    let Head::Const(constant) = &neutral.head else {
        return None;
    };
    let mut arguments = Vec::with_capacity(neutral.spine.len());
    for elimination in &neutral.spine {
        // A recursor is applied, never projected from and never eliminated at
        // the identity type, so anything else means this is not a reduction.
        let Elim::App { argument, .. } = elimination else {
            return None;
        };
        arguments.push(Value::clone(argument));
    }
    Some((constant.clone(), arguments))
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
/// A hypothesis whose method never names it is **not computed**. `01-surface.md`
/// §1.3's `match` compiles to nested recursors (§6.2), and a case tree does case
/// analysis rather than recursion — every method it writes binds its hypothesis
/// and ignores it. Computing one anyway makes each level of a tree cost the
/// descent twice, once for the hypothesis and once for the branch that reads the
/// same field, so a tree `d` levels deep costs `2^d`: `match n { 12 -> … }` at
/// `n = 12` spent the whole 200,000-step budget on values no method read. That
/// is Peyton Jones ch. 22's analysis with its sign reversed — not "this argument
/// is certainly needed" but "this binder is certainly absent" — and
/// [`crate::class::occurrences`] decides it exactly, so nothing is skipped that
/// a body could have named.
///
/// The saving is in *what is built*, not in what is charged: an unread
/// hypothesis costs no reduction steps because it is never reduced. §4's law is
/// untouched in the direction that matters — a program that was accepted still
/// evaluates to the same value, since the argument that changed is one no body
/// mentions — and it moves in the only safe direction for the budget, which is
/// that a program refused for exhaustion may now be accepted.
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
    for pending in hypotheses(meter, &reduction)? {
        let hypothesis = match unread(&answer) {
            Some(ignored) => ignored,
            None => pending.force(meter, here)?,
        };
        answer = apply(meter, here, answer, hypothesis)?;
    }
    Ok(Some(answer))
}

/// The value to hand a binder that is provably absent from the body it binds.
///
/// `Some` exactly when `method` is a λ whose body never names its own binder, in
/// which case β discards whatever is pushed and the cheapest thing to push is
/// what this returns. `Type 0` and not the field or a fresh variable: those are
/// values a body could plausibly have wanted, so a mistake in the analysis would
/// read as a wrong answer, while a universe standing where a proof belongs is
/// wrong in a way the next conversion says out loud.
fn unread(method: &Value) -> Option<Value> {
    let Form::Lam(closure) = &method.form else {
        return None;
    };
    // Depth one and level zero: at the top of a closure body the binder just
    // pushed is the innermost, and `occurrences` counts from the outside.
    (crate::class::occurrences(&closure.body, 1, 0) == 0)
        .then(|| Value::new(method.origin, Form::Universe(crate::level::Level::ZERO)))
}

/// The numeral a counting family's step constructor collapses to, when it has
/// been applied to a numeral of that same family.
///
/// The other half of [`Numeral`]'s canonicity: [`crate::eval::eval`] turns the
/// floor into a zero and this turns `step k` into `k + 1`, so a value at a
/// counting family is *always* a numeral and never a constructor spine. Called
/// from [`crate::eval::apply`] beside [`iota`], because both answer the same
/// question about a freshly blocked spine — whether it is blocked at all.
///
/// A count that would pass [`u64::MAX`] simply does not collapse: the spine is a
/// representation this crate already has and already reads correctly, so nothing
/// is lost by leaving one there and no error path has to exist for a case that
/// costs 2⁶⁴ steps to reach.
///
/// # Errors
///
/// As [`force`], from looking through a solved metavariable at the argument.
pub(crate) fn stepped(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Value>, CoreError> {
    let Head::Const(ref constructor) = neutral.head else {
        return Ok(None);
    };
    let Role::Constructor(which) = constructor.role else {
        return Ok(None);
    };
    let family = Constant {
        group: Arc::clone(&constructor.group),
        family: constructor.family,
        role: Role::Family,
    };
    if family.counting().is_none_or(|counting| which != counting.step) {
        return Ok(None);
    }
    let [Elim::App { ref argument, .. }] = *neutral.spine else {
        return Ok(None);
    };
    let below = force(meter, argument)?;
    let Form::Numeral(ref below) = below.as_ref().unwrap_or(argument).form else {
        return Ok(None);
    };
    if below.family != family {
        return Ok(None);
    }
    let Some(count) = below.count.checked_add(1) else {
        return Ok(None);
    };
    Ok(Some(Value::new(
        neutral.outer_origin(),
        Form::Numeral(Numeral { family, count }),
    )))
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
    let params = usize::try_from(group.params()).unwrap_or(usize::MAX);
    let (family, which, fields) = match target.form {
        // This is where the tower reappears, one level and no more: a numeral
        // knows which constructor it was built by from its count, and what that
        // constructor was applied to is the numeral one below. The level under
        // *that* is not built, which is the whole saving — a fold over `n` costs
        // what a fold over a tower of height `n` costs, and building the number
        // costs nothing.
        Form::Numeral(ref numeral) => {
            let Some(counting) = numeral.family.counting() else {
                return Ok(None);
            };
            let which = numeral.family.case_of(counting, numeral.count);
            let below = numeral
                .family
                .below(numeral.count)
                .map(|below| Value::new(target.origin, Form::Numeral(below)));
            (numeral.family.family, which, below.into_iter().collect())
        }
        Form::Neutral(ref target) => {
            let Some((constructor, built)) = spine(target) else {
                return Ok(None);
            };
            let Role::Constructor(which) = constructor.role else {
                return Ok(None);
            };
            (
                constructor.family,
                which,
                built.get(params..).unwrap_or_default().to_vec(),
            )
        }
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. }
        | Form::Refl(_)
        | Form::Lit(_) => return Ok(None),
    };
    let motives = usize::try_from(group.arity()).unwrap_or(usize::MAX);
    let position = usize::try_from(group.method_position(family, which)).unwrap_or(usize::MAX);
    let Some(method) = arguments.get(params.saturating_add(motives).saturating_add(position)) else {
        return Ok(None);
    };
    Ok(Some(Reduction {
        method: method.clone(),
        fields,
        prefix: arguments.get(..prefix_len).unwrap_or_default().to_vec(),
        group,
        family,
        which,
        level,
    }))
}

/// An induction hypothesis assembled up to its last argument.
///
/// The recursor at the parameters, motives, methods, and the field's own
/// indices — everything that builds a blocked spine and reduces nothing. The
/// *field* is held back, because applying it is the moment the descent fires,
/// and [`iota`] only wants that moment for a hypothesis some method reads.
struct Pending {
    recursor: Value,
    field: Value,
}

impl Pending {
    /// The hypothesis itself: one more application, and the recursion happens.
    fn force(self, meter: &mut Meter, here: Origin) -> Result<Value, CoreError> {
        apply(meter, here, self.recursor, self.field)
    }
}

/// One induction hypothesis per recursive field, in field order, each stopped
/// one argument short of firing.
///
/// Which fields are recursive is read off [`Constructor::recursive`] rather than
/// re-derived from the field types here, so that the method the hypothesis is
/// passed to and the hypothesis itself cannot disagree about how many arguments
/// there are: they are the same list.
fn hypotheses(meter: &mut Meter, reduction: &Reduction) -> Result<Vec<Pending>, CoreError> {
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
            built.push(Pending {
                recursor: hypothesis,
                field: field.clone(),
            });
        }
        reading = reading.push(field.clone());
    }
    Ok(built)
}

/// The constructor a saturated spine is built by, and how many of its arguments
/// are parameters rather than fields.
///
/// What §5.8's δ needs in order to see a *constructor application* as canonical
/// data: the qualified name a rule writes, and where its fields begin. The
/// parameters are excluded because they are types rather than data — `Some 3` is
/// `Option.Some Int 3`, and only the `3` is something a rule can be handed.
///
/// `None` for anything that is not a saturated constructor: a family, a
/// recursor, a variable, or a constructor one field short. Each of those is a
/// blocked spine, which is exactly what a δ-builtin over an open term should
/// stay.
pub(crate) fn constructed(neutral: &Neutral) -> Option<(Name, usize)> {
    let Head::Const(constant) = &neutral.head else {
        return None;
    };
    // A constructor's type is a Π chain, so anything but an application means
    // the spine was assembled by something other than the elaborator.
    if !neutral
        .spine
        .iter()
        .all(|elimination| matches!(*elimination, Elim::App { .. }))
    {
        return None;
    }
    saturated(constant, neutral.spine.len())
}

/// A numeral as canonical data: the family it stands at, and the count.
///
/// One node, like the numeral itself. The obvious alternative — read it back as
/// the `count` nested constructor applications it denotes — is what the old
/// representation forced and what this one exists to remove: the tree costs a
/// node per unit, and *dropping* it recurses on the host stack, so a host asking
/// for a large `Nat` as data would abort where the term never could. A rule that
/// wants the number reads [`Datum::Count`] and has it.
pub(crate) fn counted(numeral: &Numeral) -> Option<Datum> {
    numeral.family.counting()?;
    Some(Datum::Count {
        family: numeral.family.name(),
        count: numeral.count,
    })
}

/// The same question of a constant and how many arguments it was applied to.
///
/// The parameter-count rule itself, with the two readings of it above and in
/// [`canonical`] left holding only the walk that finds the spine. One rule
/// because there is one fact — where a constructor's fields begin is fixed by
/// the declaration — and two copies of it could disagree about a family whose
/// parameters changed.
fn saturated(constant: &Constant, applied: usize) -> Option<(Name, usize)> {
    if !matches!(constant.role, Role::Constructor(_)) {
        return None;
    }
    if u32::try_from(applied).unwrap_or(u32::MAX) != constant.arity() {
        return None;
    }
    Some((
        constant.name(),
        usize::try_from(constant.group.params()).unwrap_or(usize::MAX),
    ))
}

/// The canonical data a normal form denotes, or [`None`] when it denotes none.
///
/// The same name as `eval`'s private reading of a [`Value`], because it is the
/// same question: §5.8's D1 asked of a *term* rather than of a value: a literal, or a
/// constructor of a declared family applied to more of the same. It is what a
/// consumer of [`crate::check`] uses to look inside an answer that is not a bare
/// literal — a count written as a `Nat`, a list, a pair — and the shape it hands
/// back is the one a δ-rule is already written against, so a host reads one
/// vocabulary rather than two.
///
/// Takes no context and cannot fail. A constructor spine carries its own
/// [`Constant`], which carries the group that declared it, so where the fields
/// begin is already in the term; and a normal form has nothing left to compute,
/// which is what makes this a projection where
/// [`realize`] — its inverse — must be type-directed.
///
/// # What answers `None`
///
/// Everything that is not saturated canonical data, which is a longer list than
/// it sounds: a constructor one argument short or one too many, a variable, a
/// definition, a family or a recursor applied or bare, a builtin, a λ, a Π, a
/// universe, an identity type, a `refl`, a record, a record type, and a literal
/// that has somehow been applied to something. A record is on that list
/// deliberately — [`Datum`] has no record arm, and `01-surface.md`'s written
/// product reaches here as `Pair.Both` rather than as one.
///
/// Nothing here is an error, because "not data" is an ordinary answer: it is
/// exactly what a blocked δ-spine reports, and a caller that wanted a `Nat` says
/// so itself.
pub fn canonical(term: &Term) -> Option<Datum> {
    let (head, arguments) = applied_spine(term);
    match *head.shape() {
        Shape::Lit(ref literal) if arguments.is_empty() => Some(Datum::Lit(literal.clone())),
        Shape::Numeral(ref numeral) if arguments.is_empty() => counted(numeral),
        Shape::Const(ref constant) => {
            let (constructor, params) = saturated(constant, arguments.len())?;
            let fields = arguments
                .into_iter()
                .skip(params)
                .map(canonical)
                .collect::<Option<Vec<_>>>()?;
            Some(Datum::Case { constructor, fields })
        }
        // Written out rather than left to a wildcard so that the list in the
        // doc comment above is checked by the compiler: a shape added to the
        // core has to be classified here before this crate builds again.
        //
        // `App` cannot appear — the peel above ended because the head was not
        // one — and it is named anyway, because an arm that says "unreachable"
        // is a claim a later reader has to re-derive.
        Shape::Lit(_)
        | Shape::Numeral(_)
        | Shape::Var(_)
        | Shape::Def(_)
        | Shape::Base(_)
        | Shape::Builtin(_)
        | Shape::Universe(_)
        | Shape::Pi { .. }
        | Shape::Lam { .. }
        | Shape::App { .. }
        | Shape::RecordType(_)
        | Shape::Record(_)
        | Shape::Project { .. }
        | Shape::Id { .. }
        | Shape::Refl(_)
        | Shape::J { .. }
        | Shape::Meta(_)
        | Shape::Let { .. } => None,
    }
}

/// A term as its head and the arguments applied to it, outermost last.
///
/// `Shape::App` nests to the left, so peeling collects the arguments backwards
/// and this puts them in written order — which is the order a constructor's
/// parameters precede its fields in, and therefore the only order `skip` above
/// means anything in.
fn applied_spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut head = term;
    let mut arguments = Vec::new();
    while let Shape::App {
        ref function,
        ref argument,
    } = *head.shape()
    {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}

/// Canonical data as a value of `ty`.
///
/// The inverse of [`constructed`], and the reason it needs a type where reading
/// needed none. A [`Datum::Case`] says `Option.Some 3` and a constructor *value*
/// is `Option.Some Int 3`: the parameters come before the fields, ι reads them
/// by position, and nothing in the name or the fields says what they are. `ty`
/// is where they come from — for a δ-rule's answer it is the builtin's own
/// signature applied to the argument values, so nothing ambient is consulted.
///
/// The walk is `case.rs`'s split in reverse. At `F p⃗` the answer names one of
/// `F`'s constructors, the parameters are `p⃗`, and each field is realized at its
/// declared type read in the declaration context, then the parameters, then the
/// fields before it — the same environment a split assumes its fields in. So a
/// nested family resolves against the type in hand at that node rather than
/// against a table, which is why two occurrences of one family in a signature
/// cannot be ambiguous about anything.
///
/// # Errors
///
/// [`Malformed::MisfitAnswer`] when the data and the type disagree, otherwise as
/// [`eval`].
pub(crate) fn realize(meter: &mut Meter, here: Origin, datum: &Datum, ty: &Value) -> Result<Value, CoreError> {
    meter.nested("data realization", |meter| match *datum {
        Datum::Lit(ref literal) => Ok(Value::new(here, Form::Lit(literal.clone()))),
        Datum::Count { ref family, count } => realize_count(meter, here, family, count, ty),
        Datum::Case {
            ref constructor,
            ref fields,
        } => realize_case(meter, here, constructor, fields, ty),
    })
}

/// A count, at the counting family the answer's type says it stands at.
///
/// The type decides, and the name the rule wrote is checked against it rather
/// than trusted: a rule that answered `Nat` where the signature promised some
/// other counting family would otherwise build a value at the wrong type, and
/// the two are indistinguishable once the count is all that is left.
fn realize_count(meter: &mut Meter, here: Origin, family: &Name, count: u64, ty: &Value) -> Result<Value, CoreError> {
    let misfit = || CoreError::from(Malformed::MisfitAnswer(Arc::clone(family)));
    let Some(element) = element(meter, ty)? else {
        return Err(misfit());
    };
    let constant = Constant::family(&element.group, element.family);
    if constant.counting().is_none() || *constant.name() != **family {
        return Err(misfit());
    }
    Ok(Value::new(
        here,
        Form::Numeral(Numeral {
            family: constant,
            count,
        }),
    ))
}

/// One constructor application, at the family type it stands at.
fn realize_case(
    meter: &mut Meter,
    here: Origin,
    constructor: &Name,
    fields: &[Datum],
    ty: &Value,
) -> Result<Value, CoreError> {
    let misfit = || CoreError::from(Malformed::MisfitAnswer(Arc::clone(constructor)));
    let Some(element) = element(meter, ty)? else {
        return Err(misfit());
    };
    let Some(declared) = element.group.family_at(element.family) else {
        return Err(misfit());
    };
    // Qualified, the spelling `Constant::name` prints and `Found::named` reads,
    // so the name a rule writes is the name a diagnostic would have shown it.
    let Some(case) = constructor
        .strip_prefix(&*declared.name)
        .and_then(|rest| rest.strip_prefix('.'))
    else {
        return Err(misfit());
    };
    let Some(which) = declared
        .constructors
        .iter()
        .position(|declared| *declared.name == *case)
    else {
        return Err(misfit());
    };
    let Some(rule) = declared.constructor_at(u32::try_from(which).unwrap_or(u32::MAX)) else {
        return Err(misfit());
    };
    if rule.fields.len() != fields.len() {
        return Err(misfit());
    }

    let mut reading = Group::declarations(&element.group);
    for param in &element.params {
        reading = reading.push(param.clone());
    }
    let mut value =
        Constant::constructor(&element.group, element.family, u32::try_from(which).unwrap_or(u32::MAX)).value(here);
    for param in &element.params {
        value = apply(meter, here, value, param.clone())?;
    }
    for (binder, field) in rule.fields.iter().zip(fields) {
        let field_type = eval(meter, &reading, &binder.ty)?;
        let built = realize(meter, here, field, &field_type)?;
        reading = reading.push(built.clone());
        value = apply(meter, here, value, built)?;
    }
    Ok(value)
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
