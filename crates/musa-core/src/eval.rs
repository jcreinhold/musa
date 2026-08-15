//! Evaluation: terms to values.
//!
//! `docs/rules/language/02-core-calculus.md` §3: "Reduction is never performed
//! on syntax." Every rule of definitional equality is discharged here as a step
//! in the semantic domain rather than as a rewrite on a term.
//!
//! - **β** is [`apply`] on a [`Form::Lam`], which opens the closure.
//! - **δ** is [`eval`] on a [`Shape::Let`] and on a variable a context *defined*
//!   rather than assumed: both put the definition's value in the environment,
//!   so unfolding is what lookup already does.
//! - **ι** is [`jay`] on a [`Form::Refl`], which discards the motive and
//!   answers the base case.
//! - **η** is *not* here. It is performed by [`crate::quote`], which is why
//!   quotation is type-directed and why two records with the same projections
//!   are convertible without a rule that inspects both at once — the property
//!   `10-traits.md`'s coherence argument rests on.
//!
//! **Origins follow the value, not the use site** (§7). Evaluating a variable
//! answers whatever the environment holds, with the origin that value already
//! had, because §7 says provenance is preserved *by substitution*: in `e[a/x]`
//! the occurrences of `x` become `a`, and they carry `a`'s origins. The same
//! rule makes β answer the body's origins and δ answer the definition's. What
//! an elimination's own origin is for is the case where it stays blocked, and
//! that is why [`apply`], [`project`], and [`jay`] each take one.
//!
//! Every descent is charged and every level of it is metered (§4.1): `NbE` gives
//! the machine a second way to stand inside itself, and a total language may
//! refuse but may not crash.

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::{CoreError, Malformed};
use crate::origin::Origin;
use crate::term::{Field, Name, Plicity, Shape, Term};
use crate::value::{Closure, Env, Form, Neutral, Spine, Telescope, Value};

/// Evaluate `term` in `env`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the term's shape makes the next step meaningless.
pub(crate) fn eval(meter: &mut Meter, env: &Env, term: &Term) -> Result<Value, CoreError> {
    meter.nested("evaluation", |meter| {
        meter.step("evaluation")?;
        let here = term.origin();
        match term.shape() {
            Shape::Var(index) => env
                .get(index.0)
                .cloned()
                .ok_or_else(|| Malformed::UnboundVariable(*index).into()),
            // Resolved on the way in, so a value carries the level its arms
            // have already been solved to rather than the one written first.
            Shape::Universe(level) => Ok(Value::new(here, Form::Universe(level.resolved()))),
            // A constant is closed and rigid, so evaluating one is reading it.
            // ι does not fire here: it needs the target, which arrives through
            // [`apply`].
            Shape::Const(constant) => Ok(constant.value(here)),
            // A meta is closed, so the environment says nothing about it: it is
            // either its solution, with that solution's own origins (§7), or a
            // flexible head waiting for one.
            Shape::Meta(meta) => Ok(meta.solution().cloned().unwrap_or_else(|| {
                Value::neutral(Neutral {
                    origin: here,
                    spine: Spine::Meta(meta.clone()),
                })
            })),
            Shape::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => pi(meter, env, here, *plicity, name, domain, codomain),
            Shape::Lam { name: _, body } => Ok(Value::new(
                here,
                Form::Lam(Closure {
                    env: env.clone(),
                    body: body.clone(),
                }),
            )),
            Shape::App { function, argument } => application(meter, env, here, function, argument),
            Shape::RecordType(fields) => Ok(Value::new(
                here,
                Form::RecordType(Telescope {
                    fields: Arc::clone(fields),
                    env: env.clone(),
                }),
            )),
            Shape::Record(fields) => literal(meter, env, here, fields),
            Shape::Project { record, field } => projection(meter, env, here, record, field),
            Shape::Id { ty, left, right } => identity(meter, env, here, ty, left, right),
            Shape::Refl(value) => Ok(Value::new(here, Form::Refl(Arc::new(eval(meter, env, value)?)))),
            Shape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => elimination(meter, env, here, [ty, from, motive, base, to, proof]),
            Shape::Let {
                name: _,
                ty: _,
                value,
                body,
            } => binding(meter, env, value, body),
        }
    })
}

// Every arm that holds more than one intermediate value lives in its own
// function, and that is a stack-depth decision rather than a stylistic one.
//
// §4.1's nesting limit exists so that `NbE` cannot overflow the host's stack: a
// total language may refuse but may not crash. That guarantee is only real if
// [`Budget::NESTING`](crate::Budget::NESTING) levels of this function actually
// fit in one. A debug build gives a frame room for *every* arm's temporaries at
// once, whether or not the term took that arm, so one match holding all twelve
// cost about 9 KiB a level, and a term nested past 230 aborted the process
// before the meter reached 256 and could refuse. Split this way it is about
// 2 KiB — measured by halving `RUST_MIN_STACK` until the deepest accepted term
// crashed — so the whole limit costs ~0.5 MiB of a 2 MiB test thread.
//
// `budget_laws.rs`'s `a_term_nested_past_the_limit_is_refused` is the test that
// notices when this stops being true. Quotation walks values the same way and
// costs about the same per level; nothing there needed splitting yet.

fn pi(
    meter: &mut Meter,
    env: &Env,
    here: Origin,
    plicity: Plicity,
    name: &Name,
    domain: &Term,
    codomain: &Term,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        here,
        Form::Pi {
            plicity,
            name: Arc::clone(name),
            domain: Arc::new(eval(meter, env, domain)?),
            codomain: Closure {
                env: env.clone(),
                body: codomain.clone(),
            },
        },
    ))
}

fn application(
    meter: &mut Meter,
    env: &Env,
    here: Origin,
    function: &Term,
    argument: &Term,
) -> Result<Value, CoreError> {
    let function = eval(meter, env, function)?;
    let argument = eval(meter, env, argument)?;
    apply(meter, here, function, argument)
}

fn literal(meter: &mut Meter, env: &Env, here: Origin, fields: &[Field]) -> Result<Value, CoreError> {
    let mut built = Vec::with_capacity(fields.len());
    for field in fields {
        built.push((Arc::clone(&field.name), eval(meter, env, &field.term)?));
    }
    Ok(Value::new(here, Form::Record(built.into())))
}

fn projection(meter: &mut Meter, env: &Env, here: Origin, record: &Term, field: &Name) -> Result<Value, CoreError> {
    let record = eval(meter, env, record)?;
    project(meter, here, record, field)
}

fn identity(
    meter: &mut Meter,
    env: &Env,
    here: Origin,
    ty: &Term,
    left: &Term,
    right: &Term,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        here,
        Form::Id {
            ty: Arc::new(eval(meter, env, ty)?),
            left: Arc::new(eval(meter, env, left)?),
            right: Arc::new(eval(meter, env, right)?),
        },
    ))
}

/// `J`'s six arguments, in the order [`Shape::J`] declares them.
fn elimination(meter: &mut Meter, env: &Env, here: Origin, arguments: [&Term; 6]) -> Result<Value, CoreError> {
    let [ty, from, motive, base, to, proof] = arguments;
    let ty = eval(meter, env, ty)?;
    let from = eval(meter, env, from)?;
    let motive = eval(meter, env, motive)?;
    let base = eval(meter, env, base)?;
    let to = eval(meter, env, to)?;
    let proof = eval(meter, env, proof)?;
    jay(meter, here, ty, from, motive, base, to, proof)
}

fn binding(meter: &mut Meter, env: &Env, value: &Term, body: &Term) -> Result<Value, CoreError> {
    let value = eval(meter, env, value)?;
    eval(meter, &env.push(value), body)
}

/// The value with a solved metavariable at its head unfolded, or `None` when
/// there was nothing to unfold.
///
/// A neutral is blocked on its *head*, and the head of `?α x y .f` is `?α`. Once
/// that meta is solved the whole spine computes again, but the value already
/// built still says "blocked" — so every place that decides something by looking
/// at a value's shape has to ask here first. Returning `None` rather than a
/// clone keeps the common case, a value with no metavariable anywhere in it,
/// free.
///
/// # Errors
///
/// As [`eval`]: replaying the spine is ordinary evaluation.
pub(crate) fn force(meter: &mut Meter, value: &Value) -> Result<Option<Value>, CoreError> {
    let Form::Neutral(neutral) = &value.form else {
        return Ok(None);
    };
    if !head_is_solved(neutral) {
        return Ok(None);
    }
    replay(meter, neutral).map(Some)
}

/// Whether the innermost head of a spine is a metavariable that now has a
/// solution.
fn head_is_solved(neutral: &Neutral) -> bool {
    match &neutral.spine {
        Spine::Var(_, _) | Spine::Const(_) => false,
        Spine::Meta(meta) => meta.is_solved(),
        Spine::App { function, .. } => head_is_solved(function),
        Spine::Project { record, .. } => head_is_solved(record),
        Spine::J { proof, .. } => head_is_solved(proof),
    }
}

/// Re-run a blocked spine against a head that is no longer blocked.
fn replay(meter: &mut Meter, neutral: &Arc<Neutral>) -> Result<Value, CoreError> {
    let here = neutral.origin;
    match &neutral.spine {
        Spine::Var(_, _) | Spine::Const(_) => Ok(Value::shared_neutral(neutral)),
        Spine::Meta(meta) => Ok(meta
            .solution()
            .cloned()
            .unwrap_or_else(|| Value::shared_neutral(neutral))),
        Spine::App { function, argument } => {
            let function = replay(meter, function)?;
            apply(meter, here, function, Value::clone(argument))
        }
        Spine::Project { record, field } => {
            let record = replay(meter, record)?;
            project(meter, here, record, field)
        }
        Spine::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => {
            let proof = replay(meter, proof)?;
            jay(
                meter,
                here,
                Value::clone(ty),
                Value::clone(from),
                Value::clone(motive),
                Value::clone(base),
                Value::clone(to),
                proof,
            )
        }
    }
}

/// Open a closure at `argument`.
///
/// # Errors
///
/// As [`eval`].
pub(crate) fn apply_closure(meter: &mut Meter, closure: &Closure, argument: Value) -> Result<Value, CoreError> {
    eval(meter, &closure.env.push(argument), &closure.body)
}

/// β, or a blocked application.
///
/// `here` is the origin of the application itself, and is used only when the
/// application stays blocked: β answers the closure body, whose nodes carry
/// their own origins.
///
/// # Errors
///
/// [`Malformed::NotAFunction`] when `function` is neither a lambda nor neutral.
pub(crate) fn apply(meter: &mut Meter, here: Origin, function: Value, argument: Value) -> Result<Value, CoreError> {
    meter.step("function application")?;
    let function = force(meter, &function)?.unwrap_or(function);
    match function.form {
        Form::Lam(body) => apply_closure(meter, &body, argument),
        // A blocked application is where ι at an inductive family fires: the
        // recursor's target is its last argument, so this is the first moment the
        // elimination can know it has met a constructor.
        Form::Neutral(function) => {
            let built = Neutral {
                origin: here,
                spine: Spine::App {
                    function,
                    argument: Arc::new(argument),
                },
            };
            match crate::family::iota(meter, &built)? {
                Some(reduced) => Ok(reduced),
                None => Ok(Value::neutral(built)),
            }
        }
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. }
        | Form::Refl(_) => Err(Malformed::NotAFunction.into()),
    }
}

/// Projection, or a blocked projection.
///
/// # Errors
///
/// [`Malformed::NotARecord`] when `record` is neither a record nor neutral, and
/// [`Malformed::NoSuchField`] when it is a record without that field.
pub(crate) fn project(meter: &mut Meter, here: Origin, record: Value, field: &Name) -> Result<Value, CoreError> {
    meter.step("field projection")?;
    let record = force(meter, &record)?.unwrap_or(record);
    match record.form {
        Form::Record(fields) => fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| Malformed::NoSuchField(Arc::clone(field)).into()),
        Form::Neutral(record) => Ok(Value::neutral(Neutral {
            origin: here,
            spine: Spine::Project {
                record,
                field: Arc::clone(field),
            },
        })),
        Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::RecordType(_) | Form::Id { .. } | Form::Refl(_) => {
            Err(Malformed::NotARecord.into())
        }
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
    here: Origin,
    ty: Value,
    from: Value,
    motive: Value,
    base: Value,
    to: Value,
    proof: Value,
) -> Result<Value, CoreError> {
    meter.step("identity elimination")?;
    let proof = force(meter, &proof)?.unwrap_or(proof);
    match proof.form {
        Form::Refl(_) => Ok(base),
        Form::Neutral(proof) => Ok(Value::neutral(Neutral {
            origin: here,
            spine: Spine::J {
                ty: Arc::new(ty),
                from: Arc::new(from),
                motive: Arc::new(motive),
                base: Arc::new(base),
                to: Arc::new(to),
                proof,
            },
        })),
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. } => Err(Malformed::NotAnIdentity.into()),
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
        env = env.push(project(meter, subject.origin, subject.clone(), name)?);
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
/// The types this produces drive quotation rather than being quoted themselves,
/// so the origins it carries are those of the type's own terms — which is what
/// they should be, and why nothing here invents one.
///
/// # Errors
///
/// [`CoreError::Malformed`] when a neutral's head type does not admit the
/// elimination applied to it, which means the caller built a term the
/// elaborator would have refused.
pub(crate) fn neutral_type(meter: &mut Meter, neutral: &Neutral) -> Result<Value, CoreError> {
    meter.nested("neutral typing", |meter| match &neutral.spine {
        Spine::Var(_, ty) => Ok(Value::clone(ty)),
        // A meta is closed and carries its own type, which is why creating one
        // has to build that type rather than remember a context.
        Spine::Meta(meta) => Ok(meta.ty().clone()),
        // A constant's type is its declaration's, assembled on demand rather
        // than stored beside it — `family.rs` says why.
        Spine::Const(constant) => constant.ty(meter),
        Spine::App { function, argument } => match head_type(meter, function)?.form {
            Form::Pi { codomain, .. } => apply_closure(meter, &codomain, Value::clone(argument)),
            Form::Universe(_)
            | Form::Lam(_)
            | Form::RecordType(_)
            | Form::Record(_)
            | Form::Id { .. }
            | Form::Refl(_)
            | Form::Neutral(_) => Err(Malformed::NotAFunction.into()),
        },
        Spine::Project { record, field } => match head_type(meter, record)?.form {
            Form::RecordType(telescope) => {
                let subject = Value::shared_neutral(record);
                field_type(meter, &telescope, &subject, field)
            }
            Form::Universe(_)
            | Form::Pi { .. }
            | Form::Lam(_)
            | Form::Record(_)
            | Form::Id { .. }
            | Form::Refl(_)
            | Form::Neutral(_) => Err(Malformed::NotARecord.into()),
        },
        Spine::J { motive, to, proof, .. } => {
            let at_endpoint = apply(meter, neutral.origin, Value::clone(motive), Value::clone(to))?;
            apply(meter, neutral.origin, at_endpoint, Value::shared_neutral(proof))
        }
    })
}

/// The type of a blocked elimination's head, unfolded far enough to be matched
/// on.
///
/// [`neutral_type`] decides what an elimination is legal by matching its head's
/// type against [`Form::Pi`] or [`Form::RecordType`], and a type that was itself
/// written as a metavariable is [`Form::Neutral`] until that meta is solved.
/// Matching without forcing would answer [`Malformed::NotAFunction`] for a term
/// the elaborator had just proved well typed.
///
/// # Errors
///
/// As [`neutral_type`].
pub(crate) fn head_type(meter: &mut Meter, neutral: &Neutral) -> Result<Value, CoreError> {
    let ty = neutral_type(meter, neutral)?;
    Ok(force(meter, &ty)?.unwrap_or(ty))
}
