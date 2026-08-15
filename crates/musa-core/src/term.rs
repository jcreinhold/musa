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
use crate::meta::Meta;
use crate::origin::Origin;

/// A binder's written name, and a record field's name.
///
/// Binder names carry no meaning: α-equivalence is decided by [`Index`], so two
/// terms differing only in a binder name are equal. What names are for is the
/// diagnostic prompt 134 prints and the source the formatter writes back. Field
/// names *are* meaningful — a record is its fields, and [`Shape::Project`] finds
/// one by name.
pub type Name = Arc<str>;

/// Whether an argument is written at a use site, or inserted by the elaborator.
///
/// A property of a *binder*. It rides on [`Shape::Pi`] and **no core rule reads
/// it** (§1, amended at prompt 134): there is one function type, conversion
/// ignores this field, and evaluation and quotation only carry it along. It is
/// here rather than beside the type because a Π reached through a record
/// projection, through δ, or through substituting a type variable has been
/// through the semantic domain, and elaboration still has to be able to ask the
/// binder it found whether a use site writes that argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plicity {
    /// Written at every use.
    Explicit,
    /// Inserted at every use, as a metavariable, unless written in braces.
    Implicit,
}

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
    /// A variable, named by how many binders out its binder is.
    Var(Index),
    /// `Type l`. Predicative and not cumulative: `Type l : Type (succ l)`.
    Universe(Level),
    /// `(x : A) → B`, the one function type.
    Pi {
        /// Whether a use site writes this argument, or the elaborator inserts
        /// one. §1: exactly one Π, and **no core rule reads this** — it is here
        /// because a type reached by projection or by substitution has been
        /// through the semantic domain, and elaboration still has to be able to
        /// ask whether the binder it found was implicit.
        plicity: Plicity,
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
    /// A metavariable: a term elaboration has not determined yet (§2.1).
    ///
    /// It is closed and stands for `λx₀ … xₙ₋₁. ?α`, so the elaborator writes it
    /// applied to the binders in scope and a solution can capture nothing it was
    /// not given. **An elaborated term contains none of these**: a meta still
    /// unsolved when elaboration ends is a refusal, and one that is solved is
    /// unfolded away — which is why [`crate::check`] and [`crate::infer`] can
    /// promise an output the re-checker accepts.
    Meta(Meta),
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
/// binder's written name, and a binder's [`Plicity`]. Each is carried so that a
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
            (Self::Var(left), Self::Var(right)) => left == right,
            (Self::Universe(left), Self::Universe(right)) => left == right,
            (
                Self::Pi {
                    plicity: _,
                    name: _,
                    domain: left_domain,
                    codomain: left_codomain,
                },
                Self::Pi {
                    plicity: _,
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
                Self::Id {
                    ty: left_ty,
                    left: left_left,
                    right: left_right,
                },
                Self::Id {
                    ty: right_ty,
                    left: right_left,
                    right: right_right,
                },
            ) => left_ty == right_ty && left_left == right_left && left_right == right_right,
            (Self::Refl(left), Self::Refl(right)) => left == right,
            (
                Self::J {
                    ty: left_ty,
                    from: left_from,
                    motive: left_motive,
                    base: left_base,
                    to: left_to,
                    proof: left_proof,
                },
                Self::J {
                    ty: right_ty,
                    from: right_from,
                    motive: right_motive,
                    base: right_base,
                    to: right_to,
                    proof: right_proof,
                },
            ) => {
                left_ty == right_ty
                    && left_from == right_from
                    && left_motive == right_motive
                    && left_base == right_base
                    && left_to == right_to
                    && left_proof == right_proof
            }
            (Self::Meta(left), Self::Meta(right)) => left == right,
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
                Self::Var(_)
                | Self::Universe(_)
                | Self::Pi { .. }
                | Self::Lam { .. }
                | Self::App { .. }
                | Self::RecordType(_)
                | Self::Record(_)
                | Self::Project { .. }
                | Self::Id { .. }
                | Self::Refl(_)
                | Self::J { .. }
                | Self::Meta(_)
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
        Self::function(origin, Plicity::Explicit, name, domain, codomain)
    }

    /// `{name : domain} → codomain` — the same type, whose argument a use site
    /// does not write.
    #[must_use]
    pub fn implicit_pi(origin: Origin, name: impl Into<Name>, domain: Self, codomain: Self) -> Self {
        Self::function(origin, Plicity::Implicit, name, domain, codomain)
    }

    /// `(name : domain) → codomain` at a plicity a caller already has in hand —
    /// quotation, which must write back the Π it read.
    pub(crate) fn function(
        origin: Origin,
        plicity: Plicity,
        name: impl Into<Name>,
        domain: Self,
        codomain: Self,
    ) -> Self {
        Self::new(
            origin,
            Shape::Pi {
                plicity,
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

    /// A metavariable, as a term.
    #[must_use]
    pub fn meta(origin: Origin, meta: Meta) -> Self {
        Self::new(origin, Shape::Meta(meta))
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
