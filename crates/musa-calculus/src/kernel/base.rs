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
//!   [`Family`], and how it reduces — a δ-rule, a structural rewrite, or
//!   nothing at all. Its *typing* needs no new rule — a builtin is a constant of
//!   a declared type and application is application — so the only new arm
//!   anywhere is reduction, and the third shape adds not even that:
//!   [`Builtin::constructor`] is how a table registers a form that §2 of
//!   `../../rules/across-stages/03-machine-calculus.md` gives a typing rule and
//!   no reduction.
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

use crate::kernel::family::Parameter;
use crate::kernel::origin::Origin;
use crate::kernel::term::{Binder, Constant, Name, Role, Shape, Term};

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
    storable: bool,
    accepts: Option<Accepts>,
    measures: Option<Measures>,
    index: Option<Parameter>,
}

/// Whether a position of an indexed base type accepts a value of it at a
/// *different* index, and the operation that carries the value across.
///
/// The host's rule, because the core has no opinion about what an index means:
/// `Syntax ⟨expr⟩` standing where `Syntax ⟨token-tree⟩` is asked for is
/// `docs/rules/language/11-quotation.md` §1's forgetting rule, and only the
/// compiler that registered `Cat` knows which way it runs. Answering with a
/// *name* rather than with a [`Builtin`] is what keeps this a `fn` pointer:
/// nothing here captures, so the rule is fixed at registration exactly as a
/// [`Rule`] is, and D3's order-independence needs no second argument.
///
/// Called only when the two indices are literals of the same base type, and only
/// from the elaborator's one directional site — [`crate::elaboration::elab`]'s `check` —
/// never from conversion. That is the difference between an acceptance rule and
/// a definitional equality, and it is the whole difference: conversion is
/// symmetric, so a rule living there would admit the reverse direction too, and
/// an index that certifies both ways certifies nothing. Answer `None` when the
/// indices are the same; ordinary conversion decides that case.
///
/// The name must be a δ-[`Builtin`] of the same registry, taking one argument of
/// the found type and answering at the expected one. A name that is not is
/// [`crate::Malformed::UnregisteredCarrier`] — a table defect, like every other
/// disagreement between a host's rule and its own signature.
pub type Accepts = fn(expected: &Literal, found: &Literal) -> Option<&'static str>;

/// How to read a literal of this base type as an **index** — a value of
/// `02-core-calculus.md` §1.5's decidable arithmetic domain.
///
/// The answer is an exact fraction as `(numerator, denominator)`, so `Nat` and
/// exact `Ratio` are one signature: a whole number answers with a denominator
/// of one. `None` is "this literal is not a number", which is a host defect
/// rather than a program's fault — a base type registers this exactly when its
/// literals *are* the domain.
///
/// A `fn` pointer for [`Accepts`]'s reason, and needed for D1's: a literal's
/// payload is [`Arc<dyn Payload>`](Payload) and opaque to this crate, so the
/// only party that can turn one into a number is the host that put it there.
/// Registered on the base type rather than on the payload because the *type* is
/// what says a position is an index position; a payload that happens to hold a
/// number is not thereby an index.
///
/// A base type without one has no literals the solver can read, so an index
/// written at it is §1.5's refusal that names the expression. That is the
/// intended default: [`Base::new`] leaves it absent, and only `Nat` and `Ratio`
/// have any reason to supply it.
pub type Measures = fn(literal: &Literal) -> Option<(i128, i128)>;

/// Two base types are the same when they have the same name.
///
/// By name, not by pointer, for the reason [`crate::kernel::family::Constant`] gives:
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
    ///
    /// Not storable, which is `02-core-calculus.md` §1.2's own default for a
    /// compiler-owned type: it is storable "only when its owner guarantees that
    /// its hidden representation contains no closure and supplies the exact
    /// encoding", and a host that has not said so has not guaranteed it. See
    /// [`Self::storable`].
    #[must_use]
    pub fn new(name: impl Into<Name>, kind: Term) -> Self {
        Self(Arc::new(BaseDeclaration {
            name: name.into(),
            kind,
            storable: false,
            accepts: None,
            measures: None,
            index: None,
        }))
    }

    /// The same base type, declared to carry an index of sort `sort`, written
    /// `name` (§1.5).
    ///
    /// Beside [`Self::measuring`] and its neighbours, and the pair is worth
    /// reading together: [`Self::measuring`] says *my literals are index
    /// values*, and this says *I am written with one*. A type is at most one of
    /// the two — an index is drawn from a sort and carried by something else —
    /// and nothing stops a host declaring both, which is a host's mistake and
    /// not one this crate can see.
    ///
    /// The sort is checked when the registry is built, not here, for
    /// [`Registry::new`]'s reason: a base type is a value a host writes down,
    /// and the table is where a host's table is refused.
    #[must_use]
    pub fn indexed_by(&self, name: impl Into<Name>, sort: Term) -> Self {
        Self(Arc::new(BaseDeclaration {
            index: Some(Parameter::written(name.into(), sort)),
            ..self.declaration()
        }))
    }

    /// The index it is declared to carry, if it declares one.
    pub(crate) fn declared_index(&self) -> Option<&Parameter> {
        self.0.index.as_ref()
    }

    /// The same base type, with its owner's guarantee that it is storable data.
    ///
    /// §1.2 states storability as a `Storable` constraint whose instances are
    /// generated and never written, and a base type is the one shape this crate
    /// cannot decide for itself: there is nothing to look inside. So the host
    /// says it here and the core generates the instance, exactly as it does for
    /// a declaration group — and a host that says nothing gets the smaller set
    /// of instances, which is §1.2's "exception in the safe direction".
    ///
    /// Unconditional, because that is the whole of what has a caller. §1.2 also
    /// admits a *conditional* base instance — `Machine K A B` is storable when
    /// `A` and `B` are — and a base type that wants one is registered without
    /// this and refused at the site that needs it, which errs the same safe way.
    #[must_use]
    pub fn storable(&self) -> Self {
        Self(Arc::new(BaseDeclaration {
            storable: true,
            ..self.declaration()
        }))
    }

    /// The same base type, with the host's rule for when one of its index
    /// positions accepts a value of it at another index.
    ///
    /// Beside [`Self::storable`] rather than a parameter of [`Self::new`] for
    /// the same reason: almost no base type has one — a base type with no index
    /// cannot — and a constructor that made every caller write `None` would
    /// spell the absence more often than the presence. See [`Accepts`] for what
    /// the rule may say and where it is asked.
    #[must_use]
    pub fn accepting(&self, accepts: Accepts) -> Self {
        Self(Arc::new(BaseDeclaration {
            accepts: Some(accepts),
            ..self.declaration()
        }))
    }

    /// The same base type, with its owner's rule for reading its literals as
    /// index values (§1.5).
    ///
    /// Beside [`Self::accepting`] and [`Self::storable`] for the reason each of
    /// them gives: almost no base type is an index domain, and a constructor
    /// parameter would spell the absence at every registration.
    ///
    /// # The distinction this method records
    ///
    /// §1.5 ends by naming six compiler-owned types that *look* indexed and are
    /// not — `Duration C`, `Position C`, `EventTrack C δ`, `Syntax<Cat>`,
    /// `Primitive K δ δ`, and `Machine K δ δ` — and this is where the record
    /// belongs, because the kind alone cannot tell a reader which of the two an
    /// argument is. Each of those arguments is a **parameter**: it survives into
    /// the elaborated term and is compared by §3's ordinary conversion. None of
    /// them registers a measure, and none could:
    ///
    /// - `Duration WrittenTime` and `Duration PhysicalTime` must be
    ///   *inconvertible*. Erasure is exactly the promise that the argument stops
    ///   mattering, so an erased index would make them one type.
    /// - `Syntax<Cat>` carries a directional forgetting rule ([`Accepts`],
    ///   `11-quotation.md` §1). Conversion is symmetric; a solver that decided
    ///   both directions would certify nothing.
    /// - Two of `Machine`'s three arguments are *types*, and no arithmetic
    ///   domain contains a type.
    ///
    /// So the two mechanisms sit side by side and the question each answers is
    /// the difference: a parameter says *what this is a type of*, and survives;
    /// an index says *how many* or *how much*, is decided by arithmetic, and is
    /// erased.
    #[must_use]
    pub fn measuring(&self, measures: Measures) -> Self {
        Self(Arc::new(BaseDeclaration {
            measures: Some(measures),
            ..self.declaration()
        }))
    }

    /// Its owner's rule for reading its literals as index values, if it has one.
    pub(crate) fn measures(&self) -> Option<Measures> {
        self.0.measures
    }

    /// Whether its owner registered one.
    ///
    /// The [`Measures`] question without the `fn` pointer, because a host that
    /// wants to check its own table has no use for the pointer and every use
    /// for the answer: a base type written at one site and registered at
    /// another compares equal by name and can still differ here, which is a
    /// defect nothing else can see.
    #[must_use]
    pub fn reads_an_index(&self) -> bool {
        self.0.measures.is_some()
    }

    /// This declaration's fields, for a method that replaces one of them.
    fn declaration(&self) -> BaseDeclaration {
        BaseDeclaration {
            name: Arc::clone(&self.0.name),
            kind: self.0.kind.clone(),
            storable: self.0.storable,
            accepts: self.0.accepts,
            measures: self.0.measures,
            index: self.0.index.clone(),
        }
    }

    /// Whether its owner guaranteed it storable.
    pub(crate) fn is_storable(&self) -> bool {
        self.0.storable
    }

    /// Its owner's acceptance rule, if it has one.
    pub(crate) fn accepts(&self) -> Option<Accepts> {
        self.0.accepts
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

    /// This base type as a term: its name, at [`Role::Base`].
    ///
    /// The registration stays here and the term takes the name (§6). The role
    /// is what a reader of the term still needs — a base type is not a declared
    /// family, and `Registry`'s own checks decide the two apart without a
    /// context to ask.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::named(origin, Arc::clone(self.name()), Role::Base)
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
        Term::new(origin, Shape::Lit(Constant::Payload(self.clone())))
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
    /// A closed value of a counting family, as how far above its floor it
    /// stands.
    ///
    /// The tower is *not* built. A rule that reads a `Nat` wants the number, and
    /// the core already holds it as a [`u64`]: expanding it into `count` nested
    /// [`Self::Case`]s so the rule could count them back down would allocate a
    /// node per unit, and dropping that tree recurses on the host stack — the
    /// overflow [`Constant::Numeral`](crate::Constant::Numeral) exists to remove,
    /// reintroduced one layer out. A rule reads this arm the way it reads a
    /// literal.
    ///
    /// The family is named, and qualified the way [`Self::Case`]'s constructor
    /// is, so reading and writing speak one vocabulary and a rule can check
    /// which family it was handed.
    Count {
        /// The counting family, as the name a diagnostic would print: `Nat`.
        family: Name,
        /// How far above its floor the value stands.
        count: u64,
    },
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
/// them, and answers an [`Answer`]. Nothing in the argument slice is a term or a
/// value: roadmap §15.12's privacy boundary is what makes a rule a function over
/// data rather than a callback into the evaluator.
///
/// `None` means *this rule does not apply to these arguments*, and the
/// application stays a neutral spine. For a δ-builtin at closed data of its
/// declared argument types that answer is a host defect, which D2 forbids and
/// [`crate::Malformed::BuiltinStuck`] reports.
pub type Rule = fn(&[Datum]) -> Option<Answer>;

/// What a δ-rule says about arguments it *does* apply to.
///
/// # Why the rule can refuse at all
///
/// `02-core-calculus.md` §4 gives every judgment three outcomes, and until this
/// type existed a rule could express two of them. It answered `Option<Datum>`,
/// so "this program is wrong" had nowhere to go but `None` — which the evaluator
/// reads as D2 broken and [`crate::Malformed::BuiltinStuck`] restates as a
/// defect in the compiler rather than in the source. Every registered operation
/// with a real refusal therefore answered `Result τ Text` and handed the
/// composer's own mistake back as a *value*, which put a `Result` on the type of
/// every track a notated block builds and would put a `?` between every two
/// numbers a program adds.
///
/// So the distinction this draws is between a *program* the rule rejects and a
/// *term* the rule cannot have been given. The first is the composer's to fix
/// and lands on their span; the second is this compiler's, stays `None`, and
/// says so.
///
/// # Why the rule says the sentence and the core says the place
///
/// A rule sees [`Datum`]s. It has no term, no origin, and no access to the
/// refusal vocabulary — that is D3's whole point, and widening this to let a
/// rule build a [`crate::Refusal`] itself would hand every host table the core's
/// internals for one narrow need. [`Answer::Refused`] carries the sentence
/// alone; [`crate::kernel::eval`] attaches the origin of the application that fired,
/// which is the node the composer wrote.
#[derive(Clone, Debug)]
pub enum Answer {
    /// What the rule computed.
    Reduced(Datum),
    /// The program is wrong, and this is what to say about it.
    ///
    /// Owned rather than `&'static str` because a rule may be restating a
    /// failure a library it called reported — `musa_events`'s, in several of the
    /// track rules — and a borrowed sentence could not carry one.
    Refused(String),
}

impl From<Datum> for Answer {
    fn from(value: Datum) -> Self {
        Self::Reduced(value)
    }
}

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
/// [`eval`](crate::kernel::eval), not spliced.
///
/// **It is given itself**, because a traversal recurses: `recurse_syntax` has to
/// name `recurse_syntax` at each child, and a `fn` pointer cannot capture the
/// [`Builtin`] it lives in. [`Builtin::term`] is what turns the handle back into
/// a head.
///
/// **And through itself, the names it may write.** An argument is nameable by
/// index and the builtin by its own handle, but a constructor of a *declared*
/// family is neither — and a traversal whose group branch takes a `List A` has
/// to build one. [`Builtin::vocabulary`] is where the host puts the closed terms
/// its rewrite is allowed to write, fixed at registration by
/// [`Builtin::structural_with`]; see that constructor for why this is not a
/// weakening of D3.
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

/// How a builtin reduces: §5.8's δ-rule, a structural eliminator's rewrite, or
/// not at all.
///
/// One field rather than three optional ones, because a builtin has exactly one
/// way to take a step and separate nullable fields could disagree about which.
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
    /// A constructor's: none. Its application is its value.
    None,
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
    /// The closed terms this builtin's rewrite may write. Empty for every
    /// builtin that needs none, which is every δ-builtin and most traversals.
    vocabulary: Vec<Term>,
    /// Which index-language operator this builtin spells, if any.
    indexes: Option<Operator>,
}

/// The three operators `02-core-calculus.md` §1.5's index grammar admits.
///
/// A host tags its own arithmetic builtins with these ([`Builtin::indexing`]),
/// because the core must not know that `nat_add` is addition: naming a host's
/// operations here would put the compiler's vocabulary inside the calculus, and
/// two hosts spelling addition differently would then be two calculi.
///
/// **Only an open application is ever read through one.** A δ-rule fires as soon
/// as its arguments are canonical, so `2 + 3` has already reduced to `5` by the
/// time an index position is compared, and the operator matters exactly when a
/// variable is under it — `Bar(p + q)`, which is the case §1.5 exhibits and the
/// reason a linear form is the normal form rather than a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    /// `i + i`.
    Add,
    /// `i - i`.
    Subtract,
    /// `k * i`. §1.5 admits multiplication **by a literal** only; an
    /// application with a variable on both sides leaves the fragment and is
    /// refused at the expression, not approximated.
    Multiply,
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
    /// track constructor has too, since it builds a closed value out of closed
    /// values. A traversal is the second shape and [`Self::structural`] builds
    /// one; a form that has a type and no computation at all is the third, and
    /// [`Self::constructor`] builds that.
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
    /// [`Family::Eliminator`] because a traversal is one. Which family an
    /// operation belongs to is a *separate* question from how it reduces, and
    /// [`Self::structural_with`] takes it as a parameter for that reason; this
    /// spelling is the case where the answer is not in doubt.
    ///
    /// **The target is written down rather than inferred.** ι fires on the
    /// recursor's last argument, and the eleven traversals `musa-compiler` owns
    /// today all fire on their first — neither is a law, and a mechanism that
    /// guessed would be one more thing to remember at every registration.
    /// [`Registry::new`] checks that the index names an argument and that the
    /// argument's type is a base type of the registry.
    #[must_use]
    pub fn structural(name: impl Into<Name>, ty: Term, target: usize, rewrite: Rewrite) -> Self {
        Self::structural_with(name, ty, Family::Eliminator, target, Vec::new(), rewrite)
    }

    /// [`Self::structural`] in `family`, plus the closed terms its rewrite may
    /// write.
    ///
    /// **The family is a parameter, and that is a repair.** This used to fix
    /// [`Family::Eliminator`] by construction, arguing that "among builtins the
    /// family and the rule are the same fact said twice, and §5.8's other three
    /// families all compute a value from values". `map_note_pitches` is the
    /// counterexample: it is a §5.7 *controlled transform*, so §5.8's third
    /// family is the argument that covers it, and its mapper is a function, so
    /// no δ-rule can ever fire at it — [`crate::kernel::eval`]'s `canonical` answers
    /// `None` at a λ and the spine blocks forever. The family says which
    /// admissibility argument covers the operation; the reduction says how it
    /// computes; and one operation needing two different answers is what proves
    /// they are two questions.
    ///
    /// **Why a rewrite needs a vocabulary at all.** A [`Rewrite`] can name three
    /// things: a literal it was handed, an argument of the spine by [`Index`](crate::Index),
    /// and its own [`Builtin`]. That is enough for a traversal whose branches
    /// answer at a base type, and not enough for one whose group branch takes a
    /// `List A` — building the list means writing `List.Cons`, which is a
    /// constructor of a *declared* family, and a `fn` pointer cannot look one up.
    /// The narrow reading is that `Rewrite` wants more arguments. The real one is
    /// that it wants *names*, and the host is the only thing that has them: it
    /// declared the family, so it can resolve the constructor and hand the term
    /// over once, here.
    ///
    /// **D3 does not move.** The terms are fixed at registration and closed, so
    /// they mean the same thing under every environment the rewrite's answer is
    /// read in; the `fn` still captures nothing; and two compilers that register
    /// the same vocabulary reduce the same term to the same normal form. What
    /// would re-open D3 is a vocabulary that could change between two reductions,
    /// which is why this takes owned terms rather than a lookup.
    ///
    /// **Closedness is the host's obligation**, for the reason termination is:
    /// nothing in a `Term` says which context it was read in, so a vocabulary
    /// entry holding a free variable would be a term meaning one thing where it
    /// was resolved and another where it is spliced. Resolve them in the empty
    /// context — a declared constant is closed by construction — and the
    /// question does not arise.
    #[must_use]
    pub fn structural_with(
        name: impl Into<Name>,
        ty: Term,
        family: Family,
        target: usize,
        vocabulary: Vec<Term>,
        rewrite: Rewrite,
    ) -> Self {
        Self::declared_with(name, ty, family, Reduction::Structural { target, rewrite }, vocabulary)
    }

    /// A builtin named `name`, of type `ty`, in `family`, that computes nothing.
    ///
    /// Its saturated application is a neutral spine and stays one, so two of
    /// them are the same value exactly when they were built the same way — which
    /// is the whole of what `../../rules/across-stages/03-machine-calculus.md`
    /// §2 says about a machine. That section gives its forms **typing rules and
    /// no reductions**, and this is how a table says so.
    ///
    /// **Why this is not a [`Rule`] that always answers `None`.** Three
    /// differences, and each one is a thing that would otherwise go wrong:
    ///
    /// - A rule that answered nothing at closed data is D2 broken, and
    ///   [`crate::Malformed::BuiltinStuck`] would report it — correctly, because
    ///   that is what a forgotten arm looks like. A constructor has no rule to
    ///   answer nothing, so the diagnostic cannot misfire and the forgotten arm
    ///   it names stays a real defect everywhere else.
    /// - §5.8's inertness argument is about which operations contribute a
    ///   reduction. A constructor contributes none, so it is covered by that
    ///   argument in the way a base type is, rather than by an appeal to a rule
    ///   nobody can run.
    /// - A rule is *reachable*: `Rule` is a `fn` pointer, so a table could hand
    ///   the same pointer to a δ-builtin, and "this one is never called" would
    ///   be a claim about every registration rather than about this one. Here it
    ///   is the type.
    ///
    /// **It is not a weaker [`Self::new`], because a machine builtin's rule
    /// could never fire anyway.** Every form of §2 is polymorphic in its step
    /// tag and its ports, so a type stands on every one of those spines,
    /// [`crate::kernel::eval`]'s `canonical` answers `None` at a universe, and δ blocks
    /// forever. Registering such a form with a rule would be writing an arm that
    /// is unreachable by construction and cannot be tested; registering it
    /// without one says the same thing and can be read.
    #[must_use]
    pub fn constructor(name: impl Into<Name>, ty: Term, family: Family) -> Self {
        Self::declared(name, ty, family, Reduction::None)
    }

    fn declared(name: impl Into<Name>, ty: Term, family: Family, reduction: Reduction) -> Self {
        Self::declared_with(name, ty, family, reduction, Vec::new())
    }

    fn declared_with(
        name: impl Into<Name>,
        ty: Term,
        family: Family,
        reduction: Reduction,
        vocabulary: Vec<Term>,
    ) -> Self {
        let arity = arity_of(&ty);
        Self(Arc::new(BuiltinDeclaration {
            name: name.into(),
            ty,
            family,
            arity,
            reduction,
            vocabulary,
            indexes: None,
        }))
    }

    /// The same builtin, tagged as spelling `operator` in §1.5's index grammar.
    ///
    /// Beside the constructors rather than a parameter of them, for
    /// [`Base::measuring`]'s reason: three builtins per numeric type carry a
    /// tag and every other registration would spell its absence. See
    /// [`Operator`] for why the core takes this from the host instead of
    /// recognizing an operation by name.
    #[must_use]
    pub fn indexing(&self, operator: Operator) -> Self {
        Self(Arc::new(BuiltinDeclaration {
            name: Arc::clone(&self.0.name),
            ty: self.0.ty.clone(),
            family: self.0.family,
            arity: self.0.arity,
            reduction: self.0.reduction,
            vocabulary: self.0.vocabulary.clone(),
            indexes: Some(operator),
        }))
    }

    /// Which index-language operator it spells, if the host said it spells one.
    pub(crate) fn indexes(&self) -> Option<Operator> {
        self.0.indexes
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

    /// The closed terms its rewrite may write, in the order the host registered
    /// them.
    ///
    /// Empty unless [`Self::structural_with`] was given some. A rewrite reads
    /// these by position for the same reason it reads its arguments by position:
    /// the host wrote both lists, and a name would be a second spelling to keep
    /// in step with the first.
    #[must_use]
    pub fn vocabulary(&self) -> &[Term] {
        &self.0.vocabulary
    }

    /// This builtin as a term: its name, at [`Role::Builtin`].
    ///
    /// The δ-table stays here and the term takes the name (§6). The role is
    /// what a reader of the term still needs — `elab` reads a *registered*
    /// signature's arrow as its whole parameter list, where a definition's is a
    /// parameter list and a returned function.
    #[must_use]
    pub fn term(&self, origin: Origin) -> Term {
        Term::named(origin, Arc::clone(self.name()), Role::Builtin)
    }

    /// Its δ-rule, if it reduces that way.
    ///
    /// Called only at exactly [`Self::arity`] arguments, every one of them
    /// canonical data: an application short of the arity, or one whose argument
    /// has not reduced to data, is a blocked spine and never reaches it.
    pub(crate) fn delta_rule(&self) -> Option<Rule> {
        match self.0.reduction {
            Reduction::Delta(rule) => Some(rule),
            Reduction::Structural { .. } | Reduction::None => None,
        }
    }

    /// Its structural rule and the argument that triggers it, if it reduces that
    /// way.
    ///
    /// The index is within [`Self::arity`], because [`Registry::new`] refused
    /// the registration otherwise.
    pub(crate) fn structural_rule(&self) -> Option<(usize, Rewrite)> {
        match self.0.reduction {
            Reduction::Delta(_) | Reduction::None => None,
            Reduction::Structural { target, rewrite } => Some((target, rewrite)),
        }
    }
}

/// How many Π binders a signature has.
fn arity_of(ty: &Term) -> usize {
    let mut arity = 0usize;
    let mut rest = ty;
    while let Shape::Bind {
        binder: Binder::Pi { .. },
        body,
        ..
    } = rest.shape()
    {
        arity = arity.saturating_add(1);
        rest = body;
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
    pub(crate) names: HashMap<Name, Extern>,
}

impl Registry {
    // `Registry::new` and the admission checks it runs live in
    // `elaboration/admit.rs`: they read a signature and answer a
    // `Refusal`, which is the elaborator's word and not the kernel's.

    /// What `name` names here, if anything.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Extern> {
        self.names.get(name)
    }
}
