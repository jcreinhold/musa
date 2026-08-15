//! Evaluation: terms to values.
//!
//! `docs/rules/language/02-core-calculus.md` §3: "Reduction is never performed
//! on syntax." Every rule of definitional equality is discharged here as a step
//! in the semantic domain rather than as a rewrite on a term.
//!
//! - **β** is [`apply`] on a [`Value::Lam`], which opens the closure.
//! - **δ** is [`eval`] on a [`Term::Let`] and on a variable a context *defined*
//!   rather than assumed: both put the definition's value in the environment,
//!   so unfolding is what lookup already does.
//! - **ι** is [`jay`] on a [`Value::Refl`], which discards the motive and
//!   answers the base case.
//! - **η** is *not* here. It is performed by [`crate::quote`], which is why
//!   quotation is type-directed and why two records with the same projections
//!   are convertible without a rule that inspects both at once — the property
//!   `10-traits.md`'s coherence argument rests on.
//!
//! Every descent is charged and every level of it is metered (§4.1): `NbE` gives
//! the machine a second way to stand inside itself, and a total language may
//! refuse but may not crash.

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::{CoreError, Malformed};
use crate::term::{Field, Name, Term};
use crate::value::{Closure, Env, Neutral, Telescope, Value};

/// Evaluate `term` in `env`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the term's shape makes the next step meaningless.
pub(crate) fn eval(meter: &mut Meter, env: &Env, term: &Term) -> Result<Value, CoreError> {
    meter.nested("evaluation", |meter| {
        meter.step("evaluation")?;
        match term {
            Term::Var(index) => env
                .lookup(index.0)
                .cloned()
                .ok_or_else(|| Malformed::UnboundVariable(*index).into()),
            Term::Universe(level) => Ok(Value::Universe(*level)),
            Term::Pi { name, domain, codomain } => Ok(Value::Pi {
                name: Arc::clone(name),
                domain: Arc::new(eval(meter, env, domain)?),
                codomain: Closure {
                    env: env.clone(),
                    body: Arc::clone(codomain),
                },
            }),
            Term::Lam { name: _, body } => Ok(Value::Lam(Closure {
                env: env.clone(),
                body: Arc::clone(body),
            })),
            Term::App { function, argument } => {
                let function = eval(meter, env, function)?;
                let argument = eval(meter, env, argument)?;
                apply(meter, function, argument)
            }
            Term::RecordType(fields) => Ok(Value::RecordType(Telescope {
                fields: Arc::clone(fields),
                env: env.clone(),
            })),
            Term::Record(fields) => {
                let mut built = Vec::with_capacity(fields.len());
                for field in fields.iter() {
                    built.push((Arc::clone(&field.name), eval(meter, env, &field.term)?));
                }
                Ok(Value::Record(built.into()))
            }
            Term::Project { record, field } => {
                let record = eval(meter, env, record)?;
                project(meter, record, field)
            }
            Term::Id { ty, left, right } => Ok(Value::Id {
                ty: Arc::new(eval(meter, env, ty)?),
                left: Arc::new(eval(meter, env, left)?),
                right: Arc::new(eval(meter, env, right)?),
            }),
            Term::Refl(value) => Ok(Value::Refl(Arc::new(eval(meter, env, value)?))),
            Term::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => {
                let ty = eval(meter, env, ty)?;
                let from = eval(meter, env, from)?;
                let motive = eval(meter, env, motive)?;
                let base = eval(meter, env, base)?;
                let to = eval(meter, env, to)?;
                let proof = eval(meter, env, proof)?;
                jay(meter, ty, from, motive, base, to, proof)
            }
            Term::Let {
                name: _,
                ty: _,
                value,
                body,
            } => {
                let value = eval(meter, env, value)?;
                eval(meter, &env.extend(value), body)
            }
        }
    })
}

/// Open a closure at `argument`.
///
/// # Errors
///
/// As [`eval`].
pub(crate) fn apply_closure(meter: &mut Meter, closure: &Closure, argument: Value) -> Result<Value, CoreError> {
    eval(meter, &closure.env.extend(argument), &closure.body)
}

/// β, or a blocked application.
///
/// # Errors
///
/// [`Malformed::NotAFunction`] when `function` is neither a lambda nor neutral.
pub(crate) fn apply(meter: &mut Meter, function: Value, argument: Value) -> Result<Value, CoreError> {
    meter.step("function application")?;
    match function {
        Value::Lam(body) => apply_closure(meter, &body, argument),
        Value::Neutral(function) => Ok(Value::Neutral(Arc::new(Neutral::App {
            function,
            argument: Arc::new(argument),
        }))),
        Value::Universe(_)
        | Value::Pi { .. }
        | Value::RecordType(_)
        | Value::Record(_)
        | Value::Id { .. }
        | Value::Refl(_) => Err(Malformed::NotAFunction.into()),
    }
}

/// Projection, or a blocked projection.
///
/// # Errors
///
/// [`Malformed::NotARecord`] when `record` is neither a record nor neutral, and
/// [`Malformed::NoSuchField`] when it is a record without that field.
pub(crate) fn project(meter: &mut Meter, record: Value, field: &Name) -> Result<Value, CoreError> {
    meter.step("field projection")?;
    match record {
        Value::Record(fields) => fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| Malformed::NoSuchField(Arc::clone(field)).into()),
        Value::Neutral(record) => Ok(Value::Neutral(Arc::new(Neutral::Project {
            record,
            field: Arc::clone(field),
        }))),
        Value::Universe(_)
        | Value::Pi { .. }
        | Value::Lam { .. }
        | Value::RecordType(_)
        | Value::Id { .. }
        | Value::Refl(_) => Err(Malformed::NotARecord.into()),
    }
}

/// ι at the identity type, or a blocked `J`.
///
/// `J A x P p x (refl x) ⟶ p`: the motive and both endpoints are discarded,
/// because there is nothing left for them to decide.
///
/// # Errors
///
/// [`Malformed::NotAnIdentity`] when `proof` is neither `refl` nor neutral.
pub(crate) fn jay(
    meter: &mut Meter,
    ty: Value,
    from: Value,
    motive: Value,
    base: Value,
    to: Value,
    proof: Value,
) -> Result<Value, CoreError> {
    meter.step("identity elimination")?;
    match proof {
        Value::Refl(_) => Ok(base),
        Value::Neutral(proof) => Ok(Value::Neutral(Arc::new(Neutral::J {
            ty: Arc::new(ty),
            from: Arc::new(from),
            motive: Arc::new(motive),
            base: Arc::new(base),
            to: Arc::new(to),
            proof,
        }))),
        Value::Universe(_)
        | Value::Pi { .. }
        | Value::Lam { .. }
        | Value::RecordType(_)
        | Value::Record(_)
        | Value::Id { .. } => Err(Malformed::NotAnIdentity.into()),
    }
}

/// The type of field `field` of `subject`, a value of record type `telescope`.
///
/// The telescope's earlier binders are filled with the *subject's own*
/// projections, which is what makes a later field's type able to mention an
/// earlier field's value.
///
/// # Errors
///
/// [`Malformed::NoSuchField`] when the telescope has no such field, otherwise
/// as [`eval`].
pub(crate) fn field_type(
    meter: &mut Meter,
    telescope: &Telescope,
    subject: &Value,
    field: &Name,
) -> Result<Value, CoreError> {
    let mut env = telescope.env.clone();
    for Field { name, term } in telescope.fields.iter() {
        if name == field {
            return eval(meter, &env, term);
        }
        env = env.extend(project(meter, subject.clone(), name)?);
    }
    Err(Malformed::NoSuchField(Arc::clone(field)).into())
}

/// The type of a blocked elimination.
///
/// Computed rather than stored: every neutral is built from a variable whose
/// type is known, and each elimination transforms that type in exactly one way.
/// Storing the answer at every node would be the same information twice, and
/// the two copies would be free to disagree.
///
/// # Errors
///
/// [`CoreError::Malformed`] when a neutral's head type does not admit the
/// elimination applied to it, which means the caller built a term the
/// elaborator would have refused.
pub(crate) fn neutral_type(meter: &mut Meter, neutral: &Neutral) -> Result<Value, CoreError> {
    meter.nested("neutral typing", |meter| match neutral {
        Neutral::Var(_, ty) => Ok(Value::clone(ty)),
        Neutral::App { function, argument } => match neutral_type(meter, function)? {
            Value::Pi { codomain, .. } => apply_closure(meter, &codomain, Value::clone(argument)),
            Value::Universe(_)
            | Value::Lam { .. }
            | Value::RecordType(_)
            | Value::Record(_)
            | Value::Id { .. }
            | Value::Refl(_)
            | Value::Neutral(_) => Err(Malformed::NotAFunction.into()),
        },
        Neutral::Project { record, field } => match neutral_type(meter, record)? {
            Value::RecordType(telescope) => {
                let subject = Value::Neutral(Arc::clone(record));
                field_type(meter, &telescope, &subject, field)
            }
            Value::Universe(_)
            | Value::Pi { .. }
            | Value::Lam { .. }
            | Value::Record(_)
            | Value::Id { .. }
            | Value::Refl(_)
            | Value::Neutral(_) => Err(Malformed::NotARecord.into()),
        },
        Neutral::J { motive, to, proof, .. } => {
            let at_endpoint = apply(meter, Value::clone(motive), Value::clone(to))?;
            apply(meter, at_endpoint, Value::Neutral(Arc::clone(proof)))
        }
    })
}
