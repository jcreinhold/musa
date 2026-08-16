//! The raw term: what elaboration reads, and the crate's one boundary with
//! syntax.
//!
//! `docs/rules/language/02-core-calculus.md` §2 elaborates a *surface* term into
//! a core term. This type is what stands in for "surface" here, and it is
//! deliberately **surface-independent**: it knows about names, plicity, and
//! annotations, and it knows nothing about pitches, bars, or `.musa` grammar.
//! That is what lets `musa-core` stay a leaf, and what lets the elaborator's
//! tests be written without a parser.
//!
//! # What separates it from [`Term`](crate::Term)
//!
//! Four things, and each is a job elaboration does:
//!
//! - **Variables are names, not indices.** Resolving a name to a de Bruijn index
//!   is elaboration's, so a raw term never has to be written under a mental
//!   model of how deep it is.
//! - **Binders may be implicit.** §1: "plicity is not in the core". A raw binder
//!   marked [`Plicity::Implicit`] produces a metavariable at each use, and what
//!   reaches the core is an ordinary `(x : A) → B`.
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
//! No operators and no method syntax. `10-traits.md` §5 and §6 make both of them
//! *surface* spellings for terms this type can already hold: `x == y` is
//! `Eq.equal x y` and `x.m(y)` is a name and an application, so the elaborator
//! never learns an operator table and the surface never has to explain one to
//! this crate.

use std::sync::Arc;

use crate::level::Level;
use crate::origin::Origin;
use crate::term::{Name, Plicity};
use crate::visibility::{ModuleId, Visibility};

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
/// It does **not** write its result type. §1.1 says a parameter is fixed across
/// the declaration and an index is chosen per constructor, and a constructor that
/// wrote `Vec A n` in full could write `Vec B n` instead — so the parameters are
/// supplied by the declaration and only the indices are written here. The rule is
/// then a property of the representation rather than a check that could be
/// forgotten.
#[derive(Clone, Debug)]
pub struct RawConstructor {
    /// Where it was written. A case may have no fields and no indices, so it is
    /// the only thing a diagnostic about the case itself can point at.
    pub origin: Origin,
    /// Its name, unqualified: the family qualifies it.
    pub name: Name,
    /// Whether `private` was written before it. All the cases or none of them —
    /// a family whose cases disagree is refused at its declaration.
    pub visibility: Visibility,
    /// Its arguments, read under the family names and the group's parameters.
    pub fields: Vec<RawBinder>,
    /// The index arguments its result chooses, in the family's index order, read
    /// under those binders and its own fields.
    pub indices: Vec<Raw>,
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
    /// Its indices, read under the family names and the group's parameters.
    pub indices: Vec<RawBinder>,
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
    /// Its `where` clause, read under the parameters.
    ///
    /// Each entry becomes one more parameter, appended after [`Self::params`]
    /// and standing at the dictionary's type — `01-surface.md` §1.2's "requires
    /// the constraint at every construction and carries it to every reader",
    /// which is what a parameter every use site has to fill already means. They
    /// are parameters and not a list beside them because every count in this
    /// crate — [`Group::params`](crate::Group::params) and the arithmetic that
    /// splits a spine at it — is then counting the same thing it counted before.
    pub context: Vec<RawConstraint>,
    /// The families, in declaration order.
    pub families: Vec<RawFamily>,
}

/// A document's top-level definitions, before elaboration.
///
/// A group rather than one definition at a time for [`RawData`]'s reason, one
/// word over: `02-core-calculus.md` §2.4 lets a body name a declaration written
/// later, so no definition here is finished until the group is. What the group
/// *is not* is an ordering — the order below is the order they were written,
/// and [`declare_program`](crate::declare_program) computes the order they are
/// elaborated in from what each one names.
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

/// One constraint, before elaboration: a trait applied to type arguments.
///
/// `Eq<A>` in a `where` clause, in a trait's own context, or as an `impl`'s
/// head. One type rather than three, because `10-traits.md` §4's lookup is the
/// same question in all three positions — the difference is what is done with
/// the dictionary, not how the constraint is read.
#[derive(Clone, Debug)]
pub struct RawConstraint {
    /// Where it was written.
    pub origin: Origin,
    /// The trait's name.
    pub name: Name,
    /// One argument per trait parameter, in declaration order.
    pub args: Vec<Raw>,
}

/// One method of a trait declaration.
///
/// [`RawMethod::body`] is what separates §1's two kinds. A method **declared
/// with `;`** is required: it is a field of the dictionary and an impl supplies
/// it. A method **declared with a block** is derived: it is an ordinary function
/// of the dictionary, defined once here, and an impl may not replace it. That
/// is why §9 can refuse specialization as a *mechanism* rather than as a rule —
/// there is no overridable definition to specialize.
///
/// A method may quantify over parameters of its **own** and require its own
/// constraints — `fn map<D, B>(source: C, f: A -> B) -> D where Buildable<D, B>`
/// (`01-surface.md` §1.6). They are separate fields rather than binders written
/// into [`RawMethod::ty`] because the surface writes them separately, and
/// because a use site has to fill them by two different mechanisms: a parameter
/// by unification and a constraint by §4's lookup.
#[derive(Clone, Debug)]
pub struct RawMethod {
    /// Where it was written.
    pub origin: Origin,
    /// Its name, unqualified: the trait qualifies it.
    pub name: Name,
    /// The type parameters it quantifies over, read under the trait's own.
    ///
    /// Implicit at every use: `xs.map(f)` writes neither `D` nor `B`.
    pub params: Vec<RawBinder>,
    /// Its own constraints, read under the trait's parameters and then its own.
    ///
    /// Only a **derived** method may have them. A required method is a field of
    /// the dictionary, and a field whose type demanded a dictionary the impl
    /// never wrote would be a second resolution site inside the first — see
    /// [`Refusal::ConstrainedField`](crate::Refusal::ConstrainedField).
    pub context: Vec<RawConstraint>,
    /// Its type, read under the trait's parameters and then its own.
    pub ty: Raw,
    /// Its definition, when the trait derives it.
    ///
    /// Read under the trait's parameters, a binder for the dictionary, the
    /// trait's own methods, this method's parameters, and a binder per
    /// constraint — a derived method is written in terms of the required ones,
    /// which is the whole reason for the form.
    pub body: Option<Raw>,
}

/// A `trait` declaration, before elaboration.
#[derive(Clone, Debug)]
pub struct RawTrait {
    /// Where it was written.
    pub origin: Origin,
    /// Its name.
    pub name: Name,
    /// Whether `private` was written before it (`01-surface.md` §1.3).
    pub visibility: Visibility,
    /// Its parameters. The first is the head every instance is keyed on; §1
    /// refuses a trait none of whose parameters its head determines.
    pub params: Vec<RawBinder>,
    /// Its own constraints — `trait Ord<A> where Eq<A>` — read under the
    /// parameters. Each becomes a field of the dictionary, so reaching `Eq`
    /// from `Ord` is one projection and never a second lookup.
    pub context: Vec<RawConstraint>,
    /// Its methods, in declaration order.
    pub methods: Vec<RawMethod>,
}

/// An `impl` declaration, before elaboration.
#[derive(Clone, Debug)]
pub struct RawImpl {
    /// Where it was written.
    pub origin: Origin,
    /// The trait being implemented.
    pub name: Name,
    /// The type variables the instance abstracts over — `impl<A> Eq<List<A>>`.
    pub params: Vec<RawBinder>,
    /// The head arguments, read under those parameters.
    pub args: Vec<Raw>,
    /// Its `where` clause, read under the same parameters. §4's measure is
    /// checked here, at the declaration, and never at a use site.
    pub context: Vec<RawConstraint>,
    /// The required methods it supplies, in any order: they are matched to the
    /// trait's fields by name, since an impl that had to repeat the
    /// declaration's order would be restating what the trait already said.
    pub methods: Vec<RawDefinition>,
}

/// One method an `impl` supplies: a name and a value, and no type.
///
/// A separate type from [`RawMethod`] rather than that one with its type field
/// unused, because the difference is the point: a trait *declares* a method and
/// so writes its type, and an impl *defines* one and so writes only what it is.
/// The type comes from the trait's dictionary field, which is what makes an
/// instance's methods checked against the declaration rather than merely beside
/// it — and what leaves an impl no place to write a type that disagrees.
#[derive(Clone, Debug)]
pub struct RawDefinition {
    /// Where it was written.
    pub origin: Origin,
    /// Which method it defines.
    pub name: Name,
    /// Its value, read under the instance's parameters and its `where`
    /// dictionaries.
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
    /// One closed value of a base type, already built by whoever read the
    /// source: `3`, `"c"`, `1/4`.
    ///
    /// A literal arrives assembled rather than as text, because §5.8's payload
    /// is the host's and a core that parsed one would have to know what it
    /// parsed. It carries the base type it inhabits, so it infers.
    Lit(crate::base::Literal),
    /// `Type l`, at a level the writer states — or bare `Type`, whose level is
    /// §2.1's third metavariable site.
    Universe(Option<Level>),
    /// `[Class a⃗] → B` — the binder `01-surface.md` §1.4's `where` clause
    /// elaborates to.
    ///
    /// Its own shape rather than a [`Plicity`] on [`Self::Pi`], because what a
    /// `where` writes is a *raw* constraint: a trait name and raw arguments,
    /// which nothing has resolved yet. [`Plicity::Constraint`] carries an
    /// elaborated [`Constraint`](crate::Trait), and turning one into the other
    /// is what elaboration does here. There is no domain to write either — the
    /// dictionary's type is the trait applied to those arguments, so writing it
    /// would be writing the answer.
    ConstrainedPi {
        /// The constraint the binder answers.
        constraint: RawConstraint,
        /// `B`, under the dictionary binder.
        codomain: Raw,
    },
    /// `(x : A) → B`, or `{x : A} → B` when the binder is implicit.
    Pi {
        /// Whether uses of the function must write this argument.
        plicity: Plicity,
        /// The binder's name.
        name: Name,
        /// `A`.
        domain: Raw,
        /// `B`, under the binder.
        codomain: Raw,
    },
    /// `λx. e`, with the binder's type written only when it is not already
    /// known.
    Lam {
        /// Which kind of binder this abstracts.
        plicity: Plicity,
        /// The binder's name.
        name: Name,
        /// `A`, when the author wrote one. Checking supplies it from the Π;
        /// inferring without one is §2.1's second metavariable site.
        domain: Option<Raw>,
        /// The body, under the binder.
        body: Raw,
    },
    /// `f a`, or `f {a}` when the argument fills an implicit binder.
    App {
        /// Which kind of binder this argument fills.
        plicity: Plicity,
        /// `f`.
        function: Raw,
        /// `a`.
        argument: Raw,
    },
    /// `{ f₁ : A₁, …, fₙ : Aₙ }`, a telescope: each field's type is read under
    /// binders for the fields before it.
    RecordType(Arc<[RawField]>),
    /// `{ f₁ = e₁, …, fₙ = eₙ }`, in the order the record type declares.
    Record(Arc<[RawField]>),
    /// `x.m(…)` before its arguments — `10-traits.md` §6's method syntax.
    ///
    /// **Infers**, and holds no arguments: `x.m(y, z)` is this applied to `y`
    /// and then to `z` through the ordinary [`Self::App`] rule, so plicity,
    /// implicit insertion, and argument checking are the ones that were already
    /// written. What is new here is only *which name* the call is to, and that
    /// question needs the receiver's type, which is why the surface cannot
    /// answer it and this shape exists.
    Method {
        /// `x`. Its inferred type's head is what the lookup is keyed on.
        receiver: Raw,
        /// `m`, unqualified. The trait it belongs to is what resolution finds.
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
    /// `Id A x y`.
    Id {
        /// `A`.
        ty: Raw,
        /// `x`.
        left: Raw,
        /// `y`.
        right: Raw,
    },
    /// `refl x`.
    Refl(Raw),
    /// `J A x P p y e`, the identity type's dependent eliminator.
    J {
        /// `A`.
        ty: Raw,
        /// `x`.
        from: Raw,
        /// `P : (y : A) → Id A x y → Type l`.
        motive: Raw,
        /// `p : P x (refl x)`.
        base: Raw,
        /// `y`.
        to: Raw,
        /// `e : Id A x y`.
        proof: Raw,
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
    /// **Checks only.** The motive a split builds is the goal type abstracted
    /// over the subject, so a `match` with no goal has nothing to abstract —
    /// and inferring one from the first arm would make a program's type depend
    /// on the order its arms are written in.
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
    /// `e`. §2.4's measure is structural and the elaborator supplies it: a
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
/// [`crate::case`] refuses a [`Self::Constructor`] or [`Self::Record`] pattern
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

    /// One closed value of a base type, already built.
    #[must_use]
    pub fn lit(origin: Origin, literal: crate::base::Literal) -> Self {
        Self::new(origin, RawShape::Lit(literal))
    }

    /// `Type level`, at a level the caller names.
    #[must_use]
    pub fn universe(origin: Origin, level: Level) -> Self {
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
        Self::binder(origin, Plicity::Explicit, name, domain, codomain)
    }

    /// `{name : domain} → codomain`.
    #[must_use]
    pub fn implicit_pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::binder(origin, Plicity::Implicit, name, domain, codomain)
    }

    /// `[constraint] → codomain` — a `where` clause's binder.
    ///
    /// There is no `constrained_lam` to go with it, and there is not meant to
    /// be: `02-core-calculus.md` §2 wraps a term checked against a binder the
    /// author did not write in the λ it needs, and a constraint binder is that
    /// sentence a third time. A caller writes the *signature* and the core
    /// writes the abstraction.
    #[must_use]
    pub fn constrained_pi(origin: Origin, constraint: RawConstraint, codomain: Self) -> Self {
        Self::new(origin, RawShape::ConstrainedPi { constraint, codomain })
    }

    fn binder(origin: Origin, plicity: Plicity, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::new(
            origin,
            RawShape::Pi {
                plicity,
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
                plicity: Plicity::Explicit,
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
                plicity: Plicity::Explicit,
                name: name.into(),
                domain: Some(domain),
                body,
            },
        )
    }

    /// `λ{name}. body`, abstracting an implicit binder.
    #[must_use]
    pub fn implicit_lam(origin: Origin, name: impl Into<Name>, body: Self) -> Self {
        Self::new(
            origin,
            RawShape::Lam {
                plicity: Plicity::Implicit,
                name: name.into(),
                domain: None,
                body,
            },
        )
    }

    /// `function argument`.
    #[must_use]
    pub fn app(origin: Origin, function: Self, argument: Self) -> Self {
        Self::new(
            origin,
            RawShape::App {
                plicity: Plicity::Explicit,
                function,
                argument,
            },
        )
    }

    /// `function {argument}` — an implicit supplied at the use site rather than
    /// inserted.
    #[must_use]
    pub fn implicit_app(origin: Origin, function: Self, argument: Self) -> Self {
        Self::new(
            origin,
            RawShape::App {
                plicity: Plicity::Implicit,
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

    /// `x.m` — the method `m` of whichever trait answers for `x`'s type.
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

    /// `Id ty left right`.
    #[must_use]
    pub fn identity(origin: Origin, ty: Self, left: Self, right: Self) -> Self {
        Self::new(origin, RawShape::Id { ty, left, right })
    }

    /// `refl value`.
    #[must_use]
    pub fn refl(origin: Origin, value: Self) -> Self {
        Self::new(origin, RawShape::Refl(value))
    }

    /// `J ty from motive base to proof`.
    #[must_use]
    pub fn jay(origin: Origin, ty: Self, from: Self, motive: Self, base: Self, to: Self, proof: Self) -> Self {
        Self::new(
            origin,
            RawShape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
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
