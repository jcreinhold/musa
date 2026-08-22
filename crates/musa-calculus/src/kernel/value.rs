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

use std::sync::Arc;

use crate::kernel::context::Globals;
use crate::kernel::list::List;
use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Field, Filling, Level, Name, Term};

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
    globals: Globals,
}

impl Env {
    /// The environment with no binders and no names.
    pub(crate) const EMPTY: Self = Self {
        locals: List::EMPTY,
        globals: Globals::EMPTY,
    };

    /// The environment with no binders, reading its names in `globals`.
    pub(crate) const fn under(globals: Globals) -> Self {
        Self {
            locals: List::EMPTY,
            globals,
        }
    }

    /// This environment with one more binder, innermost.
    pub(crate) fn push(&self, value: Value) -> Self {
        Self {
            locals: self.locals.push(value),
            globals: self.globals.clone(),
        }
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

    /// The same binders, read under a different table.
    pub(crate) fn reading(&self, globals: Globals) -> Self {
        Self {
            locals: self.locals.clone(),
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

/// A record type as a telescope: the fields in order, read in one environment,
/// where field `i`'s type may mention fields `0..i`.
#[derive(Clone)]
pub(crate) struct Telescope {
    pub(crate) fields: Arc<[Field]>,
    pub(crate) env: Env,
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
    RecordType(Telescope),
    Record(Arc<[(Name, Value)]>),
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
    /// `T(i)` — a type carrying an index (§1.5).
    ///
    /// Neither canonical nor neutral in the usual sense, and it does not need
    /// to be either: nothing eliminates an indexed type, so it never blocks a
    /// spine and never reduces. It exists in the semantic domain for one
    /// reason — [`crate::elaboration::convert`] compares *values*, so an indexed type conversion
    /// has to survive evaluation to be asked about — and [`crate::kernel::quote`] drops
    /// it on the way back out, which is where §1.5's erasure lives.
    Indexed {
        /// The type being refined.
        ty: Arc<Value>,
        /// The index it is refined by.
        index: Arc<Value>,
    },
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
    Def(DefHead, Arc<Value>, Arc<Value>),
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
    /// A top-level definition, named by the declaration itself.
    Global(crate::kernel::program::Def),
}

/// One elimination applied to a blocked head.
#[derive(Clone)]
pub(crate) enum Elim {
    App { origin: Origin, argument: Arc<Value> },
    Project { origin: Origin, field: Name },
}

impl Elim {
    /// Where this elimination was written.
    pub(crate) const fn origin(&self) -> Origin {
        match self {
            Self::App { origin, .. } | Self::Project { origin, .. } => *origin,
        }
    }
}

impl Neutral {
    /// A bare head with nothing applied to it.
    pub(crate) const fn head(origin: Origin, head: Head) -> Self {
        Self {
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

impl Value {
    /// A value of form `form`, from a term that came from `origin`.
    pub(crate) const fn new(origin: Origin, form: Form) -> Self {
        Self { origin, form }
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
