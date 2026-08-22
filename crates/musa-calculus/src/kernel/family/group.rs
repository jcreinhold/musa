//! What a declaration group is: its families, their constructors, and the
//! binders they are declared with.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::Constant;
use super::iota::spine;
use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::CoreError;
use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Filling, Name, Term};
use crate::kernel::value::{Env, Form, Value};
use crate::kernel::visibility::{ModuleId, Visibility};
use std::sync::Arc;

/// One parameter of a telescope: a name, the type it stands at, and how a use
/// site fills it.
///
/// Not `Binder`, which [`crate::kernel::term::Binder`] is: that one says which of the
/// three ways a term binds, and this one is a position in a declaration's
/// telescope. Two things a reader would mix up if they shared a spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parameter {
    /// Its written name, for diagnostics and for what quotation prints.
    pub name: Name,
    /// Its type, read under the declaration context and the binders before it.
    pub ty: Term,
    /// How a use site supplies its argument.
    ///
    /// [`Filling::Written`] for everything a declaration writes — §1.1's
    /// parameters are written at every use, which is what `List<Nat>` is.
    /// [`Filling::Constraint`] appears here only for `Storable`, which is
    /// discharged by computation rather than written; see
    /// [`crate::elaboration::storable::discharge`].
    pub filling: Filling,
}

impl Parameter {
    /// A parameter a use site writes: `(name : ty)`.
    #[must_use]
    pub(crate) fn written(name: Name, ty: Term) -> Self {
        Self {
            name,
            ty,
            filling: Filling::Written,
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
    pub(crate) fields: Arc<[Parameter]>,
    /// The indices its result chooses, in the family's index order, read under
    /// the declaration context, the group's parameters, and its own fields.
    ///
    /// This is the whole of what an index adds (§1.1): a parameter is fixed
    /// across the declaration and is not written here, while an index is
    /// *chosen* per constructor and is. Empty for a family with no indices,
    /// which is every family the language had before prompt 156, so the old
    /// representation is the `chosen: []` case rather than a second shape.
    ///
    /// Read under the fields because a chosen index may name them — `Cons(head:
    /// A, tail: Vec<A>(n)) : Vec<A>(n + 1)` names `n`, which the fields bound.
    pub(crate) chosen: Arc<[Term]>,
    /// Which fields are recursive occurrences, as `(field, family)` pairs in
    /// field order.
    ///
    /// Computed once by the positivity check rather than recovered at every
    /// reduction: ι needs exactly this list to know which arguments take an
    /// induction hypothesis.
    ///
    /// **Direct occurrences only.** A field that merely *contains* the family —
    /// `items : List StaffRead` — is an ordinary field and appears nowhere here,
    /// so the generated method takes it and no hypothesis. §1.1 promises the
    /// recursor exists and does not promise a hypothesis per field, and giving
    /// none is what keeps this module's standing invariant exactly: a hypothesis
    /// stays an application rather than a synthesized closure.
    pub(crate) recursive: Arc<[(u32, u32)]>,
}

/// The two constructors that make a family *count*.
///
/// A family counts when a closed value of it is completely described by how many
/// steps it stands above a floor: the group takes no parameters, the family takes
/// no indices, and it has exactly two constructors, one with no fields and one
/// with a single field that is a recursive occurrence of the family itself. That
/// sentence is both the recognition rule and the soundness argument for
/// [`Constant::Numeral`](crate::kernel::term::Constant::Numeral), which is why it is one
/// sentence and not two.
///
/// Derived at [`crate::elaboration::declare`] from the declaration's own shape rather than
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

impl Counting {
    /// Which constructor a numeral of `count` was built by, as its index.
    pub(crate) const fn case_of(self, count: u64) -> u32 {
        if count == 0 { self.floor } else { self.step }
    }
}

/// One family of a declaration group.
#[derive(Debug)]
pub struct Declared {
    /// Its name.
    pub(crate) name: Name,
    /// The universe it lands in: the join of what its constructors store.
    ///
    /// Inferred rather than fixed. The predecessor of this field was a *check*
    /// that every field was small, because §1 had two universes and a family
    /// above the first had nowhere to go; prompt 152's hierarchy has no
    /// ceiling, so what the join says is what the family's signature says.
    pub(crate) level: Sort,
    /// Whether the *type* may be named outside the module its group was
    /// declared in. Independent of its constructors': `01-surface.md` §1.3's
    /// whole point is a public type whose cases are package-maintained.
    pub(crate) visibility: Visibility,
    /// Its indices: the telescope written after the `:` of its declaration,
    /// read under the declaration context and the group's parameters.
    ///
    /// Per family rather than per group, which is the difference §1.1 turns on:
    /// the group's [`Group::params`] are the same in every constructor's result,
    /// and these are what each constructor chooses a value at. A mutual group
    /// may pair a family with indices and one without.
    pub(crate) indices: Arc<[Parameter]>,
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
/// makes the mutual recursor statable at all: it carries one motive per family
/// at the *same* parameters the eliminated `N_i` was taken at.
#[derive(Debug)]
pub struct Group {
    /// Where the declaration was written. Every term generated from it says so,
    /// which is §7's preservation clause applied to syntax nobody wrote.
    pub(crate) origin: Origin,
    /// The parameters, read under the declaration context.
    pub(crate) params: Arc<[Parameter]>,
    /// Whether each parameter occurs only strictly positively in the fields this
    /// group stores, in parameter order.
    ///
    /// Read by a *later* declaration's positivity check, which is the whole
    /// reason it is stored: `data StaffRead { Body(items: List<StaffRead>) }` is
    /// admitted exactly when `List` is positive in its element, and that is a
    /// question about `List`'s declaration rather than about this field. Answered
    /// once, where the constructors are in hand, for
    /// [`Constructor::recursive`]'s reason — a check and its readers that
    /// re-derive the same fact can disagree about it.
    pub(crate) positive: Arc<[bool]>,
    /// The families, in declaration order.
    pub(crate) families: Arc<[Declared]>,
    /// The module the declaration was written in, when the declaring context
    /// named one. Stamped once at [`crate::elaboration::declare`] rather than asked for
    /// again, because a group is immutable and a second answer could disagree.
    pub(crate) module: Option<ModuleId>,
}

/// A type that turned out to be a family applied to its arguments.
///
/// What splitting a `match` subject needs: which family, at which parameters,
/// and at which indices. The two argument lists are kept apart rather than
/// concatenated because a split does different things with them — the
/// parameters are passed through to every constructor unchanged, and the
/// indices are *unified* against what the constructor chose, which is what
/// refinement is and what rules a branch impossible.
pub(crate) struct Element {
    pub(crate) group: Arc<Group>,
    pub(crate) family: u32,
    pub(crate) params: Vec<Value>,
    /// The index arguments, in the family's index order. Empty for a family
    /// that takes none.
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
    /// case analysis. [`declare`](crate::elaboration::declare) has already refused a family
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
/// As [`opened`](crate::kernel::eval::opened), from unfolding the type far enough to
/// see its head.
pub(crate) fn element(meter: &mut Meter, ty: &Value) -> Result<Option<Element>, CoreError> {
    let ty = crate::kernel::eval::opened(meter, ty)?.unwrap_or_else(|| ty.clone());
    let Form::Neutral(neutral) = &ty.form else {
        return Ok(None);
    };
    let Some((constant, arguments)) = spine(neutral) else {
        return Ok(None);
    };
    let Role::Family = constant.role else {
        return Ok(None);
    };
    let params = usize::try_from(constant.group.params()).unwrap_or(usize::MAX);
    let indices = usize::try_from(constant.group.indices(constant.family)).unwrap_or(usize::MAX);
    if arguments.len() != params.saturating_add(indices) {
        return Ok(None);
    }
    let mut arguments = arguments;
    let chosen = arguments.split_off(params.min(arguments.len()));
    Ok(Some(Element {
        group: Arc::clone(&constant.group),
        family: constant.family,
        params: arguments,
        indices: chosen,
    }))
}

/// Which constructor stands at the head of a value, where one does.
///
/// `(family, which)` for a constructor spine, and for a counting family's
/// numeral too: a numeral is *definitionally* the tower it counts, so zero is
/// the floor's case and anything above it the step's, and a caller that asks
/// "which constructor is this" must get the same answer either way or the
/// representation would stop being a conservative extension.
///
/// `None` for everything that is not a constructor at all — a variable, a
/// metavariable, a stuck elimination, a λ, a record. The companion of
/// [`element`], which reads a *type* of a family; this reads a *value* of one.
pub(crate) fn built_by(meter: &mut Meter, value: &Value) -> Result<Option<(u32, u32)>, CoreError> {
    let value = crate::kernel::eval::opened(meter, value)?.unwrap_or_else(|| value.clone());
    match &value.form {
        Form::Numeral(numeral) => Ok(numeral
            .family
            .counting()
            .map(|counting| (numeral.family.family, counting.case_of(numeral.count)))),
        Form::Neutral(neutral) => {
            let Some((constant, _)) = spine(neutral) else {
                return Ok(None);
            };
            match constant.role {
                Role::Constructor(which) => Ok(Some((constant.family, which))),
                Role::Family | Role::Recursor(_) => Ok(None),
            }
        }
        Form::Lam(_) | Form::Pi { .. } | Form::Universe(_) | Form::Record(_) | Form::RecordType(_) | Form::Lit(_) => {
            Ok(None)
        }
    }
}

/// Which constructor built a value, **and what it was applied to**.
///
/// [`built_by`] answers the first half and is what a refutation needs; this
/// answers both, and is what *learning* from an index needs. Splitting a
/// subject whose index stands at `suc n` against the `Succ` case has to put the
/// method's own field where `n` stood, and that substitution cannot be made
/// without the arguments the index constructor was applied to.
///
/// The fields alone, with the parameters dropped: a parameter is fixed across
/// the declaration (§1.1) and carries nothing a match could learn. A counting
/// family's numeral answers the same way its tower would, one predecessor for
/// the step case and nothing for the floor, so that the representation stays
/// the conservative extension [`built_by`] describes.
pub(crate) fn built_from(meter: &mut Meter, value: &Value) -> Result<Option<(u32, u32, Vec<Value>)>, CoreError> {
    let value = crate::kernel::eval::opened(meter, value)?.unwrap_or_else(|| value.clone());
    match &value.form {
        Form::Numeral(numeral) => {
            let Some(counting) = numeral.family.counting() else {
                return Ok(None);
            };
            let below = numeral
                .family
                .below(numeral.count)
                .map(|below| Value::new(value.origin, Form::Numeral(below)));
            Ok(Some((
                numeral.family.family,
                counting.case_of(numeral.count),
                below.into_iter().collect(),
            )))
        }
        Form::Neutral(neutral) => {
            let Some((constant, arguments)) = spine(neutral) else {
                return Ok(None);
            };
            match constant.role {
                Role::Constructor(which) => {
                    let params = usize::try_from(constant.group.params()).unwrap_or(usize::MAX);
                    let mut arguments = arguments;
                    let fields = arguments.split_off(params.min(arguments.len()));
                    Ok(Some((constant.family, which, fields)))
                }
                Role::Family | Role::Recursor(_) => Ok(None),
            }
        }
        Form::Lam(_) | Form::Pi { .. } | Form::Universe(_) | Form::Record(_) | Form::RecordType(_) | Form::Lit(_) => {
            Ok(None)
        }
    }
}

/// Which of a declaration's three constants this is.
#[derive(Clone, Debug)]
pub(crate) enum Role {
    /// The family itself, `N p⃗`.
    Family,
    /// One of its constructors.
    Constructor(u32),
    /// Its generated recursor, at the universe its motive lands in.
    ///
    /// The level rides on the *use site* rather than on the declaration: a
    /// family is declared once, and an elimination's goal is a type at `Type 0`
    /// or a type of types at `Type 1` — the use site knows which, and the
    /// declaration does not.
    Recursor(Sort),
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

    /// How many indices family `which` takes, and zero for a family that is not
    /// in this group.
    ///
    /// Zero for an absent family rather than an error, because every caller is
    /// asking in order to *count arguments*, and a family that is not there
    /// contributes none — the caller that cares which family it is has already
    /// asked [`Self::family_at`].
    pub(crate) fn indices(&self, which: u32) -> u32 {
        self.family_at(which)
            .map_or(0, |declared| u32::try_from(declared.indices.len()).unwrap_or(u32::MAX))
    }

    /// Whether a later declaration may put itself at parameter `which`.
    ///
    /// [`Self::positive`], read defensively: a position past the parameters is
    /// no position at all, since §1.1 admits none, and `false` is the answer
    /// that refuses a declaration rather than admitting one on a guess.
    pub(crate) fn positive_at(&self, which: usize) -> bool {
        self.positive.get(which).copied().unwrap_or(false)
    }

    /// How many methods a recursor over this group takes: one per constructor of
    /// every family in it.
    pub(super) fn methods(&self) -> u32 {
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
    pub(crate) fn declarations(group: &Arc<Self>, globals: &Globals) -> Env {
        let mut env = Env::under(globals.clone());
        for family in 0..group.arity() {
            env = env.push(
                Constant {
                    group: Arc::clone(group),
                    family,
                    role: Role::Family,
                }
                .value(group.origin, globals),
            );
        }
        env
    }

    /// Where the method handling family `family`'s `constructor`th constructor
    /// sits among the recursor's methods.
    pub(super) fn method_position(&self, family: u32, constructor: u32) -> u32 {
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
}
