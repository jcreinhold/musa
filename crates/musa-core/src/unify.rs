//! Unification: making two types equal by solving metavariables.
//!
//! `docs/rules/language/02-core-calculus.md` §2.1 fixes the algorithm, and the
//! whole of it is one restriction:
//!
//! > a constraint `?α x₁ … xₙ ≡ t` is solved immediately when `x₁ … xₙ` are
//! > *distinct bound variables* and every free variable of `t` is among them and
//! > the context of `?α` […] outside it, higher-order unification is undecidable
//! > and a solver that guessed would make a program's meaning depend on the
//! > order in which the checker reached its constraints.
//!
//! So this module has no heuristic, no fallback, and no "try the obvious
//! solution". A constraint it cannot solve is **postponed**, retried whenever a
//! metavariable is solved, and — if it is still blocked when the declaration
//! ends — reported as the metavariable it left undetermined.
//!
//! # Why the pattern check is nearly free here
//!
//! Every metavariable is written applied to the *identity spine* of its creation
//! context: `?α x₀ x₁ … xₙ₋₁`, in order, with no repeats. That is the pattern
//! condition, established by construction rather than tested for. What is left
//! is the other half — that the right-hand side mentions no variable bound
//! *after* the metavariable was created — and [`restrict`] checks it on the
//! quoted term, in one walk that also performs the occurs check and the index
//! shift.
//!
//! # Why solving quotes at a type
//!
//! A solution is a term, so the right-hand *value* has to be read back, and
//! quotation in this crate is type-directed (§3, η). The type is not invented:
//! unification already knows what the two sides are at, because it descended to
//! them from a pair of types it knew. That is why nothing here needs a second,
//! untyped quotation function, and why a solution comes out η-long for free.

use std::sync::Arc;

use crate::budget::Meter;
use crate::error::CoreError;
use crate::eval::{apply_closure, eval, force, head_type};
use crate::meta::Meta;
use crate::origin::Origin;
use crate::quote::{Depth, quote, quote_type};
use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
use crate::term::{DbLevel, Field, Index, Shape, Term};
use crate::value::{Env, Form, Neutral, Spine, Telescope, Value};

/// What a pair of values is being compared at.
///
/// Quotation needs it, and there is no way to say "a type" without inventing a
/// level nobody wrote — the same reason [`quote_type`] exists beside [`quote`].
#[derive(Clone, Copy)]
enum At<'a> {
    /// Both sides are types.
    Type,
    /// Both sides inhabit this type.
    Term(&'a Value),
}

impl At<'_> {
    fn quote(self, meter: &mut Meter, depth: u32, value: &Value) -> Result<Term, CoreError> {
        match self {
            Self::Type => quote_type(meter, Depth(depth), value),
            Self::Term(ty) => quote(meter, Depth(depth), ty, value),
        }
    }

    fn owned(self) -> Option<Value> {
        match self {
            Self::Type => None,
            Self::Term(ty) => Some(ty.clone()),
        }
    }
}

/// A constraint that could not be decided yet.
struct Constraint {
    depth: u32,
    /// The type both sides are at, or `None` when both are types.
    ty: Option<Value>,
    left: Value,
    right: Value,
    at: Origin,
}

/// The unifier's state: the constraints it has put aside, and whether any pass
/// has made progress.
///
/// Elaboration-scoped rather than global because §2.1 reports a metavariable
/// unsolved "at the end of the declaration it was created in", so the list has
/// to end when that declaration does.
#[derive(Default)]
pub(crate) struct Unifier {
    postponed: Vec<Constraint>,
    /// How many metavariables have been solved, ever.
    ///
    /// The retry loop's progress measure. Counting solutions rather than
    /// watching the queue shrink is what makes the loop's termination argument
    /// hold: solutions are write-once (§2.1) and metavariables are finite, so
    /// this can rise only finitely often.
    solved: u64,
}

impl Unifier {
    /// Make `left` and `right` equal as types, solving what that requires.
    ///
    /// # Errors
    ///
    /// [`Refusal::Mismatch`] when they cannot be made equal, or exhaustion.
    pub(crate) fn unify_types(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: Origin,
        left: &Value,
        right: &Value,
    ) -> Result<(), ElabError> {
        self.step(meter, depth, At::Type, at, left, right)
            .map_err(|failure| failure.into_error(at))?;
        self.retry_postponed(meter)
    }

    /// Make `left` and `right` equal as inhabitants of `ty`.
    ///
    /// Separate from [`Self::unify_types`] because quotation is type-directed
    /// (§3): a solution read back at the wrong sort would not be η-long, and
    /// there is no type to state "a type" at without inventing a level.
    ///
    /// # Errors
    ///
    /// As [`Self::unify_types`].
    pub(crate) fn unify(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: Origin,
        ty: &Value,
        left: &Value,
        right: &Value,
    ) -> Result<(), ElabError> {
        self.step(meter, depth, At::Term(ty), at, left, right)
            .map_err(|failure| failure.into_error(at))?;
        self.retry_postponed(meter)
    }

    /// The term a still-blocked constraint came from, if one is left.
    ///
    /// What a diagnostic names when it reports a metavariable it could not
    /// determine (§2.1): "what constraint was still blocked".
    pub(crate) fn blocked(&self) -> Option<Origin> {
        self.postponed.first().map(|constraint| constraint.at)
    }

    /// Retry every postponed constraint until a pass solves nothing new.
    ///
    /// Terminating for a reason worth stating: a pass either solves at least one
    /// metavariable or changes nothing at all, and a metavariable cannot be
    /// solved twice.
    fn retry_postponed(&mut self, meter: &mut Meter) -> Result<(), ElabError> {
        while !self.postponed.is_empty() {
            meter.retry("unification")?;
            let before = self.solved;
            let waiting = std::mem::take(&mut self.postponed);
            for constraint in waiting {
                let ty = constraint.ty.clone();
                let at = ty.as_ref().map_or(At::Type, At::Term);
                self.step(
                    meter,
                    constraint.depth,
                    at,
                    constraint.at,
                    &constraint.left,
                    &constraint.right,
                )
                .map_err(|failure| failure.into_error(constraint.at))?;
            }
            if self.solved == before {
                return Ok(());
            }
        }
        Ok(())
    }

    /// One step: force both sides, try the flexible cases, then descend.
    fn step(&mut self, meter: &mut Meter, depth: u32, at: At<'_>, origin: Origin, left: &Value, right: &Value) -> Step {
        // The nesting charge is what keeps unification inside §4.1's limit, and
        // it can only report a [`CoreError`], so the step's own answer travels
        // back inside its `Ok`.
        meter.nested("unification", |meter| {
            meter.step("unification")?;
            let unfolded_left = force(meter, left)?;
            let left = unfolded_left.as_ref().unwrap_or(left);
            let unfolded_right = force(meter, right)?;
            let right = unfolded_right.as_ref().unwrap_or(right);

            Ok(match self.flexible(meter, depth, at, left, right)? {
                Flexible::Solved => Ok(()),
                Flexible::Postpone => {
                    self.postponed.push(Constraint {
                        depth,
                        ty: at.owned(),
                        left: left.clone(),
                        right: right.clone(),
                        at: origin,
                    });
                    Ok(())
                }
                Flexible::Rigid => self.rigid(meter, depth, at, origin, left, right),
            })
        })?
    }

    /// What the metavariable-headed cases decided.
    fn flexible(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        left: &Value,
        right: &Value,
    ) -> Result<Flexible, CoreError> {
        let left_meta = flexible_head(left);
        let right_meta = flexible_head(right);
        // The same unknown on both sides is already equal, and solving it
        // against itself would fail the occurs check for no reason.
        if let (Some(one), Some(other)) = (left_meta, right_meta)
            && one == other
        {
            return Ok(Flexible::Solved);
        }
        for (meta, rigid) in [(left_meta, right), (right_meta, left)] {
            let Some(meta) = meta else { continue };
            if self.attempt(meter, depth, at, meta, rigid)? {
                return Ok(Flexible::Solved);
            }
        }
        if left_meta.is_some() || right_meta.is_some() {
            // Blocked, not wrong. §2.1 postpones rather than guessing.
            return Ok(Flexible::Postpone);
        }
        Ok(Flexible::Rigid)
    }

    /// Try to solve `meta` to `rigid`.
    ///
    /// Answers `false` when the solution would capture a variable the
    /// metavariable was not given, or would mention the metavariable itself.
    /// Both are reasons to wait rather than verdicts, so the caller postpones.
    fn attempt(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        meta: &Meta,
        rigid: &Value,
    ) -> Result<bool, CoreError> {
        meter.metavariable("unification")?;
        let body = at.quote(meter, depth, rigid)?;
        let arity = meta.arity();
        let Some(body) = restrict(&body, meta, arity, depth, 0) else {
            return Ok(false);
        };
        let origin = meta.origin();
        let mut abstracted = body;
        for _ in 0..arity {
            abstracted = Term::lam(origin, "_", abstracted);
        }
        // Evaluated in the *empty* environment, which is what "a metavariable is
        // closed" means operationally: its solution mentions nothing but the
        // binders it abstracts.
        let value = eval(meter, &Env::EMPTY, &abstracted)?;
        meta.solve(value)?;
        self.solved = self.solved.saturating_add(1);
        Ok(true)
    }

    /// Both sides are rigid: compare them structurally.
    fn rigid(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Step {
        match (&left.form, &right.form) {
            // Two universes agree when their levels can be made the same, which
            // is where a level metavariable is solved. `determine` refuses
            // rather than searching, and a refusal falls through to the reading
            // back that every other disagreement uses — so the diagnostic is
            // built in one place and says `Type 0` against `Type 1` rather than
            // naming a constraint the author never wrote.
            (Form::Universe(one), Form::Universe(other)) => {
                if one.determine(other) {
                    return Ok(());
                }
                Self::by_reading_back(meter, depth, at, left, right)
            }
            (
                Form::Pi {
                    domain: left_domain,
                    codomain: left_codomain,
                    ..
                },
                Form::Pi {
                    domain: right_domain,
                    codomain: right_codomain,
                    ..
                },
            ) => {
                self.step(meter, depth, At::Type, origin, left_domain, right_domain)
                    .map_err(|failure| failure.under(PathStep::Domain))?;
                let variable = Value::var(Origin::UNKNOWN, DbLevel(depth), Arc::clone(left_domain));
                let left_body = apply_closure(meter, left_codomain, variable.clone())?;
                let right_body = apply_closure(meter, right_codomain, variable)?;
                self.step(
                    meter,
                    depth.saturating_add(1),
                    At::Type,
                    origin,
                    &left_body,
                    &right_body,
                )
                .map_err(|failure| failure.under(PathStep::Codomain))
            }
            (Form::RecordType(one), Form::RecordType(other)) => {
                self.record_types(meter, depth, at, origin, left, right, one, other)
            }
            (
                Form::Id {
                    ty: left_ty,
                    left: left_from,
                    right: left_to,
                },
                Form::Id {
                    ty: right_ty,
                    left: right_from,
                    right: right_to,
                },
            ) => {
                self.step(meter, depth, At::Type, origin, left_ty, right_ty)
                    .map_err(|failure| failure.under(PathStep::IdType))?;
                self.step(meter, depth, At::Term(left_ty), origin, left_from, right_from)
                    .map_err(|failure| failure.under(PathStep::IdLeft))?;
                self.step(meter, depth, At::Term(left_ty), origin, left_to, right_to)
                    .map_err(|failure| failure.under(PathStep::IdRight))
            }
            (Form::Refl(one), Form::Refl(other)) => {
                let At::Term(ty) = at else {
                    return Self::by_reading_back(meter, depth, at, left, right);
                };
                let Form::Id { ty: subject, .. } = &ty.form else {
                    return Self::by_reading_back(meter, depth, at, left, right);
                };
                self.step(meter, depth, At::Term(subject), origin, one, other)
                    .map_err(|failure| failure.under(PathStep::Witness))
            }
            (Form::Neutral(one), Form::Neutral(other)) => self.neutrals(meter, depth, origin, one, other),
            // Everything else — including a lambda or a record literal, which
            // stand here only as an eliminated argument — is decided by reading
            // both sides back. Quotation is η-long, so that is exactly the
            // conversion §3 specifies, and a structural walk would gain nothing.
            _ => Self::by_reading_back(meter, depth, at, left, right),
        }
    }

    fn record_types(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
        one: &Telescope,
        other: &Telescope,
    ) -> Step {
        if one.fields.len() != other.fields.len() {
            return Self::by_reading_back(meter, depth, at, left, right);
        }
        let mut left_env = one.env.clone();
        let mut right_env = other.env.clone();
        let mut under = depth;
        for (mine, theirs) in one.fields.iter().zip(other.fields.iter()) {
            if mine.name != theirs.name {
                return Self::by_reading_back(meter, depth, at, left, right);
            }
            let left_ty = eval(meter, &left_env, &mine.term)?;
            let right_ty = eval(meter, &right_env, &theirs.term)?;
            self.step(meter, under, At::Type, origin, &left_ty, &right_ty)
                .map_err(|failure| failure.under(PathStep::Field(Arc::clone(&mine.name))))?;
            // Both telescopes proceed under the *same* variable, because the
            // field types have just been made equal.
            let variable = Value::var(Origin::UNKNOWN, DbLevel(under), Arc::new(left_ty));
            left_env = left_env.push(variable.clone());
            right_env = right_env.push(variable);
            under = under.saturating_add(1);
        }
        Ok(())
    }

    /// Two blocked eliminations, neither headed by a metavariable: same head,
    /// same shape, equal arguments.
    fn neutrals(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        origin: Origin,
        one: &Arc<Neutral>,
        other: &Arc<Neutral>,
    ) -> Step {
        match (&one.spine, &other.spine) {
            (Spine::Var(level, _), Spine::Var(other_level, _)) if level.0 == other_level.0 => Ok(()),
            // Rigid like a variable, and decided the same way: a constant is its
            // name, so there is nothing under it to unify.
            (Spine::Const(left), Spine::Const(right)) if left == right => Ok(()),
            (
                Spine::App {
                    function: left_function,
                    argument: left_argument,
                },
                Spine::App {
                    function: right_function,
                    argument: right_argument,
                },
            ) => {
                self.neutrals(meter, depth, origin, left_function, right_function)
                    .map_err(|failure| failure.under(PathStep::Function))?;
                let Form::Pi { domain, .. } = head_type(meter, left_function)?.form else {
                    return Err(blocked_mismatch(meter, depth, one, other)?);
                };
                self.step(meter, depth, At::Term(&domain), origin, left_argument, right_argument)
                    .map_err(|failure| failure.under(PathStep::Argument))
            }
            (
                Spine::Project {
                    record: left_record,
                    field: left_field,
                },
                Spine::Project {
                    record: right_record,
                    field: right_field,
                },
            ) if left_field == right_field => self
                .neutrals(meter, depth, origin, left_record, right_record)
                .map_err(|failure| failure.under(PathStep::Projected)),
            _ => Err(blocked_mismatch(meter, depth, one, other)?),
        }
    }

    /// The last resort: read both sides back and compare up to α.
    ///
    /// Not a weaker rule — quotation is η-long, so this decides exactly what
    /// [`crate::convertible`] decides. It solves no metavariable, which is why
    /// it is only ever reached once neither side has one at its head.
    fn by_reading_back(meter: &mut Meter, depth: u32, at: At<'_>, left: &Value, right: &Value) -> Step {
        let expected = at.quote(meter, depth, left)?;
        let found = at.quote(meter, depth, right)?;
        if expected == found {
            Ok(())
        } else {
            Err(Failure::Mismatch {
                expected,
                found,
                path: Vec::new(),
            })
        }
    }
}

/// What the flexible cases decided.
enum Flexible {
    /// A metavariable was solved; nothing is left to compare.
    Solved,
    /// One side is flexible and could not be solved yet.
    Postpone,
    /// Neither side is headed by an unsolved metavariable.
    Rigid,
}

/// What one step of unification answers.
type Step = Result<(), Failure>;

/// Why a step did not succeed.
enum Failure {
    /// The two sides disagree here.
    Mismatch {
        expected: Term,
        found: Term,
        /// Built innermost-first while unwinding, because a frame only learns it
        /// is on the path when the frame beneath it fails.
        path: Vec<PathStep>,
    },
    /// Exhaustion, or a term the caller should not have handed over.
    Core(CoreError),
}

impl From<CoreError> for Failure {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl Failure {
    /// This failure, seen from one step further out.
    fn under(self, step: PathStep) -> Self {
        match self {
            Self::Mismatch {
                expected,
                found,
                mut path,
            } => {
                path.push(step);
                Self::Mismatch { expected, found, path }
            }
            Self::Core(error) => Self::Core(error),
        }
    }

    fn into_error(self, at: Origin) -> ElabError {
        match self {
            Self::Mismatch {
                expected,
                found,
                mut path,
            } => {
                path.reverse();
                Refusal::Mismatch(Box::new(Mismatch {
                    at,
                    expected,
                    found,
                    path,
                }))
                .into()
            }
            Self::Core(error) => error.into(),
        }
    }
}

/// A mismatch between two blocked eliminations.
///
/// Read back as types, which is not a claim that they are: a neutral quotes the
/// same either way, since there is nothing at a neutral type to η-expand.
fn blocked_mismatch(
    meter: &mut Meter,
    depth: u32,
    one: &Arc<Neutral>,
    other: &Arc<Neutral>,
) -> Result<Failure, CoreError> {
    Ok(Failure::Mismatch {
        expected: quote_type(meter, Depth(depth), &Value::shared_neutral(one))?,
        found: quote_type(meter, Depth(depth), &Value::shared_neutral(other))?,
        path: Vec::new(),
    })
}

/// The unsolved metavariable at the head of a blocked elimination, if there is
/// one.
fn flexible_head(value: &Value) -> Option<&Meta> {
    let Form::Neutral(neutral) = &value.form else {
        return None;
    };
    let mut here = neutral;
    loop {
        match &here.spine {
            Spine::Var(_, _) | Spine::Const(_) => return None,
            Spine::Meta(meta) => return Some(meta),
            Spine::App { function, .. } => here = function,
            Spine::Project { record, .. } => here = record,
            Spine::J { proof, .. } => here = proof,
        }
    }
}

/// Move `term` from a context of `depth` binders into `meta`'s own, refusing
/// when that is not possible.
///
/// One walk doing the three things §2.1 asks for, because they are the same
/// walk: the **scope check** (no variable bound after `meta` was created), the
/// **occurs check** (`meta` does not appear in its own solution), and the index
/// shift that makes the result a term in a context of `arity` binders rather
/// than `depth` of them.
///
/// `bound` counts the binders entered *inside* `term`; a variable below it is
/// local and never moves.
fn restrict(term: &Term, meta: &Meta, arity: u32, depth: u32, bound: u32) -> Option<Term> {
    let dropped = depth.checked_sub(arity)?;
    let origin = term.origin();
    let shape = match term.shape() {
        Shape::Var(index) => {
            if index.0 < bound {
                return Some(term.clone());
            }
            // Out of scope: this variable was bound after the metavariable was
            // created, so a solution abstracting only its context cannot mention
            // it. `checked_sub` failing and the `< bound` test are the same
            // condition seen from two sides.
            let shifted = index.0.checked_sub(dropped)?;
            if shifted < bound {
                return None;
            }
            Shape::Var(Index(shifted))
        }
        Shape::Meta(found) => {
            if found == meta {
                return None;
            }
            Shape::Meta(found.clone())
        }
        Shape::Universe(level) => Shape::Universe(level.resolved()),
        // Closed and mentioning no binder, so it moves between contexts
        // untouched — the same reason a solved metavariable's body does.
        Shape::Const(constant) => Shape::Const(constant.clone()),
        Shape::Pi {
            plicity,
            name,
            domain,
            codomain,
        } => Shape::Pi {
            plicity: *plicity,
            name: Arc::clone(name),
            domain: restrict(domain, meta, arity, depth, bound)?,
            codomain: restrict(codomain, meta, arity, depth, bound.saturating_add(1))?,
        },
        Shape::Lam { name, body } => Shape::Lam {
            name: Arc::clone(name),
            body: restrict(body, meta, arity, depth, bound.saturating_add(1))?,
        },
        Shape::App { function, argument } => Shape::App {
            function: restrict(function, meta, arity, depth, bound)?,
            argument: restrict(argument, meta, arity, depth, bound)?,
        },
        // A record *type* is a telescope, so field `i` is read under `i` more
        // binders; a record *value* binds nothing.
        Shape::RecordType(fields) => Shape::RecordType(restrict_telescope(fields, meta, arity, depth, bound)?),
        Shape::Record(fields) => Shape::Record(restrict_fields(fields, meta, arity, depth, bound)?),
        Shape::Project { record, field } => Shape::Project {
            record: restrict(record, meta, arity, depth, bound)?,
            field: Arc::clone(field),
        },
        Shape::Id { ty, left, right } => Shape::Id {
            ty: restrict(ty, meta, arity, depth, bound)?,
            left: restrict(left, meta, arity, depth, bound)?,
            right: restrict(right, meta, arity, depth, bound)?,
        },
        Shape::Refl(value) => Shape::Refl(restrict(value, meta, arity, depth, bound)?),
        Shape::J {
            ty,
            from,
            motive,
            base,
            to,
            proof,
        } => Shape::J {
            ty: restrict(ty, meta, arity, depth, bound)?,
            from: restrict(from, meta, arity, depth, bound)?,
            motive: restrict(motive, meta, arity, depth, bound)?,
            base: restrict(base, meta, arity, depth, bound)?,
            to: restrict(to, meta, arity, depth, bound)?,
            proof: restrict(proof, meta, arity, depth, bound)?,
        },
        Shape::Let { name, ty, value, body } => Shape::Let {
            name: Arc::clone(name),
            ty: restrict(ty, meta, arity, depth, bound)?,
            value: restrict(value, meta, arity, depth, bound)?,
            body: restrict(body, meta, arity, depth, bound.saturating_add(1))?,
        },
    };
    Some(Term::new(origin, shape))
}

fn restrict_fields(fields: &[Field], meta: &Meta, arity: u32, depth: u32, bound: u32) -> Option<Arc<[Field]>> {
    fields
        .iter()
        .map(|Field { name, term }| {
            Some(Field {
                name: Arc::clone(name),
                term: restrict(term, meta, arity, depth, bound)?,
            })
        })
        .collect()
}

fn restrict_telescope(fields: &[Field], meta: &Meta, arity: u32, depth: u32, bound: u32) -> Option<Arc<[Field]>> {
    fields
        .iter()
        .enumerate()
        .map(|(position, Field { name, term })| {
            let under = bound.saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
            Some(Field {
                name: Arc::clone(name),
                term: restrict(term, meta, arity, depth, under)?,
            })
        })
        .collect()
}
