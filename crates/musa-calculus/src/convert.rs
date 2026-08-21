//! Conversion: whether two values are the same, and — in the one mode that
//! solves — what an unwritten argument has to be for them to be.
//!
//! `docs/rules/language/02-core-calculus.md` §3 decides definitional equality
//! by α-equality of η-long normal forms, computed by `NbE` over the small core.
//! This module is that decision, written as a type-directed walk with early
//! exit rather than as normalize-and-compare, so that two values that differ
//! at their heads cost one comparison instead of two normal forms.
//!
//! # One walk, two modes
//!
//! §2.1's other question — the type parameters of a callee, which the written
//! arguments determine — is the *same* walk with the holes made assignable.
//! There is no second procedure and no heuristic: a hole standing alone takes
//! the value it is compared with, once, and a hole under a spine is an opaque
//! head like any other. [`Conversion::deciding`] is the rigid mode and
//! [`Conversion::solving`] the assigning one, and the only line that reads the
//! difference is the assignment rule.
//!
//! Nothing here postpones, retries, or reaches a fixpoint. A comparison that
//! cannot be decided is a [`Refusal::Mismatch`] at the site that asked, and a
//! hole nothing determined is [`Refusal::Unsolved`] when the declaration ends.
//!
//! # Why solving quotes at a type
//!
//! A solution is a term, so the right-hand *value* has to be read back, and
//! quotation in this crate is type-directed (§3, η). The type is not invented:
//! the walk already knows what the two sides are at, because it descended to
//! them from a pair of types it knew. That is why nothing here needs a second,
//! untyped quotation function, and why a solution comes out η-long for free.
//!
//! # The third question this file answers
//!
//! Which of two folded definitions to unfold first. It lives here because it
//! is only ever asked in the middle of a comparison, and answering it anywhere
//! else would mean a second place that knows what a glued definition is.

use std::sync::Arc;

use crate::base::Operator;
use crate::budget::Meter;
use crate::error::CoreError;
use crate::eval::{apply, apply_closure, eval, field_type, force, head_type, opened, project};
use crate::index::{self, Exact, Expr, Sort, Verdict};
use crate::origin::Origin;
use crate::quote::{Depth, Mode, quote, quote_type};
use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
use crate::term::{DbLevel, Field, Shape, Term};
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

impl At<'_> {
    fn quote(self, meter: &mut Meter, depth: u32, value: &Value) -> Result<Term, CoreError> {
        match self {
            Self::Type => quote_type(meter, Depth(depth), Mode::Keep, value),
            Self::Term(ty) => quote(meter, Depth(depth), Mode::Keep, ty, value),
        }
    }
}

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
pub(crate) struct Conversion {
    /// Whether this checker answers a question rather than making one true.
    ///
    /// Set by [`Self::deciding`], and read in exactly one place: the
    /// assignment rule, which a deciding pass never fires — a hole is an
    /// opaque head there, compared by identity like any other.
    deciding: bool,
}

impl Conversion {
    /// A checker that decides §3's conversion instead of solving for it.
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

    /// A checker that may solve a hole it meets alone on one side.
    ///
    /// The elaborator's mode, and the only one that assigns. Named rather than
    /// left to [`Default`] so that the two modes read as a pair at every
    /// construction site.
    pub(crate) fn solving() -> Self {
        Self { deciding: false }
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
                Ok(None) => match self.assignment(meter, depth, at, origin, right, left) {
                    Ok(Some(())) => Ok(()),
                    Ok(None) => self.folded(meter, depth, at, origin, left, right),
                    Err(failure) => Err(failure),
                },
                Err(failure) => Err(failure),
            })
        })?
    }

    /// The one flexible case: an unsolved hole, unapplied, on either side —
    /// the step tries both, because "the pattern side" is a direction the
    /// caller picks, not a property of the values.
    ///
    /// `Some` is "handled" and `None` is "rigid", which descends as any other
    /// pair. An applied hole is rigid by choice: solving one would be
    /// higher-order, and §2.1 admits first-order assignment only — so an
    /// unsolved applied hole compares by identity and is reported as the
    /// mismatch it is, which is §2.1's "the program did not say" with the
    /// application as the place that could not say it.
    fn assignment(
        &self,
        meter: &Meter,
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
        // The reflexive case: the right forces back to this same hole, which
        // happens where two holes have already been chained — `?l ≡ ?r` when
        // `?r` was solved to `?l` is not an occurs failure, it is the one
        // solution the pair already has.
        if let Form::Neutral(right_neutral) = &right.form
            && let Head::Hole(right_hole) = &right_neutral.head
            && right_neutral.spine.is_empty()
            && right_hole == hole
        {
            return Ok(Some(()));
        }
        // The occurs check, at its first-order strength: the unknown may not
        // occur in its own answer, transitively included.
        if mentions_hole(right, hole) {
            return Err(Failure::Occurs);
        }
        let _ = (meter, depth, at, origin);
        hole.solve(right.clone())
            .map_err(|malformed| Failure::Core(malformed.into()))?;
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
                // An indexed type has whatever η what it refines has, and no η of
                // its own: §1.5 gives it no elimination form, so there is
                // nothing to expand. Left to the match below, which compares the
                // two values directly and is where the index question is asked.
                Form::Indexed { .. } => {}
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
            // §1.5's one hook. The refined types are compared by §3 as any two
            // types are; the indices are handed to the arithmetic decider, which
            // is a different question with a different answer procedure.
            (
                Form::Indexed {
                    ty: mine,
                    index: my_index,
                },
                Form::Indexed {
                    ty: theirs,
                    index: their_index,
                },
            ) => {
                self.step(meter, depth, At::Type, origin, mine, theirs)?;
                self.indices(meter, depth, origin, left, right, my_index, their_index)
            }
            // An indexed type never converts with what it refines. `Row(12)` and
            // `Row` are two types, and reading them back would print one word
            // twice, because erasure is what quotation does — so the message is
            // built here, where the index is still in hand.
            (Form::Indexed { .. }, _) | (_, Form::Indexed { .. }) => Err(Failure::Mismatch {
                expected: indexed_shown(meter, depth, left)?,
                found: indexed_shown(meter, depth, right)?,
                path: Vec::new(),
            }),
            (Form::Neutral(one), Form::Neutral(other)) => self.neutrals(meter, depth, origin, one, other),
            // Two different forms, which is a disagreement: reading both sides
            // back is how the message says so. The one pair that is not already
            // decided by the time it lands here is a λ or a record literal whose
            // type is still unknown, and quotation refuses that rather than
            // guessing.
            _ => Self::by_reading_back(meter, depth, at, left, right),
        }
    }

    /// Two index arguments of the same refined type (§1.5).
    ///
    /// The one place in this crate where a comparison is *not* answered by
    /// normalization by evaluation. §1.5's design is that two questions get two
    /// deciders: `Γ ⊢ T(a) ≡ T(b)` holds exactly when the solver decides `a = b`
    /// in the index domain, and §3's machinery is not involved because it
    /// decides terms and an index is not one.
    ///
    /// The exception is a flexible index, and it is not an exception to the
    /// separation. An index variable is an *ordinary parameter of index sort*
    /// (§1.5), solved at the call by §2.1's first-order matching from the
    /// written arguments — the same binder and the same rule a type parameter
    /// gets. So a side that still holds an unsolved hole is §2.1's question,
    /// asked of the ordinary walk; only once both sides are rigid is there
    /// arithmetic to decide.
    fn indices(
        &mut self,
        meter: &mut Meter,
        depth: u32,
        origin: Origin,
        left: &Value,
        right: &Value,
        mine: &Value,
        theirs: &Value,
    ) -> Step {
        let disagree = |meter: &mut Meter| -> Result<Failure, CoreError> {
            Ok(Failure::Mismatch {
                expected: indexed_shown(meter, depth, left)?,
                found: indexed_shown(meter, depth, right)?,
                path: Vec::new(),
            })
        };
        if mentions_unsolved(mine) || mentions_unsolved(theirs) {
            return match self.step(meter, depth, At::Type, origin, mine, theirs) {
                Err(Failure::Mismatch { .. }) => Err(disagree(meter)?),
                decided => decided,
            };
        }
        // Both sides are readable by construction. [`crate::Refusal::UnreadableIndex`]
        // refuses an index outside §1.5's grammar where the type is *formed* —
        // which is where §1.5 sites it ("named at the expression") and the one
        // place the written expression still exists to be named — so an
        // unreadable index never reaches a comparison.
        //
        // This arm is therefore a broken invariant and not a verdict. The code
        // it replaces answered `disagree` here, and answering "different" for
        // two indices nothing can read is what made `Row(mystery n)` fail to be
        // the same type as itself. §1.5 still forbids a syntactic fallback, and
        // this needs none: there is nothing left to fall back *for*.
        let (Some(mine), Some(theirs)) = (index_of(meter, mine)?, index_of(meter, theirs)?) else {
            return Err(CoreError::Malformed(crate::error::Malformed::UnreadableIndex).into());
        };
        match index::decide(&mine, &theirs) {
            Verdict::Same => Ok(()),
            Verdict::Different => Err(disagree(meter)?),
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
/// mention the other. See [`Conversion::folded`].
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
/// The occurs check of [`Conversion::assignment`]: the unknown may not occur in
/// its own answer, and "transitively" is through other holes' solutions. A
/// closure or telescope's *terms* are not walked: the value this asks about is
/// one the walk created, and a term can reach one only through the environment
/// that is walked.
fn mentions_hole(value: &Value, target: &crate::meta::Hole) -> bool {
    match &value.form {
        Form::Universe(_) | Form::Lit(_) | Form::Numeral(_) => false,
        // Both halves. `Row(?α)` mentions `?α`, and an occurs check that looked
        // past the index would let a hole be solved by a value that names it.
        Form::Indexed { ty, index } => mentions_hole(ty, target) || mentions_hole(index, target),
        Form::Pi { domain, codomain, .. } => {
            mentions_hole(domain, target) || codomain.env.iter().any(|item| mentions_hole(item, target))
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
        // Both halves, for [`mentions_hole`]'s reason read existentially: a
        // slot typed `Row(?n)` has not been determined by the call.
        Form::Indexed { ty, index } => mentions_unsolved(ty) || mentions_unsolved(index),
        Form::Pi { domain, codomain, .. } => mentions_unsolved(domain) || codomain.env.iter().any(mentions_unsolved),
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

/// A refined type as it was written, for a message that has to show the index.
///
/// [`quote_type`] cannot be used for the whole of it: erasure is what quotation
/// does (§1.5), so reading `Row(12)` back answers `Row`, and a mismatch between
/// two indexed types of one type would print the same word twice. So the wrapper
/// is rebuilt here, where both halves are still values.
///
/// A value that is not an indexed type quotes ordinarily — this is the one arm of
/// the pair where an indexed type met a bare type.
fn indexed_shown(meter: &mut Meter, depth: u32, value: &Value) -> Result<Term, CoreError> {
    let Form::Indexed { ty, index } = &value.form else {
        return quote_type(meter, Depth(depth), Mode::Keep, value);
    };
    Ok(Term::indexed(
        value.origin,
        quote_type(meter, Depth(depth), Mode::Keep, ty)?,
        index_shown(meter, depth, index)?,
    ))
}

/// One index value, as a term a message can print.
///
/// The two canonical index forms are read back directly, because neither is a
/// type and [`quote_type`] refuses both; everything else in the grammar — a
/// variable, an open application of an arithmetic builtin — is a neutral and
/// quotes as one.
fn index_shown(meter: &mut Meter, depth: u32, value: &Value) -> Result<Term, CoreError> {
    match &value.form {
        Form::Numeral(numeral) => Ok(Term::new(value.origin, Shape::Numeral(numeral.clone()))),
        Form::Lit(literal) => Ok(literal.term(value.origin)),
        Form::Universe(_)
        | Form::Pi { .. }
        | Form::Lam(_)
        | Form::RecordType(_)
        | Form::Record(_)
        | Form::Indexed { .. }
        | Form::Neutral(_) => quote_type(meter, Depth(depth), Mode::Keep, value),
    }
}

/// An index position read as §1.5's index expression, or `None` at the first
/// thing outside the grammar.
///
/// This is the *only* function that knows the grammar, which is what keeps
/// [`crate::index`] free of [`crate::value`]: it is handed a normalized
/// expression and answers, and the reading happens at the one place a
/// comparison meets an index.
///
/// `None` is a refusal and never an approximation. Two variables multiplied, a
/// division, a call, a `match`, a projection, an overflow, and a variable of any
/// sort but `Nat` and exact `Ratio` each land here, and the caller reports the
/// written expression rather than anything about a linear form.
/// Whether a value stands for an index `02-core-calculus.md` §1.5's grammar can
/// read.
///
/// The one place type formation asks, and it is deliberately the *same* reader
/// conversion uses rather than a second one that would have to be kept agreeing
/// with it. §1.5 restricts what an index may say; asking here is what keeps an
/// unreadable one out of every later comparison, so that [`crate::index::decide`]
/// is total on what reaches it and `≡` is reflexive.
pub(crate) fn reads_as_index(meter: &mut Meter, value: &Value) -> Result<bool, CoreError> {
    Ok(index_of(meter, value)?.is_some())
}

fn index_of(meter: &mut Meter, value: &Value) -> Result<Option<Expr>, CoreError> {
    meter.nested("index reading", |meter| {
        meter.step("index reading")?;
        let opened = opened(meter, value)?;
        let value = opened.as_ref().unwrap_or(value);
        match &value.form {
            // A counting family holds its count directly, so `Nat` needs no
            // host reader: §5.8's opacity is about a base type's payload, and a
            // numeral is not one.
            Form::Numeral(numeral) => Ok(Some(Expr::literal(
                Sort::Count,
                Exact::whole(i128::from(numeral.count)),
            ))),
            Form::Lit(literal) => Ok(measured(literal)),
            Form::Neutral(neutral) => match (&neutral.head, neutral.spine.as_slice()) {
                (Head::Var(DbLevel(level), ty), []) => Ok(sort_of(ty).map(|sort| Expr::variable(sort, *level))),
                (Head::Builtin(builtin), [Elim::App { argument: left, .. }, Elim::App { argument: right, .. }]) => {
                    match builtin.indexes() {
                        Some(operator) => arithmetic(meter, operator, left, right),
                        None => Ok(None),
                    }
                }
                _ => Ok(None),
            },
            Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::RecordType(_) | Form::Record(_) => Ok(None),
            // An indexed type is a type, and §1.5 refuses an index over one: that
            // would be a universe by another name.
            Form::Indexed { .. } => Ok(None),
        }
    })
}

/// One open application of an arithmetic builtin, read as a linear form.
fn arithmetic(meter: &mut Meter, operator: Operator, left: &Value, right: &Value) -> Result<Option<Expr>, CoreError> {
    let (Some(left), Some(right)) = (index_of(meter, left)?, index_of(meter, right)?) else {
        return Ok(None);
    };
    match operator {
        Operator::Add => left.add(meter, &right),
        Operator::Subtract => left.subtract(meter, &right),
        // §1.5 admits multiplication by a *literal*, so exactly one side has to
        // be closed. Two open factors is the refusal the grammar names, and it
        // is the refusal that keeps the form linear.
        Operator::Multiply => match (left.as_constant(), right.as_constant()) {
            (Some(factor), _) => right.scale(meter, factor),
            (None, Some(factor)) => left.scale(meter, factor),
            (None, None) => Ok(None),
        },
    }
}

/// A base literal read as an exact index value by its own type's host rule.
///
/// D1 keeps a payload opaque to this crate, so the party that put the number in
/// is the only one that can take it out — [`crate::base::Measures`], registered
/// on the base type. A literal at a base type that registered none is not an
/// index, which is the intended default and the reason `Syntax<Cat>`'s category
/// does not become one.
fn measured(literal: &crate::base::Literal) -> Option<Expr> {
    let mut head = literal.ty();
    while let Shape::App { function, .. } = head.shape() {
        head = function;
    }
    let Shape::Base(base) = head.shape() else {
        return None;
    };
    let (numerator, denominator) = base.measures()?(literal)?;
    Exact::new(numerator, denominator).map(|value| Expr::literal(Sort::Rational, value))
}

/// Which index sort a variable's type puts it in, if any.
///
/// The two sorts a solver is wanted for, and no third: §1.5's finite literal
/// enums are decided by [`crate::base::Payload::same`] at an ordinary base type
/// and have nothing linear to normalize. A variable of any other type is
/// outside the grammar, which is the refusal §1.5 names.
fn sort_of(ty: &Value) -> Option<Sort> {
    let Form::Neutral(neutral) = &ty.form else {
        return None;
    };
    match &neutral.head {
        Head::Const(constant) => constant.counting().map(|_| Sort::Count),
        Head::Base(base) => base.measures().map(|_| Sort::Rational),
        Head::Var(_, _) | Head::Def(_, _, _) | Head::Builtin(_) | Head::Hole(_) => None,
    }
}
