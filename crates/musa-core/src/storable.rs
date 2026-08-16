//! `02-core-calculus.md` §1.2: storability as a constraint nobody writes.
//!
//! Storability is a *structural fact about a type*, not a claim an author may
//! assert, so this module is the whole of `Storable`: the trait itself, which
//! every context has in scope before anything is declared, and the instances
//! generated from a declaration group. There is no path from source to an
//! `impl Storable` — [`declare_trait`](crate::declare_trait) refuses the name
//! and [`declare_impl`](crate::declare_impl) refuses the head — which is what
//! makes §1.2's exception an exception in the safe direction: the set of
//! instances is smaller than an author could produce and never larger.
//!
//! # The evidence is the check, so the dictionary is empty
//!
//! `Storable` declares no method. Its dictionary type is `{}` and its value is
//! `{}`, and nothing projects out of one. That is not a placeholder for an
//! encoding to be added later: the encoding is a property of the *declaration*,
//! decided once here, and a field an author could read would be a field an
//! author could supply. What the constraint carries is the right to have asked;
//! the answer is that the instance exists.
//!
//! # Two questions per stored field, not one
//!
//! §1.2 says a family is storable when every field type it stores is, that an
//! arrow is never storable, and that neither is any container holding one *at
//! any depth*. Those are two different questions about one term, and answering
//! them with a single walk is what makes `Vec A n` come out wrong:
//!
//! - The field type's **head** decides which instance would answer it, so it has
//!   to be a family: one of this group's, one already declared and storable, or
//!   a record type whose own fields answer the same two questions. A field whose
//!   type is a universe, or is an earlier field's value, has no encoding at all.
//! - The field type's **arguments** are not all types. `Vec A n` stores `A`s and
//!   is indexed by a number, and asking whether `n` is storable is a category
//!   error. So arguments are only scanned for an arrow or a universe — that is
//!   the at-any-depth rule — and for which of the group's *type* parameters
//!   occur, which is what the generated instance's context constrains.
//!
//! # Why the generated instances terminate without being measured
//!
//! §4's measure refuses an instance whose context is not smaller than its head.
//! Every context generated here is `Storable p` for a parameter `p` of the head
//! `N p⃗ i⃗`, which is a variable inside an application and so strictly smaller by
//! that measure. Running the measure over these would be checking an arithmetic
//! identity at runtime.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use crate::class::{Classes, Constraint, Instance, Key, Trait};
use crate::family::{Binder, Constant, Group};
use crate::level::Level;
use crate::origin::Origin;
use crate::term::{Index, Name, Shape, Term};

/// The one trait whose instances this crate generates and no author may write.
pub(crate) const STORABLE: &str = "Storable";

/// The `Storable` trait: one type parameter, no supers, no methods.
///
/// Built once and shared, because it is the same trait in every context — no
/// declaration introduces it and none can shadow it.
///
/// Its parameter stands at `Type 0`, which states §1.2 rather than limiting it:
/// a storable type stores no type and no function, so its constructors' fields
/// contribute nothing above level 0 and the family lands there. A `Storable`
/// constraint at a higher universe is one nothing could ever answer, and
/// refusing it where the argument is checked says so earlier than a failed
/// lookup would.
pub(crate) fn class() -> Arc<Trait> {
    static CLASS: OnceLock<Arc<Trait>> = OnceLock::new();
    Arc::clone(CLASS.get_or_init(|| {
        let at = Origin::UNKNOWN;
        Arc::new(Trait {
            name: Arc::from(STORABLE),
            package: None,
            params: Arc::from(vec![Binder::explicit(Arc::from("A"), Term::universe(at, Level::ZERO))]),
            supers: Arc::from(Vec::new()),
            dictionary: Term::lam(at, "A", Term::record_type(at, core::iter::empty())),
            methods: Arc::from(Vec::new()),
            derived: Arc::from(Vec::new()),
        })
    }))
}

/// The instances `group` generates: one per storable family, none for a family
/// that is not.
///
/// `classes` is read for the families declared *earlier* that this group's
/// fields mention. A family of this same group is not looked up there and could
/// not be — inside a declaration group the families are binders rather than
/// constants, so a mutual reference is a variable and the fixed point below is
/// what decides it. §1.2's "checked once per declaration group with mutually
/// recursive families grouped together" is that fixed point.
pub(crate) fn instances(classes: &Classes, group: &Arc<Group>) -> Vec<Arc<Instance>> {
    let telescope = Telescope {
        arity: group.families.len(),
        params: group
            .params
            .iter()
            .map(|param| matches!(param.ty.shape(), Shape::Universe(_)))
            .collect(),
    };

    // Start by assuming every family storable and let a field refute one. The
    // other direction — assuming none and growing — would decide a recursive
    // family by whether its own recursive field is storable yet, which is the
    // question being asked.
    let mut storable = vec![true; telescope.arity];
    let mut needed = vec![BTreeSet::new(); telescope.arity];
    while refine(group, &telescope, classes, &mut storable, &mut needed) {}

    (0..telescope.arity)
        .filter(|which| storable.get(*which).copied().unwrap_or(false))
        .map(|which| built(group, which, needed.get(which)))
        .collect()
}

/// Where a de Bruijn level lands in a declaration group's binders.
///
/// A field type is read under the family binders, then the group parameters,
/// then the fields before it, and every question this module asks about a
/// variable is which of those three it names.
struct Telescope {
    /// How many families the group declares, which is how many binders come
    /// first.
    arity: usize,
    /// Which group parameter is a *type*, and so may carry a constraint.
    params: Vec<bool>,
}

impl Telescope {
    /// The depth a constructor's field `which` is read at.
    fn depth(&self, which: usize) -> usize {
        self.arity.saturating_add(self.params.len()).saturating_add(which)
    }

    /// The level a variable at `index` names, read at `depth`.
    fn level(depth: usize, index: Index) -> Option<usize> {
        depth
            .checked_sub(usize::try_from(index.0).unwrap_or(usize::MAX))
            .and_then(|above| above.checked_sub(1))
    }

    /// Which family a level names, if it names one.
    const fn family(&self, level: usize) -> Option<usize> {
        if level < self.arity { Some(level) } else { None }
    }

    /// Which group parameter a level names, if it names one that may carry a
    /// constraint.
    fn parameter(&self, level: usize) -> Option<usize> {
        let which = level.checked_sub(self.arity)?;
        self.params.get(which).copied().unwrap_or(false).then_some(which)
    }
}

/// One pass of the fixed point: drop every family a field refutes, and answer
/// whether anything was dropped.
fn refine(
    group: &Arc<Group>,
    telescope: &Telescope,
    classes: &Classes,
    storable: &mut [bool],
    needed: &mut [BTreeSet<usize>],
) -> bool {
    let mut dropped = false;
    for (which, family) in group.families.iter().enumerate() {
        if !storable.get(which).copied().unwrap_or(false) {
            continue;
        }
        let mut required = BTreeSet::new();
        let mut survives = true;
        for case in family.constructors.iter() {
            for (position, field) in case.fields.iter().enumerate() {
                let depth = telescope.depth(position);
                if !stored(&field.ty, depth, telescope, classes, storable, &mut required) {
                    survives = false;
                }
            }
        }
        if survives {
            if let Some(slot) = needed.get_mut(which) {
                *slot = required;
            }
        } else {
            if let Some(slot) = storable.get_mut(which) {
                *slot = false;
            }
            dropped = true;
        }
    }
    dropped
}

/// Whether a stored field's type is storable, collecting the group parameters
/// the constraint is needed on.
fn stored(
    ty: &Term,
    depth: usize,
    telescope: &Telescope,
    classes: &Classes,
    storable: &[bool],
    required: &mut BTreeSet<usize>,
) -> bool {
    finite(ty, depth, telescope, required) && headed(ty, depth, telescope, classes, storable, required)
}

/// Whether the field type's head is one an instance answers.
///
/// Only the head, because only the head decides which instance is consulted;
/// the arguments were scanned by [`finite`] already.
fn headed(
    ty: &Term,
    depth: usize,
    telescope: &Telescope,
    classes: &Classes,
    storable: &[bool],
    required: &mut BTreeSet<usize>,
) -> bool {
    match ty.shape() {
        Shape::App { function, .. } => headed(function, depth, telescope, classes, storable, required),
        Shape::Var(index) => {
            let Some(level) = Telescope::level(depth, *index) else {
                return false;
            };
            if let Some(family) = telescope.family(level) {
                return storable.get(family).copied().unwrap_or(false);
            }
            // A parameter is answered by the constraint the generated instance
            // carries. Anything deeper is an earlier field, and a field whose
            // *value* is the type of a later one has no encoding.
            let Some(which) = telescope.parameter(level) else {
                return false;
            };
            required.insert(which);
            true
        }
        Shape::Const(constant) => {
            constant.is_family()
                && classes
                    .instance(&Key::rigid(&storable_name(), &constant.name()))
                    .is_some()
        }
        // A base type stands for the host's own data, so whether it can be
        // stored is the host's claim and not this crate's guess: it is answered
        // by an instance keyed on the base type's name, exactly as a declared
        // family's is.
        Shape::Base(base) => classes.instance(&Key::rigid(&storable_name(), base.name())).is_some(),
        // An anonymous record stores its fields, so it asks the same question
        // once per field, each read under the ones before it.
        Shape::RecordType(fields) => fields.iter().enumerate().all(|(position, field)| {
            stored(
                &field.term,
                depth.saturating_add(position),
                telescope,
                classes,
                storable,
                required,
            )
        }),
        // Everything else is refused by `finite` already, or is not a type a
        // value can be stored at. A definition is here rather than unfolded
        // because this walk runs over a *quoted* type, where δ has already
        // happened: reaching one means the type was never evaluated, and
        // guessing on its behalf is what this function must not do.
        Shape::Def(_)
        | Shape::Universe(_)
        | Shape::Pi { .. }
        | Shape::Lam { .. }
        | Shape::Record(_)
        | Shape::Project { .. }
        | Shape::Id { .. }
        | Shape::Refl(_)
        | Shape::J { .. }
        | Shape::Meta(_)
        | Shape::Builtin(_)
        | Shape::Lit(_)
        | Shape::Let { .. } => false,
    }
}

/// §1.2's at-any-depth rule: no arrow and no universe anywhere in the term,
/// collecting the type parameters it mentions on the way.
///
/// This half runs over *arguments* as well as over types, which is why it asks a
/// weaker question than [`headed`]: `Vec A n` is a storable type whose second
/// argument is a number, and a walk demanding every argument be storable would
/// refuse it.
fn finite(ty: &Term, depth: usize, telescope: &Telescope, required: &mut BTreeSet<usize>) -> bool {
    match ty.shape() {
        // The two shapes §1.2 names. An arrow has no encoding, and a stored type
        // is something nobody can encode either.
        Shape::Pi { .. } | Shape::Universe(_) => false,
        Shape::Var(index) => {
            if let Some(which) = Telescope::level(depth, *index).and_then(|level| telescope.parameter(level)) {
                required.insert(which);
            }
            true
        }
        // Neither an arrow nor a universe, and neither holds one: a base type
        // is a name, a literal is a closed value the host owns, and a builtin's
        // application is walked by the arm below.
        Shape::Const(_) | Shape::Def(_) | Shape::Base(_) | Shape::Builtin(_) | Shape::Lit(_) | Shape::Meta(_) => true,
        Shape::App { function, argument } => {
            finite(function, depth, telescope, required) && finite(argument, depth, telescope, required)
        }
        Shape::Lam { body, .. } => finite(body, depth.saturating_add(1), telescope, required),
        Shape::Project { record, .. } => finite(record, depth, telescope, required),
        Shape::Refl(value) => finite(value, depth, telescope, required),
        Shape::Id { ty, left, right } => {
            finite(ty, depth, telescope, required)
                && finite(left, depth, telescope, required)
                && finite(right, depth, telescope, required)
        }
        Shape::Let { value, body, .. } => {
            finite(value, depth, telescope, required) && finite(body, depth.saturating_add(1), telescope, required)
        }
        Shape::RecordType(fields) | Shape::Record(fields) => fields
            .iter()
            .enumerate()
            .all(|(position, field)| finite(&field.term, depth.saturating_add(position), telescope, required)),
        Shape::J {
            motive,
            base,
            ty,
            from,
            to,
            proof,
        } => [motive, base, ty, from, to, proof]
            .into_iter()
            .all(|part| finite(part, depth, telescope, required)),
    }
}

/// The instance for family `which`: `impl<p⃗, i⃗> Storable<N p⃗ i⃗> where Storable pⱼ…`.
///
/// The parameters are the group's parameters followed by the family's indices,
/// and their types transfer from the declaration unchanged rather than being
/// re-indexed. §1.1 forbids a parameter or index type from mentioning the
/// declaration, so every free index in one stands below the family binders —
/// which is to say it names the same binder at this shallower depth. This crate
/// does not shift terms, and this is one of the places where it does not have to.
fn built(group: &Arc<Group>, which: usize, needed: Option<&BTreeSet<usize>>) -> Arc<Instance> {
    let at = group.origin;
    let family = group.families.get(which);
    let mut params: Vec<Binder> = group.params.to_vec();
    if let Some(declared) = family {
        params.extend(declared.indices.iter().cloned());
    }
    let depth = params.len();

    let head = Term::new(
        at,
        Shape::Const(Constant::family(group, u32::try_from(which).unwrap_or(u32::MAX))),
    );
    let applied = (0..depth).fold(head, |function, level| {
        Term::app(at, function, variable(at, depth, level))
    });

    let class = storable_name();
    let context: Vec<Constraint> = needed
        .into_iter()
        .flatten()
        .map(|&parameter| Constraint {
            origin: at,
            class: Arc::clone(&class),
            args: Arc::from(vec![variable(at, depth, parameter)]),
        })
        .collect();

    // `λp⃗. λi⃗. λd⃗. {}` — one binder per parameter, one per constraint, and the
    // empty record the module doc argues for.
    let mut dictionary = Term::record(at, core::iter::empty());
    for _ in 0..context.len() {
        dictionary = Term::lam(at, STORABLE, dictionary);
    }
    for binder in params.iter().rev() {
        dictionary = Term::lam(at, Arc::clone(&binder.name), dictionary);
    }

    let name = family.map_or_else(|| Arc::from("?"), |declared| Arc::clone(&declared.name));
    Arc::new(Instance {
        origin: at,
        key: Key::rigid(&class, &name),
        params: Arc::from(params),
        args: Arc::from(vec![applied]),
        context: Arc::from(context),
        dictionary,
    })
}

/// `Storable` as a name, which every key and constraint here needs.
fn storable_name() -> Name {
    Arc::from(STORABLE)
}

/// The variable naming binder `level` of a telescope `depth` deep.
fn variable(at: Origin, depth: usize, level: usize) -> Term {
    let index = depth.saturating_sub(level).saturating_sub(1);
    Term::var(at, Index(u32::try_from(index).unwrap_or(u32::MAX)))
}
