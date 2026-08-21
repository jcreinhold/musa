//! The unknown a use site leaves behind: what a [`Hole`] is, and which
//! inference site it came from.
//!
//! This is what remains of the metavariable machinery after the course
//! correction, and it is smaller than the word suggests. A hole is created by
//! one instantiation walk, for one binder the author did not write, and it is
//! solved by that same walk or by the expected type at its end. Nothing here
//! postpones, retries, or generalizes. [`MetaSource`] survives beside it
//! because the two failures it names survive — a type parameter nothing in the
//! call determined, and a constraint no dictionary answers — and a diagnostic
//! is better where the failure is described as what the *program* left
//! unsaid.
use crate::origin::Origin;

/// Which of `02-core-calculus.md` §2.1's sites an unsolved unknown came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaSource {
    /// A type parameter of the called function that no written argument
    /// determined.
    TypeParameter,
    /// A constraint that no dictionary in scope and no instance answers.
    Constraint,
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
            Self::TypeParameter => "a type parameter",
            Self::Constraint => "the instance a constraint needs",
        }
    }
}

use std::sync::{Arc, OnceLock};

use crate::error::Malformed;
use crate::value::Value;

/// A placeholder for an argument the instantiation walk has not yet solved:
/// a type parameter, or a constraint's dictionary.
///
/// This is what remains of the metavariable after the course correction, and
/// the list of what it is *not* is the point: not contextual (it is closed —
/// its type was checked before it was made), not pattern-unified (an
/// unapplied occurrence takes its solution by direct assignment and an
/// applied one is an error), not postponed (the walk that created it solves
/// it or [`crate::Refusal::Unsolved`] names it when the declaration ends).
/// What it shares with the old machinery is the one property elaboration
/// cannot do without: a term can stand for a type the *rest* of the
/// expression determines — `xs.fold_from_end(None, step)` reads `None` before
/// `step`'s annotation says `Option` of what.
///
/// Cloning shares the cell, which is the point: a hole embedded in ten terms
/// is one unknown, and solving it there solves it in all ten.
#[derive(Clone)]
pub struct Hole(Arc<Cell>);

struct Cell {
    id: u32,
    origin: Origin,
    /// Its type, closed: checked before the hole was made, and the reason
    /// evaluation can answer a hole-headed neutral's type with no context.
    ty: Value,
    solution: OnceLock<Value>,
}

impl Hole {
    /// A fresh hole, identified by `id`, standing at `ty`.
    pub(crate) fn new(id: u32, origin: Origin, ty: Value) -> Self {
        Self(Arc::new(Cell {
            id,
            origin,
            ty,
            solution: OnceLock::new(),
        }))
    }

    /// Where the argument it stands for was used.
    pub(crate) fn origin(&self) -> Origin {
        self.0.origin
    }

    /// The type it stands at.
    pub(crate) fn ty(&self) -> &Value {
        &self.0.ty
    }

    /// Write its solution. Write-once: a second write is a compiler defect,
    /// because assignment is the only solver and it checks before writing.
    pub(crate) fn solve(&self, value: Value) -> Result<(), Malformed> {
        self.0
            .solution
            .set(value)
            .map_err(|_| Malformed::AlreadySolved(self.0.id))
    }

    /// Its solution, if the matching pass has found one.
    #[must_use]
    pub(crate) fn solution(&self) -> Option<&Value> {
        self.0.solution.get()
    }

    /// Whether it is solved.
    #[must_use]
    pub fn is_solved(&self) -> bool {
        self.0.solution.get().is_some()
    }
}

impl PartialEq for Hole {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Hole {}

impl core::fmt::Debug for Hole {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "?{}", self.0.id)
    }
}

impl core::fmt::Display for Hole {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "?{}", self.0.id)
    }
}
