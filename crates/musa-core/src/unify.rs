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
use crate::eval::{apply, apply_closure, eval, field_type, force, head_type, opened, project};
use crate::origin::Origin;
use crate::quote::{Depth, Mode, quote, quote_type};
use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
use crate::term::{DbLevel, Field, Term};
use crate::value::{Closure, DefHead, Elim, Form, Head, Neutral, Telescope, Value};

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
            Self::Type => quote_type(meter, Depth(depth), Mode::Keep, value),
            Self::Term(ty) => quote(meter, Depth(depth), Mode::Keep, ty, value),
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
/// The conversion checker's state: which variables a matching pass may solve,
/// and what it has solved them to.
///
/// Empty in the ordinary case, because conversion is a *rigid* question — two
/// values are equal or they are not. The one non-rigid question the language
/// still asks is `02-core-calculus.md` §2.1's: the type parameters of a callee,
/// which the written arguments determine. That question is first-order
/// matching, and it is this same engine with the parameter variables listed as
/// assignable — one algorithm with two modes, which is what keeps "the
/// conversion checker" one thing rather than two to keep in agreement.
#[derive(Default)]
pub(crate) struct Unifier {
    /// Whether this unifier answers a question rather than making one true.
    ///
    /// Set by [`Self::deciding`], and read in exactly one place: the
    /// assignment rule, which a deciding pass never fires — a hole is an
    /// opaque head there, compared by identity like any other.
    deciding: bool,
}

impl Unifier {
    /// A unifier that decides §3's conversion instead of solving for it.
    ///
    /// Definitional equality *is* the rigid fragment of the one algorithm —
    /// the same type-directed walk with η and early exit, over values whose
    /// holes are opaque heads rather than unknowns to determine. So
    /// [`crate::convertible`] is this constructor and not a second procedure:
    /// two implementations of one question are two things to keep in
    /// agreement.
    pub(crate) fn deciding() -> Self {
        Self { deciding: true }
    }

    /// Make `left` and `right` the same type.
    ///
    /// # Errors
    ///
    /// [`Refusal::Mismatch`] when they are not, or a [`CoreError`] the walk
    /// spent itself into.
    pub(crate) fn unify_types(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: Origin,
        left: &Value,
        right: &Value,
    ) -> Result<(), ElabError> {
        self.step(meter, depth, At::Type, at, left, right)
            .map_err(|failure| failure.into_error(at).rooted(At::Type, meter, depth, left, right))
    }

    /// Make `left` and `right` equal as inhabitants of `ty`.
    ///
    /// Separate from [`Self::unify_types`] because quotation is type-directed
    /// (§3): a mismatch is read back at the sort the pair was compared at, and
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
            .map_err(|failure| failure.into_error(at).rooted(At::Term(ty), meter, depth, left, right))
    }

    /// One step: the one flexible case, then the structural descent.
    fn step(&mut self, meter: &mut Meter, depth: u32, at: At<'_>, origin: Origin, left: &Value, right: &Value) -> Step {
        // The nesting charge is what keeps conversion inside §4.1's limit, and
        // it can only report a [`CoreError`], so the step's own answer travels
        // back inside its `Ok`.
        meter.nested::<Step, CoreError>("unification", |meter| {
            meter.step("unification")?;
            let unfolded_left = force(meter, left)?;
            let left = unfolded_left.as_ref().unwrap_or(left);
            let unfolded_right = force(meter, right)?;
            let right = unfolded_right.as_ref().unwrap_or(right);
            Ok(match self.assignment(meter, depth, at, origin, left, right) {
                Ok(Some(())) => Ok(()),
                Ok(None) => self.folded(meter, depth, at, origin, left, right),
                Err(failure) => Err(failure),
            })
        })?
    }

    /// The one flexible case: an unsolved hole, unapplied, on the pattern
    /// side.
    ///
    /// `Some` is "handled" and `None` is "rigid", which descends as any other
    /// pair. An applied hole is rigid by choice: solving one would be
    /// higher-order, and §2.1 admits first-order assignment only — so an
    /// unsolved applied hole compares by identity and is reported as the
    /// mismatch it is, which is §2.1's "the program did not say" with the
    /// application as the place that could not say it.
    fn assignment(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Result<Option<()>, Failure> {
        if self.deciding {
            return Ok(None);
        }
        let Form::Neutral(neutral) = &left.form else {
            return Ok(None);
        };
        let Head::Hole(hole) = &neutral.head else {
            return Ok(None);
        };
        if !neutral.spine.is_empty() {
            return Ok(None);
        }
        debug_assert!(
            hole.solution().is_none(),
            "a solved hole is forced before the assignment rule can meet it"
        );
        // The occurs check, at its first-order strength: the unknown may not
        // occur in its own answer, transitively included.
        if mentions_hole(right, hole) {
            return Err(Failure::Occurs);
        }
        let _ = (meter, depth, at, origin);
        hole.solve(right.clone()).map_err(|malformed| Failure::Core(malformed.into()))?;
        Ok(Some(()))
    }

    /// Neither side is flexible: the folded-definition cases, and then the
    /// structural descent.
    ///
    /// Two uses of the *same* definition try the folded comparison first — same
    /// head, convertible spines, nothing unfolded — which is the whole of the
    /// win on this path. On a spine disagreement either side opens, since both
    /// unfold to the same value. Otherwise one side opens and the comparison is
    /// retried, with the side chosen by **the one that can mention the other
    /// goes first**: a local before a global (a `let`'s value may name a
    /// program definition and the reverse is impossible), the larger level
    /// before the smaller, the larger name between two globals. The order is
    /// fixed so that the answer never depends on which branch ran first — §3's
    /// decision procedure rather than a heuristic — and the choice between two
    /// names is arbitrary because a definition's position is not stored and
    /// determinism is all that is owed.
    ///
    /// Retrying terminates: a local's value was evaluated before its binder was
    /// pushed, so the folded heads inside it carry strictly smaller levels, and
    /// a global's carry only earlier definitions — a recursive global's
    /// self-reference stands under a λ the recursor plan built, never at the
    /// head. An unfold chain is therefore finite before any meter is consulted.
    fn folded(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Step {
        let open = |meter: &mut Meter, folded: FoldedDef<'_>| -> Result<Value, Failure> {
            Ok(crate::eval::unfold_spine(meter, folded.value, &folded.neutral.spine)?)
        };
        match (folded_def(left), folded_def(right)) {
            (None, None) => self.rigid(meter, depth, at, origin, left, right),
            (Some(one), None) => {
                let left = open(meter, one)?;
                self.step(meter, depth, at, origin, &left, right)
            }
            (None, Some(other)) => {
                let right = open(meter, other)?;
                self.step(meter, depth, at, origin, left, &right)
            }
            (Some(one), Some(other)) if one.identity == other.identity => {
                match self.rigid(meter, depth, at, origin, left, right) {
                    // The unfolded comparison decides the question — the spines
                    // may agree once the definition is open — but when it also
                    // fails, the folded failure is the one reported: it names
                    // what the author wrote.
                    Err(folded @ Failure::Mismatch { .. }) => {
                        let left = open(meter, one)?;
                        match self.step(meter, depth, at, origin, &left, right) {
                            Err(Failure::Mismatch { .. }) => Err(folded),
                            decided => decided,
                        }
                    }
                    decided => decided,
                }
            }
            (Some(one), Some(other)) => {
                if unfolds_first(one.identity, other.identity) {
                    let left = open(meter, one)?;
                    self.step(meter, depth, at, origin, &left, right)
                } else {
                    let right = open(meter, other)?;
                    self.step(meter, depth, at, origin, left, &right)
                }
            }
        }
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
            let unfolded = opened(meter, ty)?;
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
            // Two universes agree when their levels are the same: §1 fixes the
            // levels at two, so this is an equality and never a search.
            (Form::Universe(one), Form::Universe(other)) => {
                if one == other {
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
            // A hole is as rigid as a variable here: the same one is equal
            // to itself, and two different ones are two different unknowns.
            // The matching pass never arrives here with an unsolved one on the
            // left — `assignment` answers first — and a solved one is forced
            // before the walk sees it.
            (Head::Hole(left), Head::Hole(right)) => left == right,
            // Rigid for good: §5.8 gives a base type no eliminator, so nothing
            // under one could ever unblock it, and a builtin still headed here
            // has an argument that is not a literal. Both decide by name, like a
            // constant.
            (Head::Base(left), Head::Base(right)) => left == right,
            (Head::Builtin(left), Head::Builtin(right)) => left == right,
            // Reached only at one identity: [`Self::folded`] opens a definition
            // facing anything but itself, so two `Def` heads here are the same
            // definition and the spines decide.
            (Head::Def(one, _, _), Head::Def(other, _, _)) => one == other,
            // Two different kinds of head, which never agree.
            (
                Head::Hole(_)
                | Head::Var(_, _)
                | Head::Const(_)
                | Head::Base(_)
                | Head::Builtin(_)
                | Head::Def(_, _, _),
                _,
            ) => false,
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

                (Elim::App { .. } | Elim::Project { .. }, _) => {
                    return Err(blocked_mismatch(meter, depth, one, other)?);
                }
            }
            prefix.spine.push(mine.clone());
        }
        Ok(())
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

impl ElabError {
    /// This error, with the two values its comparison started from attached
    /// when it is a [`Refusal::Mismatch`].
    ///
    /// Read back in the entry point's own sort, so the roots are spelled the
    /// way the sides were compared; a read-back that cannot complete — the
    /// budget the comparison just spent, for one — leaves the refusal to its
    /// endpoints rather than spending a second refusal on the first.
    fn rooted(self, at: At<'_>, meter: &mut Meter, depth: u32, left: &Value, right: &Value) -> Self {
        let mut error = self;
        if let Self::Refused(Refusal::Mismatch(mismatch)) = &mut error
            && let (Ok(expected), Ok(found)) = (at.quote(meter, depth, left), at.quote(meter, depth, right))
        {
            mismatch.whole = Some(Box::new((expected, found)));
        }
        error
    }
}

/// What one step of unification answers.
type Step = Result<(), Failure>;

/// Why a step did not succeed.
enum Failure {
    /// The occurs check refused: the program side mentions a variable the
    /// match could still solve, so the unknown would occur in its own answer.
    Occurs,
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
            Self::Occurs => Self::Occurs,
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
            Self::Occurs => Refusal::Unsolved {
                site: crate::meta::MetaSource::TypeParameter,
                created: at,
                blocked: None,
            }
            .into(),
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
                    whole: None,
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
        expected: quote_type(meter, Depth(depth), Mode::Keep, &Value::shared_neutral(one))?,
        found: quote_type(meter, Depth(depth), Mode::Keep, &Value::shared_neutral(other))?,
        path: Vec::new(),
    })
}

/// The unsolved metavariable at the head of a blocked elimination, if there is
/// one.
///
/// would otherwise paper over.
struct FoldedDef<'a> {
    neutral: &'a Neutral,
    identity: &'a DefHead,
    value: &'a Value,
}

/// The folded use of a definition a value is, if it is one.
fn folded_def(value: &Value) -> Option<FoldedDef<'_>> {
    let Form::Neutral(neutral) = &value.form else {
        return None;
    };
    let Head::Def(identity, _, carried) = &neutral.head else {
        return None;
    };
    Some(FoldedDef {
        neutral,
        identity,
        value: carried,
    })
}

/// Which of two different folded definitions opens first: the one that can
/// mention the other. See [`Unifier::folded`].
fn unfolds_first(one: &DefHead, other: &DefHead) -> bool {
    match (one, other) {
        (DefHead::Local(_), DefHead::Global(_)) => true,
        (DefHead::Global(_), DefHead::Local(_)) => false,
        (DefHead::Local(this), DefHead::Local(that)) => this.0 > that.0,
        (DefHead::Global(this), DefHead::Global(that)) => this.name() > that.name(),
    }
}

/// Does `value` mention this hole, unsolved, transitively?
///
/// The occurs check of [`Unifier::assignment`]: the unknown may not occur in
/// its own answer, and "transitively" is through other holes' solutions. A
/// closure or telescope's *terms* are not walked: the value this asks about is
/// one the walk created, and a term can reach one only through the environment
/// that is walked.
fn mentions_hole(value: &Value, target: &crate::meta::Hole) -> bool {
    match &value.form {
        Form::Universe(_) | Form::Lit(_) | Form::Numeral(_) => false,
        Form::Pi { domain, codomain, .. } => {
            mentions_hole(domain, target)
                || codomain.env.iter().any(|item| mentions_hole(item, target))
        }
        Form::Lam(closure) => closure.env.iter().any(|item| mentions_hole(item, target)),
        Form::RecordType(telescope) => telescope.env.iter().any(|item| mentions_hole(item, target)),
        Form::Record(fields) => fields.iter().any(|(_, item)| mentions_hole(item, target)),
        Form::Neutral(neutral) => {
            let head_mentions = match &neutral.head {
                Head::Hole(hole) if hole == target => return true,
                Head::Hole(hole) => hole.solution().is_some_and(|solution| mentions_hole(solution, target)),
                Head::Var(_, ty) => mentions_hole(ty, target),
                Head::Const(_) | Head::Base(_) | Head::Builtin(_) => false,
                Head::Def(_, ty, folded) => mentions_hole(ty, target) || mentions_hole(folded, target),
            };
            head_mentions
                || neutral.spine.iter().any(|elimination| match elimination {
                    Elim::App { argument, .. } => mentions_hole(argument, target),
                    Elim::Project { .. } => false,
                })
        }
    }
}

/// Does `value` mention a hole nothing has solved?
///
/// The elaborator's reading of `02-core-calculus.md` §2.1's "inferable": an
/// argument whose domain still mentions an unsolved hole is one whose type the
/// call has not determined, so the argument is *inferred* and the match learns
/// the parameter from it. The same walk as [`mentions_hole`], existentially.
pub(crate) fn mentions_unsolved(value: &Value) -> bool {
    match &value.form {
        Form::Universe(_) | Form::Lit(_) | Form::Numeral(_) => false,
        Form::Pi { domain, codomain, .. } => {
            mentions_unsolved(domain) || codomain.env.iter().any(mentions_unsolved)
        }
        Form::Lam(closure) => closure.env.iter().any(mentions_unsolved),
        Form::RecordType(telescope) => telescope.env.iter().any(mentions_unsolved),
        Form::Record(fields) => fields.iter().any(|(_, item)| mentions_unsolved(item)),
        Form::Neutral(neutral) => {
            let head_mentions = match &neutral.head {
                Head::Hole(hole) => match hole.solution() {
                    Some(solution) => mentions_unsolved(solution),
                    None => true,
                },
                Head::Var(_, ty) => mentions_unsolved(ty),
                Head::Const(_) | Head::Base(_) | Head::Builtin(_) => false,
                Head::Def(_, ty, folded) => mentions_unsolved(ty) || mentions_unsolved(folded),
            };
            head_mentions
                || neutral.spine.iter().any(|elimination| match elimination {
                    Elim::App { argument, .. } => mentions_unsolved(argument),
                    Elim::Project { .. } => false,
                })
        }
    }
}
