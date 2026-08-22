//! The raw term: what elaboration reads, and the crate's one boundary with
//! syntax.
//!
//! `docs/rules/language/02-core-calculus.md` §2 elaborates a *surface* term into
//! a core term. This type is what stands in for "surface" here, and it is
//! deliberately **surface-independent**: it knows about names, filling, and
//! annotations, and it knows nothing about pitches, bars, or `.musa` grammar.
//! That is what lets `musa-calculus` stay a leaf, and what lets the elaborator's
//! tests be written without a parser.
//!
//! # What separates it from [`Term`](crate::Term)
//!
//! Four things, and each is a job elaboration does:
//!
//! - **Variables are names, not indices.** Resolving a name to a de Bruijn index
//!   is elaboration's, so a raw term never has to be written under a mental
//!   model of how deep it is.
//! - **A binder says how a use site fills it.** §1: "the filling is not in the
//!   core". A raw binder marked [`Filling::Parameter`] is a type parameter §2.1
//!   solves at each use, and what reaches the core is an ordinary
//!   `(x : A) → B`.
//! - **A lambda's binder type is optional**, and a `let`'s type is too. §2's
//!   whole point is that a checked term does not repeat what its type already
//!   says.
//! - **There is an annotation form.** `(e : A)` is §2's `Annot`, one of the two
//!   rules that switch modes, which is why it is a language feature rather than
//!   a hint.
//!
//! - **A universe need not say which one.** §1: "the surface never writes a
//!   level". A bare `Type` is §2.1's third metavariable site, and the level it
//!   stands at is solved by whatever the term is used as.
//!
//! # What it does not have
//!
//! No operators. `01-surface.md` §1.5 makes them *surface* spellings for terms
//! this type can already hold: `x == y` is `equal x y`, so the elaborator never
//! learns an operator table and the surface never has to explain one to this
//! crate. Method syntax survives as [`RawShape::Method`] for the one reason
//! given there: the name the call is to is not known until the receiver's type
//! is.

use std::sync::Arc;

use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Filling, Name};
use crate::kernel::visibility::{ModuleId, Visibility};

/// What a written arrow's binder is called.
///
/// A Π always binds, and nothing in `A → B` refers to the argument, so the
/// binder needs a spelling whose only job is to be printable. This is that
/// spelling — and it is also the mark that tells an arrow's domain from a
/// *declared parameter*, which is what `02-core-calculus.md` §1.3's
/// completeness rule counts. `fn walked(items: StaffItem) -> (Position<τ> →
/// Result<…>)` and `fn lifted(what: Nat, by: Nat) -> Nat` are the same shape of
/// type and different declarations; the second binder is named `by` in one and
/// unnamed in the other, and that is the whole of the difference.
///
/// A parameter an author actually spells `argument` is therefore invisible to
/// the rule. That is a miss and not a wrong answer — the call is elaborated as
/// it always was — and the style guide's naming rules are where a word this
/// plain gets discouraged.
pub const ARROW_BINDER: &str = "argument";

/// One binder of a raw telescope: a parameter, an index, or a constructor field.
#[derive(Clone, Debug)]
pub struct RawBinder {
    /// The binder's name, which later binders and the constructor's result may
    /// mention.
    pub name: Name,
    /// Its type.
    pub ty: Raw,
}

/// One constructor of a family, before elaboration.
///
/// It does **not** write its result type: §1.1 says a parameter is fixed across
/// the declaration, and a constructor that wrote `Motive B` in full could name
/// another family of the group — so the result is supplied by the declaration
/// and never written here. The rule is a property of the representation rather
/// than a check that could be forgotten.
#[derive(Clone, Debug)]
pub struct RawConstructor {
    /// Where it was written. A case may have no fields, so it is the only
    /// thing a diagnostic about the case itself can point at.
    pub origin: Origin,
    /// Its name, unqualified: the family qualifies it.
    pub name: Name,
    /// Whether `private` was written before it. All the cases or none of them —
    /// a family whose cases disagree is refused at its declaration.
    pub visibility: Visibility,
    /// Its arguments, read under the family names and the group's parameters.
    pub fields: Vec<RawBinder>,
}

/// One family of a declaration group, before elaboration.
#[derive(Clone, Debug)]
pub struct RawFamily {
    /// Its name.
    pub name: Name,
    /// Whether `private` was written before the declaration, hiding the *type*.
    /// Independent of its cases': a public type with private cases is the shape
    /// `01-surface.md` §1.3 exists for.
    pub visibility: Visibility,
    /// The index this family is declared to carry, if it declares one:
    /// `data Pc(n: Nat)` (§1.5).
    ///
    /// Beside the group's parameters rather than among them, because the two
    /// answer different questions and §1.5 spells them differently for that
    /// reason. A parameter says *what this is a type of*, is written in angle
    /// brackets, survives into the elaborated term, and is compared by §3's
    /// ordinary conversion. An index says *how many*, is written in
    /// parentheses, is decided by arithmetic, and is erased.
    ///
    /// Per family and not per group, because it is written at the family's own
    /// name — `data Pc(n: Nat)` — while the parameters are shared by every
    /// family a group declares. `None` is a family that takes no index, which
    /// is every family declared before §1.5 existed and every family since
    /// that had no reason to.
    ///
    /// At most one, because [`Term::indexed`](crate::Term::indexed) carries one
    /// index and the use-site grammar reads one expression. A longer telescope
    /// is a change to the term representation and to erasure.
    pub index: Option<RawBinder>,
    /// Its constructors.
    pub constructors: Vec<RawConstructor>,
}

/// A `data` declaration group, before elaboration.
///
/// A group rather than a single family because §1.1 checks strict positivity on
/// the whole declaration, and because mutual families share one recursor's
/// motives and methods. A single family is the group of one.
///
/// The universe each family lands in is absent, and deliberately: §1 says the
/// surface never writes a level, so it is computed as the join of the
/// constructors' field levels.
#[derive(Clone, Debug)]
pub struct RawData {
    /// Where the declaration was written.
    pub origin: Origin,
    /// The parameters, shared by every family in the group.
    pub params: Vec<RawBinder>,
    /// The families, in declaration order.
    pub families: Vec<RawFamily>,
}

/// A document's top-level definitions and instances, before elaboration.
///
/// A group rather than one definition at a time for [`RawData`]'s reason, one
/// word over: `02-core-calculus.md` §2.4 lets a body name a declaration written
/// later, so no definition here is finished until the group is. What the group
/// *is not* is an ordering — the order below is the order they were written,
/// and [`declare_program`](crate::declare_program) computes the order they are
/// elaborated in from what each one names.
///
/// # Why a namespaced definition is one of these
///
/// `impl Pitch { fn act(…) }` writes a definition called `Pitch.act`, and it
/// arrives here as an ordinary [`RawTopLevel`] with a dotted name. Nothing else
/// distinguishes it, which is the point: a function may write `x.act(i)` and
/// the definition answering it may call the document's functions, so the two
/// have to be ordered by one dependency analysis, and §2.4's forward reference
/// already is that analysis.
#[derive(Clone, Debug)]
pub struct RawProgram {
    /// The definitions, in the order the document wrote them.
    pub definitions: Vec<RawTopLevel>,
}

/// One top-level definition, before elaboration.
///
/// # Why the type is optional and the module is not
///
/// The type is optional because the surface makes it optional (`01-surface.md`
/// §1's `binding` writes `(":" type)?`), and a definition that wrote none is
/// inferred from its value.
///
/// The module is written out per definition rather than taken from the
/// declaring context, which is where [`RawData`] gets its. One group holds
/// everything a document can see — its imports' definitions and its own — so
/// there is no single module the group was written in, and a `private` name in
/// an imported library has to stay private to *that* library rather than to
/// whichever document is being checked.
#[derive(Clone, Debug)]
pub struct RawTopLevel {
    /// Where the definition was written.
    pub origin: Origin,
    /// The name it binds.
    pub name: Name,
    /// Whether `private` was written before it.
    pub visibility: Visibility,
    /// The module it was written in, when the caller numbers modules at all.
    pub module: Option<ModuleId>,
    /// The type it wrote, when it wrote one.
    pub ty: Option<Raw>,
    /// Its value.
    pub value: Raw,
}

/// A name and a value, and no type: what a `let` inside a declaration writes.
///
/// A separate type from [`RawTopLevel`] because it carries neither a type nor a
/// visibility — the enclosing declaration supplies both.
#[derive(Clone, Debug)]
pub struct RawDefinition {
    /// Where it was written.
    pub origin: Origin,
    /// The name it binds.
    pub name: Name,
    /// Its value.
    pub value: Raw,
}

/// One field of a raw record type or record literal.
#[derive(Clone, Debug)]
pub struct RawField {
    /// The field's name.
    pub name: Name,
    /// Its type in a record type, or its value in a literal.
    pub term: Raw,
}

/// One replaced field of a [`RawShape::Update`]: which field, and its new value.
///
/// The field is a *path* rather than an expression, so `p with { f(x).g = y }`
/// is not a term this type can hold. `01-surface.md` §1.2 fixes that at the
/// grammar and the reason is here too: an update rebuilds the record along the
/// path it names, and there is nothing to rebuild along a call.
#[derive(Clone, Debug)]
pub struct RawUpdate {
    /// Where the replacement was written.
    pub origin: Origin,
    /// The path from the record to the field being replaced, outermost first.
    /// Never empty.
    pub path: Vec<Name>,
    /// The field's new value.
    pub value: Raw,
}

/// A term before elaboration: what it is, and where it was written.
///
/// Shared through [`Arc`] for the same reason [`Term`](crate::Term) is:
/// elaboration reads a subterm more than once — a checked argument is read
/// against a domain that may itself mention it — and never mutates one.
#[derive(Clone, Debug)]
pub struct Raw {
    origin: Origin,
    shape: Arc<RawShape>,
}

/// What a raw term is.
#[derive(Clone, Debug)]
pub enum RawShape {
    /// A name, to be resolved against the binders in scope.
    Var(Name),
    /// A name in the *host's* namespaces — a declared family or a registered
    /// builtin — and nowhere else.
    ///
    /// The reader builds calls the author did not write: `music { c5/1 }`
    /// becomes an application of `sounded`, a list literal becomes `List.Cons`,
    /// a written product becomes `Pair.Both`. A name standing in one of those
    /// positions is the *reader's* word, so it is resolved the way the reader
    /// meant it and cannot be captured by a binding that happens to share its
    /// spelling. `sounded` is an ordinary word for an ordinary value, and an
    /// author who binds one is not thereby redefining what a music literal
    /// means — under [`Self::Var`] they would be, and the program would report
    /// a dependency cycle between two definitions that never named each other.
    ///
    /// A name the *author* wrote stays a [`Self::Var`], so a declaration still
    /// shadows a builtin of the same spelling where the author is the one
    /// naming it.
    Hosted(Name),
    /// A number at a counting family: `count` steps above that family's floor.
    ///
    /// The family is a *name*, resolved the way [`Self::Hosted`] is, because the
    /// reader is what knows which family a written number means — `Nat` in this
    /// language, and a core that guessed would either search for a unique
    /// counting family in scope or refuse to infer at all. Naming it costs the
    /// reader one word it already has.
    ///
    /// Not a [`Self::Lit`]: a literal is a closed value of a base type, and
    /// §5.8's base types are exactly the ones no rule takes apart. A number is
    /// the opposite — every rule takes it apart — so it elaborates to a term at
    /// a declared family and eliminates by that family's recursor.
    Numeral {
        /// The counting family the number stands at.
        family: Name,
        /// How far above its floor.
        count: u64,
    },
    /// One closed value of a base type, already built by whoever read the
    /// source: `3`, `"c"`, `1/4`.
    ///
    /// A literal arrives assembled rather than as text, because §5.8's payload
    /// is the host's and a core that parsed one would have to know what it
    /// parsed. It carries the base type it inhabits, so it infers.
    Lit(crate::kernel::base::Literal),
    /// `Type l`, at a level the writer states — or bare `Type`, whose level is
    /// §2.1's third metavariable site.
    Universe(Option<Sort>),
    /// `(x : A) → B`, at whichever [`Filling`] the binder has.
    Pi {
        /// Whether uses of the function must write this argument.
        filling: Filling,
        /// The binder's name.
        name: Name,
        /// `A`.
        domain: Raw,
        /// `B`, under the binder.
        codomain: Raw,
    },
    /// `T(i)` — `T` refined by the index expression `i` (§1.5).
    ///
    /// A *wrapper*, not a family index: `Row(12)` is ordinary `Row` under a
    /// index, so `family/` gains nothing, a value of `Row(12)` is a value
    /// of `Row`, and [`crate::kernel::quote`] drops the wrapper and reads back `Row`
    /// alone. What the index changes is which programs are accepted and
    /// nothing else.
    ///
    /// The index is an ordinary [`Raw`], because §1.5 gives an index no binder
    /// form of its own: an index variable is a parameter of index sort, bound
    /// and solved exactly as a type parameter is. What is *not* ordinary is
    /// what an index may say, and that is decided where two of them are
    /// compared rather than where one is written — [`crate::elaboration::convert`] reads an
    /// index position into the index language, or refuses it by naming the
    /// expression.
    Indexed {
        /// `T`, the type being refined.
        ty: Raw,
        /// `i`, the index it carries.
        index: Raw,
    },
    /// `λx. e`, with the binder's type written only when it is not already
    /// known.
    Lam {
        /// Which kind of binder this abstracts.
        filling: Filling,
        /// The binder's name.
        name: Name,
        /// `A`, when the author wrote one. Checking supplies it from the Π;
        /// inferring without one is §2.1's second metavariable site.
        domain: Option<Raw>,
        /// The body, under the binder.
        body: Raw,
    },
    /// `f a`, at whichever [`Filling`] of binder the argument fills.
    App {
        /// Which kind of binder this argument fills.
        filling: Filling,
        /// `f`.
        function: Raw,
        /// `a`.
        argument: Raw,
    },
    /// `f(a₁, …, aₙ)` as an author writes it — §1.3's **complete** call.
    ///
    /// Not sugar for iterated [`Self::App`], and the rule is the difference.
    /// §1.3: "an application supplies every declared parameter, and an
    /// under-applied call is a type error rather than a value." A spine cannot
    /// state that, because every prefix of one is itself an application and none
    /// of them knows it is the last. This holds the whole argument list, so
    /// what is left after all of them is the type of the *call*, and an explicit
    /// Π there is a parameter nobody wrote.
    ///
    /// Everything else about it is [`Self::App`]'s: the arguments are applied
    /// left to right, type parameters are filled before each one, and the
    /// filling rules are the ones already written. Only the last step is new.
    ///
    /// A function is still first class. §1.3 refuses partial *application*, not
    /// higher-order values — a bare name passed to a higher-order argument is a
    /// [`Self::Var`] and this rule never sees it. What the rule buys is §1.2's:
    /// an argument list stops being a place where a value silently becomes a
    /// function, which is the one kind of value that may not be stored.
    ///
    /// Written calls only. A reader that builds an application the author did
    /// not write — `music { c5/1 }` becoming `sounded(…)` — builds it complete
    /// by construction and uses [`Self::App`], because a refusal about *its*
    /// arity would be a sentence about the reading rather than about the source.
    Call {
        /// `f`.
        function: Raw,
        /// `a₁ … aₙ`, in the order written.
        arguments: Arc<[Raw]>,
    },
    /// `{ f₁ : A₁, …, fₙ : Aₙ }`, a telescope: each field's type is read under
    /// binders for the fields before it.
    RecordType(Arc<[RawField]>),
    /// `{ f₁ = e₁, …, fₙ = eₙ }`, in the order the record type declares.
    Record(Arc<[RawField]>),
    /// `x.m(…)` before its arguments — `01-surface.md` §1.5's method syntax.
    ///
    /// **Infers**, and holds no arguments: `x.m(y, z)` is this applied to `x`,
    /// `y`, and then `z` through the ordinary [`Self::App`] rule, so filling,
    /// parameter solving, and argument checking are the ones that were already
    /// written. What is new here is only *which name* the call is to: `Head.m`,
    /// where `Head` is the rigid head of the receiver's type. That question
    /// needs the receiver's type, which is why the surface cannot answer it and
    /// this shape exists.
    Method {
        /// `x`. Its inferred type's head names the namespace.
        receiver: Raw,
        /// `m`, unqualified. `Head.m` is the definition the lookup finds.
        method: Name,
    },
    /// `e.f`.
    Project {
        /// The record.
        record: Raw,
        /// The field's name.
        field: Name,
    },
    /// `e with { p⃗ = v, … }` — the record `e` with the fields those paths name
    /// replaced, and every other field carried over.
    ///
    /// **Infers.** The record being updated has a type already, and the answer
    /// has the same one, so there is nothing for a checking rule to supply. It
    /// elaborates to one `let` and one literal per path segment
    /// (`01-surface.md` §9.1): the `let` is what keeps the subject from being
    /// evaluated once per field it carries over.
    Update {
        /// The record being rebuilt.
        record: Raw,
        /// The replacements, in the order written. Never empty: `p with { }`
        /// says nothing `p` does not.
        updates: Arc<[RawUpdate]>,
    },
    /// `let x : A = v in e`, with `A` written only when inference needs it.
    Let {
        /// The binder's name.
        name: Name,
        /// `A`, when written. Without one the value is inferred.
        ty: Option<Raw>,
        /// `v`.
        value: Raw,
        /// `e`, under the binder.
        body: Raw,
    },
    /// `(e : A)` — §2's `Annot`, the rule an author re-enters checking mode by.
    Annot {
        /// `e`.
        term: Raw,
        /// `A`.
        ty: Raw,
    },
    /// `match e₁, …, eₘ { p⃗ → b, … }`, compiled to a case tree and then to the
    /// generated recursors (§6.2).
    ///
    /// **Checks only.** The motive a split builds *is* the goal, so a `match`
    /// with no goal has no motive — and inferring one from the first arm would
    /// make a program's type depend on the order its arms are written in.
    Match {
        /// The terms being scrutinized, left to right.
        subjects: Arc<[Raw]>,
        /// The arms, in the order written. Earlier arms win, and there is no
        /// fall-through: §6.2 declines Peyton Jones ch. 5's `FAIL`/fat-bar
        /// because an arm that could fail into the next one is what guards
        /// reintroduce and coverage cannot survive.
        arms: Arc<[RawArm]>,
    },
    /// `rec f : A = e`, a definition that may call itself.
    ///
    /// The whole form elaborates to a term of type `A`, with `f` in scope inside
    /// `e`. §2.4's rule is structural and the elaborator supplies it: a
    /// recursive call becomes the induction hypothesis the split that reached it
    /// already provides, and a call that has no hypothesis to become is
    /// [`Refusal::UncheckedRecursion`](crate::Refusal::UncheckedRecursion).
    Rec {
        /// The name the definition calls itself by.
        name: Name,
        /// `A`, always written: a recursive definition has no principal type to
        /// infer, since inferring one would need the definition it is defining.
        ty: Raw,
        /// `e`.
        body: Raw,
    },
}

/// One arm of a [`RawShape::Match`]: a pattern per subject, and a body.
#[derive(Clone, Debug)]
pub struct RawArm {
    /// One pattern per subject, in subject order.
    pub patterns: Vec<RawPattern>,
    /// What the arm answers.
    pub body: Raw,
}

/// A pattern, which nests.
///
/// §6.2 replaced the depth-one rule deliberately: an index is only worth having
/// if matching one constructor tells you something about another position, and a
/// pattern that cannot look through two constructors cannot say what a family is
/// indexed for.
///
/// **There is no literal pattern**, and that is §5.8's D1 rather than an
/// omission. A base type has no eliminator, so a column of one has nothing to
/// split on: the only pattern that may stand there is a catch-all, which
/// [`Self::Bind`] already is. Matching *against* a literal is decidable equality
/// — the host's own δ-builtin returning the host's own `Bool` — so a surface
/// `match k { "PitchLiteral" -> … ; _ -> … }` arrives here already desugared
/// into a match on that `Bool`. The core keeps the half of D1 it can enforce:
/// [`crate::elaboration::case`] refuses a [`Self::Constructor`] or [`Self::Record`] pattern
/// at a base-typed column, which is inertness.
#[derive(Clone, Debug)]
pub enum RawPattern {
    /// A name that matches anything and binds it. `_` is spelled as an ordinary
    /// binder whose name nothing refers to.
    Bind {
        /// Where it was written.
        origin: Origin,
        /// The name the body refers to it by.
        name: Name,
    },
    /// A constructor, with one sub-pattern per field.
    ///
    /// Parameters are **not** written: they are fixed by the scrutinee's type,
    /// so a pattern that repeated them would be asking the author to restate
    /// what the type already said.
    Constructor {
        /// Where it was written.
        origin: Origin,
        /// The constructor's name, either as the declaration gives it —
        /// `Nat.Succ` — or bare. A bare name is looked up in the namespace of
        /// the family the column being split belongs to, which is `01-surface.md`
        /// §1.3's rule and not a fallback: a pattern is checked against the
        /// subject's type, so the type is already known where the name is read.
        name: Name,
        /// One sub-pattern per field, in field order.
        fields: Vec<Self>,
    },
    /// A record, with a sub-pattern for the fields it names.
    ///
    /// It binds rather than selects: a record has one shape, so there is nothing
    /// to be exhaustive about and no `..` to write. A field the pattern does not
    /// name is simply not bound, which is what the absence of `..` means here.
    Record {
        /// Where it was written.
        origin: Origin,
        /// The fields it names, in the order written, each with the pattern its
        /// value stands against.
        fields: Vec<(Name, Self)>,
    },
}

impl RawPattern {
    /// Where the pattern was written.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        match *self {
            Self::Bind { origin, .. } | Self::Constructor { origin, .. } | Self::Record { origin, .. } => origin,
        }
    }
}

impl Raw {
    /// A raw term of shape `shape`, written at `origin`.
    #[must_use]
    pub fn new(origin: Origin, shape: RawShape) -> Self {
        Self {
            origin,
            shape: Arc::new(shape),
        }
    }

    /// Where it was written.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        self.origin
    }

    /// What it is.
    #[must_use]
    pub fn shape(&self) -> &RawShape {
        &self.shape
    }

    /// A name.
    #[must_use]
    pub fn var(origin: Origin, name: impl Into<Name>) -> Self {
        Self::new(origin, RawShape::Var(name.into()))
    }

    /// A name in the host's namespaces, written by the reader rather than by
    /// the author. See [`RawShape::Hosted`].
    #[must_use]
    pub fn hosted(origin: Origin, name: impl Into<Name>) -> Self {
        Self::new(origin, RawShape::Hosted(name.into()))
    }

    /// `count` steps above `family`'s floor. See [`RawShape::Numeral`].
    #[must_use]
    pub fn numeral(origin: Origin, family: impl Into<Name>, count: u64) -> Self {
        Self::new(
            origin,
            RawShape::Numeral {
                family: family.into(),
                count,
            },
        )
    }

    /// One closed value of a base type, already built.
    #[must_use]
    pub fn lit(origin: Origin, literal: crate::kernel::base::Literal) -> Self {
        Self::new(origin, RawShape::Lit(literal))
    }

    /// `Type level`, at a level the caller names.
    #[must_use]
    pub fn universe(origin: Origin, level: Sort) -> Self {
        Self::new(origin, RawShape::Universe(Some(level)))
    }

    /// `Type`, at whatever level the surrounding term determines.
    #[must_use]
    pub fn any_universe(origin: Origin) -> Self {
        Self::new(origin, RawShape::Universe(None))
    }

    /// `(name : domain) → codomain`.
    #[must_use]
    pub fn pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::binder(origin, Filling::Written, name, domain, codomain)
    }

    /// `domain → codomain`, the arrow with nothing bound.
    ///
    /// A Π always binds, so the binder still needs a spelling and gets
    /// [`ARROW_BINDER`]. Separate from [`Self::pi`] because the two say
    /// different things and one of them is load-bearing: a *named* binder is a
    /// parameter the author declared, and `02-core-calculus.md` §1.3's
    /// completeness rule is about exactly those. `walked(items: StaffItem) ->
    /// (Position<WrittenTime> -> Result<…>)` declares one parameter and returns
    /// a function; a two-parameter declaration has the same type and a second
    /// name in it.
    #[must_use]
    pub fn arrow(origin: Origin, domain: Self, codomain: Self) -> Self {
        Self::binder(origin, Filling::Written, ARROW_BINDER, domain, codomain)
    }

    /// `(name : domain) → codomain`, binding a type parameter §2.1 solves.
    #[must_use]
    pub fn parameter_pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::binder(origin, Filling::Parameter, name, domain, codomain)
    }

    fn binder(origin: Origin, filling: Filling, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::new(
            origin,
            RawShape::Pi {
                filling,
                name: name.into(),
                domain,
                codomain,
            },
        )
    }

    /// `λname. body`, for a binder whose type the checking type supplies.
    #[must_use]
    pub fn lam(origin: Origin, name: impl Into<Name>, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Lam {
                filling: Filling::Written,
                name: name.into(),
                domain: None,
                body,
            },
        )
    }

    /// `λ(name : domain). body`.
    #[must_use]
    pub fn annotated_lam(origin: Origin, name: impl Into<Name>, domain: Self, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Lam {
                filling: Filling::Written,
                name: name.into(),
                domain: Some(domain),
                body,
            },
        )
    }

    /// `λname. body`, abstracting a type parameter.
    #[must_use]
    pub fn parameter_lam(origin: Origin, name: impl Into<Name>, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Lam {
                filling: Filling::Parameter,
                name: name.into(),
                domain: None,
                body,
            },
        )
    }

    /// Whether this is a form the elaborator can only *check*: an unannotated
    /// λ, a record literal, a `match`, a `rec` — the shapes
    /// [`Refusal::Uninferable`](crate::Refusal::Uninferable) names.
    ///
    /// The application walk asks it when an argument's slot still mentions an
    /// unsolved meta (§2.1's direction rule): inferring the argument to teach
    /// the meta is right for everything inference has a rule for, but a
    /// checking-only form has to be checked against the slot as it stands —
    /// the slot's Pi descends around it, and an annotation inside is what the
    /// meta is solved from.
    pub(crate) fn checks_only(&self) -> bool {
        matches!(
            self.shape(),
            RawShape::Record(_) | RawShape::Match { .. } | RawShape::Rec { .. } | RawShape::Lam { .. }
        )
    }

    /// Whether checking this against a still-unsolved domain would *teach*
    /// that domain something.
    ///
    /// True of exactly one form: a λ whose binder the author annotated.
    /// [`Elaborator::check`](crate::elaboration::elab::Elaborator)'s λ rule makes a written
    /// annotation agree with the domain it is checked against, so such an
    /// argument solves the slot rather than waiting on it — and the
    /// application walk skips it when it decides what to defer. Everything
    /// else [`Self::checks_only`] names — a record, a `match`, a `rec`, a bare
    /// λ — reads its types *from* the slot and has none to give back.
    pub(crate) fn annotates_its_binder(&self) -> bool {
        matches!(self.shape(), RawShape::Lam { domain: Some(_), .. })
    }

    /// `ty(index)` — `ty` refined by an index expression (§1.5).
    #[must_use]
    pub fn indexed(origin: Origin, ty: Self, index: Self) -> Self {
        Self::new(origin, RawShape::Indexed { ty, index })
    }

    /// `function argument`.
    #[must_use]
    pub fn app(origin: Origin, function: Self, argument: Self) -> Self {
        Self::new(
            origin,
            RawShape::App {
                filling: Filling::Written,
                function,
                argument,
            },
        )
    }

    /// `function(a₁, …, aₙ)` as the author wrote it, which is the one form
    /// §1.3's completeness rule applies to.
    ///
    /// A reader that builds an application nobody wrote uses [`Self::app`]; see
    /// [`RawShape::Call`] for why the two are told apart.
    #[must_use]
    pub fn call(origin: Origin, function: Self, arguments: impl IntoIterator<Item = Self>) -> Self {
        Self::new(
            origin,
            RawShape::Call {
                function,
                arguments: arguments.into_iter().collect(),
            },
        )
    }

    /// `function argument`, where the argument is a type parameter the use
    /// site writes rather than one §2.1 solves.
    #[must_use]
    pub fn parameter_app(origin: Origin, function: Self, argument: Self) -> Self {
        Self::new(
            origin,
            RawShape::App {
                filling: Filling::Parameter,
                function,
                argument,
            },
        )
    }

    /// `{ … }` as a record type, from `(name, type)` pairs in telescope order.
    #[must_use]
    pub fn record_type<'a>(origin: Origin, fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::new(origin, RawShape::RecordType(collect(fields)))
    }

    /// `{ … }` as a record literal, from `(name, value)` pairs.
    #[must_use]
    pub fn record<'a>(origin: Origin, fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::new(origin, RawShape::Record(collect(fields)))
    }

    /// `x.m` — the definition `Head.m`, where `Head` heads `x`'s type.
    ///
    /// Arguments are applied to the result, so `x.m(y)` is
    /// `Raw::app(origin, Raw::method(origin, x, "m"), y)`.
    #[must_use]
    pub fn method(origin: Origin, receiver: Self, method: impl Into<Name>) -> Self {
        Self::new(
            origin,
            RawShape::Method {
                receiver,
                method: method.into(),
            },
        )
    }

    /// `record.field`.
    #[must_use]
    pub fn project(origin: Origin, record: Self, field: impl Into<Name>) -> Self {
        Self::new(
            origin,
            RawShape::Project {
                record,
                field: field.into(),
            },
        )
    }

    /// `record with { path = value, … }`, one path per replacement.
    #[must_use]
    pub fn update<'a>(origin: Origin, record: Self, updates: impl IntoIterator<Item = (&'a [&'a str], Self)>) -> Self {
        Self::new(
            origin,
            RawShape::Update {
                record,
                updates: updates
                    .into_iter()
                    .map(|(path, value)| RawUpdate {
                        origin: value.origin(),
                        path: path.iter().map(|segment| Arc::from(*segment)).collect(),
                        value,
                    })
                    .collect(),
            },
        )
    }

    /// `let name = value in body`, with the value's type inferred.
    #[must_use]
    pub fn bind(origin: Origin, name: impl Into<Name>, value: Self, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Let {
                name: name.into(),
                ty: None,
                value,
                body,
            },
        )
    }

    /// `let name : ty = value in body`.
    #[must_use]
    pub fn annotated_bind(origin: Origin, name: impl Into<Name>, ty: Self, value: Self, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Let {
                name: name.into(),
                ty: Some(ty),
                value,
                body,
            },
        )
    }

    /// `(term : ty)`.
    #[must_use]
    pub fn annot(origin: Origin, term: Self, ty: Self) -> Self {
        Self::new(origin, RawShape::Annot { term, ty })
    }

    /// `match subjects… { arms… }`.
    #[must_use]
    pub fn match_on(origin: Origin, subjects: impl IntoIterator<Item = Self>, arms: Vec<RawArm>) -> Self {
        Self::new(
            origin,
            RawShape::Match {
                subjects: subjects.into_iter().collect(),
                arms: Arc::from(arms),
            },
        )
    }

    /// `rec name : ty = body`.
    #[must_use]
    pub fn rec(origin: Origin, name: impl Into<Name>, ty: Self, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Rec {
                name: name.into(),
                ty,
                body,
            },
        )
    }
}

impl RawPattern {
    /// A pattern that matches anything and binds it.
    #[must_use]
    pub fn bind(origin: Origin, name: impl Into<Name>) -> Self {
        Self::Bind {
            origin,
            name: name.into(),
        }
    }

    /// A constructor pattern, with one sub-pattern per field.
    #[must_use]
    pub fn constructor(origin: Origin, name: impl Into<Name>, fields: impl IntoIterator<Item = Self>) -> Self {
        Self::Constructor {
            origin,
            name: name.into(),
            fields: fields.into_iter().collect(),
        }
    }

    /// A record pattern binding the fields it names.
    #[must_use]
    pub fn record<'a>(origin: Origin, fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::Record {
            origin,
            fields: fields
                .into_iter()
                .map(|(name, pattern)| (Arc::from(name), pattern))
                .collect(),
        }
    }
}

fn collect<'a>(fields: impl IntoIterator<Item = (&'a str, Raw)>) -> Arc<[RawField]> {
    fields
        .into_iter()
        .map(|(name, term)| RawField {
            name: Arc::from(name),
            term,
        })
        .collect()
}
