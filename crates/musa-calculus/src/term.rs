//! Core terms.
//!
//! One syntactic category (`docs/rules/language/02-core-calculus.md` §1): types
//! are terms, so [`Shape`] is the whole language and there is no second grammar
//! to keep in step with it.
//!
//! **Terms are de Bruijn-*indexed*; values are de Bruijn-*levelled*.** That
//! split is the whole reason both [`Index`] and [`DbLevel`] exist as separate
//! types rather than as two `u32`s. An index counts binders *outward from the
//! use site*, so α-equivalence on terms is structural equality — which is what
//! makes conversion an `==` after quoting. A level counts binders *inward from
//! the empty context*, so a value can name a fresh variable during quotation
//! without renaming anything already built. Mixing the two is the classic bug
//! in this construction, it type-checks in every language that spells both
//! `usize`, and it does not type-check here.
//!
//! **A term is a shape and an [`Origin`].** §7 makes provenance a property of
//! the representation, and it is carried by the wrapper rather than by every
//! variant so that there is exactly one place a term's origin is read and
//! written. Two consequences worth knowing before reading further: the [`Arc`]
//! lives in the wrapper, so a shape's children are [`Term`] and not
//! `Arc<Term>`; and [`Term`]'s `PartialEq` compares shapes only, because §7
//! says origins are not part of conversion.

use std::sync::Arc;

use crate::class::Constraint;
use crate::family::Constant;
use crate::level::Level;
use crate::origin::Origin;

/// A binder's written name, and a record field's name.
///
/// Binder names carry no meaning: α-equivalence is decided by [`Index`], so two
/// terms differing only in a binder name are equal. What names are for is the
/// diagnostic prompt 134 prints and the source the formatter writes back. Field
/// names *are* meaningful — a record is its fields, and [`Shape::Project`] finds
/// one by name.
pub type Name = Arc<str>;

/// How a use site fills a binder.
///
/// A property of a *binder*. It rides on [`Shape::Pi`] and **no core rule reads
/// it** (§1, amended at prompt 134): there is one function type, conversion
/// ignores this field, and evaluation and quotation only carry it along. It is
/// here rather than beside the type because a Π reached through a record
/// projection, through δ, or through substituting a type variable has been
/// through the semantic domain, and elaboration still has to be able to ask the
/// binder it found how a use site fills it.
///
/// Three arms because there are three answers, and none of them is the
/// implicit-argument insertion this type was once named for: an author writes
/// the argument, or §2.1's first-order matching solves it, or `10-traits.md`
/// §4's lookup answers it. The elaborator's own walk spells the same three —
/// see `elab::spine`'s `Slot`.
#[derive(Clone, Debug)]
pub enum Filling {
    /// Written at every use.
    Written,
    /// A type parameter, solved at every use by §2.1's first-order matching of
    /// the written arguments' inferred types against the parameter types.
    ///
    /// Not a metavariable that survives its call: a parameter the match does
    /// not determine is [`crate::Refusal::Unsolved`] at the call, naming the
    /// parameter, and the author writes the argument. A use site may write it
    /// too, which is the one place a filling appears on an application.
    Parameter,
    /// Answered at every use by `10-traits.md` §4's lookup, and never written.
    ///
    /// Never appears in a [`Raw`](crate::Raw): a surface `where` clause is
    /// [`RawShape::ConstrainedPi`](crate::RawShape), whose constraint is
    /// unelaborated, and elaboration is what turns one into the other.
    ///
    /// The constraint travels **on the binder** rather than in a table beside
    /// the definition, because a definition is a value: `same` may be passed,
    /// stored, or returned, and at that use site there is no name to look up
    /// and only the type is in hand. It cannot be recovered from the domain
    /// either — [`crate::Trait`]'s dictionary is a closed `λp⃗. { … }`, so
    /// `Eq A` β-reduces to a record type and the trait's name is gone by the
    /// time anything asks.
    Constraint(Arc<Constraint>),
}

/// Two binders agree when a use site fills them the same way.
///
/// Written rather than derived for [`Term`]'s reason one level down: two
/// constraint binders that demand the same instance are the same binder
/// wherever they were written, so the origin is excluded here as it is there.
impl PartialEq for Filling {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Written, Self::Written) | (Self::Parameter, Self::Parameter) => true,
            (Self::Constraint(left), Self::Constraint(right)) => {
                Arc::ptr_eq(left, right) || (left.class == right.class && left.args == right.args)
            }
            _ => false,
        }
    }
}

impl Eq for Filling {}

/// A de Bruijn index: how many binders out from its use site a variable's
/// binder is. `Index(0)` is the nearest enclosing binder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index(pub u32);

/// A de Bruijn level: how many binders in from the empty context a variable's
/// binder is. `DbLevel(0)` is the outermost.
///
/// Levels appear only inside values and inside quotation, never in a [`Term`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbLevel(pub u32);

impl DbLevel {
    /// The index that names this level from inside a scope holding `depth`
    /// binders.
    ///
    /// `None` when the level is not in scope at that depth, which is a caller
    /// defect rather than a program error — quotation only ever asks about
    /// levels it created.
    pub(crate) const fn to_index(self, depth: u32) -> Option<Index> {
        match depth.checked_sub(self.0) {
            Some(0) | None => None,
            Some(back) => Some(Index(back.saturating_sub(1))),
        }
    }
}

/// One field of a record type or a record value.
///
/// A record *type*'s fields form a telescope: the type of a later field may
/// mention the values of earlier ones, which is why the order is part of the
/// type and why [`Shape::RecordType`] is a slice rather than a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// The field's name, which is how it is projected.
    pub name: Name,
    /// A field type in a [`Shape::RecordType`], or a field value in a
    /// [`Shape::Record`].
    pub term: Term,
}

/// A core term: what it is, and where it came from.
///
/// Shared through [`Arc`] rather than owned through [`Box`]: normalization by
/// evaluation copies subterms into closures constantly and never mutates one,
/// so sharing is the representation's job and a deep clone at every binder
/// would be pure waste.
#[derive(Clone, Debug)]
pub struct Term {
    origin: Origin,
    shape: Arc<Shape>,
}

/// Equality is α-equality, and α-equality does not look at provenance.
///
/// §7: "Two terms with different origins and the same normal form are
/// convertible; otherwise provenance would change what a program means, and a
/// compiler that type-checked differently after a file was moved would be the
/// result." Written here rather than derived so that the exclusion is a property
/// of the type instead of something every comparison site has to remember.
impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        // Two terms that share a shape are equal without walking it, which is
        // the common case after quotation: η-expansion and projection hand the
        // same subterm to both sides of a comparison constantly.
        Arc::ptr_eq(&self.shape, &other.shape) || self.shape == other.shape
    }
}

impl Eq for Term {}

/// What a term is.
///
/// Separate from [`Term`] so that the origin has one home. A shape's children
/// are `Term`s, each carrying its own origin, which is what makes §7's
/// preservation clause checkable node by node.
#[derive(Clone, Debug)]
pub enum Shape {
    /// A placeholder for an argument the instantiation walk has not yet
    /// solved — see [`crate::meta::Hole`].
    Hole(crate::meta::Hole),
    /// A variable, named by how many binders out its binder is.
    Var(Index),
    /// A declared constant: an inductive family, one of its constructors, or its
    /// generated recursor (§1.1).
    ///
    /// One variant rather than three saturated forms, because a family, a
    /// constructor, and a recursor are all just *applied* — [`Self::App`] already
    /// says what an argument is, and three spine-carrying variants would say it
    /// three more times while making partial application a different term.
    Const(Constant),
    /// A use of a top-level definition (§2.4).
    ///
    /// Separate from [`Self::Const`] rather than a fourth
    /// [`Role`](crate::family::Role), because a declared constant is *rigid* —
    /// a family, a constructor, and a recursor each stand for themselves — and
    /// a definition is the one global name that δ unfolds. One node whatever
    /// the definition is: see [`crate::program`] for why a use is a reference
    /// rather than a copy of the body.
    Def(crate::program::Def),
    /// A closed value of a counting family, written as a count rather than as
    /// that many applications of its step constructor.
    ///
    /// A *representation*, not a new kind of value: it is definitionally the
    /// tower it stands for, and [`crate::family::Counting`] is the shape
    /// condition that makes that true. Distinct from [`Self::Lit`] because the
    /// two mean opposite things — a base literal is opaque and nothing in this
    /// crate takes one apart, while a numeral is taken apart by every
    /// elimination at its family, one level at a time.
    Numeral(crate::family::Numeral),
    /// A base type: §5.8's conservative extension, registered by the host and
    /// inert here. It has no constructor and no eliminator, so no rule in this
    /// crate ever takes one apart — which is D1 stated as a representation.
    Base(crate::base::Base),
    /// A closed value of a base type, opaque to this crate.
    Lit(crate::base::Literal),
    /// A compiler-owned operation. Rigid until its arguments are literals, at
    /// which point [`crate::eval::apply`] runs its δ-rule — the same moment, and
    /// the same arm, at which ι fires for a recursor.
    Builtin(crate::base::Builtin),
    /// `Type l`. Predicative and not cumulative: `Type l : Type (succ l)`.
    Universe(Level),
    /// `(x : A) → B`, the one function type.
    Pi {
        /// How a use site fills this argument. §1: exactly one Π, and **no
        /// core rule reads this** — it is here because a type reached by
        /// projection or by substitution has been through the semantic domain,
        /// and elaboration still has to be able to ask how the binder it found
        /// is filled.
        filling: Filling,
        /// The binder's written name.
        name: Name,
        /// `A`.
        domain: Term,
        /// `B`, under the binder.
        codomain: Term,
    },
    /// `λx. e`.
    Lam {
        /// The binder's written name.
        name: Name,
        /// The body, under the binder.
        body: Term,
    },
    /// `f a`.
    App {
        /// `f`.
        function: Term,
        /// `a`.
        argument: Term,
    },
    /// `{ f₁ : A₁, …, fₙ : Aₙ }` — a dependent record type, primitive rather
    /// than Σ sugar (§1), whose later field types may mention earlier fields.
    RecordType(Arc<[Field]>),
    /// `{ f₁ = e₁, …, fₙ = eₙ }`.
    Record(Arc<[Field]>),
    /// `e.f`.
    Project {
        /// The record being projected.
        record: Term,
        /// The field's name.
        field: Name,
    },
    /// `T(i)` — a type carrying an index expression (§1.5).
    ///
    /// **Indexing, not refinement.** A refinement type in Freeman and Pfenning's
    /// sense is a subset carved by a predicate over the inhabitant, and it lives
    /// on subtyping: `{v : Int | v > 0}` *is* an `Int`. This is neither. The
    /// index is a parameter beside the type rather than a proposition about the
    /// value, and conversion is invariant — `Row(12)` is not a `Row`, and there
    /// is no coercion between them. What that makes this is Xi and Pfenning's
    /// indexed type, and the name is where the code says so.
    ///
    /// A **wrapper**, not a parameter and not a family index. `Row(12)` is the
    /// ordinary declared type `Row` under an index, so [`crate::family`] does
    /// not grow by a line and a value of `Row(12)` is a value of `Row`. Three
    /// consequences follow from that one choice, and each is what §1.5 asks for:
    ///
    /// - **Erasure is structural.** [`crate::quote`] drops the wrapper and reads
    ///   back `ty` alone, so a read-back term carries no index and every stored
    ///   artifact is byte-identical to what it was before the stratum existed.
    ///   It is not a property a test watches; it is where the code sits.
    /// - **Conversion has one arm.** Two indexed types agree when their types
    ///   agree and [`crate::index::decide`] answers `Same`; an indexed type never
    ///   agrees with a bare one, because `Row(12)` and `Row` are two types.
    /// - **Nothing else changes.** Evaluation passes through, the eliminators
    ///   are untouched, and no rule anywhere takes one apart.
    ///
    /// The index is an ordinary [`Term`] of index sort rather than an
    /// [`crate::index::Expr`], and that is the repair §1.5 took at 142d: an
    /// index variable is an ordinary parameter, so it is bound, substituted, and
    /// solved by machinery that already exists. Reading a *value* of this
    /// position into a linear form is [`crate::convert::reads_as_index`]'s, and
    /// prompt 142da moved the asking to type formation so that an index the
    /// grammar cannot read never reaches a comparison.
    Indexed {
        /// The type being refined.
        ty: Term,
        /// The index it is refined by.
        index: Term,
    },
    /// `let x : A = v in e`, non-recursive. Its unfolding is δ.
    Let {
        /// The binder's written name.
        name: Name,
        /// `A`.
        ty: Term,
        /// `v`.
        value: Term,
        /// `e`, under the binder.
        body: Term,
    },
}

/// α-equality: the three things a core term carries that conversion does not
/// look at.
///
/// §7 states them as one rule rather than three exceptions — an [`Origin`], a
/// binder's written name, and a binder's [`Filling`]. Each is carried so that a
/// diagnostic, a formatter, or the elaborator can read it, and each would make
/// conversion answer `false` for two spellings of one type if equality looked at
/// it. Written out rather than derived because a derive would look at all three:
/// `(x : A) → B` and `(y : A) → B` are the same function type, and so are
/// `{x : A} → B` and `(x : A) → B`.
///
/// Field names are not on that list. A record *is* its fields, and
/// [`Shape::Project`] finds one by name, so two record types differing in a
/// field name are different types.
impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Hole(left), Self::Hole(right)) => left == right,
            (Self::Var(left), Self::Var(right)) => left == right,
            (Self::Const(left), Self::Const(right)) => left == right,
            // Two uses of one definition are one term. Conversion never gets
            // this far — evaluation unfolds a definition before anything
            // compares — so this is α-equality on *syntax*, which is what a
            // semantic hash and a re-check are written against.
            (Self::Def(left), Self::Def(right)) => left == right,
            (Self::Base(left), Self::Base(right)) => left == right,
            (Self::Builtin(left), Self::Builtin(right)) => left == right,
            // §5.8: an inert base type contributes no ι-rule, so two closed
            // values of it are convertible iff they are the same constant. The
            // host decides what "the same" means for its own data.
            (Self::Lit(left), Self::Lit(right)) => left == right,
            // Constant time, and that is the representation earning its keep:
            // two numerals are equal when they count the same far at the same
            // family, where two towers would have been walked to the floor.
            (Self::Numeral(left), Self::Numeral(right)) => left == right,
            (Self::Universe(left), Self::Universe(right)) => left == right,
            // Both halves: an indexed type *is* its type and its index, and two
            // indexed types at one type by two indices are two types. α-equality
            // is syntactic here, as everywhere in this impl; deciding whether
            // two *different* index expressions denote one quantity is
            // conversion's question and [`crate::index`]'s answer.
            (
                Self::Indexed {
                    ty: left_ty,
                    index: left_index,
                },
                Self::Indexed {
                    ty: right_ty,
                    index: right_index,
                },
            ) => left_ty == right_ty && left_index == right_index,
            (
                Self::Pi {
                    filling: _,
                    name: _,
                    domain: left_domain,
                    codomain: left_codomain,
                },
                Self::Pi {
                    filling: _,
                    name: _,
                    domain: right_domain,
                    codomain: right_codomain,
                },
            ) => left_domain == right_domain && left_codomain == right_codomain,
            (
                Self::Lam {
                    name: _,
                    body: left_body,
                },
                Self::Lam {
                    name: _,
                    body: right_body,
                },
            ) => left_body == right_body,
            (
                Self::App {
                    function: left_function,
                    argument: left_argument,
                },
                Self::App {
                    function: right_function,
                    argument: right_argument,
                },
            ) => left_function == right_function && left_argument == right_argument,
            (Self::RecordType(left), Self::RecordType(right)) | (Self::Record(left), Self::Record(right)) => {
                left == right
            }
            (
                Self::Project {
                    record: left_record,
                    field: left_field,
                },
                Self::Project {
                    record: right_record,
                    field: right_field,
                },
            ) => left_field == right_field && left_record == right_record,
            (
                Self::Let {
                    name: _,
                    ty: left_ty,
                    value: left_value,
                    body: left_body,
                },
                Self::Let {
                    name: _,
                    ty: right_ty,
                    value: right_value,
                    body: right_body,
                },
            ) => left_ty == right_ty && left_value == right_value && left_body == right_body,
            // Two different shapes. Every variant is spelled out on the left
            // rather than collapsed to `_`, so that adding one to `Shape` is a
            // non-exhaustive-match error here rather than a silent `false` for
            // the new form.
            (
                Self::Hole(_)
                | Self::Var(_)
                | Self::Const(_)
                | Self::Def(_)
                | Self::Base(_)
                | Self::Lit(_)
                | Self::Builtin(_)
                | Self::Universe(_)
                | Self::Pi { .. }
                | Self::Lam { .. }
                | Self::App { .. }
                | Self::RecordType(_)
                | Self::Record(_)
                | Self::Project { .. }
                | Self::Numeral(_)
                | Self::Indexed { .. }
                | Self::Let { .. },
                _,
            ) => false,
        }
    }
}

impl Eq for Shape {}

impl Term {
    /// A term of shape `shape`, elaborated from `origin`.
    #[must_use]
    pub fn new(origin: Origin, shape: Shape) -> Self {
        Self {
            origin,
            shape: Arc::new(shape),
        }
    }

    /// Where this term came from.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        self.origin
    }

    /// What this term is.
    #[must_use]
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// The same term, said to have come from somewhere else.
    ///
    /// Shares the shape rather than copying it, so re-stamping a large term is
    /// one refcount. Used where a rule fixes the answer — §7's η clause, where
    /// an expansion carries the origin of the term it expanded.
    #[must_use]
    pub fn at(&self, origin: Origin) -> Self {
        Self {
            origin,
            shape: Arc::clone(&self.shape),
        }
    }

    /// A placeholder for an unwritten argument — see [`crate::meta::Hole`].
    #[must_use]
    pub(crate) fn hole(origin: Origin, hole: crate::meta::Hole) -> Self {
        Self::new(origin, Shape::Hole(hole))
    }

    /// A variable.
    #[must_use]
    pub fn var(origin: Origin, index: Index) -> Self {
        Self::new(origin, Shape::Var(index))
    }

    /// `count` steps above `family`'s floor, as one node. See
    /// [`Shape::Numeral`].
    ///
    /// Takes the family's constant rather than its name, because by the time a
    /// term is built the name has already been resolved and a second lookup
    /// could disagree with the first.
    pub(crate) fn numeral(origin: Origin, family: &crate::family::Constant, count: u64) -> Self {
        Self::new(
            origin,
            Shape::Numeral(crate::family::Numeral {
                family: family.clone(),
                count,
            }),
        )
    }

    /// `Type level`.
    #[must_use]
    pub fn universe(origin: Origin, level: Level) -> Self {
        Self::new(origin, Shape::Universe(level))
    }

    /// The universe a checked type inhabits, computed structurally.
    ///
    /// §1 fixes two universes, so this is a walk rather than an inference:
    /// `Type l` inhabits `succ l` — and `Type 1`'s is the refusal there is no
    /// level for — a function or record type joins its parts, and every other
    /// shape a checked type can have stands at `Type 0`: an enumeration, a base
    /// type, a variable of type `Type 0`, an application of either.
    ///
    /// # Errors
    ///
    /// [`crate::Refusal::BeyondUniverses`] at a `Type 1` — a type of types of
    /// types is the third universe the calculus does not have.
    pub fn level_of(term: &Self) -> Result<Level, crate::Refusal> {
        match term.shape() {
            Shape::Universe(level) => level
                .succ()
                .ok_or(crate::Refusal::BeyondUniverses { at: term.origin() }),
            Shape::Pi { domain, codomain, .. } => Ok(Self::level_of(domain)?.max(Self::level_of(codomain)?)),
            Shape::RecordType(fields) => fields
                .iter()
                .try_fold(Level::ZERO, |join, field| Ok(Self::level_of(&field.term)?.max(join))),
            Shape::Hole(_)
            | Shape::Var(_)
            | Shape::Const(_)
            | Shape::Def(_)
            | Shape::Numeral(_)
            | Shape::Base(_)
            | Shape::Lit(_)
            | Shape::Builtin(_)
            | Shape::Lam { .. }
            | Shape::App { .. }
            | Shape::Record(_)
            | Shape::Project { .. }
            | Shape::Let { .. } => Ok(Level::ZERO),
            // An indexed type is at the level of what it refines. The index is a
            // value, not a type, so it contributes no level at all.
            Shape::Indexed { ty, .. } => Self::level_of(ty),
        }
    }

    /// `(name : domain) → codomain`.
    #[must_use]
    pub fn pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::function(origin, Filling::Written, name, domain, codomain)
    }

    /// `{name : domain} → codomain` — the same type, whose argument a use site
    /// does not write.
    #[must_use]
    pub fn parameter_pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::function(origin, Filling::Parameter, name, domain, codomain)
    }

    /// `[Class a⃗] → codomain` — the binder `01-surface.md` §1.4's `where`
    /// elaborates to, whose argument every use site answers by `10-traits.md`
    /// §4 rather than writing.
    ///
    /// The domain is the dictionary's *record type*, which is what makes this
    /// one Π and not a new form; the constraint rides along so that resolution
    /// can key on the trait after the domain has β-reduced past it.
    #[must_use]
    pub(crate) fn constrained_pi(
        origin: Origin,
        constraint: Arc<Constraint>,
        name: impl Into<Name>,
        domain: Self,
        codomain: Self,
    ) -> Self {
        Self::function(origin, Filling::Constraint(constraint), name, domain, codomain)
    }

    /// `(name : domain) → codomain` at a filling a caller already has in hand —
    /// quotation, which must write back the Π it read.
    pub(crate) fn function(
        origin: Origin,
        filling: Filling,
        name: impl Into<Name>,
        domain: Self,
        codomain: Self,
    ) -> Self {
        Self::new(
            origin,
            Shape::Pi {
                filling,
                name: name.into(),
                domain,
                codomain,
            },
        )
    }

    /// `λname. body`.
    #[must_use]
    pub fn lam(origin: Origin, name: impl Into<Name>, body: Self) -> Self {
        Self::new(
            origin,
            Shape::Lam {
                name: name.into(),
                body,
            },
        )
    }

    /// `ty(index)` — `ty` refined by `index` (§1.5).
    #[must_use]
    pub fn indexed(origin: Origin, ty: Self, index: Self) -> Self {
        Self::new(origin, Shape::Indexed { ty, index })
    }

    /// `function argument`.
    #[must_use]
    pub fn app(origin: Origin, function: Self, argument: Self) -> Self {
        Self::new(origin, Shape::App { function, argument })
    }

    /// `record.field`.
    #[must_use]
    pub fn project(origin: Origin, record: Self, field: impl Into<Name>) -> Self {
        Self::new(
            origin,
            Shape::Project {
                record,
                field: field.into(),
            },
        )
    }

    /// `{ … }` as a record type, from `(name, type)` pairs in telescope order.
    #[must_use]
    pub fn record_type<'a>(origin: Origin, fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::new(origin, Shape::RecordType(collect_fields(fields)))
    }

    /// `{ … }` as a record value, from `(name, value)` pairs.
    #[must_use]
    pub fn record<'a>(origin: Origin, fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::new(origin, Shape::Record(collect_fields(fields)))
    }

    /// `let name : ty = value in body`.
    #[must_use]
    pub fn bind(origin: Origin, name: impl Into<Name>, ty: Self, value: Self, body: Self) -> Self {
        Self::new(
            origin,
            Shape::Let {
                name: name.into(),
                ty,
                value,
                body,
            },
        )
    }
}

fn collect_fields<'a>(fields: impl IntoIterator<Item = (&'a str, Term)>) -> Arc<[Field]> {
    fields
        .into_iter()
        .map(|(name, term)| Field {
            name: Arc::from(name),
            term,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DbLevel, Index, Term};
    use crate::level::Level;
    use crate::origin::Origin;

    #[test]
    fn a_level_reads_back_as_the_index_that_names_it() {
        // Three binders in scope: levels 0, 1, 2 are indices 2, 1, 0.
        assert_eq!(DbLevel(0).to_index(3), Some(Index(2)));
        assert_eq!(DbLevel(1).to_index(3), Some(Index(1)));
        assert_eq!(DbLevel(2).to_index(3), Some(Index(0)));
    }

    #[test]
    fn a_level_out_of_scope_has_no_index() {
        assert_eq!(DbLevel(3).to_index(3), None);
        assert_eq!(DbLevel(0).to_index(0), None);
    }

    #[test]
    fn two_terms_differing_only_in_origin_are_equal() {
        let here = Term::universe(Origin::node(1), Level::ZERO);
        let there = Term::universe(Origin::node(2), Level::ZERO);
        assert_eq!(here, there);
        assert_ne!(here.origin(), there.origin());
    }

    #[test]
    fn origins_are_ignored_arbitrarily_deep() {
        let one = Term::lam(Origin::node(1), "z", Term::var(Origin::node(2), Index(0)));
        let other = Term::lam(Origin::UNKNOWN, "z", Term::var(Origin::node(9), Index(0)));
        assert_eq!(one, other, "a child's origin is no more part of equality than a root's");
    }

    #[test]
    fn re_stamping_shares_the_shape_and_changes_nothing_else() {
        let term = Term::lam(Origin::UNKNOWN, "z", Term::var(Origin::UNKNOWN, Index(0)));
        let stamped = term.at(Origin::node(4));
        assert_eq!(term, stamped);
        assert_eq!(stamped.origin(), Origin::node(4));
    }
}
