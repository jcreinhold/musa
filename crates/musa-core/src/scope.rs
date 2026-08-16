//! The elaboration context: a [`Cx`] with the names elaboration resolves by.
//!
//! # Why the names are not in [`Cx`]
//!
//! Prompt 133a kept them out deliberately: quotation writes a binder's name from
//! the Π or λ value it is walking, so a name stored in the context would be a
//! second copy free to disagree with the one that gets printed. That argument is
//! about a context that *outlives* a term. This one does not — a scope exists
//! for the duration of one elaboration and is dropped when the core term is
//! built — and its names are written from the same raw binder, in the same
//! statement, as the core binder they parallel. They cannot drift because
//! nothing separates them in time.
//!
//! So the split is the honest one: [`Cx`] is what a core term is *read under*,
//! and a [`Scope`] is what a raw term is *elaborated in*.
//!
//! # Binders this scope did not introduce
//!
//! A caller's [`Cx`] may already have binders, and those have no names — nothing
//! written in a raw term can mention them. They are still *here*, with their
//! types, because a metavariable abstracts over every binder in scope and a
//! telescope that skipped them would produce a solution mentioning variables it
//! never bound.

use std::sync::{Arc, OnceLock};

use crate::budget::Meter;
use crate::class::Key;
use crate::context::Cx;
use crate::error::CoreError;
use crate::family::Found;
use crate::list::List;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::term::{DbLevel, Index, Name, Term};
use crate::value::{Env, Value};

/// One binder, as elaboration sees it.
struct Binding {
    /// The name a raw variable resolves by, or `None` for a binder the caller's
    /// context already held.
    name: Option<Name>,
    /// How many binders were in scope when it was introduced, which is both its
    /// de Bruijn level and the depth its type is read at.
    level: u32,
    /// Its type.
    ty: Arc<Value>,
    /// Its type as a term, quoted the first time a metavariable telescope needs
    /// it.
    ///
    /// Lazy rather than eager because most binders never appear in one, and
    /// quoting a type at every binder would make elaboration pay for a
    /// metavariable it may never create. Cached rather than recomputed because
    /// each new metavariable would otherwise re-quote the whole context, which
    /// is quadratic in a context that is only ever appended to.
    ty_term: OnceLock<Term>,
}

/// What resolving a name found.
pub(crate) struct Resolved {
    /// How many binders out it is, from here.
    pub(crate) index: Index,
    /// The type it was introduced at.
    pub(crate) ty: Arc<Value>,
}

/// The binders a raw term is elaborated under.
#[derive(Clone)]
pub(crate) struct Scope {
    cx: Cx,
    bindings: List<Binding>,
    /// The dictionaries an enclosing `where` bound, innermost first.
    ///
    /// Beside the bindings rather than among them because they answer a
    /// different question: a binding is found by *name* and one of these is
    /// found by the constraint it discharges, and `10-traits.md` §4 step 1 is
    /// exactly that lookup. Keeping them apart is also what makes
    /// local-beats-global a property of the code — nothing can reach a local
    /// dictionary except by asking for a key.
    locals: List<Local>,
}

/// A dictionary an enclosing `where` bound, and the level its binder stands at.
///
/// The level rather than the index, because these outlive the scope they were
/// made in: a `where` dictionary is looked up while checking a body nested
/// arbitrarily deep inside it, and an index would have to be corrected at every
/// one of those depths. A level is corrected once, where it is read.
#[derive(Clone)]
pub(crate) struct Local {
    /// What it answers.
    pub(crate) key: Key,
    /// Where its binder stands, counted from the outside.
    pub(crate) level: u32,
    /// The arguments it was written at, as values for the same reason.
    ///
    /// `10-traits.md` §1: "once the head is known the whole instance is known,
    /// and with it every other parameter." A global instance makes that true by
    /// unifying its written arguments against the ones asked for
    /// (`apply_instance`), and a local has to do the same or the rule would hold
    /// for one of §4's two lookups. It is `Buildable<D, B>` that shows it: a use
    /// of `empty : C` mentions the first parameter and not the second, so
    /// nothing else in the program could ever determine `B`.
    pub(crate) args: Arc<[Value]>,
}

impl Scope {
    /// The scope of a raw term elaborated in `cx`.
    pub(crate) fn new(cx: &Cx) -> Self {
        // The context lists its binders innermost first and this list is built
        // by pushing, so the walk runs outermost-inward: collect, then replay in
        // reverse. Both lists then agree on what index `i` means.
        let types: Vec<&Arc<Value>> = cx.binder_types().iter().collect();
        let mut bindings = List::EMPTY;
        for (level, ty) in types.into_iter().rev().enumerate() {
            bindings = bindings.push(Binding {
                name: None,
                level: u32::try_from(level).unwrap_or(u32::MAX),
                ty: Arc::clone(ty),
                ty_term: OnceLock::new(),
            });
        }
        Self {
            cx: cx.clone(),
            bindings,
            locals: List::EMPTY,
        }
    }

    /// This scope with `key` discharged by the binder at `level`.
    ///
    /// Recorded when a `where` constraint's dictionary is assumed, so that a use
    /// inside it prefers this to any global instance for the same key (§4 step
    /// 1). Under coherence the two can never disagree; what the rule buys is
    /// determinacy, so instantiating a parameter later cannot reroute a call
    /// that was already elaborated.
    pub(crate) fn discharging(&self, key: Key, level: u32, args: Arc<[Value]>) -> Self {
        Self {
            locals: self.locals.push(Local { key, level, args }),
            ..self.clone()
        }
    }

    /// The innermost local dictionary answering `key`.
    pub(crate) fn discharged(&self, key: &Key) -> Option<&Local> {
        self.locals.iter().find(|local| local.key == *key)
    }

    /// The context these binders make up.
    ///
    /// Handed over for the one caller that asks the *core rules* a question
    /// mid-elaboration rather than an elaboration one — [`crate::case`] asking
    /// which universe a goal inhabits. Every scope extension extends this too,
    /// so the two never disagree about what is in scope.
    pub(crate) const fn cx(&self) -> &Cx {
        &self.cx
    }

    /// How many binders are in scope.
    pub(crate) const fn depth(&self) -> u32 {
        self.cx.depth()
    }

    /// The binder `name` refers to here, innermost first.
    ///
    /// Shadowing falls out of the order rather than being a rule: the innermost
    /// binding wins because it is the one the walk reaches first.
    pub(crate) fn lookup(&self, name: &str) -> Option<Resolved> {
        self.bindings.iter().find_map(|binding| {
            if binding.name.as_deref() != Some(name) {
                return None;
            }
            // A binding's level is below the current depth by construction, so
            // this is the index that names it.
            let steps_out = self.depth().saturating_sub(binding.level).saturating_sub(1);
            Some(Resolved {
                index: Index(steps_out),
                ty: Arc::clone(&binding.ty),
            })
        })
    }

    /// What a declared name refers to here.
    ///
    /// Asked only when [`Self::lookup`] found nothing, which is what makes a
    /// binder shadow a declaration rather than the other way round.
    pub(crate) fn declared(&self, name: &str) -> Option<Found> {
        self.cx.declared(name)
    }

    /// This scope extended by an assumption named `name` at type `ty`.
    pub(crate) fn assume(&self, name: Option<Name>, binder: Origin, ty: Arc<Value>) -> Self {
        Self {
            cx: self.cx.assumed(binder, Arc::clone(&ty)),
            bindings: self.pushed(name, ty),
            locals: self.locals.clone(),
        }
    }

    /// This scope extended by a definition, so that a use of `name` unfolds to
    /// `value` (δ).
    pub(crate) fn define(&self, name: Name, ty: Arc<Value>, value: Value) -> Self {
        Self {
            cx: self.cx.defined(Arc::clone(&ty), value),
            bindings: self.pushed(Some(name), ty),
            locals: self.locals.clone(),
        }
    }

    fn pushed(&self, name: Option<Name>, ty: Arc<Value>) -> List<Binding> {
        self.bindings.push(Binding {
            name,
            level: self.depth(),
            ty,
            ty_term: OnceLock::new(),
        })
    }

    /// The variable a binder introduced here would be.
    pub(crate) fn fresh_var(&self, origin: Origin, ty: Arc<Value>) -> Value {
        Value::var(origin, DbLevel(self.depth()), ty)
    }

    /// The values these binders stand for.
    ///
    /// Handed over for the one caller that reads a term under a *different*
    /// telescope than this scope's — a `data` declaration checking a
    /// constructor's chosen index against the family's index type, which is read
    /// under the indices and not under the constructor's fields.
    pub(crate) fn env(&self) -> &Env {
        self.cx.env()
    }

    /// Evaluate a term read under these binders.
    ///
    /// # Errors
    ///
    /// As [`crate::eval`].
    pub(crate) fn eval(&self, meter: &mut Meter, term: &Term) -> Result<Value, CoreError> {
        crate::eval::eval(meter, self.cx.env(), term)
    }

    /// Read a type back as a term under these binders.
    ///
    /// # Errors
    ///
    /// As [`quote_type`].
    pub(crate) fn quote_type(&self, meter: &mut Meter, value: &Value) -> Result<Term, CoreError> {
        quote_type(meter, Depth(self.depth()), value)
    }

    /// Wrap `body` in one Π per binder in scope, outermost first.
    ///
    /// This is how a metavariable becomes **closed**: `?α` has type
    /// `(x₀ : A₀) → … → (xₙ₋₁ : Aₙ₋₁) → T` and is written applied to every
    /// binder, so its solution abstracts exactly the variables it is allowed to
    /// mention and §2.1's scope condition is a property of the representation
    /// rather than a check anyone performs.
    ///
    /// # Errors
    ///
    /// As [`quote_type`], from reading a binder's type back.
    pub(crate) fn close(&self, meter: &mut Meter, origin: Origin, body: Term) -> Result<Term, CoreError> {
        // Innermost first, and each Π is written *outside* the last, so walking
        // the list in its own order builds the telescope in the right one.
        let mut closed = body;
        for binding in self.bindings.iter() {
            let name = binding.name.clone().unwrap_or_else(|| Arc::from("_"));
            closed = Term::pi(origin, name, binding.ty_term(meter)?, closed);
        }
        Ok(closed)
    }

    /// Apply `head` to every binder in scope, outermost argument first.
    ///
    /// The other half of [`Self::close`]: a closed metavariable is *used*
    /// spine-applied to its context, which is what puts a constraint in the
    /// pattern fragment — the arguments are distinct bound variables by
    /// construction.
    pub(crate) fn spine(&self, origin: Origin, head: Term) -> Term {
        let mut applied = head;
        for steps_out in (0..self.depth()).rev() {
            applied = Term::app(origin, applied, Term::var(origin, Index(steps_out)));
        }
        applied
    }

    /// How many binders a metavariable created here abstracts.
    pub(crate) const fn arity(&self) -> u32 {
        self.depth()
    }
}

impl Binding {
    fn ty_term(&self, meter: &mut Meter) -> Result<Term, CoreError> {
        if let Some(term) = self.ty_term.get() {
            return Ok(term.clone());
        }
        let term = quote_type(meter, Depth(self.level), &self.ty)?;
        // A second thread losing the race wrote an α-equal term, so which one
        // wins does not matter; only that one of them does.
        drop(self.ty_term.set(term.clone()));
        Ok(term)
    }
}
