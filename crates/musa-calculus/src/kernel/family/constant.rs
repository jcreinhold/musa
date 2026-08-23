//! A name a group introduces — a family, a constructor, a recursor, a
//! numeral — and the type it stands at.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::assemble::{Telescope, applied};
use super::group::{Counting, Group, Role};
use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::CoreError;
use crate::kernel::eval::eval;
use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Name, Term};
use crate::kernel::value::{Env, Form, Head, Neutral, Value};
use crate::kernel::visibility::{ModuleId, Visibility};
use std::sync::Arc;

/// A declared constant: a family, a constructor, or a recursor.
///
/// Holds the whole group rather than a name, which is what lets evaluation fire ι
/// and quotation recover a type without a signature threaded through every
/// operation in the crate — `AGENTS.md`'s "hand a consumer what we already
/// computed", applied to the one place it would otherwise cost a parameter on
/// `eval`, `quote`, `apply`, and `Cx` alike.
#[derive(Clone, Debug)]
pub(crate) struct Constant {
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
            (Role::Family, Role::Family)
            | (Role::Constructor(_), Role::Constructor(_))
            | (Role::Projection(_), Role::Projection(_)) => true,
            (Role::Family | Role::Constructor(_) | Role::Recursor(_) | Role::Projection(_), _) => false,
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
/// only where an elimination asks for it — [`iota`] and [`crate::elaboration::case`].
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
            if let Some(which) = declared
                .constructors
                .iter()
                .position(|constructor| *constructor.name == *member)
            {
                return Some(Self::Rigid(Constant {
                    group: Arc::clone(group),
                    family,
                    role: Role::Constructor(u32::try_from(which).unwrap_or(u32::MAX)),
                }));
            }
            // A field, and only where the family projects: `Pending.read` is a
            // name a program may write (§1.2), and it is looked for *after* the
            // cases so that a family whose one case is spelled like a field
            // still answers with the case.
            let field = group
                .projects(family)?
                .iter()
                .position(|declared| *declared.name == *member)?;
            return Some(Self::Rigid(Constant {
                group: Arc::clone(group),
                family,
                role: Role::Projection(u32::try_from(field).unwrap_or(u32::MAX)),
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
                // A projection reads a field, which is exactly what a private
                // case hides, so it is hidden with the cases for the recursor's
                // reason: leaving it in scope would leave the type transparent.
                Self::Rigid(Constant {
                    role: Role::Projection(_),
                    ..
                }) => sees_cases(),
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

    /// A family's generated accessor for its `field`th field.
    ///
    /// Only meaningful where [`Group::projects`](super::Group::projects) says
    /// the family has one; a caller that mints one anywhere else gets a
    /// constant whose type is `Type 0` and whose ι rule never fires, which is
    /// the same defensive answer every other assembly here gives.
    pub(crate) fn projection(group: &Arc<Group>, family: u32, field: u32) -> Self {
        Self {
            group: Arc::clone(group),
            family,
            role: Role::Projection(field),
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
            Role::Projection(field) => match self
                .group
                .projects(self.family)
                .and_then(|fields| fields.get(usize::try_from(*field).unwrap_or(usize::MAX)))
            {
                Some(declared_field) => Arc::from(format!("{}.{}", declared.name, declared_field.name)),
                None => Arc::from("?"),
            },
        }
    }

    /// This constant as a term: its name, at the role it plays.
    ///
    /// The declaration stays here and the term takes the name (§6). Which of
    /// the three constants this is survives as [`Role`](crate::Role), because
    /// the spellings already differ — `Vec`, `Vec.Cons`, `Vec.elim` — and the
    /// role is what a reader of the term can act on without a context.
    pub(crate) fn term(&self, origin: Origin) -> Term {
        Term::named_at(origin, self.name(), self.written_role(), self.written_levels())
    }

    /// Which [`Role`](crate::Role) a term naming this constant carries.
    fn written_role(&self) -> crate::kernel::term::Role {
        match &self.role {
            Role::Family => crate::kernel::term::Role::TypeConstructor,
            Role::Constructor(_) => crate::kernel::term::Role::Constructor,
            Role::Recursor(_) => crate::kernel::term::Role::Recursor,
            Role::Projection(_) => crate::kernel::term::Role::Projection,
        }
    }

    /// The level arguments a term naming this constant carries.
    ///
    /// A recursor's one level parameter is the universe its motives land in, so
    /// a term naming one says which universe this elimination is into; a family
    /// and a constructor are monomorphic and say nothing.
    fn written_levels(&self) -> crate::kernel::sort::Levels {
        match &self.role {
            Role::Family | Role::Constructor(_) | Role::Projection(_) => crate::kernel::sort::Levels::NONE,
            Role::Recursor(level) => crate::kernel::sort::Levels::of([level.clone()]),
        }
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
            return Some("its declaration takes indices, and a count does not say which one it stands at");
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
    /// The collapse lives here rather than in [`crate::kernel::eval::eval`]'s arm for
    /// a name so that there is one answer to "what value is this constant":
    /// every path that turns the floor into a value gets the numeral, and no
    /// second path can produce the spine form [`Numeral`]'s canonicity says does
    /// not exist.
    ///
    /// `globals` is the table the name was resolved under, which the rigid head
    /// keeps so that [`crate::kernel::eval::neutral_type`] can read this constant's own
    /// type — see [`Head::Base`](crate::kernel::value::Head::Base).
    pub(crate) fn value(&self, origin: Origin, globals: &Globals) -> Value {
        self.floor().map_or_else(
            || Value::neutral(Neutral::head(origin, Head::Const(self.clone(), globals.clone()))),
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
    /// A family takes its parameters and its indices, a constructor takes the
    /// parameters and its fields — never the indices, which it *chose* rather
    /// than being handed — and a recursor takes everything up to and including
    /// the target, with the eliminated value's indices standing just before it.
    /// ι fires exactly at this count on a recursor, which is why it is one
    /// number rather than a shape match at every application.
    pub(crate) fn arity(&self) -> u32 {
        let params = self.group.params();
        let indices = self.group.indices(self.family);
        match &self.role {
            Role::Family => params.saturating_add(indices),
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
                .saturating_add(indices)
                .saturating_add(1),
            // The parameters and the value read from. No indices: a family that
            // takes any does not project (`Group::projects`).
            Role::Projection(_) => params.saturating_add(1),
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
    pub(crate) fn ty(&self, meter: &mut Meter, globals: &Globals) -> Result<Value, CoreError> {
        let term = self.ty_term(meter, globals)?;
        eval(meter, &Env::under(globals.clone()), &term)
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
    pub(crate) fn ty_term(&self, meter: &mut Meter, globals: &Globals) -> Result<Term, CoreError> {
        let mut builder = Telescope::new(&self.group, globals);
        match &self.role {
            Role::Family => self.family_type(meter, &mut builder),
            Role::Constructor(which) => self.constructor_type(meter, builder, *which),
            Role::Recursor(level) => self.recursor_type(meter, builder, level),
            Role::Projection(field) => self.projection_type(meter, builder, *field),
        }
    }

    /// `(p⃗ : Params) → (i⃗ : Indices) → Type l`.
    fn family_type(&self, meter: &mut Meter, builder: &mut Telescope<'_>) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let indices = Arc::clone(&declared.indices);
        builder.extend(meter, &self.group.params)?;
        builder.extend(meter, &indices)?;
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
        let fields = Arc::clone(&constructor.fields);
        let params = builder.extend(meter, &self.group.params)?;
        builder.extend(meter, &fields)?;
        // The result is `N p⃗` at the indices *this* constructor chose, which is
        // the one thing a constructor writes about its own result type and the
        // whole of what an index is (§1.1).
        let chosen = builder.chosen(meter, &params, self.family, which)?;
        let result = applied(
            self.group.origin,
            builder.family(self.family),
            builder.references(&params).into_iter().chain(chosen),
        );
        Ok(builder.close(result))
    }

    /// `(p⃗ : Params) → (t : N p⃗) → A_i`, the type of a generated field
    /// accessor.
    ///
    /// **The result type is the field's, and a field's type may name the fields
    /// before it.** `record Frame { n: Nat; held: Vect<Nat>(n); }` writes
    /// `held`'s type over `n`, and an accessor taking the whole frame has no
    /// `n` to put there — it has `Frame.n t`. So each earlier field is bound,
    /// while field `i`'s stored type is read, to that field's own accessor
    /// applied to this one's subject. Where no field type names an earlier one,
    /// every one of those bindings goes unused and the result is the plain
    /// field type, which is the ordinary case.
    ///
    /// This is the same eval-then-quote the rest of the module weakens by:
    /// nothing is shifted, and the accessor terms are built at the depth the
    /// walk has reached rather than re-indexed afterwards.
    fn projection_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, field: u32) -> Result<Term, CoreError> {
        let Some(fields) = self.group.projects(self.family) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let fields = Arc::clone(fields);
        let Some(declared) = fields.get(usize::try_from(field).unwrap_or(usize::MAX)) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let params = builder.extend(meter, &self.group.params)?;
        // No indices: `Group::projects` refused a family that takes any, which
        // is why the subject's type is the family at its parameters alone.
        let subject = builder.applied_family(self.family, [&params, &[]]);
        let target = builder.assume(meter, "t", subject)?;
        for earlier in 0..field {
            let read = applied(
                self.group.origin,
                Self::projection(&self.group, self.family, earlier).term(self.group.origin),
                builder
                    .references(&params)
                    .into_iter()
                    .chain([builder.reference(Some(target))]),
            );
            let value = eval(meter, builder.under(), &read)?;
            builder.standing(value);
        }
        let ty = eval(meter, builder.reading_env(), &declared.ty)?;
        let result = crate::kernel::quote::quote_type(meter, builder.reached(), crate::kernel::quote::Mode::Open, &ty)?;
        Ok(builder.close(result))
    }

    /// The recursor's type, at the universe its motives land in.
    fn recursor_type(&self, meter: &mut Meter, mut builder: Telescope<'_>, level: &Sort) -> Result<Term, CoreError> {
        let Some(declared) = self.group.family_at(self.family) else {
            return Ok(Term::universe(self.group.origin, Sort::ZERO));
        };
        let indices_of = Arc::clone(&declared.indices);
        let params = builder.extend(meter, &self.group.params)?;
        let motives = builder.motives(meter, &params, level)?;
        builder.methods(meter, &motives, &params)?;
        // The eliminated value's indices stand between the methods and the
        // target, because the target's *type* names them: `elim … (i⃗) (t : N p⃗
        // i⃗)`. A recursor that took them before the motives could not state
        // that type at all.
        let indices = builder.extend(meter, &indices_of)?;
        let subject = builder.applied_family(self.family, [&params, &indices]);
        let target = builder.assume(meter, "t", subject)?;
        // Dependent (§1.1): the result is the motive *applied to* the value
        // being eliminated, which is what makes an elimination able to say
        // something about its subject rather than merely produce a value.
        let motive = builder.reference(motives.get(usize::try_from(self.family).unwrap_or(usize::MAX)).copied());
        let at_indices = applied(self.group.origin, motive, builder.references(&indices));
        let result = Term::app(self.group.origin, at_indices, builder.reference(Some(target)));
        Ok(builder.close(result))
    }
}
