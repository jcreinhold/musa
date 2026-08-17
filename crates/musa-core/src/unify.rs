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
//! *after* the metavariable was created — and [`crate::quote::quote_solution`]
//! decides it while writing the solution, in the same walk that performs the
//! occurs check and the index shift.
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
use crate::eval::{apply, apply_closure, eval, field_type, force, head_type, project};
use crate::meta::Meta;
use crate::origin::Origin;
use crate::quote::{Depth, quote, quote_solution, quote_type};
use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
use crate::term::{DbLevel, Field, Term};
use crate::value::{Closure, Elim, Env, Form, Head, Neutral, Telescope, Value};

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

impl<'a> At<'a> {
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

    /// The type the two sides inhabit, or `None` when they are types.
    const fn subject(self) -> Option<&'a Value> {
        match self {
            Self::Type => None,
            Self::Term(ty) => Some(ty),
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
    /// Whether this unifier answers a question rather than making one true.
    ///
    /// Set by [`Self::deciding`], and read in exactly one place. `false` is the
    /// elaborating unifier, which is what `Default` should give.
    deciding: bool,
}

impl Unifier {
    /// A unifier that decides §3's conversion instead of solving for it.
    ///
    /// Definitional equality *is* unification's rigid fragment — the same
    /// type-directed walk with η and early exit, over a term language where a
    /// metavariable is an opaque head rather than an unknown to determine. So
    /// [`crate::convertible`] is this constructor and not a second procedure:
    /// normalizing both sides and comparing is the same answer computed the
    /// most expensive way available, and two implementations of one question
    /// are two things to keep in agreement.
    ///
    /// Nothing is postponed in this mode, because postponing is what a solver
    /// does when it does not yet know: a decision procedure that cannot solve
    /// has already learned everything it will.
    pub(crate) fn deciding() -> Self {
        Self {
            deciding: true,
            ..Self::default()
        }
    }

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
        meter.nested::<Step, CoreError>("unification", |meter| {
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
        if self.deciding {
            // Every metavariable is a rigid head here, so `neutrals` compares
            // two of them by identity and refuses a meta against anything else.
            return Ok(Flexible::Rigid);
        }
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
        let Some(body) = quote_solution(meter, Depth(depth), at.subject(), rigid, meta)? else {
            return Ok(false);
        };
        let arity = meta.arity();
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
        // η first, and read off the *type*, because §3 puts η at Π and at record
        // types — so it is the type that decides, whatever forms the two sides
        // happen to have. This is what makes `f` and `λx. f x` agree without
        // either being quoted, and it is why a λ or a record literal never
        // reaches the match below on a well-typed pair.
        //
        // Two neutrals are the one pair this skips. Expanding them adds the same
        // elimination to both spines and then compares the spines, which is the
        // answer [`Self::neutrals`] gives directly — with a message that names
        // the heads that disagreed rather than the expansion.
        if let At::Term(ty) = at
            && !matches!((&left.form, &right.form), (Form::Neutral(_), Form::Neutral(_)))
        {
            let unfolded = force(meter, ty)?;
            let ty = unfolded.as_ref().unwrap_or(ty);
            match &ty.form {
                Form::Pi { domain, codomain, .. } => {
                    return self.under_binder(meter, depth, origin, domain, codomain, left, right);
                }
                Form::RecordType(telescope) => {
                    return self.field_by_field(meter, depth, origin, telescope, left, right);
                }
                Form::Universe(_)
                | Form::Lam(_)
                | Form::Record(_)
                | Form::Id { .. }
                | Form::Refl(_)
                // A base type has no η, because η is a rule about a type's
                // eliminations and §5.8 gives it none. Neither of the two below
                // is a type at all; they are here because this match is over
                // every form a forced value can take, not over the well-typed
                // ones.
                | Form::Lit(_)
                | Form::Numeral(_) => {}
                // A type that is still a metavariable says nothing yet, and a λ
                // under it would be one the elaborator has not pinned down. The
                // match below reads both sides back, which is the honest answer
                // and not a success path.
                Form::Neutral(_) => {}
            }
        }
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
            // Two different forms, which is a disagreement: reading both sides
            // back is how the message says so. The one pair that is not already
            // decided by the time it lands here is a λ or a record literal whose
            // type is still unknown, and quotation refuses that rather than
            // guessing.
            _ => Self::by_reading_back(meter, depth, at, left, right),
        }
    }

    /// Both sides at a function type: compare them applied to one fresh
    /// variable.
    ///
    /// The `Lam`/`Lam` case is *this* case — [`apply`] answers a lambda by
    /// opening its closure — so there is no second arm for it, and none for a
    /// lambda meeting a neutral either.
    fn under_binder(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        origin: Origin,
        domain: &Arc<Value>,
        codomain: &Closure,
        left: &Value,
        right: &Value,
    ) -> Step {
        let variable = Value::var(Origin::UNKNOWN, DbLevel(depth), Arc::clone(domain));
        let body_type = apply_closure(meter, codomain, variable.clone())?;
        let left_body = apply(meter, left.origin, left.clone(), variable.clone())?;
        let right_body = apply(meter, right.origin, right.clone(), variable)?;
        self.step(
            meter,
            depth.saturating_add(1),
            At::Term(&body_type),
            origin,
            &left_body,
            &right_body,
        )
        .map_err(|failure| failure.under(PathStep::Body))
    }

    /// Both sides at a record type: compare their projections, in telescope
    /// order, stopping at the first field that disagrees.
    ///
    /// The `Record`/`Record` case is *this* case — [`project`] answers a literal
    /// by reading the field out — so a literal, a neutral, and one of each are
    /// all decided here.
    fn field_by_field(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        origin: Origin,
        telescope: &Telescope,
        left: &Value,
        right: &Value,
    ) -> Step {
        for Field { name, term: _ } in telescope.fields.iter() {
            // The field's type is read off the *left* subject, as it is in
            // [`Self::record_types`] and for the same reason: every earlier
            // field has just been made equal, so either subject gives the same
            // type.
            let ty = field_type(meter, telescope, left, name)?;
            let mine = project(meter, left.origin, left.clone(), name)?;
            let theirs = project(meter, right.origin, right.clone(), name)?;
            self.step(meter, depth, At::Term(&ty), origin, &mine, &theirs)
                .map_err(|failure| failure.under(PathStep::Field(Arc::clone(name))))?;
        }
        Ok(())
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
        let heads_agree = match (&one.head, &other.head) {
            (Head::Var(level, _), Head::Var(other_level, _)) => level.0 == other_level.0,
            // Rigid like a variable, and decided the same way: a constant is its
            // name, so there is nothing under it to unify.
            (Head::Const(left), Head::Const(right)) => left == right,
            // Two metavariables reach this only in [`Self::deciding`] mode,
            // where §3 is being *asked* rather than made true and an unsolved
            // metavariable is as rigid as a variable: the same one is equal to
            // itself, and two different ones are two different unknowns. The
            // elaborating unifier never arrives here with one, because
            // `flexible` answers `Solved` for a meta against itself and
            // `Postpone` for two different ones.
            (Head::Meta(left), Head::Meta(right)) => left == right,
            // Rigid for good: §5.8 gives a base type no eliminator, so nothing
            // under one could ever unblock it, and a builtin still headed here
            // has an argument that is not a literal. Both decide by name, like a
            // constant.
            (Head::Base(left), Head::Base(right)) => left == right,
            (Head::Builtin(left), Head::Builtin(right)) => left == right,
            // Two different kinds of head, which never agree.
            (Head::Meta(_) | Head::Var(_, _) | Head::Const(_) | Head::Base(_) | Head::Builtin(_), _) => false,
        };
        // One comparison decides a length disagreement, before any argument is
        // compared. The chain representation had to walk both to find out.
        if !heads_agree || one.spine.len() != other.spine.len() {
            return Err(blocked_mismatch(meter, depth, one, other)?);
        }
        let mut prefix = Neutral::head(one.origin, one.head.clone());
        for (mine, theirs) in one.spine.iter().zip(other.spine.iter()) {
            match (mine, theirs) {
                (
                    Elim::App {
                        argument: left_argument,
                        ..
                    },
                    Elim::App {
                        argument: right_argument,
                        ..
                    },
                ) => {
                    let Form::Pi { domain, .. } = head_type(meter, &prefix)?.form else {
                        return Err(blocked_mismatch(meter, depth, one, other)?);
                    };
                    self.step(meter, depth, At::Term(&domain), origin, left_argument, right_argument)
                        .map_err(|failure| failure.under(PathStep::Argument))?;
                }
                (Elim::Project { field: left_field, .. }, Elim::Project { field: right_field, .. })
                    if left_field == right_field => {}
                (
                    Elim::J {
                        origin: here,
                        ty: left_ty,
                        from: left_from,
                        motive: left_motive,
                        base: left_base,
                        to: left_to,
                    },
                    Elim::J {
                        ty: right_ty,
                        from: right_from,
                        motive: right_motive,
                        base: right_base,
                        to: right_to,
                        ..
                    },
                ) => {
                    self.step(meter, depth, At::Type, origin, left_ty, right_ty)?;
                    self.step(meter, depth, At::Term(left_ty), origin, left_from, right_from)?;
                    self.step(meter, depth, At::Term(left_ty), origin, left_to, right_to)?;
                    self.motives(meter, depth, origin, left_ty, left_from, left_motive, right_motive)?;
                    // The base case's type is the motive at `(from, from, refl
                    // from)`, computed the same way quotation computes it.
                    let base_type = {
                        let at_from = apply(meter, *here, Value::clone(left_motive), Value::clone(left_from))?;
                        let reflexive = Value::new(left_from.origin, Form::Refl(Arc::clone(left_from)));
                        apply(meter, *here, at_from, reflexive)?
                    };
                    self.step(meter, depth, At::Term(&base_type), origin, left_base, right_base)?;
                }
                (Elim::App { .. } | Elim::Project { .. } | Elim::J { .. }, _) => {
                    return Err(blocked_mismatch(meter, depth, one, other)?);
                }
            }
            prefix.spine.push(mine.clone());
        }
        Ok(())
    }

    /// Compare two `J` motives at the shape `(y : A) → Id A x y → Type l`.
    ///
    /// The motive's Π type is never built as a value — `l` is recorded nowhere
    /// — so this does what quotation does with the same problem: apply both to
    /// two fresh variables whose types *are* known, and compare the results as
    /// types. η at Π says that decides it.
    fn motives(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        origin: Origin,
        ty: &Arc<Value>,
        from: &Arc<Value>,
        one: &Value,
        other: &Value,
    ) -> Step {
        let endpoint = Value::var(Origin::UNKNOWN, DbLevel(depth), Arc::clone(ty));
        let identity = Arc::new(Value::new(
            one.origin,
            Form::Id {
                ty: Arc::clone(ty),
                left: Arc::clone(from),
                right: Arc::new(endpoint.clone()),
            },
        ));
        let under = depth.saturating_add(1);
        let witness = Value::var(Origin::UNKNOWN, DbLevel(under), identity);
        let mut opened = |motive: &Value| -> Result<Value, CoreError> {
            let at_endpoint = apply(meter, motive.origin, motive.clone(), endpoint.clone())?;
            apply(meter, motive.origin, at_endpoint, witness.clone())
        };
        let (left, right) = (opened(one)?, opened(other)?);
        self.step(meter, under.saturating_add(1), At::Type, origin, &left, &right)
            .map_err(|failure| failure.under(PathStep::Body))
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
///
/// Asked on both sides of every step, which is why the head is a field rather
/// than the deepest node of a chain.
fn flexible_head(value: &Value) -> Option<&Meta> {
    let Form::Neutral(neutral) = &value.form else {
        return None;
    };
    match &neutral.head {
        Head::Var(_, _) | Head::Const(_) | Head::Base(_) | Head::Builtin(_) => None,
        Head::Meta(meta) => Some(meta),
    }
}
