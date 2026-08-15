//! Quotation: values back to terms, and where η happens.
//!
//! `docs/rules/language/02-core-calculus.md` §3:
//!
//! > η at Π and at records is performed by `quote` rather than by a conversion
//! > rule, which is why two record values with the same projections are
//! > convertible without a rule that inspects both at once.
//!
//! So quotation is **type-directed**. At a Π it writes a lambda whether or not
//! the value is one, applying the value to a fresh variable; at a record type it
//! writes a record literal, projecting every field. What comes back is the
//! η-long normal form, and conversion is then α-equality, which on de Bruijn
//! *indices* is structural `==`.
//!
//! The prompt's sketch signature is `quote : Level → Value → Term`. It cannot
//! be: without the type there is nothing to η-expand *against*, and `f` and
//! `λx. f x` would read back differently. `docs/plan/roadmap.md` §15.12's
//! `convertible(left, right, ty)` already carries the type for this reason, and
//! this module is where the reason lives.
//!
//! Three functions rather than one, because a value stands in three places and
//! each knows a different amount:
//!
//! - [`quote`] reads a value back *at a known type*, and is the only one that
//!   η-expands.
//! - [`quote_type`] reads a value back *as a type*, where the type of the type
//!   is a universe and tells η nothing.
//! - [`quote_neutral`] reads a blocked elimination back, recovering each
//!   argument's type from the spine through [`crate::eval::neutral_type`].

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::{CoreError, Malformed};
use crate::eval::{apply, apply_closure, field_type, neutral_type, project};
use crate::term::{DbLevel, Field, Term};
use crate::value::{Neutral, Telescope, Value};

/// How many binders are in scope while quoting.
///
/// Carried rather than derived because it is what turns a [`DbLevel`] created
/// during quotation into the [`crate::term::Index`] that names it.
#[derive(Clone, Copy)]
pub(crate) struct Depth(pub(crate) u32);

impl Depth {
    fn under_binder(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    fn fresh(self) -> DbLevel {
        DbLevel(self.0)
    }
}

/// Read `value` back as a term of type `ty`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the value does not inhabit the shape the type demands.
pub(crate) fn quote(meter: &mut Meter, depth: Depth, ty: &Value, value: &Value) -> Result<Term, CoreError> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        match ty {
            // η at Π: a lambda, whether or not the value is one.
            Value::Pi { name, domain, codomain } => {
                let variable = Value::var(depth.fresh(), Arc::clone(domain));
                let body_type = apply_closure(meter, codomain, variable.clone())?;
                let body = apply(meter, value.clone(), variable)?;
                Ok(Term::Lam {
                    name: Arc::clone(name),
                    body: Arc::new(quote(meter, depth.under_binder(), &body_type, &body)?),
                })
            }
            // η at records: a literal holding every projection.
            Value::RecordType(telescope) => {
                let mut fields = Vec::with_capacity(telescope.fields.len());
                for Field { name, term: _ } in telescope.fields.iter() {
                    let field_ty = field_type(meter, telescope, value, name)?;
                    let field_value = project(meter, value.clone(), name)?;
                    fields.push(Field {
                        name: Arc::clone(name),
                        term: quote(meter, depth, &field_ty, &field_value)?,
                    });
                }
                Ok(Term::Record(fields.into()))
            }
            Value::Universe(_) => quote_type(meter, depth, value),
            Value::Id { ty: at, .. } => match value {
                Value::Refl(witness) => Ok(Term::Refl(Arc::new(quote(meter, depth, at, witness)?))),
                Value::Neutral(neutral) => quote_neutral(meter, depth, neutral),
                Value::Universe(_)
                | Value::Pi { .. }
                | Value::Lam { .. }
                | Value::RecordType(_)
                | Value::Record(_)
                | Value::Id { .. } => Err(Malformed::NotAnIdentity.into()),
            },
            // A neutral type has no η, so whatever inhabits it is neutral too.
            Value::Neutral(_) => match value {
                Value::Neutral(neutral) => quote_neutral(meter, depth, neutral),
                Value::Universe(_)
                | Value::Pi { .. }
                | Value::Lam { .. }
                | Value::RecordType(_)
                | Value::Record(_)
                | Value::Id { .. }
                | Value::Refl(_) => quote_type(meter, depth, value),
            },
            Value::Lam { .. } | Value::Record(_) | Value::Refl(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read `value` back as a type.
///
/// # Errors
///
/// As [`quote`], plus [`Malformed::NotAType`] when the value is a canonical
/// form no universe contains.
pub(crate) fn quote_type(meter: &mut Meter, depth: Depth, value: &Value) -> Result<Term, CoreError> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        match value {
            Value::Universe(level) => Ok(Term::Universe(*level)),
            Value::Pi { name, domain, codomain } => {
                let variable = Value::var(depth.fresh(), Arc::clone(domain));
                let opened = apply_closure(meter, codomain, variable)?;
                Ok(Term::Pi {
                    name: Arc::clone(name),
                    domain: Arc::new(quote_type(meter, depth, domain)?),
                    codomain: Arc::new(quote_type(meter, depth.under_binder(), &opened)?),
                })
            }
            Value::RecordType(telescope) => quote_telescope(meter, depth, telescope),
            Value::Id { ty, left, right } => Ok(Term::Id {
                ty: Arc::new(quote_type(meter, depth, ty)?),
                left: Arc::new(quote(meter, depth, ty, left)?),
                right: Arc::new(quote(meter, depth, ty, right)?),
            }),
            Value::Neutral(neutral) => quote_neutral(meter, depth, neutral),
            Value::Lam { .. } | Value::Record(_) | Value::Refl(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read a record type's telescope back, binding each field as it goes.
///
/// Unlike [`quote`]'s record case, the earlier fields become *variables* rather
/// than projections, because a record type binds them and a record value only
/// has them.
fn quote_telescope(meter: &mut Meter, depth: Depth, telescope: &Telescope) -> Result<Term, CoreError> {
    let mut env = telescope.env.clone();
    let mut here = depth;
    let mut fields = Vec::with_capacity(telescope.fields.len());
    for Field { name, term } in telescope.fields.iter() {
        let field_ty = crate::eval::eval(meter, &env, term)?;
        fields.push(Field {
            name: Arc::clone(name),
            term: quote_type(meter, here, &field_ty)?,
        });
        env = env.extend(Value::var(here.fresh(), Arc::new(field_ty)));
        here = here.under_binder();
    }
    Ok(Term::RecordType(fields.into()))
}

/// Read a blocked elimination back.
///
/// Each argument is quoted at the type the spine gives it, which is why
/// [`neutral_type`] exists: an argument quoted untyped would not be η-expanded,
/// and `f g` would read back differently from `f (λx. g x)`.
fn quote_neutral(meter: &mut Meter, depth: Depth, neutral: &Neutral) -> Result<Term, CoreError> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        match neutral {
            Neutral::Var(level, _) => level
                .to_index(depth.0)
                .map(Term::Var)
                .ok_or_else(|| Malformed::EscapedVariable.into()),
            Neutral::App { function, argument } => {
                let domain = match neutral_type(meter, function)? {
                    Value::Pi { domain, .. } => domain,
                    Value::Universe(_)
                    | Value::Lam { .. }
                    | Value::RecordType(_)
                    | Value::Record(_)
                    | Value::Id { .. }
                    | Value::Refl(_)
                    | Value::Neutral(_) => return Err(Malformed::NotAFunction.into()),
                };
                Ok(Term::App {
                    function: Arc::new(quote_neutral(meter, depth, function)?),
                    argument: Arc::new(quote(meter, depth, &domain, argument)?),
                })
            }
            Neutral::Project { record, field } => Ok(Term::Project {
                record: Arc::new(quote_neutral(meter, depth, record)?),
                field: Arc::clone(field),
            }),
            Neutral::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => {
                let base_type = {
                    let at_from = apply(meter, Value::clone(motive), Value::clone(from))?;
                    apply(meter, at_from, Value::Refl(Arc::clone(from)))?
                };
                Ok(Term::J {
                    ty: Arc::new(quote_type(meter, depth, ty)?),
                    from: Arc::new(quote(meter, depth, ty, from)?),
                    motive: Arc::new(quote_motive(meter, depth, ty, from, motive)?),
                    base: Arc::new(quote(meter, depth, &base_type, base)?),
                    to: Arc::new(quote(meter, depth, ty, to)?),
                    proof: Arc::new(quote_neutral(meter, depth, proof)?),
                })
            }
        }
    })
}

/// Read `J`'s motive back at the shape `(y : A) → Id A x y → Type l`.
///
/// The motive's Π type is never built as a value, because `l` is not recorded
/// anywhere and building it would mean either storing a level nobody needs or
/// inventing one. η-expanding at the shape gives the same answer: apply the
/// motive to two fresh variables whose types *are* known, and quote the result
/// as a type.
fn quote_motive(
    meter: &mut Meter,
    depth: Depth,
    ty: &Arc<Value>,
    from: &Arc<Value>,
    motive: &Value,
) -> Result<Term, CoreError> {
    let endpoint = Value::var(depth.fresh(), Arc::clone(ty));
    let under_endpoint = depth.under_binder();
    let identity = Arc::new(Value::Id {
        ty: Arc::clone(ty),
        left: Arc::clone(from),
        right: Arc::new(endpoint.clone()),
    });
    let witness = Value::var(under_endpoint.fresh(), identity);
    let at_endpoint = apply(meter, motive.clone(), endpoint)?;
    let body = apply(meter, at_endpoint, witness)?;
    Ok(Term::Lam {
        name: Arc::from("y"),
        body: Arc::new(Term::Lam {
            name: Arc::from("e"),
            body: Arc::new(quote_type(meter, under_endpoint.under_binder(), &body)?),
        }),
    })
}
