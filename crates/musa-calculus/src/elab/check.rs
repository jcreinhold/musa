//! The checking judgment: the forms that read the type they are given.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::error::Malformed;
use crate::eval::{apply_closure, eval, opened};
use crate::origin::Origin;
use crate::raw::{Raw, RawField, RawShape};
use crate::refuse::{ElabError, Refusal};
use crate::scope::Scope;
use crate::term::{Field, Filling, Name, Shape, Term};
use crate::value::{Form, Telescope, Value};

use super::{Bound, Elaborator, Typed};

impl Elaborator {
    /// `Γ ⊢ raw ⇐ ty ⇝ t`.
    pub(super) fn check(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Term, ElabError> {
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
                    // §1.5's bare member rule, and the only place in the
                    // elaborator where the expected type decides which *name* a
                    // spelling means. It is here because this is where a type is
                    // expected: `Switch` is the one rule that has both a goal
                    // and a term that has not been read yet, so a filter
                    // anywhere else would be guessing at the goal or would have
                    // already resolved the name. Nothing is elaborated in order
                    // to be discarded — the head is compared, not tried.
                    None => match (raw.shape(), crate::namespace::head_of(ty)) {
                        (RawShape::Var(name), Some(head)) if scope.lookup(name).is_none() => {
                            self.constant_at(scope, raw.origin(), name, Some(&head))?
                        }
                        _ => self.infer(scope, raw)?,
                    },
                };
                // The host's index-acceptance rule answers before conversion
                // is asked: it is a *carrying*, not an equality, and a unify
                // that ran first would only report the pair as disagreeing.
                if let Some(carried) = self.carried(scope, ty, &inferred)? {
                    return Ok(carried);
                }
                // §2.1 at the one place it can fire from below: a term whose
                // inferred type still quantifies over parameters the expected
                // type can determine — a bare constructor, a generic's name —
                // is matched against `ty` before conversion is asked.
                let inferred = self.apply_spine(scope, raw.origin(), inferred, &[], Some(ty))?;
                self.conversion
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
    /// **It is here and not in [`Conversion`].** Conversion is symmetric, so a rule
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
        // of `infer` — and for a call whose result is a type parameter that
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
        let crate::value::Head::Base(ref base, _) = neutral.head else {
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
                filling,
                name,
                domain,
                body,
            } => self.lambda(scope, raw, filling, name, domain.as_ref(), body, ty),
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
            // builds is the goal itself, so there is no motive without one —
            // and reading the type off the first arm would make a program's
            // type depend on the order its arms are written in.
            RawShape::Match { subjects, arms } => crate::case::compile(self, scope, here, subjects, arms, ty).map(Some),
            RawShape::Rec {
                name,
                ty: written,
                body,
            } => crate::rec::define(self, scope, here, name, written, body, ty).map(Some),
            // §2.1's spine in a checking position, for the one call that
            // needs it: an argument the walk will defer has nothing inside the
            // call to type it, and the position the call stands in is the last
            // thing that can. `compose(fn (p) { … }, fn (q) { … })` at an
            // annotated `let` is the shape — every argument is a bare lambda,
            // so no argument teaches another, and the annotation does.
            //
            // Guarded on the *written* form rather than on what the walk turns
            // out to defer, so the question is decided before anything is
            // elaborated: a call with no checking-only argument can never
            // defer, and takes exactly the path it always took — inferred,
            // then `Switch`, where [`Self::carried`] gets its say. That is
            // where the guard earns its keep, since an acceptance rule needs
            // the mismatch to reach it rather than be unified away.
            RawShape::Call { function, arguments } if arguments.iter().any(Raw::checks_only) => {
                match self.abstracted(scope, raw, ty)? {
                    Some(term) => Ok(Some(term)),
                    None => self
                        .complete_call(scope, here, function, arguments, Some(ty))
                        .map(|typed| Some(typed.term)),
                }
            }
            RawShape::Var(_)
            | RawShape::Hosted(_)
            | RawShape::Lit(_)
            | RawShape::Numeral { .. }
            | RawShape::Universe(_)
            | RawShape::Pi { .. }
            | RawShape::Indexed { .. }
            | RawShape::App { .. }
            | RawShape::Call { .. }
            | RawShape::RecordType(_)
            | RawShape::Method { .. }
            | RawShape::Project { .. }
            | RawShape::Update { .. }
            | RawShape::Annot { .. } => self.abstracted(scope, raw, ty),
        }
    }

    /// Wrap `raw` in a λ for an unwritten binder when the type it is checked
    /// against wants one, or answer `None` so that `Switch` runs.
    ///
    /// The second half of parameter filling, and the reason `Switch` never
    /// meets a parameter Π: a term whose own form does not abstract the binder
    /// has one abstracted for it here.
    fn abstracted(&mut self, scope: &Scope, raw: &Raw, ty: &Value) -> Result<Option<Term>, ElabError> {
        let Form::Pi {
            filling,
            name,
            domain,
            codomain,
        } = &ty.form
        else {
            return Ok(None);
        };
        if *filling == Filling::Written {
            return Ok(None);
        }
        let (name, domain, codomain) = (Arc::clone(name), Arc::clone(domain), codomain.clone());
        let here = raw.origin();
        let variable = scope.fresh_var(here, Arc::clone(&domain));
        let body_ty = apply_closure(&mut self.meter, &codomain, variable)?;
        // Nothing stands between the binder and the scope. Before prompt 146
        // a constraint binder discharged its key here, so that a body writing
        // `x == y` inside a constrained `same` reached *that* binder rather
        // than a global instance. `Storable` is the only constraint left, no
        // source program can write one, and nothing in a machine constructor's
        // body asks for the evidence by name.
        let inner = scope.assume(Some(Arc::clone(&name)), here, domain);
        Ok(Some(Term::lam(here, name, self.check(&inner, raw, &body_ty)?)))
    }

    /// `λx. e ⇐ (x : A) → B`, and the filling rules that go with it.
    fn lambda(
        &mut self,
        scope: &Scope,
        raw: &Raw,
        filling: &Filling,
        name: &Name,
        domain: Option<&Raw>,
        body: &Raw,
        ty: &Value,
    ) -> Result<Option<Term>, ElabError> {
        let here = raw.origin();
        let Form::Pi {
            filling: expected,
            name: _,
            domain: expected_domain,
            codomain,
        } = &ty.form
        else {
            return self.abstracted(scope, raw, ty);
        };
        if *filling != *expected {
            // A parameter λ at a written binder is a mistake rather than a
            // term to abstract around: the author wrote the binder, at the
            // filling the type does not have.
            if *filling == Filling::Parameter {
                return Err(Refusal::FillingMismatch { at: here }.into());
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
            self.conversion.unify_types(
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

    /// Elaborate a `let`'s definition and answer the scope its body is read in.
    pub(super) fn definition(
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
}
