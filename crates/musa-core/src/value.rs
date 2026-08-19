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
//! must be able to say what type it has. [`crate::eval::neutral_type`] is that
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

use crate::level::Level;
use crate::list::List;
use crate::meta::Meta;
use crate::origin::Origin;
use crate::term::{DbLevel, Field, Name, Plicity, Term};

/// An immutable environment: the values of the binders in scope, innermost
/// first.
///
/// An alias rather than a newtype: an environment is a [`List`] and nothing
/// about it is more specific than that, so a wrapper here would be a type that
/// only forwards.
pub(crate) type Env = List<Value>;

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
    Universe(Level),
    Pi {
        /// Carried so that elaboration can read it off a *type it computed*
        /// rather than off the syntax it was written as — the point of §1's
        /// amendment. No operation in this crate branches on it.
        plicity: Plicity,
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
    Id {
        ty: Arc<Value>,
        left: Arc<Value>,
        right: Arc<Value>,
    },
    Refl(Arc<Value>),
    /// A closed value of a base type. Canonical, not neutral: a `Text` is a
    /// value the way `refl x` is a value of `Id A x x`, and §5.8's inertness is
    /// that nothing eliminates it rather than that it is stuck.
    Lit(crate::base::Literal),
    /// A closed value of a counting family, as how far above the floor it
    /// stands.
    ///
    /// Canonical, and the *only* canonical form at such a family:
    /// [`crate::eval::apply`] collapses the step constructor applied to one of
    /// these into one of these, and [`crate::eval::eval`] turns the floor into a
    /// zero, so no value at a counting family ever holds a constructor spine.
    /// That is what makes conversion here a [`u64`] comparison instead of a walk
    /// whose cost is the number the author wrote.
    ///
    /// A step applied to a *neutral* is still an ordinary neutral spine and
    /// needs no case of its own: a neutral is not a closed value, so it is not
    /// this, and the two are unequal for the reason any two different normal
    /// forms are.
    Numeral(crate::family::Numeral),
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
    Var(DbLevel, Arc<Value>),
    /// A declared constant. Rigid, like a variable: a family and a constructor
    /// never compute, and a recursor computes only when ι fires — which
    /// [`crate::eval::apply`] does at the moment the target becomes a
    /// constructor, so a spine that is still headed by one here is genuinely
    /// blocked.
    ///
    /// It carries no type, unlike [`Self::Var`], because a constant's type is
    /// determined by its declaration and [`crate::family::Constant`] holds that.
    Const(crate::family::Constant),
    /// A base type, registered by the host. Rigid forever: §5.8 gives it no
    /// eliminator, so a spine headed by one is blocked with nothing that could
    /// unblock it. It is a head rather than a [`Form`] because a base type may
    /// take parameters — `Syntax Expr` is `Syntax` applied — and an applied
    /// canonical form would need an arm that says what applying it means.
    Base(crate::base::Base),
    /// A compiler-owned operation. Rigid until every argument is a literal:
    /// [`crate::eval::apply`] runs the δ-rule at that moment, in the same arm
    /// that fires ι for a recursor, so a spine still headed by one here is
    /// genuinely blocked.
    Builtin(crate::base::Builtin),
    /// An unsolved metavariable. The one *flexible* head: a neutral headed by a
    /// variable can never compute, while this one computes the moment the meta
    /// is solved, which is exactly the distinction unification turns on.
    Meta(Meta),
    /// A definition, held folded: what it is known as, its type, and the value
    /// it unfolds to.
    ///
    /// **Flexible-rigid.** It never blocks like a variable and never solves
    /// like a meta — unification treats it as rigid — but it computes on
    /// demand, so conversion treats it as reducible. That pairing is why it is
    /// one head rather than a second [`Form`].
    ///
    /// The type travels beside the value for [`Head::Var`]'s reason:
    /// [`crate::eval::neutral_type`] answers a head's type with no context to
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
    Local(DbLevel),
    /// A top-level definition, named by the declaration itself.
    Global(crate::program::Def),
}

/// One elimination applied to a blocked head.
#[derive(Clone)]
pub(crate) enum Elim {
    App {
        origin: Origin,
        argument: Arc<Value>,
    },
    Project {
        origin: Origin,
        field: Name,
    },
    /// `J` blocked on a proof that is not `refl`. The proof is what the spine
    /// leads to, so it is not stored here.
    J {
        origin: Origin,
        ty: Arc<Value>,
        from: Arc<Value>,
        motive: Arc<Value>,
        base: Arc<Value>,
        to: Arc<Value>,
    },
}

impl Elim {
    /// Where this elimination was written.
    pub(crate) const fn origin(&self) -> Origin {
        match self {
            Self::App { origin, .. } | Self::Project { origin, .. } | Self::J { origin, .. } => *origin,
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
    /// has one: eliminating a blocked value is what [`crate::eval::apply`],
    /// [`crate::eval::project`], and [`crate::eval::jay`] each do to a value
    /// they were handed.
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
    pub(crate) fn var(origin: Origin, level: DbLevel, ty: Arc<Self>) -> Self {
        Self::neutral(Neutral::head(origin, Head::Var(level, ty)))
    }
}
