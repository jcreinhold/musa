//! The unknown a use site leaves behind: what a [`Meta`] is, the scope it may
//! mention, and which inference site it came from.
//!
//! `docs/rules/language/02-core-calculus.md` §2.1 writes a metavariable
//! `?m[σ]`: an unknown *carrying the scope σ of local variables it may
//! mention*. This module is the σ. What it is not is a context stored beside
//! the unknown and consulted when someone remembers to: it is the shape of the
//! unknown itself. A meta created under n binders has the **closed** type
//! `(x₀ : A₀) → … → (x_{n-1} : A_{n-1}) → A`, every occurrence of it is that
//! meta *applied to those n variables*, and its solution is a closed
//! λ-abstraction of the same n binders.
//!
//! # Why closed, when a context would have been less code
//!
//! Because this crate has no substitution function on terms (`value.rs`), and
//! a scope stored as data would need one. A solution read back where its meta
//! was created is a term whose indices count from *that* depth; the occurrence
//! it replaces may sit deeper, and making the two agree is a shift — which is
//! substitution, which the `NbE` presentation §3 specifies does not exist here.
//! Abstracting instead moves the whole question into β: the solution is closed,
//! so it means the same thing at every depth, and the occurrence's own spine
//! puts the variables back. Idris2 represents holes as top-level definitions
//! for exactly this reason.
//!
//! Two consequences worth stating, because they are what the rest of the crate
//! relies on. The **scope check is the read-back**: quoting a solution at the
//! meta's own arity refuses a variable from outside it, so nothing needs a
//! second walk to notice a capture ([`crate::kernel::unify`] and
//! [`crate::kernel::recheck`] each make that refusal, once at solving and once
//! at verification). And the **pattern fragment has something to match**:
//! `?m x₀ … x_{n-1} ≟ t` is §2.1's shape, and it is the shape every occurrence
//! already has.
//!
//! [`MetaSource`] survives beside all of it because the failure it names
//! survives — a type parameter nothing in the call determined — and a
//! diagnostic is better where the failure is described as what the *program*
//! left unsaid.
use crate::kernel::origin::Origin;

/// Which of `02-core-calculus.md` §2.1's sites an unsolved unknown came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaSource {
    /// A type parameter of the called function that no written argument
    /// determined.
    TypeParameter,
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
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::kernel::context::Globals;
use crate::kernel::error::Malformed;
use crate::kernel::value::Value;

/// How many metavariables have been solved in this process.
///
/// Not bookkeeping about metas: it is the invalidation stamp
/// [`crate::kernel::eval::unfold`]'s memo is guarded by. A folded definition
/// unfolds to a value that may contain an occurrence of an unsolved unknown,
/// and reducing the same neutral after that unknown is solved may answer
/// something further reduced — so a memo filled before a solution arrived must
/// not be read after one. A counter answers that with one comparison and no
/// walk: **equal stamps mean no solution arrived in between**, which is
/// exactly the premise the memo needs, and it is conservative in the safe
/// direction because a solution anywhere invalidates every memo rather than
/// only the ones that mention it.
static SOLUTIONS: AtomicU64 = AtomicU64::new(0);

/// The current value of that stamp.
pub(crate) fn solutions() -> u64 {
    SOLUTIONS.load(Ordering::Relaxed)
}

/// A placeholder for a term the elaborator cannot yet determine, carrying the
/// scope it may mention.
///
/// §2.1's `?m[σ]`, represented as this module's header describes: the type is
/// the whole telescope `(x₀ : A₀) → … → A`, the arity is σ's length, and a
/// solution is a closed λ-abstraction of that telescope. Nothing about the
/// scope is optional — a meta with the wrong arity is an occurrence whose spine
/// does not match its type, which [`crate::kernel::recheck`] refuses.
///
/// What a meta buys is the one property elaboration cannot do without: a term
/// can stand for a type the *rest* of the expression determines —
/// `xs.fold_from_end(None, step)` reads `None` before `step`'s annotation says
/// `Option` of what. What §2.1 adds on top of that is patience: an unknown that
/// nothing has determined *yet* is a postponed constraint rather than a
/// refusal, and only the declaration's end turns one into the other.
///
/// Cloning shares the cell, which is the point: a meta embedded in ten terms
/// is one unknown, and solving it there solves it in all ten.
#[derive(Clone)]
pub struct Meta(Arc<Cell>);

struct Cell {
    id: u32,
    origin: Origin,
    /// Its type, and it is genuinely closed: the goal abstracted over every
    /// binder that was in scope. That is what lets
    /// [`neutral_type`](crate::kernel::eval::neutral_type) answer a
    /// meta-headed neutral's type with no context to ask, and what makes the
    /// spine on an occurrence type-check against it.
    ty: Value,
    /// How many binders were in scope where it was created: the length of the
    /// telescope [`Self::ty`] abstracts, of the spine every occurrence applies,
    /// and of the λ-prefix any solution carries.
    arity: u32,
    /// The table the telescope and any solution are read under.
    ///
    /// Here for the reason it is on [`Head::Base`](crate::kernel::value::Head)
    /// and [`Head::Builtin`](crate::kernel::value::Head): a solution is built
    /// by evaluating a term with no locals but plenty of names, and the value
    /// that comes out has to unfold the same definitions the scope it came
    /// from would have. It is not part of the meta's identity.
    globals: Globals,
    solution: OnceLock<Value>,
}

impl Meta {
    /// A fresh meta, identified by `id`, standing at the closed telescope type
    /// `ty` over `arity` binders, read under `globals`.
    ///
    /// The caller owes the invariant this type cannot state: `ty` is `arity`
    /// nested Π binders around the goal, and it mentions no free variable.
    /// [`crate::kernel::unify::scope_of`] is the only reader of that shape, and
    /// it answers a [`Malformed`] rather than assuming when the shape is wrong.
    pub(crate) fn new(id: u32, origin: Origin, ty: Value, arity: u32, globals: Globals) -> Self {
        Self(Arc::new(Cell {
            id,
            origin,
            ty,
            arity,
            globals,
            solution: OnceLock::new(),
        }))
    }

    /// How many binders its telescope abstracts.
    pub(crate) fn arity(&self) -> u32 {
        self.0.arity
    }

    /// The table its telescope and its solution are read under.
    pub(crate) fn globals(&self) -> &Globals {
        &self.0.globals
    }

    /// Which meta this is, for a report that has to name one.
    pub(crate) fn id(&self) -> u32 {
        self.0.id
    }

    /// Where the argument it stands for was used.
    pub(crate) fn origin(&self) -> Origin {
        self.0.origin
    }

    /// The type it stands at.
    pub(crate) fn ty(&self) -> &Value {
        &self.0.ty
    }

    /// Write its solution: a **closed** value, `arity` λ binders around the
    /// term the unknown stands for.
    ///
    /// Write-once, and §2.1 is why rather than tidiness: a unifier that
    /// overwrote a solution would make acceptance depend on the order
    /// constraints arrived in. A second write is therefore a compiler defect
    /// and is reported as one — [`crate::kernel::unify::assign`] is the only
    /// caller, and it decides before it writes.
    pub(crate) fn solve(&self, value: Value) -> Result<(), Malformed> {
        self.0
            .solution
            .set(value)
            .map_err(|_| Malformed::AlreadySolved(self.0.id))?;
        SOLUTIONS.fetch_add(1, Ordering::Relaxed);
        Ok(())
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

impl PartialEq for Meta {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Meta {}

impl core::fmt::Debug for Meta {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "?{}", self.0.id)
    }
}

impl core::fmt::Display for Meta {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(out, "?{}", self.0.id)
    }
}
