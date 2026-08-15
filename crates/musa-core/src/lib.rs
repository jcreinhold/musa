//! The dependently typed core the source language elaborates into
//! (`docs/rules/language/02-core-calculus.md`), and a **leaf**: it depends on
//! no other Musa crate and knows nothing about pitch, time, notation, or audio.
//!
//! Owns: core terms, typing contexts, universes with their levels, dependent
//! function types, primitive dependent records with η, the identity type with
//! `refl` and `J`, non-recursive `let`, and definitional equality decided by
//! normalization by evaluation under a deterministic budget (prompt 133);
//! bidirectional elaboration with contextual metavariables, pattern-fragment
//! unification, and implicit arguments (prompt 134); and parameterized and
//! indexed inductive families with strict positivity, generated dependent
//! recursors, dependent `match` compiled to them through case trees with
//! coverage, and the checked termination rule (prompt 135).
//!
//! **The core is complete: everything above it is library code.** Records,
//! enums, traits, `Syntax<Cat>`, and the collections are elaborated *into* this
//! calculus by later prompts and add nothing to it. `Storable` and every
//! musical type belong to stages above this one and must never arrive here — a
//! core that knows what a duration is has stopped being the part that has to be
//! provably right.
//!
//! # The facade, and the one thing it will not show you
//!
//! Three operations and a context. [`Cx`] carries the binders a term is read
//! under and the budget its conversions run in; [`normalize`] answers a normal
//! form; [`convertible`] answers whether two terms are definitionally equal.
//!
//! **`Value` — the semantic domain `NbE` evaluates into — stays private**, along
//! with the evaluator, the environment, and quotation. `docs/plan/roadmap.md`
//! §15.12 fixes that boundary and the argument is worth having in the code,
//! because the pressure to leak it arrives with the first caller that wants to
//! inspect a normal form. Prompt 134's elaborator will want to check against a
//! *value* type rather than re-normalize at every step, and exposing `Value`
//! would let it. The answer is that the elaborator then belongs in this crate,
//! not that `Value` belongs in the facade: closures hold the evaluator's own
//! representation, so every later change to evaluation would otherwise be a
//! breaking change for `musa-compiler`.
//!
//! # Why the facade is type-directed
//!
//! §3 says η at Π and at records is performed by quotation rather than by a
//! conversion rule. Quotation therefore needs the type: without one there is
//! nothing to η-expand *against*, and `f` and `λx. f x` would read back
//! differently even though §3 calls them equal. That is why every operation
//! here takes the type its terms are at, and why §15.12's sketch already spells
//! `convertible(left, right, ty)`.
//!
//! # Invariants
//!
//! - **Terms are de Bruijn-indexed; values are de Bruijn-levelled.** α-
//!   equivalence is structural equality on terms, and quotation names a fresh
//!   variable without renaming anything. [`Index`](term::Index) and
//!   [`DbLevel`](term::DbLevel) are separate types so that confusing them is a
//!   compile error rather than the classic bug in this construction.
//! - **Reduction is never performed on syntax.** There is no substitution
//!   function in this crate; β, δ, and ι are steps in the semantic domain.
//! - **Universes are predicative and not cumulative.** `Type l : Type (succ l)`,
//!   conversion compares levels for equality, and there is no subtyping inside
//!   conversion.
//! - **Records are primitive with η**, not Σ sugar, so two records with the same
//!   projections are convertible without a rule that inspects both at once —
//!   the property `10-traits.md`'s coherence argument rests on.
//! - **One evaluator.** The one that decides conversion and the one that will
//!   run an accepted program are the same, because a second would be a second
//!   semantics obliged to agree with the first by a law nobody could state.
//! - **The budget may end an operation and may never change one that
//!   finished** (§4). Exhaustion is its own outcome, not a negative answer.
//! - **Every term carries an [`Origin`], and no comparison looks at it** (§7).
//!   Provenance is a property of the representation, preserved by evaluation and
//!   by quotation, so a diagnostic about a normal form can still point at
//!   source. It is excluded from equality by [`Term`]'s own `PartialEq`, because
//!   a compiler that type-checked differently after a file was moved would be
//!   the alternative.
//!
//! # What this crate does not do
//!
//! [`normalize`] and [`convertible`] do not type-check what they are given.
//! Typing is [`check`], [`infer`], and [`well_typed`]; a term that reaches the
//! evaluator having projected a field from a function is not refused, it is
//! [`CoreError::Malformed`] — a caller defect, reported rather than panicked
//! on, because a total language that aborts has replaced a diagnostic with a
//! crash.

mod budget;
mod case;
mod class;
mod context;
mod declare;
mod dictionary;
mod elab;
mod error;
mod eval;
mod family;
mod level;
mod list;
mod meta;
mod origin;
mod quote;
mod raw;
mod rec;
mod recheck;
mod refuse;
mod scope;
mod storable;
mod term;
mod unify;
mod value;
mod visibility;

pub use crate::budget::{Budget, Metric, ResourceError};
pub use crate::class::{Instance, PackageId, Trait};
pub use crate::context::Cx;
pub use crate::error::{CoreError, Malformed};
pub use crate::family::{Binder, Constant, Constructor, Declared, Group};
pub use crate::level::Level;
pub use crate::meta::{Meta, MetaSource};
pub use crate::origin::Origin;
pub use crate::raw::{
    Raw, RawArm, RawBinder, RawConstraint, RawConstructor, RawData, RawDefinition, RawFamily, RawField, RawImpl,
    RawMethod, RawPattern, RawShape, RawTrait,
};
pub use crate::recheck::well_typed;
pub use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
pub use crate::term::{DbLevel, Field, Index, Name, Plicity, Shape, Term};
pub use crate::visibility::{ModuleId, Visibility};

use std::sync::Arc;

use crate::elab::Elaborator;
use crate::eval::eval;
use crate::quote::{quote, quote_type};
use crate::scope::Scope;
use crate::unify::Unifier;

/// Elaborate a `data` declaration group, in context `cx`.
///
/// One call declares *all* the families that may mention each other, because
/// mutual recursion is not a relation between two finished declarations — a
/// constructor of the first may store the second, so neither exists until both
/// do (§1.1).
///
/// The result is opaque on purpose. A caller brings the declaration into scope
/// with [`Cx::declaring`] and then writes `Vec`, `Vec.Cons`, and `Vec.elim` in
/// ordinary raw terms; it never assembles a constructor's type itself, because
/// the recursor's is a term nobody wrote and the group is the only thing that
/// knows how to build it.
///
/// # Errors
///
/// [`Refusal::NonPositive`] for an occurrence §1.1 forbids, and otherwise as
/// [`check`] — a declaration's parameters, indices, fields, and chosen index
/// arguments are ordinary elaboration and fail in the ordinary ways.
pub fn declare(cx: &Cx, data: &RawData) -> Result<Arc<Group>, ElabError> {
    crate::declare::declare(cx, data)
}

/// Elaborate a `trait` declaration, in context `cx`.
///
/// The result is a dictionary *record type* wrapped in whatever the declaration
/// said about it, and a caller brings it into scope with
/// [`Cx::declaring_class`]. It never sees the record: `10-traits.md` §1 makes a
/// trait a record of methods, but a caller that assembled that record itself
/// could assemble one the derived methods do not fit.
///
/// # Errors
///
/// [`Refusal::ReservedClass`] for a trait named `Storable`,
/// [`Refusal::HeadlessClass`] for one with no parameters,
/// [`Refusal::DuplicateMethod`] for a repeated name, and otherwise as
/// [`check`] — a trait's parameters, constraints, and method types are ordinary
/// elaboration and fail in the ordinary ways.
pub fn declare_trait(cx: &Cx, raw: &RawTrait) -> Result<Arc<Trait>, ElabError> {
    crate::dictionary::declare_trait(cx, raw)
}

/// Elaborate an `impl` declaration, in context `cx`.
///
/// Every check `10-traits.md` makes at a declaration happens here and none
/// happens at a use site, which is §4's rule and not an implementation choice:
/// the author reading a use-site failure is not the author who can fix it.
///
/// # Errors
///
/// [`Refusal::DuplicateInstance`] naming both declarations,
/// [`Refusal::OrphanInstance`], [`Refusal::UnboundedInstance`],
/// [`Refusal::BlanketInstance`], [`Refusal::HandWrittenStorable`], the method
/// mismatches [`Refusal::DerivedMethod`], [`Refusal::NoSuchMethod`] and
/// [`Refusal::MissingMethod`], and otherwise as [`check`].
pub fn declare_impl(cx: &Cx, raw: &RawImpl) -> Result<Arc<Instance>, ElabError> {
    crate::dictionary::declare_impl(cx, raw)
}

/// Elaborate `raw` against the type `ty`, in context `cx`.
///
/// The output is a core term with **no metavariables left in it**: §2.1 never
/// defaults and never generalizes, so one still undetermined here is
/// [`Refusal::Unsolved`] rather than a hole the next stage inherits. It is
/// independently re-checkable, which is what [`well_typed`] is for and the
/// single most valuable invariant in this crate.
///
/// `ty` is a [`Term`] rather than the semantic type elaboration actually works
/// against, and that is roadmap §15.12's boundary holding: the caller has a type
/// it wrote, and the value it evaluates to is this crate's business.
///
/// # Errors
///
/// [`ElabError::Refused`] when the program is wrong, [`ElabError::Exhausted`]
/// when the budget ended the judgment — which is **not** a type error — and
/// [`ElabError::Malformed`] when `ty` is not a term this crate could produce.
pub fn check(cx: &Cx, ty: &Term, raw: &Raw) -> Result<Term, ElabError> {
    let mut elaborator = Elaborator::new(cx);
    let scope = Scope::new(cx);
    let ty = scope.eval(&mut cx.meter(), ty)?;
    elaborator.run_check(&scope, raw, &ty)
}

/// Elaborate `raw`, answering it and the type it was found to have.
///
/// Not every term has one. §2 gives the introduction forms — a record literal
/// above all — a rule that *reads* a type rather than producing one, because the
/// type such a term "obviously" has is a guess and not a principal type:
/// `{ ty = {}, val = {} }` inhabits `{ ty : Type 0, val : ty }` and
/// `{ ty : Type 0, val : {} }` equally. Those are [`Refusal::Uninferable`] here,
/// and the answer is [`check`] with the type the author meant.
///
/// # Errors
///
/// As [`check`].
pub fn infer(cx: &Cx, raw: &Raw) -> Result<(Term, Term), ElabError> {
    Elaborator::new(cx).run_infer(&Scope::new(cx), raw)
}

/// The normal form of `term` at type `ty`, in context `cx`.
///
/// The result is η-long: `f : A → B` normalizes to `λx. f x`, and a record
/// normalizes to a literal holding all of its projections. That is what makes
/// [`convertible`] an α-comparison of normal forms rather than a second
/// algorithm — and what makes "conversion agrees with normalization" a law
/// rather than an approximation.
///
/// # Errors
///
/// [`CoreError::Exhausted`] when the deterministic budget ends the operation,
/// [`CoreError::Malformed`] when the term does not fit the shape `ty` demands.
pub fn normalize(cx: &Cx, ty: &Term, term: &Term) -> Result<Term, CoreError> {
    let mut meter = cx.meter();
    let ty = eval(&mut meter, cx.env(), ty)?;
    let value = eval(&mut meter, cx.env(), term)?;
    quote(&mut meter, cx.quoting_depth(), &ty, &value)
}

/// The normal form of a type.
///
/// A type's own type is a universe, which tells η nothing, so this is the one
/// case that cannot go through [`normalize`] without inventing a level to state
/// it at.
///
/// # Errors
///
/// As [`normalize`].
pub fn normalize_type(cx: &Cx, ty: &Term) -> Result<Term, CoreError> {
    let mut meter = cx.meter();
    let value = eval(&mut meter, cx.env(), ty)?;
    quote_type(&mut meter, cx.quoting_depth(), &value)
}

/// Whether `left` and `right` are definitionally equal at type `ty`.
///
/// β, η at Π and at records, δ, and ι (§3), decided by the same procedure the
/// checker's `Switch` rule calls — a type-directed walk over both values that
/// stops at the first node they disagree on, with every metavariable treated as
/// an opaque head rather than an unknown to solve for. Normalizing both sides
/// and comparing was the same answer computed the most expensive way available,
/// and it was a second implementation of a question the unifier already
/// answers; `conversion_laws.rs` keeps that version as the oracle this one is
/// checked against.
///
/// The answer is a `bool` *inside* a `Result` rather than a bare `bool`, and
/// that is §4's three-outcome law rather than Rust habit: `Ok(false)` means the
/// two terms are different, `Err(Exhausted)` means the question was not
/// answered, and a signature that could not tell those apart would let a
/// resource limit silently decide a program's meaning.
///
/// # Errors
///
/// As [`normalize`].
pub fn convertible(cx: &Cx, ty: &Term, left: &Term, right: &Term) -> Result<bool, CoreError> {
    let mut meter = cx.meter();
    let ty = eval(&mut meter, cx.env(), ty)?;
    let left = eval(&mut meter, cx.env(), left)?;
    let right = eval(&mut meter, cx.env(), right)?;
    decided(Unifier::deciding().unify(&mut meter, cx.depth(), Origin::UNKNOWN, &ty, &left, &right))
}

/// Whether two *types* are definitionally equal.
///
/// # Errors
///
/// As [`normalize`].
pub fn convertible_types(cx: &Cx, left: &Term, right: &Term) -> Result<bool, CoreError> {
    let mut meter = cx.meter();
    let left = eval(&mut meter, cx.env(), left)?;
    let right = eval(&mut meter, cx.env(), right)?;
    decided(Unifier::deciding().unify_types(&mut meter, cx.depth(), Origin::UNKNOWN, &left, &right))
}

/// A conversion question's answer, read off what the unifier did.
///
/// The one place §4's three outcomes are folded back into two: a refusal *is*
/// the negative answer, so it becomes `Ok(false)`, while exhaustion stays an
/// error because the question was not answered. The mismatch's own report — the
/// pair of subterms and the path to them — is what the checker prints and what
/// a `bool` has no room for, so it is dropped here rather than never built.
fn decided(outcome: Result<(), ElabError>) -> Result<bool, CoreError> {
    match outcome {
        Ok(()) => Ok(true),
        Err(ElabError::Refused(_)) => Ok(false),
        Err(ElabError::Exhausted(exhausted)) => Err(CoreError::Exhausted(exhausted)),
        Err(ElabError::Malformed(malformed)) => Err(CoreError::Malformed(malformed)),
    }
}
