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
//! No traits, no operators, no method syntax — prompt 137.

use std::sync::Arc;

use crate::level::Level;
use crate::origin::Origin;
use crate::term::{Name, Plicity};
use crate::visibility::Visibility;

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
    /// The families, in declaration order.
    pub families: Vec<RawFamily>,
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
    /// `Type l`, at a level the writer states — or bare `Type`, whose level is
    /// §2.1's third metavariable site.
    Universe(Option<Level>),
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
