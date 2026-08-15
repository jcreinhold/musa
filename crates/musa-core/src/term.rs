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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A variable, named by how many binders out its binder is.
    Var(Index),
    /// `Type l`. Predicative and not cumulative: `Type l : Type (succ l)`.
    Universe(Level),
    /// `(x : A) → B`, the one function type. Plicity is an elaboration notion
    /// and never reaches here (§1).
    Pi {
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
    /// `Id A x y`.
    Id {
        /// `A`.
        ty: Term,
        /// `x`.
        left: Term,
        /// `y`.
        right: Term,
    },
    /// `refl x`, the identity type's sole constructor.
    Refl(Term),
    /// `J`, the identity type's dependent eliminator, at all six of its
    /// arguments. K is *not* here: `02-core-calculus.md` §1.4 declines to admit
    /// it as an axiom, because `DecEq` supplies it as a theorem for every family
    /// Musa declares.
    J {
        /// `A`, the type the identity is at.
        ty: Term,
        /// `x`, the identity's left endpoint.
        from: Term,
        /// `P : (y : A) → Id A x y → Type l`, the motive.
        motive: Term,
        /// `p : P x (refl x)`, the base case.
        base: Term,
        /// `y`, the identity's right endpoint.
        to: Term,
        /// `e : Id A x y`, the proof being eliminated.
        proof: Term,
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

    /// A variable.
    #[must_use]
    pub fn var(origin: Origin, index: Index) -> Self {
        Self::new(origin, Shape::Var(index))
    }

    /// `Type level`.
    #[must_use]
    pub fn universe(origin: Origin, level: Level) -> Self {
        Self::new(origin, Shape::Universe(level))
    }

    /// `(name : domain) → codomain`.
    #[must_use]
    pub fn pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::new(
            origin,
            Shape::Pi {
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

    /// `Id ty left right`.
    #[must_use]
    pub fn identity(origin: Origin, ty: Self, left: Self, right: Self) -> Self {
        Self::new(origin, Shape::Id { ty, left, right })
    }

    /// `refl value`.
    #[must_use]
    pub fn refl(origin: Origin, value: Self) -> Self {
        Self::new(origin, Shape::Refl(value))
    }

    /// `J ty from motive base to proof`.
    #[must_use]
    pub fn jay(origin: Origin, ty: Self, from: Self, motive: Self, base: Self, to: Self, proof: Self) -> Self {
        Self::new(
            origin,
            Shape::J {
                ty,
                from,
                motive,
                base,
                to,
                proof,
            },
        )
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
