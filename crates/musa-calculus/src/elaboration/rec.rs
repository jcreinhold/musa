//! `rec f : A = e`: how a recursion becomes a definition, and §2.4's structural
//! rule over the tree it leaves behind.
//!
//! # A recursion is a definition, and that is the whole mechanism
//!
//! `02-core-calculus.md` §1's `Definition` list says a name's reduction
//! behaviour may be **a compiled case tree**, and §2.4 says descent is checked
//! over that tree. Put together, a recursive definition is one thing: a global
//! name whose body is [`Compiled`] — the binders it abstracts, and the tree
//! beneath them — admitted when [`descends`] can see every call get smaller.
//!
//! A `rec` written *inside* a term is not a global name yet, and
//! `kernel/program.rs`'s module doc says why it cannot be left where it is: a
//! definition is a global name, a de Bruijn binder refers outward only, and
//! §1.3 refuses the fixed point that would let a local recursion stand without
//! a name. So [`lift`] makes it one — it abstracts the `rec` over **the whole
//! context**, declares that as an auxiliary definition, and leaves the
//! expression as that definition applied to the binders it abstracted. Peyton
//! Jones ch. 13, and ch. 14 for the recursive case: §14.0 is explicit that its
//! subject is recursive supercombinators *without using Y*.
//!
//! Two choices in that sentence are load-bearing. **The whole context, not the
//! body's free variables**: the narrower rule is the textbook one and costs a
//! free-variable analysis over raw syntax that would have to agree with the
//! elaborator's own scoping — a second resolver, which
//! [`Globals::definition`](crate::kernel::context::Globals::definition) refuses
//! for exactly this reason — while abstracting the context is decided by the
//! scope and is the same walk [`crate::kernel::meta`] already performs. **And
//! nothing is shifted**: a body elaborated under `n` binders already *is* the
//! body of `n` λs, because de Bruijn indices are relative, which is what makes
//! the whole transformation expressible in a crate with no substitution on
//! terms (§3).
//!
//! # The definition is in scope during its own elaboration
//!
//! Before prompt 155a a recursive call was rewritten, on *raw syntax*, into
//! `<field>#ih` — the induction hypothesis the generated eliminator's method
//! supplies. That rewrite is gone, and what replaces it is smaller: the
//! definition is installed at [`Body::Pending`] before its body is read, so a
//! call resolves to the definition itself. The spine is rigid — a `Pending`
//! body has nothing to unfold — so a body being elaborated can name itself,
//! type-check against its declared type, and compute with nothing.
//!
//! That is why the *order* here matters and is not an implementation detail.
//! The name has to be in the table before [`tree_body`] runs, and the finished
//! definition has to **replace** it rather than shadow it, which is what
//! [`Globals::lift`](crate::kernel::context::Globals::lift) does: a `Pending`
//! found after the definition is complete is a body that never reduces.
//!
//! # What a tree body can be, and what it therefore refuses
//!
//! [`tree_body`] peels the λs of a body against the Π chain of its type and
//! requires a `match` underneath them, because a [`Compiled`] is binders and a
//! tree **with nothing in between**. Two consequences, and both are refusals a
//! reader can act on:
//!
//! - A body whose top-level form is not a `match` has no tree, so a recursive
//!   definition of that shape is [`Refusal::UncheckedRecursion`]. `λn. loop n`
//!   is the whole of this case.
//! - A `match` whose subject is not already a variable is named with a `let`
//!   by [`case`](crate::elaboration::case), and a tree body has nowhere to put
//!   one. `match f(x) { … }` at a definition's top is the case, and a
//!   non-recursive definition of that shape keeps the evaluated body it has
//!   always had.
//!
//! Everything else a recursion can get wrong is [`descends`]'s to say, and
//! `kernel/terminate.rs` states that rule before implementing it.

use std::sync::Arc;

use crate::elaboration::elab::Elaborator;
use crate::elaboration::raw::{Raw, RawField, RawPattern, RawShape, RawUpdate};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::case_tree::{Alternative, CaseTree, Compiled, Split};
use crate::kernel::context::Globals;
use crate::kernel::eval::{apply_closure, eval, opened};
use crate::kernel::origin::Origin;
use crate::kernel::program::{Body, Defined};
use crate::kernel::quote::{Mode, quote_type};
use crate::kernel::scope::Scope;
use crate::kernel::sort::Levels;
use crate::kernel::term::{Filling, Index, Level, Name, Role, Term};
use crate::kernel::terminate::descends;
use crate::kernel::value::{Env, Form, Value};
use crate::kernel::visibility::Visibility;

/// Elaborate a `rec` written inside a term, by lifting it out of that term.
///
/// The definition it becomes is abstracted over every binder in `scope`, and
/// what this hands back is that definition applied to them — so the term the
/// caller gets is an ordinary application of an ordinary name, and the
/// recursion is somewhere a definition body can be. See the module docs.
///
/// **A `rec` that never names itself is not lifted.** It is the term it wraps,
/// at the type it declares, and a definition for it would be a member of the
/// program that no recursion needed. The invariant prompt 155a rests on is
/// about *recursions*, and this leaves it exact rather than approximating it
/// upwards.
///
/// # Errors
///
/// [`Refusal::UncheckedRecursion`] for a recursion §2.4's structural rule
/// cannot see descend, for a body that is not a `match` under λs, and for a
/// context with no table to lift into — a bare [`check`](crate::check) against
/// the empty context, which has no [`Program`] a definition could join.
/// Otherwise as [`crate::check`].
///
/// [`Program`]: crate::kernel::program::Program
pub(crate) fn lift(
    elaborator: &mut Elaborator,
    scope: &Scope,
    here: Origin,
    name: &Name,
    ty: &Raw,
    body: &Raw,
    goal: &Value,
) -> Result<Term, ElabError> {
    // The written type first, and the goal met by conversion: `rec` is a term
    // of the type it declares, and a `rec` in checking position owes conversion
    // like anything else.
    let (ty_term, _) = elaborator.check_type(scope, ty)?;
    let declared = scope.eval(elaborator.meter(), &ty_term)?;
    elaborator.same_types(scope, here, goal, &declared)?;
    if !names(body, name) {
        return elaborator.check_open(scope, body, &declared);
    }
    let refuse = || -> ElabError {
        Refusal::UncheckedRecursion {
            at: here,
            name: Arc::clone(name),
        }
        .into()
    };
    let globals = scope.cx().globals().clone();
    if !globals.lifts() {
        return Err(refuse());
    }
    let telescope = scope.telescope();
    // The lifted type is the written one under a Π per binder, each domain
    // quoted at the depth that binder stands at — the walk
    // `Elaborator::fresh_meta` performs to close an unknown over its context.
    let mut lifted_ty = ty_term;
    for (position, binder) in telescope.iter().enumerate().rev() {
        let depth = Level(u32::try_from(position).unwrap_or(0));
        let domain = quote_type(elaborator.meter(), depth, Mode::Keep, &binder.ty)?;
        lifted_ty = Term::pi(here, Arc::clone(&binder.name), domain, lifted_ty);
    }
    let env = Env::under(globals.clone());
    let lifted_ty = elaborator.zonk(&lifted_ty)?;
    let ty_value = Arc::new(eval(elaborator.meter(), &env, &lifted_ty)?);
    let lifted_name = lifted_name(elaborator, &globals, name);
    let held = |body: Body, ty_term: Term| Defined {
        name: Arc::clone(&lifted_name),
        // Unnameable by spelling and unnameable by rule: `#` is not an
        // identifier character, and a lifted definition is nobody's export.
        visibility: Visibility::Private,
        module: scope.cx().module(),
        levels: Arc::from([]),
        ty: Arc::clone(&ty_value),
        body,
        ty_term,
        lifted: true,
    };
    globals.lift(&Arc::new(held(Body::Pending, lifted_ty.clone())));
    let prefix: Vec<Name> = telescope.iter().map(|binder| Arc::clone(&binder.name)).collect();
    elaborator.recursing(
        name,
        &lifted_name,
        telescope.iter().map(|binder| binder.value.clone()).collect(),
    );
    let compiled = tree_body(elaborator, scope, &prefix, body, &declared);
    elaborator.recursed();
    let Some(compiled) = compiled? else {
        return Err(refuse());
    };
    if let Some(undescending) = descends(&compiled, &lifted_name) {
        return Err(Refusal::UncheckedRecursion {
            at: undescending.0,
            name: Arc::clone(name),
        }
        .into());
    }
    let compiled = zonked(elaborator, &compiled)?;
    let emitted = emission(elaborator, here, &compiled)?;
    let lifted = Arc::new(held(
        Body::Compiled {
            tree: Arc::new(compiled),
            term: emitted,
        },
        lifted_ty,
    ));
    globals.lift(&lifted);
    // Outermost first, which is the order the Πs were wrapped in, so binder `p`
    // of the telescope is `depth - 1 - p` steps out from here.
    let depth = scope.depth().0;
    let mut term = Term::named_at(here, Arc::clone(&lifted.name), Role::Defined, Levels::NONE);
    for position in 0..u32::try_from(telescope.len()).unwrap_or(u32::MAX) {
        let steps_out = depth.saturating_sub(position).saturating_sub(1);
        term = Term::app(here, term, Term::var(here, Index(steps_out)));
    }
    Ok(term)
}

/// A definition's body as binders and a tree, or `None` when it is not one.
///
/// `prefix` is the binders already standing in `scope` that the definition
/// abstracts — the context a [`lift`] closed over, and nothing for a definition
/// written at a document's top. The λs of `body` are peeled against the Π chain
/// of `goal`, an unwritten parameter binder is abstracted exactly as
/// [`check`](crate::check)'s own rule does, and what is left has to be a
/// `match` that needed no `let`.
///
/// # Errors
///
/// As [`crate::elaboration::case::tree`] — a `match` that misses a
/// constructor, names one twice, or fails to elaborate an arm. A body that is
/// merely not a tree is `Ok(None)`, so that the caller can say what *that*
/// means for the definition it is building.
pub(crate) fn tree_body(
    elaborator: &mut Elaborator,
    scope: &Scope,
    prefix: &[Name],
    body: &Raw,
    goal: &Value,
) -> Result<Option<Compiled>, ElabError> {
    let mut binders: Vec<Name> = prefix.to_vec();
    let mut inner = scope.clone();
    let mut goal = goal.clone();
    let mut at = body;
    loop {
        let forced = opened(elaborator.meter(), &goal)?;
        let Form::Pi {
            filling: expected,
            name: unwritten,
            domain,
            codomain,
        } = &forced.as_ref().unwrap_or(&goal).form
        else {
            break;
        };
        let (expected, unwritten, domain, codomain) = (
            expected.clone(),
            Arc::clone(unwritten),
            Arc::clone(domain),
            codomain.clone(),
        );
        let written = if let RawShape::Lam {
            filling,
            name,
            domain: annotation,
            body: rest,
        } = at.shape()
            && *filling == expected
        {
            Some((Arc::clone(name), annotation.clone(), rest))
        } else {
            None
        };
        let name = match written {
            Some((ref name, _, _)) => Arc::clone(name),
            // A binder the type wants and the author did not write is
            // abstracted for them, which is `check`'s `abstracted` rule.
            None if expected == Filling::Written => break,
            None => unwritten,
        };
        // An annotation on a binder whose type is already known is not ignored:
        // it is elaborated and made to agree, so a wrong one is a refusal
        // rather than dead text.
        if let Some((_, Some(ref annotation), _)) = written {
            let (term, _) = elaborator.check_type(&inner, annotation)?;
            let stated = inner.eval(elaborator.meter(), &term)?;
            elaborator.same_types(&inner, annotation.origin(), &domain, &stated)?;
        }
        let variable = inner.fresh_var(at.origin(), Arc::clone(&domain));
        goal = apply_closure(elaborator.meter(), &codomain, variable)?;
        inner = inner.assume(Some(Arc::clone(&name)), at.origin(), domain);
        binders.push(name);
        if let Some((_, _, rest)) = written {
            at = rest;
        }
    }
    let RawShape::Match { subjects, arms } = at.shape() else {
        return Ok(None);
    };
    let (tree, bound) = crate::elaboration::case::tree(elaborator, &inner, at.origin(), subjects, arms, &goal)?;
    // A subject the builder had to name is a `let` around the emitted term, and
    // a tree body is binders and a tree with nothing in between.
    if !bound.is_empty() {
        return Ok(None);
    }
    Ok(Some(Compiled {
        binders: Arc::from(binders),
        tree,
    }))
}

/// A tree body as the term a reader gets: the emission, under one λ per binder.
///
/// See [`Body::Compiled`]'s `term` for what it is for. Zonked, because it is
/// stored and read again after this elaboration has ended.
///
/// # Errors
///
/// As [`CaseTree::emitted`], and as zonking.
pub(crate) fn emission(elaborator: &mut Elaborator, here: Origin, compiled: &Compiled) -> Result<Term, ElabError> {
    let emitted = compiled.tree.emitted()?;
    let term = compiled
        .binders
        .iter()
        .rev()
        .fold(emitted, |built, name| Term::lam(here, Arc::clone(name), built));
    elaborator.zonk(&term)
}

/// The same tree with every metavariable this elaboration solved written in.
///
/// The tree is stored and reduced long after the elaboration that built it, so
/// what it holds has to be the solutions rather than the unknowns — the same
/// reason a definition's written term is zonked before it is kept.
pub(crate) fn zonked(elaborator: &mut Elaborator, compiled: &Compiled) -> Result<Compiled, ElabError> {
    let depth = deeper(Level::ZERO, compiled.binders.len());
    Ok(Compiled {
        binders: Arc::clone(&compiled.binders),
        tree: zonked_tree(elaborator, &compiled.tree, depth)?,
    })
}

/// `depth` with `more` binders added to it.
fn deeper(depth: Level, more: usize) -> Level {
    (0..more).fold(depth, |level, _| level.deeper())
}

/// The walk [`zonked`] is the top of, carrying the depth each node stands at.
///
/// The depth is the whole reason this is not [`Elaborator::zonk`] applied to
/// each term: a tree's binders are beside it rather than around it, so nothing
/// in an `Answer` says how many λs enclose it. See [`Elaborator::zonk_at`].
fn zonked_tree(elaborator: &mut Elaborator, tree: &CaseTree, depth: Level) -> Result<CaseTree, ElabError> {
    match tree {
        CaseTree::Answer(term) => Ok(CaseTree::Answer(elaborator.zonk_at(term, depth)?)),
        CaseTree::Impossible => Ok(CaseTree::Impossible),
        CaseTree::Split(split) => {
            let mut params = Vec::with_capacity(split.params.len());
            for param in split.params.iter() {
                params.push(elaborator.zonk_at(param, depth)?);
            }
            let mut motives = Vec::with_capacity(split.motives.len());
            for motive in split.motives.iter() {
                motives.push(elaborator.zonk_at(motive, depth)?);
            }
            let mut alternatives = Vec::with_capacity(split.alternatives.len());
            for alternative in split.alternatives.iter() {
                // A method binds every field and then every hypothesis, which
                // is the order `kernel::family` assembles its type in.
                let inner = deeper(
                    depth,
                    alternative.fields.len().saturating_add(alternative.hypotheses.len()),
                );
                alternatives.push(Alternative {
                    constructor: Arc::clone(&alternative.constructor),
                    fields: Arc::clone(&alternative.fields),
                    hypotheses: Arc::clone(&alternative.hypotheses),
                    body: zonked_tree(elaborator, &alternative.body, inner)?,
                });
            }
            Ok(CaseTree::Split(Box::new(Split {
                origin: split.origin,
                group: Arc::clone(&split.group),
                family: split.family,
                params: Arc::from(params),
                motives: Arc::from(motives),
                level: split.level.clone(),
                on: elaborator.zonk_at(&split.on, depth)?,
                alternatives: Arc::from(alternatives),
            })))
        }
    }
}

/// What to call a lifted definition: the enclosing definition, then the name
/// the author gave the `rec`.
///
/// `#` is what makes it safe — no source identifier holds one, so a lifted
/// definition cannot be named, shadowed, or collided with by any program. The
/// count is appended only when one definition lifts two `rec`s of the same
/// name, which is the one way the pair can repeat.
fn lifted_name(elaborator: &Elaborator, globals: &Globals, name: &Name) -> Name {
    let enclosing = elaborator.declared_name().unwrap_or_else(|| Arc::from("_"));
    let first: Name = Arc::from(format!("{enclosing}#{name}"));
    if globals.defined(&first).is_none() {
        return first;
    }
    Arc::from(format!("{first}{}", globals.lifted().len()))
}

/// Whether `raw` names `name` freely — whether, that is, the `rec` recurses.
///
/// Shadowing is respected, so a `rec walk` whose body binds its own `walk`
/// somewhere inside is not made recursive by the inner one. Asked of raw syntax
/// because the answer decides whether anything is elaborated at all.
fn names(raw: &Raw, name: &Name) -> bool {
    let mut bound = Vec::new();
    free(raw, name, &mut bound)
}

fn free(raw: &Raw, name: &Name, bound: &mut Vec<Name>) -> bool {
    let under = |raw: &Raw, binders: &[Name], bound: &mut Vec<Name>| {
        let depth = bound.len();
        bound.extend(binders.iter().map(Arc::clone));
        let found = free(raw, name, bound);
        bound.truncate(depth);
        found
    };
    match raw.shape() {
        RawShape::Var(written) => **written == **name && !bound.contains(written),
        RawShape::Hosted(_) | RawShape::Universe(_) | RawShape::Lit(_) | RawShape::Numeral { .. } => false,
        RawShape::Pi {
            name: binder,
            domain,
            codomain,
            ..
        } => free(domain, name, bound) || under(codomain, std::slice::from_ref(binder), bound),
        RawShape::Lam {
            name: binder,
            domain,
            body,
            ..
        } => {
            domain.as_ref().is_some_and(|written| free(written, name, bound))
                || under(body, std::slice::from_ref(binder), bound)
        }
        RawShape::Method { receiver, .. } => free(receiver, name, bound),
        RawShape::Project { record, .. } => free(record, name, bound),
        RawShape::App { function, argument, .. } => free(function, name, bound) || free(argument, name, bound),
        RawShape::Call {
            function,
            arguments,
            supplied,
        } => {
            free(function, name, bound)
                || arguments.iter().any(|argument| free(argument, name, bound))
                || fields(supplied, name, bound)
        }
        RawShape::RecordType(written) | RawShape::Record(written) => fields(written, name, bound),
        RawShape::Update { record, updates } => {
            free(record, name, bound)
                || updates
                    .iter()
                    .any(|update: &RawUpdate| free(&update.value, name, bound))
        }
        RawShape::Let {
            name: binder,
            ty,
            value,
            body,
        } => {
            ty.as_ref().is_some_and(|written| free(written, name, bound))
                || free(value, name, bound)
                || under(body, std::slice::from_ref(binder), bound)
        }
        RawShape::Annot { term, ty } => free(term, name, bound) || free(ty, name, bound),
        RawShape::Match { subjects, arms } => {
            subjects.iter().any(|subject| free(subject, name, bound))
                || arms.iter().any(|arm| {
                    let mut binders = Vec::new();
                    for pattern in &arm.patterns {
                        pattern_binders(pattern, &mut binders);
                    }
                    under(&arm.body, &binders, bound)
                })
        }
        // An inner `rec` binds its own name, and its recursion is its own.
        RawShape::Rec { name: binder, ty, body } => {
            free(ty, name, bound) || under(body, std::slice::from_ref(binder), bound)
        }
    }
}

fn fields(written: &[RawField], name: &Name, bound: &mut Vec<Name>) -> bool {
    written.iter().any(|field| free(&field.term, name, bound))
}

/// The names a pattern binds, appended in order.
fn pattern_binders(pattern: &RawPattern, into: &mut Vec<Name>) {
    match pattern {
        RawPattern::Bind { name, .. } => into.push(Arc::clone(name)),
        RawPattern::Constructor { fields, .. } => {
            for field in fields {
                pattern_binders(field, into);
            }
        }
        RawPattern::Record { fields, .. } => {
            for (_, field) in fields {
                pattern_binders(field, into);
            }
        }
    }
}
