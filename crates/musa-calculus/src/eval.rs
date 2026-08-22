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
//! - **ι** is [`crate::family::iota`] on a recursor whose target is a
//!   constructor, which selects that constructor's method and applies it to the
//!   fields and one induction hypothesis per recursive field.
//! - **η** is *not* here. It is performed by [`crate::quote`], which is why
//!   quotation is type-directed and why two records with the same projections
//!   are convertible without a rule that inspects both at once.
//!
//! **Origins follow the value, not the use site** (§7). Evaluating a variable
//! answers whatever the environment holds, with the origin that value already
//! had, because §7 says provenance is preserved *by substitution*: in `e[a/x]`
//! the occurrences of `x` become `a`, and they carry `a`'s origins. The same
//! rule makes β answer the body's origins and δ answer the definition's. What
//! an elimination's own origin is for is the case where it stays blocked, and
//! that is why [`apply`] and [`project`] each take one.
//!
//! Every descent is charged and every level of it is metered (§4.1): `NbE` gives
//! the machine a second way to stand inside itself, and a total language may
//! refuse but may not crash.

use std::sync::Arc;

use crate::base::{Answer, Builtin, Datum};
use crate::budget::Meter;
use crate::context::Globals;
use crate::error::{CoreError, Malformed};
use crate::origin::Origin;
use crate::term::{Binder, Constant, Definition, Field, Filling, Name, Role, Shape, Term};
use crate::value::{Closure, DefHead, Elim, Env, Form, Head, Neutral, Telescope, Value};

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
            // A meta is closed, so the environment says nothing about it: it is
            // either its solution, with that solution's own origins (§7), or a
            // flexible head waiting for one.
            Shape::Meta(meta) => Ok(meta
                .solution()
                .cloned()
                .unwrap_or_else(|| Value::neutral(Neutral::head(here, Head::Meta(meta.clone()))))),
            // Resolved on the way in, so a value carries the level its arms
            // have already been solved to rather than the one written first.
            Shape::Universe(level) => Ok(Value::new(here, Form::Universe(*level))),
            // §1's one name node, resolved through the context (§6). What the
            // name reduces to is the table's answer and not the term's, which
            // is the whole of this arm.
            Shape::Named { name, role } => named(env, here, name, *role),
            // Nothing to do, and that is the point: a numeral of 384 is one node
            // here, so evaluating it charges one step and one nesting level
            // rather than 384 of each.
            Shape::Lit(Constant::Payload(literal)) => Ok(Value::new(here, Form::Lit(literal.clone()))),
            Shape::Lit(Constant::Numeral(numeral)) => Ok(Value::new(here, Form::Numeral(numeral.clone()))),
            // §1's one binder node, read three ways. The written form shares a
            // constructor; the value forms do not, because a Π and a λ are told
            // apart by what eliminates them and nothing eliminates a `let`.
            Shape::Bind { name, binder, body } => match binder {
                Binder::Pi { filling, ty } => pi(meter, env, here, filling.clone(), name, ty, body),
                Binder::Lam => Ok(Value::new(
                    here,
                    Form::Lam(Closure {
                        env: env.clone(),
                        body: body.clone(),
                    }),
                )),
                Binder::Let { ty: _, value } => binding(meter, env, value, body),
            },
            Shape::App { function, argument } => application(meter, env, here, function, argument),
            // An indexed type evaluates both halves and reduces neither: there is
            // no ι, no δ, and no β at one, because §1.5 gives it no elimination
            // form. It is carried so that conversion can ask about it, and
            // dropped by `quote` so that nothing downstream ever sees it.
            Shape::Indexed { ty, index } => Ok(Value::new(
                here,
                Form::Indexed {
                    ty: Arc::new(eval(meter, env, ty)?),
                    index: Arc::new(eval(meter, env, index)?),
                },
            )),
            Shape::RecordType(fields) => Ok(Value::new(
                here,
                Form::RecordType(Telescope {
                    fields: Arc::clone(fields),
                    env: env.clone(),
                }),
            )),
            Shape::Record(fields) => literal(meter, env, here, fields),
            Shape::Project { record, field } => projection(meter, env, here, record, field),
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

/// What a name stands for here (§1, §6).
///
/// Five answers and one refusal, and each is what the removed variant did:
///
/// - a **declared constant** is closed and rigid, so evaluating one is reading
///   it. ι does not fire here: it needs the target, which arrives through
///   [`apply`]. A counting family's floor is the one constant that is not rigid
///   — [`Constant::value`](crate::family::Constant) turns it into the numeral
///   zero, which is where that collapse lives so that it cannot be done twice
///   or forgotten once.
/// - a **definition** is δ *deferred*: the use evaluates to a folded neutral
///   carrying the value computed once at the declaration, and [`unfold`] opens
///   it where something needs it open. The origins inside it are the
///   definition's own, which is §7 working — the value came from where it was
///   written, not from where it was named.
/// - a **base type** is rigid forever, and a **builtin** is rigid until its
///   arguments are literals, which is a question [`apply`] asks once the spine
///   is long enough (§5.8).
///
/// The three rigid heads keep the table they were resolved under, because
/// [`neutral_type`] answers their types by evaluating a declaration term and
/// has no context to ask — see [`Head::Base`].
///
/// # Errors
///
/// [`Malformed::UndeclaredName`] when nothing in scope answers to the name. The
/// elaborator resolved it once already, so this is a defect in whoever built or
/// moved the term — never a silent resolution to a different declaration of the
/// same spelling.
fn named(env: &Env, here: Origin, name: &Name, role: Role) -> Result<Value, CoreError> {
    let globals = env.globals();
    match globals.definition(name, role) {
        Definition::Declared(constant) => Ok(constant.value(here, globals)),
        Definition::Defined(def) => Ok(Value::neutral(Neutral::head(
            here,
            Head::Def(DefHead::Global(def.clone()), def.ty(), def.value()),
        ))),
        Definition::Base(base) => Ok(Value::neutral(Neutral::head(here, Head::Base(base, globals.clone())))),
        Definition::Builtin(builtin) => Ok(Value::neutral(Neutral::head(
            here,
            Head::Builtin(builtin, globals.clone()),
        ))),
        Definition::Undeclared => Err(Malformed::UndeclaredName(Arc::clone(name)).into()),
    }
}

fn pi(
    meter: &mut Meter,
    env: &Env,
    here: Origin,
    filling: Filling,
    name: &Name,
    domain: &Term,
    codomain: &Term,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        here,
        Form::Pi {
            filling,
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

fn binding(meter: &mut Meter, env: &Env, value: &Term, body: &Term) -> Result<Value, CoreError> {
    let value = eval(meter, env, value)?;
    eval(meter, &env.push(value), body)
}

/// The value with a solved meta at its head seen through, or `None` when the
/// head is not one.
///
/// A neutral is blocked on its *head*, and the head of `?α x y .f` is `?α`. Once
/// that meta is solved the whole spine computes again, but the value already
/// built still says "blocked" — so every place that decides something by looking
/// at a value's shape has to ask here first. Returning `None` rather than a
/// clone keeps the common case, a value with no meta anywhere in it, free.
///
/// A loop rather than a step: a solution can itself be headed by a meta that
/// has since been solved, and a caller that trusted one step would read a
/// solved meta as an unsolved one — in [`crate::convert`] that is not a missed
/// reduction but a wrong answer. Each pass is charged, so a chain is bounded
/// by the budget rather than by a claim that chains are short.
///
/// # Errors
///
/// As [`eval`]: replaying the spine is ordinary evaluation.
pub(crate) fn force(meter: &mut Meter, value: &Value) -> Result<Option<Value>, CoreError> {
    let Form::Neutral(neutral) = &value.form else {
        return Ok(None);
    };
    let Head::Meta(meta) = &neutral.head else {
        return Ok(None);
    };
    let Some(solution) = meta.solution().cloned() else {
        return Ok(None);
    };
    let mut answer = replay(meter, solution, &neutral.spine)?;
    loop {
        let Form::Neutral(blocked) = &answer.form else {
            return Ok(Some(answer));
        };
        let Head::Meta(meta) = &blocked.head else {
            return Ok(Some(answer));
        };
        let Some(solution) = meta.solution().cloned() else {
            return Ok(Some(answer));
        };
        let blocked = Arc::clone(blocked);
        meter.step("forcing")?;
        answer = replay(meter, solution, &blocked.spine)?;
    }
}

/// A solution with the blocked spine re-run over it, innermost first — the
/// order the spine is stored in.
fn replay(meter: &mut Meter, solution: Value, spine: &[Elim]) -> Result<Value, CoreError> {
    let mut answer = solution;
    for elimination in spine {
        answer = eliminate(meter, answer, elimination)?;
    }
    Ok(answer)
}

/// The value with a folded definition at its head unfolded, or `None` when
/// the head is not a definition.
///
/// δ on demand — the one operation all five kinds of forcing site share. The
/// definition's value was computed once at the declaration; unfolding replays
/// the spine over it, which is [`replay`] for a head that was never a
/// metavariable. Neither the unfold nor the replay carries a bookkeeping
/// charge: evaluation charged each elimination when it entered the spine, and
/// charging again here would count every application of a definition twice —
/// what changed is *when* the work runs, not what it costs. Termination needs
/// no meter here: a definition's value names only what was declared before it
/// (a local's carried heads sit at strictly smaller levels, a global's at
/// earlier declarations, and a recursive definition's self-reference stands
/// under a λ), so an unfold chain is finite, and the real work the replay
/// runs — β bodies, ι steps, builtin rules — is charged by itself.
///
/// # Errors
///
/// As [`eval`]: replaying the spine is ordinary evaluation.
pub(crate) fn unfold(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Value>, CoreError> {
    let Head::Def(_, _, value) = &neutral.head else {
        return Ok(None);
    };
    Ok(Some(unfold_spine(meter, value, &neutral.spine)?))
}

/// [`unfold`] with the head question already answered: the caller matched the
/// `Def` head itself, so the value and the spine are what it holds.
pub(crate) fn unfold_spine(meter: &mut Meter, value: &Value, spine: &[Elim]) -> Result<Value, CoreError> {
    let mut answer = value.clone();
    // Innermost first, which is the order the spine is stored in.
    for elimination in spine {
        answer = eliminate_replayed(meter, answer, elimination)?;
    }
    Ok(answer)
}

/// One elimination replayed over a definition's value at an unfold, uncharged
/// — see [`unfold`]'s doc for the accounting. [`eliminate`] is the charging
/// twin the metavariable replay keeps, because a spine behind a metavariable
/// is charged when it is built *and* its replay is where the waiting work
/// finally runs; a spine behind a definition waited behind nothing.
fn eliminate_replayed(meter: &mut Meter, target: Value, elimination: &Elim) -> Result<Value, CoreError> {
    match elimination {
        Elim::App { origin, argument } => applying(meter, *origin, target, Value::clone(argument)),
        Elim::Project { origin, field } => projecting(meter, *origin, target, field),
    }
}

/// The value seen through solved metavariables *and* folded definitions at
/// its head, or `None` when there was nothing to see through.
///
/// The fixed point of [`force`] and [`unfold`], and the operation every place
/// that asks "is this a canonical form yet" takes: conversion on a folded
/// disagreement, ι at a recursor target, δ at a builtin's arguments, quotation
/// in the opening mode, and the elaborator wherever it already forced a value
/// before matching its form. [`force`] alone remains what conversion's folded
/// comparison and quotation's keeping mode use, because both exist to *not*
/// open definitions.
///
/// The loop, as [`force`]'s: unfolding a definition can answer a value headed
/// by another one (a definition whose value is an earlier definition), and
/// the postcondition is the fixed point — the head of what comes back is
/// neither a solved metavariable nor a folded definition.
///
/// # Errors
///
/// As [`force`] and [`unfold`].
pub(crate) fn opened(meter: &mut Meter, value: &Value) -> Result<Option<Value>, CoreError> {
    let mut answer = match force(meter, value)? {
        Some(forced) => forced,
        None => match &value.form {
            // Nothing to open. An indexed type holds no meta at its head and no
            // definition to unfold: §1.5 gives it no reduction at all.
            Form::Indexed { .. } => return Ok(None),
            Form::Neutral(neutral) => match unfold(meter, neutral)? {
                Some(unfolded) => unfolded,
                None => return Ok(None),
            },
            Form::Universe(_)
            | Form::Pi { .. }
            | Form::Lam(_)
            | Form::RecordType(_)
            | Form::Record(_)
            | Form::Lit(_)
            | Form::Numeral(_) => return Ok(None),
        },
    };
    loop {
        if let Some(forced) = force(meter, &answer)? {
            answer = forced;
            continue;
        }
        let Form::Neutral(neutral) = &answer.form else {
            return Ok(Some(answer));
        };
        match unfold(meter, neutral)? {
            Some(unfolded) => answer = unfolded,
            None => return Ok(Some(answer)),
        }
    }
}

/// Apply one elimination to a value that is no longer blocked.
fn eliminate(meter: &mut Meter, target: Value, elimination: &Elim) -> Result<Value, CoreError> {
    match elimination {
        Elim::App { origin, argument } => apply(meter, *origin, target, Value::clone(argument)),
        Elim::Project { origin, field } => project(meter, *origin, target, field),
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
    applying(meter, here, function, argument)
}

/// [`apply`] without the bookkeeping charge, for a spine being replayed at an
/// unfold: the elimination was charged when it entered the spine, and charging
/// the replay as well would count every application of a definition twice.
/// The work inside — a β body, an ι step, a builtin's rule — carries its own
/// charges either way.
fn applying(meter: &mut Meter, here: Origin, function: Value, argument: Value) -> Result<Value, CoreError> {
    match function.form {
        Form::Lam(body) => apply_closure(meter, &body, argument),
        // Not a function, and not applied to anything: `Row(12) x` is what a
        // caller wrote when it meant `Row x`, and this is where it says so.
        Form::Indexed { .. } => Err(Malformed::NotAFunction.into()),
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
            // The other direction: not an elimination firing but a construction
            // collapsing, so that a counting family's values stay numerals and
            // never accumulate a spine. Here rather than in `eval` because a
            // constructor meets its argument at an application and nowhere else.
            if let Some(counted) = crate::family::stepped(meter, &built)? {
                return Ok(counted);
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
        | Form::Lit(_)
        | Form::Numeral(_) => Err(Malformed::NotAFunction.into()),
    }
}

/// δ at a compiler-owned builtin, or `None` when the spine is not ready.
///
/// The mirror of [`crate::family::iota`], in the same arm and for the same
/// reason: an application is the first moment a rule can know its last argument
/// has arrived. A builtin fires when three things hold at once — the head is a
/// builtin, the spine is exactly its arity of applications, and every argument
/// has reduced to **canonical data**. Any one of them failing leaves an ordinary
/// blocked spine, which is what a builtin applied to a variable *is*.
///
/// Canonical data rather than a literal, because §5.8's D1 admits "a base type
/// **or a finite constructor over base types**" and a host that answers an
/// `Option` or takes a `List` is writing the second. The two conditions coincide
/// wherever no declared family is involved, so a table of base-typed rules
/// behaves exactly as it did.
///
/// The meter is charged before the rule runs, which is where D4's "charged to
/// the §4 meter before construction begins" can actually be enforced: after the
/// fact, the result already exists.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`Malformed::BuiltinStuck`] when
/// the rule answers nothing at closed data — D2 broken, which is a defect in the
/// host's table rather than in the program — and [`Malformed::MisfitAnswer`]
/// when what it answers does not fit its own declared result type.
fn delta(meter: &mut Meter, built: &Neutral) -> Result<Option<Value>, CoreError> {
    let Head::Builtin(builtin, globals) = &built.head else {
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
        let Some(datum) = canonical(meter, argument)? else {
            return Ok(None);
        };
        arguments.push(datum);
    }
    meter.step("builtin reduction")?;
    let Some(answer) = rule(&arguments) else {
        return Err(Malformed::BuiltinStuck(Arc::clone(builtin.name())).into());
    };
    let here = built.outer_origin();
    // §4's first outcome, reaching the one place that used to collapse it into
    // the third. The rule said the sentence and this says where: the origin of
    // the application that fired, which is the node the composer wrote.
    let answer = match answer {
        Answer::Reduced(datum) => datum,
        Answer::Refused(message) => return Err(CoreError::Refused { message, at: here }),
    };
    match answer {
        // The overwhelmingly common answer, and it needs no type: a literal
        // carries its own. Only a constructed answer pays for the walk below.
        Datum::Lit(literal) => Ok(Some(Value::new(here, Form::Lit(literal)))),
        Datum::Count { .. } | Datum::Case { .. } => {
            let ty = result_type(meter, builtin, globals, built)?;
            crate::family::realize(meter, here, globals, &answer, &ty).map(Some)
        }
    }
}

/// `value` as canonical data, or `None` when it is not data at all.
///
/// §5.8's D1 turned into a predicate: a literal, or a constructor of a declared
/// family applied to more of the same. A record, a λ, a universe, or a
/// constructor one field short is none of those and leaves the spine blocked,
/// which is what a δ-builtin over an open term must do.
///
/// Forcing first is not optional. A metavariable that has since been solved may
/// have data behind it, and a builtin that ignored that would answer "blocked"
/// for an argument the program has already determined.
///
/// Needs no type, where [`crate::family::realize`] does: canonical data is
/// canonical, so looking says which shape it is, and η — the one thing that
/// makes reading back type-directed — has nothing to expand here.
///
/// # Errors
///
/// As [`force`], plus §4.1's nesting limit at data nested deeper than the meter
/// allows.
fn canonical(meter: &mut Meter, value: &Value) -> Result<Option<Datum>, CoreError> {
    meter.nested("canonical data", |meter| {
        let forced = opened(meter, value)?;
        match forced.as_ref().unwrap_or(value).form {
            // A type, not data. See `crate::family::canonical`.
            Form::Indexed { .. } => Ok(None),
            Form::Lit(ref literal) => Ok(Some(Datum::Lit(literal.clone()))),
            // The count read back as the tower it stands for. See
            // [`crate::family::canonical`], which is the same answer one layer
            // up and carries the argument for building it with a loop.
            Form::Numeral(ref numeral) => Ok(crate::family::counted(numeral)),
            Form::Neutral(ref neutral) => constructed(meter, neutral),
            Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::RecordType(_) | Form::Record(_) => Ok(None),
        }
    })
}

/// A blocked spine as canonical data, when it is a saturated constructor whose
/// fields are themselves data.
fn constructed(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Datum>, CoreError> {
    let Some((constructor, params)) = crate::family::constructed(neutral) else {
        return Ok(None);
    };
    let mut fields = Vec::with_capacity(neutral.spine.len().saturating_sub(params));
    for elimination in neutral.spine.iter().skip(params) {
        let Elim::App { argument, .. } = elimination else {
            return Ok(None);
        };
        let Some(field) = canonical(meter, argument)? else {
            return Ok(None);
        };
        fields.push(field);
    }
    Ok(Some(Datum::Case { constructor, fields }))
}

/// The type a builtin answers at, given the arguments on its spine.
///
/// Its declared signature, opened one Π per argument. Computed here rather than
/// stored because it is needed only by an answer that names a constructor, and
/// because the signature and the answer must not be able to disagree about what
/// the result type is — they are the same term either way.
///
/// # Errors
///
/// [`Malformed::NotAFunction`] when the signature runs out of Π before the spine
/// runs out of arguments, which means the arity and the type disagree.
fn result_type(meter: &mut Meter, builtin: &Builtin, globals: &Globals, built: &Neutral) -> Result<Value, CoreError> {
    let mut ty = eval(meter, &Env::under(globals.clone()), builtin.ty())?;
    for elimination in &built.spine {
        let Elim::App { argument, .. } = elimination else {
            return Err(Malformed::NotAFunction.into());
        };
        let Form::Pi { codomain, .. } = ty.form else {
            return Err(Malformed::NotAFunction.into());
        };
        ty = apply_closure(meter, &codomain, Value::clone(argument))?;
    }
    Ok(ty)
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
    let Head::Builtin(builtin, globals) = &built.head else {
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
    let forced = opened(meter, subject)?;
    let rewritten = match forced.as_ref().unwrap_or(subject).form {
        // A structural rule rewrites a *literal*; an indexed type is a type, and
        // no builtin's target position holds one.
        Form::Indexed { .. } => None,
        Form::Lit(ref literal) => {
            meter.step("structural reduction")?;
            rewrite(builtin, literal)
        }
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Numeral(_)
        | Form::Neutral(_) => return Ok(None),
    };
    let Some(rewritten) = rewritten else {
        return Err(Malformed::BuiltinStuck(Arc::clone(builtin.name())).into());
    };
    // Innermost last, so index 0 is the last argument — the order a telescope of
    // binders over the same spine would have produced.
    let env = arguments
        .into_iter()
        .fold(Env::under(globals.clone()), |env, argument| env.push(argument));
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
    projecting(meter, here, record, field)
}

/// [`project`] without the bookkeeping charge — see [`applying`].
fn projecting(_meter: &mut Meter, here: Origin, record: Value, field: &Name) -> Result<Value, CoreError> {
    match record.form {
        // Not a record, so there is no field to find — the same answer a
        // universe or a λ gets below.
        Form::Indexed { .. } => Err(Malformed::NotARecord.into()),
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
        Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::RecordType(_) | Form::Lit(_) | Form::Numeral(_) => {
            Err(Malformed::NotARecord.into())
        }
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
            // The type travels in the head, as it does for a variable.
            Head::Def(_, ty, _) => Value::clone(ty),
            // A constant's type is its declaration's, assembled on demand
            // rather than stored beside it — `family.rs` says why.
            Head::Const(constant, globals) => constant.ty(meter, globals)?,
            // A base type's kind and a builtin's signature are closed in
            // *binders* and not in names, so the environment they are read in
            // has no locals and the table the head was resolved under.
            Head::Base(base, globals) => eval(meter, &Env::under(globals.clone()), base.kind())?,
            Head::Builtin(builtin, globals) => eval(meter, &Env::under(globals.clone()), builtin.ty())?,
        };
        // The prefix each elimination is applied to, grown in place. A
        // projection's field type may mention the record it projects from, and
        // `J`'s result type mentions the proof, so the walk has to be able to
        // name what it has consumed so far.
        let mut prefix = Neutral::head(neutral.origin, neutral.head.clone());
        for elimination in &neutral.spine {
            // A type written as a metavariable is blocked until that meta is
            // solved, and one written as a definition stays folded until it is
            // opened; matching it as either would answer `NotAFunction` for a
            // term the elaborator had just proved well typed.
            let head = opened(meter, &ty)?.unwrap_or(ty);
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
            | Form::Lit(_)
            | Form::Numeral(_)
            | Form::Indexed { .. }
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
            | Form::Lit(_)
            | Form::Numeral(_)
            | Form::Indexed { .. }
            | Form::Neutral(_) => Err(Malformed::NotARecord.into()),
        },
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
    // Opened rather than merely forced: the answer is matched against `Π` and
    // record types, and a type that names a definition hides both while folded.
    Ok(opened(meter, &ty)?.unwrap_or(ty))
}
