//! The dependently typed core the source language elaborates into
//! (`docs/rules/language/02-core-calculus.md`), and a **leaf**: it depends on
//! no other Musa crate and knows nothing about pitch, time, notation, or audio.
//!
//! Owns, as of prompt 133: core terms, typing contexts, universes with their
//! levels, dependent function types, primitive dependent records with η, the
//! identity type with `refl` and `J`, non-recursive `let`, and definitional
//! equality decided by normalization by evaluation under a deterministic
//! budget.
//!
//! Absent by design and named so nobody looks for them: bidirectional
//! elaboration, metavariables, and unification are prompt 134's; inductive
//! families, dependent `match`, coverage, and the termination checker are
//! prompt 135's. `Storable`, traits, `Syntax`, and every musical type belong to
//! stages above this one and must never arrive here — a core that knows what a
//! duration is has stopped being the part that has to be provably right.
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
//!
//! # What this crate does not do
//!
//! It does not type-check. A term that projects a field from a function is not
//! refused, it is [`CoreError::Malformed`] — a caller defect, reported rather
//! than panicked on, because a total language that aborts has replaced a
//! diagnostic with a crash.

mod budget;
mod context;
mod error;
mod eval;
mod level;
mod quote;
mod term;
mod value;

pub use crate::budget::{Budget, Metric, ResourceError};
pub use crate::context::Cx;
pub use crate::error::{CoreError, Malformed};
pub use crate::level::Level;
pub use crate::term::{DbLevel, Field, Index, Name, Term};

use crate::eval::eval;
use crate::quote::{quote, quote_type};

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
/// β, η at Π and at records, δ, and ι, decided by normalizing both sides and
/// comparing up to α (§3).
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
    Ok(normalize(cx, ty, left)? == normalize(cx, ty, right)?)
}

/// Whether two *types* are definitionally equal.
///
/// # Errors
///
/// As [`normalize`].
pub fn convertible_types(cx: &Cx, left: &Term, right: &Term) -> Result<bool, CoreError> {
    Ok(normalize_type(cx, left)? == normalize_type(cx, right)?)
}
