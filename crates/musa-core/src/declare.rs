//! Elaborating a `data` declaration, and the positivity check that admits it.
//!
//! [`family`](crate::family) says what a declaration group *is*; this says how
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

use crate::context::Cx;
use crate::elab::Elaborator;
use crate::error::CoreError;
use crate::eval::eval;
use crate::family::{Binder, Constructor, Declared, Group};
use crate::level::Level;
use crate::origin::Origin;
use crate::raw::{RawBinder, RawConstructor, RawData, RawFamily};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Index, Name, Shape, Term};
use crate::value::{Form, Value};

/// Elaborate a `data` declaration group.
///
/// # Errors
///
/// [`Refusal::NonPositive`] for an occurrence §1.1 forbids, and otherwise as
/// [`crate::check`]: the parameters, indices, fields, and chosen indices are
/// ordinary elaboration.
pub(crate) fn declare(cx: &Cx, data: &RawData) -> Result<Arc<Group>, ElabError> {
    let here = data.origin;
    let closed = cx.closed();
    let mut elaborator = Elaborator::new(cx);

    // Pass one: the signatures. The family binders stand at a type nothing may
    // use, and `Occurrence` is what makes that safe rather than merely quiet.
    let opaque = Arc::new(Value::new(here, Form::Universe(Level::ZERO)));
    let outline = declaring(&Scope::new(&closed), data, |_| Arc::clone(&opaque));
    let arity = u32::try_from(data.families.len()).unwrap_or(u32::MAX);
    let (params, under_params) = telescope(&mut elaborator, &outline, &data.params, arity)?;

    let mut indices = Vec::with_capacity(data.families.len());
    let mut levels = Vec::with_capacity(data.families.len());
    for family in &data.families {
        let (bound, _) = telescope(&mut elaborator, &under_params, &family.indices, arity)?;
        levels.push(elaborator.fresh_level(here)?);
        indices.push(bound);
    }

    // Pass two: the constructors, with the families at their real types.
    let signatures = signatures(&mut elaborator, &outline, here, &params, &indices, &levels)?;
    let scope = declaring(&Scope::new(&closed), data, |which| {
        Arc::clone(signatures.get(which).unwrap_or(&opaque))
    });
    let under_params = assumed(&mut elaborator, &scope, &params)?;

    let mut families = Vec::with_capacity(data.families.len());
    for (which, family) in data.families.iter().enumerate() {
        let Some((declared_indices, level)) = indices.get(which).zip(levels.get(which)) else {
            continue;
        };
        let built = constructors(&mut elaborator, &under_params, data, family, declared_indices, arity)?;
        // The family's own level is the join of what its constructors store,
        // which is the constraint `Type l` has to satisfy for every field type
        // to be a type at or below it.
        if !level.determine(&built.level) {
            return Err(Refusal::Unsolved {
                site: crate::meta::MetaSource::UniverseLevel,
                created: here,
                blocked: None,
            }
            .into());
        }
        families.push(Declared {
            name: Arc::clone(&family.name),
            indices: Arc::from(declared_indices.clone()),
            level: level.resolved(),
            constructors: Arc::from(built.constructors),
        });
    }

    elaborator.settled()?;
    Ok(Arc::new(Group {
        origin: here,
        params: Arc::from(params),
        families: Arc::from(families),
    }))
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
) -> Result<(Vec<Binder>, Scope), ElabError> {
    let mut binders = Vec::with_capacity(raw.len());
    let mut inner = scope.clone();
    for binder in raw {
        let (ty, _) = elaborator.check_type(&inner, &binder.ty)?;
        // Refused here rather than left to type-check against the opaque
        // signature the first pass stands the families at: a parameter whose type
        // is the family being declared has no meaning, and the first pass is
        // exactly where it would look like it did.
        if let Some(at) = mentions(&ty, arity, inner.depth(), 0) {
            return Err(Refusal::NonPositive {
                at,
                family: Arc::clone(&binder.name),
                constructor: Arc::clone(&binder.name),
            }
            .into());
        }
        inner = assume(elaborator, &inner, binder.ty.origin(), &binder.name, &ty)?;
        binders.push(Binder {
            name: Arc::clone(&binder.name),
            ty,
        });
    }
    Ok((binders, inner))
}

/// The scope with already-elaborated binders assumed.
fn assumed(elaborator: &mut Elaborator, scope: &Scope, binders: &[Binder]) -> Result<Scope, ElabError> {
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
    params: &[Binder],
    indices: &[Vec<Binder>],
    levels: &[Level],
) -> Result<Vec<Arc<Value>>, CoreError> {
    indices
        .iter()
        .zip(levels)
        .map(|(bound, level)| {
            let result = Term::universe(here, level.clone());
            let term = closed_over(here, params, closed_over(here, bound, result));
            Ok(Arc::new(eval(elaborator.meter(), scope.env(), &term)?))
        })
        .collect()
}

/// `(b₀ : B₀) → … → body`.
fn closed_over(here: Origin, binders: &[Binder], body: Term) -> Term {
    binders.iter().rev().fold(body, |codomain, binder| {
        Term::pi(here, Arc::clone(&binder.name), binder.ty.clone(), codomain)
    })
}

/// A family's constructors, and the level they force it to.
struct Built {
    constructors: Vec<Constructor>,
    level: Level,
}

fn constructors(
    elaborator: &mut Elaborator,
    scope: &Scope,
    data: &RawData,
    family: &RawFamily,
    indices: &[Binder],
    arity: u32,
) -> Result<Built, ElabError> {
    let mut built = Vec::with_capacity(family.constructors.len());
    let mut level = Level::ZERO;
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
            let depth = scope.depth().saturating_add(position);
            match occurrence(&binder.ty, arity, depth) {
                Ok(None) => {
                    if let Some(found) = levels.get(usize::try_from(position).unwrap_or(usize::MAX)) {
                        level = level.max(found);
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
        let chosen = chosen_indices(elaborator, &inner, scope, constructor, indices, data.origin)?;
        built.push(Constructor {
            name: Arc::clone(&constructor.name),
            fields: Arc::from(fields),
            recursive: Arc::from(recursive),
            indices: Arc::from(chosen),
        });
    }
    Ok(Built {
        constructors: built,
        level,
    })
}

fn telescope_fields(
    elaborator: &mut Elaborator,
    scope: &Scope,
    constructor: &RawConstructor,
) -> Result<(Vec<Binder>, Vec<Level>, Scope), ElabError> {
    let mut binders = Vec::with_capacity(constructor.fields.len());
    let mut levels = Vec::with_capacity(constructor.fields.len());
    let mut inner = scope.clone();
    for field in &constructor.fields {
        let (ty, level) = elaborator.check_type(&inner, &field.ty)?;
        inner = assume(elaborator, &inner, field.ty.origin(), &field.name, &ty)?;
        binders.push(Binder {
            name: Arc::clone(&field.name),
            ty,
        });
        levels.push(level);
    }
    Ok((binders, levels, inner))
}

/// The index arguments a constructor's result chooses, each checked against the
/// family's index type at the arguments already chosen.
fn chosen_indices(
    elaborator: &mut Elaborator,
    inner: &Scope,
    under_params: &Scope,
    constructor: &RawConstructor,
    indices: &[Binder],
    here: Origin,
) -> Result<Vec<Term>, ElabError> {
    if constructor.indices.len() != indices.len() {
        return Err(Refusal::IndexCount {
            at: here,
            expected: indices.len(),
            found: constructor.indices.len(),
        }
        .into());
    }
    // The family's index telescope is read under the parameters and the indices
    // before it, which is a different context from the constructor's fields — so
    // the type is evaluated in that environment and the *argument* is elaborated
    // in this one.
    let mut reading = under_params.env().clone();
    let mut chosen = Vec::with_capacity(indices.len());
    for (binder, raw) in indices.iter().zip(&constructor.indices) {
        let ty = eval(elaborator.meter(), &reading, &binder.ty)?;
        let term = elaborator.check_open(inner, raw, &ty)?;
        reading = reading.push(inner.eval(elaborator.meter(), &term)?);
        chosen.push(term);
    }
    Ok(chosen)
}

/// Whether a field type is a recursive occurrence, and where it is one §1.1 does
/// not allow.
///
/// Three answers rather than two, because "mentions the declaration" is not the
/// question — *where* it mentions it is. A field that is `N p⃗ i⃗` is the recursive
/// field the recursor gives an induction hypothesis for; a field that mentions
/// `N` anywhere else is refused.
///
/// Refused includes two things §1.1 itself permits, and both are argued in
/// [`crate::family`]'s module doc: an occurrence to the right of an arrow
/// (`(Nat → W) → W`), and an occurrence nested inside another family's argument
/// (`List (Rose A)`). The second is written as a mutual declaration instead,
/// which this module already supports.
type Occurrence = Result<Option<u32>, Origin>;

fn occurrence(ty: &Term, arity: u32, depth: u32) -> Occurrence {
    let (head, arguments) = spine(ty);
    if let Shape::Var(index) = head.shape()
        && let Some(family) = declared_by(arity, depth, *index)
    {
        for argument in &arguments {
            if let Some(at) = mentions(argument, arity, depth, 0) {
                return Err(at);
            }
        }
        return Ok(Some(family));
    }
    match mentions(ty, arity, depth, 0) {
        Some(at) => Err(at),
        None => Ok(None),
    }
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
fn mentions(term: &Term, arity: u32, depth: u32, bound: u32) -> Option<Origin> {
    let here = term.origin();
    let under = bound.saturating_add(1);
    match term.shape() {
        Shape::Var(index) => declared_by(arity, depth.saturating_add(bound), *index).map(|_| here),
        Shape::Const(_) | Shape::Universe(_) | Shape::Meta(_) => None,
        Shape::Pi { domain, codomain, .. } => {
            mentions(domain, arity, depth, bound).or_else(|| mentions(codomain, arity, depth, under))
        }
        Shape::Lam { body, .. } => mentions(body, arity, depth, under),
        Shape::App { function, argument } => {
            mentions(function, arity, depth, bound).or_else(|| mentions(argument, arity, depth, bound))
        }
        Shape::RecordType(fields) => fields.iter().enumerate().find_map(|(position, field)| {
            let inside = bound.saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
            mentions(&field.term, arity, depth, inside)
        }),
        Shape::Record(fields) => fields
            .iter()
            .find_map(|field| mentions(&field.term, arity, depth, bound)),
        Shape::Project { record, .. } => mentions(record, arity, depth, bound),
        Shape::Id { ty, left, right } => [ty, left, right]
            .into_iter()
            .find_map(|part| mentions(part, arity, depth, bound)),
        Shape::Refl(value) => mentions(value, arity, depth, bound),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => [ty, from, motive, base, to, proof]
            .into_iter()
            .find_map(|part| mentions(part, arity, depth, bound)),
        Shape::Let { ty, value, body, .. } => mentions(ty, arity, depth, bound)
            .or_else(|| mentions(value, arity, depth, bound))
            .or_else(|| mentions(body, arity, depth, under)),
    }
}
