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

use std::sync::Arc;

use crate::kernel::budget::Meter;
use crate::kernel::context::Cx;
use crate::kernel::error::CoreError;
use crate::kernel::family::Found;
use crate::kernel::list::List;
use crate::kernel::origin::Origin;
use crate::kernel::quote::quote_type;
use crate::kernel::term::{Index, Level, Name, Term};
use crate::kernel::value::{Env, Value};

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
}

/// One binder a metavariable abstracts over: how the telescope names it, the
/// type it stands at, and what its occurrences apply.
///
/// Three facts and not two, because the third is the one a caller would get
/// wrong: see [`Scope::telescope`].
pub(crate) struct Abstracted {
    /// The name the Π binder is written with.
    pub(crate) name: Name,
    /// The type it was introduced at, read at its own depth.
    pub(crate) ty: Arc<Value>,
    /// What it stands for in this scope's environment.
    pub(crate) value: Value,
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
            });
        }
        Self {
            cx: cx.clone(),
            bindings,
        }
    }

    /// Every name a written variable could have meant here.
    ///
    /// Binders the elaboration introduced, then the context's definitions:
    /// the list an unknown-name refusal hands the surface, whose "did you
    /// mean" is its own policy over exactly these.
    pub(crate) fn nameable(&self) -> Vec<Name> {
        self.bindings
            .iter()
            .filter_map(|binding| binding.name.clone())
            .chain(self.cx.defined_names())
            .collect()
    }

    /// The context these binders make up.
    ///
    /// Handed over for the one caller that asks the *core rules* a question
    /// mid-elaboration rather than an elaboration one — [`crate::elaboration::case`] asking
    /// which universe a goal inhabits. Every scope extension extends this too,
    /// so the two never disagree about what is in scope.
    pub(crate) const fn cx(&self) -> &Cx {
        &self.cx
    }

    /// How many binders are in scope.
    pub(crate) const fn depth(&self) -> Level {
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
            let steps_out = self.depth().0.saturating_sub(binding.level).saturating_sub(1);
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
        }
    }

    /// This scope extended by a definition, so that a use of `name` unfolds to
    /// `value` (δ).
    pub(crate) fn define(
        &self,
        meter: &mut Meter,
        name: Name,
        ty: Arc<Value>,
        value: Value,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            cx: self.cx.defined(meter, Arc::clone(&ty), value)?,
            bindings: self.pushed(Some(name), ty),
        })
    }

    fn pushed(&self, name: Option<Name>, ty: Arc<Value>) -> List<Binding> {
        self.bindings.push(Binding {
            name,
            level: self.depth().0,
            ty,
        })
    }

    /// Every binder in scope, outermost first: its name where it has one, and
    /// the type it was introduced at.
    ///
    /// What a metavariable abstracts over — the telescope this module's header
    /// says a solution must bind. Outermost first because that is the order the
    /// Π chain is built in, and because binder `i`'s type is read at depth `i`,
    /// which is its own position in the list.
    ///
    /// A binder the caller's context already held has no name, and gets one
    /// here rather than at the use site: the telescope is written into a type
    /// that may be printed, and an unnamed Π binder has nothing to print.
    pub(crate) fn telescope(&self) -> Vec<Abstracted> {
        let mut binders: Vec<&Binding> = self.bindings.iter().collect();
        binders.reverse();
        binders
            .iter()
            .enumerate()
            .map(|(position, binding)| {
                let steps_out = binders.len().saturating_sub(position).saturating_sub(1);
                let name = binding
                    .name
                    .clone()
                    .unwrap_or_else(|| Arc::from(format!("x{}", binding.level).as_str()));
                Abstracted {
                    name,
                    ty: Arc::clone(&binding.ty),
                    // The environment's own entry, not a variable built to
                    // match it. A `let` binder stands in the environment as the
                    // definition *folded* — `Cx::defined` says why — and an
                    // occurrence whose spine held a fresh variable there would
                    // evaluate to something the elaborator never reasoned
                    // about, because evaluating the occurrence's term reads the
                    // environment and gets the folded definition back.
                    value: self
                        .cx
                        .env()
                        .get(u32::try_from(steps_out).unwrap_or(u32::MAX))
                        .cloned()
                        .unwrap_or_else(|| {
                            Value::var(binding.ty.origin, Level(binding.level), Arc::clone(&binding.ty))
                        }),
                }
            })
            .collect()
    }

    /// The variable a binder introduced here would be.
    pub(crate) fn fresh_var(&self, origin: Origin, ty: Arc<Value>) -> Value {
        Value::var(origin, self.depth(), ty)
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
    /// As [`crate::kernel::eval`].
    pub(crate) fn eval(&self, meter: &mut Meter, term: &Term) -> Result<Value, CoreError> {
        crate::kernel::eval::eval(meter, self.cx.env(), term)
    }

    /// Read a type back as a term under these binders.
    ///
    /// # Errors
    ///
    /// As [`quote_type`].
    pub(crate) fn quote_type(&self, meter: &mut Meter, value: &Value) -> Result<Term, CoreError> {
        quote_type(meter, self.depth(), crate::kernel::quote::Mode::Keep, value)
    }
}
