//! The kernel re-deriving the type of a term elaboration says it already gave
//! one.
//!
//! This module is what makes `TRUST.md`'s claim enforceable rather than
//! aspirational. Elaboration is a large program with metavariables, case trees,
//! coverage, dependency ordering and a name resolver; the kernel is a
//! structural recursion with no search in it. If the two disagree about a
//! finished term, the kernel is right and something above it has a bug — and
//! the point of running the kernel over elaboration's own output is that the
//! disagreement is *found*, at the declaration that caused it, rather than
//! surfacing later as a program that means something nobody wrote.
//!
//! # What it is not
//!
//! Not a second type-checker with rules of its own. Every rule here is one the
//! evaluator already implements: a Π's codomain comes from
//! [`apply_closure`](crate::kernel::eval::apply_closure), a field's type from
//! [`field_type`](crate::kernel::eval::field_type), a name's type from the same
//! [`Definition`](crate::kernel::term::Definition) evaluation reaches for. A
//! rule this pass needed and the kernel lacked would mean the kernel is what is
//! wrong, and that is a different prompt.
//!
//! Not a refusal, either. Every failure here is [`Malformed`] — a caller
//! defect. The program was accepted by elaboration; if the kernel disagrees,
//! *this compiler* is what is broken, and saying so in the composer's direction
//! would be a failed diagnostic.
//!
//! # Conversion, here, is `quote ∘ eval`
//!
//! Two types agree when their normal forms are equal terms, by [`Term`]'s own
//! `PartialEq` — which ignores origins and performs no search. That is a
//! deliberately weaker instrument than the elaborator's conversion checker,
//! which also solves metavariables and reports a path into the mismatch. The
//! weakness is the feature: a re-checker that shared the solver would be
//! re-running the machinery it exists to audit.
//!
//! # Where it runs
//!
//! Behind [`debug_assert`] on every elaborated declaration, and unconditionally
//! in the conformance suite. Not in release builds: it roughly doubles the cost
//! of checking, and its job is to catch *our* bugs rather than an author's.

use std::sync::Arc;

use crate::kernel::budget::Meter;
use crate::kernel::checked::Checked;
use crate::kernel::context::Cx;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::eval::{apply_closure, eval, field_type, opened};
use crate::kernel::origin::Origin;
use crate::kernel::quote::{Mode, quote_type};
use crate::kernel::sort::Sort;
use crate::kernel::term::{Binder, Constant, Definition, Field, Shape, Term};
use crate::kernel::value::{Form, Value};

/// Require the kernel to agree that `term` has type `ty` in `cx`.
///
/// The entry point every caller wants: a declaration knows the type it claimed,
/// so the question is whether the kernel agrees rather than what the kernel
/// would have guessed. `ty` is a [`Value`] because every caller has one —
/// elaboration evaluated it to check against, and the conformance suite
/// evaluates it to ask.
///
/// # Errors
///
/// [`CoreError::Malformed`] when the kernel does not agree, naming the subterm
/// it disagreed at; [`CoreError::Exhausted`] when the context's budget ends the
/// derivation, which is not a verdict either way.
pub(crate) fn recheck(cx: &Cx, ty: &Value, term: &Checked) -> Result<(), CoreError> {
    let mut meter = cx.meter();
    check(cx, &mut meter, term.term(), ty)
}

/// What the kernel disagrees with elaboration about, if anything.
///
/// The shape the debug-time audit needs, and the two things it drops are the
/// point. **Exhaustion is not a disagreement**: re-checking runs on its own
/// meter at the context's budget, so a large declaration may run out of steps,
/// and turning that into a panic would make a debug build reject programs a
/// release build accepts. **A δ-rule's refusal is not one either**: that is a
/// host rule answering about the author's own arguments, which the re-checker
/// has no opinion about.
pub(crate) fn disagreement(cx: &Cx, ty: &Value, term: &Term) -> Option<Malformed> {
    let checked = match Checked::try_from(term.clone()) {
        Ok(checked) => checked,
        Err(CoreError::Malformed(fault)) => return Some(fault),
        Err(CoreError::Exhausted(_) | CoreError::Refused { .. }) => return None,
    };
    match recheck(cx, ty, &checked) {
        Ok(()) | Err(CoreError::Exhausted(_) | CoreError::Refused { .. }) => None,
        Err(CoreError::Malformed(fault)) => Some(fault),
    }
}

/// The universe a checked type inhabits.
///
/// A structural walk rather than an inference: `Type ℓ` inhabits `ℓ+1`, a
/// function or record type joins its parts, and every other shape a checked
/// type can have stands at `Type 0`: an enumeration, a base type, a variable of
/// type `Type 0`, an application of either.
///
/// Prompt 148 filed this under the elaborator because the ceiling case answered
/// a `Refusal`. Prompt 152 deleted the ceiling — `succ` is total — so what is
/// left is the kernel's own rule, and it cannot fail.
///
/// # Errors
///
/// None today. The signature keeps its `Result` because every caller is in one
/// and because the shapes this walks are the shapes a `Term` can be.
pub(crate) fn universe_of(term: &Term) -> Result<Sort, CoreError> {
    match term.shape() {
        Shape::Universe(level) => Ok(level.succ()),
        Shape::Bind {
            binder: Binder::Pi { ty, .. },
            body,
            ..
        } => Ok(universe_of(ty)?.max(&universe_of(body)?)),
        Shape::RecordType(fields) => fields
            .iter()
            .try_fold(Sort::ZERO, |join, field| Ok(universe_of(&field.term)?.max(&join))),
        Shape::Meta(_)
        | Shape::Var(_)
        | Shape::Named { .. }
        | Shape::Lit(_)
        | Shape::Bind { .. }
        | Shape::App { .. }
        | Shape::Record(_)
        | Shape::Project { .. } => Ok(Sort::ZERO),
    }
}

/// Require `term` to have type `expected` in `cx`.
///
/// Bidirectional for §2's reason rather than for elaboration's: a core λ
/// carries no domain and a record literal no field types, so the two
/// introduction forms have a type read into them and everything else has one
/// derived out.
fn check(cx: &Cx, meter: &mut Meter, term: &Term, expected: &Value) -> Result<(), CoreError> {
    meter.nested("re-checking", |meter| {
        let want = opened(meter, expected)?;
        let want = want.as_ref().unwrap_or(expected);
        // A β-redex is transparent to the direction the term is read in, the
        // way a `let` is, and for the same reason: it *is* a `let`.
        if let Some((under, body)) = peeled(cx, meter, term)? {
            return check(&under, meter, body, want);
        }
        match (term.shape(), &want.form) {
            (
                Shape::Bind {
                    binder: Binder::Lam,
                    body,
                    ..
                },
                Form::Pi { domain, codomain, .. },
            ) => {
                let under = cx.assumed(term.origin(), Arc::clone(domain));
                let variable = Value::var(term.origin(), cx.depth(), Arc::clone(domain));
                let inside = apply_closure(meter, codomain, variable)?;
                check(&under, meter, body, &inside)
            }
            (Shape::Record(written), Form::RecordType(telescope)) => {
                let subject = eval(meter, cx.env(), term)?;
                for Field { name, term: field } in written.iter() {
                    let at = field_type(meter, telescope, &subject, name)?;
                    check(cx, meter, field, &at)?;
                }
                // A literal missing one of the telescope's fields would have
                // been caught above only if the field it *does* have is the one
                // asked for, so the count is asked separately.
                if written.len() == telescope.fields.len() {
                    Ok(())
                } else {
                    Err(mistyped(meter, cx, term, want, want)?)
                }
            }
            // A `let` is transparent to the direction the term is read in: what
            // it binds is checked against the type it states, and its body is
            // read at the type the `let` itself was read at. Without this arm a
            // `let` standing in front of a λ would be *inferred*, and a core λ
            // has no type of its own — which is a fact about λ rather than
            // about the program.
            (
                Shape::Bind {
                    binder: Binder::Let { ty, value },
                    body,
                    ..
                },
                _,
            ) => {
                let under = bound(cx, meter, ty, value)?;
                check(&under, meter, body, want)
            }
            _ => {
                let found = infer(cx, meter, term)?;
                if same(meter, cx, &found, want)? {
                    Ok(())
                } else {
                    Err(mistyped(meter, cx, term, want, &found)?)
                }
            }
        }
    })
}

/// Derive `term`'s type in `cx`.
fn infer(cx: &Cx, meter: &mut Meter, term: &Term) -> Result<Value, CoreError> {
    let here = term.origin();
    // `(λz. b) a` is `let z = a in b`: the same step, written the other way
    // round. Reading it as the `let` it is definitionally equal to is what lets
    // the audit see through a redex a Curry-style λ gives it no other way in —
    // the λ has no domain, and the argument's type is the domain.
    if let Some((under, body)) = peeled(cx, meter, term)? {
        return infer(&under, meter, body);
    }
    match term.shape() {
        // Its type is written in the context, at the binder it names. Reaching
        // past the context is the scope-discipline failure this pass exists to
        // catch — see `TRUST.md`'s third invariant.
        Shape::Var(index) => cx
            .binder_types()
            .get(index.0)
            .map(|ty| Value::clone(ty))
            .ok_or_else(|| Malformed::UnboundVariable(*index).into()),
        // The same four answers evaluation gives a name, asked for the type
        // rather than the value. Reading them here rather than evaluating and
        // asking the neutral keeps the rule readable as a rule.
        Shape::Named { name, role, levels } => {
            let globals = cx.globals();
            match globals.definition(name, role, levels) {
                Definition::Declared(constant) => constant.ty(meter, globals),
                // The type *at the levels the term names*, which is this
                // pass's universe-polymorphism obligation: a use that
                // instantiated a declaration's parameters inconsistently
                // derives a type the term around it does not accept, and the
                // audit sees a [`Malformed::Mistyped`] rather than nothing.
                Definition::Defined(def) => def.instance(meter, globals, levels).map(|(ty, _)| Value::clone(&ty)),
                Definition::Base(base) => eval(meter, &crate::kernel::value::Env::under(globals.clone()), base.kind()),
                Definition::Builtin(builtin) => {
                    eval(meter, &crate::kernel::value::Env::under(globals.clone()), builtin.ty())
                }
                Definition::Undeclared => Err(Malformed::UndeclaredName(Arc::clone(name)).into()),
            }
        }
        Shape::Lit(constant) => literal_type(cx, meter, here, constant),
        Shape::Universe(level) => Ok(Value::new(here, Form::Universe(level.succ()))),
        Shape::Bind {
            binder: Binder::Pi { ty, .. },
            body,
            ..
        } => {
            let domain = universe(cx, meter, ty)?;
            let evaluated = Arc::new(eval(meter, cx.env(), ty)?);
            let under = cx.assumed(here, evaluated);
            let codomain = universe(&under, meter, body)?;
            Ok(Value::new(here, Form::Universe(domain.max(&codomain))))
        }
        Shape::Bind {
            binder: Binder::Let { ty, value },
            body,
            ..
        } => {
            let under = bound(cx, meter, ty, value)?;
            infer(&under, meter, body)
        }
        // A core λ carries no domain: §2 makes it a checking form, and
        // inferring one means the term around it gave it no type.
        Shape::Bind {
            binder: Binder::Lam, ..
        } => Err(Malformed::Uninferable.into()),
        Shape::App { function, argument } => {
            let of = infer(cx, meter, function)?;
            let of = opened(meter, &of)?.unwrap_or(of);
            match of.form {
                Form::Pi { domain, codomain, .. } => {
                    check(cx, meter, argument, &domain)?;
                    let supplied = eval(meter, cx.env(), argument)?;
                    apply_closure(meter, &codomain, supplied)
                }
                Form::Universe(_)
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Lit(_)
                | Form::Numeral(_)
                | Form::Neutral(_) => Err(Malformed::NotAFunction.into()),
            }
        }
        Shape::RecordType(fields) => {
            let mut join = Sort::ZERO;
            let mut under = cx.clone();
            for Field { name: _, term: ty } in fields.iter() {
                join = join.max(&universe(&under, meter, ty)?);
                let evaluated = Arc::new(eval(meter, under.env(), ty)?);
                under = under.assumed(here, evaluated);
            }
            Ok(Value::new(here, Form::Universe(join)))
        }
        // A record literal inhabits every record type its fields fit, so §2
        // gives it no inference rule and neither does this.
        Shape::Record(_) => Err(Malformed::Uninferable.into()),
        // `{f = a, …}.f` is `a`: the projection rule, not a new one. A record
        // literal has no type of its own — §2 makes it a checking form because
        // the type it "obviously" has is a guess — so reducing the projection is
        // the only way in, and it is the way the evaluator goes too.
        Shape::Project { record, field } if matches!(record.shape(), Shape::Record(_)) => {
            let Shape::Record(fields) = record.shape() else {
                return Err(Malformed::Uninferable.into());
            };
            fields
                .iter()
                .find(|written| &written.name == field)
                .ok_or_else(|| CoreError::from(Malformed::NoSuchField(Arc::clone(field))))
                .and_then(|written| infer(cx, meter, &written.term))
        }
        Shape::Project { record, field } => {
            let of = infer(cx, meter, record)?;
            let of = opened(meter, &of)?.unwrap_or(of);
            match of.form {
                Form::RecordType(telescope) => {
                    let subject = eval(meter, cx.env(), record)?;
                    field_type(meter, &telescope, &subject, field)
                }
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::Record(_)
                | Form::Lit(_)
                | Form::Numeral(_)
                | Form::Neutral(_) => Err(Malformed::NotARecord.into()),
            }
        }
        // `Checked` is what stops an *unsolved* one arriving; reaching this
        // with one means a caller went around it. A solved one is a different
        // fact, and the one this arm exists for — see [`solved`].
        Shape::Meta(meta) => solved(cx, meter, meta),
    }
}

/// A solved unknown's type, and the second check that its solution stayed in
/// scope.
///
/// **The scope check made twice, and the second time is this one.**
/// `02-core-calculus.md` §2.1 admits a solution only when it "mentions no
/// variable outside" the unknown's scope, and [`crate::kernel::unify::assign`]
/// enforces that where it writes one — by reading the value back at the arity,
/// where a variable from outside is a level the depth does not name. That is
/// the check a *correct* unifier makes. Verification that trusts the pass it
/// verifies verifies nothing, so this pass makes it again, over the solution
/// that was actually stored and by whatever route it got there. Without it a
/// capture is silent: the term re-checks, the program runs, and the variable it
/// names is whichever binder happens to stand at that index.
///
/// The type is derived the same way and for the same reason: not read off
/// anything the elaborator recorded, but walked out of the unknown's own
/// telescope with the scope this context holds.
///
/// # Errors
///
/// [`Malformed::UnsolvedMeta`] when nothing solved it — `Checked` should have
/// stopped that one — [`Malformed::EscapedSolution`] when the solution names a
/// variable from outside the scope, and [`Malformed::MetaTelescope`] when the
/// unknown's type does not have the telescope its arity claims or this context
/// is shallower than that arity.
fn solved(cx: &Cx, meter: &mut Meter, meta: &crate::kernel::meta::Meta) -> Result<Value, CoreError> {
    if !meta.is_solved() {
        return Err(Malformed::UnsolvedMeta(meta.id()).into());
    }
    let Some((body, goal)) = crate::kernel::unify::opened_solution(meter, meta)? else {
        return Err(Malformed::UnsolvedMeta(meta.id()).into());
    };
    let scope = crate::kernel::term::Level(meta.arity());
    // Only the escape is renamed. Reading a solution back can fail for the
    // ordinary reasons any read-back fails — a budget, a term that does not fit
    // the type it is read at — and answering all of them with one sentence
    // about scope would be a diagnostic that is wrong most of the times it
    // fires.
    match crate::kernel::quote::quote(meter, scope, Mode::Open, &goal, &body) {
        Ok(_) => {}
        Err(CoreError::Malformed(Malformed::EscapedVariable)) => {
            return Err(Malformed::EscapedSolution(meta.id()).into());
        }
        Err(other) => return Err(other),
    }
    let depth = cx.env().depth().0;
    let mut ty = meta.ty().clone();
    for level in 0..meta.arity() {
        let index = depth
            .checked_sub(level.saturating_add(1))
            .ok_or(Malformed::MetaTelescope(meta.id()))?;
        let argument = cx.env().get(index).ok_or(Malformed::MetaTelescope(meta.id()))?.clone();
        let unfolded = opened(meter, &ty)?;
        let current = unfolded.as_ref().unwrap_or(&ty);
        let Form::Pi { codomain, .. } = &current.form else {
            return Err(Malformed::MetaTelescope(meta.id()).into());
        };
        let codomain = codomain.clone();
        ty = apply_closure(meter, &codomain, argument)?;
    }
    Ok(ty)
}

/// `cx` extended by a `let`'s binder, its stated type checked and its value
/// checked against it.
///
/// The binder goes into the environment as its *value* rather than as a fresh
/// variable, so a type derived under it cannot mention a binder that stops
/// existing when the `let` does — which is why a δ-step needs no substitution
/// here any more than it does anywhere else in this crate.
fn bound(cx: &Cx, meter: &mut Meter, ty: &Term, value: &Term) -> Result<Cx, CoreError> {
    universe(cx, meter, ty)?;
    let stated = Arc::new(eval(meter, cx.env(), ty)?);
    check(cx, meter, value, &stated)?;
    let evaluated = eval(meter, cx.env(), value)?;
    cx.defined(meter, stated, evaluated)
}

/// A β-redex spine read as the nested `let` the evaluator reduces it to, or
/// [`None`] where `term` is not one.
///
/// `(λx. λy. b) a₁ a₂` is `let x = a₁ in let y = a₂ in b`, so the answer is the
/// context those bindings make and the body they stand in front of. This is
/// [`bound`] along a spine, with each binder's type inferred rather than
/// written — a λ carries no domain, and the argument is what determines it.
///
/// The whole spine is taken at once rather than one application at a time,
/// because each binding pushed is a binder the *later* arguments do not stand
/// under: `a₂` was written outside `x`, and reading it in the extended context
/// would read its indices one binder off. So every argument is read in `cx`,
/// and only the body descends.
fn peeled<'t>(cx: &Cx, meter: &mut Meter, term: &'t Term) -> Result<Option<(Cx, &'t Term)>, CoreError> {
    let mut arguments = Vec::new();
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument);
        head = function;
    }
    // A λ with nothing applied to it is not a redex, and answering with the
    // term itself would be an answer that never gets smaller.
    if arguments.is_empty()
        || !matches!(
            head.shape(),
            Shape::Bind {
                binder: Binder::Lam,
                ..
            }
        )
    {
        return Ok(None);
    }
    // The spine was walked from the outside in, so the argument applied first
    // is the one found last.
    arguments.reverse();
    let mut under = cx.clone();
    let mut body = head;
    for argument in arguments {
        let Shape::Bind {
            binder: Binder::Lam,
            body: inside,
            ..
        } = body.shape()
        else {
            // More arguments than the head has binders. What is left applies a
            // term that now stands under the bindings to arguments that do not,
            // which is not a term this pass can read — and not one elaboration
            // builds, since the λs it applies are the ones it wrote.
            return Ok(None);
        };
        let stated = Arc::new(infer(cx, meter, argument)?);
        let evaluated = eval(meter, cx.env(), argument)?;
        under = under.defined(meter, stated, evaluated)?;
        body = inside;
    }
    Ok(Some((under, body)))
}

/// The universe `ty` inhabits, having first required that it is a type.
fn universe(cx: &Cx, meter: &mut Meter, ty: &Term) -> Result<Sort, CoreError> {
    let of = infer(cx, meter, ty)?;
    let of = opened(meter, &of)?.unwrap_or(of);
    match of.form {
        Form::Universe(level) => Ok(level),
        Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Lit(_)
        | Form::Numeral(_)
        | Form::Neutral(_) => Err(Malformed::NotAType.into()),
    }
}

/// The type a closed constant stands at.
fn literal_type(cx: &Cx, meter: &mut Meter, here: Origin, constant: &Constant) -> Result<Value, CoreError> {
    let globals = cx.globals();
    match constant {
        // A payload's type is the term the host registered it at, closed in
        // binders rather than in names — so it reads in the table's own
        // environment and not in this context's.
        Constant::Payload(literal) => eval(meter, &crate::kernel::value::Env::under(globals.clone()), literal.ty()),
        // A numeral's type is the counting family it says it counts at.
        Constant::Numeral(numeral) => Ok(numeral.family.value(here, globals)),
    }
}

/// Whether two types are the same, by `quote ∘ eval` compared as terms.
fn same(meter: &mut Meter, cx: &Cx, found: &Value, expected: &Value) -> Result<bool, CoreError> {
    let found = quote_type(meter, cx.depth(), Mode::Open, found)?;
    let expected = quote_type(meter, cx.depth(), Mode::Open, expected)?;
    Ok(found == expected)
}

/// The disagreement, with both types read back so a reader can see them.
fn mistyped(meter: &mut Meter, cx: &Cx, term: &Term, expected: &Value, found: &Value) -> Result<CoreError, CoreError> {
    Ok(Malformed::Mistyped {
        at: term.origin(),
        expected: quote_type(meter, cx.depth(), Mode::Open, expected)?,
        found: quote_type(meter, cx.depth(), Mode::Open, found)?,
    }
    .into())
}
