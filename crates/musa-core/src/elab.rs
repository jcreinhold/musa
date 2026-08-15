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
use crate::context::Cx;
use crate::error::{CoreError, Malformed};
use crate::eval::{apply, apply_closure, eval, field_type, force};
use crate::level::{Level, LevelMeta};
use crate::meta::{Meta, MetaSource};
use crate::origin::Origin;
use crate::quote::{Depth, quote};
use crate::raw::{Raw, RawField, RawShape, RawUpdate};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{DbLevel, Field, Index, Name, Plicity, Shape, Term};
use crate::unify::Unifier;
use crate::value::{Closure, Env, Form, Telescope, Value};

/// A term and the type it was elaborated at.
///
/// The type is a [`Value`] rather than a [`Term`] because every rule that reads
/// it matches on its shape, and re-normalizing at each node is the cost the
/// semantic domain exists to avoid.
struct Typed {
    term: Term,
    ty: Value,
}

/// One elaboration.
///
/// Scoped to a declaration rather than to a session, because §2.1 reports a
/// metavariable unsolved "at the end of the declaration it was created in" —
/// a longer-lived elaborator would have no such end to report at.
pub(crate) struct Elaborator {
    meter: Meter,
    unifier: Unifier,
    /// Every metavariable created here, in creation order.
    ///
    /// Kept so that the unsolved report names the *first* hole the author left
    /// rather than whichever one a walk of the output happened to reach — the
    /// earliest is the one their next edit is about.
    metas: Vec<Meta>,
    /// Every *level* metavariable, with the term that created it.
    ///
    /// A separate list because a level is a separate sort: its solution is a
    /// level and not a value, so it cannot live in [`Meta`]. What it shares is
    /// the report, which is why the origin travels with it.
    levels: Vec<(LevelMeta, Origin)>,
}

impl Elaborator {
    pub(crate) fn new(cx: &Cx) -> Self {
        Self {
            meter: cx.meter(),
            unifier: Unifier::default(),
            metas: Vec::new(),
            levels: Vec::new(),
        }
    }

    /// Elaborate `raw` against the type `ty`, and finish.
    pub(crate) fn run_check(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
        let term = self.check(scope, raw, ty)?;
        self.settled()?;
        Ok(zonk(&mut self.meter, scope.depth(), &term)?)
    }

    /// Elaborate `raw`, answering it and its type, and finish.
    pub(crate) fn run_infer(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Term), ElabError> {
        let inferred = self.infer(scope, raw)?;
        let inferred = self.inserted(scope, inferred)?;
        let ty = scope.quote_type(&mut self.meter, &inferred.ty)?;
        self.settled()?;
        let depth = scope.depth();
        Ok((
            zonk(&mut self.meter, depth, &inferred.term)?,
            zonk(&mut self.meter, depth, &ty)?,
        ))
    }

    /// Refuse if any metavariable — of either sort — is still undetermined.
    pub(crate) fn settled(&self) -> Result<(), ElabError> {
        if let Some(unsolved) = self.metas.iter().find(|meta| !meta.is_solved()) {
            return Err(Refusal::Unsolved {
                site: unsolved.source(),
                created: unsolved.origin(),
                blocked: self.unifier.blocked(),
            }
            .into());
        }
        // Levels are checked after terms rather than before, because a level is
        // usually determined *by* a term constraint: reporting the level first
        // would name a consequence where the cause is a hole the author can see.
        let Some((_, created)) = self.levels.iter().find(|(level, _)| !level.is_solved()) else {
            return Ok(());
        };
        Err(Refusal::Unsolved {
            site: MetaSource::UniverseLevel,
            created: *created,
            blocked: self.unifier.blocked(),
        }
        .into())
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

    /// Elaborate `raw` against `ty` **without finishing**.
    ///
    /// [`Self::run_check`] is one whole judgment: it checks that nothing is left
    /// unsolved and zonks. A declaration is many judgments that share one set of
    /// metavariables — a constructor's chosen index may be what determines a
    /// parameter's level — so it checks each part with this and finishes once.
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
        let inferred = self.inserted(scope, inferred)?;
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
            return Err(self.unresolved(scope, here, name));
        };
        let level = if found.is_recursor() {
            self.fresh_level(here)?
        } else {
            Level::ZERO
        };
        let constant = found.at(level);
        let ty = constant.ty(&mut self.meter)?;
        Ok(Typed {
            term: constant.term(here),
            ty,
        })
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
        }
        .into()
    }

    /// A universe at a level nobody wrote — §2.1's third creation site.
    pub(crate) fn fresh_level(&mut self, origin: Origin) -> Result<Level, ElabError> {
        self.meter.metavariable("elaboration")?;
        let meta = LevelMeta::new(u32::try_from(self.levels.len()).unwrap_or(u32::MAX));
        self.levels.push((meta.clone(), origin));
        Ok(Level::variable(meta))
    }

    // ---- checking ----------------------------------------------------------

    /// `Γ ⊢ raw ⇐ ty ⇝ t`.
    fn check(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
        let unfolded = force(&mut self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        match self.checked(scope, raw, ty)? {
            Some(term) => Ok(term),
            // §2's `Switch`, and the only rule in this module that calls
            // conversion.
            None => {
                let inferred = self.infer(scope, raw)?;
                let inferred = self.inserted(scope, inferred)?;
                self.unifier
                    .unify_types(&mut self.meter, scope.depth(), raw.origin(), ty, &inferred.ty)?;
                Ok(inferred.term)
            }
        }
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
            } => self.lambda(scope, raw, *plicity, name, domain.as_ref(), body, ty),
            RawShape::Record(fields) => {
                let Form::RecordType(telescope) = &ty.form else {
                    return self.abstracted(scope, raw, ty);
                };
                let telescope = telescope.clone();
                self.literal(scope, here, fields, &telescope).map(Some)
            }
            RawShape::Refl(witness) => {
                let Form::Id { ty: at, left, right } = &ty.form else {
                    return self.abstracted(scope, raw, ty);
                };
                let (at, left, right) = (Value::clone(at), Value::clone(left), Value::clone(right));
                self.reflexivity(scope, here, witness, &at, &left, &right).map(Some)
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
            // §1.3's bare constructor. Only in this direction, and that is the
            // rule rather than a fallback: here the expected type names the
            // family whose namespace the word is read in, and where it does not
            // there is nothing to read it in.
            RawShape::Var(name) => match self.bare(scope, here, name, ty)? {
                Some(term) => Ok(Some(term)),
                None => self.abstracted(scope, raw, ty),
            },
            RawShape::Universe(_)
            | RawShape::Pi { .. }
            | RawShape::App { .. }
            | RawShape::RecordType(_)
            | RawShape::Project { .. }
            | RawShape::Update { .. }
            | RawShape::Id { .. }
            | RawShape::J { .. }
            | RawShape::Annot { .. } => self.abstracted(scope, raw, ty),
        }
    }

    /// `Untied ⇐ Tying`, when nothing nearer already means `Untied`.
    ///
    /// Answers `None` when the name is bound, is declared in its own right, is
    /// already qualified, or when the expected type is not a family with a case
    /// of that name — every one of which is a term with an ordinary rule, and
    /// none of which this may take over.
    fn bare(&mut self, scope: &Scope, here: Origin, name: &Name, ty: &Value) -> Result<Option<Term>, ElabError> {
        if name.contains('.') || scope.lookup(name).is_some() || scope.declared(name).is_some() {
            return Ok(None);
        }
        let Some(element) = crate::family::element(&mut self.meter, ty)? else {
            return Ok(None);
        };
        let Some(declared) = element.group.family_at(element.family) else {
            return Ok(None);
        };
        if !declared
            .constructors
            .iter()
            .any(|constructor| *constructor.name == **name)
        {
            return Ok(None);
        }
        let qualified = Raw::var(here, format!("{}.{name}", declared.name));
        self.check(scope, &qualified, ty).map(Some)
    }

    /// Wrap `raw` in an implicit λ when the type it is checked against wants
    /// one, or answer `None` so that `Switch` runs.
    ///
    /// The second half of implicit insertion, and the reason `Switch` never
    /// meets an implicit Π: a term whose own form does not abstract the binder
    /// has one abstracted for it here.
    fn abstracted(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Term>, ElabError> {
        let Form::Pi {
            plicity: Plicity::Implicit,
            name,
            domain,
            codomain,
        } = &ty.form
        else {
            return Ok(None);
        };
        let (name, domain, codomain) = (Arc::clone(name), Arc::clone(domain), codomain.clone());
        let here = raw.origin();
        let variable = scope.fresh_var(here, Arc::clone(&domain));
        let body_ty = apply_closure(&mut self.meter, &codomain, variable)?;
        let inner = scope.assume(Some(Arc::clone(&name)), here, domain);
        Ok(Some(Term::lam(here, name, self.check(&inner, raw, &body_ty)?)))
    }

    /// `λx. e ⇐ (x : A) → B`, and the plicity rules that go with it.
    fn lambda(
        &mut self,
        scope: &Scope,
        raw: &Raw,
        plicity: Plicity,
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
        if plicity != *expected {
            // An implicit λ at an explicit binder is a mistake rather than a
            // term to abstract around: the author wrote the binder, at the
            // plicity the type does not have.
            if plicity == Plicity::Implicit {
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

    /// `refl e ⇐ Id A x y`, which holds when `e`, `x`, and `y` are all the same.
    fn reflexivity(
        &mut self,
        scope: &Scope,
        here: Origin,
        witness: &Raw,
        at: &Value,
        left: &Value,
        right: &Value,
    ) -> Result<Term, ElabError> {
        let term = self.check(scope, witness, at)?;
        let value = scope.eval(&mut self.meter, &term)?;
        for endpoint in [left, right] {
            self.unifier
                .unify(&mut self.meter, scope.depth(), witness.origin(), at, &value, endpoint)?;
        }
        Ok(Term::refl(here, term))
    }

    // ---- inference ---------------------------------------------------------

    /// `Γ ⊢ raw ⇒ ty ⇝ t`.
    fn infer(&mut self, scope: &Scope, raw: &Raw) -> Result<Typed, ElabError> {
        let here = raw.origin();
        match raw.shape() {
            RawShape::Var(name) => {
                let Some(found) = scope.lookup(name) else {
                    return self.constant(scope, here, name);
                };
                Ok(Typed {
                    term: Term::var(here, found.index),
                    ty: Value::clone(&found.ty),
                })
            }
            RawShape::Universe(written) => {
                let level = match written {
                    Some(level) => level.clone(),
                    None => self.fresh_level(here)?,
                };
                Ok(Typed {
                    ty: Value::new(here, Form::Universe(level.succ())),
                    term: Term::universe(here, level),
                })
            }
            RawShape::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => self.function_type(scope, here, *plicity, name, domain, codomain),
            RawShape::Lam {
                plicity,
                name,
                domain,
                body,
            } => self.infer_lambda(scope, here, *plicity, name, domain.as_ref(), body),
            RawShape::App {
                plicity,
                function,
                argument,
            } => self.application(scope, here, *plicity, function, argument),
            RawShape::RecordType(fields) => self.record_type(scope, here, fields),
            // §2: a record literal is an introduction form, so it checks. The
            // type it "obviously" has is a guess rather than a principal type —
            // `{ ty = {}, val = {} }` inhabits both `{ ty : Type 0, val : ty }`
            // and `{ ty : Type 0, val : {} }` — and picking one would be the
            // unifier's forbidden habit of trying the solution that comes to
            // hand. So there is no inference rule, and an author who wants to
            // project out of a literal writes the type it should have.
            RawShape::Record(_) => Err(Refusal::Uninferable { at: here }.into()),
            RawShape::Project { record, field } => self.projection(scope, here, record, field),
            RawShape::Update { record, updates } => self.update(scope, here, record, updates),
            RawShape::Id { ty, left, right } => self.identity(scope, here, ty, left, right),
            RawShape::Refl(witness) => {
                let inferred = self.infer(scope, witness)?;
                let inferred = self.inserted(scope, inferred)?;
                let value = scope.eval(&mut self.meter, &inferred.term)?;
                Ok(Typed {
                    term: Term::refl(here, inferred.term),
                    ty: Value::new(
                        here,
                        Form::Id {
                            ty: Arc::new(inferred.ty),
                            left: Arc::new(value.clone()),
                            right: Arc::new(value),
                        },
                    ),
                })
            }
            RawShape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => self.elimination(scope, here, [ty, from, motive, base, to, proof]),
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

    /// Fill every leading implicit binder of an inferred type with a
    /// metavariable.
    fn inserted(&mut self, scope: &Scope, mut inferred: Typed) -> Result<Typed, ElabError> {
        loop {
            let unfolded = force(&mut self.meter, &inferred.ty)?;
            let ty = unfolded.as_ref().unwrap_or(&inferred.ty);
            let Form::Pi {
                plicity: Plicity::Implicit,
                name: _,
                domain,
                codomain,
            } = &ty.form
            else {
                return Ok(inferred);
            };
            let (domain, codomain) = (Arc::clone(domain), codomain.clone());
            let here = inferred.term.origin();
            let argument = self.fresh_meta(scope, here, MetaSource::ImplicitArgument, &domain)?;
            let value = scope.eval(&mut self.meter, &argument)?;
            inferred = Typed {
                term: Term::app(here, inferred.term, argument),
                ty: apply_closure(&mut self.meter, &codomain, value)?,
            };
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
            ty: Value::new(here, Form::Universe(domain_level.max(&codomain_level))),
        })
    }

    /// `λx. e ⇒ (x : ?A) → B`, where `?A` is §2.1's second metavariable site.
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
                // §2.1's third site is "a level position *the surface did not
                // write*", and this is not one: no universe stands here at all,
                // only the sort a hole's type has to have. A metavariable would
                // be an unknown no program could determine — the elaborator
                // never compares a hole's type against anything — so it would
                // refuse every unannotated binder rather than describe one.
                //
                // The choice is unobservable. A core λ records no domain (§1),
                // and this level is read only by [`zonk`], which quotes the
                // solution *as a type* and never looks at which universe it was
                // told. What it costs is nothing: a binder whose type genuinely
                // lives higher is one the author annotates, which §2 asks for at
                // every signature anyway.
                let universe = Value::new(here, Form::Universe(Level::ZERO));
                let hole = self.fresh_meta(scope, here, MetaSource::BinderType, &universe)?;
                Arc::new(scope.eval(&mut self.meter, &hole)?)
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

    /// `f a ⇒ B[a]`.
    fn application(
        &mut self,
        scope: &Scope,
        here: Origin,
        plicity: Plicity,
        function: &Raw,
        argument: &Raw,
    ) -> Result<Typed, ElabError> {
        let inferred = self.infer(scope, function)?;
        // An argument the author wrote implicitly is the one the binder wanted,
        // so insertion is skipped: inserting first would fill that binder with a
        // metavariable and then refuse the argument as one too many.
        let inferred = match plicity {
            Plicity::Explicit => self.inserted(scope, inferred)?,
            Plicity::Implicit => inferred,
        };
        let unfolded = force(&mut self.meter, &inferred.ty)?;
        let function_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::Pi {
            plicity: expected,
            name: _,
            domain,
            codomain,
        } = &function_ty.form
        else {
            return Err(Refusal::NotAFunction {
                at: here,
                ty: scope.quote_type(&mut self.meter, function_ty)?,
            }
            .into());
        };
        if plicity != *expected {
            return Err(Refusal::PlicityMismatch { at: argument.origin() }.into());
        }
        let (domain, codomain) = (Arc::clone(domain), codomain.clone());
        let argument_term = self.check(scope, argument, &domain)?;
        let argument_value = scope.eval(&mut self.meter, &argument_term)?;
        Ok(Typed {
            term: Term::app(here, inferred.term, argument_term),
            ty: apply_closure(&mut self.meter, &codomain, argument_value)?,
        })
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
            level = level.max(&field_level);
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
        let inferred = self.inserted(scope, inferred)?;
        let unfolded = force(&mut self.meter, &inferred.ty)?;
        let record_ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: here,
                ty: scope.quote_type(&mut self.meter, record_ty)?,
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
        let inferred = self.inserted(scope, inferred)?;
        let unfolded = force(&mut self.meter, &inferred.ty)?;
        let record_ty = Value::clone(unfolded.as_ref().unwrap_or(&inferred.ty));
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at: record.origin(),
                ty: scope.quote_type(&mut self.meter, &record_ty)?,
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
        let inner = scope.define(Arc::clone(&name), Arc::new(record_ty.clone()), subject.clone());
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
        let unfolded = force(&mut self.meter, expected)?;
        let field_ty = Value::clone(unfolded.as_ref().unwrap_or(expected));
        let Form::RecordType(inner) = &field_ty.form else {
            let at = updates.first().map_or(here, |update| update.origin);
            return Err(Refusal::NotARecord {
                at,
                ty: scope.quote_type(&mut self.meter, &field_ty)?,
            }
            .into());
        };
        let inner = inner.clone();
        let projected = Term::project(here, subject.clone(), Arc::clone(field));
        let projected_value = scope.eval(&mut self.meter, &projected)?;
        let ty_term = scope.quote_type(&mut self.meter, &field_ty)?;
        let under = scope.define(Arc::clone(field), Arc::new(field_ty), projected_value.clone());
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

    /// `Id A x y ⇒ Type l`.
    fn identity(&mut self, scope: &Scope, here: Origin, ty: &Raw, left: &Raw, right: &Raw) -> Result<Typed, ElabError> {
        let (ty_term, level) = self.check_type(scope, ty)?;
        let ty_value = scope.eval(&mut self.meter, &ty_term)?;
        Ok(Typed {
            term: Term::identity(
                here,
                ty_term,
                self.check(scope, left, &ty_value)?,
                self.check(scope, right, &ty_value)?,
            ),
            ty: Value::new(here, Form::Universe(level)),
        })
    }

    /// `J A x P p y e ⇒ P y e`.
    ///
    /// The motive is *inferred* and then taken apart, rather than checked
    /// against a type built here. Building `(y : A) → Id A x y → Type l'` would
    /// need an `l'` nothing has written and this crate cannot yet infer (see the
    /// module note on levels); taking the inferred type apart needs no such
    /// number, and each premise it fails is a refusal that names where.
    fn elimination(&mut self, scope: &Scope, here: Origin, parts: [&Raw; 6]) -> Result<Typed, ElabError> {
        let [ty, from, motive, base, to, proof] = parts;
        let (ty_term, _) = self.check_type(scope, ty)?;
        let ty_value = scope.eval(&mut self.meter, &ty_term)?;

        let from_term = self.check(scope, from, &ty_value)?;
        let from_value = scope.eval(&mut self.meter, &from_term)?;

        let motive_inferred = self.infer(scope, motive)?;
        let motive_inferred = self.inserted(scope, motive_inferred)?;
        let motive_value = scope.eval(&mut self.meter, &motive_inferred.term)?;
        self.motive_shape(scope, motive.origin(), &motive_inferred.ty, &ty_value, &from_value)?;

        let refl_from = Value::new(from.origin(), Form::Refl(Arc::new(from_value.clone())));
        let at_from = apply(&mut self.meter, here, motive_value.clone(), from_value.clone())?;
        let base_ty = apply(&mut self.meter, here, at_from, refl_from)?;
        let base_term = self.check(scope, base, &base_ty)?;

        let to_term = self.check(scope, to, &ty_value)?;
        let to_value = scope.eval(&mut self.meter, &to_term)?;

        let proof_ty = Value::new(
            here,
            Form::Id {
                ty: Arc::new(ty_value),
                left: Arc::new(from_value),
                right: Arc::new(to_value.clone()),
            },
        );
        let proof_term = self.check(scope, proof, &proof_ty)?;
        let proof_value = scope.eval(&mut self.meter, &proof_term)?;

        let at_to = apply(&mut self.meter, here, motive_value, to_value)?;
        Ok(Typed {
            ty: apply(&mut self.meter, here, at_to, proof_value)?,
            term: Term::jay(
                here,
                ty_term,
                from_term,
                motive_inferred.term,
                base_term,
                to_term,
                proof_term,
            ),
        })
    }

    /// Require that `motive_ty` is `(y : at) → Id at from y → Type l'`.
    fn motive_shape(
        &mut self,
        scope: &Scope,
        at: Origin,
        motive_ty: &Value,
        subject: &Value,
        from: &Value,
    ) -> Result<(), ElabError> {
        let (endpoint_domain, endpoint_codomain) = self.function_parts(scope, at, motive_ty)?;
        self.unifier
            .unify_types(&mut self.meter, scope.depth(), at, subject, &endpoint_domain)?;
        let endpoint = scope.fresh_var(at, Arc::new(Value::clone(subject)));
        let under = scope.assume(None, at, Arc::new(Value::clone(subject)));
        let after_endpoint = apply_closure(&mut self.meter, &endpoint_codomain, endpoint.clone())?;
        let (proof_domain, proof_codomain) = self.function_parts(&under, at, &after_endpoint)?;
        let expected_proof = Value::new(
            at,
            Form::Id {
                ty: Arc::new(Value::clone(subject)),
                left: Arc::new(Value::clone(from)),
                right: Arc::new(endpoint),
            },
        );
        self.unifier
            .unify_types(&mut self.meter, under.depth(), at, &expected_proof, &proof_domain)?;
        let witness = under.fresh_var(at, Arc::new(proof_domain));
        let inside = under.assume(None, at, Arc::new(expected_proof));
        let result = apply_closure(&mut self.meter, &proof_codomain, witness)?;
        let unfolded = force(&mut self.meter, &result)?;
        let result = unfolded.as_ref().unwrap_or(&result);
        let Form::Universe(_) = result.form else {
            return Err(Refusal::NotAType {
                at,
                ty: inside.quote_type(&mut self.meter, result)?,
            }
            .into());
        };
        Ok(())
    }

    /// Take a Π type apart, refusing what is not one.
    fn function_parts(&mut self, scope: &Scope, at: Origin, ty: &Value) -> Result<(Value, Closure), ElabError> {
        let unfolded = force(&mut self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        let Form::Pi { domain, codomain, .. } = &ty.form else {
            return Err(Refusal::NotAFunction {
                at,
                ty: scope.quote_type(&mut self.meter, ty)?,
            }
            .into());
        };
        Ok((Value::clone(domain), codomain.clone()))
    }

    // ---- shared premises ---------------------------------------------------

    /// Elaborate a term standing in type position, answering its universe.
    pub(crate) fn check_type(&mut self, scope: &Scope, raw: &Raw) -> Result<(Term, Level), ElabError> {
        let inferred = self.infer(scope, raw)?;
        let inferred = self.inserted(scope, inferred)?;
        let unfolded = force(&mut self.meter, &inferred.ty)?;
        let ty = unfolded.as_ref().unwrap_or(&inferred.ty);
        let Form::Universe(level) = &ty.form else {
            return Err(Refusal::NotAType {
                at: raw.origin(),
                ty: scope.quote_type(&mut self.meter, ty)?,
            }
            .into());
        };
        Ok((inferred.term, level.resolved()))
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
                let inferred = self.infer(scope, value)?;
                let inferred = self.inserted(scope, inferred)?;
                let ty_term = scope.quote_type(&mut self.meter, &inferred.ty)?;
                (ty_term, inferred.ty, inferred.term)
            }
        };
        let bound = scope.eval(&mut self.meter, &value_term)?;
        Ok(Bound {
            scope: scope.define(Arc::clone(name), Arc::new(ty_value), bound),
            ty_term,
            value_term,
        })
    }

    /// A new metavariable, written applied to every binder in scope.
    fn fresh_meta(&mut self, scope: &Scope, origin: Origin, source: MetaSource, ty: &Value) -> Result<Term, ElabError> {
        self.meter.metavariable("elaboration")?;
        let stated = scope.quote_type(&mut self.meter, ty)?;
        // Closed, and applied to the identity spine of its context: §2.1's scope
        // condition then holds by construction rather than by a check.
        let closed = scope.close(&mut self.meter, origin, stated)?;
        let closed_ty = eval(&mut self.meter, &Env::EMPTY, &closed)?;
        let id = u32::try_from(self.metas.len()).unwrap_or(u32::MAX);
        let meta = Meta::new(id, origin, source, scope.arity(), closed_ty);
        self.metas.push(meta.clone());
        Ok(scope.spine(origin, Term::meta(origin, meta)))
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

/// Replace every metavariable in an elaborated term by what it stands for.
///
/// A metavariable is written applied to the identity spine of its creation
/// context, and this reads that whole application back at the type the
/// metavariable was created at. Doing it in the semantic domain rather than on
/// syntax is what keeps the crate's promise that reduction never happens on
/// terms: the β-redex a naive substitution would leave behind is contracted by
/// evaluation, and quotation writes the result at `depth` so the indices come
/// out right wherever the spine stood.
///
/// The spine's argument *values* are read off the metavariable's own closed
/// type — `(x₀ : A₀) → … → T` — rather than from the term, because that
/// telescope is exactly the context the hole was made in. That is sound because
/// the spine is the identity one by construction; a spine that is not fully
/// applied is [`Malformed::UnderappliedMeta`], reported rather than assumed
/// away.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at §4.1's limits, or [`CoreError::Malformed`] for a
/// term the elaborator could not have built.
fn zonk(meter: &mut Meter, depth: u32, term: &Term) -> Result<Term, CoreError> {
    meter.nested("zonking", |meter| {
        meter.quoted_node("zonking")?;
        let here = term.origin();
        let under = depth.saturating_add(1);
        if let Some(spine) = MetaSpine::of(term) {
            return spine.read_back(meter, depth);
        }
        let shape = match term.shape() {
            // Reached only for a metavariable that is not at the head of an
            // application, which [`MetaSpine::of`] has already handled.
            Shape::Meta(meta) => return Err(Malformed::UnderappliedMeta(meta.id()).into()),
            Shape::Var(index) => Shape::Var(*index),
            // A recursor's motive universe is the one level a constant carries,
            // and it is resolved by [`Constant`]'s own equality rather than here:
            // the level lives inside the group, which zonking does not rebuild.
            Shape::Const(constant) => Shape::Const(constant.clone()),
            Shape::Universe(level) => Shape::Universe(level.resolved()),
            Shape::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => Shape::Pi {
                plicity: *plicity,
                name: Arc::clone(name),
                domain: zonk(meter, depth, domain)?,
                codomain: zonk(meter, under, codomain)?,
            },
            Shape::Lam { name, body } => Shape::Lam {
                name: Arc::clone(name),
                body: zonk(meter, under, body)?,
            },
            Shape::App { function, argument } => Shape::App {
                function: zonk(meter, depth, function)?,
                argument: zonk(meter, depth, argument)?,
            },
            // A record type is a telescope, so field `i` is read under `i` more
            // binders; a record value binds nothing.
            Shape::RecordType(fields) => Shape::RecordType(zonk_telescope(meter, depth, fields)?),
            Shape::Record(fields) => Shape::Record(zonk_fields(meter, depth, fields)?),
            Shape::Project { record, field } => Shape::Project {
                record: zonk(meter, depth, record)?,
                field: Arc::clone(field),
            },
            Shape::Id { ty, left, right } => Shape::Id {
                ty: zonk(meter, depth, ty)?,
                left: zonk(meter, depth, left)?,
                right: zonk(meter, depth, right)?,
            },
            Shape::Refl(value) => Shape::Refl(zonk(meter, depth, value)?),
            Shape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => Shape::J {
                ty: zonk(meter, depth, ty)?,
                from: zonk(meter, depth, from)?,
                motive: zonk(meter, depth, motive)?,
                base: zonk(meter, depth, base)?,
                to: zonk(meter, depth, to)?,
                proof: zonk(meter, depth, proof)?,
            },
            Shape::Let { name, ty, value, body } => Shape::Let {
                name: Arc::clone(name),
                ty: zonk(meter, depth, ty)?,
                value: zonk(meter, depth, value)?,
                body: zonk(meter, under, body)?,
            },
        };
        Ok(Term::new(here, shape))
    })
}

/// A metavariable at the head of an application, and how many arguments it is
/// under.
struct MetaSpine<'a> {
    origin: Origin,
    meta: &'a Meta,
    applied: u32,
    /// Arguments beyond the metavariable's own arity, outermost first. Empty in
    /// every term the elaborator builds; kept so that a metavariable of function
    /// type applied further still reads back.
    extra: Vec<&'a Term>,
}

impl<'a> MetaSpine<'a> {
    /// The metavariable spine `term` is, if it is one.
    fn of(term: &'a Term) -> Option<Self> {
        let mut arguments = Vec::new();
        let mut head = term;
        while let Shape::App { function, argument } = head.shape() {
            arguments.push(argument);
            head = function;
        }
        let Shape::Meta(meta) = head.shape() else {
            return None;
        };
        arguments.reverse();
        let applied = u32::try_from(arguments.len()).unwrap_or(u32::MAX);
        let taken = usize::try_from(meta.arity().min(applied)).unwrap_or(usize::MAX);
        Some(Self {
            origin: term.origin(),
            meta,
            applied,
            extra: arguments.split_off(taken.min(arguments.len())),
        })
    }

    /// The term this spine stands for, at `depth` binders.
    fn read_back(self, meter: &mut Meter, depth: u32) -> Result<Term, CoreError> {
        let arity = self.meta.arity();
        if self.applied < arity {
            return Err(Malformed::UnderappliedMeta(self.meta.id()).into());
        }
        let Some(solution) = self.meta.solution() else {
            return Err(Malformed::UnderappliedMeta(self.meta.id()).into());
        };
        let mut ty = self.meta.ty().clone();
        let mut value = solution.clone();
        for level in 0..arity {
            let unfolded = force(meter, &ty)?;
            let forced = unfolded.as_ref().unwrap_or(&ty);
            let Form::Pi { domain, codomain, .. } = &forced.form else {
                return Err(Malformed::NotAFunction.into());
            };
            let variable = Value::var(self.origin, DbLevel(level), Arc::clone(domain));
            let codomain = codomain.clone();
            value = apply(meter, self.origin, value, variable.clone())?;
            ty = apply_closure(meter, &codomain, variable)?;
        }
        let mut read = quote(meter, Depth(depth), &ty, &value)?.at(self.origin);
        for argument in self.extra {
            read = Term::app(self.origin, read, zonk(meter, depth, argument)?);
        }
        Ok(read)
    }
}

fn zonk_fields(meter: &mut Meter, depth: u32, fields: &[Field]) -> Result<Arc<[Field]>, CoreError> {
    fields
        .iter()
        .map(|field| {
            Ok(Field {
                name: Arc::clone(&field.name),
                term: zonk(meter, depth, &field.term)?,
            })
        })
        .collect()
}

fn zonk_telescope(meter: &mut Meter, depth: u32, fields: &[Field]) -> Result<Arc<[Field]>, CoreError> {
    fields
        .iter()
        .enumerate()
        .map(|(position, field)| {
            let under = depth.saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
            Ok(Field {
                name: Arc::clone(&field.name),
                term: zonk(meter, under, &field.term)?,
            })
        })
        .collect()
}
