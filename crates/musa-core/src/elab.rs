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

use std::sync::Arc;

use crate::budget::Meter;
use crate::class::{Head, Key, head_of};
use crate::context::Cx;

use crate::error::Malformed;
use crate::eval::{apply_closure, eval, field_type, opened};
use crate::family::Found;
use crate::level::Level;
use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::quote::{Depth, quote_type};
use crate::raw::{Raw, RawConstraint, RawField, RawShape, RawUpdate};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Field, Index, Name, Plicity, Shape, Term};
use crate::unify::Unifier;
use crate::value::{Env, Form, Neutral, Telescope, Value};

/// A term and the type it was elaborated at.
///
/// The type is a [`Value`] rather than a [`Term`] because every rule that reads
/// it matches on its shape, and re-normalizing at each node is the cost the
/// semantic domain exists to avoid.
struct Typed {
    term: Term,
    ty: Value,
}

/// One instantiation pass's local state — see [`Elaborator::apply_spine`].
///
/// Just the spine: constraints are the elaborator's business, resolved at
/// declaration end, and an argument's term is pushed as it is elaborated.
#[derive(Default)]
struct Walk {
    /// The spine slots to fill when the walk ends, in walk order.
    slots: Vec<Slot>,
}

/// One spine slot of a [`Walk`].
enum Slot {
    /// An implicit parameter: the hole stands in the term whether or not the
    /// walk solved it, and [`Elaborator::settled`] audits at declaration end.
    Parameter(crate::meta::Hole),
    /// A constraint's dictionary, resolved at declaration end.
    Dictionary(crate::meta::Hole),
    /// An argument the author wrote, elaborated.
    Argument(Term),
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
        self.check(scope, raw, ty)
    }

    /// Elaborate `raw`, answering it and its type, and finish.
    ///
    /// The inferred type is answered *outside* the term: an inference like
    /// `let r = … in r.val` has a type mentioning `r`, and the binder is not in
    /// scope where the answer is read — so definitions are opened.
    pub(crate) fn run_infer(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Term), ElabError> {
        let inferred = self.infer(scope, raw)?;
        let ty = quote_type(
            &mut self.meter,
            Depth(scope.depth()),
            crate::quote::Mode::Open,
            &inferred.ty,
        )?;
        Ok((inferred.term, ty))
    }

    /// The one finish point a declaration shares: refuse the first parameter
    /// nothing in it determined.
    ///
    /// §2.1's discipline, stated where it is enforced: instantiation is one
    /// matching pass, and the report names the *earliest* unsolved parameter
    /// because that is the one the author's next edit is about.
    pub(crate) fn settled(&mut self) -> Result<(), ElabError> {
        // Constraints first: a dictionary resolved here is a hole solved, and
        // the parameter audit below should not call an instance's own
        // parameters undetermined for having answered one.
        let waiting = core::mem::take(&mut self.constraints);
        for (constraint, scope, env, at, hole) in waiting {
            // Resolved against the *creation* scope: the local dictionaries
            // step 1 looks up are the `where` binders in scope where the
            // constraint was met, and a retry anywhere else would prefer a
            // global instance to the author's own clause.
            let term = crate::dictionary::resolve_at(self, &scope, &constraint, &env, at)?;
            let value = crate::eval::eval(&mut self.meter, &env, &term)?;
            hole.solve(value).map_err(crate::error::CoreError::from)?;
        }
        if let Some(hole) = self.created.iter().find(|hole| !hole.is_solved()) {
            return Err(Refusal::Unsolved {
                site: MetaSource::TypeParameter,
                created: hole.origin(),
                blocked: None,
            }
            .into());
        }
        Ok(())
    }

    /// A hole for a constraint's dictionary, registered for resolution at
    /// [`Self::settled`].
    ///
    /// The registration is the whole of postponement that survives the course
    /// correction: no queue is retried, the constraint is resolved once, when
    /// the declaration's matching has said everything it can.
    pub(crate) fn constrain(
        &mut self,
        scope: &Scope,
        constraint: Arc<crate::class::Constraint>,
        env: Env,
        at: Origin,
        ty: &Value,
    ) -> crate::meta::Hole {
        let hole = self.fresh_hole(at, ty);
        self.constraints.push((constraint, scope.clone(), env, at, hole.clone()));
        hole
    }

    /// A placeholder for an implicit argument the walk has not solved yet.
    ///
    /// Creation is where §2.1's discipline is cheap to state: the hole's type
    /// was checked before it was made, and solving is write-once, so the audit
    /// [`Self::settled`] runs is a walk of a list and not a query of a solver.
    pub(crate) fn fresh_hole(&mut self, here: Origin, ty: &Value) -> crate::meta::Hole {
        let hole = crate::meta::Hole::new(self.next_hole, here, ty.clone());
        self.next_hole = self.next_hole.saturating_add(1);
        self.created.push(hole.clone());
        hole
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

    /// A name no binder in scope answers to, which a declaration may.
    ///
    /// Constants are looked up *after* binders rather than merged with them, so
    /// a local binder named `Nat` shadows the family — the ordinary rule, and the
    /// one an author expects from every other name in the language.
    ///
    /// A recursor gets its motive universe here, and here is the only place it
    /// can: §1.3 admits no universe polymorphism beyond level metavariables, so
    /// the level is one per *use site* and this is what a use site is.
    fn constant(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        let Some(found) = scope.declared(name) else {
            // A top-level definition (§2.4), after declarations for the reason
            // declarations come after binders — the more local answer wins —
            // and before the two below because both of those are the host's
            // namespaces rather than the author's.
            if let Some(defined) = scope.cx().definition(name) {
                if let Some(module) = defined.hidden_from(scope.cx().module()) {
                    return Err(Refusal::Private {
                        name: Arc::clone(name),
                        module,
                        at: here,
                    }
                    .into());
                }
                let def = crate::program::one(defined);
                let ty = Value::clone(&def.ty());
                return Ok(Typed {
                    term: def.term(here),
                    ty,
                });
            }
            // `Class.method` before the general report, and only after binders
            // and declarations: a trait's methods live in the trait's namespace,
            // so nothing here can shadow a name an author declared themselves.
            if let Some((term, ty)) = crate::dictionary::method_at(self, scope, here, name)? {
                return Ok(Typed { term, ty });
            }
            // Last, and last on purpose: the host's registry is consulted only
            // where nothing the author wrote answers, so a declaration always
            // shadows a base type or a builtin of the same spelling rather than
            // the other way round. A registry that won would let the host
            // silently redefine a name in a program it never read.
            return self.registered(scope, here, name);
        };
        // Found, and possibly not for this reader. The check is here rather
        // than inside the lookup so that the answer is "private" and not "not
        // found" — see [`crate::visibility`] for why that distinction is the
        // whole of `01-surface.md` §1.3's value.
        if let Some(module) = found.hidden_from(scope.cx().module()) {
            return Err(Refusal::Private {
                name: Arc::clone(name),
                module,
                at: here,
            }
            .into());
        }
        // A recursor's motive universe rides on the use site; a bare reference
        // is always one whose goal is an ordinary type. The match compiler
        // computes its own from the goal (`case.rs`) and never comes through
        // here.
        let level = Level::ZERO;
        let constant = found.at(level);
        let ty = constant.ty(&mut self.meter)?;
        Ok(Typed {
            term: constant.term(here),
            ty,
        })
    }

    /// A registered builtin or base type, and nothing else.
    fn registered(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        let Some(entry) = scope.cx().extern_named(name) else {
            return Err(self.unresolved(scope, here, name));
        };
        let ty = eval(&mut self.meter, &Env::EMPTY, entry.ty())?;
        Ok(Typed {
            term: entry.term(here),
            ty,
        })
    }

    /// [`RawShape::Hosted`]: a name the *reader* wrote, resolved in the host's
    /// namespaces only.
    ///
    /// Declarations then the registry, and neither binders nor top-level
    /// definitions. [`Self::constant`]'s order is the author's — the more local
    /// answer wins, so a program may name a value `sounded` and mean its own —
    /// and this one is the reader's, which is why the two orders differ. A
    /// desugaring that went through the author's namespace would let an
    /// ordinary binding change what `music { c5/1 }` means, and would make two
    /// definitions that never mentioned each other look like a cycle.
    fn hosted(&mut self, scope: &Scope, here: Origin, name: &Name) -> Result<Typed, ElabError> {
        if scope.declared(name).is_some() {
            return self.constant(scope, here, name);
        }
        // A trait's methods, for the reason [`Self::constant`] gives: they live
        // in the trait's namespace, which is the host's here as well.
        if let Some((term, ty)) = crate::dictionary::method_at(self, scope, here, name)? {
            return Ok(Typed { term, ty });
        }
        self.registered(scope, here, name)
    }

    /// [`RawShape::Numeral`]: a number at the family the reader named.
    ///
    /// Resolved in the host's namespaces, like [`Self::hosted`] and for the same
    /// reason: the family is the reader's word, so a binding that happens to
    /// spell `Nat` cannot change what a written number means. It infers rather
    /// than checks, because the raw term already says which type it is at — the
    /// same argument [`RawShape::Lit`] makes, one namespace over.
    fn numeral(&mut self, scope: &Scope, here: Origin, family: &Name, count: u64) -> Result<Typed, ElabError> {
        let Some(Found::Rigid(constant)) = scope.declared(family) else {
            return Err(self.unresolved(scope, here, family));
        };
        let Some(reason) = constant.uncounted() else {
            return Ok(Typed {
                term: Term::numeral(here, &constant, count),
                ty: constant.value(here),
            });
        };
        Err(Refusal::NotANumeralFamily {
            at: here,
            name: Arc::clone(family),
            reason,
        }
        .into())
    }

    /// Why a name resolved to nothing, as precisely as the context can say.
    ///
    /// Three different mistakes wear the same spelling, and telling them apart
    /// is the whole value of the report. `Tying.Tied` reached a namespace that
    /// exists and has no such case; a bare `Untied` is a case of something, and
    /// this position does not say of what; anything else is a name nobody
    /// declared.
    fn unresolved(&mut self, scope: &Scope, here: Origin, name: &Name) -> ElabError {
        if let Some(family) = scope.cx().stranger(name) {
            let ty = match family.ty_term(&mut self.meter) {
                Ok(ty) => ty,
                Err(error) => return error.into(),
            };
            return Refusal::NoSuchConstructor {
                at: here,
                name: Arc::clone(name),
                ty,
                cases: family.cases(),
            }
            .into();
        }
        let families = scope.cx().cases(name);
        if !families.is_empty() {
            return Refusal::BareConstructor {
                at: here,
                name: Arc::clone(name),
                families,
            }
            .into();
        }
        Refusal::UnknownName {
            name: Arc::clone(name),
            at: here,
            candidates: scope.nameable(),
        }
        .into()
    }

    // ---- checking ----------------------------------------------------------

    /// `Γ ⊢ raw ⇐ ty ⇝ t`.
    fn check(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
        let unfolded = opened(&mut self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        match self.checked(scope, raw, ty)? {
            Some(term) => Ok(term),
            // §2's `Switch`, and the only rule in this module that calls
            // conversion. A constructor reaches it having read its family's
            // parameters off `ty` — see [`Self::constructed`] — and everything
            // else reaches it having inferred, which is the difference between
            // the two and the whole of it.
            None => {
                let inferred = match self.constructed(scope, raw, ty)? {
                    Some(supplied) => supplied,
                    None => self.infer(scope, raw)?,
                };
                // §2.1 at the one place it can fire from below: a term whose
                // inferred type still quantifies over parameters the expected
                // type can determine — a bare constructor, a generic's name —
                // is matched against `ty` before conversion is asked.
                let inferred = self.apply_spine(scope, raw.origin(), inferred, &[], Some(ty))?;
                if let Some(carried) = self.carried(scope, ty, &inferred)? {
                    return Ok(carried);
                }
                self.unifier
                    .unify_types(&mut self.meter, scope.depth(), raw.origin(), ty, &inferred.ty)?;
                Ok(inferred.term)
            }
        }
    }

    /// `inferred`, carried into a position of type `ty` that accepts it at a
    /// different index — or `None` when no host rule applies and conversion
    /// decides.
    ///
    /// The core's one subsumption rule, and it is the host's rather than the
    /// core's: see [`Accepts`]. Two things about *where* it stands are the whole
    /// of why it is sound.
    ///
    /// **It is here and not in [`Unifier`].** Conversion is symmetric, so a rule
    /// living there would let a value stand at either index and the index would
    /// certify nothing. `check`'s `Switch` is the only place in the elaborator
    /// where one type is *expected* and another *found*, which is exactly the
    /// asymmetry an acceptance rule needs — `11-quotation.md` §1 states its
    /// forgetting rule directionally for the same reason.
    ///
    /// **It elaborates to a coercion rather than to bare acceptance.** The
    /// alternative — return `inferred.term` unchanged and skip unification — is
    /// what the replaced checker did, and it would leave a term whose type is
    /// `Syntax ⟨expr⟩` standing where the elaborated program says
    /// `Syntax ⟨token-tree⟩`. [`crate::well_typed`] would then refuse a term
    /// this elaborator produced, which is the one invariant prompt 134 called
    /// the most valuable in the crate. So the checker inserts the carrier, the
    /// author never writes it, and the core keeps no notion of subtyping at all.
    ///
    /// # Errors
    ///
    /// [`Malformed::UnregisteredCarrier`] when the host's rule names an
    /// operation its own registry does not hold, and whatever forcing the two
    /// indices costs.
    fn carried(&mut self, scope: &Scope, ty: &Value, inferred: &Typed) -> Result<Option<Term>, ElabError> {
        let (Some((expected, wanted)), Some((found, held))) =
            (self.at_a_literal_index(ty)?, self.at_a_literal_index(&inferred.ty)?)
        else {
            return Ok(None);
        };
        if expected != found {
            return Ok(None);
        }
        // The rule is the *registry's*, not the one carried by whichever `Base`
        // this type was built from. A host writes a base type's term at many
        // sites — `Base` compares by name for exactly that reason — so reading
        // the rule off the embedded declaration reads it off whichever copy the
        // type happened to be built from, and only the copy in the registry was
        // decorated. Storability is already read this way, off the registered
        // bases and never off a type's own; this is the same authority.
        let Some(crate::base::Extern::Base(declared)) = scope.cx().extern_named(expected.name()) else {
            return Ok(None);
        };
        let Some(carrier) = declared.accepts().and_then(|accepts| accepts(&wanted, &held)) else {
            return Ok(None);
        };
        let Some(crate::base::Extern::Builtin(builtin)) = scope.cx().extern_named(carrier) else {
            return Err(Malformed::UnregisteredCarrier(carrier.into()).into());
        };
        let here = inferred.term.origin();
        let builtin = builtin.term(here);
        Ok(Some(Term::app(here, builtin, inferred.term.clone())))
    }

    /// The base type and index of `ty`, when it is one applied to one literal.
    ///
    /// The shape [`Accepts`] is asked about and the only one: a base type at no
    /// index has no second position to accept from, and one whose index is a
    /// variable or a metavariable is not settled enough to ask about — the rule
    /// reads two literals, so both sides must have got that far.
    fn at_a_literal_index(
        &mut self,
        ty: &Value,
    ) -> Result<Option<(crate::base::Base, crate::base::Literal)>, ElabError> {
        // Forced first, and not only at the index. `check` forces the type it
        // was handed, but the *inferred* type reaching this arrives straight out
        // of `infer` — and for a call whose result is an implicit parameter that
        // is a metavariable, solved by the first argument that mentions it. A
        // solved metavariable is a neutral with no spine, so reading the head
        // without forcing sees `Head::Meta` and answers that this is not a type
        // at a literal index, one layer of indirection away from the type that
        // plainly is.
        let unfolded = opened(&mut self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        let Form::Neutral(ref neutral) = ty.form else {
            return Ok(None);
        };
        let crate::value::Head::Base(ref base) = neutral.head else {
            return Ok(None);
        };
        let [crate::value::Elim::App { ref argument, .. }] = neutral.spine[..] else {
            return Ok(None);
        };
        let unfolded = opened(&mut self.meter, argument)?;
        let index = unfolded.as_ref().unwrap_or(argument);
        let Form::Lit(ref literal) = index.form else {
            return Ok(None);
        };
        Ok(Some((base.clone(), literal.clone())))
    }

    /// The checking rule for `raw` at `ty`, or `None` when it has none and §2's
    /// `Switch` applies.
    fn checked(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Term>, ElabError> {
        let here = raw.origin();
        match raw.shape() {
            RawShape::Lam {
                plicity,
                name,
                domain,
                body,
            } => self.lambda(scope, raw, plicity, name, domain.as_ref(), body, ty),
            RawShape::Record(fields) => {
                let Form::RecordType(telescope) = &ty.form else {
                    return self.abstracted(scope, raw, ty);
                };
                let telescope = telescope.clone();
                self.literal(scope, here, fields, &telescope).map(Some)
            }
            // A `let` checks by checking its body: the definition is elaborated
            // either way, and its binder is what the body is read under.
            RawShape::Let {
                name,
                ty: written,
                value,
                body,
            } => {
                let bound = self.definition(scope, name, written.as_ref(), value)?;
                Ok(Some(Term::bind(
                    here,
                    Arc::clone(name),
                    bound.ty_term,
                    bound.value_term,
                    self.check(&bound.scope, body, ty)?,
                )))
            }
            // §6.2: a `match` checks and never infers. The motive a split
            // builds is the goal abstracted over the subject, so there is
            // nothing to abstract without one — and reading the type off the
            // first arm would make a program's type depend on the order its
            // arms are written in.
            RawShape::Match { subjects, arms } => crate::case::compile(self, scope, here, subjects, arms, ty).map(Some),
            RawShape::Rec {
                name,
                ty: written,
                body,
            } => crate::rec::define(self, scope, here, name, written, body, ty).map(Some),
            RawShape::Var(_)
            | RawShape::Hosted(_)
            | RawShape::Lit(_)
            | RawShape::Numeral { .. }
            | RawShape::Universe(_)
            | RawShape::Pi { .. }
            | RawShape::ConstrainedPi { .. }
            | RawShape::App { .. }
            | RawShape::Call { .. }
            | RawShape::RecordType(_)
            | RawShape::Method { .. }
            | RawShape::Project { .. }
            | RawShape::Update { .. }
            | RawShape::Annot { .. } => self.abstracted(scope, raw, ty),
        }
    }

    /// §2's constructor rule in a checking position: `C a⃗ ⇐ N p⃗`.
    ///
    /// A bare case name resolves against the family the expected type names —
    /// the whole of what "a constructor is checked" means; [`case_named`]
    /// owns the two spellings and the one coincidence §1.3 allows. The
    /// family's parameters are no longer read off and applied here: they are
    /// the constructor's implicit binders, so the application pass matches
    /// them out of `ty` at the end, which is the same rule §2.1 states for
    /// every other call.
    fn constructed(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Typed>, ElabError> {
        let here = raw.origin();
        let Some((head, arguments)) = written_spine(raw) else {
            return Ok(None);
        };
        let Some((case, fields)) = self.case_of(scope, head, ty)? else {
            return Ok(None);
        };
        if arguments.len() > fields {
            return Ok(None);
        }
        // Through the ordinary constant rule, so that a case a module keeps to
        // itself is refused here the same way it is refused when its qualified
        // name is written out.
        let built = self.infer(scope, &Raw::var(here, case))?;
        Ok(Some(self.apply_spine(scope, here, built, &arguments, Some(ty))?))
    }

    /// The qualified case `head` denotes, and how many fields it takes.
    ///
    /// The expected type first, because that is the rule §2 states and the
    /// only one a bare case name can use — and it applies to a name from
    /// either namespace, since which namespace a name came from decides how it
    /// *resolves* and not what it may be checked against.
    ///
    /// The written registry second, and only for [`RawShape::Hosted`]. An
    /// author's `Option.Some Int` is `Some` at the parameter `Int`, still
    /// wanting its field; a reader's is `Some` holding the field `Int` at a
    /// parameter the application pass will solve. Both are well-typed readings
    /// of the same spine, so the namespace is what tells them apart — see
    /// [`Self::constructed_open`], which draws the same line where there is no
    /// expected type at all.
    fn case_of(&mut self, scope: &Scope, head: &Raw, ty: &Value) -> Result<Option<(String, usize)>, ElabError> {
        let (RawShape::Var(name) | RawShape::Hosted(name)) = head.shape() else {
            return Ok(None);
        };
        if let Some(element) = crate::family::element(&mut self.meter, ty)?
            && let Some(declared) = element.group.family_at(element.family)
            && let Some((case, fields)) = case_named(scope, name, declared)
        {
            return Ok(Some((format!("{}.{case}", declared.name), fields)));
        }
        if !matches!(head.shape(), RawShape::Hosted(_)) {
            return Ok(None);
        }
        let Some((fields, _)) = written_case(scope, name) else {
            return Ok(None);
        };
        Ok(Some((name.to_string(), fields)))
    }

    /// §2's constructor rule reached from the other direction: `C a⃗ ⇒ N ?p⃗`,
    /// where the `?p⃗` are the application pass's holes.
    ///
    /// Three shapes arrive here. A **reader-written** constructor (a
    /// [`RawShape::Hosted`] name) knows its family from the name itself. A
    /// **bare** case name — `Nothing`, not applied — answers to no binder and
    /// no declaration, and then §2.1's rule is uniqueness: exactly one family
    /// may declare the case, which is what makes the reading determined rather
    /// than guessed. Any other shape is not a constructor and answers `None`,
    /// leaving the name to the ordinary variable rule and its refusal.
    ///
    /// A case that is still *applied* afterwards keeps its remaining fields as
    /// an ordinary Π: `Some` alone is a function value, and §1.3's arity law
    /// is about calls, not names.
    fn constructed_open(&mut self, scope: &Scope, raw: &Raw) -> Result<Option<Typed>, ElabError> {
        let here = raw.origin();
        let (head, arguments) = written_spine(raw).unwrap_or((raw, Vec::new()));
        match head.shape() {
            RawShape::Hosted(name) => {
                let Some((fields, _)) = written_case(scope, name) else {
                    return Ok(None);
                };
                if arguments.len() > fields {
                    return Ok(None);
                }
                let built = self.infer(scope, &Raw::var(here, name.to_string()))?;
                Ok(Some(self.apply_spine(scope, here, built, &arguments, None)?))
            }
            RawShape::Var(name) => {
                if scope.lookup(name).is_some() {
                    return Ok(None);
                }
                let families = scope.cx().cases(name);
                let [family] = families.as_slice() else {
                    return Ok(None);
                };
                let Some(crate::family::Found::Rigid(constant)) = scope.declared(family) else {
                    return Ok(None);
                };
                let Some(declared) = constant.group.family_at(constant.family) else {
                    return Ok(None);
                };
                let Some((case, fields)) = case_named(scope, name, declared) else {
                    return Ok(None);
                };
                if arguments.len() > fields {
                    return Ok(None);
                }
                let qualified: Name = Arc::from(format!("{}.{case}", declared.name).as_str());
                let built = self.infer(scope, &Raw::var(here, qualified))?;
                Ok(Some(self.apply_spine(scope, here, built, &arguments, None)?))
            }
            _ => Ok(None),
        }
    }

    /// §2.1's one instantiation pass: apply `head` to the written arguments,
    /// then match what remains against `expected` when the call is in a
    /// checking position.
    ///
    /// One left-to-right walk, and the discipline is the document's: an
    /// implicit parameter becomes a [hole](crate::meta::Hole) that the first
    /// argument to mention it solves; an argument is *inferred* when its
    /// domain still mentions an unsolved hole and *checked* otherwise, because
    /// those are the two directions in which information can flow; a
    /// constraint waits until the walk has said everything matching can say,
    /// and is then resolved once, by lookup, never postponed. The author sees
    /// the two errors this can raise — a parameter nothing determined
    /// ([`Refusal::Unsolved`], at declaration end) and a call against a
    /// non-function ([`Refusal::NotAFunction`], here) — and neither involves
    /// a mechanism they have to name.
    fn apply_spine(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Typed,
        arguments: &[&Raw],
        expected: Option<&Value>,
    ) -> Result<Typed, ElabError> {
        let mut walk = Walk::default();
        let mut ty = head.ty.clone();
        self.advance(scope, &mut ty, &mut walk)?;
        for argument in arguments {
            let unfolded = opened(&mut self.meter, &ty)?;
            let current = unfolded.as_ref().unwrap_or(&ty);
            let Form::Pi { domain, codomain, .. } = &current.form else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &ty)?,
                }
                .into());
            };
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            // The domain decides the direction (§2.1): still quantified, the
            // argument teaches the parameter — inferred, and matched;
            // settled, the argument is checked.
            let argument_term = if crate::unify::mentions_unsolved(&domain) {
                let inferred = self.infer(scope, argument)?;
                self.unifier.unify_types(
                    &mut self.meter,
                    scope.depth(),
                    argument.origin(),
                    &domain,
                    &inferred.ty,
                )?;
                inferred.term
            } else {
                self.check(scope, argument, &domain)?
            };
            let value = scope.eval(&mut self.meter, &argument_term)?;
            walk.slots.push(Slot::Argument(argument_term));
            ty = apply_closure(&mut self.meter, &codomain, value)?;
            self.advance(scope, &mut ty, &mut walk)?;
        }
        if let Some(expected) = expected {
            // Checking position: the rest of the type is matched against what
            // the position wants, which is where a bare constructor's family
            // parameters — and any argument's still-unsolved ones — are
            // learned.
            self.unifier
                .unify_types(&mut self.meter, scope.depth(), here, &ty, expected)?;
        }
        Ok(self.finish_walk(scope, here, head.term, ty, walk)?)
    }

    /// Skip the binders §2.1 fills rather than the author: an implicit
    /// parameter becomes a fresh hole, a constraint is noted for the walk's
    /// end.
    fn advance(&mut self, scope: &Scope, ty: &mut Value, walk: &mut Walk) -> Result<(), ElabError> {
        let _ = scope;
        loop {
            let unfolded = opened(&mut self.meter, ty)?;
            let current = unfolded.as_ref().unwrap_or(ty);
            let Form::Pi {
                plicity,
                domain,
                codomain,
                ..
            } = &current.form
            else {
                return Ok(());
            };
            match plicity {
                Plicity::Explicit => return Ok(()),
                Plicity::Implicit => {
                    let hole = self.fresh_hole(current.origin, domain);
                    walk.slots.push(Slot::Parameter(hole.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Hole(hole)));
                    *ty = apply_closure(&mut self.meter, &codomain, value)?;
                }
                Plicity::Constraint(constraint) => {
                    let constraint = Arc::clone(constraint);
                    // The codomain reads the dictionary off its binder; a hole
                    // stands for it, and [`Self::settled`] writes the resolved
                    // dictionary in — the one place resolution runs, for the
                    // reason the field's doc gives.
                    let hole = self.fresh_hole(current.origin, domain);
                    self.constraints.push((
                        constraint,
                        scope.clone(),
                        codomain.env.clone(),
                        current.origin,
                        hole.clone(),
                    ));
                    walk.slots.push(Slot::Dictionary(hole.clone()));
                    let value = Value::neutral(Neutral::head(current.origin, crate::value::Head::Hole(hole)));
                    *ty = apply_closure(&mut self.meter, &codomain, value)?;
                }
            }
        }
    }

    /// The walk's end: build the spine, and leave the residual type with the
    /// holes it still mentions — solved or not, which [`Self::settled`]
    /// audits.
    fn finish_walk(
        &mut self,
        scope: &Scope,
        here: Origin,
        head: Term,
        ty: Value,
        walk: Walk,
    ) -> Result<Typed, ElabError> {
        let _ = scope;
        let mut term = head;
        for slot in &walk.slots {
            let argument = match slot {
                Slot::Parameter(hole) | Slot::Dictionary(hole) => Term::hole(here, hole.clone()),
                Slot::Argument(term) => term.clone(),
            };
            term = Term::app(here, term, argument);
        }
        Ok(Typed { term, ty })
    }

    /// Wrap `raw` in an implicit λ when the type it is checked against wants
    /// one, or answer `None` so that `Switch` runs.
    ///
    /// The second half of implicit insertion, and the reason `Switch` never
    /// meets an implicit Π: a term whose own form does not abstract the binder
    /// has one abstracted for it here.
    fn abstracted(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Term>, ElabError> {
        let Form::Pi {
            plicity,
            name,
            domain,
            codomain,
        } = &ty.form
        else {
            return Ok(None);
        };
        if *plicity == Plicity::Explicit {
            return Ok(None);
        }
        let (plicity, name, domain, codomain) =
            (plicity.clone(), Arc::clone(name), Arc::clone(domain), codomain.clone());
        let here = raw.origin();
        let variable = scope.fresh_var(here, Arc::clone(&domain));
        let body_ty = apply_closure(&mut self.meter, &codomain, variable)?;
        let discharged = self.discharging(scope, &plicity, &codomain.env)?;
        let inner = discharged.assume(Some(Arc::clone(&name)), here, domain);
        Ok(Some(Term::lam(here, name, self.check(&inner, raw, &body_ty)?)))
    }

    /// `scope` with a constraint binder's key discharged, for the binder about
    /// to be assumed.
    ///
    /// The second half of the abstracting rule, and not an optimization:
    /// `10-traits.md` §4 step 1 answers a constraint from "a dictionary bound by
    /// an enclosing `where` clause", so a body that writes `x == y` inside
    /// `same` has to reach *this* binder rather than a global instance. Without
    /// it the binder would be bound and unreachable, and a generic definition
    /// would type-check and then resolve to an instance its caller did not
    /// choose. [`crate::dictionary`]'s `requirements` does both for a derived
    /// method's `where`, and this is the same pair one level out.
    ///
    /// A constraint whose head is not yet known discharges nothing, which is
    /// §4's postponement rather than a failure: the key does not exist yet, and
    /// a use inside the body is postponed until it does.
    pub(crate) fn discharging(&mut self, scope: &Scope, plicity: &Plicity, at: &Env) -> Result<Scope, ElabError> {
        let Plicity::Constraint(constraint) = plicity else {
            return Ok(scope.clone());
        };
        let needed = crate::dictionary::instantiated(self, scope, constraint, at)?;
        let Some(key) = crate::dictionary::discharges(&needed, scope) else {
            return Ok(scope.clone());
        };
        let args = crate::dictionary::valued(self, scope, &needed)?;
        Ok(scope.discharging(key, scope.depth(), args))
    }

    /// `λx. e ⇐ (x : A) → B`, and the plicity rules that go with it.
    fn lambda(
        &mut self,
        scope: &Scope,
        raw: &Raw,
        plicity: &Plicity,
        name: &Name,
        domain: Option<&Raw>,
        body: &Raw,
        ty: &Value,
    ) -> Result<Option<Term>, ElabError> {
        let here = raw.origin();
        let Form::Pi {
            plicity: expected,
            name: _,
            domain: expected_domain,
            codomain,
        } = &ty.form
        else {
            return self.abstracted(scope, raw, ty);
        };
        if *plicity != *expected {
            // An implicit λ at an explicit binder is a mistake rather than a
            // term to abstract around: the author wrote the binder, at the
            // plicity the type does not have.
            if *plicity == Plicity::Implicit {
                return Err(Refusal::PlicityMismatch { at: here }.into());
            }
            return self.abstracted(scope, raw, ty);
        }
        let (expected_domain, codomain) = (Arc::clone(expected_domain), codomain.clone());
        // An annotation on a binder whose type is already known is not ignored:
        // it is elaborated and made to agree, so a wrong one is a refusal rather
        // than dead text.
        if let Some(written) = domain {
            let (term, _) = self.check_type(scope, written)?;
            let written_domain = scope.eval(&mut self.meter, &term)?;
            self.unifier.unify_types(
                &mut self.meter,
                scope.depth(),
                written.origin(),
                &expected_domain,
                &written_domain,
            )?;
        }
        let variable = scope.fresh_var(here, Arc::clone(&expected_domain));
        let body_ty = apply_closure(&mut self.meter, &codomain, variable)?;
        let inner = scope.assume(Some(Arc::clone(name)), here, expected_domain);
        Ok(Some(Term::lam(
            here,
            Arc::clone(name),
            self.check(&inner, body, &body_ty)?,
        )))
    }

    /// `{ f = e, … } ⇐ { f : A, … }`.
    ///
    /// The literal must give the telescope's fields in its order, because a
    /// later field's type may mention an earlier field's *value* — so the order
    /// is part of the type rather than a formatting preference.
    fn literal(
        &mut self,
        scope: &Scope,
        here: Origin,
        fields: &[RawField],
        telescope: &Telescope,
    ) -> Result<Term, ElabError> {
        if fields.len() != telescope.fields.len()
            || fields
                .iter()
                .zip(telescope.fields.iter())
                .any(|(written, declared)| written.name != declared.name)
        {
            return Err(Refusal::RecordShape {
                at: here,
                expected: telescope.fields.iter().map(|field| Arc::clone(&field.name)).collect(),
                found: fields.iter().map(|field| Arc::clone(&field.name)).collect(),
            }
            .into());
        }
        let mut env = telescope.env.clone();
        let mut elaborated = Vec::with_capacity(fields.len());
        for (written, declared) in fields.iter().zip(telescope.fields.iter()) {
            let field_ty = eval(&mut self.meter, &env, &declared.term)?;
            let term = self.check(scope, &written.term, &field_ty)?;
            // The telescope proceeds under this field's own value, which is what
            // makes a later field's type able to mention it.
            env = env.push(scope.eval(&mut self.meter, &term)?);
            elaborated.push(Field {
                name: Arc::clone(&written.name),
                term,
            });
        }
        Ok(Term::new(here, Shape::Record(elaborated.into())))
    }

    /// `Γ ⊢ raw ⇒ ty ⇝ t`.
    fn infer(&mut self, scope: &Scope, raw: &Raw) -> Result<Typed, ElabError> {
        let here = raw.origin();
        match raw.shape() {
            RawShape::Var(name) => {
                // §2.1's uniqueness rule: a name no binder answers to may be
                // the one case exactly one family declares. Tried before the
                // declaration chain so that the family reading is the one an
                // argument position can still determine the parameters of.
                if scope.lookup(name).is_none()
                    && let Some(built) = self.constructed_open(scope, raw)?
                {
                    return Ok(built);
                }
                let Some(found) = scope.lookup(name) else {
                    return self.constant(scope, here, name);
                };
                Ok(Typed {
                    term: Term::var(here, found.index),
                    ty: Value::clone(&found.ty),
                })
            }
            RawShape::Hosted(name) => self.hosted(scope, here, name),
            // A literal carries the base type it inhabits, so it infers rather
            // than checks: the host wrote the type down when it made the
            // literal, and reading it off anything else would be guessing at
            // what the host already said.
            RawShape::Lit(literal) => Ok(Typed {
                term: literal.term(here),
                ty: eval(&mut self.meter, &Env::EMPTY, literal.ty())?,
            }),
            RawShape::Numeral { family, count } => self.numeral(scope, here, family, *count),
            RawShape::Universe(written) => {
                // §1: two universes, and a bare `Type` is `Type 0`.
                let level = written.as_ref().copied().unwrap_or(Level::ZERO);
                let Some(above) = level.succ() else {
                    return Err(Refusal::BeyondUniverses { at: here }.into());
                };
                Ok(Typed {
                    ty: Value::new(here, Form::Universe(above)),
                    term: Term::universe(here, level),
                })
            }
            RawShape::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => self.function_type(scope, here, plicity.clone(), name, domain, codomain),
            RawShape::ConstrainedPi { constraint, codomain } => {
                self.constrained_function_type(scope, here, constraint, codomain)
            }
            RawShape::Lam {
                plicity,
                name,
                domain,
                body,
            } => self.infer_lambda(scope, here, plicity.clone(), name, domain.as_ref(), body),
            RawShape::App {
                plicity,
                function,
                argument,
            } => match self.constructed_open(scope, raw)? {
                Some(built) => Ok(built),
                None => self.application(scope, here, plicity, function, argument),
            },
            // The same two steps the arm above takes, because a constructor
            // written with its fields is the form an author actually writes and
            // `Succ(fewer)` is one: `written_spine` reads both forms, so the
            // constructor rule sees the same head and the same arguments here.
            RawShape::Call { function, arguments } => match self.constructed_open(scope, raw)? {
                Some(built) => Ok(built),
                None => self.complete_call(scope, here, function, arguments),
            },
            RawShape::RecordType(fields) => self.record_type(scope, here, fields),
            // §2: a record literal is an introduction form, so it checks. The
            // type it "obviously" has is a guess rather than a principal type —
            // `{ ty = {}, val = {} }` inhabits both `{ ty : Type 0, val : ty }`
            // and `{ ty : Type 0, val : {} }` — and picking one would be the
            // unifier's forbidden habit of trying the solution that comes to
            // hand. So there is no inference rule, and an author who wants to
            // project out of a literal writes the type it should have.
            RawShape::Record(_) => Err(Refusal::Uninferable { at: here }.into()),
            RawShape::Method { receiver, method } => self.method(scope, here, receiver, method),
            RawShape::Project { record, field } => self.projection(scope, here, record, field),
            RawShape::Update { record, updates } => self.update(scope, here, record, updates),
            RawShape::Let {
                name,
                ty: written,
                value,
                body,
            } => {
                let bound = self.definition(scope, name, written.as_ref(), value)?;
                let inferred = self.infer(&bound.scope, body)?;
                Ok(Typed {
                    term: Term::bind(here, Arc::clone(name), bound.ty_term, bound.value_term, inferred.term),
                    ty: inferred.ty,
                })
            }
            // §2's `Annot`, the other mode-switch rule: an author writes a type
            // and the term is checked against it.
            RawShape::Annot { term, ty } => {
                let (ty_term, _) = self.check_type(scope, ty)?;
                let ty_value = scope.eval(&mut self.meter, &ty_term)?;
                Ok(Typed {
                    term: self.check(scope, term, &ty_value)?,
                    ty: ty_value,
                })
            }
            // §6.2's `match` and §2.4's `rec` check and never infer, for the
            // same reason a record literal does not: the type is what decides
            // the elaboration, and guessing it from an arm or from a body would
            // make the answer depend on which one was written first.
            RawShape::Match { .. } | RawShape::Rec { .. } => Err(Refusal::Uninferable { at: here }.into()),
        }
    }
    /// `(x : A) → B ⇒ Type (max l l')`.
    fn function_type(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: Plicity,
        name: &Name,
        domain: &Raw,
        codomain: &Raw,
    ) -> Result<Typed, ElabError> {
        let (domain_term, domain_level) = self.check_type(scope, domain)?;
        let domain_value = scope.eval(&mut self.meter, &domain_term)?;
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::new(domain_value));
        let (codomain_term, codomain_level) = self.check_type(&inner, codomain)?;
        Ok(Typed {
            term: Term::function(here, plicity, Arc::clone(name), domain_term, codomain_term),
            ty: Value::new(here, Form::Universe(domain_level.max(codomain_level))),
        })
    }

    /// `[Class a⃗] → B ⇒ Type (max l l')` — `01-surface.md` §1.4's `where`.
    ///
    /// It adds no term to the calculus, which is §1.4's own claim: what this
    /// builds is the Π that was already there, at a domain the author did not
    /// have to write because the trait and its arguments determine it. The
    /// level is asked of the assembled dictionary type rather than read off a
    /// raw one, since there is no raw one — [`crate::dictionary`] hands back a
    /// term, and `Class a⃗` β-reduces to the record type whose universe is the
    /// answer.
    ///
    /// The binder takes the trait's own name, which is what makes a body's
    /// `Eq` and the dictionary it stands at the same word in a diagnostic.
    fn constrained_function_type(
        &mut self,
        scope: &Scope,
        here: Origin,
        raw: &RawConstraint,
        codomain: &Raw,
    ) -> Result<Typed, ElabError> {
        let classes = scope.cx().classes().clone();
        let (constraint, domain_term) = crate::dictionary::constraint_at(self, scope, &classes, raw)?;
        let domain_value = scope.eval(&mut self.meter, &domain_term)?;
        let domain_level = Term::level_of(&domain_term)?;
        let name: Name = Arc::clone(&constraint.class);
        let constraint = Arc::new(constraint);
        // Discharged as well as assumed, for [`Self::discharging`]'s reason: a
        // codomain that mentions the trait's own methods is answered by the
        // binder standing right there.
        let inner = self.discharging(scope, &Plicity::Constraint(Arc::clone(&constraint)), scope.env())?;
        let inner = inner.assume(Some(Arc::clone(&name)), here, Arc::new(domain_value));
        let (codomain_term, codomain_level) = self.check_type(&inner, codomain)?;
        Ok(Typed {
            term: Term::constrained_pi(here, constraint, name, domain_term, codomain_term),
            ty: Value::new(here, Form::Universe(domain_level.max(codomain_level))),
        })
    }

    /// `λx. e ⇒ (x : A) → B`, where `A` is the annotation §2 asks for.
    fn infer_lambda(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: Plicity,
        name: &Name,
        domain: Option<&Raw>,
        body: &Raw,
    ) -> Result<Typed, ElabError> {
        let domain_value = match domain {
            Some(written) => {
                let (term, _) = self.check_type(scope, written)?;
                Arc::new(scope.eval(&mut self.meter, &term)?)
            }
            None => {
                // §2: a binder the checking type did not describe must be
                // annotated. No hole stands here, because nothing downstream
                // of an inferred λ ever determines one — the annotation is the
                // program saying what it means.
                return Err(Refusal::Uninferable { at: here }.into());
            }
        };
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::clone(&domain_value));
        let inferred = self.infer(&inner, body)?;
        // The body's type is read under the binder, so the Π that reports it has
        // to be built from a term rather than from the value directly.
        let codomain = inner.quote_type(&mut self.meter, &inferred.ty)?;
        let domain_term = scope.quote_type(&mut self.meter, &domain_value)?;
        let ty = scope.eval(
            &mut self.meter,
            &Term::function(here, plicity, Arc::clone(name), domain_term, codomain),
        )?;
        Ok(Typed {
            term: Term::lam(here, Arc::clone(name), inferred.term),
            ty,
        })
    }


    /// §1.3's arity law: a `Call` supplies every declared parameter, and this
    /// is where the count is checked.
    ///
    /// **Measured on the function, not on the result.** §1.3's word is
    /// "declared", and [`declared_parameters`] is what reads it: the named
    /// explicit binders of `f`'s own type, taken before an argument has solved
    /// anything. Counting what stands after the last argument instead would
    /// count a *result* that happens to be a function, which is what a generic
    /// answer becomes the moment a caller instantiates it.
    ///
    /// Over-application is left to the walk. §1.3 names one direction, and
    /// "supplies every declared parameter" is silent about an argument list
    /// that runs past them into a result that takes more — which either
    /// applies, or earns [`Refusal::NotAFunction`] from the argument that
    /// could not.
    fn complete_call(
        &mut self,
        scope: &Scope,
        here: Origin,
        function: &Raw,
        arguments: &[Raw],
    ) -> Result<Typed, ElabError> {
        let head = self.infer(scope, function)?;
        let stated = scope.quote_type(&mut self.meter, &head.ty)?;
        let declared = declared_parameters(&head.term, &stated);
        if let Some(missing) = declared.get(arguments.len()..).filter(|rest| !rest.is_empty()) {
            return Err(Refusal::Underapplied {
                at: here,
                function: crate::show::head_spelled(&head.term),
                wanted: declared.len(),
                written: arguments.len(),
                missing: missing.to_vec(),
            }
            .into());
        }
        self.apply_spine(scope, here, head, &arguments.iter().collect::<Vec<_>>(), None)
    }

    /// `f a ⇒ B[a]`.
    fn application(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: &Plicity,
        function: &Raw,
        argument: &Raw,
    ) -> Result<Typed, ElabError> {
        if *plicity != Plicity::Explicit {
            return Err(Refusal::PlicityMismatch { at: argument.origin() }.into());
        }
        let inferred = self.infer(scope, function)?;
        self.apply_spine(scope, here, inferred, &[argument], None)
    }

    /// `x.m` — `10-traits.md` §6's method syntax, resolved by exact receiver.
    ///
    /// Three steps and no search. The receiver is inferred, the head of its
    /// type is read, and the traits that declare a method spelled `m` are
    /// intersected with the ones that have a dictionary at that head. Exactly
    /// one survivor is the call; none and two are the two refusals §6 names.
    ///
    /// What makes this a lookup rather than a search is that both operands are
    /// tables: [`Classes::declaring_method`](crate::class::Classes::declaring_method)
    /// is an index read and the dictionary test is the same keyed read §4 step 2
    /// already was. Nothing is tried and undone, so nothing can be tried in a
    /// different order and answer differently.
    ///
    /// The survivor is then elaborated as if the author had written
    /// `Class.m(x)`: the same [`method_at`](crate::dictionary::method_at) a
    /// qualified name goes through, applied to the receiver already in hand.
    /// That is what makes "a method is a spelling" true of the elaboration and
    /// not only of the prose — `x.m(y)` and `Class.m(x, y)` are one term.
    fn method(&mut self, scope: &Scope, here: Origin, receiver: &Raw, method: &Name) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, receiver)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let receiver_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let stated = scope.quote_type(&mut self.meter, receiver_ty)?;
        // A local head is §6's generic parameter and `None` is a type with no
        // name at its head. Neither is a key, and the message is the same
        // because the repair is: write the trait.
        let Some(Head::Rigid(head)) = head_of(&stated, scope.depth()) else {
            return Err(Refusal::MethodOnVariable {
                at: here,
                method: Arc::clone(method),
            }
            .into());
        };
        let classes = scope.cx().classes().clone();
        let candidates: Vec<Name> = classes
            .declaring_method(method)
            .iter()
            .filter(|class| {
                let key = Key::rigid(class, &head);
                scope.discharged(&key).is_some() || classes.instance(&key).is_some()
            })
            .map(Arc::clone)
            .collect();
        let [class] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                Refusal::NoMethodForType {
                    at: here,
                    head,
                    method: Arc::clone(method),
                }
            } else {
                Refusal::AmbiguousMethod {
                    at: here,
                    head,
                    method: Arc::clone(method),
                    classes: candidates,
                }
            }
            .into());
        };
        let qualified: Name = Arc::from(format!("{class}.{method}"));
        let Some((term, ty)) = crate::dictionary::method_at(self, scope, here, &qualified)? else {
            return Err(Refusal::NoMethodForType {
                at: here,
                head,
                method: Arc::clone(method),
            }
            .into());
        };
        self.receiving(scope, here, Typed { term, ty }, inferred)
    }

    /// A method applied to the receiver it was found for.
    ///
    /// The receiver is elaborated already, so this is [`Self::application`]
    /// with its argument arriving as a term rather than as syntax — and it is a
    /// separate function rather than a parameter on that one because the two
    /// differ in what they do with the domain: there the argument is *checked*
    /// against it, here the two types are unified, which is what solves the
    /// trait arguments `method_at` left as metavariables.
    fn receiving(&mut self, scope: &Scope, here: Origin, function: Typed, receiver: Typed) -> Result<Typed, ElabError> {
        let mut walk = Walk::default();
        let mut ty = function.ty.clone();
        self.advance(scope, &mut ty, &mut walk)?;
        let unfolded = opened(&mut self.meter, &ty)?;
        let function_ty = unfolded.as_ref().unwrap_or(&ty);
        let Form::Pi { domain, codomain, .. } = &function_ty.form else {
            return Err(Refusal::NotAFunction {
                at: here,
                ty: scope.quote_type(&mut self.meter, &ty)?,
            }
            .into());
        };
        let (domain, codomain) = (Arc::clone(domain), codomain.clone());
        // Matching mode: the receiver is where the method's parameters are
        // learned — `xs.fold_from_end` reads `A` off `xs`.
        self.unifier
            .unify_types(&mut self.meter, scope.depth(), here, &domain, &receiver.ty)?;
        walk.slots.push(Slot::Argument(receiver.term.clone()));
        let value = scope.eval(&mut self.meter, &receiver.term)?;
        let ty = apply_closure(&mut self.meter, &codomain, value)?;
        self.finish_walk(scope, here, function.term, ty, walk)
    }

    /// `{ f : A, … } ⇒ Type (max …)`.
    fn record_type(&mut self, scope: &Scope, here: Origin, fields: &[RawField]) -> Result<Typed, ElabError> {
        let mut level = Level::ZERO;
        let mut inner = scope.clone();
        let mut elaborated = Vec::with_capacity(fields.len());
        for (position, field) in fields.iter().enumerate() {
            if let Some(previous) = fields.iter().take(position).find(|earlier| earlier.name == field.name) {
                return Err(Refusal::DuplicateField {
                    at: field.term.origin(),
                    previous: previous.term.origin(),
                    field: Arc::clone(&field.name),
                }
                .into());
            }
            let (term, field_level) = self.check_type(&inner, &field.term)?;
            level = level.max(field_level);
            let value = inner.eval(&mut self.meter, &term)?;
            inner = inner.assume(Some(Arc::clone(&field.name)), field.term.origin(), Arc::new(value));
            elaborated.push(Field {
                name: Arc::clone(&field.name),
                term,
            });
        }
        Ok(Typed {
            term: Term::new(here, Shape::RecordType(elaborated.into())),
            ty: Value::new(here, Form::Universe(level)),
        })
    }

    /// `e.f ⇒ A[e]`.
    fn projection(&mut self, scope: &Scope, here: Origin, record: &Raw, field: &Name) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, record)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let record_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: here,
                ty: scope.quote_type(&mut self.meter, &inferred.ty)?,
            }
            .into());
        };
        let telescope = telescope.clone();
        if !telescope.fields.iter().any(|declared| &declared.name == field) {
            return Err(Refusal::NoSuchField {
                at: here,
                field: Arc::clone(field),
            }
            .into());
        }
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        Ok(Typed {
            ty: field_type(&mut self.meter, &telescope, &subject, field)?,
            term: Term::project(here, inferred.term, Arc::clone(field)),
        })
    }

    /// `e with { p⃗ = v, … } ⇒ A`, where `A` is `e`'s own type.
    ///
    /// The update is a rebuild, not a mutation: every field the paths do not
    /// name is carried over by projection, and every field they do is checked at
    /// the type the telescope gives it *after* the fields before it have been
    /// replaced. That is what makes an incoherent update — changing `n` in
    /// `{ n : Nat, xs : Vec A n }` and keeping `xs` — an ordinary type error
    /// rather than a rule this function has to state.
    fn update(&mut self, scope: &Scope, here: Origin, record: &Raw, updates: &[RawUpdate]) -> Result<Typed, ElabError> {
        overlapping(updates)?;
        let inferred = self.infer(scope, record)?;
        let unfolded = opened(&mut self.meter, &inferred.ty)?;
        let record_ty = Value::clone(unfolded.as_ref().unwrap_or(&inferred.ty));
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: record.origin(),
                ty: scope.quote_type(&mut self.meter, &inferred.ty)?,
            }
            .into());
        };
        let telescope = telescope.clone();
        let ty_term = scope.quote_type(&mut self.meter, &record_ty)?;
        let subject = scope.eval(&mut self.meter, &inferred.term)?;
        // §9.1's "one `let` per segment". Without it the subject is written once
        // per field it carries over, and a record of eight fields updated at one
        // of them would evaluate the thing being updated eight times.
        let name: Name = Arc::from("with");
        let inner = scope.define(
            &mut self.meter,
            Arc::clone(&name),
            Arc::new(record_ty.clone()),
            subject.clone(),
        )?;
        let replacements: Vec<Replacement<'_>> = updates.iter().map(Replacement::of).collect();
        let literal = self.rebuilt(
            &inner,
            here,
            &Term::var(here, Index(0)),
            &subject,
            &telescope,
            &replacements,
        )?;
        Ok(Typed {
            term: Term::bind(here, name, ty_term, inferred.term, literal),
            ty: record_ty,
        })
    }

    /// The literal one segment of an update rebuilds.
    ///
    /// `subject` denotes the record being rebuilt *in `scope`*, and `value` is
    /// what it evaluates to. They are separate arguments because the term is a
    /// variable the caller just bound and the value is the record it was bound
    /// to: the type of a field carried over is read off the second, and the term
    /// that carries it over is a projection of the first.
    fn rebuilt(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        value: &Value,
        telescope: &Telescope,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        for update in updates {
            let Some(head) = update.path.first() else { continue };
            if !telescope.fields.iter().any(|declared| declared.name == *head) {
                return Err(Refusal::NoSuchField {
                    at: update.origin,
                    field: Arc::clone(head),
                }
                .into());
            }
        }
        let mut env = telescope.env.clone();
        let mut fields = Vec::with_capacity(telescope.fields.len());
        for declared in telescope.fields.iter() {
            let expected = eval(&mut self.meter, &env, &declared.term)?;
            let mine: Vec<Replacement<'_>> = updates
                .iter()
                .filter(|update| update.path.first() == Some(&declared.name))
                .filter_map(Replacement::rest)
                .collect();
            let term = match mine.split_first() {
                // Carried over. Its type is read at the *old* record and the
                // literal wants it at the new one, so the two are unified rather
                // than assumed equal — that is where a dependent field whose
                // type an earlier replacement invalidated reports.
                None => {
                    let term = Term::project(here, subject.clone(), Arc::clone(&declared.name));
                    let found = field_type(&mut self.meter, telescope, value, &declared.name)?;
                    self.unifier
                        .unify_types(&mut self.meter, scope.depth(), here, &expected, &found)?;
                    term
                }
                // The path ends here: the new value is checked at the field's
                // type, like any other field of any other literal.
                Some((update, rest)) if update.path.is_empty() && rest.is_empty() => {
                    self.check(scope, update.value, &expected)?
                }
                Some(_) => self.deeper(scope, here, subject, &declared.name, &expected, &mine)?,
            };
            env = env.push(scope.eval(&mut self.meter, &term)?);
            fields.push(Field {
                name: Arc::clone(&declared.name),
                term,
            });
        }
        Ok(Term::new(here, Shape::Record(fields.into())))
    }

    /// One field of an update whose paths reach through it.
    fn deeper(
        &mut self,
        scope: &Scope,
        here: Origin,
        subject: &Term,
        field: &Name,
        expected: &Value,
        updates: &[Replacement<'_>],
    ) -> Result<Term, ElabError> {
        let unfolded = opened(&mut self.meter, expected)?;
        let field_ty = Value::clone(unfolded.as_ref().unwrap_or(expected));
        let Form::RecordType(inner) = &field_ty.form else {
            let at = updates.first().map_or(here, |update| update.origin);
            return Err(Refusal::NotARecord {
                at,
                ty: scope.quote_type(&mut self.meter, expected)?,
            }
            .into());
        };
        let inner = inner.clone();
        let projected = Term::project(here, subject.clone(), Arc::clone(field));
        let projected_value = scope.eval(&mut self.meter, &projected)?;
        let ty_term = scope.quote_type(&mut self.meter, &field_ty)?;
        let under = scope.define(
            &mut self.meter,
            Arc::clone(field),
            Arc::new(field_ty),
            projected_value.clone(),
        )?;
        let body = self.rebuilt(
            &under,
            here,
            &Term::var(here, Index(0)),
            &projected_value,
            &inner,
            updates,
        )?;
        Ok(Term::bind(here, Arc::clone(field), ty_term, projected, body))
    }
    // ---- shared premises ---------------------------------------------------

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

    /// Elaborate a `let`'s definition and answer the scope its body is read in.
    fn definition(
        &mut self,
        scope: &Scope,
        name: &Name,
        written: Option<&Raw>,
        value: &Raw,
    ) -> Result<Bound, ElabError> {
        let (ty_term, ty_value, value_term) = match written {
            Some(written) => {
                let (ty_term, _) = self.check_type(scope, written)?;
                let ty_value = scope.eval(&mut self.meter, &ty_term)?;
                let value_term = self.check(scope, value, &ty_value)?;
                (ty_term, ty_value, value_term)
            }
            None => {
                // No insertion: the binder keeps the inferred Π as it stands,
                // and a use of the name instantiates it through the
                // application walk — §2.1's "quantified at the declaration,
                // solved at the use".
                let inferred = self.infer(scope, value)?;
                let ty_term = scope.quote_type(&mut self.meter, &inferred.ty)?;
                (ty_term, inferred.ty, inferred.term)
            }
        };
        let bound = scope.eval(&mut self.meter, &value_term)?;
        Ok(Bound {
            scope: scope.define(&mut self.meter, Arc::clone(name), Arc::new(ty_value), bound)?,
            ty_term,
            value_term,
        })
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

/// A `let`'s definition, and the scope its body is read in.
struct Bound {
    scope: Scope,
    ty_term: Term,
    value_term: Term,
}

/// One replacement of an update, as the rebuild of one segment sees it.
///
/// The path shortens by a segment at each level, which is why this borrows the
/// [`RawUpdate`]'s rather than owning a copy: the recursion is over suffixes of
/// one path and nothing needs a second.
#[derive(Clone, Copy)]
struct Replacement<'a> {
    origin: Origin,
    path: &'a [Name],
    value: &'a Raw,
}

impl<'a> Replacement<'a> {
    /// The whole replacement, at the outermost record.
    fn of(update: &'a RawUpdate) -> Self {
        Self {
            origin: update.origin,
            path: &update.path,
            value: &update.value,
        }
    }

    /// The same replacement, one segment further in.
    fn rest(&self) -> Option<Self> {
        Some(Self {
            path: self.path.split_first()?.1,
            ..*self
        })
    }
}

/// Refuse an update where one path is a prefix of another.
fn overlapping(updates: &[RawUpdate]) -> Result<(), ElabError> {
    for (position, update) in updates.iter().enumerate() {
        for earlier in updates.iter().take(position) {
            let shorter = earlier.path.len().min(update.path.len());
            if earlier.path.iter().take(shorter).eq(update.path.iter().take(shorter)) {
                return Err(Refusal::OverlappingUpdate {
                    at: update.origin,
                    previous: earlier.origin,
                }
                .into());
            }
        }
    }
    Ok(())
}

/// The parameters a call to `head`, whose type is `ty`, must supply — in the
/// order they were declared.
///
/// Explicit binders only, and the implicit and constraint ones are skipped
/// rather than collected: neither is written at a call site, so neither is a
/// parameter an argument list can be measured against. The walk stops at the
/// first form that is not a Π, so a generic answer `A` contributes nothing: it
/// is a parameter's worth of nothing until a caller instantiates it, and a
/// caller that instantiates it to a function did not thereby leave an argument
/// out.
///
/// # Why the head decides how the type is read
///
/// §1.3's word is "declared", and there are two kinds of declaration.
///
/// A *source* definition writes a parameter list, and the names in it survive
/// into the binders — so a named binder is a declared parameter and
/// [`ARROW_BINDER`](crate::raw::ARROW_BINDER) is the mark of one that is not.
/// The distinction is load-bearing rather than cosmetic: `walked(items:
/// StaffItem) -> (Position<τ> → Result<…>)` declares one parameter and returns
/// a function, a two-parameter definition has the same type, and only the
/// names tell them apart. Without this the corpus would be refused for writing
/// down the functions it returns.
///
/// A *registered* signature has no such second reading. There is no surface
/// declaration beside it to disagree with, so the arrow it was built from **is**
/// its parameter list, names or none — which is `base.rs`'s law again, the
/// shape of the table being the host's to state. That is why `transpose(P8)` is
/// under-applied even though the host wrote its binders anonymously.
fn declared_parameters(head: &Term, ty: &Term) -> Vec<Name> {
    let registered = matches!(head.shape(), Shape::Builtin(_));
    let mut declared = Vec::new();
    let mut rest = ty;
    while let Shape::Pi {
        plicity,
        name,
        codomain,
        ..
    } = rest.shape()
    {
        if *plicity == Plicity::Explicit {
            if !registered && &**name == crate::raw::ARROW_BINDER {
                break;
            }
            declared.push(Arc::clone(name));
        }
        rest = codomain;
    }
    declared
}

/// A raw term as a head and the arguments the author wrote after it.
///
/// [`None`] where any of them is implicit: an author supplying an implicit
/// argument is telling the elaborator which binder they mean, and
/// [`Elaborator::constructed`] would be filling a different one.
///
/// Both application forms, because the question here is what the *author*
/// wrote and [`RawShape::Call`] is the form they wrote it in: `Succ(fewer)`
/// reaches [`Elaborator::constructed`] as one node with its arguments beside
/// it, and a reader that only knew the iterated [`RawShape::App`] would see a
/// head it could not name and leave the constructor to infer — which is
/// [`Refusal::BareConstructor`](crate::Refusal::BareConstructor), for a term
/// whose family the expected type was holding all along.
fn written_spine(raw: &Raw) -> Option<(&Raw, Vec<&Raw>)> {
    let mut arguments = Vec::new();
    let mut head = raw;
    loop {
        if let RawShape::App {
            plicity,
            function,
            argument,
        } = head.shape()
        {
            if *plicity != Plicity::Explicit {
                return None;
            }
            arguments.push(argument);
            head = function;
            continue;
        }
        // Pushed in reverse because the whole vector is reversed below, which
        // is what lets the two forms nest in either order.
        if let RawShape::Call {
            function,
            arguments: written,
        } = head.shape()
        {
            arguments.extend(written.iter().rev());
            head = function;
            continue;
        }
        break;
    }
    arguments.reverse();
    Some((head, arguments))
}

/// How many fields the constructor `name` names takes, and how many parameters
/// its family has — or [`None`] for a name that is not a declared case.
///
/// The written name's half of [`Elaborator::case_of`], read out so that
/// [`Elaborator::constructed_open`] asks the same question in the direction that
/// has no expected type to ask it of.
fn written_case(scope: &Scope, name: &Name) -> Option<(usize, u32)> {
    let crate::family::Found::Rigid(constant) = scope.declared(name)? else {
        return None;
    };
    let crate::family::Role::Constructor(which) = constant.role else {
        return None;
    };
    let fields = constant
        .group
        .family_at(constant.family)
        .and_then(|declared| declared.constructor_at(which))
        .map(|constructor| constructor.fields.len())?;
    Some((fields, constant.group.params()))
}

/// The case of `declared` that a written name denotes, and how many fields it
/// takes — the count being what tells a missing parameter from a missing field.
///
/// Two spellings, and the difference between them is who wrote the family's
/// name. `Option.Some` says it, and the only question is whether the family is
/// the one expected. `Some` does not, and then §1.3's rule applies: the word is
/// read in the expected type's namespace, but only after a binder and a
/// declaration have both declined it, so nothing an author named themselves can
/// be taken for a constructor.
///
/// # The family answers to its own name, and that is not a declaration declining
///
/// `01-surface.md` §1.3 blesses `enum Beats { Beats(Nat) }` in as many words, so
/// a word that names a family and one of its cases is an ordinary program and
/// not a collision. Counting the family as "a declaration answered" would make
/// every such case unwritable in checking position, which is the position §1.3
/// says a bare constructor is *for*. The exception is exactly as wide as the
/// coincidence: the case wins only over the family the expected type already
/// names, and reading the word as that family is never what this position
/// wanted, because a family is a type and an element of one is what is being
/// checked. Any other declaration — a `let`, a function, a different family —
/// still declines, and a binder still declines first.
fn case_named(scope: &Scope, name: &Name, declared: &crate::family::Declared) -> Option<(Name, usize)> {
    let case = match name.split_once('.') {
        Some((family, case)) if family == &*declared.name => case,
        Some(_) => return None,
        None => {
            if scope.lookup(name).is_some() || answered_apart_from(scope, name, declared) {
                return None;
            }
            name
        }
    };
    declared
        .constructors
        .iter()
        .find(|constructor| *constructor.name == *case)
        .map(|constructor| (Arc::clone(&constructor.name), constructor.fields.len()))
}

/// Whether a declaration other than `declared` itself answers to `name`.
///
/// [`case_named`]'s "a declaration declined it" test, with the one coincidence
/// §1.3 allows taken out — see the note there.
fn answered_apart_from(scope: &Scope, name: &Name, declared: &crate::family::Declared) -> bool {
    match scope.declared(name) {
        Some(crate::family::Found::Rigid(constant)) => !(constant.is_family() && *constant.name() == *declared.name),
        Some(crate::family::Found::Recursor(..)) => true,
        None => false,
    }
}
