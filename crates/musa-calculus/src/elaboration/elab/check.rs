//! The checking judgment: the forms that read the type they are given.
//!
//! See the `elab` module docs for the judgments these rules belong to.

use std::sync::Arc;

use crate::elaboration::raw::{Raw, RawField, RawShape};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::eval::{apply_closure, opened};
use crate::kernel::origin::Origin;
use crate::kernel::scope::Scope;
use crate::kernel::term::{Filling, Name, Term};
use crate::kernel::value::{Form, Value};

use super::{Bound, Elaborator};

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
                    None => match (raw.shape(), crate::elaboration::namespace::head_of(ty)) {
                        (RawShape::Var(name), Some(head)) if scope.lookup(name).is_none() => {
                            self.constant_at(scope, raw.origin(), name, Some(&head))?
                        }
                        _ => self.infer(scope, raw)?,
                    },
                };
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
            RawShape::Record { head, fields } => match self.product(ty)? {
                Some(product) => {
                    self.head_agrees(scope, head.as_deref(), &product)?;
                    self.literal(scope, here, fields, &product).map(Some)
                }
                None => self.abstracted(scope, raw, ty),
            },
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
            RawShape::Match { subjects, arms } => {
                crate::elaboration::case::compile(self, scope, here, subjects, arms, ty).map(Some)
            }
            // A `rec` at a definition's top is compiled where it stands; a
            // `rec` written *inside* a term closes over the binders around it,
            // which is a thing no global name can be, so it is lifted out to
            // one first (prompt 155aa). `lift` decides by the scope it is
            // handed, which is the only place the difference shows.
            RawShape::Rec {
                name,
                ty: written,
                body,
            } => crate::elaboration::rec::lift(self, scope, here, name, written, body, ty).map(Some),
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
            // then `Switch`.
            RawShape::Call {
                function,
                arguments,
                supplied,
            } if arguments.iter().any(Raw::checks_only) => match self.abstracted(scope, raw, ty)? {
                Some(term) => Ok(Some(term)),
                None => self
                    .complete_call(scope, here, function, arguments, supplied, Some(ty))
                    .map(|typed| Some(typed.term)),
            },
            RawShape::Var(_)
            | RawShape::Hosted(_)
            | RawShape::Lit(_)
            | RawShape::Numeral { .. }
            | RawShape::Universe(_)
            | RawShape::Pi { .. }
            | RawShape::App { .. }
            | RawShape::Call { .. }
            | RawShape::Method { .. }
            | RawShape::Project { .. }
            | RawShape::Update { .. }
            | RawShape::Annot { .. } => self.abstracted(scope, raw, ty),
        }
    }

    /// Wrap `raw` in a λ for an unwritten binder when the type it is checked
    /// against wants one, or answer `None` so that `Switch` runs.
    ///
    /// The second half of parameter filling: a term whose own form does not
    /// abstract the binder has one abstracted for it here.
    ///
    /// **Except when the term already quantifies the same way**, which is
    /// prompt 154's stopping rule on this side of it. `Switch` does meet a
    /// parameter Π, and it has to: wrapping a λ around a term that carries its
    /// own scheme takes the expected binder away before the term's own
    /// inferred one can be matched against it, and an implicit the body cannot
    /// determine is then reported as undetermined. `{A} → {B} → A → A` checked
    /// at that very type is the case, and [`super::spine::Keeping`] is the
    /// other half — it is what stops `Switch`'s own walk from filling the
    /// scheme back in.
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
        if self.quantifies_already(scope, raw)? {
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

    /// Whether `raw`'s own type already begins with an inferred binder, so that
    /// [`Self::abstracted`] must leave the expected one for it to meet.
    ///
    /// **Answered without elaborating anything.** `Switch` is the rule that
    /// elaborates, and a peek that inferred `raw` here in order to decide
    /// whether to throw the answer away would be the "try it and see" the
    /// conversion checker is written to avoid — and for a declared name it
    /// would mint a second set of level unknowns for a reading that never
    /// happens. So the question is asked only of the two things a scope can
    /// answer by looking: a binder's recorded type, and a definition's stored
    /// one. A name that is neither — a constructor, a recursor, a registered
    /// base, a namespaced member — answers `false` and is abstracted as before,
    /// which is what those all did before this rule existed.
    ///
    /// Only [`RawShape::Var`] for the same reason. Every other form that
    /// reaches [`Self::abstracted`] would have to be elaborated to have a type
    /// at all, and a *call*'s result quantifying over an inferred binder is a
    /// call §1.3 has already refused as under-applied.
    fn quantifies_already(&mut self, scope: &Scope, raw: &Raw) -> Result<bool, ElabError> {
        let RawShape::Var(name) = raw.shape() else {
            return Ok(false);
        };
        let stated = match scope.lookup(name) {
            Some(found) => found.ty,
            None => match scope.cx().definition(name) {
                Some(defined) => Arc::clone(&defined.ty),
                None => return Ok(false),
            },
        };
        let unfolded = opened(&mut self.meter, &stated)?;
        Ok(matches!(
            unfolded.as_ref().unwrap_or(&stated).form,
            Form::Pi {
                filling: Filling::Parameter,
                ..
            }
        ))
    }

    /// `ty` made into a function type, when it is an unknown that has to be
    /// one — `None` when it is anything else.
    ///
    /// §2.1's fragment says a solution is unique when the spine is one, and it
    /// says nothing about the shape of the value: `?m x⃗ ≟ (x : ?a) → ?b` is an
    /// ordinary assignment with two fresh unknowns on the right. So this is not
    /// a guess and not a default. It commits to the one thing a λ's type must
    /// be and to nothing else — the domain and the codomain are unknowns like
    /// any other, solved by the argument that determines them or reported at
    /// declaration end.
    ///
    /// The codomain is a closure over the *scope's* environment with one more
    /// binder, which is what makes the codomain unknown's own scope the inner
    /// one: an occurrence names its scope by level and reads it out of the
    /// environment it is evaluated in (`kernel::meta`), so applying the closure
    /// to an argument is exactly what puts that argument in the codomain's
    /// scope.
    fn expanded_unknown(
        &mut self,
        scope: &Scope,
        here: Origin,
        filling: &Filling,
        name: &Name,
        ty: &Value,
    ) -> Result<Option<Value>, ElabError> {
        if crate::kernel::unify::flexible_head(ty).is_none() {
            return Ok(None);
        }
        let sort = self.fresh_level(here);
        let domain_ty = Value::new(here, Form::Universe(sort));
        let domain = self.fresh_meta(scope, here, &domain_ty)?;
        let domain_value = Arc::new(domain.value);
        let inner = scope.assume(Some(Arc::clone(name)), here, Arc::clone(&domain_value));
        let sort = self.fresh_level(here);
        let codomain_ty = Value::new(here, Form::Universe(sort));
        let codomain = self.fresh_meta(&inner, here, &codomain_ty)?;
        let pi = Value::new(
            here,
            Form::Pi {
                filling: filling.clone(),
                name: Arc::clone(name),
                domain: domain_value,
                codomain: crate::kernel::value::Closure {
                    env: scope.env().clone(),
                    body: codomain.term,
                },
            },
        );
        self.conversion
            .unify_types(&mut self.meter, scope.depth(), here, ty, &pi)?;
        Ok(Some(pi))
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
        // A λ read against an unknown: the unknown is η-expanded into a Π over
        // two fresh ones, and the λ is then read against that.
        //
        // This is what replaces prompt 153's deleted deferral. A λ describes
        // nothing on its own, so before there was a queue the walk could not
        // read one against a domain it had not yet determined and put the
        // argument aside instead. The determination it was waiting for is the
        // one made here, and it is *forced* rather than waited for: a value
        // that has to accept a λ is a function type, whatever else is still
        // unknown about it. Nothing is guessed — the domain and the codomain
        // stay unknowns, to be solved by the same matching as any other.
        let expanded;
        let ty = match ty.form {
            Form::Pi { .. } => ty,
            Form::Universe(_) | Form::Lam(_) | Form::Lit(_) | Form::Numeral(_) | Form::Neutral(_) => {
                match self.expanded_unknown(scope, here, filling, name, ty)? {
                    Some(pi) => {
                        expanded = pi;
                        &expanded
                    }
                    None => ty,
                }
            }
        };
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

    /// `R { f = e, … } ⇐ R p⃗`, as the family's one constructor applied to its
    /// fields.
    ///
    /// The literal must give the declaration's fields in its order, because a
    /// later field's type may mention an earlier field's *value* — so the order
    /// is part of the type rather than a formatting preference.
    ///
    /// The constructor's own type is what the fields are checked against, one Π
    /// at a time, rather than a telescope re-read here: it is the same walk
    /// [`crate::kernel::family`] does to give the constructor a type, so a
    /// literal and the constructor it elaborates to cannot disagree about what
    /// field `i` stands at.
    fn literal(
        &mut self,
        scope: &Scope,
        here: Origin,
        fields: &[RawField],
        product: &crate::kernel::family::Product,
    ) -> Result<Term, ElabError> {
        if fields.len() != product.fields.len()
            || fields
                .iter()
                .zip(product.fields.iter())
                .any(|(written, declared)| written.name != declared.name)
        {
            return Err(Refusal::RecordShape {
                at: here,
                expected: product.fields.iter().map(|field| Arc::clone(&field.name)).collect(),
                found: fields.iter().map(|field| Arc::clone(&field.name)).collect(),
            }
            .into());
        }
        let globals = scope.cx().globals().clone();
        let constructor = crate::kernel::family::Constant::constructor(&product.group, product.family, 0);
        let mut built = constructor.term(here);
        let mut ty = constructor.ty(&mut self.meter, &globals)?;
        for param in &product.params {
            built = Term::app(here, built, scope.quote_type(&mut self.meter, param)?);
            // Through the telescope walk and not `apply`, for the reason
            // `record::instantiated` gives: a constructor's *type* is a Π, and
            // applying a Π as though it were a λ is the malformed-core report.
            // Unreachable until a parameterized record could be written at all,
            // which is why it stood.
            ty = crate::elaboration::elab::record::instantiated(&mut self.meter, here, &ty, param.clone())?;
        }
        for written in fields {
            let unfolded = opened(&mut self.meter, &ty)?;
            let Form::Pi { domain, codomain, .. } = &unfolded.as_ref().unwrap_or(&ty).form else {
                return Err(Refusal::NotAFunction {
                    at: here,
                    ty: scope.quote_type(&mut self.meter, &ty)?,
                }
                .into());
            };
            let (field_ty, codomain) = (Value::clone(domain), codomain.clone());
            let term = self.check(scope, &written.term, &field_ty)?;
            // The telescope proceeds under this field's own value, which is what
            // makes a later field's type able to mention it.
            let value = scope.eval(&mut self.meter, &term)?;
            ty = crate::kernel::eval::apply_closure(&mut self.meter, &codomain, value)?;
            built = Term::app(here, built, term);
        }
        Ok(built)
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
