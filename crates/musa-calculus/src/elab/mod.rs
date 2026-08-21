//! Bidirectional elaboration: a raw term goes in, a typed core term comes out.
//!
//! `docs/rules/language/02-core-calculus.md` §2 gives two judgments —
//! `Γ ⊢ e ⇐ A ⇝ t` and `Γ ⊢ e ⇒ A ⇝ t` — and exactly two rules that move
//! between them. This module is those judgments and nothing else: it decides no
//! surface question, knows no musical type, and never sees a token.
//!
//! # Which forms check and which infer
//!
//! Introduction forms **check**: a λ, a record literal, and a `let` all have a
//! rule that reads the type they are given. Elimination forms **infer**: an
//! application and a projection compute a type from the type of what they
//! eliminate. That split is not an implementation preference — it is what
//! makes the *only* place conversion is called be §2's `Switch` rule, so a
//! program's acceptance depends on one comparison per node rather than on the
//! order in which a checker happened to reach its constraints.
//!
//! The split is also *exclusive*, and that is the part worth defending. A λ and
//! a record literal each have a type it is tempting to hand them — the binder's
//! domain from a metavariable, the fields' own types as a non-dependent
//! telescope — and neither of those is a principal type. `{ ty = {}, val = {} }`
//! inhabits `{ ty : Type 0, val : ty }` just as well as `{ ty : Type 0,
//! val : {} }`, so choosing the second is deciding what the program means on
//! evidence the author did not give. It is the same move conversion is
//! forbidden from making one layer down, and it is refused here for the same
//! reason: [`Refusal::Uninferable`], not a guess. The λ is the one exception,
//! and only because its guess is confined to a hole that must still be
//! *solved* by something the author wrote.
//!
//! # Where a type parameter is filled, and where filling stops
//!
//! A binder marked [`Filling::Parameter`] is filled at every use by a hole the
//! written arguments then solve ([`MetaSource::TypeParameter`] names it when
//! they do not). Filling happens in two places and stops on two conditions, and
//! the stopping conditions are the whole subtlety:
//!
//! - **At a use site**, [`spine`]'s walk fills parameter binders until the type
//!   is no longer one. It is *not* run when the author wrote the argument
//!   themselves, because that argument is the one the binder wanted.
//! - **In checking mode**, a term checked against a parameter Π is wrapped in a
//!   λ for it rather than switched to inference. Switching instead would infer
//!   a type, fill its parameters, and unify against a type that is still a
//!   parameter Π — which fills forever.
//!
//! # What the output contains
//!
//! No holes. One that is still unsolved when elaboration ends is
//! [`Refusal::Unsolved`] (§2.1 never defaults and never generalizes), and one
//! that is solved is substituted away by [`Elaborator::zonk`]. That is what
//! lets [`crate::check`] promise a term whose every argument is written.
//!
//! # Levels are not unknowns
//!
//! §2.1's third creation site was a level, and it is gone with universe
//! polymorphism: there are two levels, `Type 0` and `Type 1`, and a bare
//! `Type` elaborates at the one its use demands rather than at a level
//! something has to solve. Nothing here creates a level unknown, and there is
//! no level sort to unify in.
//!
//! # The rules, by file
//!
//! Each file below holds the rules of one judgment, split by what a rule
//! *does* rather than by the syntax it happens to match:
//!
//! - [`check`] — the checking judgment, and the introduction forms that read
//!   the type they are given.
//! - [`infer`] — the inference judgment, and the elimination forms that
//!   compute a type from what they eliminate.
//! - [`name`] — what a bare name denotes: a binder, a declaration, a host
//!   item, a numeral.
//! - [`construct`] — a constructor applied, with or without an expected type
//!   naming its family.
//! - [`record`] — record types, projection, and update.
//! - [`spine`] — one instantiation walk: parameters filled, constraints noted,
//!   arguments placed.
//! - [`holes`] — an unknown's lifecycle: created, constrained, and audited at
//!   the one finish point.
//! - [`zonk`] — solutions written back into the term that is stored.

use std::sync::Arc;

use crate::budget::Meter;
use crate::context::Cx;

use crate::convert::Conversion;
use crate::eval::opened;
use crate::level::Level;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::raw::Raw;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::Term;
use crate::value::{Env, Form, Value};

mod check;
mod construct;
mod holes;
mod infer;
mod name;
mod record;
mod spine;
mod zonk;

/// A term and the type it was elaborated at.
///
/// The type is a [`Value`] rather than a [`Term`] because every rule that reads
/// it matches on its shape, and re-normalizing at each node is the cost the
/// semantic domain exists to avoid.
struct Typed {
    term: Term,
    ty: Value,
}

/// A `let`'s definition, and the scope its body is read in.
struct Bound {
    scope: Scope,
    ty_term: Term,
    value_term: Term,
}

/// One elaboration.
///
/// There is nothing in here but the budget and the fresh-variable counter: the
/// calculus has no metavariables to track, no constraints to postpone, and no
/// levels to solve — `02-core-calculus.md` §2.1's instantiation is one matching
/// pass per application, and this struct is what a pass borrows.
pub(crate) struct Elaborator {
    meter: Meter,
    /// The one matching table every pass in this elaboration shares.
    ///
    /// Shared rather than per-application because a solution can arrive from
    /// anywhere in the expression: `xs.fold_from_end(None, step)` learns the
    /// seed's parameter only when `step`'s annotation meets the fold's type.
    /// What makes sharing sound is that assignment is the *only* thing a pass
    /// can do to the table, and an assignment is justified by the match that
    /// made it wherever the variable was created.
    conversion: Conversion,
    /// The constraints the walks have met, each with the hole its dictionary
    /// will fill — resolved once, at [`Self::settled`], when matching has said
    /// everything it can.
    ///
    /// Declaration-end rather than walk-end because a method's constraint is
    /// created before the receiver that solves its parameter is applied, and
    /// walk-end would be a second, earlier place the same failure could be
    /// reported — one place, one message.
    constraints: Vec<(Arc<crate::class::Constraint>, Scope, Env, Origin, crate::meta::Hole)>,
    /// Every hole an instantiation walk has created, in creation order.
    ///
    /// Kept so that [`Self::settled`] names the *first* parameter nothing
    /// determined rather than whichever one a walk of the output happened to
    /// reach — the earliest is the one the author's next edit is about.
    created: Vec<crate::meta::Hole>,
    /// The next hole's identity.
    next_hole: u32,
}

impl Elaborator {
    pub(crate) fn new(cx: &Cx) -> Self {
        Self {
            meter: cx.meter(),
            conversion: Conversion::solving(),
            constraints: Vec::new(),
            created: Vec::new(),
            next_hole: 0,
        }
    }

    /// The meter this elaboration is spending.
    ///
    /// Handed out rather than wrapped, for the one caller outside this module —
    /// [`crate::declare`], which evaluates and quotes between elaborations and
    /// must spend the same budget doing it, or a declaration would get a fresh
    /// allowance per telescope.
    pub(crate) const fn meter(&mut self) -> &mut Meter {
        &mut self.meter
    }

    /// What this elaboration has charged.
    pub(crate) const fn spent(&self) -> crate::Spend {
        self.meter.spent()
    }

    /// Elaborate `raw` against the type `ty`, and finish.
    pub(crate) fn run_check(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
        let term = self.check(scope, raw, ty)?;
        self.settled()?;
        self.zonk(&term)
    }

    /// Elaborate `raw`, answering it and its type, and finish.
    ///
    /// The inferred type is answered *outside* the term: an inference like
    /// `let r = … in r.val` has a type mentioning `r`, and the binder is not in
    /// scope where the answer is read — so definitions are opened.
    pub(crate) fn run_infer(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Term), ElabError> {
        let inferred = self.infer(scope, raw)?;
        // After resolution, so the type's quotation reads the solutions the
        // constraints were resolved to rather than the holes they stood at.
        self.settled()?;
        let term = self.zonk(&inferred.term)?;
        let ty = quote_type(
            &mut self.meter,
            Depth(scope.depth()),
            crate::quote::Mode::Open,
            &inferred.ty,
        )?;
        Ok((term, ty))
    }

    /// Elaborate `raw` against `ty` **without finishing**.
    ///
    /// [`Self::run_check`] is one whole judgment. A declaration is many
    /// judgments that share one budget, so it checks each part with this and
    /// finishes once.
    pub(crate) fn check_open(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
        self.check(scope, raw, ty)
    }

    /// Elaborate `raw` and answer its type as a **value**, without finishing.
    ///
    /// The type is not read back, unlike [`Self::run_infer`]'s: a `match`
    /// subject's type is immediately taken apart into the family it names, and
    /// quoting it only to evaluate it again would be work performed to be
    /// undone.
    pub(crate) fn infer_open(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Value), ElabError> {
        let inferred = self.infer(scope, raw)?;
        Ok((inferred.term, inferred.ty))
    }

    /// Elaborate a term standing in type position, answering its universe.
    pub(crate) fn check_type(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Level), ElabError> {
        let (term, level) = self.formed_type(scope, raw)?;
        // §1.5's arity check, on the side an indexed type is *not* written.
        // `Pc` alone is not a type when `Pc` was declared `Pc(n : Nat)`: it is
        // a type still waiting for the number it carries, and admitting it
        // would make the index optional, which is the same as not having one —
        // two values at two indices would meet at the bare type.
        //
        // **It is a check on what was written, not on what a term may hold**,
        // and the difference is erasure. `quote` drops the wrapper, so the
        // read-back of `Pc(12)` *is* the bare `Pc` and every stored artifact
        // holds one: a check over terms would refuse the erasure this stratum
        // exists to produce. A [`Raw`] is the one thing erasure never makes, so
        // asking here — where a written type expression arrives and nowhere
        // else — refuses the author and leaves the term language exactly as
        // prompt 142d left it.
        //
        // The one written form that is not a mistake is the head of `Pc(12)`,
        // which is holding the index this would say was missing; that caller
        // reads [`Self::formed_type`] instead.
        if !matches!(term.shape(), crate::term::Shape::Indexed { .. })
            && let Some(binder) = term.declared_index()
        {
            return Err(Refusal::MissingIndex {
                ty: crate::show::head_spelled(&term),
                binder: Arc::clone(&binder.name),
                at: raw.origin(),
            }
            .into());
        }
        Ok((term, level))
    }

    /// `raw` read as a type expression, with §1.5's arity check left to the
    /// caller.
    ///
    /// [`Self::check_type`] is this and that check; the split exists for one
    /// caller — `indexed_type_formation`, which is reading the *head* of
    /// `Pc(12)` and so is holding the index the check would complain was
    /// missing.
    ///
    /// # Errors
    ///
    /// [`Refusal::NotAType`] when the expression's own type is not a universe,
    /// and otherwise as [`crate::check`].
    pub(crate) fn formed_type(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Level), ElabError> {
        let inferred = self.infer(scope, raw)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::Universe(level) = &ty.form else {
            return Err(Refusal::NotAType {
                at: raw.origin(),
                ty: scope.quote_type(&mut self.meter, &inferred.ty)?,
            }
            .into());
        };
        Ok((inferred.term, *level))
    }

    /// # Errors
    ///
    /// [`Refusal::Mismatch`] when they cannot be made equal, or exhaustion.
    pub(crate) fn unify_types(
        &mut self,
        scope: &Scope,
        at: Origin,
        left: &Value,
        right: &Value,
    ) -> Result<(), ElabError> {
        self.conversion
            .unify_types(&mut self.meter, scope.depth(), at, left, right)
    }
}
