//! A document's top-level definitions and instances, declared together.
//!
//! `docs/rules/language/02-core-calculus.md` §2.4's last paragraph is what this
//! module implements: "Top-level signatures are collected before bodies are
//! elaborated, so a later declaration may be referenced. From each body the
//! checker records an edge to every free named declaration. **Non-recursive**
//! declarations must form an acyclic graph, exactly as before; a *recursive*
//! declaration is the case §2.4 admits, and it is admitted through the measure
//! rather than through the graph."
//!
//! # Why a definition is a constant and not a binder
//!
//! The cheaper arrangement is a chain of [`RawShape::Let`](crate::RawShape),
//! and it cannot work: a `let` scopes forward only, so a body could never name
//! a definition written after it. The next cheapest is
//! [`Cx::define`](crate::Cx::define) with a name attached, and it cannot work
//! either, for a sharper reason — a de Bruijn binder refers *outward*, so the
//! last definition can see the first and never the other way round. Closing
//! that circle would need either substitution, which this crate deliberately
//! does not have (§3: "reduction is never performed on syntax"), or a fixed
//! point, which §1.3 refuses.
//!
//! So a definition is a **global name**, exactly as a family is, and a
//! [`Program`] sits beside the declared groups in a [`Cx`] rather than in
//! its binder environment. That also leaves α-equivalence alone: the
//! environment stays nameless, which is the property [`Cx::assume`]'s own
//! documentation is protecting.
//!
//! # A use is a reference, not a copy
//!
//! [`Shape::Def`](crate::Shape) is one node whose size does not depend on the
//! definition's body, and δ happens in [`crate::kernel::eval`] by handing back the
//! value the definition was elaborated to. Inlining the body at each use site
//! would type-check and would be wrong at scale: `stdlib/` is two thousand
//! definitions, every use would carry a copy, and a semantic hash would become
//! a function of what a definition happens to be written as rather than of what
//! it means.
//!
//! The value is stored and the *term* is not, which is what keeps the whole
//! arrangement free of the [`Arc`] cycle [`crate::kernel::family`] had to work around
//! with a declaration context. A [`Defined`] holds two [`Value`]s; nothing it
//! holds can point back at the group it is in, because evaluation has already
//! replaced every reference with the value it named.
//!
//! # The order is computed, and that is the mechanism
//!
//! Peyton Jones ch. 6 §6.2.8 is dependency analysis — sort definitions "into
//! minimal groups" and use `letrec` only "where it is actually necessary" — and
//! ch. 8 says when it has to run: "it is, however, important that the program
//! is subjected to the dependency analysis referred to in Section 6.2.8 before
//! type-checking", because a definition put into a recursive group it does not
//! belong in may fail to type-check at all. Here the analysis is the whole of
//! how §2.4's first sentence comes true: elaborating in dependency order is
//! what lets a body name a declaration written later, and the minimal groups it
//! finds are exactly the two cases §2.4 distinguishes — a group of one is an
//! ordinary definition or, with a self-edge, the recursive case the measure
//! admits, and a group of more than one is the cycle that has nowhere to go.
//!
//! The analysis runs over the document's `impl`s as well, for the reason
//! [`RawProgram`] states: a method body is an ordinary term that may name any
//! definition, and a definition that writes `x.m(y)` needs the instance at the
//! receiver's head to already be declared. Neither kind comes first, so neither
//! is declared first — the edges decide, one document at a time.

use std::sync::Arc;

use crate::kernel::origin::Origin;
use crate::kernel::sort::{Levels, Sort, SortVar};
use crate::kernel::term::{Name, Role, Term};
use crate::kernel::value::Value;
use crate::kernel::visibility::{ModuleId, Visibility};

/// One elaborated top-level definition.
///
/// Its type and its value are held twice, and the pair is the whole of what
/// universe polymorphism costs here. The [`Value`]s are what a *monomorphic*
/// use needs — [`crate::kernel::eval`] hands back the value and
/// [`recheck`](crate::kernel::recheck::recheck) asks for the type, with no work
/// in between. The [`Term`]s are what a *polymorphic* use needs, because
/// instantiating `levels` is a substitution and a [`Value`] holds closures a
/// substitution cannot reach without a second traversal of the whole semantic
/// domain. So a definition with no level parameters costs exactly what it cost
/// before, and one with parameters pays an evaluation per use, charged to the
/// meter that asked.
///
/// Storing the terms does not re-open the [`Arc`] cycle the module
/// documentation warns about: a use of a definition is
/// [`Shape::Named`](crate::Shape) holding a [`Name`], never an [`Arc<Defined>`],
/// so nothing a stored term holds can point back at the group.
pub(crate) struct Defined {
    /// The name it binds.
    pub(crate) name: Name,
    /// Whether it may be named outside its module.
    pub(crate) visibility: Visibility,
    /// The module it was written in, when the caller numbers modules.
    pub(crate) module: Option<ModuleId>,
    /// The level parameters generalization found, in the order the type
    /// mentions them. Empty for all but a universe-polymorphic definition.
    pub(crate) levels: Arc<[SortVar]>,
    /// Its type, at the level parameters themselves.
    pub(crate) ty: Arc<Value>,
    /// Its value, likewise.
    pub(crate) value: Arc<Value>,
    /// Its type as written, kept so `levels` can be instantiated.
    pub(crate) ty_term: Term,
    /// Its value as written, likewise.
    pub(crate) value_term: Term,
}

impl Defined {
    /// The module this definition is private to, when `viewer` may not name it.
    pub(crate) fn hidden_from(&self, viewer: Option<ModuleId>) -> Option<ModuleId> {
        (!self.visibility.visible_from(self.module, viewer)).then_some(self.module)?
    }
}

/// A definition shows as its name.
///
/// Written out rather than derived because a [`Value`] has no `Debug` — the
/// semantic domain holds closures — and because a definition's *body* is not
/// what a reader of a debug dump is looking for when a term mentions one.
impl core::fmt::Debug for Defined {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_tuple("Defined").field(&self.name).finish()
    }
}

/// A document's top level, elaborated.
///
/// Opaque for [`crate::Group`]'s reason: a caller brings it into scope with
/// [`Cx::defining`] and then writes the names in ordinary raw terms. It never
/// assembles a definition's type itself, because the elaboration that produced
/// it is the only thing that knows what the type came out as.
///
/// A namespaced definition — `Pitch.act` — is a member like any other. That is
/// the whole of what an `impl Pitch { … }` block leaves here.
pub struct Program {
    pub(crate) members: Arc<[Arc<Defined>]>,
}

impl Program {
    /// Every definition in the group, in the order they were written.
    pub(crate) fn members(&self) -> &[Arc<Defined>] {
        &self.members
    }
}

/// The definitions, in the order the document wrote them.
///
/// A named list rather than a bare one: what the order shows is the *written*
/// order, which [`declare_program`] restores after elaborating in the order it
/// computed.
impl core::fmt::Debug for Program {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_struct("Program").field("definitions", &self.members).finish()
    }
}

/// A use of a top-level definition.
///
/// One node. What it costs to write a definition's name is the same whatever
/// the definition is, which is the property the module documentation calls a
/// reference rather than a copy.
#[derive(Clone)]
pub struct Def(Arc<Defined>);

impl core::fmt::Debug for Def {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_tuple("Def").field(&self.0.name).finish()
    }
}

/// Two uses are the same use when they name the same definition.
///
/// By name, not by pointer, for [`crate::Constant`]'s reason: a term elaborated
/// twice from one source must compare equal, and two elaborations build two
/// groups.
impl PartialEq for Def {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
    }
}

impl Eq for Def {}

impl core::fmt::Display for Def {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(&self.0.name)
    }
}

impl Def {
    /// The name the definition binds.
    pub(crate) fn name(&self) -> &Name {
        &self.0.name
    }

    /// This use as a term: the definition's name, at [`Role::Defined`], with
    /// the levels it was instantiated at.
    ///
    /// The definition stays here and the term takes the name (§6), which is
    /// what makes a use one node whatever the definition is. The levels ride
    /// along rather than being re-inferred, because
    /// [`recheck`](crate::kernel::recheck::recheck)'s obligation is to *verify*
    /// that every use of a declaration instantiated its parameters
    /// consistently, and a checker that re-derived them could not disagree.
    pub(crate) fn term(&self, origin: Origin, levels: Levels) -> Term {
        Term::named_at(origin, Arc::clone(&self.0.name), Role::Defined, levels)
    }

    /// The level parameters this definition was generalized over.
    pub(crate) fn levels(&self) -> &[SortVar] {
        &self.0.levels
    }

    /// The definition's type.
    pub(crate) fn ty(&self) -> Arc<Value> {
        Arc::clone(&self.0.ty)
    }

    /// The definition's value, which is what δ unfolds a use to.
    pub(crate) fn value(&self) -> Arc<Value> {
        Arc::clone(&self.0.value)
    }

    /// This definition's type and value with its level parameters replaced by
    /// `levels`.
    ///
    /// The stored pair when there is nothing to instantiate, which is every
    /// definition the language wrote before this one existed. Otherwise the
    /// written terms, substituted and evaluated — see [`Defined`] for why the
    /// terms are kept.
    ///
    /// # Errors
    ///
    /// [`Malformed::LevelArity`] when `levels` is not one level per parameter:
    /// the term was assembled by something that did not read the declaration.
    pub(crate) fn instance(
        &self,
        meter: &mut crate::kernel::budget::Meter,
        globals: &crate::kernel::context::Globals,
        levels: &Levels,
    ) -> Result<(Arc<Value>, Arc<Value>), crate::kernel::error::CoreError> {
        let parameters = self.levels();
        if parameters.is_empty() && levels.is_empty() {
            return Ok((self.ty(), self.value()));
        }
        let given = levels.as_slice();
        if given.len() != parameters.len() {
            return Err(crate::kernel::error::Malformed::LevelArity(Arc::clone(&self.0.name)).into());
        }
        let with = |var: &SortVar| -> Option<Sort> {
            parameters
                .iter()
                .position(|parameter| parameter == var)
                .and_then(|at| given.get(at).cloned())
        };
        let env = crate::kernel::value::Env::under(globals.clone());
        let ty = crate::kernel::eval::eval(meter, &env, &self.0.ty_term.substitute_levels(&with))?;
        let value = crate::kernel::eval::eval(meter, &env, &self.0.value_term.substitute_levels(&with))?;
        Ok((Arc::new(ty), Arc::new(value)))
    }
}

/// The definition a single-member context extension holds.
pub(crate) fn one(defined: &Arc<Defined>) -> Def {
    Def(Arc::clone(defined))
}
