//! `Storable`: a structural fact, discharged by computing it.
//!
//! `02-core-calculus.md` §1.2, as the course correction rewrote it: storability
//! is a fact about a type's shape — no function at any depth — that the checker
//! computes where the language needs it, which is a machine port's signature
//! (§2.3) and nothing else. It is not a trait, there is no instance table, and
//! no source program can name it: after prompt 146 there is no declaration form
//! a program could name it *from*.
//!
//! This module holds the three pieces that remain. [`requiring_storable`] is
//! how the host writes the constraint into a machine constructor's scheme — a
//! constrained Π over the reserved name. [`discharge`] is what answers it at
//! elaboration, and [`is_storable`] is the structural walk that decides it,
//! where the old implementation generated an instance per declaration group and
//! resolved it from a table.
//!
//! After prompt 146 this is the **only** constraint there is. The trait system
//! is gone, no source program can write a `where` clause, and
//! [`Filling::Constraint`](crate::kernel::term::Filling::Constraint) is reachable only
//! through [`requiring_storable`] — so [`discharge`] has no lookup in it and
//! nothing to prefer one answer to another.

use std::sync::Arc;

use crate::kernel::error::CoreError;
use crate::kernel::family::Role;
use crate::kernel::origin::Origin;
use crate::kernel::term::{Constraint, Level, Term};
use crate::kernel::value::{Elim, Form, Head, Value};

/// The one constraint name this crate reserves and no author may write.
pub(crate) const STORABLE: &str = "Storable";

/// `Storable argument` as a constrained Π over `codomain` — the way a machine
/// constructor's scheme states that a port stores data.
///
/// The evidence the constraint would hold is the empty record: the check itself
/// is the evidence, and there is nothing for a use site to pass.  [`discharge`]
/// answers the constraint by computing [`is_storable`], which is why this one
/// constraint survived the deletion of the mechanism that once carried it —
/// nothing about it was ever a table lookup.
#[must_use]
pub fn requiring_storable(origin: Origin, argument: Term, codomain: Term) -> Term {
    let constraint = Arc::new(Constraint {
        origin,
        class: Arc::from(STORABLE),
        args: Arc::from(vec![argument]),
    });
    Term::constrained_pi(
        origin,
        constraint,
        STORABLE,
        Term::record_type(origin, core::iter::empty()),
        codomain,
    )
}

/// Answer a `Storable` constraint: compute it, and hand back the evidence.
///
/// The one resolution step in the crate, and it is a computation rather than a
/// lookup. `constraint`'s argument is read under `env` — the environment the
/// constraint was *written* against — evaluated, and walked by [`is_storable`];
/// the answer is the empty record, because the check is the evidence.
///
/// # Errors
///
/// [`Refusal::Unsolved`] when the port's type is an unsolved metavariable — a
/// port nothing determined is *undetermined*, not unstorable, and the refusal
/// is the meta's rather than a verdict about a type nobody wrote.
/// [`Refusal::NotStorable`] naming the type when the walk says no, and
/// [`Malformed::NotAType`](crate::Malformed::NotAType) when the argument is no
/// type at all, which is a caller defect.
pub(crate) fn discharge(
    elaborator: &mut crate::elaboration::elab::Elaborator,
    cx: &crate::kernel::context::Cx,
    constraint: &Constraint,
    env: &crate::kernel::value::Env,
    at: Origin,
) -> Result<Term, crate::elaboration::refuse::ElabError> {
    let depth = Level(u32::try_from(env.iter().count()).unwrap_or(u32::MAX));
    let Some(argument) = constraint.args.first() else {
        return Err(crate::kernel::error::Malformed::NotAType.into());
    };
    let ty = crate::kernel::eval::eval(elaborator.meter(), env, argument)?;
    let written = crate::kernel::quote::quote_type(elaborator.meter(), depth, crate::kernel::quote::Mode::Open, &ty)?;
    if let Some(opened) = crate::kernel::eval::opened(elaborator.meter(), &ty)?
        && let Form::Neutral(neutral) = &opened.form
        && let crate::kernel::value::Head::Meta(meta) = &neutral.head
        && !meta.is_solved()
    {
        return Err(crate::elaboration::refuse::Refusal::Unsolved {
            site: crate::kernel::meta::MetaSource::TypeParameter,
            created: meta.origin(),
            blocked: None,
        }
        .into());
    }
    if is_storable(elaborator.meter(), cx, &ty)? {
        Ok(Term::record(at, core::iter::empty()))
    } else {
        Err(crate::elaboration::refuse::Refusal::NotStorable { at, ty: written }.into())
    }
}

/// Is this type storable data — no function at any depth (§1.2)?
///
/// The walk is over the *value* the type evaluates to, because that is where the
/// question is asked: a machine port's type at a concrete call site. A family
/// asks the question of its constructors' fields at the spine's arguments, and a
/// family already being asked assumes itself storable — the coinductive reading
/// of §1.2's "checked once per declaration group": a recursive family is
/// storable exactly when its fields' other types are.
///
/// # Errors
///
/// [`crate::Malformed::NotAType`] if the value is no type at all — a compiler
/// defect, since a constraint's argument is checked before it is discharged.
pub(crate) fn is_storable(
    meter: &mut crate::kernel::budget::Meter,
    cx: &crate::kernel::context::Cx,
    ty: &Value,
) -> Result<bool, CoreError> {
    stor(meter, cx, ty, &mut Vec::new(), &mut Level(0))
}

/// [`is_storable`], under `visiting` (the families currently being decided, by
/// group identity) and `depth` (the next fresh variable's level).
fn stor(
    meter: &mut crate::kernel::budget::Meter,
    cx: &crate::kernel::context::Cx,
    ty: &Value,
    visiting: &mut Vec<(usize, u32)>,
    depth: &mut Level,
) -> Result<bool, CoreError> {
    // A meta solved after this value was built still heads it — open first,
    // which is also what unfolds a definition standing in type position.
    let opened = crate::kernel::eval::opened(meter, ty)?;
    let ty = opened.as_ref().unwrap_or(ty);
    match &ty.form {
        // A function is never storable, and neither is a type standing where
        // data should: §1.2's two negative rules.
        Form::Pi { .. } | Form::Lam(_) | Form::Universe(_) => Ok(false),
        Form::RecordType(telescope) => {
            let mut env = telescope.env.clone();
            for field in telescope.fields.iter() {
                let field_ty = crate::kernel::eval::eval(meter, &env, &field.term)?;
                if !stor(meter, cx, &field_ty, visiting, depth)? {
                    return Ok(false);
                }
                let fresh = Value::var(ty.origin, *depth, Arc::new(field_ty));
                *depth = depth.deeper();
                env = env.push(fresh);
            }
            Ok(true)
        }
        Form::Neutral(neutral) => match &neutral.head {
            Head::Base(base, _) => {
                // The flag is the *registry's*: a base type's term is written
                // at many sites and `Base` compares by name, so the decorated
                // copy is the registered one — the same authority the carrier
                // rule in `elab` reads. A base the registry does not know is
                // the host's own object, and its own flag answers.
                let registered = match cx.extern_named(base.name()) {
                    Some(crate::kernel::base::Extern::Base(declared)) => declared.is_storable(),
                    Some(crate::kernel::base::Extern::Builtin(_)) | None => base.is_storable(),
                };
                Ok(registered)
            }
            Head::Const(constant, _) => match &constant.role {
                Role::Family => {
                    let key = (Arc::as_ptr(&constant.group) as usize, constant.family);
                    if visiting.contains(&key) {
                        return Ok(true);
                    }
                    let Some(declared) = constant.group.family_at(constant.family) else {
                        return Ok(true);
                    };
                    let params = constant.group.params.len();
                    let mut arguments = Vec::with_capacity(params);
                    for elimination in neutral.spine.iter().take(params) {
                        let Elim::App { argument, .. } = elimination else {
                            return Ok(false);
                        };
                        arguments.push(Value::clone(argument));
                    }
                    visiting.push(key);
                    let mut answer = true;
                    for constructor in declared.constructors.iter() {
                        // The declaration's own environment first: a field
                        // type's variables name the group's families (a
                        // recursive occurrence most of all) and then its
                        // parameters, which the spine's arguments answer.
                        let mut env = crate::kernel::family::Group::declarations(&constant.group, cx.globals());
                        for argument in &arguments {
                            env = env.push(argument.clone());
                        }
                        for field in constructor.fields.iter() {
                            let field_ty = crate::kernel::eval::eval(meter, &env, &field.ty)?;
                            if !stor(meter, cx, &field_ty, visiting, depth)? {
                                answer = false;
                                break;
                            }
                            let fresh = Value::var(ty.origin, *depth, Arc::new(field_ty));
                            *depth = depth.deeper();
                            env = env.push(fresh);
                        }
                        if !answer {
                            break;
                        }
                    }
                    visiting.pop();
                    Ok(answer)
                }
                // A stuck elimination in type position cannot be shown to hold
                // no function, and a constructor is not a type at all. Both are
                // the conservative answer; the second is a compiler defect.
                Role::Recursor(_) => Ok(false),
                Role::Constructor(_) => Err(crate::kernel::error::Malformed::NotAType.into()),
            },
            // An unknown type could hold a function, and a definition or a
            // builtin stuck at the head of one is no more decidable: refused.
            Head::Var(_, _) | Head::Def(_, _, _) | Head::Builtin(..) | Head::Meta(_) => Ok(false),
        },
        // A checked type never evaluates to one of these; reaching one is a
        // compiler defect rather than a program's fault.
        Form::Record(_) | Form::Lit(_) | Form::Numeral(_) => Err(crate::kernel::error::Malformed::NotAType.into()),
    }
}
