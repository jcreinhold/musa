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
use crate::eval::{apply, apply_closure, field_type, head_type, opened, project};
use crate::origin::Origin;
use crate::term::{DbLevel, Field, Index, Term};
use crate::value::{DefHead, Elim, Form, Head, Neutral, Telescope, Value};

/// How many binders are in scope while quoting.
///
/// Carried rather than derived because it is what turns a [`DbLevel`] created
/// during quotation into the [`Index`] that names it.
#[derive(Clone, Copy)]
pub(crate) struct Depth(pub(crate) u32);

/// Whether quotation opens a folded definition or keeps it.
///
/// Two modes, because there are two callers. Diagnostics **keep**: a
/// conversion mismatch should name `pitch_of`, not print its normal form.
/// Metavariable solutions and the canonical readback **open**: a solution
/// mentioning a definition that escapes its scope is unsound, and a musical
/// value must be a value rather than a name for one. There is no third
/// caller, so there is no third mode.
///
/// The mode governs the *value* being read back. The *type* directing η is
/// always opened — it is never written into the term, and a folded type
/// synonym would otherwise skip the η the answer is owed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Stop at a folded definition and write its name.
    Keep,
    /// Unfold folded definitions.
    Open,
}

/// The depth, and — when the term being written is a metavariable's solution —
/// what that solution is allowed to mention.
///
/// One value rather than two parameters because the two are asked together at
/// exactly one place, [`Reading::index`], and separating them would put the
/// depth in every signature twice.
#[derive(Clone, Copy)]
struct Reading {
    depth: u32,
    mode: Mode,
}

/// Why reading a value back did not produce a term.
///
/// A newtype around [`CoreError`] rather than the error itself, because the
/// one caller pattern here is a `From` conversion in a walk that returns it.
enum Escape {
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
    /// This escape as a [`CoreError`].
    fn core(self) -> CoreError {
        match self {
            Self::Core(error) => error,
        }
    }
}

impl Reading {
    /// A plain quotation: no metavariable to fit the answer into.
    const fn open(depth: Depth, mode: Mode) -> Self {
        Self { depth: depth.0, mode }
    }

    /// The value with whatever the head hides seen through: solved
    /// metavariables always, folded definitions in [`Mode::Open`].
    fn seen(self, meter: &mut Meter, value: &Value) -> Result<Option<Value>, Escape> {
        match self.mode {
            Mode::Keep => Ok(None),
            Mode::Open => Ok(opened(meter, value)?),
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
        level
            .to_index(self.depth)
            .ok_or_else(|| Escape::Core(Malformed::EscapedVariable.into()))
    }
}

/// Read `value` back as a term of type `ty`.
///
/// # Errors
///
/// [`CoreError::Exhausted`] at a budget limit, [`CoreError::Malformed`] when
/// the value does not inhabit the shape the type demands.
pub(crate) fn quote(meter: &mut Meter, depth: Depth, mode: Mode, ty: &Value, value: &Value) -> Result<Term, CoreError> {
    read(meter, Reading::open(depth, mode), ty, value).map_err(Escape::core)
}

fn read(meter: &mut Meter, reading: Reading, ty: &Value, value: &Value) -> Result<Term, Escape> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        // `as_ref().unwrap_or` rather than `unwrap_or_else(clone)`: the common
        // case is a value with no metavariable in it, and that case must not pay
        // an allocation per quoted node.
        // The type is opened in either mode: it directs η and is never written
        // into the term, so a folded type synonym must not hide a Π.
        let unfolded_ty = opened(meter, ty)?;
        let ty = unfolded_ty.as_ref().unwrap_or(ty);
        let unfolded_value = reading.seen(meter, value)?;
        let value = unfolded_value.as_ref().unwrap_or(value);
        let here = value.origin;
        match &ty.form {
            // §1.5: a refinement adds no inhabitants and no η. What inhabits
            // `Row(12)` is exactly what inhabits `Row`, so the type directing
            // this read is the one underneath.
            Form::Refine { ty, .. } => read(meter, reading, ty, value),
            // η at Π: a lambda, whether or not the value is one. A λ has no
            // filling to write — it is the Π that says how the argument arrives.
            Form::Pi {
                filling: _,
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
            // A neutral type has no η, so whatever inhabits it is neutral too —
            // with one exception, and it is the one §5.8 adds. A base type
            // evaluates to a spine headed by [`Head::Base`], so *every* literal
            // is quoted here, and a literal is already its own normal form.
            Form::Neutral(_) => match &value.form {
                Form::Neutral(neutral) => read_neutral(meter, reading, neutral),
                Form::Lit(literal) => Ok(literal.term(here)),
                // Already its own normal form, and one node rather than `count`
                // of them — which is what keeps quotation's node charge
                // independent of the number the author wrote.
                Form::Numeral(numeral) => Ok(Term::new(here, crate::term::Shape::Numeral(numeral.clone()))),
                Form::Universe(_)
                | Form::Pi { .. }
                | Form::Lam(_)
                | Form::RecordType(_)
                | Form::Refine { .. }
                | Form::Record(_) => read_type(meter, reading, value),
            },
            Form::Lam(_) | Form::Record(_) | Form::Lit(_) | Form::Numeral(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read `value` back as a type.
///
/// # Errors
///
/// As [`quote`], plus [`Malformed::NotAType`] when the value is a canonical
/// form no universe contains.
pub(crate) fn quote_type(meter: &mut Meter, depth: Depth, mode: Mode, value: &Value) -> Result<Term, CoreError> {
    read_type(meter, Reading::open(depth, mode), value).map_err(Escape::core)
}

fn read_type(meter: &mut Meter, reading: Reading, value: &Value) -> Result<Term, Escape> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        let unfolded = reading.seen(meter, value)?;
        let value = unfolded.as_ref().unwrap_or(value);
        let here = value.origin;
        match &value.form {
            Form::Universe(level) => Ok(Term::universe(here, *level)),
            Form::Pi {
                filling,
                name,
                domain,
                codomain,
            } => {
                let variable = Value::var(Origin::UNKNOWN, reading.fresh(), Arc::clone(domain));
                let opened = apply_closure(meter, codomain, variable)?;
                Ok(Term::function(
                    here,
                    filling.clone(),
                    Arc::clone(name),
                    read_type(meter, reading, domain)?,
                    read_type(meter, reading.under_binder(), &opened)?,
                ))
            }
            Form::RecordType(telescope) => read_telescope(meter, here, reading, telescope),
            // **This is erasure.** The wrapper is dropped and the refined type
            // is read back alone, so no index survives into a `Term` — and
            // therefore none reaches a snapshot, a digest, or a stored
            // artifact. §1.5's "erased before evaluation" is this one line, not
            // a property a test watches.
            Form::Refine { ty, .. } => read_type(meter, reading, ty),
            Form::Neutral(neutral) => read_neutral(meter, reading, neutral),
            Form::Lam(_) | Form::Record(_) | Form::Lit(_) | Form::Numeral(_) => Err(Malformed::NotAType.into()),
        }
    })
}

/// Read a record type's telescope back, binding each field as it goes.
///
/// Unlike [`quote`]'s record case, the earlier fields become *variables* rather
/// than projections, because a record type binds them and a record value only
/// has them.
fn read_telescope(meter: &mut Meter, here: Origin, reading: Reading, telescope: &Telescope) -> Result<Term, Escape> {
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
fn read_neutral(meter: &mut Meter, reading: Reading, neutral: &Neutral) -> Result<Term, Escape> {
    meter.nested("quotation", |meter| {
        meter.quoted_node("quotation")?;
        let here = neutral.origin;
        // The two leaves a solution can refuse at, and the only two: a variable
        // it may not name, and itself.
        let mut term = match &neutral.head {
            Head::Var(level, _) => Term::var(here, reading.index(*level)?),
            // The whole of [`Mode::Keep`]: a definition writes its name — the
            // binder for a local, the declaration for a global — rather than
            // its normal form.
            Head::Def(which, _, _) => match which {
                DefHead::Local(level) => Term::var(here, reading.index(*level)?),
                DefHead::Global(def) => def.term(here),
            },
            // Reached only unsolved: a solved hole is forced before quotation,
            // and a spine whose head is solved forces whole.
            Head::Hole(hole) => Term::hole(here, hole.clone()),
            Head::Const(constant) => constant.term(here),
            // Both rigid, both closed, and both already their own normal form:
            // a base type has no eliminator and a builtin whose arguments were
            // literals would have reduced before quotation saw it.
            Head::Base(base) => base.term(here),
            Head::Builtin(builtin) => builtin.term(here),
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
    reading: Reading,
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
                | Form::Lit(_)
                | Form::Numeral(_)
                | Form::Refine { .. }
                | Form::Neutral(_) => return Err(Malformed::NotAFunction.into()),
            };
            Ok(Term::app(*origin, quoted, read(meter, reading, &domain, argument)?))
        }
        Elim::Project { origin, field } => Ok(Term::project(*origin, quoted, Arc::clone(field))),
    }
}
