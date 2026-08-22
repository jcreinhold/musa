//! `rec f : A = e`, and §2.4's structural rule.
//!
//! # The check is the compilation
//!
//! §2.4 asks that every recursive call descend structurally, and structural
//! descent is the case the elaborator supplies for free. Here it is not
//! *checked* and then compiled — it is the compilation. A `match` on an argument already binds
//! one induction hypothesis per recursive field (`case.rs`), and the hypothesis
//! is exactly "the answer for a structurally smaller argument". So a recursive
//! call becomes a reference to that hypothesis, and a call with no hypothesis to
//! become has nowhere to go: it is [`Refusal::UncheckedRecursion`], naming the
//! call. Nothing takes a definition on trust because there is no path by which a
//! definition could be taken on trust — the core has no fixed point to emit.
//!
//! # Why the rewrite is on raw syntax
//!
//! Peyton Jones ch. 3 and ch. 6 transform *source* into a smaller calculus, and
//! this is that transformation: `f n xs'` is replaced by `xs'#ih` before
//! elaboration, and elaboration then does what it always does. Doing it after
//! elaboration would mean rewriting core terms, which this crate deliberately
//! cannot do (§1: no substitution function). Doing it during would mean
//! `case.rs` knowing what a recursive definition is, which is a second thing to
//! keep in step with this one.
//!
//! The generated name is `<field>#ih` — [`crate::elaboration::case::hypothesis_name`] — and
//! `#` is not an identifier character, so no source program can shadow one or
//! refer to one it was not given.
//!
//! # What decrease means here
//!
//! A definition recurses on **one** argument, and the `match` at the top of its
//! body is what says which: the subject that is one of the definition's own
//! arguments is the recursive position. A call is then a call on a *pattern
//! binder in that column*, and the arguments **before** that one are fixed —
//! the hypothesis is the answer at the goal this match was split at, and
//! everything abstracted before the subject is part of that goal. `xs#ih` is
//! the answer for this branch's tail at the indices that tail has, so a
//! `count k ys` is `ys#ih` and the `k` is not a second thing to check but a
//! consequence of the first.
//!
//! That last point is why "exactly one argument changed" is the wrong rule and
//! was tried first: a recursion over `Vec A n` *must* change two arguments, the
//! index and the vector, and under that rule no indexed family could be
//! recursed over at all.
//!
//! # Why the later arguments move inside the match
//!
//! An argument the definition abstracts *after* the recursive one is a
//! different matter, and getting it wrong is how a `fold` that accumulates
//! forwards came out computing its seed. `λxs. λbuilt. match xs { … }` splits
//! at the goal `B`, so the hypothesis is `B` — the answer for the tail *at this
//! branch's own accumulator* — and a call that passes a new accumulator has
//! nowhere to put it. Dropping it type-checks and means something else.
//!
//! So the binders after the recursive position are moved inside the arms before
//! the match is elaborated. The goal at the split becomes `B → B`, the
//! hypothesis becomes a function of the accumulator, and `walk t (step built h)`
//! is `t#ih (step built h)` — the strong induction hypothesis, which is what an
//! accumulating traversal has always needed. Authors write the natural
//! `λxs. λbuilt. match xs`; the transformation is what makes it mean what it
//! reads as. A binder is moved under a mangled name in an arm whose pattern
//! already binds that name, so that the arm's occurrences keep resolving to the
//! pattern's binder as they did before.
//!
//! What this refuses, and why each is genuinely out of reach:
//!
//! - A body whose top-level form is not a `match`: nothing split anything, so
//!   there is no hypothesis. `λn. loop n` is the whole of this case.
//! - A top-level `match` on two of the definition's arguments: the hypothesis
//!   from one column holds the other column's subject *fixed*, so a call that
//!   descends in both descends lexicographically. §2.4 admits no order but the
//!   structural one, and this checker supplies none.
//! - An argument in the recursive position that is not a binder of that
//!   column's pattern: `f (Succ (Succ k))` is a call on something no match made
//!   smaller.
//! - A call that changes an argument *before* the recursive one. The hypothesis
//!   holds those fixed, so `f (Succ m) k` is asking it a question it does not
//!   answer. What is admitted there is the definition's own binder, unchanged,
//!   or a binder this branch's pattern introduced — which is how an index
//!   reaches the hypothesis.
//! - The definition used as a value rather than called. A core with no fixed
//!   point has nothing to hand over.
//!
//! A `match` nested inside an arm is not consulted, and that is not an
//! oversight: its hypotheses are the recursion *it* generated, over its own
//! subject, and answer that match rather than this definition. Reading them as
//! the definition's would be the one way this rewrite could be unsound.

use std::sync::Arc;

use crate::elaboration::case::hypothesis_name;
use crate::elaboration::elab::Elaborator;
use crate::elaboration::raw::{Raw, RawArm, RawField, RawPattern, RawShape};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Filling, Name, Term};
use crate::kernel::value::Value;

/// Elaborate `rec name : ty = body` against `goal`.
///
/// # Errors
///
/// [`Refusal::UncheckedRecursion`] for a call §2.4's structural rule cannot see
/// descend, and otherwise as [`crate::check`].
pub(crate) fn define(
    elaborator: &mut Elaborator,
    scope: &Scope,
    here: Origin,
    name: &Name,
    ty: &Raw,
    body: &Raw,
    goal: &Value,
) -> Result<Term, ElabError> {
    let rewritten = Plan::read(name, body).rewritten(body)?;
    // The written type is checked against the goal by the ordinary rule: `rec`
    // is a term of the type it declares, and a `rec` in checking position owes
    // conversion like anything else.
    let annotated = Raw::annot(here, rewritten, ty.clone());
    elaborator
        .check_open(scope, &annotated, goal)
        .map_err(|error| unavailable(name, error))
}

/// A missing hypothesis, reported as the recursion it was.
///
/// The rewrite knows which *column* a call descends in but not which of that
/// constructor's fields are recursive — that is the declaration's knowledge, one
/// stage further in. So a call on a field nothing stores reaches elaboration as
/// a name that is not in scope, and `xs#ih` is a name no author wrote. Naming
/// the call instead is the honest report; the `#` is what makes the test sound,
/// since no source identifier can hold one.
fn unavailable(name: &Name, error: ElabError) -> ElabError {
    let ElabError::Refused(Refusal::UnknownName { name: missing, at, .. }) = &error else {
        return error;
    };
    if !missing.contains('#') {
        return error;
    }
    Refusal::UncheckedRecursion {
        at: *at,
        name: Arc::clone(name),
    }
    .into()
}

/// How one definition's calls become hypotheses: the argument it recurses on,
/// and the `match` column that argument is a subject of.
struct Plan {
    name: Name,
    arguments: Vec<Name>,
    recursion: Option<Recursion>,
}

/// The recursive position, as both a call's argument and a matrix's column.
struct Recursion {
    /// Which argument of a call has to be the smaller one.
    position: usize,
    /// Which subject of the top-level `match` that argument is, so that an arm's
    /// pattern in *that* column says what "smaller" means in this branch.
    column: usize,
}

impl Plan {
    /// Read a definition's shape: its arguments, and what it recurses on.
    fn read(name: &Name, body: &Raw) -> Self {
        let arguments = abstracted(body);
        let mut candidates = Vec::new();
        if let RawShape::Match { subjects, .. } = under_lambdas(body).shape() {
            for (column, subject) in subjects.iter().enumerate() {
                let RawShape::Var(subject) = subject.shape() else {
                    continue;
                };
                if let Some(position) = arguments.iter().position(|argument| argument == subject) {
                    candidates.push(Recursion { position, column });
                }
            }
        }
        // Exactly one, or none: two of the definition's own arguments split in
        // one matrix is the lexicographic case, and a hypothesis from either
        // column holds the other column's subject fixed.
        let recursion = match candidates.len() {
            1 => candidates.pop(),
            _ => None,
        };
        Self {
            name: Arc::clone(name),
            arguments,
            recursion,
        }
    }

    /// The body with every recursive call replaced by the hypothesis it is.
    fn rewritten(&self, body: &Raw) -> Result<Raw, ElabError> {
        let Some(recursion) = &self.recursion else {
            // Nothing here can be a call, so every occurrence is refused — which
            // is what a definition with no `match` to descend in deserves.
            return Rewrite {
                plan: self,
                smaller: Vec::new(),
                position: 0,
            }
            .term(body, &mut Vec::new());
        };
        let mut lambdas = Vec::new();
        let mut at = body;
        while let RawShape::Lam {
            filling,
            name,
            domain,
            body: inner,
        } = at.shape()
        {
            lambdas.push((at.origin(), filling.clone(), Arc::clone(name), domain.clone()));
            at = inner;
        }
        let RawShape::Match { subjects, arms } = at.shape() else {
            return Rewrite {
                plan: self,
                smaller: Vec::new(),
                position: 0,
            }
            .term(body, &mut Vec::new());
        };

        let outside = Rewrite {
            plan: self,
            smaller: Vec::new(),
            position: 0,
        };
        let mut rebuilt = Vec::with_capacity(subjects.len());
        for subject in subjects.iter() {
            // A subject is evaluated before anything is split, so a call there
            // has no hypothesis available and is refused like any other.
            rebuilt.push(outside.term(subject, &mut Vec::new())?);
        }
        // Everything abstracted after the recursive argument is generalized by
        // moving it inside the arms, so that the hypothesis is a function of it
        // rather than a value at this branch's own copy of it.
        let after = recursion.position.saturating_add(1).min(lambdas.len());
        let hoisted = lambdas.split_off(after);
        let mut built = Vec::with_capacity(arms.len());
        for arm in arms.iter() {
            let mut smaller = Vec::new();
            if let Some(pattern) = arm.patterns.get(recursion.column) {
                binders(pattern, &mut smaller);
            }
            let mut taken = Vec::new();
            if !hoisted.is_empty() {
                for pattern in &arm.patterns {
                    binders(pattern, &mut taken);
                }
            }
            let inside = Rewrite {
                plan: self,
                smaller,
                position: recursion.position,
            };
            let mut body = inside.term(&arm.body, &mut Vec::new())?;
            for (origin, filling, name, domain) in hoisted.iter().rev() {
                // An arm whose pattern binds this name already answers to it,
                // and did so before the move; the mangled binder takes the
                // argument without taking those occurrences with it.
                let name = if taken.contains(name) {
                    Arc::from(format!("{name}#arg"))
                } else {
                    Arc::clone(name)
                };
                body = Raw::new(
                    *origin,
                    RawShape::Lam {
                        filling: filling.clone(),
                        name,
                        domain: domain.clone(),
                        body,
                    },
                );
            }
            built.push(RawArm {
                patterns: arm.patterns.clone(),
                body,
            });
        }
        let mut rewritten = Raw::new(
            at.origin(),
            RawShape::Match {
                subjects: Arc::from(rebuilt),
                arms: Arc::from(built),
            },
        );
        for (origin, filling, name, domain) in lambdas.into_iter().rev() {
            rewritten = Raw::new(
                origin,
                RawShape::Lam {
                    filling,
                    name,
                    domain,
                    body: rewritten,
                },
            );
        }
        Ok(rewritten)
    }
}

/// The λ-bound argument names of a definition's body, outermost first.
///
/// Read off the body rather than the type, because the body is what a recursive
/// call inside it is written against: a definition may abstract fewer binders
/// than its type has arrows, and the arguments a call has to repeat are the ones
/// the author actually named.
fn abstracted(body: &Raw) -> Vec<Name> {
    let mut names = Vec::new();
    let mut at = body;
    while let RawShape::Lam { name, body: inner, .. } = at.shape() {
        names.push(Arc::clone(name));
        at = inner;
    }
    names
}

/// What a definition's λs abstract over.
fn under_lambdas(body: &Raw) -> &Raw {
    let mut at = body;
    while let RawShape::Lam { body: inner, .. } = at.shape() {
        at = inner;
    }
    at
}

/// One branch's recursive calls, being turned into hypotheses.
struct Rewrite<'a> {
    plan: &'a Plan,
    /// The names this branch's pattern bound in the recursive column.
    smaller: Vec<Name>,
    /// Which argument of a call has to be one of them.
    position: usize,
}

impl Rewrite<'_> {
    /// `raw` with every recursive call replaced by the hypothesis it is.
    ///
    /// `bound` is the names a binder inside `raw` has taken over, so that a
    /// program that shadows the definition's own name — or one of its arguments
    /// — is read the way an author reads it.
    fn term(&self, raw: &Raw, bound: &mut Vec<Name>) -> Result<Raw, ElabError> {
        let here = raw.origin();
        if let Some(rewritten) = self.call(raw, bound)? {
            return Ok(rewritten);
        }
        let shape = match raw.shape() {
            RawShape::Var(name) => {
                if **name == *self.plan.name && !bound.contains(name) {
                    // Every *call* was consumed above, so an occurrence reaching
                    // here is the definition used as a value: partially applied,
                    // passed along, or applied to arguments this rule cannot see
                    // decrease.
                    return Err(Refusal::UncheckedRecursion {
                        at: here,
                        name: Arc::clone(&self.plan.name),
                    }
                    .into());
                }
                return Ok(raw.clone());
            }
            // A universe and a literal are both closed: neither can hold a call,
            // so neither needs rewriting. A hosted name is closed for the same
            // purpose — it names the host, never the definition being measured.
            RawShape::Hosted(_) | RawShape::Universe(_) | RawShape::Lit(_) | RawShape::Numeral { .. } => {
                return Ok(raw.clone());
            }
            RawShape::Pi {
                filling,
                name,
                domain,
                codomain,
            } => RawShape::Pi {
                filling: filling.clone(),
                name: Arc::clone(name),
                domain: self.term(domain, bound)?,
                codomain: self.under(name, codomain, bound)?,
            },
            RawShape::Lam {
                filling,
                name,
                domain,
                body,
            } => RawShape::Lam {
                filling: filling.clone(),
                name: Arc::clone(name),
                domain: domain.as_ref().map(|ty| self.term(ty, bound)).transpose()?,
                body: self.under(name, body, bound)?,
            },
            RawShape::Method { receiver, method } => RawShape::Method {
                receiver: self.term(receiver, bound)?,
                method: Arc::clone(method),
            },
            RawShape::App {
                filling,
                function,
                argument,
            } => RawShape::App {
                filling: filling.clone(),
                function: self.term(function, bound)?,
                argument: self.term(argument, bound)?,
            },
            // Both halves, for the reason the arm above walks both: a call to
            // the definition being measured can stand in either, and one this
            // rule did not see is one it did not check.
            RawShape::Indexed { ty, index } => RawShape::Indexed {
                ty: self.term(ty, bound)?,
                index: self.term(index, bound)?,
            },
            // A call whose head is not the definition being defined: `spine`
            // read it as one and `self.call` declined it, so what is left is an
            // ordinary walk into the parts.
            RawShape::Call { function, arguments } => RawShape::Call {
                function: self.term(function, bound)?,
                arguments: arguments
                    .iter()
                    .map(|argument| self.term(argument, bound))
                    .collect::<Result<Vec<_>, _>>()?
                    .into(),
            },
            RawShape::RecordType(fields) => RawShape::RecordType(self.fields(fields, bound, true)?),
            RawShape::Record(fields) => RawShape::Record(self.fields(fields, bound, false)?),
            RawShape::Project { record, field } => RawShape::Project {
                record: self.term(record, bound)?,
                field: Arc::clone(field),
            },
            RawShape::Update { record, updates } => {
                let mut rewritten = Vec::with_capacity(updates.len());
                for update in updates.iter() {
                    rewritten.push(crate::elaboration::raw::RawUpdate {
                        origin: update.origin,
                        path: update.path.clone(),
                        value: self.term(&update.value, bound)?,
                    });
                }
                RawShape::Update {
                    record: self.term(record, bound)?,
                    updates: Arc::from(rewritten),
                }
            }
            RawShape::Let { name, ty, value, body } => RawShape::Let {
                name: Arc::clone(name),
                ty: ty.as_ref().map(|written| self.term(written, bound)).transpose()?,
                value: self.term(value, bound)?,
                body: self.under(name, body, bound)?,
            },
            RawShape::Annot { term, ty } => RawShape::Annot {
                term: self.term(term, bound)?,
                ty: self.term(ty, bound)?,
            },
            RawShape::Match { subjects, arms } => {
                let mut rewritten = Vec::with_capacity(subjects.len());
                for subject in subjects.iter() {
                    rewritten.push(self.term(subject, bound)?);
                }
                let mut built = Vec::with_capacity(arms.len());
                for arm in arms.iter() {
                    let depth = bound.len();
                    for pattern in &arm.patterns {
                        binders(pattern, bound);
                    }
                    let body = self.term(&arm.body, bound);
                    bound.truncate(depth);
                    built.push(RawArm {
                        patterns: arm.patterns.clone(),
                        body: body?,
                    });
                }
                RawShape::Match {
                    subjects: Arc::from(rewritten),
                    arms: Arc::from(built),
                }
            }
            // An inner `rec` binds its own name, and its calls are its own.
            RawShape::Rec { name, ty, body } => RawShape::Rec {
                name: Arc::clone(name),
                ty: self.term(ty, bound)?,
                body: self.under(name, body, bound)?,
            },
        };
        Ok(Raw::new(here, shape))
    }

    /// `raw`, rewritten under a binder named `name`.
    fn under(&self, name: &Name, raw: &Raw, bound: &mut Vec<Name>) -> Result<Raw, ElabError> {
        bound.push(Arc::clone(name));
        let rewritten = self.term(raw, bound);
        bound.pop();
        rewritten
    }

    /// A record telescope or literal.
    ///
    /// `telescope` says whether a field's name is in scope for the fields after
    /// it, which is true of a record *type* and false of a literal.
    fn fields(
        &self,
        fields: &[RawField],
        bound: &mut Vec<Name>,
        telescope: bool,
    ) -> Result<Arc<[RawField]>, ElabError> {
        let depth = bound.len();
        let mut built = Vec::with_capacity(fields.len());
        for field in fields {
            let term = self.term(&field.term, bound);
            if telescope {
                bound.push(Arc::clone(&field.name));
            }
            built.push(RawField {
                name: Arc::clone(&field.name),
                term: term?,
            });
        }
        bound.truncate(depth);
        Ok(Arc::from(built))
    }

    /// A recursive call, as the hypothesis it becomes.
    ///
    /// `None` when `raw` is not one, which is every other term.
    fn call(&self, raw: &Raw, bound: &mut Vec<Name>) -> Result<Option<Raw>, ElabError> {
        let (head, arguments) = spine(raw);
        let RawShape::Var(name) = head.shape() else {
            return Ok(None);
        };
        if **name != *self.plan.name || bound.contains(name) {
            return Ok(None);
        }
        let here = raw.origin();
        let refuse = || -> ElabError {
            Refusal::UncheckedRecursion {
                at: here,
                name: Arc::clone(&self.plan.name),
            }
            .into()
        };
        let Some(argument) = arguments.get(self.position) else {
            return Err(refuse());
        };
        let RawShape::Var(passed) = argument.shape() else {
            return Err(refuse());
        };
        // `bound` holds only the binders *inside* this arm's body, so a name the
        // pattern bound and an inner binder took over are told apart.
        if bound.contains(passed) || !self.smaller.contains(passed) {
            return Err(refuse());
        }
        // The arguments before the recursive one are part of the goal the match
        // was split at, so the hypothesis already stands at them: a call may
        // repeat the definition's own binder, or name what this branch's pattern
        // bound, which is how an index reaches the hypothesis. Anything else is
        // asking for an answer at something no hypothesis holds.
        for (index, argument) in arguments.iter().enumerate().take(self.position) {
            let RawShape::Var(passed) = argument.shape() else {
                return Err(refuse());
            };
            let own = self.plan.arguments.get(index).is_some_and(|name| name == passed);
            if bound.contains(passed) || !(own || self.smaller.contains(passed)) {
                return Err(refuse());
            }
        }
        // The arguments after it are what the hoist generalized, so they are
        // applied rather than dropped — `t#ih` is a function of exactly them.
        let mut rewritten = Raw::var(here, hypothesis_name(passed));
        for argument in arguments.iter().skip(self.position.saturating_add(1)) {
            rewritten = Raw::app(here, rewritten, self.term(argument, bound)?);
        }
        Ok(Some(rewritten))
    }
}

/// The names a pattern binds, appended in order.
fn binders(pattern: &RawPattern, into: &mut Vec<Name>) {
    match pattern {
        RawPattern::Bind { name, .. } => into.push(Arc::clone(name)),
        RawPattern::Constructor { fields, .. } => {
            for field in fields {
                binders(field, into);
            }
        }
        RawPattern::Record { fields, .. } => {
            for (_, field) in fields {
                binders(field, into);
            }
        }
    }
}

/// The head of a written application spine, and what is applied to it.
///
/// Type arguments are not counted: they are filled by elaboration, and a
/// recursive call that wrote one has written the same thing the definition's own
/// binder did.
fn spine(raw: &Raw) -> (&Raw, Vec<&Raw>) {
    // A written call already *is* the spine, with its head and arguments told
    // apart by the author rather than by a walk. Read directly, so that a
    // recursive call reaches the measure check whichever form the reader built.
    if let RawShape::Call { function, arguments } = raw.shape() {
        return (function, arguments.iter().collect());
    }
    let mut arguments = Vec::new();
    let mut head = raw;
    while let RawShape::App {
        filling: Filling::Written,
        function,
        argument,
    } = head.shape()
    {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}
