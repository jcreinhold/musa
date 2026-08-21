//! Bidirectional elaboration: a raw term goes in, a typed core term comes out.
//!
//! `docs/rules/language/02-core-calculus.md` §2 gives two judgments —
//! `Γ ⊢ e ⇐ A ⇝ t` and `Γ ⊢ e ⇒ A ⇝ t` — and exactly two rules that move
//! between them. This module is those judgments and nothing else: it decides no
//! surface question, knows no musical type, and never sees a token.
//!
//! # Which forms check and which infer
//!
//! Introduction forms **check**: a λ, a record literal, `refl`, and a `let` all
//! have a rule that reads the type they are given. Elimination forms **infer**:
//! an application, a projection, and `J` compute a type from the type of what
//! they eliminate. That split is not an implementation preference — it is what
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
//! evidence the author did not give. It is the same move the unifier is
//! forbidden from making one layer down, and it is refused here for the same
//! reason: [`Refusal::Uninferable`], not a guess. The λ is the one exception,
//! and only because its guess is confined to a metavariable that must still be
//! *solved* by something the author wrote.
//!
//! # Where implicits are inserted, and where insertion stops
//!
//! A binder marked [`Plicity::Implicit`] is filled by a metavariable at every
//! use ([`MetaSource::ImplicitArgument`]). Insertion happens in two places and
//! stops on two conditions, and the stopping conditions are the whole subtlety:
//!
//! - **At a use site**, [`Elaborator::inserted`] fills implicit binders until
//!   the type is no longer an implicit Π. It is *not* run when the author wrote
//!   the argument implicitly (`f {a}`), because that argument is the one the
//!   binder wanted.
//! - **In checking mode**, a term checked against `{x : A} → B` is wrapped in an
//!   implicit λ rather than switched to inference. Switching instead would infer
//!   a type, insert implicits into it, and unify against a type that is still an
//!   implicit Π — which inserts forever.
//!
//! # What the output contains
//!
//! No metavariables. One that is still unsolved when elaboration ends is
//! [`Refusal::Unsolved`] (§2.1 never defaults and never generalizes), and one
//! that is solved is substituted away by [`zonk`]. That is what lets
//! [`crate::check`] promise a term the re-checker accepts.
//!
//! # Levels are unknowns too
//!
//! §2.1's third creation site is a level, and prompt 135 opened it: a bare
//! `Type` gets a [`LevelMeta`] rather than a number chosen here, solved by the
//! same discipline as a term metavariable and refused by the same rule — one
//! still undetermined when elaboration ends is [`Refusal::Unsolved`], never
//! defaulted to zero.
//!
//! Read that site precisely. It is "a level position **the surface did not
//! write**", which the universe a *hole's own type* stands in is not: no
//! universe stands there at all, and [`Elaborator::infer_lambda`] says why a
//! metavariable there would refuse every unannotated binder instead of
//! describing one.
//!
//! The solver is [`Level::determine`] and it is narrower than the term unifier
//! on purpose — its bound is documented there rather than here, because it is a
//! property of the level sort and not of this module.
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
//! - [`spine`] — one instantiation walk: implicits filled, constraints noted,
//!   arguments placed.
//! - [`holes`] — an unknown's lifecycle: created, constrained, and audited at
//!   the one finish point.
//! - [`zonk`] — solutions written back into the term that is stored.

use std::sync::Arc;

use crate::budget::Meter;
use crate::context::Cx;

use crate::eval::opened;
use crate::level::Level;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::raw::Raw;
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::Term;
use crate::unify::Unifier;
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
    unifier: Unifier,
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
            unifier: Unifier::default(),
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
        self.unifier
            .unify_types(&mut self.meter, scope.depth(), at, left, right)
    }
}
