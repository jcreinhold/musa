//! Core terms.
//!
//! One syntactic category (`docs/rules/language/02-core-calculus.md` §1): types
//! are terms, so [`Shape`] is the whole language and there is no second grammar
//! to keep in step with it.
//!
//! **Terms are de Bruijn-*indexed*; values are de Bruijn-*levelled*.** That
//! split is the whole reason both [`Index`] and [`Level`] exist as separate
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

use std::fmt;
use std::sync::Arc;

use crate::kernel::origin::Origin;
use crate::kernel::sort::{Levels, Sort, SortVar};

/// A binder's written name, and a record field's name.
///
/// Binder names carry no meaning: α-equivalence is decided by [`Index`], so two
/// terms differing only in a binder name are equal. What names are for is the
/// diagnostic prompt 134 prints and the source the formatter writes back. Field
/// names *are* meaningful — a record is its fields, and [`Shape::Project`] finds
/// one by name.
pub type Name = Arc<str>;

/// What a term fixes about a name no binder introduced.
///
/// Idris2's `NameType` (`Core/TT/Term.idr`), with the two arms §5.8 adds and
/// the payload §1.3 forces. A *tag*, not a declaration: one word where
/// [`Shape`] used to carry an [`Arc`] to a whole declaration group, an
/// elaborated definition, a base type's kind and host rules, or a δ-table.
/// What the name reduces to is [`Definition`], which the context answers.
///
/// Six arms and not four, because three readers decide the merged pairs apart
/// off a term with no context in hand: `base.rs`'s registration checks tell a
/// base type this registry never registered from a declared family it cannot
/// see, and [`crate::elaboration::elab`] reads a *registered* signature's arrow as its whole
/// parameter list where a source definition's is a parameter list and a
/// returned function.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Role {
    /// A top-level definition (§2.4) — the one global name δ unfolds.
    Defined,
    /// One of a declared family's constructors.
    Constructor,
    /// A declared family, as the type constructor it is (§1.1).
    TypeConstructor,
    /// A family's generated recursor.
    ///
    /// The universe its motives land in used to ride here, because §1 had no
    /// universe polymorphism to put it anywhere else. It is now the recursor's
    /// own level parameter, so it rides where every other level argument does:
    /// in the use site's [`Levels`].
    Recursor,
    /// A base type the host registered (§5.8). Inert: no constructor, no
    /// eliminator, and no rule in this crate takes one apart.
    Base,
    /// A compiler-owned operation the host registered (§5.8). Rigid until its
    /// arguments are literals, at which point [`crate::kernel::eval::apply`] runs its
    /// δ-rule — the same moment, and the same arm, at which ι fires for a
    /// recursor.
    Builtin,
}

/// What the *context* answers about a name.
///
/// [`Role`]'s counterpart and the other half of §6's boundary: the term says
/// which kind of name this is, and this says what it stands for here. Reached
/// by [`Globals::definition`](crate::kernel::context::Globals::definition), which is
/// the one lookup reduction performs.
///
/// Six arms where [`Role`] has six, and they do not line up: a family, a
/// constructor, and a recursor are one
/// [`Constant`](crate::kernel::family::Constant) — one lookup answers all three,
/// and a second copy of the split would be free to disagree with the first —
/// while a *definition* answers as two, because §1 lists an evaluated body and a
/// compiled case tree as different reduction behaviours and reduction has to
/// tell them apart.
#[derive(Clone)]
pub(crate) enum Definition {
    /// Nothing in scope answers to this name.
    ///
    /// A defect in whoever built or moved the term rather than in the program:
    /// the elaborator resolved the name once already. It becomes a
    /// [`Malformed`](crate::Malformed) refusal carrying the name, and never a
    /// silent resolution to a different declaration of the same spelling.
    Undeclared,
    /// A top-level definition whose body was evaluated at its declaration,
    /// held as the reference [`crate::kernel::program`] describes.
    Defined(crate::kernel::program::Def),
    /// A top-level definition whose body is **a compiled case tree** — §1's
    /// second arm, delivered at prompt 155a.
    ///
    /// It reduces by matching rather than by unfolding: given the arguments its
    /// binders abstract, the tree forces the scrutinee and takes the
    /// alternative that constructor names. `None` while the definition is being
    /// elaborated, which is how a recursive body names itself — see
    /// [`Body::Pending`](crate::kernel::program::Body).
    Compiled(
        crate::kernel::program::Def,
        Option<std::sync::Arc<crate::kernel::case_tree::Compiled>>,
    ),
    /// A declared family, one of its constructors, or its recursor.
    Declared(crate::kernel::family::Constant),
    /// A base type the host registered (§5.8).
    Base(crate::kernel::base::Base),
    /// A compiler-owned operation the host registered (§5.8).
    Builtin(crate::kernel::base::Builtin),
}

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
/// the argument, or §2.1's first-order matching solves it, or the checker
/// computes it. The elaborator's own walk spells the same three — see
/// `elab::spine`'s `Slot`.
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
    /// Computed at every use, and never written.
    ///
    /// **`Storable` and nothing else**, since prompt 146 deleted the trait
    /// system: `02-core-calculus.md` §1.2's fact about a type's shape, which a
    /// machine port's signature states and `crate::elaboration::storable` discharges by
    /// walking the type it turned out to have. No source program can write one
    /// — there is no `where` clause in the surface — so this arm is reachable
    /// only through [`crate::requiring_storable`].
    ///
    /// The constraint travels **on the binder** rather than in a table beside
    /// the definition, because a definition is a value: a constructor may be
    /// passed, stored, or returned, and at that use site there is no name to
    /// look up and only the type is in hand.
    Constraint(Arc<Constraint>),
}

/// What a [`Filling::Constraint`] binder demands: a name and its arguments.
///
/// One name only — `Storable` — and this type stays a name-and-arguments pair
/// rather than collapsing to the one argument because the refusal a failed
/// discharge produces names what was asked for, and a nameless demand would
/// make that diagnostic a constant string.
///
/// The arguments are [`Term`]s and are read under whatever telescope the
/// constraint was written under, which is why [`Constraint::at`] exists.
#[derive(Clone, Debug)]
pub struct Constraint {
    /// Where it was written.
    pub(crate) origin: Origin,
    /// The demand's name.
    pub(crate) class: Name,
    /// One argument per parameter of the demand.
    pub(crate) args: Arc<[Term]>,
}

impl Constraint {
    /// The same demand, with its arguments read somewhere else.
    ///
    /// A constraint travels: it is written under one telescope and asked under
    /// another — a use site's, a recursor's, a nested telescope's. What changes
    /// is only how the arguments are spelled, so the name and the origin come
    /// along unexamined and there is one place that says so.
    pub(crate) fn at(&self, args: Arc<[Term]>) -> Self {
        Self {
            origin: self.origin,
            class: Arc::clone(&self.class),
            args,
        }
    }
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
/// binder is. `Level(0)` is the outermost.
///
/// Levels appear only inside values and inside quotation, never in a [`Term`].
///
/// **A position and a count are the same number, and this type is both.** A
/// scope holding `n` binders has levels `0..n`, so `Level(n)` names the next
/// variable to be assumed *and* says how many are already in scope. Carrying
/// the two as separate types — a level and a depth — invited passing one where
/// the other was meant, in the one construction where that mistake is silent;
/// [`Self::to_index`] is where both readings meet and is the reason there is
/// one type here rather than two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(pub u32);

impl Level {
    /// The outermost level, and the empty scope's depth.
    pub const ZERO: Self = Self(0);

    /// The same scope with one more binder in it.
    #[must_use]
    pub const fn deeper(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// The index that names this level from inside a scope holding `depth`
    /// binders.
    ///
    /// `None` when the level is not in scope at that depth, which is a caller
    /// defect rather than a program error — quotation only ever asks about
    /// levels it created.
    pub(crate) const fn to_index(self, depth: Self) -> Option<Index> {
        match depth.0.checked_sub(self.0) {
            Some(0) | None => None,
            Some(back) => Some(Index(back.saturating_sub(1))),
        }
    }
}

/// Which binder a [`Shape::Bind`] introduces, and what it carries beside its
/// body.
///
/// `02-core-calculus.md` §1 writes the term language with **one** binder former
/// and a tag saying which binder it is, and this is that tag. Three
/// constructors become one plus this, which is no saving in variants and is not
/// what it is for: "go under a binder" is written once — in [`crate::kernel::eval`], in
/// [`crate::kernel::quote`], in [`crate::elaboration::elab::zonk`], and in every structural walk —
/// instead of three times, and that is exactly where a de Bruijn bug would
/// otherwise live.
///
/// **A λ carries no domain.** The one place a λ's domain is needed is quotation,
/// which is type-directed and therefore already holds the Π it is quoting at.
/// Storing it on the λ as well would be the same fact twice, free to disagree,
/// and would oblige every site that builds a λ from a type it does not have —
/// an impossible branch, a wrapper around a recursor method — to invent one.
#[derive(Clone, Debug)]
pub enum Binder {
    /// `λx. body`.
    Lam,
    /// `(x : ty) → body`, the one function type.
    Pi {
        /// How a use site fills this argument. §1: exactly one Π, and **no
        /// core rule reads this** — it is here because a type reached by
        /// projection or by substitution has been through the semantic domain,
        /// and elaboration still has to be able to ask how the binder it found
        /// is filled.
        filling: Filling,
        /// `A`, the domain.
        ty: Term,
    },
    /// `let x : ty = value in body`, non-recursive. Its unfolding is δ.
    Let {
        /// `A`.
        ty: Term,
        /// `v`.
        value: Term,
    },
}

/// α-equality on a binder: the type it stands at, and the value a `let` binds.
///
/// A [`Filling`] is excluded for [`Shape`]'s reason — `{x : A} → B` and
/// `(x : A) → B` are one function type — and the two other binders are
/// distinguished by their constructor, not by a tag anyone compares.
impl PartialEq for Binder {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Lam, Self::Lam) => true,
            (Self::Pi { ty: mine, .. }, Self::Pi { ty: theirs, .. }) => mine == theirs,
            (
                Self::Let {
                    ty: my_ty,
                    value: my_value,
                },
                Self::Let {
                    ty: their_ty,
                    value: their_value,
                },
            ) => my_ty == their_ty && my_value == their_value,
            (Self::Lam | Self::Pi { .. } | Self::Let { .. }, _) => false,
        }
    }
}

impl Eq for Binder {}

impl Binder {
    /// The subterms read *outside* the binder: a domain, or a `let`\'s type and
    /// value.
    ///
    /// A λ has none, which is the whole difference between the three as far as
    /// a structural walk is concerned — so a walk asks this and then descends
    /// into the body one binder deeper, once, instead of spelling three arms.
    pub fn outer(&self) -> impl Iterator<Item = &Term> {
        let (first, second) = match self {
            Self::Lam => (None, None),
            Self::Pi { ty, .. } => (Some(ty), None),
            Self::Let { ty, value } => (Some(ty), Some(value)),
        };
        first.into_iter().chain(second)
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

/// A closed value written as one node: a base-type payload, or a numeral at a
/// counting family.
///
/// **Two arms, and the reason is a rule rather than taste.** §3 compares a
/// numeral *as a number*, and ι decrements one, which is what lets `Nat`'s
/// eliminator fire without unfolding a tower of `Succ`. A base literal is the
/// opposite: opaque, with no constructor and no eliminator, so nothing in this
/// crate takes one apart. One constructor of the term language because both are
/// closed values written as one node; two arms because the rules that read them
/// are not the same rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Constant {
    /// A closed value of a base type, opaque to this crate (§5.8).
    ///
    /// Equal to another exactly when the host says the payloads agree at one
    /// type — §5.8's sentence, and [`crate::kernel::base::Literal`]'s own `PartialEq`.
    Payload(crate::kernel::base::Literal),
    /// A closed value of a counting family, written as a count rather than as
    /// that many applications of its step constructor.
    ///
    /// A *representation*, not a new kind of value: it is definitionally the
    /// tower it stands for, and [`crate::kernel::family::Counting`] is the shape
    /// condition that makes that true. Equality is constant time — two numerals
    /// agree when they count the same far at the same family — where two towers
    /// would have been walked to the floor.
    Numeral(crate::kernel::family::Numeral),
}

impl fmt::Display for Constant {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `384`, not 384 `Nat.Succ`s. What the author wrote is what a
        // diagnostic, a hover, and a semantic hash should all say back.
        match self {
            Self::Payload(literal) => write!(out, "{literal}"),
            Self::Numeral(numeral) => write!(out, "{numeral}"),
        }
    }
}

/// What a term is.
///
/// Separate from [`Term`] so that the origin has one home. A shape's children
/// are `Term`s, each carrying its own origin, which is what makes §7's
/// preservation clause checkable node by node.
#[derive(Clone, Debug)]
pub enum Shape {
    /// A placeholder for an argument the instantiation walk has not yet
    /// solved — see [`crate::kernel::meta::Meta`].
    Meta(crate::kernel::meta::Meta),
    /// A variable, named by how many binders out its binder is.
    Var(Index),
    /// A name no binder introduced, and what kind of thing it is (§1).
    ///
    /// One node for the four a declaration used to be smuggled through: a
    /// declared family, one of its constructors, its generated recursor, a
    /// top-level definition, a base type, and a compiler-owned operation are
    /// all *a name*, and what any of them reduces to is what the context says
    /// — [`Definition`], reached by
    /// [`Globals::definition`](crate::kernel::context::Globals::definition). §6 is
    /// where that boundary is stated as a rule, and this variant is where it
    /// holds.
    ///
    /// Nothing here is applied: a family, a constructor, a recursor, and a
    /// builtin all take their arguments through [`Self::App`], so partial
    /// application is the same term either way.
    Named {
        /// The name, as the author could write it: `Vec`, `Vec.Cons`,
        /// `Vec.elim`, `transpose`.
        name: Name,
        /// What the elaborator resolved it to.
        role: Role,
        /// The levels this use instantiates the declaration's level parameters
        /// at (§1, *The hierarchy*).
        ///
        /// [`Levels::NONE`] for a name that is not level-polymorphic, which is
        /// nearly all of them. Recorded in the term rather than re-inferred,
        /// because a re-checker that inferred them again would not be checking
        /// this program — it would be elaborating a second one and hoping the
        /// two agreed.
        levels: Levels,
    },
    /// A closed value written as one node: see [`Constant`].
    Lit(Constant),
    /// `Type l`. Predicative and not cumulative: `Type l : Type (succ l)`.
    Universe(Sort),
    /// `bind x. body` — a λ, a Π, or a `let`, at one node (§1).
    Bind {
        /// The binder's written name.
        name: Name,
        /// Which binder this is, and what it carries beside the body.
        binder: Binder,
        /// The body, under the binder: a λ\'s body, a Π\'s codomain, a
        /// `let`\'s continuation.
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
            (Self::Meta(left), Self::Meta(right)) => left == right,
            (Self::Var(left), Self::Var(right)) => left == right,
            // Two uses of one name are one term — including two uses of one
            // definition, where conversion never gets this far because
            // evaluation unfolds a definition before anything compares. This is
            // α-equality on *syntax*, which is what a semantic hash and a
            // re-check are written against.
            (
                Self::Named {
                    name: left,
                    role: left_role,
                    levels: left_levels,
                },
                Self::Named {
                    name: right,
                    role: right_role,
                    levels: right_levels,
                },
            ) => left == right && left_role == right_role && left_levels == right_levels,
            // §5.8 for a payload and constant-time counting for a numeral —
            // both [`Constant`]'s own rule, stated once where the two arms are.
            (Self::Lit(left), Self::Lit(right)) => left == right,
            (Self::Universe(left), Self::Universe(right)) => left == right,
            (
                Self::Bind {
                    name: _,
                    binder: left_binder,
                    body: left_body,
                },
                Self::Bind {
                    name: _,
                    binder: right_binder,
                    body: right_body,
                },
            ) => left_binder == right_binder && left_body == right_body,
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
            // Two different shapes. Every variant is spelled out on the left
            // rather than collapsed to `_`, so that adding one to `Shape` is a
            // non-exhaustive-match error here rather than a silent `false` for
            // the new form.
            (
                Self::Meta(_)
                | Self::Var(_)
                | Self::Named { .. }
                | Self::Lit(_)
                | Self::Universe(_)
                | Self::Bind { .. }
                | Self::App { .. }
                | Self::RecordType(_)
                | Self::Record(_)
                | Self::Project { .. },
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

    /// The same term with every level variable `with` answers for replaced.
    ///
    /// One instantiation of a level-polymorphic declaration
    /// (`docs/rules/language/02-core-calculus.md` §1). It is a *level*
    /// substitution and not a term substitution — §3's "reduction is never
    /// performed on syntax" is about the term's own binders, which this never
    /// touches: indices, binders, and every subterm's shape come through
    /// unchanged, and only the two places a [`Sort`] can hide are rewritten.
    ///
    /// Rebuilding rather than mutating shares nothing, which is why the two
    /// leaf arms that cannot contain a level — a variable, a literal — hand the
    /// term back whole.
    pub(crate) fn substitute_levels(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        let shape = match self.shape() {
            Shape::Meta(_) | Shape::Var(_) | Shape::Lit(_) => return self.clone(),
            Shape::Named { name, role, levels } => Shape::Named {
                name: Arc::clone(name),
                role: role.clone(),
                levels: levels.substitute(with),
            },
            Shape::Universe(level) => Shape::Universe(level.substitute(with)),
            Shape::Bind { name, binder, body } => Shape::Bind {
                name: Arc::clone(name),
                binder: binder.substitute_levels(with),
                body: body.substitute_levels(with),
            },
            Shape::App { function, argument } => Shape::App {
                function: function.substitute_levels(with),
                argument: argument.substitute_levels(with),
            },
            Shape::RecordType(fields) => Shape::RecordType(substitute_fields(fields, with)),
            Shape::Record(fields) => Shape::Record(substitute_fields(fields, with)),
            Shape::Project { record, field } => Shape::Project {
                record: record.substitute_levels(with),
                field: Arc::clone(field),
            },
        };
        Self {
            origin: self.origin,
            shape: Arc::new(shape),
        }
    }

    /// Every level variable this term mentions, in the order it mentions them,
    /// each once.
    ///
    /// First-occurrence order rather than creation order because it is what a
    /// declaration's level parameters are numbered by: the parameter list a
    /// reader sees should follow the type they are reading, not the order the
    /// elaborator happened to invent unknowns in.
    pub(crate) fn level_vars(&self, found: &mut Vec<SortVar>) {
        match self.shape() {
            Shape::Meta(_) | Shape::Var(_) | Shape::Lit(_) => {}
            Shape::Named { levels, .. } => {
                for level in levels.as_slice() {
                    note_vars(level, found);
                }
            }
            Shape::Universe(level) => note_vars(level, found),
            Shape::Bind { binder, body, .. } => {
                binder.level_vars(found);
                body.level_vars(found);
            }
            Shape::App { function, argument } => {
                function.level_vars(found);
                argument.level_vars(found);
            }
            Shape::RecordType(fields) | Shape::Record(fields) => {
                for field in fields.iter() {
                    field.term.level_vars(found);
                }
            }
            Shape::Project { record, .. } => record.level_vars(found),
        }
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

    /// A placeholder for an unwritten argument — see [`crate::kernel::meta::Meta`].
    #[must_use]
    pub(crate) fn meta(origin: Origin, meta: crate::kernel::meta::Meta) -> Self {
        Self::new(origin, Shape::Meta(meta))
    }

    /// A variable.
    #[must_use]
    pub fn var(origin: Origin, index: Index) -> Self {
        Self::new(origin, Shape::Var(index))
    }

    /// A name no binder introduced, at the role the elaborator resolved it to.
    ///
    /// At no levels: the overwhelming majority of names are not
    /// level-polymorphic, and [`Self::named_at`] is where the ones that are go.
    #[must_use]
    pub fn named(origin: Origin, name: impl Into<Name>, role: Role) -> Self {
        Self::named_at(origin, name, role, Levels::NONE)
    }

    /// The same, instantiated at `levels`.
    #[must_use]
    pub fn named_at(origin: Origin, name: impl Into<Name>, role: Role, levels: Levels) -> Self {
        Self::new(
            origin,
            Shape::Named {
                name: name.into(),
                role,
                levels,
            },
        )
    }

    /// `count` steps above `family`'s floor, as one node. See
    /// [`Constant::Numeral`].
    ///
    /// Takes the family's constant rather than its name, because by the time a
    /// term is built the name has already been resolved and a second lookup
    /// could disagree with the first.
    pub(crate) fn numeral(origin: Origin, family: &crate::kernel::family::Constant, count: u64) -> Self {
        Self::new(
            origin,
            Shape::Lit(Constant::Numeral(crate::kernel::family::Numeral {
                family: family.clone(),
                count,
            })),
        )
    }

    /// `Type level`.
    #[must_use]
    pub fn universe(origin: Origin, level: Sort) -> Self {
        Self::new(origin, Shape::Universe(level))
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

    /// `[Storable a] → codomain` — a binder whose argument the use site does not
    /// write and no instance table answers: it is discharged by computation, in
    /// [`crate::elaboration::storable::discharge`].
    ///
    /// The domain is an ordinary record type, which is what makes this one Π and
    /// not a new form; the constraint rides along so that discharge can still
    /// name what it is answering after the domain has β-reduced past it.
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
            Shape::Bind {
                name: name.into(),
                binder: Binder::Pi { filling, ty: domain },
                body: codomain,
            },
        )
    }

    /// `λname. body`.
    #[must_use]
    pub fn lam(origin: Origin, name: impl Into<Name>, body: Self) -> Self {
        Self::new(
            origin,
            Shape::Bind {
                name: name.into(),
                binder: Binder::Lam,
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

    /// `let name : ty = value in body`.
    #[must_use]
    pub fn bind(origin: Origin, name: impl Into<Name>, ty: Self, value: Self, body: Self) -> Self {
        Self::new(
            origin,
            Shape::Bind {
                name: name.into(),
                binder: Binder::Let { ty, value },
                body,
            },
        )
    }
}

/// Every field, with its term's levels substituted.
fn substitute_fields(fields: &Arc<[Field]>, with: &impl Fn(&SortVar) -> Option<Sort>) -> Arc<[Field]> {
    fields
        .iter()
        .map(|field| Field {
            name: Arc::clone(&field.name),
            term: field.term.substitute_levels(with),
        })
        .collect()
}

/// Add `level`'s variables to `found`, each once, in first-occurrence order.
fn note_vars(level: &Sort, found: &mut Vec<SortVar>) {
    for var in level.vars() {
        if !found.contains(&var) {
            found.push(var);
        }
    }
}

impl Binder {
    /// The same binder with every level variable `with` answers for replaced.
    fn substitute_levels(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        match self {
            Self::Lam => Self::Lam,
            Self::Pi { filling, ty } => Self::Pi {
                filling: filling.substitute_levels(with),
                ty: ty.substitute_levels(with),
            },
            Self::Let { ty, value } => Self::Let {
                ty: ty.substitute_levels(with),
                value: value.substitute_levels(with),
            },
        }
    }

    /// Every level variable this binder mentions, added to `found`.
    fn level_vars(&self, found: &mut Vec<SortVar>) {
        match self {
            Self::Lam => {}
            Self::Pi { filling, ty } => {
                filling.level_vars(found);
                ty.level_vars(found);
            }
            Self::Let { ty, value } => {
                ty.level_vars(found);
                value.level_vars(found);
            }
        }
    }
}

impl Filling {
    /// The same filling with every level variable `with` answers for replaced.
    fn substitute_levels(&self, with: &impl Fn(&SortVar) -> Option<Sort>) -> Self {
        match self {
            Self::Written => Self::Written,
            Self::Parameter => Self::Parameter,
            Self::Constraint(constraint) => Self::Constraint(Arc::new(
                constraint.at(constraint.args.iter().map(|arg| arg.substitute_levels(with)).collect()),
            )),
        }
    }

    /// Every level variable this filling mentions, added to `found`.
    fn level_vars(&self, found: &mut Vec<SortVar>) {
        match self {
            Self::Written | Self::Parameter => {}
            Self::Constraint(constraint) => {
                for arg in constraint.args.iter() {
                    arg.level_vars(found);
                }
            }
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

/// How many times the variable at `level` occurs in `term`.
///
/// A structural count and nothing cleverer. Its one caller is
/// [`crate::kernel::family`] deciding whether a closure's binder is absent from its
/// body, where β can then discard the argument without evaluating it.
pub(crate) fn occurrences(term: &Term, depth: u32, level: u32) -> u32 {
    let deeper = |term: &Term, by: u32| occurrences(term, depth.saturating_add(by), level);
    match term.shape() {
        Shape::Var(index) => u32::from(depth.checked_sub(index.0.saturating_add(1)) == Some(level)),
        // Closed leaves: none of them can be a variable, so none of them can
        // hold an occurrence of one.
        Shape::Named { .. } | Shape::Lit(_) | Shape::Meta(_) | Shape::Universe(_) => 0,
        // One arm for three binders: whatever the binder carries is read
        // outside it, and the body one deeper. That is the saving [`Binder`]
        // exists for, and this is the smallest place it shows.
        Shape::Bind { binder, body, .. } => binder
            .outer()
            .fold(deeper(body, 1), |total, term| total.saturating_add(deeper(term, 0))),
        Shape::App { function, argument, .. } => deeper(function, 0).saturating_add(deeper(argument, 0)),
        Shape::RecordType(fields) => fields.iter().enumerate().fold(0, |total, (which, field)| {
            total.saturating_add(deeper(&field.term, u32::try_from(which).unwrap_or(u32::MAX)))
        }),
        Shape::Record(fields) => fields
            .iter()
            .fold(0, |total, field| total.saturating_add(deeper(&field.term, 0))),
        Shape::Project { record, .. } => deeper(record, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::{Index, Level, Term};
    use crate::kernel::origin::Origin;
    use crate::kernel::sort::Sort;

    #[test]
    fn a_level_reads_back_as_the_index_that_names_it() {
        // Three binders in scope: levels 0, 1, 2 are indices 2, 1, 0.
        assert_eq!(Level(0).to_index(Level(3)), Some(Index(2)));
        assert_eq!(Level(1).to_index(Level(3)), Some(Index(1)));
        assert_eq!(Level(2).to_index(Level(3)), Some(Index(0)));
    }

    #[test]
    fn a_level_out_of_scope_has_no_index() {
        assert_eq!(Level(3).to_index(Level(3)), None);
        assert_eq!(Level(0).to_index(Level(0)), None);
    }

    #[test]
    fn two_terms_differing_only_in_origin_are_equal() {
        let here = Term::universe(Origin::node(1), Sort::ZERO);
        let there = Term::universe(Origin::node(2), Sort::ZERO);
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
