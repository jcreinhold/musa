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
//! arguments determine — is the *same* walk with the metas made assignable.
//! There is no second procedure and no heuristic: a meta standing alone takes
//! the value it is compared with, once, and a meta under a spine is an opaque
//! head like any other. [`Conversion::deciding`] is the rigid mode and
//! [`Conversion::solving`] the assigning one, and the only line that reads the
//! difference is the assignment rule.
//!
//! Nothing here postpones, retries, or reaches a fixpoint. A comparison that
//! cannot be decided is a [`Refusal::Mismatch`] at the site that asked, and a
//! meta nothing determined is [`Refusal::Unsolved`] when the declaration ends.
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

use crate::elaboration::refuse::{ElabError, Mismatch, PathStep, Refusal};
use crate::kernel::budget::Meter;
use crate::kernel::error::CoreError;
use crate::kernel::eval::{apply, apply_closure, force, head_type, opened};
use crate::kernel::family::{Product, product};
use crate::kernel::origin::Origin;
use crate::kernel::quote::{At, Mode, quote_type};
use crate::kernel::sort::{Sort, SortVar};
use crate::kernel::term::{Level, Term};
use crate::kernel::unify::{self, Outcome, Postponed, Queue};
use crate::kernel::value::{Closure, DefHead, Elim, Form, Head, Neutral, Value};

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
    /// assignment rule, which a deciding pass never fires — a meta is an
    /// opaque head there, compared by identity like any other.
    deciding: bool,
    /// The universe-level equations no single assignment answered, kept until
    /// the declaration's finish point.
    ///
    /// The one thing this module postpones, and §1's own reason for it:
    /// `max ?u ?v ≡ 3` has several solutions, so a solver that picked one here
    /// would decide a program's meaning on the order its constraints arrived
    /// in. They are re-read once, after defaulting, by
    /// [`Elaborator::settled`](crate::elaboration::elab::Elaborator) — which is
    /// also why a *deciding* pass never fills this: it has no finish point to
    /// drain it at, so it answers immediately or refuses.
    levels: Vec<LevelEquation>,
    /// The comparisons no assignment could decide yet.
    ///
    /// §2.1 makes this queue part of the judgment: a comparison that is not
    /// decidable *now* is postponed and retried when an unknown it mentions is
    /// solved, and only the declaration's end turns a survivor into an error.
    /// The type lives in [`crate::kernel::unify`] because a blocked pair is a
    /// fact about two values with no author to address; what lives here is the
    /// decision to keep it, which is a decision about diagnostics.
    ///
    /// A *deciding* pass never fills it, for the reason the levels above are
    /// not filled either: it has no finish point to drain at, so it answers
    /// immediately or refuses.
    queue: Queue,
}

/// Two levels that had to be equal, and what the two sides read back as.
///
/// The terms travel with the equation so that a failure discovered after
/// defaulting can still be reported as the ordinary type mismatch it is when
/// both sides turn out closed — `Type 0` against `Type 1` is two types, and the
/// reader is better served by seeing them than by a sentence about levels.
pub(crate) struct LevelEquation {
    at: Origin,
    expected: Sort,
    found: Sort,
    expected_term: Term,
    found_term: Term,
}

impl LevelEquation {
    /// Read this equation once more, after defaulting has closed every level
    /// nothing determined.
    ///
    /// # Errors
    ///
    /// [`Refusal::Mismatch`] when both sides came out closed and different,
    /// which is the ordinary "these are two types" report; otherwise
    /// [`Refusal::LevelMismatch`], which is the one a level variable earns.
    pub(crate) fn settled(self) -> Result<(), Refusal> {
        if self.expected == self.found {
            return Ok(());
        }
        if self.expected.vars().is_empty() && self.found.vars().is_empty() {
            let close = |term: &Term| term.substitute_levels(&|var: &SortVar| var.solution().cloned());
            return Err(Refusal::Mismatch(Box::new(Mismatch {
                at: self.at,
                expected: close(&self.expected_term),
                found: close(&self.found_term),
                path: Vec::new(),
                whole: None,
            })));
        }
        Err(Refusal::LevelMismatch {
            at: self.at,
            expected: self.expected.forced(),
            found: self.found.forced(),
        })
    }
}

impl Conversion {
    /// A checker that decides §3's conversion instead of solving for it.
    ///
    /// Definitional equality *is* the rigid fragment of the one algorithm —
    /// the same type-directed walk with η and early exit, over values whose
    /// metas are opaque heads rather than unknowns to determine. So
    /// [`crate::convertible`] is this constructor and not a second procedure:
    /// two implementations of one question are two things to keep in
    /// agreement.
    pub(crate) fn deciding() -> Self {
        Self {
            deciding: true,
            levels: Vec::new(),
            queue: Queue::default(),
        }
    }

    /// A checker that may solve a meta it meets alone on one side.
    ///
    /// The elaborator's mode, and the only one that assigns. Named rather than
    /// left to [`Default`] so that the two modes read as a pair at every
    /// construction site.
    pub(crate) fn solving() -> Self {
        Self {
            deciding: false,
            levels: Vec::new(),
            queue: Queue::default(),
        }
    }

    /// The level equations this pass postponed, taken for their one re-reading.
    pub(crate) fn postponed_levels(&mut self) -> Vec<LevelEquation> {
        core::mem::take(&mut self.levels)
    }

    /// The comparisons this pass postponed, for the driver that retries them.
    pub(crate) const fn postponed(&mut self) -> &mut Queue {
        &mut self.queue
    }

    /// Retry one postponed comparison, as though it had just been asked.
    ///
    /// The same entry point the original comparison went through, so a retry
    /// answers by the same rules and charges the same meter (§4's conversion
    /// work): `n` retries cost `n` times, which is what makes a queue that
    /// never settles exhaust rather than spin.
    ///
    /// # Errors
    ///
    /// As [`Self::unify`] and [`Self::unify_types`].
    pub(crate) fn retry(&mut self, meter: &mut Meter, entry: &Postponed) -> Result<(), ElabError> {
        let at = entry.at();
        self.step(meter, entry.depth, at, entry.origin, &entry.left, &entry.right)
            .map_err(|failure| {
                failure
                    .into_error(entry.origin)
                    .rooted(at, meter, entry.depth, &entry.left, &entry.right)
            })
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
        depth: Level,
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
        depth: Level,
        at: Origin,
        ty: &Value,
        left: &Value,
        right: &Value,
    ) -> Result<(), ElabError> {
        self.step(meter, depth, At::Term(ty), at, left, right)
            .map_err(|failure| failure.into_error(at).rooted(At::Term(ty), meter, depth, left, right))
    }

    /// One step: the one flexible case, then the structural descent.
    fn step(
        &mut self,
        meter: &mut Meter,
        depth: Level,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Step {
        // The nesting charge is what keeps conversion inside §4.1's limit, and
        // it can only report a [`CoreError`], so the step's own answer travels
        // back inside its `Ok`.
        meter.nested::<Step, CoreError>("unification", |meter| {
            meter.step("unification")?;
            let unfolded_left = force(meter, left)?;
            let left = unfolded_left.as_ref().unwrap_or(left);
            let unfolded_right = force(meter, right)?;
            let right = unfolded_right.as_ref().unwrap_or(right);
            let flexible = match self.assignment(meter, left, right) {
                Ok(outcome) => outcome,
                Err(failure) => return Ok(Err(failure)),
            };
            Ok(match flexible {
                Outcome::Solved => Ok(()),
                Outcome::Blocked => {
                    self.queue.postpone(Postponed {
                        origin,
                        depth,
                        goal: match at {
                            At::Type => None,
                            At::Term(ty) => Some(ty.clone()),
                        },
                        left: left.clone(),
                        right: right.clone(),
                    });
                    Ok(())
                }
                Outcome::Rigid => self.folded(meter, depth, at, origin, left, right),
            })
        })?
    }

    /// The flexible case: an unsolved unknown at the head of either side — the
    /// step tries both, because "the pattern side" is a direction the caller
    /// picks, not a property of the values.
    ///
    /// The whole rule is [`unify::assign`]'s; what is decided here is only
    /// *whether it may run*. A deciding pass never assigns: §3 is being asked
    /// rather than made true, so an unknown there is an opaque head compared by
    /// identity like any other, and the answer is [`Outcome::Rigid`] whatever
    /// the shapes are.
    ///
    /// Combining the two directions is the obvious lattice and the order it is
    /// written in is the meaning: a side that *solved* settles the pair, and a
    /// side that could not decide blocks it even when the other side was merely
    /// rigid — the pair is undecided, and calling it rigid would send it to a
    /// structural descent that would report a mismatch for a comparison nothing
    /// has answered yet.
    fn assignment(&self, meter: &mut Meter, left: &Value, right: &Value) -> Result<Outcome, Failure> {
        if self.deciding {
            return Ok(Outcome::Rigid);
        }
        let mine = unify::assign(meter, left, right)?;
        if mine == Outcome::Solved {
            return Ok(Outcome::Solved);
        }
        let theirs = unify::assign(meter, right, left)?;
        Ok(match (mine, theirs) {
            (_, Outcome::Solved) | (Outcome::Solved, _) => Outcome::Solved,
            (Outcome::Blocked, _) | (_, Outcome::Blocked) => Outcome::Blocked,
            (Outcome::Rigid, Outcome::Rigid) => Outcome::Rigid,
        })
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
        depth: Level,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Step {
        let open = |meter: &mut Meter, folded: FoldedDef<'_>| -> Result<Value, Failure> {
            Ok(crate::kernel::eval::unfold_spine(
                meter,
                folded.value,
                &folded.neutral.spine,
            )?)
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
        depth: Level,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
    ) -> Step {
        // η first, and read off the *type*, because §3 puts η at Π and at a
        // one-constructor family — so it is the type that decides, whatever
        // forms the two sides happen to have. This is what makes `f` and
        // `λx. f x` agree without either being quoted, and it is why a λ never
        // reaches the match below on a well-typed pair.
        //
        // Two neutrals are the pair η at Π skips. Expanding them adds the same
        // elimination to both spines and then compares the spines, which is the
        // answer [`Self::neutrals`] gives directly — with a message that names
        // the heads that disagreed rather than the expansion.
        if let At::Term(ty) = at {
            let unfolded = opened(meter, ty)?;
            let ty = unfolded.as_ref().unwrap_or(ty);
            let both_blocked = matches!((&left.form, &right.form), (Form::Neutral(_), Form::Neutral(_)));
            match &ty.form {
                Form::Pi { domain, codomain, .. } if !both_blocked => {
                    return self.under_binder(meter, depth, origin, domain, codomain, left, right);
                }
                // A family type is itself a blocked spine, so η at a record
                // lands here rather than beside η at Π — and a constructor
                // application is blocked too, which is why `both_blocked` cannot
                // be the test. [`Self::expanded`] asks the sharper question:
                // whether either side was *built* by the one constructor.
                Form::Neutral(_) => {
                    if let Some(answer) = self.expanded(meter, depth, origin, ty, left, right)? {
                        return answer;
                    }
                }
                Form::Pi { .. }
                | Form::Universe(_)
                | Form::Lam(_)
                // A base type has no η, because η is a rule about a type's
                // eliminations and §5.8 gives it none. Neither of the two below
                // is a type at all; they are here because this match is over
                // every form a forced value can take, not over the well-typed
                // ones.
                | Form::Lit(_)
                | Form::Numeral(_) => {}
            }
        }
        match (&left.form, &right.form) {
            (Form::Universe(one), Form::Universe(other)) => {
                self.universes(meter, depth, at, origin, left, right, one, other)
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
                let variable = Value::var(Origin::UNKNOWN, depth, Arc::clone(left_domain));
                let left_body = apply_closure(meter, left_codomain, variable.clone())?;
                let right_body = apply_closure(meter, right_codomain, variable)?;
                self.step(meter, depth.deeper(), At::Type, origin, &left_body, &right_body)
                    .map_err(|failure| failure.under(PathStep::Codomain))
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
        depth: Level,
        origin: Origin,
        domain: &Arc<Value>,
        codomain: &Closure,
        left: &Value,
        right: &Value,
    ) -> Step {
        let variable = Value::var(Origin::UNKNOWN, depth, Arc::clone(domain));
        let body_type = apply_closure(meter, codomain, variable.clone())?;
        let left_body = apply(meter, left.origin, left.clone(), variable.clone())?;
        let right_body = apply(meter, right.origin, right.clone(), variable)?;
        self.step(
            meter,
            depth.deeper(),
            At::Term(&body_type),
            origin,
            &left_body,
            &right_body,
        )
        .map_err(|failure| failure.under(PathStep::Body))
    }

    /// η at a one-constructor family, when this pair calls for it.
    ///
    /// `Some` with the comparison's answer where the rule fired, `None` where
    /// the two sides are left to [`Self::neutrals`]. The rule fires when the
    /// type is a non-recursive one-constructor family *and* at least one side
    /// is a constructor application: two values blocked at anything else are
    /// compared as spines, so a disagreement names the heads rather than a
    /// field, and two identical variables cost no projections at all.
    fn expanded(
        &mut self,
        meter: &mut Meter,
        depth: Level,
        origin: Origin,
        ty: &Value,
        left: &Value,
        right: &Value,
    ) -> Result<Option<Step>, Failure> {
        let Some(product) = product(meter, ty)? else {
            return Ok(None);
        };
        if !product.expandable() {
            return Ok(None);
        }
        let built = crate::kernel::family::built_by(meter, left)?.is_some()
            || crate::kernel::family::built_by(meter, right)?.is_some();
        if !built {
            return Ok(None);
        }
        Ok(Some(self.field_by_field(meter, depth, origin, &product, left, right)))
    }

    /// Both sides at a one-constructor family: compare their projections, in
    /// declaration order, stopping at the first field that disagrees.
    ///
    /// η at a record, now that a record is a family (`01-surface.md` §1.2).
    /// Unlike η at Π this cannot be read off the two *values*: a constructor
    /// application is a blocked spine, so `f` and `Frame f.span f.held` are both
    /// [`Form::Neutral`] and the head comparison below would call them
    /// different. [`Self::step`] therefore asks for this rule whenever the type
    /// admits it and either side was actually built by the constructor; two
    /// values blocked at something else are left to [`Self::neutrals`], which
    /// answers with the heads that disagreed rather than with a field.
    fn field_by_field(
        &mut self,
        meter: &mut Meter,
        depth: Level,
        origin: Origin,
        product: &Product,
        left: &Value,
        right: &Value,
    ) -> Step {
        let mut reading = product.reading();
        for (position, field) in product.fields.iter().enumerate() {
            let position = u32::try_from(position).unwrap_or(u32::MAX);
            let mine = read_field(meter, product, position, left)?;
            let theirs = read_field(meter, product, position, right)?;
            let field_ty = crate::kernel::eval::eval(meter, &reading, &field.ty)?;
            self.step(meter, depth, At::Term(&field_ty), origin, &mine, &theirs)
                .map_err(|failure| failure.under(PathStep::Field(Arc::clone(&field.name))))?;
            // The telescope proceeds under the *left* subject's field, because
            // it has just been made equal to the right one's.
            reading = reading.push(mine);
        }
        Ok(())
    }

    /// Two blocked eliminations, neither headed by a metavariable: same head,
    /// same shape, equal arguments.
    fn neutrals(
        &mut self,
        meter: &mut Meter,
        depth: Level,
        origin: Origin,
        one: &Arc<Neutral>,
        other: &Arc<Neutral>,
    ) -> Step {
        let heads_agree = match (&one.head, &other.head) {
            (Head::Var(level, _), Head::Var(other_level, _)) => level.0 == other_level.0,
            // Rigid like a variable, and decided the same way: a constant is its
            // name, so there is nothing under it to unify.
            (Head::Const(left, _), Head::Const(right, _)) => left == right,
            // Two metavariables reach this only in [`Self::deciding`] mode,
            // where §3 is being *asked* rather than made true and an unsolved
            // A meta is as rigid as a variable here: the same one is equal
            // to itself, and two different ones are two different unknowns.
            // The matching pass never arrives here with an unsolved one on the
            // left — `assignment` answers first — and a solved one is forced
            // before the walk sees it.
            (Head::Meta(left), Head::Meta(right)) => left == right,
            // Rigid for good: §5.8 gives a base type no eliminator, so nothing
            // under one could ever unblock it, and a builtin still headed here
            // has an argument that is not a literal. Both decide by name, like a
            // constant.
            (Head::Base(left, _), Head::Base(right, _)) => left == right,
            (Head::Builtin(left, _), Head::Builtin(right, _)) => left == right,
            // Reached only at one identity: [`Self::folded`] opens a definition
            // facing anything but itself, so two `Def` heads here are the same
            // definition and the spines decide.
            (Head::Def(one, _, _), Head::Def(other, _, _)) => one == other,
            // Two different kinds of head, which never agree.
            (
                Head::Meta(_)
                | Head::Var(_, _)
                | Head::Const(..)
                | Head::Base(..)
                | Head::Builtin(..)
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
            let (
                Elim::App {
                    argument: left_argument,
                    ..
                },
                Elim::App {
                    argument: right_argument,
                    ..
                },
            ) = (mine, theirs);
            let Form::Pi { domain, .. } = head_type(meter, &prefix)?.form else {
                return Err(blocked_mismatch(meter, depth, one, other)?);
            };
            // The second place `02-core-calculus.md` §3's fifth rule names: two
            // spines that both stayed stuck are being compared, so whatever
            // either holds delayed is evaluated before the comparison. §3's `≡`
            // is what it always was; this is where the rule pays for that.
            let left_argument = &crate::kernel::eval::demanded(meter, left_argument)?;
            let right_argument = &crate::kernel::eval::demanded(meter, right_argument)?;
            self.step(meter, depth, At::Term(&domain), origin, left_argument, right_argument)
                .map_err(|failure| failure.under(PathStep::Argument))?;
            prefix.spine.push(mine.clone());
        }
        Ok(())
    }

    /// The last resort: read both sides back and compare up to α.
    ///
    /// Not a weaker rule — quotation is η-long, so this decides exactly what
    /// [`crate::convertible`] decides. It solves no metavariable, which is why
    /// it is only ever reached once neither side has one at its head.
    /// Two universes, which agree when their levels do.
    ///
    /// §1's hierarchy is non-cumulative, so this is an *equality* on normal
    /// forms and never a search for the smaller — the sentence `sort.rs` states
    /// as "compared with `==`, never with `<`", enforced here because here is
    /// the only place two levels meet.
    ///
    /// A solving pass may do two things beyond comparing. It may **assign**,
    /// where exactly one level makes the equation true — [`Self::assign_level`]
    /// says which shapes those are. Otherwise it **postpones**: `max ?u ?v`
    /// against `3` has several solutions, and choosing one here would decide a
    /// program's meaning by the order its constraints arrived in. The equation
    /// is read once more after defaulting, at the declaration's finish point,
    /// and that is where a level disagreement is reported.
    fn universes(
        &mut self,
        meter: &mut Meter,
        depth: Level,
        at: At<'_>,
        origin: Origin,
        left: &Value,
        right: &Value,
        one: &Sort,
        other: &Sort,
    ) -> Step {
        if one == other {
            return Ok(());
        }
        // Two closed levels that differ are two types, and the report that
        // reads back `Type 0` against `Type 1` says so better than a sentence
        // about universe levels would.
        if one.vars().is_empty() && other.vars().is_empty() {
            return Self::by_reading_back(meter, depth, at, left, right);
        }
        if self.deciding {
            return Err(Failure::Levels {
                expected: one.forced(),
                found: other.forced(),
            });
        }
        if Self::assign_level(one, other) {
            return Ok(());
        }
        self.levels.push(LevelEquation {
            at: origin,
            expected: one.clone(),
            found: other.clone(),
            expected_term: at.quote(meter, depth, left)?,
            found_term: at.quote(meter, depth, right)?,
        });
        Ok(())
    }

    /// Solve one of two levels to the other, when exactly one assignment does
    /// it.
    ///
    /// A variable standing alone on one side and not occurring in the other:
    /// `?u ≡ ℓ` has the single solution `ℓ`. The occurs check is the same one
    /// [`Self::assignment`] makes and refuses for the same reason — `?u ≡ ?u+1`
    /// has no solution at all, and the level algebra has no ω to give it one.
    fn assign_level(one: &Sort, other: &Sort) -> bool {
        for (side, against) in [(one, other), (other, one)] {
            if let Some(var) = side.as_var()
                && !against.vars().contains(&var)
                && var.solve(against.forced()).is_ok()
            {
                return true;
            }
            // `max(k, u+j) ≡ c` for a closed `c` strictly above `k`: the
            // constant cannot reach `c`, so the variable term must, and
            // `u = c - j` is the only level that does. At `c == k` the equation
            // says `u + j ≤ c` instead, which has several answers below `c` and
            // is left to the defaulting rule.
            if let Some((floor, var, plus)) = side.as_single_var()
                && let Some(closed) = against.as_constant()
                && closed > floor
                && closed >= plus
                && var.solve(Sort::constant(closed.saturating_sub(plus))).is_ok()
            {
                return true;
            }
        }
        false
    }

    fn by_reading_back(meter: &mut Meter, depth: Level, at: At<'_>, left: &Value, right: &Value) -> Step {
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
    ///
    /// **A pair that reads back equal is not attached.** A renderer that
    /// preferred the roots would then say "expected `T`, found `T`" over
    /// endpoints that had said exactly what disagreed. The roots earn their
    /// place by naming what was asked; a pair that names the same thing on both
    /// sides names nothing, and the endpoints are left to speak.
    fn rooted(self, at: At<'_>, meter: &mut Meter, depth: Level, left: &Value, right: &Value) -> Self {
        let mut error = self;
        if let Self::Refused(Refusal::Mismatch(mismatch)) = &mut error
            && let (Ok(expected), Ok(found)) = (at.quote(meter, depth, left), at.quote(meter, depth, right))
            && expected != found
        {
            mismatch.whole = Some(Box::new((expected, found)));
        }
        error
    }
}

/// What one step of unification answers.
type Step = Result<(), Failure>;

/// Field `position` of `subject`, as the generated accessor applied to the
/// family's parameters and then to the subject.
///
/// The same application [`crate::kernel::quote`]'s η builds, so a comparison
/// that agrees field by field and a read-back that expands both sides cannot
/// disagree about what a field is.
fn read_field(meter: &mut Meter, product: &Product, position: u32, subject: &Value) -> Result<Value, CoreError> {
    let here = subject.origin;
    let mut read = product.projection(position).value(here, &product.globals);
    for param in &product.params {
        read = apply(meter, here, read, param.clone())?;
    }
    apply(meter, here, read, subject.clone())
}

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
    /// Two universes whose levels no assignment can make equal.
    Levels { expected: Sort, found: Sort },
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
            // A level disagreement is about the two levels and not about where
            // in a type they were reached: reading back `Type u` under six
            // steps of path would name the path and bury the levels.
            Self::Levels { expected, found } => Self::Levels { expected, found },
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
            Self::Levels { expected, found } => Refusal::LevelMismatch { at, expected, found }.into(),
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
    depth: Level,
    one: &Arc<Neutral>,
    other: &Arc<Neutral>,
) -> Result<Failure, CoreError> {
    Ok(Failure::Mismatch {
        expected: quote_type(meter, depth, Mode::Keep, &Value::shared_neutral(one))?,
        found: quote_type(meter, depth, Mode::Keep, &Value::shared_neutral(other))?,
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
    // Only a definition with a value behind it is *folded*. One whose body is a
    // compiled case tree is rigid, so it compares by name and spine exactly as a
    // recursor does, and there is nothing to open it to.
    let Head::Def(identity, _, crate::kernel::value::Folding::Value(carried)) = &neutral.head else {
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
        (DefHead::Local(_), DefHead::Global(..)) => true,
        (DefHead::Global(..), DefHead::Local(_)) => false,
        (DefHead::Local(this), DefHead::Local(that)) => this.0 > that.0,
        (DefHead::Global(this, _), DefHead::Global(that, _)) => this.name() > that.name(),
    }
}
