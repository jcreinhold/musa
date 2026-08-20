//! `Storable`: a structural fact, discharged by computing it.
//!
//! `02-core-calculus.md` §1.2, as the course correction rewrote it: storability
//! is a fact about a type's shape — no function at any depth — that the checker
//! computes where the language needs it, which is a machine port's signature
//! (§2.3) and nothing else. It is not a trait, there is no instance table, and
//! no source program can name it: the two guards that keep it that way are
//! `declare_trait`'s and `declare_impl`'s ordinary reserved-name refusals.
//!
//! This module holds the two pieces that remain. [`requiring_storable`] is how
//! the host writes the constraint into a machine constructor's scheme — a
//! constrained Π over the reserved name, exactly as before. [`is_storable`] is
//! what discharges that constraint at elaboration: a structural walk over the
//! type the port turned out to have, where the old implementation generated an
//! instance per declaration group and resolved it from a table.

use std::sync::Arc;

use crate::class::Constraint;
use crate::error::CoreError;
use crate::family::Role;
use crate::list::List;
use crate::origin::Origin;
use crate::term::{DbLevel, Term};
use crate::value::{Elim, Form, Head, Value};


/// The one constraint name this crate reserves and no author may write.
pub(crate) const STORABLE: &str = "Storable";

/// `Storable argument` as a constrained Π over `codomain` — the way a machine
/// constructor's scheme states that a port stores data.
///
/// The dictionary the constraint would hold is the empty record: the evidence
/// is the check itself, which is §1.2's "the dictionary is empty" stated as a
/// construction. [`crate::dictionary`] discharges the constraint by computing
/// [`is_storable`], never by table lookup.
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
    meter: &mut crate::budget::Meter,
    ty: &Value,
) -> Result<bool, CoreError> {
    stor(meter, ty, &mut Vec::new(), &mut 0)
}

/// [`is_storable`], under `visiting` (the families currently being decided, by
/// group identity) and `depth` (the next fresh variable's level).
fn stor(
    meter: &mut crate::budget::Meter,
    ty: &Value,
    visiting: &mut Vec<(usize, u32)>,
    depth: &mut u32,
) -> Result<bool, CoreError> {
    match &ty.form {
        // A function is never storable, and neither is a type standing where
        // data should: §1.2's two negative rules.
        Form::Pi { .. } | Form::Lam(_) | Form::Universe(_) => Ok(false),
        Form::RecordType(telescope) => {
            let mut env = telescope.env.clone();
            for field in telescope.fields.iter() {
                let field_ty = crate::eval::eval(meter, &env, &field.term)?;
                if !stor(meter, &field_ty, visiting, depth)? {
                    return Ok(false);
                }
                let fresh = Value::var(ty.origin, DbLevel(*depth), Arc::new(field_ty));
                *depth = depth.saturating_add(1);
                env = env.push(fresh);
            }
            Ok(true)
        }
        Form::Neutral(neutral) => match &neutral.head {
            Head::Base(base) => Ok(base.is_storable()),
            Head::Const(constant) => match &constant.role {
                Role::Family => {
                    let key = (Arc::as_ptr(&constant.group) as usize, constant.family);
                    if visiting.contains(&key) {
                        return Ok(true);
                    }
                    let Some(declared) = constant.group.family_at(constant.family) else {
                        return Ok(true);
                    };
                    let params = usize::try_from(constant.group.params.len()).unwrap_or(usize::MAX);
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
                        let mut env = crate::family::Group::declarations(&constant.group);
                        for argument in &arguments {
                            env = env.push(argument.clone());
                        }
                        for field in constructor.fields.iter() {
                            let field_ty = crate::eval::eval(meter, &env, &field.ty)?;
                            if !stor(meter, &field_ty, visiting, depth)? {
                                answer = false;
                                break;
                            }
                            let fresh = Value::var(ty.origin, DbLevel(*depth), Arc::new(field_ty));
                            *depth = depth.saturating_add(1);
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
                Role::Constructor(_) => Err(crate::error::Malformed::NotAType.into()),
            },
            // An unknown type could hold a function, and a definition or a
            // builtin stuck at the head of one is no more decidable: refused.
            Head::Var(_, _) | Head::Def(_, _, _) | Head::Builtin(_) | Head::Hole(_) => Ok(false),
        },
        // A checked type never evaluates to one of these; reaching one is a
        // compiler defect rather than a program's fault.
        Form::Record(_) | Form::Lit(_) | Form::Numeral(_) => Err(crate::error::Malformed::NotAType.into()),
    }
}
