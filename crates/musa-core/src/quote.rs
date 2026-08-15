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
//!   argument's type from the spine through [`crate::eval::head_type`].
//!
//! **Every value quotation matches on is forced first.** A value built before a
//! metavariable was solved still says "blocked"; matching it unforced would read
//! an unsolved `?α` back into a normal form that has one, and answer
//! [`Malformed::NotAFunction`] for a term the elaborator had just accepted (§2.1).
//! Forcing is only ever *at the head*, so a solved meta buried under a Π's
//! codomain is unfolded when quotation reaches it, not before.
//!
//! **Every node written here carries an origin, and §7 fixes which one.** A node
//! that reads a value back takes that *value's* origin — never the type's, which
//! belongs to a different term and would point a reader at the signature when
//! the mistake is in the expression. Where quotation η-expands, "the expansion
//! carries the origin of the term it expanded", so the λ written at a Π and the
//! literal written at a record type both take the expanded value's origin. The
//! variables quotation invents have no surface node at all and take
//! [`Origin::UNKNOWN`] rather than a plausible-looking guess.

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::{CoreError, Malformed};
use crate::eval::{apply, apply_closure, field_type, force, head_type, project};
use crate::origin::Origin;
use crate::term::{DbLevel, Field, Term};
use crate::value::{Form, Neutral, Spine, Telescope, Value};

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
        // `as_ref().unwrap_or` rather than `unwrap_or_else(clone)`: the common
        // case is a value with no metavariable in it, and that case must not pay
        // an allocation per quoted node.
        let unfolded_ty = force(meter, ty)?;
        let ty = unfolded_ty.as_ref().unwrap_or(ty);
        let unfolded_value = force(meter, value)?;
        let value = unfolded_value.as_ref().unwrap_or(value);
        let here = value.origin;
        match &ty.form {
            // η at Π: a lambda, whether or not the value is one. A λ has no
            // plicity to write — it is the Π that says how the argument arrives.
            Form::Pi {
                plicity: _,
                name,
                domain,
                codomain,
            } => {
                let variable = Value::var(Origin::UNKNOWN, depth.fresh(), Arc::clone(domain));
                let body_type = apply_closure(meter, codomain, variable.clone())?;
                let body = apply(meter, here, value.clone(), variable)?;
                Ok(Term::lam(
                    here,
                    Arc::clone(name),
                    quote(meter, depth.under_binder(), &body_type, &body)?,
                ))
            }
            // η at records: a literal holding every projection.
            Form::RecordType(telescope) => {
                let mut fields = Vec::with_capacity(telescope.fields.len());
                for Field { name, term: _ } in telescope.fields.iter() {
                    let field_ty = field_type(meter, telescope, value, name)?;
                    let field_value = project(meter, here, value.clone(), name)?;
                    fields.push(Field {
                        name: Arc::clone(name),
                        term: quote(meter, depth, &field_ty, &field_value)?,
                    });
                }
                Ok(Term::new(here, crate::term::Shape::Record(fields.into())))
            }
            Form::Universe(_) => quote_type(meter, depth, value),
            Form::Id { ty: at, .. } => match &value.form {
                Form::Refl(witness) => Ok(Term::refl(here, quote(meter, depth, at, witness)?)),
                Form::Neutral(neutral) => quote_neutral(meter, depth, neutral),
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Id { .. } => Err(Malformed::NotAnIdentity.into()),
            },
            // A neutral type has no η, so whatever inhabits it is neutral too.
            Form::Neutral(_) => match &value.form {
                Form::Neutral(neutral) => quote_neutral(meter, depth, neutral),
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Id { .. }
                | Form::Refl(_) => quote_type(meter, depth, value),
            },
            Form::Lam(_) | Form::Record(_) | Form::Refl(_) => Err(Malformed::NotAType.into()),
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
        let unfolded = force(meter, value)?;
        let value = unfolded.as_ref().unwrap_or(value);
        let here = value.origin;
        match &value.form {
            Form::Universe(level) => Ok(Term::universe(here, level.resolved())),
            Form::Pi {
                plicity,
                name,
                domain,
                codomain,
            } => {
                let variable = Value::var(Origin::UNKNOWN, depth.fresh(), Arc::clone(domain));
                let opened = apply_closure(meter, codomain, variable)?;
                Ok(Term::function(
                    here,
                    *plicity,
                    Arc::clone(name),
                    quote_type(meter, depth, domain)?,
                    quote_type(meter, depth.under_binder(), &opened)?,
                ))
            }
            Form::RecordType(telescope) => quote_telescope(meter, here, depth, telescope),
            Form::Id { ty, left, right } => Ok(Term::identity(
                here,
                quote_type(meter, depth, ty)?,
                quote(meter, depth, ty, left)?,
                quote(meter, depth, ty, right)?,
            )),
            Form::Neutral(neutral) => quote_neutral(meter, depth, neutral),
            Form::Lam(_) | Form::Record(_) | Form::Refl(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read a record type's telescope back, binding each field as it goes.
///
/// Unlike [`quote`]'s record case, the earlier fields become *variables* rather
/// than projections, because a record type binds them and a record value only
/// has them.
fn quote_telescope(meter: &mut Meter, here: Origin, depth: Depth, telescope: &Telescope) -> Result<Term, CoreError> {
    let mut env = telescope.env.clone();
    let mut at = depth;
    let mut fields = Vec::with_capacity(telescope.fields.len());
    for Field { name, term } in telescope.fields.iter() {
        let field_ty = crate::eval::eval(meter, &env, term)?;
        fields.push(Field {
            name: Arc::clone(name),
            term: quote_type(meter, at, &field_ty)?,
        });
        env = env.push(Value::var(Origin::UNKNOWN, at.fresh(), Arc::new(field_ty)));
        at = at.under_binder();
    }
    Ok(Term::new(here, crate::term::Shape::RecordType(fields.into())))
}

/// Read a blocked elimination back.
///
/// Each argument is quoted at the type the spine gives it, which is why
/// [`head_type`] exists: an argument quoted untyped would not be η-expanded,
/// and `f g` would read back differently from `f (λx. g x)`.
fn quote_neutral(meter: &mut Meter, depth: Depth, neutral: &Neutral) -> Result<Term, CoreError> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        let here = neutral.origin;
        match &neutral.spine {
            Spine::Var(level, _) => level
                .to_index(depth.0)
                .map(|index| Term::var(here, index))
                .ok_or_else(|| Malformed::EscapedVariable.into()),
            // Reached only unsolved: [`quote`] forces first, and a spine whose
            // innermost head is solved forces whole.
            Spine::Meta(meta) => Ok(Term::meta(here, meta.clone())),
            Spine::Const(constant) => Ok(constant.term(here)),
            Spine::App { function, argument } => {
                let domain = match head_type(meter, function)?.form {
                    Form::Pi { domain, .. } => domain,
                    Form::Universe(_)
                    | Form::Lam(_)
                    | Form::RecordType(_)
                    | Form::Record(_)
                    | Form::Id { .. }
                    | Form::Refl(_)
                    | Form::Neutral(_) => return Err(Malformed::NotAFunction.into()),
                };
                Ok(Term::app(
                    here,
                    quote_neutral(meter, depth, function)?,
                    quote(meter, depth, &domain, argument)?,
                ))
            }
            Spine::Project { record, field } => Ok(Term::project(
                here,
                quote_neutral(meter, depth, record)?,
                Arc::clone(field),
            )),
            Spine::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            } => {
                let base_type = {
                    let at_from = apply(meter, here, Value::clone(motive), Value::clone(from))?;
                    let reflexive = Value::new(from.origin, Form::Refl(Arc::clone(from)));
                    apply(meter, here, at_from, reflexive)?
                };
                Ok(Term::jay(
                    here,
                    quote_type(meter, depth, ty)?,
                    quote(meter, depth, ty, from)?,
                    quote_motive(meter, depth, ty, from, motive)?,
                    quote(meter, depth, &base_type, base)?,
                    quote(meter, depth, ty, to)?,
                    quote_neutral(meter, depth, proof)?,
                ))
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
    let here = motive.origin;
    let endpoint = Value::var(Origin::UNKNOWN, depth.fresh(), Arc::clone(ty));
    let under_endpoint = depth.under_binder();
    let identity = Arc::new(Value::new(
        here,
        Form::Id {
            ty: Arc::clone(ty),
            left: Arc::clone(from),
            right: Arc::new(endpoint.clone()),
        },
    ));
    let witness = Value::var(Origin::UNKNOWN, under_endpoint.fresh(), identity);
    let at_endpoint = apply(meter, here, motive.clone(), endpoint)?;
    let body = apply(meter, here, at_endpoint, witness)?;
    Ok(Term::lam(
        here,
        "y",
        Term::lam(here, "e", quote_type(meter, under_endpoint.under_binder(), &body)?),
    ))
}
