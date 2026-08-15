//! The semantic domain, and why it is private.
//!
//! `docs/rules/language/02-core-calculus.md` §3 decides definitional equality
//! by normalization by evaluation: both sides are evaluated into values with
//! closures and neutrals, and quoted back to a normal form. **Reduction is
//! never performed on syntax**, which is why there is no substitution function
//! anywhere in this crate.
//!
//! [`Value`] never leaves. A caller asks whether two terms are the same and
//! gets an answer, or asks for a normal form and gets a [`Term`]; it never
//! learns what a closure is. `docs/plan/roadmap.md` §15.12 fixes that boundary
//! and prompt 133 states the argument, because the pressure to leak it arrives
//! with the first caller that wants to inspect a normal form:
//!
//! > Prompt 134's elaborator wants to check against a *value* type rather than
//! > re-normalizing at every step, and exposing `Value` would let it. The
//! > answer is that the elaborator then belongs in this crate, not that `Value`
//! > belongs in the facade.
//!
//! Concretely: closures hold the evaluator's own environment representation, so
//! every later change to evaluation would be a breaking change for
//! `musa-compiler`. Prompt 134 adds check and infer *inside* this crate over a
//! surface-independent raw term, and `musa-compiler` keeps only the translation
//! into that raw term.
//!
//! **A neutral variable carries its type.** That is not decoration. Quotation
//! is type-directed — it is what performs η at Π and at records (§3) — so
//! quoting the *argument* of a blocked application needs that argument's type,
//! which means the type of the function it is applied to, which means a neutral
//! must be able to say what type it has. [`crate::eval::neutral_type`] is that
//! function and the type on the variable is its base case. Without it, `f g`
//! and `f (λx. g x)` would quote to different terms and conversion would answer
//! `false` for two terms §3 says are equal.
//!
//! **A value carries an origin, because quotation has to give one back.** §7
//! says the normal form of a term carries the origins of the terms it was built
//! from. A normal form is written by `quote` out of a value, so a value that had
//! dropped its origin could not put one back. Values and neutrals therefore have
//! the same shape as [`Term`] does — provenance in a wrapper, everything else in
//! a [`Form`] or a [`Spine`].

use std::sync::Arc;

use crate::level::Level;
use crate::origin::Origin;
use crate::term::{DbLevel, Field, Name, Term};

/// An immutable environment: the values of the binders in scope, innermost
/// first.
///
/// A persistent list rather than a vector, because evaluation extends it once
/// per binder and conversion under a binder extends it again for every
/// comparison. Extension is one allocation and no copy; lookup walks, and the
/// walk is bounded by how many binders a term actually mentions.
#[derive(Clone, Default)]
pub(crate) struct Env(Option<Arc<Cell>>);

struct Cell {
    value: Value,
    rest: Env,
}

impl Env {
    /// No binders at all.
    pub(crate) const EMPTY: Self = Self(None);

    /// This environment with `value` bound innermost.
    pub(crate) fn extend(&self, value: Value) -> Self {
        Self(Some(Arc::new(Cell {
            value,
            rest: self.clone(),
        })))
    }

    /// The value `index` binders out, or `None` when it names no binder.
    pub(crate) fn lookup(&self, index: u32) -> Option<&Value> {
        let mut here = self;
        let mut remaining = index;
        loop {
            let cell = here.0.as_ref()?;
            if remaining == 0 {
                return Some(&cell.value);
            }
            remaining = remaining.checked_sub(1)?;
            here = &cell.rest;
        }
    }
}

/// A term paired with the environment its free variables are read in.
#[derive(Clone)]
pub(crate) struct Closure {
    pub(crate) env: Env,
    pub(crate) body: Term,
}

/// A record type as a telescope: the fields in order, read in one environment,
/// where field `i`'s type may mention fields `0..i`.
#[derive(Clone)]
pub(crate) struct Telescope {
    pub(crate) fields: Arc<[Field]>,
    pub(crate) env: Env,
}

/// A semantic value: what it is, and where the term that produced it came from.
#[derive(Clone)]
pub(crate) struct Value {
    pub(crate) origin: Origin,
    pub(crate) form: Form,
}

/// What a value is.
#[derive(Clone)]
pub(crate) enum Form {
    Universe(Level),
    Pi {
        name: Name,
        domain: Arc<Value>,
        codomain: Closure,
    },
    /// A lambda, and no name: quotation writes a binder's name from the Π it
    /// is quoting at, never from the λ, so a name here would be a second copy
    /// free to disagree with the one that gets printed.
    Lam(Closure),
    RecordType(Telescope),
    Record(Arc<[(Name, Value)]>),
    Id {
        ty: Arc<Value>,
        left: Arc<Value>,
        right: Arc<Value>,
    },
    Refl(Arc<Value>),
    Neutral(Arc<Neutral>),
}

/// An elimination blocked on a variable.
///
/// Each node carries its own origin because each becomes its own node of a
/// quoted normal form: the spine of `f x y` reads back as three terms, and §7
/// wants each of them to say where it came from.
pub(crate) struct Neutral {
    pub(crate) origin: Origin,
    pub(crate) spine: Spine,
}

/// What a blocked elimination is blocked on, and what has been applied to it.
pub(crate) enum Spine {
    /// A variable, with the type it was assumed at.
    Var(DbLevel, Arc<Value>),
    App {
        function: Arc<Neutral>,
        argument: Arc<Value>,
    },
    Project {
        record: Arc<Neutral>,
        field: Name,
    },
    /// `J` blocked on a proof that is not `refl`.
    J {
        ty: Arc<Value>,
        from: Arc<Value>,
        motive: Arc<Value>,
        base: Arc<Value>,
        to: Arc<Value>,
        proof: Arc<Neutral>,
    },
}

impl Value {
    /// A value of form `form`, from a term that came from `origin`.
    pub(crate) const fn new(origin: Origin, form: Form) -> Self {
        Self { origin, form }
    }

    /// A blocked elimination, as a value.
    ///
    /// The origin comes from the neutral rather than from a second argument:
    /// the two would be the same fact stored twice, and the copies would be free
    /// to disagree.
    pub(crate) fn neutral(neutral: Neutral) -> Self {
        Self::new(neutral.origin, Form::Neutral(Arc::new(neutral)))
    }

    /// An already-shared blocked elimination, as a value.
    pub(crate) fn shared_neutral(neutral: &Arc<Neutral>) -> Self {
        Self::new(neutral.origin, Form::Neutral(Arc::clone(neutral)))
    }

    /// A fresh variable at `level`, assumed at `ty`.
    pub(crate) fn var(origin: Origin, level: DbLevel, ty: Arc<Self>) -> Self {
        Self::neutral(Neutral {
            origin,
            spine: Spine::Var(level, ty),
        })
    }
}
