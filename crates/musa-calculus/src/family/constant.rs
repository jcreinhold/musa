//! A name a group introduces — a family, a constructor, a recursor, a
//! numeral — and the type it stands at.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::assemble::{Telescope, applied};
use super::group::{Counting, Group, Role};
use crate::budget::Meter;
use crate::error::CoreError;
use crate::eval::eval;
use crate::origin::Origin;
use crate::sort::Sort;
use crate::term::{Name, Shape, Term};
use crate::value::{Env, Form, Head, Neutral, Value};
use crate::visibility::{ModuleId, Visibility};
use std::sync::Arc;

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
            (Role::Recursor(one), Role::Recursor(two)) => one == two,
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
    pub(crate) fn at(self, level: Sort) -> Constant {
        match self {
            Self::Rigid(constant) => constant,
            Self::Recursor(group, family) => Constant {
                group,
                family,
                role: Role::Recursor(level),
            },
        }
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
    pub(crate) fn recursor(group: &Arc<Group>, family: u32, level: Sort) -> Self {
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

    /// The index this family is declared to carry, if it declares one (§1.5).
    ///
    /// Asked of the [`Role::Family`] constant for [`Self::counting`]'s reason:
    /// an index sits on the *type* — `Pc(12)` — and a constructor of it carries
    /// no index, because [`Shape::Indexed`](crate::Shape::Indexed) is a wrapper
    /// and a value of `Pc(12)` is a value of `Pc`.
    pub(crate) fn declared_index(&self) -> Option<&super::group::Parameter> {
        matches!(self.role, Role::Family)
            .then(|| self.group.family_at(self.family)?.index.as_ref())
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
        if declared.constructors.len() != 2 {
            return Some("it does not have exactly two cases");
        }
        Some("neither case is one with no fields whose partner takes exactly one value of the same type")
    }

    /// This family's numeral for `count`, taken apart one level.
    ///
    /// `None` at zero — the floor takes no argument, so there is nothing below
    /// it — and otherwise the argument the step constructor was applied to.
    /// [`Counting::case_of`] says which constructor it is either way; the two
    /// are separate because a caller that only needs to know *which* branch to
    /// take should not have to build the value under it.
    pub(crate) fn below(&self, count: u64) -> Option<Numeral> {
        count.checked_sub(1).map(|below| Numeral {
            family: self.clone(),
            count: below,
        })
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
    pub(super) fn floor(&self) -> Option<Numeral> {
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
    /// A family takes its parameters, a constructor takes the parameters and
    /// its fields, and a recursor takes everything up to and including the
    /// target. ι fires exactly at this count on a recursor, which is why it is
    /// one number rather than a shape match at every application.
    pub(crate) fn arity(&self) -> u32 {
        let params = self.group.params();
        match &self.role {
            Role::Family => params,
            Role::Constructor(which) => {
                let fields = self.group.family_at(self.family).and_then(|declared| {
                    declared
                        .constructor_at(*which)
                        .map(|constructor| u32::try_from(constructor.fields.len()).unwrap_or(u32::MAX))
                });
                params.saturating_add(fields.unwrap_or(0))
            }
            Role::Recursor(_) => params
                .saturating_add(self.group.arity())
                .saturating_add(self.group.methods())
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
            Role::Recursor(level) => self.recursor_type(meter, builder, *level),
        }
    }

    /// `(p⃗ : Params) → (i⃗ : Indices) → Type l`.
    fn family_type(&self, meter: &mut Meter, builder: &mut Telescope<'_>) -> Result<Term, CoreError> {
        let Some(_declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        builder.extend(meter, &self.group.params)?;
        // §1: a data family stores small types, so it lands at `Type 0`; the
        // declaration check is what makes that a theorem rather than a hope.
        Ok(builder.finish(Term::universe(self.group.origin, Sort::ZERO)))
    }

    /// `(p⃗ : Params) → (a⃗ : Fields) → N p⃗ idx⃗`.
    fn constructor_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, which: u32) -> Result<Term, CoreError> {
        let Some(constructor) = self
            .group
            .family_at(self.family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let params = builder.extend(meter, &self.group.params)?;
        builder.extend(meter, &constructor.fields)?;
        let result = applied(
            self.group.origin,
            builder.family(self.family),
            builder.references(&params),
        );
        Ok(builder.close(result))
    }

    /// The recursor's type, at the universe its motives land in.
    fn recursor_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, level: Sort) -> Result<Term, CoreError> {
        let Some(_declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let params = builder.extend(meter, &self.group.params)?;
        let motives = builder.motives(meter, level)?;
        builder.methods(meter, &motives)?;
        let subject = builder.applied_family(self.family, [&params, &[]]);
        builder.assume(meter, "t", subject)?;
        // Non-dependent: the result is the answer type itself, with nothing
        // applied to the value being eliminated.
        let result = builder.reference(motives.get(usize::try_from(self.family).unwrap_or(usize::MAX)).copied());
        Ok(builder.close(result))
    }
}
