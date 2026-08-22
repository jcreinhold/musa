//! Elaborating a `data` declaration, and the positivity check that admits it.
//!
//! [`family`](crate::kernel::family) says what a declaration group *is*; this says how
//! one is made. The two are separate modules because they answer to different
//! things: the representation answers to evaluation and quotation, and this
//! answers to §1.1 and to the author who wrote the declaration wrong.
//!
//! # Two passes over the same binders
//!
//! A constructor's field may mention the family being declared, so the family
//! must be in scope while its constructors are elaborated — at the type it will
//! have, which is `(p⃗ : Params) → (i⃗ : Indices) → Type l`. That type is not known
//! until the parameters and indices are elaborated, and *those* are read under
//! the declaration context, which is the family binders. The circle is only
//! apparent: §1.1 forbids a parameter or an index type from mentioning the
//! declaration, so the first pass elaborates them with the family binders present
//! but standing at a type nothing may use, and [`Occurrence`] refuses anything
//! that reached for one. The second pass rebuilds the same binders at their real
//! types, and every de Bruijn index from the first pass still names what it named
//! — the count did not change.
//!
//! # The universe is computed, never written
//!
//! §1: "the surface never writes a level". A family's is the join of its
//! constructors' field levels, with recursive occurrences contributing nothing —
//! they stand at the very level being computed, so counting them would be
//! `l = max(…, l)` and no solution. The signature therefore carries a level
//! metavariable while the constructors are elaborated, and it is solved to the
//! join once they are.

use std::sync::Arc;

use crate::elaboration::elab::Elaborator;
use crate::elaboration::raw::{RawBinder, RawConstructor, RawData, RawFamily};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::context::{Cx, Globals};
use crate::kernel::error::CoreError;
use crate::kernel::eval::eval;
use crate::kernel::family::{Constant, Constructor, Counting, Declared, Group, Parameter};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Binder, Definition, Index, Name, Role, Shape, Term};
use crate::kernel::value::{Form, Value};
use crate::kernel::visibility::Visibility;

/// Elaborate a `data` declaration group.
///
/// # Errors
///
/// [`Refusal::NonPositive`] for an occurrence §1.1 forbids, and otherwise as
/// [`crate::check`]: the parameters, indices, fields, and chosen indices are
/// ordinary elaboration.
pub(crate) fn declare(cx: &Cx, data: &RawData) -> Result<(Arc<Group>, crate::Spend), ElabError> {
    let here = data.origin;
    let closed = cx.closed();
    let mut elaborator = Elaborator::new(cx);

    // Pass one: the signatures. The family binders stand at a type nothing may
    // use, and `Occurrence` is what makes that safe rather than merely quiet.
    let opaque = Arc::new(Value::new(here, Form::Universe(Sort::ZERO)));
    let outline = declaring(&Scope::new(&closed), data, |_| Arc::clone(&opaque));
    let arity = u32::try_from(data.families.len()).unwrap_or(u32::MAX);
    let (params, _under_params) = telescope(&mut elaborator, &outline, &data.params, arity)?;

    // Pass two: the constructors, with the families at their real types. §1: a
    // data family stores small types and so lands at `Type 0`, which is what
    // every signature says and what the constructor check enforces.
    let signatures = signatures(&mut elaborator, &outline, here, &params, data)?;
    let scope = declaring(&Scope::new(&closed), data, |which| {
        Arc::clone(signatures.get(which).unwrap_or(&opaque))
    });
    let under_params = assumed(&mut elaborator, &scope, &params)?;

    let mut families = Vec::with_capacity(data.families.len());
    for (which, family) in data.families.iter().enumerate() {
        let built = constructors(&mut elaborator, &under_params, data, family, arity)?;
        // The family's level is the join of what its constructors store; §1's
        // two universes make that join a check — every field small — rather
        // than an inference.
        if built.level == Sort::One {
            return Err(Refusal::BeyondUniverses { at: data.origin }.into());
        }
        uniform(family)?;
        let index = index_binder(&mut elaborator, &Scope::new(&closed), family)?;
        let which = u32::try_from(which).unwrap_or(u32::MAX);
        families.push(Declared {
            counting: counting(which, &params, &built.constructors),
            name: Arc::clone(&family.name),
            visibility: family.visibility,
            constructors: Arc::from(built.constructors),
            index,
        });
    }

    elaborator.settled()?;
    // The signatures are stored, so the metas elaboration solved in them are
    // written back as terms first — [`Elaborator::zonk`] gives the reason.
    let params = params
        .into_iter()
        .map(|binder| {
            Ok::<_, ElabError>(Parameter {
                ty: elaborator.zonk(&binder.ty)?,
                ..binder
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let families = families
        .into_iter()
        .map(|declared| {
            let crate::kernel::family::Declared {
                counting,
                name,
                visibility,
                constructors,
                index,
            } = declared;
            let constructors = constructors
                .iter()
                .map(|constructor| {
                    let fields = constructor
                        .fields
                        .iter()
                        .map(|field| {
                            Ok::<_, ElabError>(Parameter {
                                name: Arc::clone(&field.name),
                                ty: elaborator.zonk(&field.ty)?,
                                filling: field.filling.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(crate::kernel::family::Constructor {
                        name: Arc::clone(&constructor.name),
                        visibility: constructor.visibility,
                        fields: Arc::from(fields),
                        recursive: Arc::clone(&constructor.recursive),
                    })
                })
                .collect::<Result<Vec<_>, ElabError>>()?;
            Ok(crate::kernel::family::Declared {
                counting,
                name,
                visibility,
                constructors: Arc::from(constructors),
                index,
            })
        })
        .collect::<Result<Vec<_>, ElabError>>()?;
    // Answered here, after the constructors and before anything can read them:
    // a later declaration asks this group whether it may put itself at one of
    // these parameters, and the fields that decide it are all in hand exactly
    // once.
    let positive: Vec<bool> = (0..u32::try_from(params.len()).unwrap_or(u32::MAX))
        .map(|which| parameter_is_positive(&families, arity, &under_params, which))
        .collect();
    let group = Arc::new(Group {
        origin: here,
        params: Arc::from(params),
        positive: Arc::from(positive),
        families: Arc::from(families),
        module: cx.module(),
    });
    Ok((group, elaborator.spent()))
}

/// Refuse a family whose cases are not all equally visible.
///
/// `01-surface.md` §1.3 takes all the cases or none of them, and the reason is
/// coverage rather than tidiness: outside the module a `match` on a partly
/// private type could still be written, the arms it is *allowed* to write would
/// never exhaust the type, and every such `match` would need a catch-all for
/// cases the author cannot see. Refusing the declaration is a better thing to
/// explain than that.
///
/// It runs at the declaration and not at the use site because that is where the
/// author can fix it, and because everything downstream — [`Found::hidden_from`]
/// most of all — is then entitled to ask the first case and stop.
fn uniform(family: &RawFamily) -> Result<(), ElabError> {
    let mut public = None;
    let mut private = None;
    for case in &family.constructors {
        let seen = match case.visibility {
            Visibility::Public => &mut public,
            Visibility::Private => &mut private,
        };
        seen.get_or_insert((Arc::clone(&case.name), case.origin));
    }
    match (public, private) {
        (Some((public, at)), Some((private, _))) => Err(Refusal::MixedVisibility {
            family: Arc::clone(&family.name),
            public,
            private,
            at,
        }
        .into()),
        // Every other shape is uniform: all public, all private, or no cases.
        _ => Ok(()),
    }
}

/// The scope holding one binder per family, at whatever type `ty` says.
fn declaring(scope: &Scope, data: &RawData, ty: impl Fn(usize) -> Arc<Value>) -> Scope {
    let mut built = scope.clone();
    for (which, family) in data.families.iter().enumerate() {
        built = built.assume(Some(Arc::clone(&family.name)), data.origin, ty(which));
    }
    built
}

/// Elaborate a parameter or index telescope, whose types §1.1 does not let
/// mention the declaration.
fn telescope(
    elaborator: &mut Elaborator,
    scope: &Scope,
    raw: &[RawBinder],
    arity: u32,
) -> Result<(Vec<Parameter>, Scope), ElabError> {
    let mut binders = Vec::with_capacity(raw.len());
    let mut inner = scope.clone();
    for binder in raw {
        let (ty, _) = elaborator.check_type(&inner, &binder.ty)?;
        // Refused here rather than left to type-check against the opaque
        // signature the first pass stands the families at: a parameter whose type
        // is the family being declared has no meaning, and the first pass is
        // exactly where it would look like it did.
        if let Some(at) = mentions(&ty, Watched::families(arity), inner.depth().0, 0) {
            return Err(Refusal::NonPositive {
                at,
                family: Arc::clone(&binder.name),
                constructor: Arc::clone(&binder.name),
            }
            .into());
        }
        inner = assume(elaborator, &inner, binder.ty.origin(), &binder.name, &ty)?;
        binders.push(Parameter::written(Arc::clone(&binder.name), ty));
    }
    Ok((binders, inner))
}

/// Elaborate the index a family declares, and refuse a sort §1.5 does not admit.
///
/// # Why it is elaborated closed
///
/// An index sort is `Nat`, an exact `Ratio`, or a finite literal enum, and all
/// three are *closed* types. So the binder is read with nothing in scope — not
/// the group's parameters and not the families — and a sort that wanted either
/// would be refused for naming something that is not there before it was
/// refused for not being a sort. That is what lets a use site evaluate the
/// stored type under whatever scope it happens to be in, which is what
/// `indexed_type_formation` does.
///
/// # Why the check is `convert`'s own predicate
///
/// [`crate::elaboration::convert::is_index_sort`] is what decides, at a comparison, whether
/// a variable standing in an index has a sort at all. Asking it here rather
/// than listing the admissible types again means the declaration and the
/// comparison cannot disagree about what a sort is: an index the declaration
/// admitted is one [`crate::kernel::index`] can read, by construction rather than by
/// two lists kept in step.
///
/// # Errors
///
/// [`Refusal::NotAnIndexSort`] for a binder at any other type, and otherwise as
/// [`crate::check`].
fn index_binder(
    elaborator: &mut Elaborator,
    closed: &Scope,
    family: &RawFamily,
) -> Result<Option<Parameter>, ElabError> {
    let Some(binder) = family.index.as_ref() else {
        return Ok(None);
    };
    let (ty, _) = elaborator.check_type(closed, &binder.ty)?;
    let sort = closed.eval(elaborator.meter(), &ty)?;
    if !crate::elaboration::convert::is_index_sort(&sort) {
        return Err(Refusal::NotAnIndexSort {
            binder: Arc::clone(&binder.name),
            ty: Arc::clone(&family.name),
            at: binder.ty.origin(),
        }
        .into());
    }
    Ok(Some(Parameter::written(Arc::clone(&binder.name), ty)))
}

/// The scope with already-elaborated binders assumed.
fn assumed(elaborator: &mut Elaborator, scope: &Scope, binders: &[Parameter]) -> Result<Scope, ElabError> {
    let mut inner = scope.clone();
    for binder in binders {
        inner = assume(elaborator, &inner, binder.ty.origin(), &binder.name, &binder.ty)?;
    }
    Ok(inner)
}

fn assume(elaborator: &mut Elaborator, scope: &Scope, at: Origin, name: &Name, ty: &Term) -> Result<Scope, ElabError> {
    let value = scope.eval(elaborator.meter(), ty)?;
    Ok(scope.assume(Some(Arc::clone(name)), at, Arc::new(value)))
}

/// Each family's type, as a value, read in the declaration context.
fn signatures(
    elaborator: &mut Elaborator,
    scope: &Scope,
    here: Origin,
    params: &[Parameter],
    data: &RawData,
) -> Result<Vec<Arc<Value>>, CoreError> {
    data.families
        .iter()
        .map(|_| {
            let term = closed_over(here, params, Term::universe(here, Sort::ZERO));
            Ok(Arc::new(eval(elaborator.meter(), scope.env(), &term)?))
        })
        .collect()
}

/// `(b₀ : B₀) → … → body`, each binder at the filling it was declared with.
fn closed_over(here: Origin, binders: &[Parameter], body: Term) -> Term {
    binders.iter().rev().fold(body, |codomain, binder| {
        Term::function(
            here,
            binder.filling.clone(),
            Arc::clone(&binder.name),
            binder.ty.clone(),
            codomain,
        )
    })
}

/// A family's constructors, and the level they force it to.
struct Built {
    constructors: Vec<Constructor>,
    level: Sort,
}

fn constructors(
    elaborator: &mut Elaborator,
    scope: &Scope,
    _data: &RawData,
    family: &RawFamily,
    arity: u32,
) -> Result<Built, ElabError> {
    let mut built = Vec::with_capacity(family.constructors.len());
    let mut level = Sort::ZERO;
    for (position, constructor) in family.constructors.iter().enumerate() {
        // Checked here rather than at the group, because a name is only
        // ambiguous within the namespace that qualifies it: two families may
        // each declare an `Untied` and neither shadows the other (§1.3).
        if let Some(previous) = family
            .constructors
            .iter()
            .take(position)
            .find(|earlier| earlier.name == constructor.name)
        {
            return Err(Refusal::DuplicateCase {
                at: constructor.origin,
                previous: previous.origin,
                family: Arc::clone(&family.name),
                case: Arc::clone(&constructor.name),
            }
            .into());
        }
        let (fields, levels, inner) = telescope_fields(elaborator, scope, constructor)?;
        let mut recursive = Vec::new();
        for (position, binder) in fields.iter().enumerate() {
            let position = u32::try_from(position).unwrap_or(u32::MAX);
            let depth = scope.depth().0.saturating_add(position);
            match occurrence(scope.cx().globals(), &binder.ty, arity, depth) {
                Ok(None) => {
                    if let Some(found) = levels.get(usize::try_from(position).unwrap_or(usize::MAX)) {
                        level = level.max(*found);
                    }
                }
                // A recursive field stands at the level being computed, so it
                // contributes nothing to the join — see the module doc.
                Ok(Some(of_family)) => recursive.push((position, of_family)),
                Err(at) => {
                    return Err(Refusal::NonPositive {
                        at,
                        family: Arc::clone(&family.name),
                        constructor: Arc::clone(&constructor.name),
                    }
                    .into());
                }
            }
        }
        let _ = &inner;
        built.push(Constructor {
            name: Arc::clone(&constructor.name),
            visibility: constructor.visibility,
            fields: Arc::from(fields),
            recursive: Arc::from(recursive),
        });
    }
    Ok(Built {
        constructors: built,
        level,
    })
}

/// Whether family `which` counts, and by which two constructors.
///
/// The three conditions of [`Counting`], read off the declaration that was just
/// checked: no group parameters, exactly two constructors, and between them one
/// with no fields and one whose single field is a recursive occurrence of *this*
/// family. Recursion is read off
/// [`Constructor::recursive`](crate::kernel::family::Constructor) rather than re-derived
/// from the field's type, so the positivity check and this recognition cannot
/// disagree about what a recursive field is: they are the same list.
///
/// Deliberately silent when the shape does not match. A family that misses by
/// one constructor is an ordinary family, not a mistake — there is nothing to
/// refuse, only a representation not to use.
fn counting(which: u32, params: &[Parameter], constructors: &[Constructor]) -> Option<Counting> {
    if !params.is_empty() {
        return None;
    }
    let [first, second] = constructors else {
        return None;
    };
    let steps = |case: &Constructor| case.fields.len() == 1 && *case.recursive == [(0, which)];
    let floors = |case: &Constructor| case.fields.is_empty();
    if floors(first) && steps(second) {
        return Some(Counting { floor: 0, step: 1 });
    }
    (floors(second) && steps(first)).then_some(Counting { floor: 1, step: 0 })
}

fn telescope_fields(
    elaborator: &mut Elaborator,
    scope: &Scope,
    constructor: &RawConstructor,
) -> Result<(Vec<Parameter>, Vec<Sort>, Scope), ElabError> {
    let mut binders = Vec::with_capacity(constructor.fields.len());
    let mut levels = Vec::with_capacity(constructor.fields.len());
    let mut inner = scope.clone();
    for field in &constructor.fields {
        let (ty, level) = elaborator.check_type(&inner, &field.ty)?;
        inner = assume(elaborator, &inner, field.ty.origin(), &field.name, &ty)?;
        binders.push(Parameter::written(Arc::clone(&field.name), ty));
        levels.push(level);
    }
    Ok((binders, levels, inner))
}

/// Whether a field type is a recursive occurrence, and where it is one §1.1 does
/// not allow.
///
/// Three answers rather than two, because "mentions the declaration" is not the
/// question — *where* it mentions it is. A field that is `N p⃗ i⃗` is the recursive
/// field the recursor gives an induction hypothesis for; a field that only
/// *contains* `N`, at a positive parameter of some other family, is an ordinary
/// field whose type happens to mention the declaration; and a field that mentions
/// `N` anywhere else is refused.
///
/// The middle answer is why `Ok(None)` is not the same as "does not mention".
/// `Body(items : List StaffRead)` stores a list, and the list is a field like any
/// other: `case.rs` binds it at `List StaffRead`, and the recursor hands the
/// method no induction hypothesis for it. §1.1 promises the recursor exists and
/// does not promise a hypothesis per field, and giving none is what keeps
/// [`crate::kernel::family`]'s standing invariant exactly — an induction hypothesis is an
/// application rather than a synthesized closure, so ι never builds syntax.
/// A fold that recurses *into* such a field therefore cannot be written yet: the
/// only route is mutual recursion through `List`'s own eliminator, and §2.4's
/// measure has no reason to believe an element of `items` is smaller than
/// `Body(items)`.
///
/// Refused still includes one thing §1.1 permits, argued in [`crate::kernel::family`]'s
/// module doc: an occurrence anywhere inside an arrow (`(Nat → W) → W`), which
/// would be a recursive field whose hypothesis is a function.
type Occurrence = Result<Option<u32>, Origin>;

fn occurrence(globals: &Globals, ty: &Term, arity: u32, depth: u32) -> Occurrence {
    let watched = Watched::families(arity);
    let (head, arguments) = spine(ty);
    if let Shape::Var(index) = head.shape()
        && let Some(family) = declared_by(arity, depth, *index)
    {
        // The arguments of a *direct* occurrence stay closed to the declaration.
        // `Tree (List Tree)` is non-uniform recursion, whose recursor is not the
        // one this module generates, and admitting it here would produce methods
        // at a type no ι rule can fire against.
        for argument in &arguments {
            if let Some(at) = mentions(argument, watched, depth, 0) {
                return Err(at);
            }
        }
        return Ok(Some(family));
    }
    stored_positively(globals, ty, watched, depth).map(|()| None)
}

/// Where a field type that is not a direct occurrence mentions the declaration
/// outside a strictly positive position.
///
/// Descends only through an applied family's *parameters*, and only through the
/// ones that family declared positive, bottoming out on the occurrence itself:
/// `List StaffRead` is admitted because
/// [`Group::positive_at`] says `List` is positive in its element, and
/// `Cont StaffRead` — a family with a field `A → Nat` — is refused because it
/// says the opposite. An argument past the parameters is an index, which
/// [`Group::positive_at`] answers `false` for; see its doc.
///
/// No arm for [`Shape::Pi`]: a mention inside an arrow, on either side, is
/// refused by the fall-through, which is what [`crate::kernel::family`]'s "recursive
/// fields are direct" narrowing *is*. That narrowing is deliberately kept, so
/// this walk widens exactly one rule rather than two.
fn stored_positively(globals: &Globals, ty: &Term, watched: Watched, depth: u32) -> Result<(), Origin> {
    let (head, arguments) = spine(ty);
    // The occurrence itself, reached by descending into a positive parameter.
    // This is the arm that admits `List StaffRead`: the descent bottoms out on
    // the bare `StaffRead`, and refusing it here would refuse the very shape the
    // parameter was found positive for. Its own arguments stay closed to the
    // declaration for [`occurrence`]'s reason one branch up.
    if let Shape::Var(index) = head.shape()
        && watched.holds(depth, *index)
    {
        for argument in &arguments {
            if let Some(at) = mentions(argument, watched, depth, 0) {
                return Err(at);
            }
        }
        return Ok(());
    }
    if let Some(constant) = declared_family(globals, head) {
        for (position, argument) in arguments.iter().enumerate() {
            let Some(at) = mentions(argument, watched, depth, 0) else {
                continue;
            };
            if !constant.group.positive_at(position) {
                return Err(at);
            }
            stored_positively(globals, argument, watched, depth)?;
        }
        return Ok(());
    }
    match mentions(ty, watched, depth, 0) {
        Some(at) => Err(at),
        None => Ok(()),
    }
}

/// Which de Bruijn levels a walk is looking for.
///
/// One type rather than two `u32` parameters everywhere, because the two
/// questions this module asks — *does this field mention the declaration* and
/// *does this field mention one parameter* — are the same walk over a different
/// set, and spelling the set once is what lets [`mentions`] serve both.
#[derive(Clone, Copy)]
struct Watched {
    start: u32,
    end: u32,
}

impl Watched {
    /// The declaration's families: the outermost `arity` binders.
    const fn families(arity: u32) -> Self {
        Self { start: 0, end: arity }
    }

    /// One parameter, which stands just inside them.
    const fn parameter(arity: u32, which: u32) -> Self {
        let level = arity.saturating_add(which);
        Self {
            start: level,
            end: level.saturating_add(1),
        }
    }

    /// Whether `index`, read at `depth`, is one of them.
    fn holds(self, depth: u32, index: Index) -> bool {
        depth
            .checked_sub(1)
            .and_then(|last| last.checked_sub(index.0))
            .is_some_and(|level| level >= self.start && level < self.end)
    }
}

/// Whether parameter `which` occurs only strictly positively in what this group
/// stores.
///
/// Answered here so that a *later* declaration can put itself at that parameter
/// — see [`Group::positive`]. Every constructor field of every family is walked,
/// each at the depth its own binder stands at, which is the depth
/// [`constructors`] hands [`occurrence`].
fn parameter_is_positive(families: &[Declared], arity: u32, under_params: &Scope, which: u32) -> bool {
    let watched = Watched::parameter(arity, which);
    let globals = under_params.cx().globals();
    families.iter().all(|declared| {
        declared.constructors.iter().all(|case| {
            case.fields.iter().enumerate().all(|(position, field)| {
                let depth = under_params
                    .depth()
                    .0
                    .saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
                positive_in(globals, &field.ty, watched, arity, depth, 0)
            })
        })
    })
}

/// Whether every occurrence of `watched` in `ty` is strictly positive.
///
/// The textbook walk, and it differs from [`stored_positively`] in the two
/// places the two questions differ. It descends into an arrow's *codomain*,
/// because a parameter to the right of an arrow is positive even though a
/// recursive occurrence there is refused for a reason that is not positivity.
/// And it passes straight through a family of *this* group, applied to anything:
/// `Cons : A → List A → List A` carries `A` through the very declaration whose
/// polarity is being decided, so the alternative to assuming it positive is not
/// answering at all. That optimism is the fixed point, and it is the same one
/// [`crate::elaboration::storable`] takes for the same reason one section over.
fn positive_in(globals: &Globals, ty: &Term, watched: Watched, arity: u32, depth: u32, bound: u32) -> bool {
    let own = Watched::families(arity);
    let here = depth.saturating_add(bound);
    let (head, arguments) = spine(ty);
    // The parameter itself, or a family of this group carrying it. One arm for
    // both: what each does with its arguments is the same walk, and the second
    // is the optimism the doc argues for.
    if let Shape::Var(index) = head.shape()
        && (watched.holds(here, *index) || own.holds(here, *index))
    {
        return arguments
            .iter()
            .all(|argument| positive_in(globals, argument, watched, arity, depth, bound));
    }
    if let Some(constant) = declared_family(globals, head) {
        return arguments.iter().enumerate().all(|(position, argument)| {
            mentions(argument, watched, depth, bound).is_none()
                || (constant.group.positive_at(position)
                    && positive_in(globals, argument, watched, arity, depth, bound))
        });
    }
    if let Shape::Bind {
        binder: Binder::Pi { ty: domain, .. },
        body,
        ..
    } = head.shape()
    {
        return mentions(domain, watched, depth, bound).is_none()
            && positive_in(globals, body, watched, arity, depth, bound.saturating_add(1));
    }
    mentions(ty, watched, depth, bound).is_none()
}

/// The declared family `head` names, when it names one.
///
/// The declaration is the context's (§6), so a positivity walk that wants to
/// know whether `List` is positive in its element asks the table rather than the
/// term. A name at any other role — a constructor, a recursor, a definition, a
/// base type, a builtin — is not a family and answers `None`, which is the
/// fall-through both callers already had.
fn declared_family(globals: &Globals, head: &Term) -> Option<Constant> {
    let Shape::Named {
        name,
        role: role @ Role::TypeConstructor,
    } = head.shape()
    else {
        return None;
    };
    let Definition::Declared(constant) = globals.definition(name, *role) else {
        return None;
    };
    constant.is_family().then_some(constant)
}

/// The head of an application spine, and what is applied to it.
fn spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut arguments = Vec::new();
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}

/// Which family a variable names, or `None` when it names an ordinary binder.
///
/// The declaration binders are the outermost `arity` of them, so a variable
/// names one exactly when it counts past every binder introduced since.
fn declared_by(arity: u32, depth: u32, index: Index) -> Option<u32> {
    // A de Bruijn *level* is what stays put as binders are added, and the
    // declaration's binders are the first `arity` of them — so the level is both
    // the test and the answer. Read as an index, "which family" would change
    // with every field the constructor happens to have.
    let level = depth.checked_sub(1)?.checked_sub(index.0)?;
    (level < arity).then_some(level)
}

/// Where `term` first mentions a declaration binder, if it does.
///
/// `bound` counts the binders entered inside `term`, so that a variable local to
/// it is never mistaken for one of the declaration's.
fn mentions(term: &Term, watched: Watched, depth: u32, bound: u32) -> Option<Origin> {
    let here = term.origin();
    let under = bound.saturating_add(1);
    match term.shape() {
        Shape::Var(index) => watched.holds(depth.saturating_add(bound), *index).then_some(here),
        // Both halves, for the reason `class::occurrences` gives: an index is an
        // ordinary term and a declaration binder it mentions is mentioned.
        Shape::Indexed { ty, index } => {
            mentions(ty, watched, depth, bound).or_else(|| mentions(index, watched, depth, bound))
        }
        Shape::Named { .. } | Shape::Lit(_) | Shape::Meta(_) | Shape::Universe(_) => None,
        // Whatever sits outside the binder is read where the binder is; the
        // body is read one binder in. Which subterms those are is the
        // `Binder`'s question, so the three forms share this arm.
        Shape::Bind { binder, body, .. } => binder
            .outer()
            .find_map(|term| mentions(term, watched, depth, bound))
            .or_else(|| mentions(body, watched, depth, under)),
        Shape::App { function, argument } => {
            mentions(function, watched, depth, bound).or_else(|| mentions(argument, watched, depth, bound))
        }
        Shape::RecordType(fields) => fields.iter().enumerate().find_map(|(position, field)| {
            let inside = bound.saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
            mentions(&field.term, watched, depth, inside)
        }),
        Shape::Record(fields) => fields
            .iter()
            .find_map(|field| mentions(&field.term, watched, depth, bound)),
        Shape::Project { record, .. } => mentions(record, watched, depth, bound),
    }
}
