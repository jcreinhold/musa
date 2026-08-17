//! Metavariables: the holes elaboration leaves and unification fills.
//!
//! `docs/rules/language/02-core-calculus.md` §2.1 names exactly three places one
//! is created — an inserted implicit argument, a binder whose type the checking
//! type did not supply, and a level position the surface did not write — and
//! [`MetaSource`] is that list, because a diagnostic that cannot say which site
//! it came from has to say "somewhere".
//!
//! The third site is an unknown of a *different sort*: a level, not a term. It
//! is [`crate::Level`]'s own metavariable rather than a [`Meta`], since a level
//! is not a term and a solution for one is a number and not a value — but it
//! shares this list, because the question a diagnostic asks is the same one.
//! Prompt 135 opened it, and what it cost is that `succ` and `max` stop
//! computing on an unsolved arm, so every place that reads a level resolves it
//! first.
//!
//! # Contextual, and closed
//!
//! §2.1 writes a metavariable as `?α[σ]`: an unknown *together with the context
//! it may refer to*, so that a solution can never capture a variable that was
//! not in scope where the hole was made. This crate takes the standard
//! representation of that idea rather than storing a substitution: a meta is a
//! **closed** unknown of type `(x₀ : A₀) → … → (xₙ₋₁ : Aₙ₋₁) → T`, and the
//! elaborator writes it *applied to every binder in scope*. Scope safety is then
//! not a rule anyone has to enforce — a solution is a closed term, and the only
//! variables it can mention are the ones it abstracts.
//!
//! # The solution lives in the reference
//!
//! A meta is an [`Arc`] to a cell holding a write-once solution, so evaluating a
//! term that mentions one can unfold it without being handed a table. The
//! alternative — a store threaded through `eval`, `quote`, `apply`, and every
//! context operation — would put a parameter on the whole crate for the benefit
//! of one caller, and [`crate::Cx`] would have to carry it too because extending
//! a context evaluates.
//!
//! [`OnceLock`] rather than a mutex or a cell: §2's "solved metavariables are
//! stable under further solving" is then a property of the type rather than a
//! discipline. A meta is solved at most once and never revised, so a term that
//! mentions one only ever becomes *more* defined.

use std::fmt;
use std::sync::{Arc, OnceLock};

use crate::error::{CoreError, Malformed};
use crate::origin::Origin;
use crate::value::Value;

/// Which of §2.1's creation sites a metavariable came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaSource {
    /// An implicit argument the elaborator inserted at a use site.
    ImplicitArgument,
    /// A binder whose type the checking type did not supply.
    BinderType,
    /// A parameter of the family a constructor belongs to, where the type the
    /// constructor was checked against did not say what it is.
    ///
    /// `Some(x)` names a case and supplies a field; its family's parameter is
    /// read off the expected type — see
    /// [`Elaborator::constructed`](crate::elab::Elaborator). When the expected
    /// type is itself unknown, as it is at `f(Some(x))` for a generic `f`, there
    /// is nothing to read it off and the parameter becomes a hole like any
    /// other. It is filled by the conversion the surrounding term forces, and if
    /// nothing forces one, the program really did not say.
    FamilyParameter,
    /// A universe whose level the surface did not write.
    UniverseLevel,
    /// A dictionary for a constraint whose head was not yet known.
    ///
    /// `10-traits.md` §4 postpones such a constraint rather than guessing, and
    /// the hole it leaves is an ordinary metavariable — so a constraint still
    /// blocked when a declaration ends is reported by the machinery that
    /// already reports an unsolved hole, and nothing was built for
    /// postponement.
    Dictionary,
}

impl MetaSource {
    /// How a diagnostic names this site.
    ///
    /// Written to complete "could not determine …", so it is a noun phrase
    /// rather than a sentence: the message says where the unknown came from,
    /// and the term's own origin says where to look.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::ImplicitArgument => "an implicit argument",
            Self::BinderType => "the type of a binder",
            Self::FamilyParameter => "a constructor's family parameter",
            Self::UniverseLevel => "the level of a universe",
            Self::Dictionary => "the instance a constraint needs",
        }
    }
}

struct Cell {
    id: u32,
    origin: Origin,
    source: MetaSource,
    /// How many binders were in scope where it was created, and so how many
    /// arguments it is applied to and how many lambdas its solution has.
    arity: u32,
    /// Its type, closed: the telescope of the creation context over the type
    /// the hole stood at.
    ty: Value,
    solution: OnceLock<Value>,
}

/// An unknown term, and the one place its solution is written.
///
/// Cloning shares the cell, which is the point: a meta embedded in ten terms is
/// one unknown, and solving it there solves it in all ten.
#[derive(Clone)]
pub struct Meta(Arc<Cell>);

/// Identity, not structure. Two metas are the same unknown when they are the
/// same cell; comparing solutions would make a term's equality depend on how
/// much of elaboration had run.
impl PartialEq for Meta {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Meta {}

impl fmt::Debug for Meta {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "?{}", self.0.id)?;
        if self.is_solved() {
            out.write_str(" (solved)")?;
        }
        Ok(())
    }
}

impl fmt::Display for Meta {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "?{}", self.0.id)
    }
}

impl Meta {
    pub(crate) fn new(id: u32, origin: Origin, source: MetaSource, arity: u32, ty: Value) -> Self {
        Self(Arc::new(Cell {
            id,
            origin,
            source,
            arity,
            ty,
            solution: OnceLock::new(),
        }))
    }

    /// Its number, which is how a diagnostic names it.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.0.id
    }

    /// The term whose elaboration created it.
    #[must_use]
    pub fn origin(&self) -> Origin {
        self.0.origin
    }

    /// Which of §2.1's three sites created it.
    #[must_use]
    pub fn source(&self) -> MetaSource {
        self.0.source
    }

    /// Whether it has been solved.
    #[must_use]
    pub fn is_solved(&self) -> bool {
        self.0.solution.get().is_some()
    }

    pub(crate) fn arity(&self) -> u32 {
        self.0.arity
    }

    pub(crate) fn ty(&self) -> &Value {
        &self.0.ty
    }

    pub(crate) fn solution(&self) -> Option<&Value> {
        self.0.solution.get()
    }

    /// Record `value` — a closed `λx₀ … xₐ₋₁. t` — as this meta's solution.
    ///
    /// The solution is kept as a *value* and not also as the term it was built
    /// from, because every reader unfolds it: evaluation splices it in, and
    /// zonking reads it back at the type the hole stood at. A stored term would
    /// be the same solution twice, in two representations free to disagree.
    ///
    /// # Errors
    ///
    /// [`Malformed::AlreadySolved`] if it had one. Reported rather than
    /// asserted: a unifier that solved twice is a defect worth a name, and a
    /// silent second write would make the write-once guarantee a comment.
    pub(crate) fn solve(&self, value: Value) -> Result<(), CoreError> {
        self.0
            .solution
            .set(value)
            .map_err(|_| Malformed::AlreadySolved(self.0.id).into())
    }
}
