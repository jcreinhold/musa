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
use crate::value::{Closure, Elim, Env, Form, Head, Neutral, Telescope, Value};

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
            // §5.8's extension. A base type is rigid forever — nothing
            // eliminates it — and a builtin is rigid until its arguments are
            // literals, which is a question [`apply`] asks once the spine is
            // long enough. A literal is already canonical.
            Shape::Base(base) => Ok(Value::neutral(Neutral::head(here, Head::Base(base.clone())))),
            Shape::Builtin(builtin) => Ok(Value::neutral(Neutral::head(here, Head::Builtin(builtin.clone())))),
            Shape::Lit(literal) => Ok(Value::new(here, Form::Lit(literal.clone()))),
            // A meta is closed, so the environment says nothing about it: it is
            // either its solution, with that solution's own origins (§7), or a
            // flexible head waiting for one.
            Shape::Meta(meta) => Ok(meta
                .solution()
                .cloned()
                .unwrap_or_else(|| Value::neutral(Neutral::head(here, Head::Meta(meta.clone()))))),
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

/// Whether the head of a spine is a metavariable that now has a solution.
///
/// One field read rather than a walk to the deepest node: that is the whole
/// point of storing the head beside the spine instead of under it.
fn head_is_solved(neutral: &Neutral) -> bool {
    match &neutral.head {
        Head::Var(_, _) | Head::Const(_) | Head::Base(_) | Head::Builtin(_) => false,
        Head::Meta(meta) => meta.is_solved(),
    }
}

/// Re-run a blocked spine against a head that is no longer blocked.
fn replay(meter: &mut Meter, neutral: &Arc<Neutral>) -> Result<Value, CoreError> {
    let Head::Meta(meta) = &neutral.head else {
        return Ok(Value::shared_neutral(neutral));
    };
    let Some(solution) = meta.solution().cloned() else {
        return Ok(Value::shared_neutral(neutral));
    };
    let mut answer = solution;
    // Innermost first, which is the order the spine is stored in: `?α x .f`
    // applies before it projects.
    for elimination in &neutral.spine {
        answer = eliminate(meter, answer, elimination)?;
    }
    Ok(answer)
}

/// Apply one elimination to a value that is no longer blocked.
fn eliminate(meter: &mut Meter, target: Value, elimination: &Elim) -> Result<Value, CoreError> {
    match elimination {
        Elim::App { origin, argument } => apply(meter, *origin, target, Value::clone(argument)),
        Elim::Project { origin, field } => project(meter, *origin, target, field),
        Elim::J {
            origin,
            ty,
            from,
            motive,
            base,
            to,
        } => jay(
            meter,
            *origin,
            Value::clone(ty),
            Value::clone(from),
            Value::clone(motive),
            Value::clone(base),
            Value::clone(to),
            target,
        ),
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
            let built = Neutral::eliminated(
                &function,
                Elim::App {
                    origin: here,
                    argument: Arc::new(argument),
                },
            );
            if let Some(reduced) = crate::family::iota(meter, &built)? {
                return Ok(reduced);
            }
            if let Some(reduced) = delta(meter, &built)? {
                return Ok(reduced);
            }
            match structural(meter, &built)? {
                Some(reduced) => Ok(reduced),
                None => Ok(Value::neutral(built)),
            }
        }
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. }
        | Form::Refl(_)
        | Form::Lit(_) => Err(Malformed::NotAFunction.into()),
    }
}

/// δ at a compiler-owned builtin, or `None` when the spine is not ready.
///
/// The mirror of [`crate::family::iota`], in the same arm and for the same
/// reason: an application is the first moment a rule can know its last argument
/// has arrived. A builtin fires when three things hold at once — the head is a
/// builtin, the spine is exactly its arity of applications, and every argument
/// has reduced to a literal. Any one of them failing leaves an ordinary blocked
/// spine, which is what a builtin applied to a variable *is*.
///
/// The meter is charged before the rule runs, which is where D4's "charged to
/// the §4 meter before construction begins" can actually be enforced: after the
/// fact, the result already exists.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, and [`Malformed::BuiltinStuck`]
/// when the rule answers nothing at closed literal arguments — D2 broken, which
/// is a defect in the host's table rather than in the program.
fn delta(meter: &mut Meter, built: &Neutral) -> Result<Option<Value>, CoreError> {
    let Head::Builtin(builtin) = &built.head else {
        return Ok(None);
    };
    let Some(rule) = builtin.delta_rule() else {
        return Ok(None);
    };
    if built.spine.len() != builtin.arity() {
        return Ok(None);
    }
    let mut arguments = Vec::with_capacity(built.spine.len());
    for elimination in &built.spine {
        let Elim::App { argument, .. } = elimination else {
            return Ok(None);
        };
        // A metavariable that has since been solved may have a literal behind
        // it, and a builtin that ignored that would answer "blocked" for an
        // argument the program has already determined.
        let forced = force(meter, argument)?;
        match forced.as_ref().unwrap_or(argument).form {
            Form::Lit(ref literal) => arguments.push(literal.clone()),
            Form::Universe(_)
            | Form::Pi { .. }
            | Form::Lam(_)
            | Form::RecordType(_)
            | Form::Record(_)
            | Form::Id { .. }
            | Form::Refl(_)
            | Form::Neutral(_) => return Ok(None),
        }
    }
    meter.step("builtin reduction")?;
    let borrowed: Vec<&crate::base::Literal> = arguments.iter().collect();
    match rule(&borrowed) {
        Some(answer) => Ok(Some(Value::new(built.outer_origin(), Form::Lit(answer)))),
        None => Err(Malformed::BuiltinStuck(Arc::clone(builtin.name())).into()),
    }
}

/// A structural eliminator's step, or `None` when the spine is not ready.
///
/// §5.8's second family, in the same arm as [`delta`] and [`crate::family::iota`]
/// and for the same reason. It differs from δ in the two ways a traversal
/// differs from an arithmetic operation:
///
/// - **it fires on its target, not on all of its arguments.** δ waits for every
///   argument to be a literal, because a first-order function needs them all. A
///   traversal's other arguments are the algebra — functions, which never become
///   literals — so it waits for the one argument the registration declared, and
///   passes the rest through untouched. That is ι's condition, which asks about
///   the recursor's target and nothing about its methods.
/// - **it answers a term, which this evaluates.** The rewrite writes down the
///   next step in the traversal, and the arguments it names by position are
///   bound to the values already on the spine. Nothing is re-evaluated and
///   nothing is quoted, so a function argument is never forced — a traversal
///   that needed one forced would be asking for a strictness the calculus does
///   not have.
///
/// The meter is charged before the rewrite runs, D4's reason again, and it is
/// also the backstop for a rewrite that does not descend: a rule that reapplied
/// its builtin to the same literal would exhaust the budget rather than hang.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`Malformed::BuiltinStuck`] when
/// the rewrite answers nothing at a literal target and a full spine, and
/// whatever evaluating the answer answers.
fn structural(meter: &mut Meter, built: &Neutral) -> Result<Option<Value>, CoreError> {
    let Head::Builtin(builtin) = &built.head else {
        return Ok(None);
    };
    let Some((target, rewrite)) = builtin.structural_rule() else {
        return Ok(None);
    };
    if built.spine.len() != builtin.arity() {
        return Ok(None);
    }
    let mut arguments = Vec::with_capacity(built.spine.len());
    for elimination in &built.spine {
        let Elim::App { argument, .. } = elimination else {
            return Ok(None);
        };
        arguments.push(Value::clone(argument));
    }
    // Registration checked that the target names an argument, so this indexes a
    // spine of exactly the arity.
    let Some(subject) = arguments.get(target) else {
        return Ok(None);
    };
    let forced = force(meter, subject)?;
    let rewritten = match forced.as_ref().unwrap_or(subject).form {
        Form::Lit(ref literal) => {
            meter.step("structural reduction")?;
            rewrite(builtin, literal)
        }
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. }
        | Form::Refl(_)
        | Form::Neutral(_) => return Ok(None),
    };
    let Some(rewritten) = rewritten else {
        return Err(Malformed::BuiltinStuck(Arc::clone(builtin.name())).into());
    };
    // Innermost last, so index 0 is the last argument — the order a telescope of
    // binders over the same spine would have produced.
    let env = arguments
        .into_iter()
        .fold(Env::EMPTY, |env, argument| env.push(argument));
    eval(meter, &env, &rewritten).map(Some)
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
        Form::Neutral(record) => Ok(Value::neutral(Neutral::eliminated(
            &record,
            Elim::Project {
                origin: here,
                field: Arc::clone(field),
            },
        ))),
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Id { .. }
        | Form::Refl(_)
        | Form::Lit(_) => Err(Malformed::NotARecord.into()),
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
        Form::Neutral(proof) => Ok(Value::neutral(Neutral::eliminated(
            &proof,
            Elim::J {
                origin: here,
                ty: Arc::new(ty),
                from: Arc::new(from),
                motive: Arc::new(motive),
                base: Arc::new(base),
                to: Arc::new(to),
            },
        ))),
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Id { .. }
        | Form::Lit(_) => Err(Malformed::NotAnIdentity.into()),
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
    meter.nested("neutral typing", |meter| {
        let mut ty = match &neutral.head {
            Head::Var(_, ty) => Value::clone(ty),
            // A meta is closed and carries its own type, which is why creating
            // one has to build that type rather than remember a context.
            Head::Meta(meta) => meta.ty().clone(),
            // A constant's type is its declaration's, assembled on demand
            // rather than stored beside it — `family.rs` says why.
            Head::Const(constant) => constant.ty(meter)?,
            // A base type's kind and a builtin's signature are closed terms the
            // host registered, so the empty environment is the whole context
            // either needs.
            Head::Base(base) => eval(meter, &Env::EMPTY, base.kind())?,
            Head::Builtin(builtin) => eval(meter, &Env::EMPTY, builtin.ty())?,
        };
        // The prefix each elimination is applied to, grown in place. A
        // projection's field type may mention the record it projects from, and
        // `J`'s result type mentions the proof, so the walk has to be able to
        // name what it has consumed so far.
        let mut prefix = Neutral::head(neutral.origin, neutral.head.clone());
        for elimination in &neutral.spine {
            // A type written as a metavariable is blocked until that meta is
            // solved; matching it unforced would answer `NotAFunction` for a
            // term the elaborator had just proved well typed.
            let head = force(meter, &ty)?.unwrap_or(ty);
            ty = eliminated_type(meter, head, &prefix, elimination)?;
            prefix.spine.push(elimination.clone());
        }
        Ok(ty)
    })
}

/// The type of `prefix` eliminated by `elimination`, given the prefix's own
/// type already forced.
fn eliminated_type(meter: &mut Meter, head: Value, prefix: &Neutral, elimination: &Elim) -> Result<Value, CoreError> {
    match elimination {
        Elim::App { argument, .. } => match head.form {
            Form::Pi { codomain, .. } => apply_closure(meter, &codomain, Value::clone(argument)),
            Form::Universe(_)
            | Form::Lam(_)
            | Form::RecordType(_)
            | Form::Record(_)
            | Form::Id { .. }
            | Form::Refl(_)
            | Form::Lit(_)
            | Form::Neutral(_) => Err(Malformed::NotAFunction.into()),
        },
        Elim::Project { field, .. } => match head.form {
            Form::RecordType(telescope) => {
                let subject = Value::neutral(prefix.clone());
                field_type(meter, &telescope, &subject, field)
            }
            Form::Universe(_)
            | Form::Pi { .. }
            | Form::Lam(_)
            | Form::Record(_)
            | Form::Id { .. }
            | Form::Refl(_)
            | Form::Lit(_)
            | Form::Neutral(_) => Err(Malformed::NotARecord.into()),
        },
        Elim::J { origin, motive, to, .. } => {
            let at_endpoint = apply(meter, *origin, Value::clone(motive), Value::clone(to))?;
            apply(meter, *origin, at_endpoint, Value::neutral(prefix.clone()))
        }
    }
}

/// The type of a blocked elimination, unfolded far enough to be matched on.
///
/// A caller decides what an elimination is legal by matching this against
/// [`Form::Pi`] or [`Form::RecordType`], and a type that was itself written as a
/// metavariable is [`Form::Neutral`] until that meta is solved. Matching without
/// forcing would answer [`Malformed::NotAFunction`] for a term the elaborator
/// had just proved well typed.
///
/// # Errors
///
/// As [`neutral_type`].
pub(crate) fn head_type(meter: &mut Meter, neutral: &Neutral) -> Result<Value, CoreError> {
    let ty = neutral_type(meter, neutral)?;
    Ok(force(meter, &ty)?.unwrap_or(ty))
}
