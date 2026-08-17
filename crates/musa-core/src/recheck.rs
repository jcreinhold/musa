//! An independent type checker for core terms.
//!
//! # Why a second checker is not a second semantics
//!
//! `docs/rules/language/02-core-calculus.md` §3 insists there is **one**
//! evaluator, because two would be two semantics obliged to agree by a law
//! nobody could state. This module is not a second one: it evaluates with the
//! same [`crate::eval`], compares with the same quotation, and decides
//! conversion by the same normal forms. What it does not share with
//! [`crate::elab`] is the *rules* — no metavariables, no unification, no
//! implicit insertion, no raw term — and that is the whole point. It re-derives
//! a typing for a term the elaborator claims it produced, using nothing the
//! elaborator inferred.
//!
//! Prompt 134's Design calls the invariant it gates "the single most valuable
//! invariant in the whole crate": **an elaborated term type-checks in the
//! core.** A checker that reused the elaborator's own decisions could not
//! notice a wrong one.
//!
//! # What it refuses that the elaborator accepts
//!
//! A metavariable. §2.1 leaves none in an accepted term — solved ones are read
//! back, and an unsolved one is a refusal — so one arriving here means zonking
//! missed it, and that is [`Malformed::UnderappliedMeta`] rather than a verdict
//! about the author's program.
//!
//! And **an elimination applied directly to an introduction form**: `(λx. e) a`,
//! `{ f = e }.f`. Both are well typed, and neither can be re-checked here, for a
//! reason that is the core's design rather than an omission. §1's λ carries no
//! domain and the core has no annotation form, so the type the author supplied —
//! the only thing that ever gave those subterms a type — is exactly what
//! elaboration erased. This module reports them as [`Refusal::Uninferable`]
//! rather than guessing, which keeps its promise one-directional and honest:
//! everything it accepts is well typed, and a refusal here is a claim about this
//! checker's reach, not about the program.
//!
//! Elaboration reaches such a term only by a β- or projection-redex the author
//! wrote themselves — and that is *kept* true rather than observed. A solved
//! metavariable is read back as a normal form, and a normal form at a Π is
//! η-long while one at a record type is a literal, so a resolved trait method is
//! an introduction form and the call the author wrote around it is exactly the
//! redex above. [`crate::elab`]'s read-back therefore writes the type the hole
//! stood at into a `let` — §1 has one and §2 gives it a rule — so the head of
//! every elimination elaboration produces infers.
//!
//! The corpus in `tests/suite` stays clear of author-written redexes, and prompt
//! 135 revisits the question when `match` gives the core a second elimination
//! form to answer it for.

use std::sync::Arc;

use crate::budget::Meter;
use crate::context::Cx;
use crate::error::Malformed;
use crate::eval::{apply, apply_closure, eval, field_type, force};
use crate::level::Level;
use crate::origin::Origin;
use crate::quote::{quote, quote_type};
use crate::refuse::{ElabError, Mismatch, Refusal};
use crate::term::{DbLevel, Field, Shape, Term};
use crate::value::{Closure, Env, Form, Telescope, Value};

/// Check that `term` inhabits `ty` in `cx`, from the core rules alone.
///
/// # Errors
///
/// [`ElabError::Refused`] when it does not, [`ElabError::Exhausted`] at a budget
/// limit, and [`ElabError::Malformed`] when the term is not one this crate could
/// have produced — a leftover metavariable among them.
pub fn well_typed(cx: &Cx, ty: &Term, term: &Term) -> Result<(), ElabError> {
    let mut meter = cx.meter();
    let ty = eval(&mut meter, cx.env(), ty)?;
    Checker { meter: &mut meter }.check(cx, term, &ty)
}

/// The universe a type inhabits, from the core rules alone.
///
/// The one part of the re-checker elaboration itself uses. [`crate::case`] needs
/// it because §1.3 admits no universe polymorphism: a recursor's motive level is
/// chosen per use site, and a `match`'s use site has no choice to make — the
/// motive's body *is* the goal, so the level is the goal's and asking is the
/// only way to learn it.
///
/// The meter is borrowed rather than made fresh, because a second one would let
/// a nested question spend a budget §4 has already accounted for.
///
/// # Errors
///
/// [`Refusal::NotAType`] when `ty` is not one, and otherwise as [`well_typed`].
pub(crate) fn universe_of(meter: &mut Meter, cx: &Cx, ty: &Term) -> Result<Level, ElabError> {
    Checker { meter }.universe(cx, ty)
}

struct Checker<'a> {
    meter: &'a mut Meter,
}

impl Checker<'_> {
    /// `Γ ⊢ term ⇐ ty`.
    fn check(&mut self, cx: &Cx, term: &Term, ty: &Value) -> Result<(), ElabError> {
        let unfolded = force(self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        let here = term.origin();
        match (term.shape(), &ty.form) {
            (Shape::Lam { name: _, body }, Form::Pi { domain, codomain, .. }) => {
                let variable = Value::var(here, DbLevel(cx.depth()), Arc::clone(domain));
                let body_ty = apply_closure(self.meter, codomain, variable)?;
                self.check(&cx.assumed(here, Arc::clone(domain)), body, &body_ty)
            }
            (Shape::Record(fields), Form::RecordType(telescope)) => self.literal(cx, here, fields, telescope),
            // A `let` passes the goal through to its body. Without this rule a
            // definition could only stand where its body infers, and a case
            // tree's leaf is a `let` per pattern binder around a body that may
            // be any introduction form — an accumulating recursion's arms are λs
            // — so inferring through one would refuse programs elaboration
            // accepted for reasons that are about this checker and not them.
            (
                Shape::Let {
                    name: _,
                    ty: declared,
                    value,
                    body,
                },
                _,
            ) => {
                self.universe(cx, declared)?;
                let declared = eval(self.meter, cx.env(), declared)?;
                self.check(cx, value, &declared)?;
                let bound = eval(self.meter, cx.env(), value)?;
                // The goal is unchanged rather than weakened: a value indexes
                // its variables by level, so one built here still names the same
                // binders one binder deeper.
                self.check(&cx.defined(Arc::new(declared), bound), body, ty)
            }
            (Shape::Refl(witness), Form::Id { ty: at, left, right }) => {
                self.check(cx, witness, at)?;
                let value = eval(self.meter, cx.env(), witness)?;
                for endpoint in [left.as_ref(), right.as_ref()] {
                    self.same(cx, here, at, &value, endpoint)?;
                }
                Ok(())
            }
            // Everything else infers and converts. Deliberately the same shape
            // as §2's `Switch`, minus the unification: by this point there is
            // nothing left to solve.
            _ => {
                let found = self.infer(cx, term)?;
                self.same_types(cx, here, ty, &found)
            }
        }
    }

    /// `Γ ⊢ term ⇒ ty`.
    fn infer(&mut self, cx: &Cx, term: &Term) -> Result<Value, ElabError> {
        self.meter.step("re-checking")?;
        let here = term.origin();
        match term.shape() {
            Shape::Var(index) => {
                let Some(ty) = cx.binder_type(*index) else {
                    return Err(Malformed::UnboundVariable(*index).into());
                };
                Ok(Value::clone(ty))
            }
            // A constant's type is decided by its declaration, and the
            // declaration was checked when it was made. Re-checking it here would
            // re-run strict positivity at every occurrence of `Nat`.
            Shape::Const(constant) => Ok(constant.ty(self.meter)?),
            // And once more for §2.4: a definition's type was checked when the
            // program was declared, and a use is a reference to it rather than
            // a copy of the body — so this reads the type off and does not
            // re-check the definition at every name of it.
            Shape::Def(def) => Ok(Value::clone(&def.ty())),
            // The same argument one line up, for §5.8's extension: a base type's
            // kind, a builtin's signature, and a literal's type were all fixed
            // by the host's registration, and re-deriving one here would be
            // re-checking the registry at every occurrence of `Nat`.
            Shape::Base(base) => Ok(eval(self.meter, &Env::EMPTY, base.kind())?),
            Shape::Builtin(builtin) => Ok(eval(self.meter, &Env::EMPTY, builtin.ty())?),
            Shape::Lit(literal) => Ok(eval(self.meter, &Env::EMPTY, literal.ty())?),
            // And once more, for the same reason: a numeral's type is the
            // family it counts at, which the declaration already fixed. The
            // count is not re-derived either — a numeral is well-typed at its
            // family for every value of `u64`, which is exactly what makes the
            // representation total.
            Shape::Numeral(numeral) => Ok(numeral.family.value(here)),
            Shape::Universe(level) => Ok(Value::new(here, Form::Universe(level.succ()))),
            Shape::Pi {
                plicity: _,
                name: _,
                domain,
                codomain,
            } => {
                let domain_level = self.universe(cx, domain)?;
                let domain_value = eval(self.meter, cx.env(), domain)?;
                let inner = cx.assumed(here, Arc::new(domain_value));
                let codomain_level = self.universe(&inner, codomain)?;
                Ok(Value::new(here, Form::Universe(domain_level.max(&codomain_level))))
            }
            // Introduction forms check; see the module doc for the one term
            // shape this makes un-re-checkable and why that is the core's
            // erasure rather than a hole here.
            Shape::Lam { .. } => Err(Refusal::Uninferable { at: here }.into()),
            Shape::App { function, argument } => {
                let function_ty = self.infer(cx, function)?;
                let (domain, codomain) = self.function_parts(cx, here, &function_ty)?;
                self.check(cx, argument, &domain)?;
                let value = eval(self.meter, cx.env(), argument)?;
                Ok(apply_closure(self.meter, &codomain, value)?)
            }
            Shape::RecordType(fields) => {
                let mut level = Level::ZERO;
                let mut inner = cx.clone();
                for field in fields.iter() {
                    level = level.max(&self.universe(&inner, &field.term)?);
                    let value = eval(self.meter, inner.env(), &field.term)?;
                    inner = inner.assumed(field.term.origin(), Arc::new(value));
                }
                Ok(Value::new(here, Form::Universe(level)))
            }
            Shape::Record(_) => Err(Refusal::Uninferable { at: here }.into()),
            Shape::Project { record, field } => {
                let record_ty = self.infer(cx, record)?;
                let unfolded = force(self.meter, &record_ty)?;
                let record_ty = unfolded.as_ref().unwrap_or(&record_ty);
                let Form::RecordType(telescope) = &record_ty.form else {
                    return Err(Refusal::NotARecord {
                        at: here,
                        ty: quote_type(self.meter, cx.quoting_depth(), record_ty)?,
                    }
                    .into());
                };
                if !telescope.fields.iter().any(|declared| &declared.name == field) {
                    return Err(Refusal::NoSuchField {
                        at: here,
                        field: Arc::clone(field),
                    }
                    .into());
                }
                let telescope = telescope.clone();
                let subject = eval(self.meter, cx.env(), record)?;
                Ok(field_type(self.meter, &telescope, &subject, field)?)
            }
            Shape::Id { ty, left, right } => {
                let level = self.universe(cx, ty)?;
                let at = eval(self.meter, cx.env(), ty)?;
                self.check(cx, left, &at)?;
                self.check(cx, right, &at)?;
                Ok(Value::new(here, Form::Universe(level)))
            }
            Shape::Refl(witness) => {
                let at = self.infer(cx, witness)?;
                let value = eval(self.meter, cx.env(), witness)?;
                Ok(Value::new(
                    here,
                    Form::Id {
                        ty: Arc::new(at),
                        left: Arc::new(value.clone()),
                        right: Arc::new(value),
                    },
                ))
            }
            Shape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => self.elimination(cx, here, [ty, from, motive, base, to, proof]),
            Shape::Meta(meta) => Err(Malformed::UnderappliedMeta(meta.id()).into()),
            Shape::Let {
                name: _,
                ty,
                value,
                body,
            } => {
                self.universe(cx, ty)?;
                let ty_value = eval(self.meter, cx.env(), ty)?;
                self.check(cx, value, &ty_value)?;
                let bound = eval(self.meter, cx.env(), value)?;
                self.infer(&cx.defined(Arc::new(ty_value), bound), body)
            }
        }
    }

    /// `J A x P p y e ⇒ P y e`.
    fn elimination(&mut self, cx: &Cx, here: Origin, parts: [&Term; 6]) -> Result<Value, ElabError> {
        let [ty, from, motive, base, to, proof] = parts;
        self.universe(cx, ty)?;
        let ty_value = eval(self.meter, cx.env(), ty)?;

        self.check(cx, from, &ty_value)?;
        let from_value = eval(self.meter, cx.env(), from)?;

        self.motive(cx, here, motive, &ty_value, &from_value)?;
        let motive_value = eval(self.meter, cx.env(), motive)?;

        let refl_from = Value::new(here, Form::Refl(Arc::new(from_value.clone())));
        let at_from = apply(self.meter, here, motive_value.clone(), from_value.clone())?;
        let base_ty = apply(self.meter, here, at_from, refl_from)?;
        self.check(cx, base, &base_ty)?;

        self.check(cx, to, &ty_value)?;
        let to_value = eval(self.meter, cx.env(), to)?;

        let proof_ty = Value::new(
            here,
            Form::Id {
                ty: Arc::new(ty_value),
                left: Arc::new(from_value),
                right: Arc::new(to_value.clone()),
            },
        );
        self.check(cx, proof, &proof_ty)?;
        let proof_value = eval(self.meter, cx.env(), proof)?;

        let at_to = apply(self.meter, here, motive_value, to_value)?;
        Ok(apply(self.meter, here, at_to, proof_value)?)
    }

    /// Require that `motive` is a `(y : subject) → Id subject from y → Type l`.
    ///
    /// J's motive stands in a **checking** position: `subject` and `from` fix
    /// everything about its type except which universe it lands in. So when the
    /// motive is the double λ that elaborating one almost always produces, this
    /// checks it directly — its body, read under the two binders, is where the
    /// level is written, and asking whether that body is a type *is* the check.
    /// A motive of any other shape (a variable, an application) has a type to
    /// infer, and [`Self::motive_shape`] tests that one against the same
    /// telescope.
    fn motive(&mut self, cx: &Cx, at: Origin, motive: &Term, subject: &Value, from: &Value) -> Result<(), ElabError> {
        let Some(body) = under_two_binders(motive) else {
            let motive_ty = self.infer(cx, motive)?;
            return self.motive_shape(cx, at, &motive_ty, subject, from);
        };
        let endpoint = Value::var(at, DbLevel(cx.depth()), Arc::new(Value::clone(subject)));
        let under = cx.assumed(at, Arc::new(Value::clone(subject)));
        let proof_ty = Value::new(
            at,
            Form::Id {
                ty: Arc::new(Value::clone(subject)),
                left: Arc::new(Value::clone(from)),
                right: Arc::new(endpoint),
            },
        );
        self.universe(&under.assumed(at, Arc::new(proof_ty)), body)?;
        Ok(())
    }

    /// Require that `motive_ty` is `(y : subject) → Id subject from y → Type l`.
    fn motive_shape(
        &mut self,
        cx: &Cx,
        at: Origin,
        motive_ty: &Value,
        subject: &Value,
        from: &Value,
    ) -> Result<(), ElabError> {
        let (endpoint_domain, endpoint_codomain) = self.function_parts(cx, at, motive_ty)?;
        self.same_types(cx, at, subject, &endpoint_domain)?;
        let endpoint = Value::var(at, DbLevel(cx.depth()), Arc::new(Value::clone(subject)));
        let under = cx.assumed(at, Arc::new(Value::clone(subject)));
        let after_endpoint = apply_closure(self.meter, &endpoint_codomain, endpoint.clone())?;
        let (proof_domain, proof_codomain) = self.function_parts(&under, at, &after_endpoint)?;
        let expected_proof = Value::new(
            at,
            Form::Id {
                ty: Arc::new(Value::clone(subject)),
                left: Arc::new(Value::clone(from)),
                right: Arc::new(endpoint),
            },
        );
        self.same_types(&under, at, &expected_proof, &proof_domain)?;
        let witness = Value::var(at, DbLevel(under.depth()), Arc::new(proof_domain));
        let inside = under.assumed(at, Arc::new(expected_proof));
        let result = apply_closure(self.meter, &proof_codomain, witness)?;
        let unfolded = force(self.meter, &result)?;
        let result = unfolded.as_ref().unwrap_or(&result);
        let Form::Universe(_) = result.form else {
            return Err(Refusal::NotAType {
                at,
                ty: quote_type(self.meter, inside.quoting_depth(), result)?,
            }
            .into());
        };
        Ok(())
    }

    /// `{ f = e, … } ⇐ { f : A, … }`.
    fn literal(&mut self, cx: &Cx, here: Origin, fields: &[Field], telescope: &Telescope) -> Result<(), ElabError> {
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
        for (written, declared) in fields.iter().zip(telescope.fields.iter()) {
            let field_ty = eval(self.meter, &env, &declared.term)?;
            self.check(cx, &written.term, &field_ty)?;
            env = env.push(eval(self.meter, cx.env(), &written.term)?);
        }
        Ok(())
    }

    /// The level of a term standing in type position.
    fn universe(&mut self, cx: &Cx, term: &Term) -> Result<Level, ElabError> {
        let ty = self.infer(cx, term)?;
        let unfolded = force(self.meter, &ty)?;
        let ty = unfolded.as_ref().unwrap_or(&ty);
        let Form::Universe(level) = &ty.form else {
            return Err(Refusal::NotAType {
                at: term.origin(),
                ty: quote_type(self.meter, cx.quoting_depth(), ty)?,
            }
            .into());
        };
        Ok(level.clone())
    }

    /// Take a Π type apart, refusing what is not one.
    fn function_parts(&mut self, cx: &Cx, at: Origin, ty: &Value) -> Result<(Value, Closure), ElabError> {
        let unfolded = force(self.meter, ty)?;
        let ty = unfolded.as_ref().unwrap_or(ty);
        let Form::Pi { domain, codomain, .. } = &ty.form else {
            return Err(Refusal::NotAFunction {
                at,
                ty: quote_type(self.meter, cx.quoting_depth(), ty)?,
            }
            .into());
        };
        Ok((Value::clone(domain), codomain.clone()))
    }

    /// Conversion at the level of types: both sides read back, compared up to α.
    fn same_types(&mut self, cx: &Cx, at: Origin, expected: &Value, found: &Value) -> Result<(), ElabError> {
        let expected = quote_type(self.meter, cx.quoting_depth(), expected)?;
        let found = quote_type(self.meter, cx.quoting_depth(), found)?;
        Self::agree(at, expected, found)
    }

    /// Conversion at a type.
    fn same(&mut self, cx: &Cx, at: Origin, ty: &Value, expected: &Value, found: &Value) -> Result<(), ElabError> {
        let expected = quote(self.meter, cx.quoting_depth(), ty, expected)?;
        let found = quote(self.meter, cx.quoting_depth(), ty, found)?;
        Self::agree(at, expected, found)
    }

    /// The path is empty here on purpose. `refuse.rs`'s policy — the smallest
    /// disagreeing pair, and the route to it — is unification's to produce,
    /// because unification is what descends. This checker compares two whole
    /// normal forms and has no route to report; it is a gate on the elaborator,
    /// and its message is read by whoever broke one.
    fn agree(at: Origin, expected: Term, found: Term) -> Result<(), ElabError> {
        if expected == found {
            return Ok(());
        }
        Err(Refusal::Mismatch(Box::new(Mismatch {
            at,
            expected,
            found,
            path: Vec::new(),
        }))
        .into())
    }
}

/// The body of `λ_. λ_. body`, when the term is one.
///
/// Free of the checker so that it reads as what it is: a question about a
/// term's shape, asked in one place.
fn under_two_binders(term: &Term) -> Option<&Term> {
    let Shape::Lam { body: inner, .. } = term.shape() else {
        return None;
    };
    if let Shape::Lam { body, .. } = inner.shape() {
        Some(body)
    } else {
        None
    }
}
