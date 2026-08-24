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
//! - **ι** is [`crate::kernel::family::iota`] on a recursor whose target is a
//!   constructor, which selects that constructor's method and applies it to the
//!   fields and one induction hypothesis per recursive field.
//! - **η** is *not* here. It is performed by [`crate::kernel::quote`], which is why
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

use std::sync::{Arc, OnceLock};

use crate::kernel::base::{Answer, Builtin, Datum};
use crate::kernel::budget::{Meter, Stamp};
use crate::kernel::case_tree::Matched;
use crate::kernel::context::Globals;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::family::{Fired, Pending, Reduction};
use crate::kernel::meta::Meta;
use crate::kernel::origin::Origin;
use crate::kernel::sort::Levels;
use crate::kernel::term::{Binder, Constant, Definition, Filling, Name, Role, Shape, Term};
use crate::kernel::value::{Arg, Closure, DefHead, Delay, Elim, Env, Folding, Form, Head, Neutral, Value};

/// Evaluate `term` in `env`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the term's shape makes the next step meaningless.
pub(crate) fn eval(meter: &mut Meter, env: &Env, term: &Term) -> Result<Value, CoreError> {
    run(meter, Step::Term(env.clone(), term.clone()))
}

/// What the evaluator does next.
///
/// Three of the four are a value arriving somewhere; the first is the only one
/// that reads a term. Together with [`Frame`] they are the evaluator's whole
/// state, which is the point — see [`run`].
enum Step {
    /// Evaluate this term in this environment.
    Term(Env, Term),
    /// Hand this value to the frame on top of the control stack, or answer with
    /// it when there is none.
    Value(Value),
    /// Apply `function` to `argument`, with the bookkeeping charge already made
    /// (or deliberately not made — see [`applying`]).
    Apply {
        here: Origin,
        function: Value,
        argument: Arg,
    },
    /// Look through this value's head: solved metavariables always, folded
    /// definitions when `definitions`.
    Open { value: Value, definitions: bool },
}

/// One piece of work the evaluator is in the middle of.
///
/// # Why this is data and not a Rust frame
///
/// `docs/rules/language/02-core-calculus.md` §4.1 derives the nesting metric
/// from how deeply a *term* is written and how far `quote` descends over a
/// value. Neither clause mentions recursion, and a tree-walking evaluator
/// charges it anyway: `f` calling itself is evaluated *inside* the enclosing
/// evaluation, so the enclosing level is held until the steps beneath it
/// finish, and one number then decides both how deeply a composer may write a
/// term and how many times a definition may call itself. Prompt 165a retires
/// that, and the only way to retire it is to stop spending host stack on the
/// recursion: a charge removed without the frames removed would turn a refusal
/// into an abort, which is the one outcome §4 does not have.
///
/// So the pending work lives here. Peyton Jones ch. 11 §11.6 is the shape —
/// the applications a reducer is in the middle of are a stack in the machine's
/// own store rather than a chain of host frames — and ch. 18 §18.8's *dump* is
/// the second half, a nested evaluation returning without a host call. Musa
/// needs neither the graph nor the thunk that come with them: the core is
/// strict, finite and total, so what transfers is the control state and
/// nothing else.
///
/// **What bounds it.** Every frame pushed is pushed on the way through
/// [`Step::Term`] or an elimination, and both charge the step meter, so a
/// runaway recursion is exhausted at `reduction steps` — the counter that
/// measures work done, which is what a recursion spends. The stack is heap
/// memory bounded by that count, not host stack bounded by nothing.
enum Frame {
    /// A nested evaluation is running; this is the level the enclosing one
    /// stands at, to be stood at again when the nested one answers.
    ///
    /// Peyton Jones ch. 18 §18.8's *dump*. See [`Meter::at`].
    Dump { level: u64 },
    /// An application whose function is being evaluated; its argument waits.
    Argument { env: Env, argument: Term, here: Origin },
    /// An application whose argument is being evaluated; its function waits.
    Applied { function: Value, here: Origin },
    /// A `let` whose value is being evaluated; its body waits.
    Body { env: Env, body: Term },
    /// A Π whose domain is being evaluated; its codomain waits.
    Codomain {
        env: Env,
        here: Origin,
        filling: Filling,
        name: Name,
        codomain: Term,
    },
    /// The incoming value is a function; apply it to what is left here, last
    /// entry first. `charged` is whether each application is charged a step —
    /// false for a definition's spine being replayed, whose eliminations were
    /// charged when they entered the spine.
    Spine { pending: Vec<(Origin, Arg)>, charged: bool },
    /// The incoming value came through one δ or one solved metavariable; keep
    /// looking through it until its head is neither.
    ///
    /// `forced` records that the value arriving is a metavariable *replay*, so
    /// that a second one in a row is charged `forcing` and the first is not —
    /// which is what the recursive `force` charged.
    Opening { forced: bool, definitions: bool },
    /// The incoming value is what a definition's spine replayed to; record it
    /// on the neutral that asked, so the replay happens once.
    Unfolding {
        cell: Arc<OnceLock<(Stamp, Value)>>,
        stamp: Stamp,
    },
    /// The incoming value is `built`'s last argument, opened; decide what the
    /// elimination does now that it can be looked at.
    Eliminating { built: Neutral },
    /// The incoming value is a recursor's method applied to the constructor's
    /// fields; the induction hypotheses are still to come.
    Hypotheses { reduction: Box<Reduction>, here: Origin },
    /// The same, with the hypotheses already assembled and `pending` the ones
    /// left, last first.
    Hypothesis { pending: Vec<Pending>, here: Origin },
    /// The incoming value is what a delayed method evaluated to; record it on
    /// the delay so that the next turn of the fold finds it there.
    ///
    /// The first of the two places `02-core-calculus.md` §3's fifth rule names,
    /// and a frame rather than a host call for the reason [`demanded`] gives: this
    /// happens once per turn of a fold, and a host frame here would put one
    /// under every level of the data being folded (§4.1).
    Forcing { delay: Arc<Delay> },
}

/// Run the machine until the control stack is empty.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the term's shape makes the next step meaningless.
fn run(meter: &mut Meter, start: Step) -> Result<Value, CoreError> {
    running(meter, Vec::new(), start)
}

/// [`run`] with work already on the stack, for an entry point that is handed a
/// spine rather than a term.
///
/// # Errors
///
/// As [`run`].
fn running(meter: &mut Meter, mut stack: Vec<Frame>, start: Step) -> Result<Value, CoreError> {
    let mut step = start;
    loop {
        step = match step {
            Step::Term(env, term) => evaluating(meter, &mut stack, &env, &term)?,
            Step::Apply {
                here,
                function,
                argument,
            } => applied(meter, &mut stack, here, function, argument)?,
            Step::Open { value, definitions } => {
                stack.push(Frame::Opening {
                    forced: false,
                    definitions,
                });
                Step::Value(value)
            }
            Step::Value(value) => match stack.pop() {
                None => return Ok(value),
                Some(frame) => resumed(meter, &mut stack, frame, value)?,
            },
        };
    }
}

/// One term, read (§1) — the seven shapes, and what each one does next.
fn evaluating(meter: &mut Meter, stack: &mut Vec<Frame>, env: &Env, term: &Term) -> Result<Step, CoreError> {
    meter.step("evaluation")?;
    let here = term.origin();
    Ok(match term.shape() {
        Shape::Var(index) => Step::Value(
            env.get(index.0)
                .cloned()
                .ok_or_else(|| CoreError::from(Malformed::UnboundVariable(*index)))?,
        ),
        // An unknown stands for a whole occurrence, spine and all: it is
        // closed, and §2.1 writes it `?m[σ]` — applied to the scope it may
        // mention (`kernel::meta`). The spine is *not* in the term, and
        // reading it out of the environment here is the reason. A term
        // moves: it is placed under further binders, put in a closure, and
        // evaluated again in whatever environment that closure was built
        // in. A spine written as indices would have to be shifted each
        // time, which is the substitution this crate does not have; a spine
        // read from the environment by *level* names the same binders
        // wherever the term ends up, because every environment a term is
        // re-read in extends the one it was written in.
        Shape::Meta(meta) => occurrence(stack, env, here, meta)?,
        // Resolved on the way in, so a value carries the level its arms
        // have already been solved to rather than the one written first.
        Shape::Universe(level) => Step::Value(Value::new(here, Form::Universe(level.clone()))),
        // §1's one name node, resolved through the context (§6). What the
        // name reduces to is the table's answer and not the term's, which
        // is the whole of this arm.
        Shape::Named { name, role, levels } => Step::Value(named(meter, env, here, name, role, levels)?),
        // Nothing to do, and that is the point: a numeral of 384 is one node
        // here, so evaluating it charges one step rather than 384.
        Shape::Lit(Constant::Payload(literal)) => Step::Value(Value::new(here, Form::Lit(literal.clone()))),
        Shape::Lit(Constant::Numeral(numeral)) => Step::Value(Value::new(here, Form::Numeral(numeral.clone()))),
        // §1's one binder node, read three ways. The written form shares a
        // constructor; the value forms do not, because a Π and a λ are told
        // apart by what eliminates them and nothing eliminates a `let`.
        Shape::Bind { name, binder, body } => match binder {
            Binder::Pi { filling, ty } => {
                meter.enter("evaluation")?;
                stack.push(Frame::Codomain {
                    env: env.clone(),
                    here,
                    filling: filling.clone(),
                    name: Arc::clone(name),
                    codomain: body.clone(),
                });
                Step::Term(env.clone(), ty.clone())
            }
            Binder::Lam => Step::Value(Value::new(
                here,
                Form::Lam(Closure {
                    env: env.clone(),
                    body: body.clone(),
                }),
            )),
            Binder::Let { ty: _, value } => {
                meter.enter("evaluation")?;
                stack.push(Frame::Body {
                    env: env.clone(),
                    body: body.clone(),
                });
                Step::Term(env.clone(), value.clone())
            }
        },
        Shape::App { function, argument } => {
            meter.enter("evaluation")?;
            stack.push(Frame::Argument {
                env: env.clone(),
                argument: argument.clone(),
                here,
            });
            Step::Term(env.clone(), function.clone())
        }
    })
}

/// A value arriving at the frame that was waiting for it.
fn resumed(meter: &mut Meter, stack: &mut Vec<Frame>, frame: Frame, value: Value) -> Result<Step, CoreError> {
    Ok(match frame {
        Frame::Dump { level } => {
            meter.at(level);
            Step::Value(value)
        }
        Frame::Argument { env, argument, here } => {
            // §3's fifth rule, decided at the one moment it can be: the
            // function has just been evaluated, so whether the argument about
            // to be evaluated stands at a method position of a recursor is
            // known, and the two frames that would evaluate it are simply not
            // pushed. The charges are the ones the strict path made — the
            // `enter` at the application is given back and the application
            // itself is charged — because what changed is the argument, not
            // the application.
            if let Form::Neutral(ref function) = value.form
                && crate::kernel::family::delays_next(function)
            {
                let built = Neutral::eliminated(
                    function,
                    Elim::App {
                        origin: here,
                        argument: Arg::delayed(env, argument),
                    },
                );
                meter.leave();
                meter.step("function application")?;
                return eliminating(meter, stack, built);
            }
            stack.push(Frame::Applied { function: value, here });
            Step::Term(env, argument)
        }
        Frame::Applied { function, here } => {
            // The application itself is a tail transition of this node: both
            // subterms are values, so nothing of this term is still being
            // descended into and the level goes back before the application
            // runs. Peyton Jones ch. 21's tail case, at an application rather
            // than at a call.
            meter.leave();
            meter.step("function application")?;
            Step::Apply {
                here,
                function,
                argument: Arg::ready(value),
            }
        }
        Frame::Body { env, body } => {
            // The same: a `let`'s body is its tail, so the level the value was
            // evaluated under is given back before the body is entered.
            meter.leave();
            Step::Term(env.push(value), body)
        }
        Frame::Codomain {
            env,
            here,
            filling,
            name,
            codomain,
        } => {
            meter.leave();
            Step::Value(Value::new(
                here,
                Form::Pi {
                    filling,
                    name,
                    domain: Arc::new(value),
                    codomain: Closure { env, body: codomain },
                },
            ))
        }
        Frame::Spine { pending, charged } => spined(meter, stack, pending, charged, value)?,
        Frame::Opening { forced, definitions } => opening(meter, stack, value, forced, definitions)?,
        Frame::Unfolding { cell, stamp } => {
            // `set` fails only where the cell already holds an answer at a
            // stamp this one is not reading, which is a memo that has gone
            // stale and stays stale. Recomputing is what the miss already
            // decided.
            drop(cell.set((stamp, value.clone())));
            Step::Value(value)
        }
        Frame::Eliminating { built } => eliminated(meter, stack, built, Some(&value))?,
        Frame::Hypotheses { reduction, here } => {
            let mut pending = crate::kernel::family::hypotheses(meter, &reduction)?;
            pending.reverse();
            hypothesis(meter, stack, pending, here, value)?
        }
        Frame::Hypothesis { pending, here } => hypothesis(meter, stack, pending, here, value)?,
        Frame::Forcing { delay } => {
            delay.fill(value.clone());
            Step::Value(value)
        }
    })
}

/// The incoming value is a function; apply it to the next waiting argument.
fn spined(
    meter: &mut Meter,
    stack: &mut Vec<Frame>,
    mut pending: Vec<(Origin, Arg)>,
    charged: bool,
    function: Value,
) -> Result<Step, CoreError> {
    let Some((here, argument)) = pending.pop() else {
        return Ok(Step::Value(function));
    };
    if !pending.is_empty() {
        stack.push(Frame::Spine { pending, charged });
    }
    if charged {
        meter.step("function application")?;
    }
    Ok(Step::Apply {
        here,
        function,
        argument,
    })
}

/// One induction hypothesis, or the answer when there are none left.
///
/// `answer` is the method applied to everything before this hypothesis. A
/// hypothesis the method provably never names is not computed — see
/// [`unread`](crate::kernel::family::unread) — and one it does name is the
/// recursor at the field, which is the descent, and which goes on the stack
/// rather than on a host frame.
fn hypothesis(
    meter: &mut Meter,
    stack: &mut Vec<Frame>,
    mut pending: Vec<Pending>,
    here: Origin,
    answer: Value,
) -> Result<Step, CoreError> {
    let Some(next) = pending.pop() else {
        return Ok(Step::Value(answer));
    };
    if !pending.is_empty() {
        stack.push(Frame::Hypothesis { pending, here });
    }
    match crate::kernel::family::unread(&answer) {
        Some(ignored) => {
            meter.step("function application")?;
            Ok(Step::Apply {
                here,
                function: answer,
                argument: Arg::ready(ignored),
            })
        }
        None => {
            let (recursor, field) = next.parts();
            stack.push(Frame::Applied { function: answer, here });
            meter.step("function application")?;
            Ok(Step::Apply {
                here,
                function: recursor,
                argument: Arg::ready(field),
            })
        }
    }
}

// Every arm that holds more than one intermediate value lives in its own
// function, and that used to be a stack-depth decision: each arm was a host
// call and a debug build gives a frame room for every arm's temporaries at
// once, so one match holding all twelve cost about 9 KiB a level.
//
// It is a legibility decision now. The machine above spends one frame however
// deep the term is, so the frames a level of *this* file costs are no longer
// what §4.1's room obligation has to hold. What it has to hold is the
// traversal, quotation, and the elaborator, which still descend on the host's
// stack because their depth is the depth of what they walk — see
// [`crate::kernel::room`], where the ceiling is measured.

/// What a name stands for here (§1, §6).
///
/// Five answers and one refusal, and each is what the removed variant did:
///
/// - a **declared constant** is closed and rigid, so evaluating one is reading
///   it. ι does not fire here: it needs the target, which arrives through
///   [`apply`]. A counting family's floor is the one constant that is not rigid
///   — [`Constant::value`](crate::kernel::family::Constant) turns it into the numeral
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
/// An occurrence of `meta` in `env`: the unknown applied to the scope it may
/// mention, and then reduced if a solution has arrived.
///
/// The scope is levels `0 … arity-1` — the *outermost* binders, which is what
/// makes this stable. A meta is created under some prefix of binders, and every
/// environment the term is later evaluated in extends that prefix at the
/// innermost end, so the outermost `arity` entries are still the very binders
/// the unknown was created under.
///
/// # Errors
///
/// [`Malformed::MetaTelescope`] when the environment is shallower than the
/// arity, which is a term moved somewhere its unknown's scope does not reach.
fn occurrence(stack: &mut Vec<Frame>, env: &Env, here: Origin, meta: &Meta) -> Result<Step, CoreError> {
    let depth = env.depth().0;
    let mut arguments = Vec::with_capacity(meta.arity() as usize);
    for level in 0..meta.arity() {
        let index = depth
            .checked_sub(level.saturating_add(1))
            .ok_or(Malformed::MetaTelescope(meta.id()))?;
        let argument = env.get(index).ok_or(Malformed::MetaTelescope(meta.id()))?;
        arguments.push(argument.clone());
    }
    match meta.solution() {
        // Its own origins (§7): what the unknown stood for was written
        // somewhere, and the occurrence is not that place.
        Some(solution) => {
            let mut pending: Vec<(Origin, Arg)> = arguments
                .into_iter()
                .map(|argument| (here, Arg::ready(argument)))
                .collect();
            pending.reverse();
            if pending.is_empty() {
                return Ok(Step::Value(solution.clone()));
            }
            stack.push(Frame::Spine { pending, charged: true });
            Ok(Step::Value(solution.clone()))
        }
        None => Ok(Step::Value(Value::neutral(Neutral {
            origin: here,
            head: Head::Meta(meta.clone()),
            spine: arguments
                .into_iter()
                .map(|argument| Elim::App {
                    origin: here,
                    argument: Arg::ready(argument),
                })
                .collect(),
            // An unknown is not a definition: there is nothing folded here to
            // unfold, so there is nothing to record.
            unfolded: None,
        }))),
    }
}

/// [`neutral_type`] answers their types by evaluating a declaration term and
/// has no context to ask — see [`Head::Base`].
///
/// # Errors
///
/// [`Malformed::UndeclaredName`] when nothing in scope answers to the name. The
/// elaborator resolved it once already, so this is a defect in whoever built or
/// moved the term — never a silent resolution to a different declaration of the
/// same spelling.
fn named(
    meter: &mut Meter,
    env: &Env,
    here: Origin,
    name: &Name,
    role: &Role,
    levels: &Levels,
) -> Result<Value, CoreError> {
    let globals = env.globals();
    match globals.definition(name, role, levels) {
        Definition::Declared(constant) => Ok(constant.value(here, globals)),
        Definition::Defined(def) | Definition::Compiled(def) => {
            let (ty, body) = def.instance(meter, globals, levels)?;
            let folding = match body {
                crate::kernel::program::Body::Value { value, .. } => Folding::Value(value),
                crate::kernel::program::Body::Compiled { tree, .. } => Folding::Compiled(tree, globals.clone()),
                crate::kernel::program::Body::Pending => Folding::Pending,
            };
            Ok(Value::neutral(Neutral::head(
                here,
                Head::Def(DefHead::Global(def.clone(), levels.clone()), ty, folding),
            )))
        }
        Definition::Base(base) => Ok(Value::neutral(Neutral::head(here, Head::Base(base, globals.clone())))),
        Definition::Builtin(builtin) => Ok(Value::neutral(Neutral::head(
            here,
            Head::Builtin(builtin, globals.clone()),
        ))),
        Definition::Undeclared => Err(Malformed::UndeclaredName(Arc::clone(name)).into()),
    }
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
/// A fixed point rather than a step: a solution can itself be headed by a meta
/// that has since been solved, and a caller that trusted one step would read a
/// solved meta as an unsolved one — in [`crate::elaboration::convert`] that is
/// not a missed reduction but a wrong answer. Each pass after the first is
/// charged, so a chain is bounded by the budget rather than by a claim that
/// chains are short.
///
/// # Errors
///
/// As [`eval`]: replaying the spine is ordinary evaluation.
pub(crate) fn force(meter: &mut Meter, value: &Value) -> Result<Option<Value>, CoreError> {
    if solution_of(value).is_none() {
        return Ok(None);
    }
    run(
        meter,
        Step::Open {
            value: value.clone(),
            definitions: false,
        },
    )
    .map(Some)
}

/// The value seen through solved metavariables *and* folded definitions at
/// its head, or `None` when there was nothing to see through.
///
/// The fixed point of [`force`] and δ, and the operation every place that asks
/// "is this a canonical form yet" takes: conversion on a folded disagreement, ι
/// at a recursor target, δ at a builtin's arguments, quotation in the opening
/// mode, and the elaborator wherever it already forced a value before matching
/// its form. [`force`] alone remains what conversion's folded comparison and
/// quotation's keeping mode use, because both exist to *not* open definitions.
///
/// The postcondition is the fixed point — the head of what comes back is
/// neither a solved metavariable nor a folded definition.
///
/// # Errors
///
/// As [`eval`].
pub(crate) fn opened(meter: &mut Meter, value: &Value) -> Result<Option<Value>, CoreError> {
    if solution_of(value).is_none() && folded(value).is_none() {
        return Ok(None);
    }
    run(
        meter,
        Step::Open {
            value: value.clone(),
            definitions: true,
        },
    )
    .map(Some)
}

/// One pass of [`force`] or [`opened`], as a transition.
///
/// δ carries no bookkeeping charge and a metavariable replay does: evaluation
/// charged each elimination when it entered the spine, so charging a
/// definition's replay again would count every application of a definition
/// twice, while a spine behind a metavariable was waiting for the solution and
/// its replay is where that work finally runs. `forced` is what keeps the
/// second and later passes of a metavariable chain charged `forcing` while the
/// first is not, which is what the recursion this replaced charged.
fn opening(
    meter: &mut Meter,
    stack: &mut Vec<Frame>,
    value: Value,
    forced: bool,
    definitions: bool,
) -> Result<Step, CoreError> {
    if let Some((solution, spine)) = solution_of(&value) {
        if forced {
            meter.step("forcing")?;
        }
        stack.push(Frame::Opening {
            forced: true,
            definitions,
        });
        push_spine(stack, &spine, true);
        return Ok(Step::Value(solution));
    }
    if !definitions {
        return Ok(Step::Value(value));
    }
    let Some((body, spine, cell)) = folded(&value) else {
        return Ok(Step::Value(value));
    };
    let stamp = meter.stamp();
    if let Some(cell) = cell.as_ref()
        && let Some(&(filled, ref answer)) = cell.get()
        && filled == stamp
    {
        stack.push(Frame::Opening {
            forced: false,
            definitions,
        });
        return Ok(Step::Value(answer.clone()));
    }
    stack.push(Frame::Opening {
        forced: false,
        definitions,
    });
    if let Some(cell) = cell {
        stack.push(Frame::Unfolding { cell, stamp });
    }
    push_spine(stack, &spine, false);
    Ok(Step::Value(body))
}

/// The solution a value's head metavariable has, and the spine waiting on it.
fn solution_of(value: &Value) -> Option<(Value, Vec<Elim>)> {
    let Form::Neutral(ref neutral) = value.form else {
        return None;
    };
    let Head::Meta(ref meta) = neutral.head else {
        return None;
    };
    Some((meta.solution().cloned()?, neutral.spine.clone()))
}

/// A folded definition's value, the spine over it, and the cell that records
/// the replay.
///
/// The memo is what makes δ cost once rather than once per consumer: a `Value`
/// is `Arc`-shared and pure, so without it every reader of the same neutral
/// re-ran the whole replay. See prompt 165b and
/// `../../../../docs/notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md`.
#[expect(clippy::type_complexity, reason = "one destructuring, read at one call site")]
fn folded(value: &Value) -> Option<(Value, Vec<Elim>, Option<Arc<OnceLock<(Stamp, Value)>>>)> {
    let Form::Neutral(ref neutral) = value.form else {
        return None;
    };
    let Head::Def(_, _, Folding::Value(ref body)) = neutral.head else {
        return None;
    };
    Some((Value::clone(body), neutral.spine.clone(), neutral.unfolded.clone()))
}

/// Put a spine on the control stack, innermost first — the order it is stored
/// in, and so the order it is replayed in.
fn push_spine(stack: &mut Vec<Frame>, spine: &[Elim], charged: bool) {
    if spine.is_empty() {
        return;
    }
    let mut pending: Vec<(Origin, Arg)> = spine
        .iter()
        .map(|elimination| {
            let Elim::App { origin, ref argument } = *elimination;
            (origin, argument.clone())
        })
        .collect();
    pending.reverse();
    stack.push(Frame::Spine { pending, charged });
}

/// A definition whose body is a compiled case tree, reduced (§1), or `None`
/// when it stays blocked.
///
/// The mirror of [`crate::kernel::family::iota`] and of [`delta`], in the same
/// arm and for the same reason: an application is the first moment a rule can
/// know its last argument has arrived. Blocked here means either too few
/// arguments or a scrutinee that is not canonical, and both leave an ordinary
/// neutral spine — which is what a definition applied to a variable *is*.
///
/// # Errors
///
/// As [`Compiled::reduce`](crate::kernel::case_tree::Compiled::reduce).
fn matched(meter: &mut Meter, neutral: &Neutral) -> Result<Option<Matched>, CoreError> {
    let Head::Def(DefHead::Global(def, _), _, Folding::Compiled(compiled, globals)) = &neutral.head else {
        return Ok(None);
    };
    let mut arguments = Vec::with_capacity(neutral.spine.len());
    for elimination in &neutral.spine {
        let Elim::App { ref argument, .. } = *elimination;
        arguments.push(demanded(meter, argument)?);
    }
    compiled.reduce(meter, globals, def.name(), &arguments)
}

/// A definition's value with a spine replayed over it, uncharged.
///
/// The caller matched the `Def` head itself, so the value and the spine are
/// what it holds.
///
/// # Errors
///
/// As [`eval`].
pub(crate) fn unfold_spine(meter: &mut Meter, value: &Value, spine: &[Elim]) -> Result<Value, CoreError> {
    let mut stack: Vec<Frame> = Vec::new();
    push_spine(&mut stack, spine, false);
    running(meter, stack, Step::Value(value.clone()))
}

/// Open a closure at `argument`.
///
/// # Errors
///
/// As [`eval`].
pub(crate) fn apply_closure(meter: &mut Meter, closure: &Closure, argument: Value) -> Result<Value, CoreError> {
    eval(meter, &closure.env.push(argument), &closure.body)
}

/// Open a closure at whatever `argument` computes, computing it only if the
/// closure can read it.
///
/// [`Closure::reads_its_binder`] states the condition and why it is
/// conservative. This exists because the elaborator's instantiation walk holds
/// a *term* for each argument and needs only the codomain: evaluating that term
/// to push a value the codomain never reads costs the whole subterm, and a walk
/// that does it at every level of a nested application pays the sum of the
/// subtree sizes rather than the tree's. Measured at prompt 165 on a voice of
/// 400 plain notes: 281,264 of the run's 367,097 reduction steps were spent
/// evaluating arguments, and every one of them stood at a codomain that never
/// read its binder.
///
/// The argument still reaches the elaborated term, so nothing is skipped that
/// the program's own evaluation does not do once.
///
/// # Errors
///
/// Whatever `argument` returns, or as [`eval`].
pub(crate) fn apply_closure_read<E: From<CoreError>>(
    meter: &mut Meter,
    closure: &Closure,
    argument: impl FnOnce(&mut Meter) -> Result<Value, E>,
) -> Result<Value, E> {
    // Nothing in the body reaches the binder, so which value stands there
    // cannot change the answer. `Value::unread` is what stands there instead.
    let value = if closure.reads_its_binder() {
        argument(meter)?
    } else {
        Value::unread()
    };
    Ok(eval(meter, &closure.env.push(value), &closure.body)?)
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
/// The value a spine argument stands for, evaluating it if nothing has yet.
///
/// The second of the two places `02-core-calculus.md` §3's fifth rule names: a
/// spine that stayed stuck is about to be read back, compared, or asked its
/// type, and a reader that met a term where it expected a value would have to
/// know about the rule. This is the boundary that means it does not.
///
/// Forcing is a host call and not a transition of the machine, which is right
/// here and wrong at the other place: reading a stuck spine descends on the
/// host's stack already (§4.1's room obligation is about that descent), while ι
/// selecting a method happens once per turn of a fold and must not put a frame
/// under it — see [`Frame::Forcing`].
///
/// # Errors
///
/// As [`eval`], for the term the delay holds.
pub(crate) fn demanded(meter: &mut Meter, argument: &Arg) -> Result<Value, CoreError> {
    if let Some(value) = argument.settled() {
        return Ok(value.clone());
    }
    let Arg::Delayed(ref delay) = *argument else {
        // `settled` answers `Some` for every `Ready`, so this is unreachable;
        // written as a match rather than an unwrap so that it stays so.
        return Err(Malformed::NotAFunction.into());
    };
    let (env, term) = delay.parts();
    let value = eval(meter, env, term)?;
    delay.fill(value.clone());
    Ok(value)
}

pub(crate) fn apply(meter: &mut Meter, here: Origin, function: Value, argument: Value) -> Result<Value, CoreError> {
    apply_arg(meter, here, function, Arg::ready(argument))
}

/// [`apply`] to an argument that may not have been evaluated.
///
/// The one caller is [`crate::kernel::family::hypotheses`], reassembling a
/// recursor over the prefix it was applied to. That prefix holds the methods,
/// which under §3's fifth rule are delayed, and an induction hypothesis that
/// forced them to rebuild the spine they already sit on would evaluate every
/// method of the fold at every level of it.
///
/// # Errors
///
/// As [`apply`].
pub(crate) fn apply_arg(meter: &mut Meter, here: Origin, function: Value, argument: Arg) -> Result<Value, CoreError> {
    meter.step("function application")?;
    applying(meter, here, function, argument)
}

/// [`apply`] without the bookkeeping charge, for a spine being replayed at an
/// unfold: the elimination was charged when it entered the spine, and charging
/// the replay as well would count every application of a definition twice.
/// The work inside — a β body, an ι step, a builtin's rule — carries its own
/// charges either way.
fn applying(meter: &mut Meter, here: Origin, function: Value, argument: Arg) -> Result<Value, CoreError> {
    run(
        meter,
        Step::Apply {
            here,
            function,
            argument,
        },
    )
}

/// One application, as a transition.
///
/// β is the whole reason the machine exists: the closure's body becomes the
/// control, and the enclosing work stays on the stack. A host call here is a
/// host frame per turn of every fold in the program.
fn applied(
    meter: &mut Meter,
    stack: &mut Vec<Frame>,
    here: Origin,
    function: Value,
    argument: Arg,
) -> Result<Step, CoreError> {
    match function.form {
        // A λ is not a recursor, so nothing delayed is ever applied to one by
        // the rule above; an argument that arrived delayed some other way is
        // forced here rather than pushed, because β substitutes a value.
        Form::Lam(body) => {
            let argument = demanded(meter, &argument)?;
            Ok(entering(meter, stack, body.env.push(argument), body.body))
        }
        // Not a function, and not applied to anything: `Row(12) x` is what a
        // caller wrote when it meant `Row x`, and this is where it says so.
        // A blocked application is where ι at an inductive family fires: the
        // recursor's target is its last argument, so this is the first moment the
        // elimination can know it has met a constructor.
        Form::Neutral(function) => {
            let built = Neutral::eliminated(&function, Elim::App { origin: here, argument });
            eliminating(meter, stack, built)
        }
        Form::Universe(_) | Form::Pi { .. } | Form::Lit(_) | Form::Numeral(_) => Err(Malformed::NotAFunction.into()),
    }
}

/// A freshly blocked spine, with the one argument three of the five rules
/// below have to look through opened first.
///
/// ι at a projection, ι at a recursor, and a counting constructor's collapse
/// each read exactly one value — the last argument — and each used to open it
/// from inside the rule, which put a host frame under every level of the data
/// they walk. [`opens_last`](crate::kernel::family::opens_last) asks the
/// question separately so that the opening is a transition of this machine,
/// and the rules themselves decide without evaluating anything.
fn eliminating(meter: &mut Meter, stack: &mut Vec<Frame>, built: Neutral) -> Result<Step, CoreError> {
    if crate::kernel::family::opens_last(&built)
        && let Some(Elim::App { argument, .. }) = built.spine.last()
    {
        // Forced, and it is always a clone: a spine that `opens_last` admits is
        // saturated, and the last argument of a saturated recursor is the
        // target, which `delays_next` never delays.
        let subject = demanded(meter, argument)?;
        stack.push(Frame::Eliminating { built });
        return Ok(Step::Open {
            value: subject,
            definitions: true,
        });
    }
    eliminated(meter, stack, built, None)
}

/// Begin a *nested* evaluation: a λ's body opened, a case tree's arm chosen.
///
/// The one place §4.1's metric is given back rather than spent. What is being
/// entered is a different term from the one the frames below were descending,
/// and the metric measures how deeply a term is written — so the enclosing
/// depth is saved and the new term starts at the bottom, which is exactly what
/// stops a definition calling itself from being charged as though the composer
/// had written a deeper term. Peyton Jones ch. 11's observation that a nested
/// evaluation "needs a brand new stack", with ch. 18 §18.8's dump holding the
/// old one.
fn entering(meter: &mut Meter, stack: &mut Vec<Frame>, env: Env, term: Term) -> Step {
    stack.push(Frame::Dump { level: meter.level() });
    meter.at(0);
    Step::Term(env, term)
}

/// The five rules a blocked spine is offered to, in the order they were always
/// tried, with `subject` the last argument opened when one of them asked.
fn eliminated(
    meter: &mut Meter,
    stack: &mut Vec<Frame>,
    built: Neutral,
    subject: Option<&Value>,
) -> Result<Step, CoreError> {
    if let Some(fired) = crate::kernel::family::iota(&built, subject) {
        return Ok(match fired {
            Fired::Field(field) => Step::Value(field),
            Fired::Method(reduction) => {
                let here = built.origin;
                let mut fields: Vec<(Origin, Arg)> = reduction
                    .fields
                    .iter()
                    .map(|field| (here, Arg::ready(field.clone())))
                    .collect();
                fields.reverse();
                let method = reduction.method.clone();
                stack.push(Frame::Hypotheses { reduction, here });
                if !fields.is_empty() {
                    stack.push(Frame::Spine {
                        pending: fields,
                        charged: true,
                    });
                }
                // §3's fifth rule, collected: the method ι chose is the one of
                // the recursor's methods that is evaluated, and the frames
                // above are already waiting for what it evaluates to.
                match method {
                    Arg::Ready(ref value) => Step::Value(Value::clone(value)),
                    Arg::Delayed(delay) => match delay.settled() {
                        Some(value) => Step::Value(value.clone()),
                        None => {
                            let (env, term) = delay.parts();
                            let (env, term) = (env.clone(), term.clone());
                            stack.push(Frame::Forcing { delay });
                            Step::Term(env, term)
                        }
                    },
                }
            }
        });
    }
    // The other direction: not an elimination firing but a construction
    // collapsing, so that a counting family's values stay numerals and never
    // accumulate a spine.
    if let Some(counted) = crate::kernel::family::stepped(&built, subject) {
        return Ok(Step::Value(counted));
    }
    if let Some(reduced) = delta(meter, &built)? {
        return Ok(Step::Value(reduced));
    }
    if let Some(chosen) = matched(meter, &built)? {
        let here = built.outer_origin();
        let mut rest: Vec<(Origin, Arg)> = chosen
            .rest
            .into_iter()
            .map(|argument| (here, Arg::ready(argument)))
            .collect();
        rest.reverse();
        if !rest.is_empty() {
            stack.push(Frame::Spine {
                pending: rest,
                charged: true,
            });
        }
        return Ok(entering(meter, stack, chosen.env, chosen.term));
    }
    match structural(meter, &built)? {
        Some(reduced) => Ok(Step::Value(reduced)),
        None => Ok(Step::Value(Value::neutral(built))),
    }
}

/// δ at a compiler-owned builtin, or `None` when the spine is not ready.
///
/// The mirror of [`crate::kernel::family::iota`], in the same arm and for the same
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
        let Elim::App { argument, .. } = elimination;
        let argument = demanded(meter, argument)?;
        let Some(datum) = canonical(meter, &argument)? else {
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
            crate::kernel::family::realize(meter, here, globals, &answer, &ty).map(Some)
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
/// Needs no type, where [`crate::kernel::family::realize`] does: canonical data is
/// canonical, so looking says which shape it is, and η — the one thing that
/// makes reading back type-directed — has nothing to expand here.
///
/// # Why this is a loop and charges steps
///
/// It used to recurse per field and charge one **nesting** level per level of
/// data, and that was two mistakes in one line. §4.1 derives the nesting metric
/// from how deeply a *term* is written and how deeply `quote` walks a value
/// back; reading a δ-builtin's argument is neither, and the depth it reached
/// was the length of one list. Measured on `examples/staff-page.musa`, this
/// walk and its inverse peaked at 672 of 320 levels between them, out of 679
/// for the whole run — a limit on the size of an argument, wearing the name of
/// a stack guard. So the descent is an explicit stack in this function's own
/// frame, and the charge is one step a node against the budget that already
/// prices how much work a run does.
/// `../../../../docs/notes/research/language-design-closure/59-the-staff-wall-is-the-evaluators.md`
/// is the measurement and prompt 165b is the change.
///
/// # Errors
///
/// As [`force`], plus the step budget at data larger than the meter allows.
fn canonical(meter: &mut Meter, value: &Value) -> Result<Option<Datum>, CoreError> {
    /// One constructor whose earlier fields are read and whose later ones are not.
    struct Building {
        constructor: Name,
        /// The field values still to read, innermost last: [`Vec::pop`] takes
        /// the next one, so the spine is pushed reversed.
        rest: Vec<Arc<Value>>,
        read: Vec<Datum>,
    }

    let mut stack: Vec<Building> = Vec::new();
    let mut here = Arc::new(value.clone());
    loop {
        meter.step("canonical data")?;
        let forced = opened(meter, &here)?;
        let mut answer = match forced.as_ref().unwrap_or(&here).form {
            // A type, not data. See `crate::kernel::family::canonical`.
            Form::Lit(ref literal) => Datum::Lit(literal.clone()),
            // The count read back as the tower it stands for. See
            // [`crate::kernel::family::canonical`], which is the same answer one layer
            // up and carries the argument for building it with a loop.
            Form::Numeral(ref numeral) => match crate::kernel::family::counted(numeral) {
                Some(datum) => datum,
                None => return Ok(None),
            },
            Form::Neutral(ref neutral) => {
                let Some((constructor, params)) = crate::kernel::family::constructed(neutral) else {
                    return Ok(None);
                };
                let mut rest = Vec::with_capacity(neutral.spine.len().saturating_sub(params));
                for elimination in neutral.spine.iter().skip(params) {
                    let Elim::App { ref argument, .. } = *elimination;
                    rest.push(Arc::new(demanded(meter, argument)?));
                }
                rest.reverse();
                match rest.pop() {
                    Some(first) => {
                        stack.push(Building {
                            constructor,
                            rest,
                            read: Vec::new(),
                        });
                        here = first;
                        continue;
                    }
                    None => Datum::Case {
                        constructor,
                        fields: Vec::new(),
                    },
                }
            }
            Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) => return Ok(None),
        };
        // Hand the field up, and keep handing completed constructors up until
        // one still has a field waiting.
        loop {
            let Some(building) = stack.last_mut() else {
                return Ok(Some(answer));
            };
            building.read.push(answer);
            if let Some(next) = building.rest.pop() {
                here = next;
                break;
            }
            let Some(finished) = stack.pop() else {
                return Ok(None);
            };
            answer = Datum::Case {
                constructor: finished.constructor,
                fields: finished.read,
            };
        }
    }
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
        let Elim::App { argument, .. } = elimination;
        let Form::Pi { codomain, .. } = ty.form else {
            return Err(Malformed::NotAFunction.into());
        };
        let argument = demanded(meter, argument)?;
        ty = apply_closure(meter, &codomain, argument)?;
    }
    Ok(ty)
}

/// A structural eliminator's step, or `None` when the spine is not ready.
///
/// §5.8's second family, in the same arm as [`delta`] and [`crate::kernel::family::iota`]
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
        let Elim::App { argument, .. } = elimination;
        arguments.push(demanded(meter, argument)?);
    }
    // Registration checked that the target names an argument, so this indexes a
    // spine of exactly the arity.
    let Some(subject) = arguments.get(target) else {
        return Ok(None);
    };
    let forced = opened(meter, subject)?;
    let rewritten = match forced.as_ref().unwrap_or(subject).form {
        Form::Lit(ref literal) => {
            meter.step("structural reduction")?;
            rewrite(builtin, literal)
        }
        Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::Numeral(_) | Form::Neutral(_) => return Ok(None),
    };
    let Some(rewritten) = rewritten else {
        return Err(Malformed::BuiltinStuck(Arc::clone(builtin.name())).into());
    };
    // Innermost last, so index 0 is the last argument — the order a telescope of
    // binders over the same spine would have produced.
    let env = arguments
        .into_iter()
        .fold(Env::under(globals.clone()), |env, argument| env.push(argument));
    // The one nesting charge evaluation still makes, and §4.1's first clause is
    // exactly it: "§5.9's traversal descends *through* the transformer's own
    // branches, so one level of a region's nesting costs a whole chain of
    // evaluator frames rather than one." A rewrite that names its own builtin
    // again is a host call from here, so the depth of a traversal is the depth
    // of this chain, and a region nested past the limit has to be refused
    // rather than abort the process. Every other descent evaluation makes is on
    // the control stack above and charges steps.
    meter.nested("traversal", |meter| eval(meter, &env, &rewritten).map(Some))
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
        // The prefix each elimination is applied to, grown in place: `J`'s
        // result type mentions the proof, so the walk has to be able to name
        // what it has consumed so far.
        let mut prefix = Neutral::head(neutral.origin, neutral.head.clone());
        for elimination in &neutral.spine {
            // A type written as a metavariable is blocked until that meta is
            // solved, and one written as a definition stays folded until it is
            // opened; matching it as either would answer `NotAFunction` for a
            // term the elaborator had just proved well typed.
            let head = opened(meter, &ty)?.unwrap_or(ty);
            ty = eliminated_type(meter, head, elimination)?;
            prefix.spine.push(elimination.clone());
        }
        Ok(ty)
    })
}

/// The type of `prefix` eliminated by `elimination`, given the prefix's own
/// type already forced.
fn eliminated_type(meter: &mut Meter, head: Value, elimination: &Elim) -> Result<Value, CoreError> {
    match elimination {
        Elim::App { argument, .. } => match head.form {
            Form::Pi { codomain, .. } => {
                let argument = demanded(meter, argument)?;
                apply_closure(meter, &codomain, argument)
            }
            Form::Universe(_) | Form::Lam(_) | Form::Lit(_) | Form::Numeral(_) | Form::Neutral(_) => {
                Err(Malformed::NotAFunction.into())
            }
        },
    }
}

/// The type of a blocked elimination, unfolded far enough to be matched on.
///
/// A caller decides what an elimination is legal by matching this against
/// [`Form::Pi`], and a type that was itself written as a metavariable is
/// [`Form::Neutral`] until that meta is solved. Matching without forcing would
/// answer [`Malformed::NotAFunction`] for a term the elaborator had just proved
/// well typed.
///
/// # Errors
///
/// As [`neutral_type`].
pub(crate) fn head_type(meter: &mut Meter, neutral: &Neutral) -> Result<Value, CoreError> {
    let ty = neutral_type(meter, neutral)?;
    // Opened rather than merely forced: the answer is matched against `Π`, and
    // a type that names a definition hides that while folded.
    Ok(opened(meter, &ty)?.unwrap_or(ty))
}

#[cfg(test)]
// A kernel that refuses what this module built is a defect in this module, so
// panicking is the correct report.
#[allow(clippy::panic)]
#[allow(clippy::expect_used)]
mod tests {
    //! The δ memo: `unfold`'s graph update, and the one condition that makes it
    //! sound.
    //!
    //! In-crate rather than in `tests/suite/`, because the memo is a property of
    //! a [`Neutral`] and no public constructor builds one. What is observable
    //! from outside is only that a program is faster, which is not the law.

    use super::opened;
    use crate::kernel::budget::Budget;
    use crate::kernel::context::Cx;
    use crate::kernel::meta::Meta;
    use crate::kernel::origin::Origin;
    use crate::kernel::sort::Sort;
    use crate::kernel::term::{Level, Term};
    use crate::kernel::value::{Arg, Closure, DefHead, Elim, Env, Folding, Form, Head, Neutral, Value};
    use std::sync::Arc;

    const HERE: Origin = Origin::node(921);

    /// `Type 0`, the type and the argument everything below stands at.
    fn type0() -> Value {
        Value::new(HERE, Form::Universe(Sort::ZERO))
    }

    /// Whether a value is `Type 0`, which is what everything below unfolds to.
    ///
    /// A predicate rather than an equality because [`Value`] has neither
    /// `PartialEq` nor `Debug`: conversion is the kernel's equality and it takes
    /// a type, which is more machinery than a law about a universe needs.
    fn universe(value: &Value) -> bool {
        matches!(value.form, Form::Universe(ref sort) if *sort == Sort::ZERO)
    }

    /// A definition holding `λ_. let z … let z … Type 0`, `depth` lets deep,
    /// applied to one argument.
    ///
    /// A shape whose *unfolding* costs real steps and whose neutral costs none,
    /// which is the only way a spend can say whether the unfolding happened.
    fn folded(depth: u32) -> Neutral {
        let ty = Term::universe(HERE, Sort::ZERO);
        let body = (0..depth).fold(ty.clone(), |inner, _| {
            Term::bind(HERE, "z", ty.clone(), ty.clone(), inner)
        });
        let value = Value::new(HERE, Form::Lam(Closure { env: Env::EMPTY, body }));
        let head = Head::Def(
            DefHead::Local(Level(0)),
            Arc::new(type0()),
            Folding::Value(Arc::new(value)),
        );
        Neutral::eliminated(
            &Neutral::head(HERE, head),
            Elim::App {
                origin: HERE,
                argument: Arg::ready(type0()),
            },
        )
    }

    /// The graph update, stated as the law it encodes: a folded neutral's
    /// unfolding is a function of the definition's value and the spine, both of
    /// which the neutral owns, so forcing it a second time answers the same
    /// thing and does no work.
    ///
    /// Peyton Jones ch. 12 §12.4 overwrites a shared redex's root with its
    /// result; this is that, on a value that is shared by `Arc` rather than by
    /// a pointer into a heap.
    #[test]
    fn a_folded_neutral_unfolds_once_and_answers_the_same_thing() {
        let cx = Cx::with_budget(Budget::LANGUAGE);
        let mut meter = cx.meter();
        let held = Value::neutral(folded(120));

        let first = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let after_first = meter.spent().steps;
        let second = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let after_second = meter.spent().steps;

        assert!(
            universe(&first) && universe(&second),
            "the same neutral unfolds to the same value, and it is `Type 0`"
        );
        assert!(after_first >= 120, "the first force does the work: {after_first} steps");
        assert_eq!(
            after_second,
            after_first,
            "the second force reads the cell: {} steps",
            after_second.saturating_sub(after_first)
        );
    }

    /// The soundness condition, stated as what it actually guards.
    ///
    /// A memo is unsound over an unsolved metavariable: the spine or the
    /// definition's value may mention one, and the answer changes when the
    /// solution arrives. The test is
    /// [`Stamp`](crate::kernel::budget::Stamp), moved in [`Meta::solve`] —
    /// equal stamps mean this run solved nothing in between — which is
    /// conservative in the safe direction: *any* solution this run makes
    /// invalidates *every* memo it holds, and the only cost of being wrong is
    /// the work being done again. That is what this pins: after a solution,
    /// the same neutral is unfolded again rather than answered from the cell.
    #[test]
    fn a_solved_metavariable_makes_a_forced_value_unfold_again() {
        let cx = Cx::with_budget(Budget::LANGUAGE);
        let mut meter = cx.meter();
        let held = Value::neutral(folded(120));

        let before = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let first = meter.spent().steps;

        // Unrelated to the neutral above, which is the point: the guard does
        // not ask *which* metavariable, because asking would mean walking the
        // value to find out.
        let meta = Meta::new(0, HERE, type0(), 0, cx.globals().clone(), crate::kernel::meta::MetaSource::TypeParameter(None));
        meta.solve(&mut meter, type0())
            .expect("an unsolved metavariable takes a solution");

        let after = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let second = meter.spent().steps;

        assert!(
            universe(&before) && universe(&after),
            "re-unfolding answers what the memo would have"
        );
        assert!(
            second.saturating_sub(first) >= 120,
            "the stale memo was not read: {} steps",
            second.saturating_sub(first)
        );
    }

    /// The other half of that condition, and the half a process-wide stamp got
    /// wrong.
    ///
    /// *Any* solution invalidating *every* memo is only conservative while
    /// "every memo" means this run's. Counted across the process it also meant
    /// the memos of runs that cannot reach the metavariable at all, so a
    /// compile paid for whatever else the process was doing — which is
    /// `budget.rs`'s "the same terms exhaust at the same operation on every
    /// host", broken. It broke it in practice too: this crate's laws run as
    /// threads of one process under `cargo test`, and the solutions they made
    /// while `musa-compiler`'s corpus law was compiling
    /// `examples/staff-page.musa` re-unfolded enough of it to cross
    /// [`Budget::LANGUAGE`] and refuse a file that compiles. `cargo nextest`,
    /// which this repository runs by default, gives every law its own process
    /// and so never showed it.
    #[test]
    fn another_runs_solution_leaves_this_runs_memo_alone() {
        let cx = Cx::with_budget(Budget::LANGUAGE);
        let mut meter = cx.meter();
        let held = Value::neutral(folded(120));

        let before = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let first = meter.spent().steps;

        // A second run, solving an unknown of its own — what another document
        // being compiled beside this one amounts to.
        let mut elsewhere = cx.meter();
        let meta = Meta::new(0, HERE, type0(), 0, cx.globals().clone(), crate::kernel::meta::MetaSource::TypeParameter(None));
        meta.solve(&mut elsewhere, type0())
            .expect("an unsolved metavariable takes a solution");

        let after = opened(&mut meter, &held)
            .expect("a definition unfolds")
            .expect("a definition");
        let second = meter.spent().steps;

        assert!(
            universe(&before) && universe(&after),
            "the memo answers what re-unfolding would"
        );
        assert_eq!(
            second,
            first,
            "another run's solution is not this run's: {} steps",
            second.saturating_sub(first)
        );
    }
}
