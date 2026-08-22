//! The dependently typed core the source language elaborates into
//! (`docs/rules/language/02-core-calculus.md`), and a **leaf**: it depends on
//! no other Musa crate and knows nothing about pitch, time, notation, or audio.
//!
//! Owns: core terms, typing contexts, two fixed universes, dependent function
//! types, primitive dependent records with η, non-recursive `let`, and
//! definitional equality decided by normalization by evaluation under a
//! deterministic budget (prompt 133); bidirectional elaboration with
//! first-order metas and type parameters (prompt 134); and parameterized
//! inductive families with strict positivity, generated non-dependent
//! recursors, `match` compiled to them through case trees with coverage, and
//! the checked structural termination rule (prompt 135).
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
//!   variable without renaming anything. [`Index`] and [`Level`] are separate
//!   types so that confusing them is a compile error rather than the classic
//!   bug in this construction.
//! - **Reduction is never performed on syntax.** There is no substitution
//!   function in this crate; β, δ, and ι are steps in the semantic domain.
//! - **Universes are predicative and not cumulative.** `Type l : Type (succ l)`,
//!   conversion compares [`Sort`]s for equality, and there is no subtyping
//!   inside conversion.
//! - **Records are primitive with η**, not Σ sugar, so two records with the same
//!   projections are convertible without a rule that inspects both at once.
//!   Prompt 157 turns records into data and this is what it must preserve.
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
//! - **The kernel does not know the elaborator exists.** `kernel/` decides
//!   typing and definitional equality on finished [`Term`]s and answers
//!   [`CoreError`]; `elaboration/` reads a [`Raw`] and answers [`Refusal`], and
//!   reads the kernel to do it. The dependency is one-way. Rust cannot enforce
//!   that between sibling modules — a descendant may always name `crate::`, and
//!   the re-exports below put [`Refusal`] within reach of every file — so the
//!   direction is a law rather than a keyword, checked over the source text by
//!   `tests/suite/boundary_laws.rs`.
//!
//! # What this crate does not do
//!
//! [`normalize`] and [`convertible`] do not type-check what they are given.
//! Typing is [`check`] and [`infer`]; a term that reaches the
//! evaluator having projected a field from a function is not refused, it is
//! [`CoreError::Malformed`] — a caller defect, reported rather than panicked
//! on, because a total language that aborts has replaced a diagnostic with a
//! crash.

mod elaboration;
mod kernel;

pub use crate::elaboration::raw::{
    ARROW_BINDER, Raw, RawArm, RawBinder, RawConstructor, RawData, RawDefinition, RawFamily, RawField, RawPattern,
    RawProgram, RawShape, RawTopLevel,
};
pub use crate::elaboration::refuse::{ElabError, Mismatch, PathStep, Refusal};
pub use crate::elaboration::storable::requiring_storable;
pub use crate::kernel::base::{
    Accepts, Answer, Base, Builtin, Datum, Extern, Family, Literal, Measures, Operator, Payload, Registry, Rewrite,
    Rule,
};
pub use crate::kernel::budget::{Budget, Metric, ResourceError, Spend};
pub use crate::kernel::checked::Checked;
pub use crate::kernel::context::Cx;
pub use crate::kernel::error::{CoreError, Malformed};
pub use crate::kernel::family::{Constructor, Declared, Group, canonical};
pub use crate::kernel::meta::MetaSource;
pub use crate::kernel::origin::Origin;
pub use crate::kernel::program::{Def, Program};
pub use crate::kernel::sort::Sort;
pub use crate::kernel::term::Constraint;
pub use crate::kernel::term::{Binder, Constant, Field, Filling, Index, Level, Name, Role, Shape, Term};
pub use crate::kernel::visibility::{ModuleId, Visibility};

use std::sync::Arc;

use crate::elaboration::convert::Conversion;
use crate::elaboration::elab::Elaborator;
use crate::kernel::eval::eval;
use crate::kernel::quote::{Mode, quote, quote_type};
use crate::kernel::room::with_room;
use crate::kernel::scope::Scope;

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
    declare_metered(cx, data).map(|(group, _)| group)
}

/// [`declare`], and what elaborating the declaration charged.
///
/// # Errors
///
/// As [`declare`].
pub fn declare_metered(cx: &Cx, data: &RawData) -> Result<(Arc<Group>, Spend), ElabError> {
    with_room(|| crate::elaboration::declare::declare(cx, data))
}

/// Elaborate a document's top-level definitions, in context `cx`.
///
/// The other half of a document, and the same arrangement [`declare`] has for
/// the first: one call takes all of them, because `02-core-calculus.md` §2.4
/// lets a body name a declaration written later, and the result is brought into
/// scope with [`Cx::defining`].
///
/// A namespaced definition — `Pitch.act`, what an `impl Pitch { … }` block
/// writes — is one of these and not a second kind. That is the whole of what
/// prompt 146 left where the instance table used to be: a dotted name, declared
/// in the same group as every other definition, so that a body naming `p.act(i)`
/// and the definition answering it are ordered by the same dependency analysis.
///
/// # Errors
///
/// [`Refusal::DefinitionCycle`] for definitions that name each other, except
/// through a bare member spelling: `x.m(y)` stands for "whichever namespace
/// declares `m`" rather than for a particular definition, so an edge the
/// analysis guessed at is dropped rather than refused, and a genuine need is
/// refused where it is written as the call's [`Refusal::NoMethodForType`].
/// [`Refusal::UntypedRecursion`] for a self-recursive definition that wrote no
/// type, and otherwise as [`check`].
pub fn declare_program(cx: &Cx, program: &RawProgram) -> Result<Arc<Program>, ElabError> {
    declare_program_metered(cx, program).map(|(declared, _)| declared)
}

/// [`declare_program`], and what elaborating the group charged.
///
/// The sum over its definitions, which is what a caller keeping a budget of its
/// own wants: `26-language-design-decision.md` §3.5 charges the
/// expansion phase for reading its adapter module, and reading a module *is*
/// declaring the program it holds.
///
/// # Errors
///
/// As [`declare_program`].
pub fn declare_program_metered(cx: &Cx, program: &RawProgram) -> Result<(Arc<Program>, Spend), ElabError> {
    with_room(|| crate::elaboration::declare_program::declare_program(cx, program))
}

/// Elaborate `raw` against the type `ty`, in context `cx`.
///
/// The output is a core term with **no metavariables left in it**: §2.1 never
/// defaults and never generalizes, so one still undetermined here is
/// [`Refusal::Unsolved`] rather than a meta the next stage inherits. Every
/// argument the term applies is written in it, which is the single most
/// valuable invariant in this crate.
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
    with_room(|| {
        let mut elaborator = Elaborator::new(cx);
        let scope = Scope::new(cx);
        let ty = scope.eval(&mut cx.meter(), ty)?;
        elaborator.run_check(&scope, raw, &ty)
    })
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
    Ok(infer_metered(cx, raw)?.0)
}

/// The same, and what it charged.
///
/// For a caller that keeps a budget of its own — the expansion phase does, and
/// two of `26-language-design-decision.md` §3.5's four counters are elaboration
/// work rather than the phase's. A separate entry point rather than a second
/// return value on [`infer`], because almost nobody is counting and a spend
/// every caller had to ignore would be a parameter this crate charges everyone
/// for.
///
/// # Errors
///
/// As [`infer`].
pub fn infer_metered(cx: &Cx, raw: &Raw) -> Result<((Term, Term), Spend), ElabError> {
    with_room(|| {
        let mut elaborator = Elaborator::new(cx);
        let inferred = elaborator.run_infer(&Scope::new(cx), raw)?;
        Ok((inferred, elaborator.spent()))
    })
}

/// Re-derive `term`'s type with the kernel alone, and require it to be `ty`.
///
/// The operation `crates/musa-calculus/TRUST.md` is about. Elaboration is
/// outside the trusted computing base, so what it produces is audited rather
/// than believed: this walks the finished term with the kernel's rules, deriving
/// a type for every subterm and comparing by `quote ∘ eval`. It runs behind a
/// debug assertion on every elaborated declaration already, and a conformance
/// suite runs it unconditionally.
///
/// It takes a [`Checked`] rather than a [`Term`] because "no unsolved
/// metavariable crosses this line" is the first of the three acceptance
/// invariants, and a signature is a better place to keep an invariant than a
/// paragraph.
///
/// # Errors
///
/// [`CoreError::Malformed`] when the kernel does not agree — which is a defect
/// in *this compiler*, not a verdict about the program — and
/// [`CoreError::Exhausted`] when `cx`'s budget ends the derivation, which is no
/// verdict at all.
pub fn recheck(cx: &Cx, ty: &Term, term: &Checked) -> Result<(), CoreError> {
    with_room(|| {
        let mut meter = cx.meter();
        let ty = eval(&mut meter, cx.env(), ty)?;
        crate::kernel::recheck::recheck(cx, &ty, term)
    })
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
    Ok(normalize_metered(cx, ty, term)?.0)
}

/// The same, and what it charged.
///
/// [`infer_metered`]'s counterpart one stage on, for the same caller and the
/// same reason.
///
/// # Errors
///
/// As [`normalize`].
pub fn normalize_metered(cx: &Cx, ty: &Term, term: &Term) -> Result<(Term, Spend), CoreError> {
    with_room(|| {
        let mut meter = cx.meter();
        let ty = eval(&mut meter, cx.env(), ty)?;
        let value = eval(&mut meter, cx.env(), term)?;
        let normal = quote(&mut meter, cx.depth(), Mode::Open, &ty, &value)?;
        Ok((normal, meter.spent()))
    })
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
    with_room(|| {
        let mut meter = cx.meter();
        let value = eval(&mut meter, cx.env(), ty)?;
        quote_type(&mut meter, cx.depth(), Mode::Open, &value)
    })
}

/// Whether `left` and `right` are definitionally equal at type `ty`.
///
/// β, η at Π and at records, δ, and ι (§3), decided by the same procedure the
/// checker's `Switch` rule calls — a type-directed walk over both values that
/// stops at the first node they disagree on, with every metavariable treated as
/// an opaque head rather than an unknown to solve for. Normalizing both sides
/// and comparing was the same answer computed the most expensive way available,
/// and it was a second implementation of a question the conversion checker already
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
    convertible_metered(cx, ty, left, right).map(|(answer, _)| answer)
}

/// [`convertible`], and what deciding it charged.
///
/// The folded-comparison law is stated in spend: two uses of one definition
/// compare as *references*, and the only way to say so from outside the crate
/// is to read the meter. `glued_laws.rs` is the caller.
///
/// # Errors
///
/// As [`convertible`].
pub fn convertible_metered(cx: &Cx, ty: &Term, left: &Term, right: &Term) -> Result<(bool, Spend), CoreError> {
    with_room(|| {
        let mut meter = cx.meter();
        let ty = eval(&mut meter, cx.env(), ty)?;
        let left = eval(&mut meter, cx.env(), left)?;
        let right = eval(&mut meter, cx.env(), right)?;
        let answer = decided(Conversion::deciding(cx.globals().clone()).unify(
            &mut meter,
            cx.depth(),
            Origin::UNKNOWN,
            &ty,
            &left,
            &right,
        ))?;
        Ok((answer, meter.spent()))
    })
}

/// Whether two *types* are definitionally equal.
///
/// # Errors
///
/// As [`normalize`].
pub fn convertible_types(cx: &Cx, left: &Term, right: &Term) -> Result<bool, CoreError> {
    with_room(|| {
        let mut meter = cx.meter();
        let left = eval(&mut meter, cx.env(), left)?;
        let right = eval(&mut meter, cx.env(), right)?;
        decided(Conversion::deciding(cx.globals().clone()).unify_types(
            &mut meter,
            cx.depth(),
            Origin::UNKNOWN,
            &left,
            &right,
        ))
    })
}

/// A conversion question's answer, read off what the conversion checker did.
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
