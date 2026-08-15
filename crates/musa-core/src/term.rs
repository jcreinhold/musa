//! Core terms.
//!
//! One syntactic category (`docs/rules/language/02-core-calculus.md` §1): types
//! are terms, so this enum is the whole language and there is no second grammar
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

use std::sync::Arc;

use crate::level::Level;

/// A binder's written name, and a record field's name.
///
/// Binder names carry no meaning: α-equivalence is decided by [`Index`], so two
/// terms differing only in a binder name are equal. What names are for is the
/// diagnostic prompt 134 prints and the source the formatter writes back. Field
/// names *are* meaningful — a record is its fields, and [`Term::Project`] finds
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
/// type and why [`Term::RecordType`] is a slice rather than a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// The field's name, which is how it is projected.
    pub name: Name,
    /// A field type in a [`Term::RecordType`], or a field value in a
    /// [`Term::Record`].
    pub term: Term,
}

/// A core term.
///
/// Shared through [`Arc`] rather than owned through [`Box`]: normalization by
/// evaluation copies subterms into closures constantly and never mutates one,
/// so sharing is the representation's job and a deep clone at every binder
/// would be pure waste.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
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
        domain: Arc<Self>,
        /// `B`, under the binder.
        codomain: Arc<Self>,
    },
    /// `λx. e`.
    Lam {
        /// The binder's written name.
        name: Name,
        /// The body, under the binder.
        body: Arc<Self>,
    },
    /// `f a`.
    App {
        /// `f`.
        function: Arc<Self>,
        /// `a`.
        argument: Arc<Self>,
    },
    /// `{ f₁ : A₁, …, fₙ : Aₙ }` — a dependent record type, primitive rather
    /// than Σ sugar (§1), whose later field types may mention earlier fields.
    RecordType(Arc<[Field]>),
    /// `{ f₁ = e₁, …, fₙ = eₙ }`.
    Record(Arc<[Field]>),
    /// `e.f`.
    Project {
        /// The record being projected.
        record: Arc<Self>,
        /// The field's name.
        field: Name,
    },
    /// `Id A x y`.
    Id {
        /// `A`.
        ty: Arc<Self>,
        /// `x`.
        left: Arc<Self>,
        /// `y`.
        right: Arc<Self>,
    },
    /// `refl x`, the identity type's sole constructor.
    Refl(Arc<Self>),
    /// `J`, the identity type's dependent eliminator, at all six of its
    /// arguments. K is *not* here: `02-core-calculus.md` §1.4 declines to admit
    /// it as an axiom, because `DecEq` supplies it as a theorem for every family
    /// Musa declares.
    J {
        /// `A`, the type the identity is at.
        ty: Arc<Self>,
        /// `x`, the identity's left endpoint.
        from: Arc<Self>,
        /// `P : (y : A) → Id A x y → Type l`, the motive.
        motive: Arc<Self>,
        /// `p : P x (refl x)`, the base case.
        base: Arc<Self>,
        /// `y`, the identity's right endpoint.
        to: Arc<Self>,
        /// `e : Id A x y`, the proof being eliminated.
        proof: Arc<Self>,
    },
    /// `let x : A = v in e`, non-recursive. Its unfolding is δ.
    Let {
        /// The binder's written name.
        name: Name,
        /// `A`.
        ty: Arc<Self>,
        /// `v`.
        value: Arc<Self>,
        /// `e`, under the binder.
        body: Arc<Self>,
    },
}

impl Term {
    /// `(name : domain) → codomain`.
    #[must_use]
    pub fn pi(name: &str, domain: Self, codomain: Self) -> Self {
        Self::Pi {
            name: Arc::from(name),
            domain: Arc::new(domain),
            codomain: Arc::new(codomain),
        }
    }

    /// `λname. body`.
    #[must_use]
    pub fn lam(name: &str, body: Self) -> Self {
        Self::Lam {
            name: Arc::from(name),
            body: Arc::new(body),
        }
    }

    /// `function argument`.
    #[must_use]
    pub fn app(function: Self, argument: Self) -> Self {
        Self::App {
            function: Arc::new(function),
            argument: Arc::new(argument),
        }
    }

    /// `record.field`.
    #[must_use]
    pub fn project(record: Self, field: &str) -> Self {
        Self::Project {
            record: Arc::new(record),
            field: Arc::from(field),
        }
    }

    /// `{ … }` as a record type, from `(name, type)` pairs in telescope order.
    #[must_use]
    pub fn record_type<'a>(fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::RecordType(collect_fields(fields))
    }

    /// `{ … }` as a record value, from `(name, value)` pairs.
    #[must_use]
    pub fn record<'a>(fields: impl IntoIterator<Item = (&'a str, Self)>) -> Self {
        Self::Record(collect_fields(fields))
    }

    /// `Id ty left right`.
    #[must_use]
    pub fn identity(ty: Self, left: Self, right: Self) -> Self {
        Self::Id {
            ty: Arc::new(ty),
            left: Arc::new(left),
            right: Arc::new(right),
        }
    }

    /// `refl value`.
    #[must_use]
    pub fn refl(value: Self) -> Self {
        Self::Refl(Arc::new(value))
    }

    /// `J ty from motive base to proof`.
    #[must_use]
    pub fn jay(ty: Self, from: Self, motive: Self, base: Self, to: Self, proof: Self) -> Self {
        Self::J {
            ty: Arc::new(ty),
            from: Arc::new(from),
            motive: Arc::new(motive),
            base: Arc::new(base),
            to: Arc::new(to),
            proof: Arc::new(proof),
        }
    }

    /// `let name : ty = value in body`.
    #[must_use]
    pub fn bind(name: &str, ty: Self, value: Self, body: Self) -> Self {
        Self::Let {
            name: Arc::from(name),
            ty: Arc::new(ty),
            value: Arc::new(value),
            body: Arc::new(body),
        }
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
    use super::{DbLevel, Index};

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
}
