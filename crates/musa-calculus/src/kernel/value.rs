//! The semantic domain, and why it is private.
//!
//! `docs/rules/language/02-core-calculus.md` §3 decides definitional equality
//! by normalization by evaluation: both sides are evaluated into values with
//! closures and neutrals, and quoted back to a normal form. **Reduction is
//! never performed on syntax**, which is why there is no substitution function
//! anywhere in this crate.
//!
//! [`Value`] never leaves. A caller asks whether two terms are the same and
//! gets an answer, or asks for a normal form and gets a [`Term`]; it never
//! learns what a closure is. `docs/plan/roadmap.md` §15.12 fixes that boundary
//! and prompt 133 states the argument, because the pressure to leak it arrives
//! with the first caller that wants to inspect a normal form:
//!
//! > Prompt 134's elaborator wants to check against a *value* type rather than
//! > re-normalizing at every step, and exposing `Value` would let it. The
//! > answer is that the elaborator then belongs in this crate, not that `Value`
//! > belongs in the facade.
//!
//! Concretely: closures hold the evaluator's own environment representation, so
//! every later change to evaluation would be a breaking change for
//! `musa-compiler`. Prompt 134 adds check and infer *inside* this crate over a
//! surface-independent raw term, and `musa-compiler` keeps only the translation
//! into that raw term.
//!
//! **A neutral variable carries its type.** That is not decoration. Quotation
//! is type-directed — it is what performs η at Π and at records (§3) — so
//! quoting the *argument* of a blocked application needs that argument's type,
//! which means the type of the function it is applied to, which means a neutral
//! must be able to say what type it has. [`crate::kernel::eval::neutral_type`] is that
//! function and the type on the variable is its base case. Without it, `f g`
//! and `f (λx. g x)` would quote to different terms and conversion would answer
//! `false` for two terms §3 says are equal.
//!
//! **A value carries an origin, because quotation has to give one back.** §7
//! says the normal form of a term carries the origins of the terms it was built
//! from. A normal form is written by `quote` out of a value, so a value that had
//! dropped its origin could not put one back. Values and neutrals therefore have
//! the same shape as [`Term`] does — provenance in a wrapper, everything else in
//! a [`Form`], a [`Head`], or an [`Elim`].

use std::sync::{Arc, OnceLock};

use crate::kernel::budget::Stamp;
use crate::kernel::context::Globals;
use crate::kernel::list::List;
use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Binder, Filling, Level, Name, Shape, Term};

/// An immutable environment: the values of the binders in scope, innermost
/// first, and the names in scope that no binder introduced.
///
/// A pair rather than the bare [`List`] it used to be, because a term names two
/// kinds of thing (`02-core-calculus.md` §1) and both have to be resolvable
/// where reduction happens. The locals answer [`Term::var`](crate::Term::var);
/// [`Globals`] answers [`Shape::Named`](crate::Shape::Named), and rides here
/// rather than in a parameter so that a closure opens under the table it was
/// *built* under — see [`Globals`].
#[derive(Clone)]
pub(crate) struct Env {
    locals: List<Value>,
    /// How many binders [`Self::locals`] holds.
    ///
    /// Carried rather than counted because [`crate::kernel::eval::eval`] asks
    /// for it at every metavariable it meets: an unknown's occurrence names its
    /// scope by *level* (`kernel::meta`), and a level is only an index once the
    /// depth is known.
    depth: u32,
    globals: Globals,
}

impl Env {
    /// The environment with no binders and no names.
    pub(crate) const EMPTY: Self = Self {
        locals: List::EMPTY,
        depth: 0,
        globals: Globals::EMPTY,
    };

    /// The environment with no binders, reading its names in `globals`.
    pub(crate) const fn under(globals: Globals) -> Self {
        Self {
            locals: List::EMPTY,
            depth: 0,
            globals,
        }
    }

    /// This environment with one more binder, innermost.
    pub(crate) fn push(&self, value: Value) -> Self {
        Self {
            locals: self.locals.push(value),
            depth: self.depth.saturating_add(1),
            globals: self.globals.clone(),
        }
    }

    /// How many binders are in scope: the depth every [`Level`] here counts
    /// from.
    pub(crate) const fn depth(&self) -> Level {
        Level(self.depth)
    }

    /// The value of the binder `index` steps out, if there is one.
    pub(crate) fn get(&self, index: u32) -> Option<&Value> {
        self.locals.get(index)
    }

    /// The binders in scope, innermost first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Value> {
        self.locals.iter()
    }

    /// The names in scope here that no binder introduced.
    pub(crate) const fn globals(&self) -> &Globals {
        &self.globals
    }

    /// The same binders, with the one at `level` standing for `value` instead.
    ///
    /// **Substitution, done where §1 admits it.** The core has no substitution
    /// function on terms, so the elaborator abstracts a subject out of a goal by
    /// re-evaluating the goal in an environment where the subject's binder holds
    /// something else — which is what substitution *is* in a levelled semantic
    /// domain. `crate::elaboration::case` uses it once per split, to build the
    /// dependent motive §1.1 asks for.
    ///
    /// A level this environment does not reach leaves it unchanged: an
    /// environment that never bound the variable has nothing to rebind.
    pub(crate) fn rebinding(&self, level: Level, value: Value) -> Self {
        let Some(index) = level.to_index(self.depth()) else {
            return self.clone();
        };
        // Innermost first, matching `locals`, and rebuilt outermost first
        // because [`Self::push`] is the only way to extend a [`List`].
        let mut held: Vec<Value> = self.locals.iter().cloned().collect();
        let Some(slot) = held.get_mut(usize::try_from(index.0).unwrap_or(usize::MAX)) else {
            return self.clone();
        };
        *slot = value;
        held.into_iter()
            .rev()
            .fold(Self::under(self.globals.clone()), |env, held| env.push(held))
    }

    /// The same binders, read under a different table.
    pub(crate) fn reading(&self, globals: Globals) -> Self {
        Self {
            locals: self.locals.clone(),
            depth: self.depth,
            globals,
        }
    }
}

/// A term paired with the environment its free variables are read in.
#[derive(Clone)]
pub(crate) struct Closure {
    pub(crate) env: Env,
    pub(crate) body: Term,
}

impl Closure {
    /// Whether opening this closure can observe what it is opened at.
    ///
    /// A Π's codomain that never mentions its own binder is the ordinary
    /// non-dependent arrow written in the dependent syntax, and `A → B` says
    /// nothing about the `A` it was handed. So a caller that has a *term* for
    /// the argument and needs only the codomain may skip evaluating it: no
    /// value it could have computed is reachable from the body.
    ///
    /// Conservative at a metavariable, and that is the one subtle arm. §2.1
    /// writes an unknown `?m[σ]` — the spine is read out of the environment
    /// rather than carried in the term ([`crate::kernel::eval`]'s `Shape::Meta`
    /// arm says why) — so a body that is, or contains, an unsolved unknown may
    /// read *every* binder in scope including this one, whatever its written
    /// nodes mention. Answering `true` there costs an evaluation that might
    /// not have been needed; answering `false` would drop a binder an
    /// occurrence names.
    pub(crate) fn reads_its_binder(&self) -> bool {
        reads(&self.body, 0)
    }
}

/// Whether `term` mentions the variable bound `depth` binders further out.
fn reads(term: &Term, depth: u32) -> bool {
    match term.shape() {
        Shape::Var(index) => index.0 == depth,
        // See [`Closure::reads_its_binder`]: an occurrence names its scope by
        // level and the scope is not in the term.
        Shape::Meta(_) => true,
        Shape::Named { .. } | Shape::Lit(_) | Shape::Universe(_) => false,
        Shape::Bind { binder, body, .. } => {
            let carried = match binder {
                Binder::Pi { ty, .. } => reads(ty, depth),
                Binder::Let { ty, value } => reads(ty, depth) || reads(value, depth),
                Binder::Lam => false,
            };
            carried || reads(body, depth.saturating_add(1))
        }
        Shape::App { function, argument } => reads(function, depth) || reads(argument, depth),
    }
}

/// A semantic value: what it is, and where the term that produced it came from.
#[derive(Clone)]
pub(crate) struct Value {
    pub(crate) origin: Origin,
    pub(crate) form: Form,
}

/// What a value is.
#[derive(Clone)]
pub(crate) enum Form {
    Universe(Sort),
    Pi {
        /// Carried so that elaboration can read it off a *type it computed*
        /// rather than off the syntax it was written as — the point of §1's
        /// amendment. No operation in this crate branches on it.
        filling: Filling,
        name: Name,
        domain: Arc<Value>,
        codomain: Closure,
    },
    /// A lambda, and no name: quotation writes a binder's name from the Π it
    /// is quoting at, never from the λ, so a name here would be a second copy
    /// free to disagree with the one that gets printed.
    Lam(Closure),
    /// A closed value of a base type. Canonical, not neutral: §5.8's
    /// inertness is that nothing eliminates it rather than that it is stuck.
    Lit(crate::kernel::base::Literal),
    /// A closed value of a counting family, as how far above the floor it
    /// stands.
    ///
    /// Canonical, and the *only* canonical form at such a family:
    /// [`crate::kernel::eval::apply`] collapses the step constructor applied to one of
    /// these into one of these, and [`crate::kernel::eval::eval`] turns the floor into a
    /// zero, so no value at a counting family ever holds a constructor spine.
    /// That is what makes conversion here a [`u64`] comparison instead of a walk
    /// whose cost is the number the author wrote.
    ///
    /// A step applied to a *neutral* is still an ordinary neutral spine and
    /// needs no case of its own: a neutral is not a closed value, so it is not
    /// this, and the two are unequal for the reason any two different normal
    /// forms are.
    Numeral(crate::kernel::family::Numeral),
    Neutral(Arc<Neutral>),
}

/// An elimination blocked on its head: what it is blocked on, and what has been
/// applied to it since.
///
/// **A head and a vector, not a chain.** The obvious encoding — one node per
/// elimination, each holding the neutral it eliminates — puts the head at the
/// *deepest* position, so finding it is one hop per argument. Unification asks
/// for the head of both sides at every step and forcing asks again before that,
/// so the question that is asked most often was the one that cost the most.
/// Here the head is a field: `f x y z` is one allocation with a three-element
/// spine, a length mismatch is decided before any argument is compared, and
/// [`Head::Var`]'s type is stored once instead of once per node. Note 44 §9 is
/// the audit that named this, and smalltt is where the shape comes from.
///
/// **Each elimination still carries its own origin.** §7 wants every node of a
/// quoted normal form to say where it came from, and the spine of `f x y` reads
/// back as three terms — a fact about quotation, not about how many allocations
/// the value needs. [`Self::origin`] is the head's; the value's is the outermost
/// elimination's, which [`Self::outer_origin`] answers.
#[derive(Clone)]
pub(crate) struct Neutral {
    /// Where the *head* was written.
    pub(crate) origin: Origin,
    pub(crate) head: Head,
    /// What has been applied to the head, innermost first.
    pub(crate) spine: Vec<Elim>,
    /// This neutral's δ-unfolding, once something has asked for it.
    ///
    /// Peyton Jones ch. 12 §12.4's update: what makes shared work happen once
    /// is that the root of a shared redex is overwritten with its result. A
    /// folded definition's unfolding is a function of the definition's value
    /// and this spine, both of which are immutable, so the *first* consumer to
    /// force this value may record the answer for every later one — and every
    /// later one shares it, because a [`Value`] is `Arc`-shared and a clone of
    /// this neutral clones the handle rather than the cell.
    ///
    /// `None` where there is nothing to record: a variable, a constant, a base
    /// type, a builtin, a metavariable, a compiled case tree. Only
    /// [`Folding::Value`] behind a [`Head::Def`] unfolds by replaying a spine,
    /// and only those neutrals pay for the cell.
    ///
    /// The stamp is [`Meter::stamp`](crate::kernel::budget::Meter::stamp) at
    /// the moment the cell was filled, and a hit is a hit only at the same
    /// stamp — see [`Stamp`](crate::kernel::budget::Stamp) for why. `Value` is
    /// not `PartialEq` and this field takes no part in identity: two neutrals
    /// are the same neutral when their head and spine agree, whatever either
    /// has been asked for.
    pub(crate) unfolded: Option<Arc<OnceLock<(Stamp, Value)>>>,
}

/// What a blocked elimination is blocked on.
#[derive(Clone)]
pub(crate) enum Head {
    /// A variable, with the type it was assumed at.
    Var(Level, Arc<Value>),
    /// A declared constant. Rigid, like a variable: a family and a constructor
    /// never compute, and a recursor computes only when ι fires — which
    /// [`crate::kernel::eval::apply`] does at the moment the target becomes a
    /// constructor, so a spine that is still headed by one here is genuinely
    /// blocked.
    ///
    /// It carries no type, unlike [`Self::Var`], because a constant's type is
    /// determined by its declaration and [`crate::kernel::family::Constant`] holds that.
    /// It does carry the table it was resolved under, for the reason spelled at
    /// [`Self::Base`].
    Const(crate::kernel::family::Constant, Globals),
    /// A base type, registered by the host. Rigid forever: §5.8 gives it no
    /// eliminator, so a spine headed by one is blocked with nothing that could
    /// unblock it. It is a head rather than a [`Form`] because a base type may
    /// take parameters — `Syntax Expr` is `Syntax` applied — and an applied
    /// canonical form would need an arm that says what applying it means.
    ///
    /// **It carries the table its own kind is written in.**
    /// [`crate::kernel::eval::neutral_type`] answers a head's type with no context to
    /// ask, and answers this one by *evaluating* the kind the host registered —
    /// a closed term in locals, but not in names. So the table travels here,
    /// exactly as the type travels on [`Self::Var`] and [`Self::Def`]. It is not
    /// part of the head's identity and [`crate::elaboration::convert`] does not compare it.
    Base(crate::kernel::base::Base, Globals),
    /// A compiler-owned operation. Rigid until every argument is a literal:
    /// [`crate::kernel::eval::apply`] runs the δ-rule at that moment, in the same arm
    /// that fires ι for a recursor, so a spine still headed by one here is
    /// genuinely blocked. It carries the table its signature is written in, for
    /// the reason spelled at [`Self::Base`].
    Builtin(crate::kernel::base::Builtin, Globals),
    /// An unsolved placeholder for an unwritten argument. The one *flexible*
    /// head: a neutral headed by a variable can never compute, while this one
    /// computes the moment the meta is solved — which is exactly the
    /// distinction the matching pass turns on. See [`crate::kernel::meta::Meta`].
    Meta(crate::kernel::meta::Meta),
    /// A definition, held folded: what it is known as, its type, and the value
    /// it unfolds to.
    ///
    /// **Flexible-rigid.** It never blocks like a variable and never solves
    /// like a meta — unification treats it as rigid — but it computes on
    /// demand, so conversion treats it as reducible. That pairing is why it is
    /// one head rather than a second [`Form`].
    ///
    /// The type travels beside the value for [`Head::Var`]'s reason:
    /// [`crate::kernel::eval::neutral_type`] answers a head's type with no context to
    /// ask. The value travels so that δ stays a *local* rule — `eval` takes an
    /// `&Env` and not a `Cx`, and a head that had to consult a context to
    /// unfold would make δ a lookup the evaluator cannot perform where it
    /// needs to.
    ///
    /// **Flexible-rigid is the *body's* property, not the head's.** A definition
    /// whose body is a compiled case tree does not unfold at all; it reduces
    /// when its arguments arrive, exactly as a recursor does. [`Folding`] is
    /// which of the two this one is.
    Def(DefHead, Arc<Value>, Folding),
}

/// What a folded definition can be unfolded *to*.
///
/// A definition whose body was evaluated at its declaration carries the value,
/// and δ replays the spine over it. A definition whose body is a compiled case
/// tree (§1) carries the tree instead and is **rigid**: it does not unfold, it
/// *reduces*, when [`crate::kernel::eval::apply`] sees its arguments arrive and
/// the scrutinee of its first split become canonical — the same discipline a
/// recursor is under, in the same arm.
///
/// The table travels with the tree for [`Head::Base`]'s reason: the tree holds
/// terms, and evaluating one needs the names it was read in.
#[derive(Clone)]
pub(crate) enum Folding {
    /// Evaluated at the declaration; δ replays the spine over it.
    Value(Arc<Value>),
    /// A compiled case tree, and the table its terms are read in.
    Compiled(Arc<crate::kernel::case_tree::Compiled>, Globals),
    /// A definition in scope during its own elaboration. Rigid, with nothing
    /// behind it yet — see [`Body::Pending`](crate::kernel::program::Body).
    Pending,
}

/// What a folded definition is known as.
///
/// One notion of identity with two disjoint constructors — disjoint is the
/// soundness property: a binder level and a program position are two
/// numberings that both start at zero, so a single constructor for both would
/// equate a `let` with whichever top-level definition happened to share its
/// number, and one such program plus one `let` is a constructible wrong
/// answer. Two constructors of one enum cannot disagree with each other.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum DefHead {
    /// A `let` or context definition, named by the binder's level.
    Local(Level),
    /// A top-level definition, named by the declaration itself **and by the
    /// levels it was instantiated at**.
    ///
    /// Both halves are identity. `id.{0}` and `id.{1}` unfold to two different
    /// values, so a comparison that stopped at the name would call two folded
    /// uses of one polymorphic definition equal without ever unfolding either
    /// — which is the one way non-cumulativity could be lost by accident (§1).
    Global(crate::kernel::program::Def, crate::kernel::sort::Levels),
}

/// One elimination applied to a blocked head.
#[derive(Clone)]
pub(crate) enum Elim {
    App { origin: Origin, argument: Arg },
}

/// An argument on a blocked spine, evaluated or not yet.
///
/// `02-core-calculus.md` §3's fifth strategy rule is what this enum exists for:
/// a recursor is strict in its target and lazy in its methods, so the argument
/// standing at a method position is carried as the term it was written as until
/// ι asks for it. Every other argument in the calculus is [`Self::Ready`]
/// before it ever reaches a spine, because [`crate::kernel::eval`] evaluates it
/// on the way in.
///
/// **A delayed argument is not a [`Value`], deliberately.** The rule is that it
/// is looked at in exactly two places — ι selecting it, and a spine that stayed
/// stuck being read back or compared — and a variant of [`Form`] would make
/// every match on a value a place where the third could be written by accident.
/// A separate type makes the two places the only ones that compile.
#[derive(Clone)]
pub(crate) enum Arg {
    /// Evaluated, as every argument outside a recursor's methods is.
    Ready(Arc<Value>),
    /// Written but not evaluated; shared, so that forcing it happens once.
    Delayed(Arc<Delay>),
}

/// A term held with the environment it is read in, and the value it forced to.
///
/// The memo is the difference between this being a saving and a loss. A
/// recursor under a fold meets its methods once per turn, and a delay that
/// re-evaluated its term at every turn would spend more than the strict
/// machine did on exactly the programs §3's fifth rule is for.
pub(crate) struct Delay {
    env: Env,
    term: Term,
    forced: OnceLock<Value>,
}

impl Delay {
    /// The environment and the term, for the one caller that evaluates them.
    pub(crate) const fn parts(&self) -> (&Env, &Term) {
        (&self.env, &self.term)
    }

    /// The value this forced to, if something already forced it.
    pub(crate) fn settled(&self) -> Option<&Value> {
        self.forced.get()
    }

    /// Record what this forced to, if nothing recorded it first.
    ///
    /// A second writer is not an error: two threads may force one delay, and
    /// totality (§2.4) says they computed the same value. The first one wins
    /// and the second's is dropped.
    pub(crate) fn fill(&self, value: Value) {
        drop(self.forced.set(value));
    }
}

impl Arg {
    /// An argument already evaluated.
    pub(crate) fn ready(value: Value) -> Self {
        Self::Ready(Arc::new(value))
    }

    /// An argument to be evaluated when something asks.
    pub(crate) fn delayed(env: Env, term: Term) -> Self {
        Self::Delayed(Arc::new(Delay {
            env,
            term,
            forced: OnceLock::new(),
        }))
    }

    /// The value, where one is already there to be had.
    ///
    /// [`None`] means a delay nothing has forced yet — not that there is no
    /// value, which is why the callers that cannot evaluate treat it as "leave
    /// this spine alone" rather than as an answer.
    pub(crate) fn settled(&self) -> Option<&Value> {
        match *self {
            Self::Ready(ref value) => Some(value),
            Self::Delayed(ref delay) => delay.forced.get(),
        }
    }
}

impl Elim {
    /// Where this elimination was written.
    pub(crate) const fn origin(&self) -> Origin {
        match self {
            Self::App { origin, .. } => *origin,
        }
    }
}

/// A fresh unfolding cell, for the one head shape that unfolds.
///
/// Allocating one per neutral would put an `Arc` behind every variable
/// occurrence; allocating one per *foldable* head puts it behind exactly the
/// neutrals [`crate::kernel::eval::unfold`] can answer about.
fn memo_for(head: &Head) -> Option<Arc<OnceLock<(Stamp, Value)>>> {
    matches!(*head, Head::Def(_, _, Folding::Value(_))).then(|| Arc::new(OnceLock::new()))
}

impl Neutral {
    /// A bare head with nothing applied to it.
    pub(crate) fn head(origin: Origin, head: Head) -> Self {
        Self {
            unfolded: memo_for(&head),
            origin,
            head,
            spine: Vec::new(),
        }
    }

    /// This neutral with one more elimination on the end.
    ///
    /// Takes the shared neutral rather than an owned one because every caller
    /// has one: eliminating a blocked value is what [`crate::kernel::eval::apply`] and
    /// [`crate::kernel::eval::project`] each do to a value they were handed.
    pub(crate) fn eliminated(neutral: &Self, elimination: Elim) -> Self {
        let mut spine = Vec::with_capacity(neutral.spine.len().saturating_add(1));
        spine.extend(neutral.spine.iter().cloned());
        spine.push(elimination);
        Self {
            unfolded: memo_for(&neutral.head),
            origin: neutral.origin,
            head: neutral.head.clone(),
            spine,
        }
    }

    /// Where the whole elimination was written: the outermost one, or the head
    /// when nothing has been applied.
    pub(crate) fn outer_origin(&self) -> Origin {
        self.spine.last().map_or(self.origin, Elim::origin)
    }
}

/// Reclaiming a value without one host frame per level of it.
///
/// A value is a tree of [`Arc`]s and the derived destructor walks it with the
/// host's stack: freeing `Cons 0 (Cons 1 (…))` frees the outer neutral, which
/// frees its spine, which frees the argument, which frees the next neutral. So
/// a list of a few thousand elements is a few thousand host frames spent on
/// *freeing* it — measured before this impl existed, `range(4000)` compiled and
/// `range(4500)` aborted the process in `drop_glue<Value>` with no musa frame on
/// the stack at all.
///
/// That is the same defect as an evaluator written on the host's stack, arriving
/// at the other end of a value's life, and §4.1 forbids the outcome just as
/// firmly at this end: a total language may refuse but may not crash, and a
/// refusal this crate lifted must not be paid for with an abort. Peyton Jones
/// ch. 17 §17.2 is the analysis — reclamation is a traversal, and a traversal of
/// a deep structure may not be written as host calls — and the repair is the one
/// this whole prompt is about: the pending work becomes a `Vec` this function
/// owns.
///
/// **This is not a collector and changes nothing about ownership.** Every
/// allocation is still freed by `Arc` at the moment its last handle goes, in the
/// same order. What the loop does is take a child *out* of the node before the
/// node is dropped, so the node's own destructor finds nothing left to descend
/// into. A child that is still shared is left alone: [`Arc::into_inner`] answers
/// `None`, and everything beneath it is still reachable from somewhere else.
///
/// **The chain that had to be flattened is the spine.** A neutral's arguments —
/// and its unfolding memo, which holds a value of the same depth — are what a
/// structurally recursive definition builds thousands of levels of. A head's
/// types and a closure's environment are freed by their own destructors, one
/// frame down, because their depth is the depth of a *type* or of a binder
/// context, which is charged nesting and so is bounded by §4.1 already.
impl Drop for Neutral {
    fn drop(&mut self) {
        let spine = std::mem::take(&mut self.spine);
        let unfolded = self.unfolded.take();
        if spine.is_empty() && unfolded.is_none() {
            return;
        }
        let mut work = Vec::new();
        loosen(&mut work, spine, unfolded);
        while let Some(value) = work.pop() {
            match value.form {
                Form::Neutral(shared) => {
                    if let Some(mut inner) = Arc::into_inner(shared) {
                        let spine = std::mem::take(&mut inner.spine);
                        let unfolded = inner.unfolded.take();
                        loosen(&mut work, spine, unfolded);
                    }
                }
                Form::Pi { domain, .. } => {
                    if let Some(inner) = Arc::into_inner(domain) {
                        work.push(inner);
                    }
                }
                Form::Universe(_) | Form::Lam(_) | Form::Lit(_) | Form::Numeral(_) => {}
            }
        }
    }
}

/// The values a neutral owned alone, moved onto the worklist.
///
/// A value still shared elsewhere is not ours to dismantle and is dropped here
/// as a handle, which is what [`Arc::into_inner`] answering `None` means.
fn loosen(work: &mut Vec<Value>, spine: Vec<Elim>, unfolded: Option<Arc<OnceLock<(Stamp, Value)>>>) {
    for elimination in spine {
        let Elim::App { argument, .. } = elimination;
        match argument {
            Arg::Ready(value) => {
                if let Some(value) = Arc::into_inner(value) {
                    work.push(value);
                }
            }
            // A delay owns an environment as well as a memo, and an
            // environment is a shared list whose own destructor is one frame
            // deep. What has this neutral's depth is the value it forced to,
            // so that is what comes onto the worklist.
            Arg::Delayed(delay) => {
                if let Some(delay) = Arc::into_inner(delay)
                    && let Some(value) = delay.forced.into_inner()
                {
                    work.push(value);
                }
            }
        }
    }
    if let Some(cell) = unfolded
        && let Some(lock) = Arc::into_inner(cell)
        && let Some((_, value)) = lock.into_inner()
    {
        work.push(value);
    }
}

impl Value {
    /// A value of form `form`, from a term that came from `origin`.
    pub(crate) const fn new(origin: Origin, form: Form) -> Self {
        Self { origin, form }
    }

    /// What stands at a binder nothing reads.
    ///
    /// [`Closure::reads_its_binder`] decides when this is admissible and
    /// [`crate::kernel::eval::apply_closure_read`] is the only caller. The form is
    /// `Type 0` because the environment has to hold *something* and this is the
    /// cheapest thing to build; which form it is cannot matter, because the
    /// body that would observe it does not mention the binder it stands at.
    pub(crate) const fn unread() -> Self {
        Self::new(Origin::UNKNOWN, Form::Universe(Sort::ZERO))
    }

    /// A blocked elimination, as a value.
    ///
    /// The origin comes from the neutral rather than from a second argument:
    /// the two would be the same fact stored twice, and the copies would be free
    /// to disagree.
    pub(crate) fn neutral(neutral: Neutral) -> Self {
        Self::new(neutral.outer_origin(), Form::Neutral(Arc::new(neutral)))
    }

    /// An already-shared blocked elimination, as a value.
    pub(crate) fn shared_neutral(neutral: &Arc<Neutral>) -> Self {
        Self::new(neutral.outer_origin(), Form::Neutral(Arc::clone(neutral)))
    }

    /// A fresh variable at `level`, assumed at `ty`.
    pub(crate) fn var(origin: Origin, level: Level, ty: Arc<Self>) -> Self {
        Self::neutral(Neutral::head(origin, Head::Var(level, ty)))
    }
}
