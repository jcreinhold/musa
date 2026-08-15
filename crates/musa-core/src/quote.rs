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
//! Three entry points rather than one, because a value stands in three places
//! and each knows a different amount:
//!
//! - [`quote`] reads a value back *at a known type*, and is the only one that
//!   η-expands.
//! - [`quote_type`] reads a value back *as a type*, where the type of the type
//!   is a universe and tells η nothing.
//! - [`quote_solution`] reads a value back *into a metavariable's context*,
//!   which is either of the first two plus the right to refuse.
//!
//! # Why the scope check is here rather than after
//!
//! §2.1 asks a metavariable's solution to mention no variable bound after the
//! metavariable was created, and not to mention the metavariable itself. Both
//! are decided at a leaf — a variable, and a meta — of the walk that writes the
//! term, so both are decided *by that walk*: [`Reading`] carries what the
//! solution may name, and [`Reading::index`] is where scope check, occurs check
//! and index shift all land. A second pass over the finished term would build
//! the whole of it before discovering its first node was already out of scope.
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
use crate::meta::Meta;
use crate::origin::Origin;
use crate::term::{DbLevel, Field, Index, Term};
use crate::value::{Elim, Form, Head, Neutral, Telescope, Value};

/// How many binders are in scope while quoting.
///
/// Carried rather than derived because it is what turns a [`DbLevel`] created
/// during quotation into the [`Index`] that names it.
#[derive(Clone, Copy)]
pub(crate) struct Depth(pub(crate) u32);

/// The depth, and — when the term being written is a metavariable's solution —
/// what that solution is allowed to mention.
///
/// One value rather than two parameters because the two are asked together at
/// exactly one place, [`Reading::index`], and separating them would put the
/// depth in every signature twice.
#[derive(Clone, Copy)]
struct Reading<'a> {
    depth: u32,
    solving: Option<Solving<'a>>,
}

/// A metavariable's solution under construction.
///
/// §2.1 asks three things of a solution — the **scope check** (it mentions no
/// variable bound after the metavariable was created), the **occurs check** (it
/// does not mention the metavariable), and the **index shift** into a context of
/// `arity` binders — and all three are decided at a leaf of the very walk that
/// writes the term. They used to be a second walk over the finished term, which
/// built the whole of it even when the first node already refused.
#[derive(Clone, Copy)]
struct Solving<'a> {
    /// The metavariable being solved. It may not appear in its own solution.
    meta: &'a Meta,
    /// The binders the solution abstracts: levels `0..arity` survive the move.
    arity: u32,
    /// The depth quotation started at. A variable at or above it was introduced
    /// by quotation itself, is local to the solution, and never moves.
    outer: u32,
}

/// Why reading a value back did not produce a term.
///
/// Not a [`CoreError`], because [`Self::OutOfScope`] is not a failure: §2.1
/// answers a constraint it cannot solve by *waiting*, and this is the shape that
/// waiting has here.
enum Escape {
    /// The value mentions a variable the metavariable's context does not have,
    /// or the metavariable itself.
    OutOfScope,
    Core(CoreError),
}

impl From<CoreError> for Escape {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<Malformed> for Escape {
    fn from(error: Malformed) -> Self {
        Self::Core(error.into())
    }
}

impl Escape {
    /// This escape as a [`CoreError`], for a quotation that was not solving.
    ///
    /// Total rather than a hidden panic: [`Self::OutOfScope`] is raised only
    /// under a [`Solving`], which such a quotation does not carry — and if one
    /// ever arrived anyway, "quotation reached a variable outside the scope it
    /// was quoting in" is precisely what [`Malformed::EscapedVariable`] says.
    fn core(self) -> CoreError {
        match self {
            Self::Core(error) => error,
            Self::OutOfScope => Malformed::EscapedVariable.into(),
        }
    }
}

impl<'a> Reading<'a> {
    /// A plain quotation: no metavariable to fit the answer into.
    const fn open(depth: Depth) -> Self {
        Self {
            depth: depth.0,
            solving: None,
        }
    }

    /// A quotation whose answer must live in `meta`'s own context.
    const fn solving(depth: Depth, meta: &'a Meta, arity: u32) -> Self {
        Self {
            depth: depth.0,
            solving: Some(Solving {
                meta,
                arity,
                outer: depth.0,
            }),
        }
    }

    fn under_binder(self) -> Self {
        Self {
            depth: self.depth.saturating_add(1),
            ..self
        }
    }

    const fn fresh(self) -> DbLevel {
        DbLevel(self.depth)
    }

    /// The index that names `level` in the term being written.
    ///
    /// Without a [`Solving`] this is the ordinary level-to-index conversion.
    /// With one, the answer is the same for a variable quotation introduced
    /// itself — the solution has just as many binders inside it — and shifted by
    /// the binders the solution drops for one from the outer context, which is
    /// also where a variable the solution may not mention is refused.
    fn index(self, level: DbLevel) -> Result<Index, Escape> {
        let named = |depth: u32| {
            level
                .to_index(depth)
                .ok_or_else(|| Escape::Core(Malformed::EscapedVariable.into()))
        };
        let Some(Solving { arity, outer, .. }) = self.solving else {
            return named(self.depth);
        };
        if level.0 >= outer {
            return named(self.depth);
        }
        if level.0 >= arity {
            return Err(Escape::OutOfScope);
        }
        // The solution's context is `arity` binders where the value's was
        // `outer` of them, and the binders quotation entered since are common to
        // both.
        named(self.depth.saturating_sub(outer).saturating_add(arity))
    }

    /// Refuse if `found` is the metavariable whose solution is being written.
    fn occurs(self, found: &Meta) -> Result<(), Escape> {
        match self.solving {
            Some(Solving { meta, .. }) if meta == found => Err(Escape::OutOfScope),
            Some(_) | None => Ok(()),
        }
    }
}

/// Read `value` back as a term of type `ty`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the value does not inhabit the shape the type demands.
pub(crate) fn quote(meter: &mut Meter, depth: Depth, ty: &Value, value: &Value) -> Result<Term, CoreError> {
    read(meter, Reading::open(depth), ty, value).map_err(Escape::core)
}

/// Read `value` back as a metavariable's solution, or refuse to.
///
/// `ty` is the type both the value and the solution are at, or `None` when the
/// value is itself a type — the same distinction [`quote`] and [`quote_type`]
/// draw, asked once here because the caller knows it and quotation does not.
///
/// `Ok(None)` is the answer §2.1 calls waiting: the value mentions something the
/// solution may not, which is a reason to postpone the constraint rather than a
/// verdict about the program.
///
/// # Errors
///
/// As [`quote`].
pub(crate) fn quote_solution(
    meter: &mut Meter,
    depth: Depth,
    ty: Option<&Value>,
    value: &Value,
    meta: &Meta,
) -> Result<Option<Term>, CoreError> {
    let reading = Reading::solving(depth, meta, meta.arity());
    let written = match ty {
        Some(ty) => read(meter, reading, ty, value),
        None => read_type(meter, reading, value),
    };
    match written {
        Ok(term) => Ok(Some(term)),
        Err(Escape::OutOfScope) => Ok(None),
        Err(Escape::Core(error)) => Err(error),
    }
}

fn read(meter: &mut Meter, reading: Reading<'_>, ty: &Value, value: &Value) -> Result<Term, Escape> {
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
                let variable = Value::var(Origin::UNKNOWN, reading.fresh(), Arc::clone(domain));
                let body_type = apply_closure(meter, codomain, variable.clone())?;
                let body = apply(meter, here, value.clone(), variable)?;
                Ok(Term::lam(
                    here,
                    Arc::clone(name),
                    read(meter, reading.under_binder(), &body_type, &body)?,
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
                        term: read(meter, reading, &field_ty, &field_value)?,
                    });
                }
                Ok(Term::new(here, crate::term::Shape::Record(fields.into())))
            }
            Form::Universe(_) => read_type(meter, reading, value),
            Form::Id { ty: at, .. } => match &value.form {
                Form::Refl(witness) => Ok(Term::refl(here, read(meter, reading, at, witness)?)),
                Form::Neutral(neutral) => read_neutral(meter, reading, neutral),
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Id { .. } => Err(Malformed::NotAnIdentity.into()),
            },
            // A neutral type has no η, so whatever inhabits it is neutral too.
            Form::Neutral(_) => match &value.form {
                Form::Neutral(neutral) => read_neutral(meter, reading, neutral),
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Id { .. }
                | Form::Refl(_) => read_type(meter, reading, value),
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
    read_type(meter, Reading::open(depth), value).map_err(Escape::core)
}

fn read_type(meter: &mut Meter, reading: Reading<'_>, value: &Value) -> Result<Term, Escape> {
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
                let variable = Value::var(Origin::UNKNOWN, reading.fresh(), Arc::clone(domain));
                let opened = apply_closure(meter, codomain, variable)?;
                Ok(Term::function(
                    here,
                    *plicity,
                    Arc::clone(name),
                    read_type(meter, reading, domain)?,
                    read_type(meter, reading.under_binder(), &opened)?,
                ))
            }
            Form::RecordType(telescope) => read_telescope(meter, here, reading, telescope),
            Form::Id { ty, left, right } => Ok(Term::identity(
                here,
                read_type(meter, reading, ty)?,
                read(meter, reading, ty, left)?,
                read(meter, reading, ty, right)?,
            )),
            Form::Neutral(neutral) => read_neutral(meter, reading, neutral),
            Form::Lam(_) | Form::Record(_) | Form::Refl(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read a record type's telescope back, binding each field as it goes.
///
/// Unlike [`quote`]'s record case, the earlier fields become *variables* rather
/// than projections, because a record type binds them and a record value only
/// has them.
fn read_telescope(
    meter: &mut Meter,
    here: Origin,
    reading: Reading<'_>,
    telescope: &Telescope,
) -> Result<Term, Escape> {
    let mut env = telescope.env.clone();
    let mut at = reading;
    let mut fields = Vec::with_capacity(telescope.fields.len());
    for Field { name, term } in telescope.fields.iter() {
        let field_ty = crate::eval::eval(meter, &env, term)?;
        fields.push(Field {
            name: Arc::clone(name),
            term: read_type(meter, at, &field_ty)?,
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
fn read_neutral(meter: &mut Meter, reading: Reading<'_>, neutral: &Neutral) -> Result<Term, Escape> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        let here = neutral.origin;
        // The two leaves a solution can refuse at, and the only two: a variable
        // it may not name, and itself.
        let mut term = match &neutral.head {
            Head::Var(level, _) => Term::var(here, reading.index(*level)?),
            // Reached only unsolved: [`quote`] forces first, and a spine whose
            // head is solved forces whole.
            Head::Meta(meta) => {
                reading.occurs(meta)?;
                Term::meta(here, meta.clone())
            }
            Head::Const(constant) => constant.term(here),
        };
        // Innermost first, and each node takes its *own* origin (§7): the spine
        // of `f x y` reads back as three terms and each says where it was
        // written. The prefix is grown alongside, because an argument's type is
        // the type of the prefix it is applied to.
        let mut prefix = Neutral::head(here, neutral.head.clone());
        for elimination in &neutral.spine {
            term = read_elimination(meter, reading, &prefix, term, elimination)?;
            prefix.spine.push(elimination.clone());
        }
        Ok(term)
    })
}

/// One elimination of an already-quoted prefix, read back.
fn read_elimination(
    meter: &mut Meter,
    reading: Reading<'_>,
    prefix: &Neutral,
    quoted: Term,
    elimination: &Elim,
) -> Result<Term, Escape> {
    match elimination {
        Elim::App { origin, argument } => {
            let domain = match head_type(meter, prefix)?.form {
                Form::Pi { domain, .. } => domain,
                Form::Universe(_)
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Record(_)
                | Form::Id { .. }
                | Form::Refl(_)
                | Form::Neutral(_) => return Err(Malformed::NotAFunction.into()),
            };
            Ok(Term::app(*origin, quoted, read(meter, reading, &domain, argument)?))
        }
        Elim::Project { origin, field } => Ok(Term::project(*origin, quoted, Arc::clone(field))),
        Elim::J {
            origin,
            ty,
            from,
            motive,
            base,
            to,
        } => {
            let base_type = {
                let at_from = apply(meter, *origin, Value::clone(motive), Value::clone(from))?;
                let reflexive = Value::new(from.origin, Form::Refl(Arc::clone(from)));
                apply(meter, *origin, at_from, reflexive)?
            };
            Ok(Term::jay(
                *origin,
                read_type(meter, reading, ty)?,
                read(meter, reading, ty, from)?,
                read_motive(meter, reading, ty, from, motive)?,
                read(meter, reading, &base_type, base)?,
                read(meter, reading, ty, to)?,
                quoted,
            ))
        }
    }
}

/// Read `J`'s motive back at the shape `(y : A) → Id A x y → Type l`.
///
/// The motive's Π type is never built as a value, because `l` is not recorded
/// anywhere and building it would mean either storing a level nobody needs or
/// inventing one. η-expanding at the shape gives the same answer: apply the
/// motive to two fresh variables whose types *are* known, and quote the result
/// as a type.
fn read_motive(
    meter: &mut Meter,
    reading: Reading<'_>,
    ty: &Arc<Value>,
    from: &Arc<Value>,
    motive: &Value,
) -> Result<Term, Escape> {
    let here = motive.origin;
    let endpoint = Value::var(Origin::UNKNOWN, reading.fresh(), Arc::clone(ty));
    let under_endpoint = reading.under_binder();
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
        Term::lam(here, "e", read_type(meter, under_endpoint.under_binder(), &body)?),
    ))
}
