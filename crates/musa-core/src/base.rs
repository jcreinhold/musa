//! Base types, literals, and compiler-owned builtins.
//!
//! `docs/rules/language/02-core-calculus.md` §5.8 admits a **conservative
//! extension** to §1's calculus: a base type with no eliminator, together with a
//! finite set of builtins over the extended base set. §1's grammar is the pure
//! core and says nothing about either, because neither is part of it — they are
//! what the extension adds, and §5.8 re-derives §5's matrix over the result.
//!
//! **The core owns the mechanism; the host owns the table.** Nothing in this
//! crate knows what a pitch is, and nothing may learn. §5.8's corollary is the
//! reason: "A later musical domain needs no new proof — it needs a base type
//! with no eliminator, builtin signatures containing no arrow, and a discharge
//! of D1–D4." A core that enumerated the base types would make every new domain
//! a core amendment and falsify that corollary outright. So a [`Registry`] is
//! built by the caller, checked once, and carried on [`Cx`](crate::Cx) beside
//! the declared groups and the trait table, which are already exactly this
//! shape.
//!
//! **Three things, each with one job.**
//!
//! - A [`Base`] is a base type: a name and the kind it inhabits. It has no
//!   constructor, no eliminator, and contributes no ι-rule. That is D1
//!   *inertness* stated as a representation rather than as a rule to obey —
//!   there is no arm anywhere in this crate that inspects a base type's value,
//!   because there is nothing to inspect it with.
//! - A [`Literal`] is a closed value of a base type, opaque to the core.
//!   Conversion is §5.8's own sentence: two closed values of an inert base type
//!   are convertible iff they are the same constant.
//! - A [`Builtin`] is a compiler-owned operation: a name, a declared type, a
//!   [`Family`], and a δ-rule. Its *typing* needs no new rule — a builtin is a
//!   constant of a declared type and application is application — so the only
//!   new arm anywhere is reduction.
//!
//! **D1's *or* is [`Datum`].** A δ-builtin's argument and result types are "a
//! base type **or a finite constructor over base types**", and the second half
//! is not optional decoration: a host that answers `Option Scale` or takes a
//! `List Pc12` is writing an ordinary first-order function, because neither
//! family has an arrow in it. So a rule reads and writes [`Datum`] rather than
//! [`Literal`] — a literal, or a constructor of a declared family applied to
//! more of the same — and the firing condition is "every argument is canonical
//! data" rather than "every argument is a literal". The two conditions coincide
//! exactly when no declared family is involved, which is why the base-only rules
//! 141b registered behave as they always did.
//!
//! **A literal's payload is opaque, and the alternative was refused.** The core
//! needs three things from a literal and no more: which type it inhabits,
//! whether it is the same literal as another, and how to show it in a
//! diagnostic. A closed payload universe — integer, rational, text, bytes —
//! would answer all three without `dyn`, and was refused twice over: `Syntax` is
//! a tree, so every phase operation would encode and every read would decode;
//! and a list of the host's data shapes inside a leaf calculus is the
//! enumeration the corollary forbids, wearing a different hat. [`Payload`] has
//! three methods with one reason each, and the downcast is the host's own
//! concern at the host's own δ-rule.
//!
//! **D3 is enforced by the type.** A δ-rule is a `fn` pointer and not a closure,
//! so it cannot capture host state and "the result is a function of the argument
//! values alone" is checked by the Rust compiler rather than reviewed by a
//! reader. A rule that wants ambient context cannot be written down.
//!
//! **The second family reduces to a term rather than to a value.** §5.8's
//! *structural eliminators* are "the generated recursors and the derived
//! traversals over them". The recursors are `family.rs`'s ι-rule; the traversals
//! are [`Rewrite`], and they exist because a traversal takes a *function*
//! argument, which is not a literal and cannot be one. A [`Rule`] computes a
//! value from values; a [`Rewrite`] does what ι does — it rewrites
//! `Nat.elim P z s (succ k)` into `s k (Nat.elim P z s k)`, a term the core then
//! evaluates. So it reads the target it fired on and writes down what to do
//! next, and it never holds one of this crate's values: roadmap §15.12's privacy
//! boundary is what makes a rewrite honest rather than a callback.
//!
//! **Which half of D1–D4 is checked here.** [`Registry::new`] checks what the
//! signature makes visible: every name registered once, every argument and
//! result type of a δ-builtin *finite data* — a registered base type, or a
//! declared family applied to more of the same — and every structural
//! eliminator firing on an argument it takes. D2 totality, D3's
//! order-independence, and D4's size bound are
//! properties of the host's *functions over the host's domains*, and the host's
//! own law suite samples them where the table lives — `crates/musa-compiler`'s
//! `BUILTIN_OWNERSHIP` suite, since prompt 127ca. Do not look for D2 here; it is
//! not checkable from a signature.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::origin::Origin;
use crate::refuse::Refusal;
use crate::term::{Name, Shape, Term};

/// A base type: a name, and the kind it inhabits.
///
/// The kind is a term rather than a level because a base type may take
/// parameters — `Syntax` is indexed by its category and `EventTrack` by its time
/// and payload — so `Nat : Type 0` and `Syntax : Type 0 → Type 0` are both base
/// types and only a type can tell them apart. A base type with parameters is
/// still inert: it is applied, never eliminated.
///
/// Cheap to clone: everything is behind one [`Arc`], because a base type is
/// named at every occurrence of its type and copied into every value of it.
#[derive(Clone, Debug)]
pub struct Base(Arc<BaseDeclaration>);

#[derive(Debug)]
struct BaseDeclaration {
    name: Name,
    kind: Term,
}

/// Two base types are the same when they have the same name.
///
/// By name, not by pointer, for the reason [`crate::family::Constant`] gives:
/// one host may build its registry twice — a second `Cx`, a second compilation —
/// and two closed values of one base type have to stay convertible across that.
/// A registry refuses a duplicate name, so within one registry the name decides.
impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.name == other.0.name
    }
}

impl Eq for Base {}

impl fmt::Display for Base {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.name)
    }
}

impl Base {
    /// A base type named `name`, inhabiting `kind`.
    ///
    /// `kind` is `Type l` for an unparameterized base type and a Π ending in one
    /// for a parameterized one. It is read in the empty context: a base type is
    /// closed, which is what lets [`Registry`] check a signature without an
    /// environment.
    #[must_use]
    pub fn new(name: impl Into<Name>, kind: Term) -> Self {
        Self(Arc::new(BaseDeclaration {
            name: name.into(),
            kind,
        }))
    }

    /// Its name.
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.0.name
    }

    /// The kind it inhabits.
    #[must_use]
    pub fn kind(&self) -> &Term {
        &self.0.kind
    }

    /// This base type as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Base(self.clone()))
    }
}

/// A literal's value, owned by the host and opaque to the core.
///
/// Three methods, one reason each, and no fourth: [`Self::same`] is what
/// conversion at a base type calls, [`Self::shown`] is what a diagnostic prints,
/// and [`Self::as_any`] is how a host's own δ-rule reads back what it put in.
/// Nothing in this crate downcasts.
///
/// `Send + Sync` because a literal rides inside a [`Term`], and terms cross
/// threads wherever the host compiles more than one document at once.
pub trait Payload: fmt::Debug + Send + Sync + 'static {
    /// Whether this is the same value as `other`.
    ///
    /// The host decides, because the host knows what equality means for its own
    /// data. It is called only on two payloads of the *same* base type — the
    /// core compares types first — so an implementation may downcast to its own
    /// type and answer `false` when the downcast fails.
    ///
    /// It must be an equivalence relation. Conversion is decidable only if this
    /// is, and §5's matrix rests on that.
    fn same(&self, other: &dyn Payload) -> bool;

    /// How to show this value in a diagnostic.
    fn shown(&self) -> String;

    /// This value, for the host's own δ-rule to downcast.
    fn as_any(&self) -> &dyn Any;
}

/// A closed value of a base type.
///
/// It carries its whole type rather than just its [`Base`], because a
/// parameterized base type's literals are at different types: a `Syntax Expr` is
/// not a `Syntax Pattern`, and conversion has to be able to say so before it
/// reaches [`Payload::same`]. The type is closed, so it needs no environment.
#[derive(Clone, Debug)]
pub struct Literal {
    ty: Term,
    payload: Arc<dyn Payload>,
}

/// Two literals are the same when they are at the same type and the host says
/// their payloads agree.
///
/// This is §5.8's sentence and nothing more: "an inert base type contributes no
/// ι-rule, so two closed values of it are convertible iff they are the same
/// constant."
impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.payload.same(other.payload.as_ref())
    }
}

impl Eq for Literal {}

impl fmt::Display for Literal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.payload.shown())
    }
}

impl Literal {
    /// A literal of type `ty` carrying `payload`.
    ///
    /// `ty` is a closed type headed by a [`Base`]. Nothing checks that here: a
    /// literal whose type is not a base type is a host defect, and the
    /// re-checker is where a term the host built wrong is caught.
    #[must_use]
    pub fn new(ty: Term, payload: Arc<dyn Payload>) -> Self {
        Self { ty, payload }
    }

    /// Its type.
    #[must_use]
    pub fn ty(&self) -> &Term {
        &self.ty
    }

    /// Its value, for the host that put it there.
    #[must_use]
    pub fn payload(&self) -> &dyn Payload {
        self.payload.as_ref()
    }

    /// This literal as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Lit(self.clone()))
    }
}

/// Which of §5.8's four families a builtin belongs to.
///
/// Disjoint and exhaustive is a checked law over there, where the table is; here
/// it is one field, so a builtin cannot be classified twice and cannot be
/// classified none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Every argument type and the result type is a base type or a finite
    /// constructor over base types, with no arrow anywhere in the signature.
    /// D1–D4 apply to exactly these.
    Delta,
    /// A generated recursor or a derived traversal over one.
    Eliminator,
    /// A constructor or controlled transform of §5.7's event tracks.
    Track,
    /// A constructor of `../across-stages/03-machine-calculus.md` §2's machines.
    Machine,
}

impl fmt::Display for Family {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match *self {
            Self::Delta => "δ-builtin",
            Self::Eliminator => "structural eliminator",
            Self::Track => "track builtin",
            Self::Machine => "machine builtin",
        })
    }
}

/// Canonical data: what a δ-rule reads, and what it answers.
///
/// D1's disjunction, as one recursive type. A [`Literal`] is "a base type"; a
/// [`Self::Case`] is "a finite constructor over base types", and the recursion
/// is what makes `Cons(1, Cons(2, Nil))` one datum rather than a shape the core
/// would have to special-case per depth.
///
/// **One type for both directions, rather than a borrowing view in and an owned
/// tree out.** The core has to *force* an argument before it can see whether it
/// is canonical, and a forced value is a new value a borrow would have to
/// outlive — keeping them alive while building references into them is an arena,
/// which is a great deal of machinery for two [`Arc`] bumps per literal that δ
/// already pays. Symmetry buys something as well: a rule that answers one of its
/// arguments unchanged — an `or_else`, a `flatten` at the outer level — writes
/// `arguments[0].clone()` rather than transcribing a tree into the other type.
///
/// **Constructor names are qualified**, the spelling
/// [`Constant::name`](crate::Constant) prints and a diagnostic already shows:
/// `Option.Some`, never `Some`. Reading and writing therefore speak one
/// vocabulary, so handing an argument straight back really is the identity it
/// looks like, and a signature naming two families that each declare a `Nil` is
/// not ambiguous about which one it meant.
///
/// **A constructor's parameters are not fields and are not here.** `Option.Some`
/// applied to `3` has arity two — the element type, then the value — and only
/// the second is data. The parameters are recovered from the type when an answer
/// is realized, which is why realization is type-directed and reading is not.
///
/// **There is no record arm.** A product is spellable in the host's shape
/// language and used by nothing, so admitting one would be public surface with
/// no caller; a record argument is not canonical data and leaves the spine
/// blocked. When something wants one, the arm and the signature check that
/// admits it arrive together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Datum {
    /// A closed value of a base type.
    Lit(Literal),
    /// A constructor of a declared family, applied to its fields.
    Case {
        /// The constructor's qualified name, as `Family.Case`.
        constructor: Name,
        /// Its fields, in declaration order. The family's parameters are not
        /// among them.
        fields: Vec<Self>,
    },
}

/// What a builtin does to its arguments.
///
/// A `fn` pointer rather than a boxed closure, and that is D3 rather than a
/// micro-optimization: a function pointer cannot capture host state, so "the
/// result is a function of the argument values alone, with no ambient context,
/// evaluation-order dependence, hash-iteration order, or diagnostic emission" is
/// a property of the type rather than a promise a reader has to audit.
///
/// It is handed one [`Datum`] per argument, in the order the signature declares
/// them, and answers one. Nothing in the argument slice is a term or a value:
/// roadmap §15.12's privacy boundary is what makes a rule a function over data
/// rather than a callback into the evaluator.
///
/// `None` means *this rule does not apply to these arguments*, and the
/// application stays a neutral spine. For a δ-builtin at closed data of its
/// declared argument types that answer is a host defect, which D2 forbids and
/// [`crate::Malformed::BuiltinStuck`] reports.
pub type Rule = fn(&[Datum]) -> Option<Datum>;

/// How a structural eliminator takes one step: it reads the literal it fired on
/// and answers the term to evaluate in its place.
///
/// **The result is read in the environment of the arguments.** A rewrite is
/// called on a spine of exactly [`Builtin::arity`] applications, and the term it
/// answers is evaluated with those argument *values* as its environment —
/// innermost last, so [`Index`](crate::Index) `0` names the last argument,
/// `1` the one before it, and `arity - 1` the first. That is the only way a
/// traversal can hand a child to an algebra it was passed: the algebra is an
/// argument, so the rewrite names it by position, and the core supplies the
/// value it already has rather than re-evaluating anything. Binders inside the
/// answer behave as binders always do — the term is read by
/// [`eval`](crate::eval), not spliced.
///
/// **It is given itself**, because a traversal recurses: `recurse_syntax` has to
/// name `recurse_syntax` at each child, and a `fn` pointer cannot capture the
/// [`Builtin`] it lives in. [`Builtin::term`] is what turns the handle back into
/// a head.
///
/// `fn` rather than a closure for [`Rule`]'s reason, unchanged: a rewrite that
/// captured host state would make reduction depend on which compiler ran it.
///
/// **Termination is the host's obligation.** A rewrite that applied its builtin
/// to the *same* literal would not converge, and no signature shows that. §5.8
/// splits its conditions this way already — D2 and D3 are checked where the
/// table lives — so the rule to discharge over there is that every application
/// of this builtin in the answer stands at a literal strictly smaller than the
/// target. §4's meter is the backstop, and it refuses rather than hangs.
///
/// `None` means *this rule does not apply*, and at a literal target with the
/// spine full it is a host defect: [`crate::Malformed::BuiltinStuck`] says so,
/// the same way it does for a δ-rule that answers nothing.
pub type Rewrite = fn(&Builtin, &Literal) -> Option<Term>;

/// How a builtin reduces: §5.8's δ-rule, or a structural eliminator's rewrite.
///
/// One field rather than two optional ones, because a builtin has exactly one
/// way to take a step and two nullable fields could disagree about which.
#[derive(Clone, Copy, Debug)]
enum Reduction {
    /// D1–D4's: data in, data out, once every argument is canonical.
    Delta(Rule),
    /// A traversal's: a term out, once the argument at `target` is a literal.
    /// The other arguments are passed through as whatever they already are.
    Structural {
        /// Which argument has to be a literal before the rule may fire.
        target: usize,
        /// What to answer when it is.
        rewrite: Rewrite,
    },
}

/// A compiler-owned operation: what it is called, what type it has, which family
/// it belongs to, and what it computes.
///
/// Cheap to clone for the same reason [`Base`] is.
#[derive(Clone, Debug)]
pub struct Builtin(Arc<BuiltinDeclaration>);

#[derive(Debug)]
struct BuiltinDeclaration {
    name: Name,
    ty: Term,
    family: Family,
    arity: usize,
    reduction: Reduction,
}

/// Two builtins are the same when they have the same name, for the reason
/// [`Base`]'s equality gives.
impl PartialEq for Builtin {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.name == other.0.name
    }
}

impl Eq for Builtin {}

impl fmt::Display for Builtin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.name)
    }
}

impl Builtin {
    /// A builtin named `name`, of type `ty`, in `family`, computed by `rule`.
    ///
    /// Builds a builtin reduced by a δ-rule — every argument canonical data,
    /// data out. That is D1–D4's shape for [`Family::Delta`] and the shape a
    /// track or machine constructor has too, since both build a closed value out
    /// of closed values. A traversal is the other shape; [`Self::structural`]
    /// builds one.
    ///
    /// The arity is read off `ty` rather than passed: it is the number of Π
    /// binders the signature has, and a second copy would be free to disagree
    /// with the first. `ty` is read in the empty context, because a builtin is
    /// closed.
    #[must_use]
    pub fn new(name: impl Into<Name>, ty: Term, family: Family, rule: Rule) -> Self {
        Self::declared(name, ty, family, Reduction::Delta(rule))
    }

    /// A structural eliminator named `name`, of type `ty`, firing on its
    /// `target`th argument and rewriting by `rewrite`.
    ///
    /// [`Family::Eliminator`] by construction rather than by parameter: among
    /// builtins the family and the rule are the same fact said twice, and §5.8's
    /// other three families all compute a value from values.
    ///
    /// **The target is written down rather than inferred.** ι fires on the
    /// recursor's last argument, and the eleven traversals `musa-compiler` owns
    /// today all fire on their first — neither is a law, and a mechanism that
    /// guessed would be one more thing to remember at every registration.
    /// [`Registry::new`] checks that the index names an argument and that the
    /// argument's type is a base type of the registry.
    #[must_use]
    pub fn structural(name: impl Into<Name>, ty: Term, target: usize, rewrite: Rewrite) -> Self {
        Self::declared(name, ty, Family::Eliminator, Reduction::Structural { target, rewrite })
    }

    fn declared(name: impl Into<Name>, ty: Term, family: Family, reduction: Reduction) -> Self {
        let arity = arity_of(&ty);
        Self(Arc::new(BuiltinDeclaration {
            name: name.into(),
            ty,
            family,
            arity,
            reduction,
        }))
    }

    /// Its name.
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.0.name
    }

    /// Its declared type.
    #[must_use]
    pub fn ty(&self) -> &Term {
        &self.0.ty
    }

    /// Which family it belongs to.
    #[must_use]
    pub fn family(&self) -> Family {
        self.0.family
    }

    /// How many arguments it takes before it can compute.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.0.arity
    }

    /// This builtin as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::new(origin, Shape::Builtin(self.clone()))
    }

    /// Its δ-rule, if it reduces that way.
    ///
    /// Called only at exactly [`Self::arity`] arguments, every one of them
    /// canonical data: an application short of the arity, or one whose argument
    /// has not reduced to data, is a blocked spine and never reaches it.
    pub(crate) fn delta_rule(&self) -> Option<Rule> {
        match self.0.reduction {
            Reduction::Delta(rule) => Some(rule),
            Reduction::Structural { .. } => None,
        }
    }

    /// Its structural rule and the argument that triggers it, if it reduces that
    /// way.
    ///
    /// The index is within [`Self::arity`], because [`Registry::new`] refused
    /// the registration otherwise.
    pub(crate) fn structural_rule(&self) -> Option<(usize, Rewrite)> {
        match self.0.reduction {
            Reduction::Delta(_) => None,
            Reduction::Structural { target, rewrite } => Some((target, rewrite)),
        }
    }
}

/// How many Π binders a signature has.
fn arity_of(ty: &Term) -> usize {
    let mut arity = 0usize;
    let mut rest = ty;
    while let Shape::Pi { codomain, .. } = rest.shape() {
        arity = arity.saturating_add(1);
        rest = codomain;
    }
    arity
}

/// A name the host has registered: a base type or a builtin.
///
/// One enum rather than two lookups, because a caller that resolves a name wants
/// to know what it found in one question, and because a name may be either but
/// never both.
#[derive(Clone, Debug)]
pub enum Extern {
    /// A base type.
    Base(Base),
    /// A compiler-owned operation.
    Builtin(Builtin),
}

impl Extern {
    /// Its type: the kind for a base type, the signature for a builtin.
    #[must_use]
    pub fn ty(&self) -> &Term {
        match *self {
            Self::Base(ref base) => base.kind(),
            Self::Builtin(ref builtin) => builtin.ty(),
        }
    }

    /// This name as a term.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        match *self {
            Self::Base(ref base) => base.term(origin),
            Self::Builtin(ref builtin) => builtin.term(origin),
        }
    }
}

/// The host's base types and builtins, checked once and then immutable.
///
/// Immutable on purpose, and the alternative is worth naming because it is the
/// obvious one: a registry that interned new literals as evaluation produced
/// them would need interior mutability, and a table that changes while a term is
/// being normalized is an evaluation-order dependence — the exact D3 this module
/// exists to enforce. A [`Literal`] therefore carries its own payload and no
/// table is consulted to compare two.
#[derive(Debug, Default)]
pub struct Registry {
    names: HashMap<Name, Extern>,
}

impl Registry {
    /// The registry holding `bases` and `builtins`, or why it is not one.
    ///
    /// Checks the half of §5.8 a signature makes visible, and says in the module
    /// doc which half that is:
    ///
    /// - **one name, one meaning** — no name registered twice, whether as two
    ///   base types, two builtins, or one of each;
    /// - **a δ signature is finite data** (D1) — every argument type and the
    ///   result type is a registered base type, or a declared family applied to
    ///   argument types that are themselves this, at any depth. Stated
    ///   positively rather than as "no arrow", because "no arrow" also admitted
    ///   the record types, universes, and bare variables D1 never meant, and
    ///   because a rule can only *read* and *answer* what [`Datum`] can say;
    /// - **a structural eliminator has a target it could fire on** — the index
    ///   [`Builtin::structural`] wrote down names an argument of the signature,
    ///   and that argument's type is headed by a base type of this registry.
    ///
    /// D1 is checked over δ-builtins and nowhere else, which is where §5.8
    /// states it. A structural eliminator's signature holds an arrow by
    /// definition — a traversal takes an algebra — so a registry that applied D1
    /// to the whole table would refuse the family it is registering. What
    /// replaces it for that family is the target check: a rewrite whose target
    /// is a *declared* type would be a second ι-rule for something that already
    /// has one, and the second path is what the audits keep looking for.
    ///
    /// An arrow keeps its own diagnostic inside the positive check, because a Π
    /// where data was wanted is the D1 violation a table author actually writes,
    /// and [`Refusal::HigherOrderDelta`] says the specific thing.
    ///
    /// # Errors
    ///
    /// [`Refusal::DuplicateExtern`], [`Refusal::HigherOrderDelta`],
    /// [`Refusal::NotFiniteData`], [`Refusal::UnknownBase`],
    /// [`Refusal::TargetOutsideSignature`], or [`Refusal::TargetNotABase`].
    pub fn new(bases: Vec<Base>, builtins: Vec<Builtin>) -> Result<Self, Refusal> {
        let mut names: HashMap<Name, Extern> = HashMap::with_capacity(bases.len().saturating_add(builtins.len()));
        for base in bases {
            claim(&mut names, Arc::clone(base.name()), Extern::Base(base))?;
        }
        for builtin in builtins {
            claim(&mut names, Arc::clone(builtin.name()), Extern::Builtin(builtin))?;
        }
        let registry = Self { names };
        registry.check_delta_signatures()?;
        registry.check_structural_targets()?;
        Ok(registry)
    }

    /// What `name` names here, if anything.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Extern> {
        self.names.get(name)
    }

    /// Every structural eliminator's target: an argument of its own signature,
    /// at a base type this registry declared inert.
    fn check_structural_targets(&self) -> Result<(), Refusal> {
        for entry in self.names.values() {
            let Extern::Builtin(builtin) = entry else {
                continue;
            };
            let Some((target, _)) = builtin.structural_rule() else {
                continue;
            };
            // `signature_parts` ends with the result type, and a rule cannot
            // fire on what it produces, so the bound is the arity rather than
            // the number of parts.
            let domain = signature_parts(builtin.ty())
                .into_iter()
                .nth(target)
                .filter(|_| target < builtin.arity())
                .ok_or_else(|| Refusal::TargetOutsideSignature {
                    name: Arc::clone(builtin.name()),
                    at: builtin.ty().origin(),
                })?;
            let Some(base) = head_base(&domain) else {
                return Err(Refusal::TargetNotABase {
                    name: Arc::clone(builtin.name()),
                    at: domain.origin(),
                });
            };
            if self.named(base.name()).is_none() {
                return Err(Refusal::UnknownBase {
                    name: Arc::clone(base.name()),
                    at: domain.origin(),
                });
            }
        }
        Ok(())
    }

    /// D1's signature half, over every δ-builtin at once.
    fn check_delta_signatures(&self) -> Result<(), Refusal> {
        for entry in self.names.values() {
            let Extern::Builtin(builtin) = entry else {
                continue;
            };
            if builtin.family() != Family::Delta {
                continue;
            }
            for argument in signature_parts(builtin.ty()) {
                self.check_finite_data(builtin, &argument)?;
            }
        }
        Ok(())
    }

    /// One argument or result type of a δ-builtin: finite data, at any depth.
    ///
    /// Two shapes and no third. A registered base type, applied to whatever it
    /// takes, is data because §5.8 declares it inert. A declared family applied
    /// to more data is data because prompt 141 proved `List`, `Option`, and
    /// `Result` are ordinary declarations — their values are constructor
    /// applications, which is exactly what [`Datum::Case`] says.
    ///
    /// Everything else is refused, including the ones that would be *harmless*
    /// to admit. A literal in an index position — `Vec Nat 3` — is not written
    /// by anything in the table this check exists for, and a check that admits
    /// what nothing writes is a check nobody has read; the day something wants
    /// one, it arrives with a caller and a law.
    fn check_finite_data(&self, builtin: &Builtin, ty: &Term) -> Result<(), Refusal> {
        let mut head = ty;
        let mut arguments = Vec::new();
        while let Shape::App { function, argument } = head.shape() {
            arguments.push(argument);
            head = function;
        }
        match head.shape() {
            // A Π keeps its own diagnostic: it is what a table author writes
            // when they reach for a higher-order operation, and "not finite
            // data" would be a true sentence about the wrong problem.
            Shape::Pi { .. } => {
                return Err(Refusal::HigherOrderDelta {
                    name: Arc::clone(builtin.name()),
                    at: builtin.ty().origin(),
                });
            }
            Shape::Base(base) => {
                if self.named(base.name()).is_none() {
                    return Err(Refusal::UnknownBase {
                        name: Arc::clone(base.name()),
                        at: head.origin(),
                    });
                }
            }
            Shape::Const(constant) if constant.is_family() => {}
            Shape::Var(_)
            | Shape::Universe(_)
            | Shape::Const(_)
            | Shape::Lam { .. }
            | Shape::App { .. }
            | Shape::RecordType(_)
            | Shape::Record(_)
            | Shape::Project { .. }
            | Shape::Id { .. }
            | Shape::Refl(_)
            | Shape::J { .. }
            | Shape::Let { .. }
            | Shape::Builtin(_)
            | Shape::Lit(_)
            | Shape::Meta(_) => {
                return Err(Refusal::NotFiniteData {
                    name: Arc::clone(builtin.name()),
                    at: head.origin(),
                });
            }
        }
        for argument in arguments {
            self.check_finite_data(builtin, argument)?;
        }
        Ok(())
    }
}

/// Claim a name for one meaning, or refuse because something else has it.
fn claim(names: &mut HashMap<Name, Extern>, name: Name, entry: Extern) -> Result<(), Refusal> {
    // The declared type is the only piece of a registration that came from
    // anywhere, so it is where a diagnostic about the registration points.
    let at = entry.ty().origin();
    if names.contains_key(&name) {
        return Err(Refusal::DuplicateExtern { name, at });
    }
    names.insert(name, entry);
    Ok(())
}

/// The base type a type is an application of, if it is one of anything.
///
/// A base type may take parameters, so `Syntax Expr` is at `Syntax` and the
/// spine says which one. Anything else — a variable, a declared family, a record
/// type — is not a base type and has no answer here.
fn head_base(ty: &Term) -> Option<&Base> {
    let mut head = ty;
    while let Shape::App { function, .. } = head.shape() {
        head = function;
    }
    if let Shape::Base(base) = head.shape() {
        Some(base)
    } else {
        None
    }
}

/// A signature's argument types and its result type, in order.
fn signature_parts(ty: &Term) -> Vec<Term> {
    let mut parts = Vec::new();
    let mut rest = ty.clone();
    while let Shape::Pi { domain, codomain, .. } = rest.shape() {
        parts.push(domain.clone());
        let next = codomain.clone();
        rest = next;
    }
    parts.push(rest);
    parts
}
