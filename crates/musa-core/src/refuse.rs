//! What elaboration says when it rejects a program.
//!
//! `docs/rules/language/02-core-calculus.md` §4 gives every judgment three
//! outcomes — accepted, refused, exhausted — and [`ElabError`] is that list
//! written down. Refusal and exhaustion are separate variants rather than two
//! shades of one error because §4 is explicit that they are different answers:
//! "this program was not rejected; the checker ran out of room" is a different
//! sentence from "this program is wrong", and a caller that could not tell them
//! apart would let a resource limit silently decide a program's meaning.
//!
//! # How much of a type a mismatch prints
//!
//! This is the policy the prompt owes, and it is short: **report the smallest
//! pair of subterms unification actually disagreed on, together with the path
//! from the root of the two types to it.** Not the whole normal forms.
//!
//! It falls out of the algorithm rather than being a separate printer.
//! Unification descends two types structurally, forcing each head only as far as
//! it needs to compare it, so the pair it fails on is by construction the
//! shallowest place they differ and is already unfolded exactly as far as
//! deciding the question required. Anything more would be a second traversal
//! whose output no rule constrains — and a conversion error that prints two
//! forty-line normal forms is a failed diagnostic even when it is a correct one.
//!
//! The path is what keeps that honest. `Nat` versus `Bool` is useless on its
//! own; "the domain of the second argument's function type: expected `Nat`,
//! found `Bool`" is not. And because every term carries an [`Origin`] (§7), each
//! side of the pair can point at the source node it came from even though
//! neither is a term the author wrote.

use std::fmt;

use crate::budget::ResourceError;
use crate::error::{CoreError, Malformed};
use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::term::{Name, Term};

/// Why elaboration did not produce a core term.
///
/// §4's three outcomes, minus acceptance: the two ways of not answering, plus
/// the caller defect [`CoreError`] already distinguishes.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ElabError {
    /// The program was rejected, and this says why.
    #[error(transparent)]
    Refused(#[from] Refusal),
    /// The deterministic budget ended elaboration. **Not** a type error: the
    /// program was not judged.
    #[error(transparent)]
    Exhausted(ResourceError),
    /// A term handed in — a checking type, or a term given to the re-checker —
    /// was not one this crate could have produced.
    #[error("malformed core term: {0}")]
    Malformed(Malformed),
}

impl From<Malformed> for ElabError {
    fn from(malformed: Malformed) -> Self {
        Self::Malformed(malformed)
    }
}

impl From<CoreError> for ElabError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Exhausted(exhausted) => Self::Exhausted(exhausted),
            CoreError::Malformed(malformed) => Self::Malformed(malformed),
        }
    }
}

/// A program elaboration rejected, and the reason.
///
/// Every variant carries the [`Origin`] of the term it is about, because a
/// refusal a reader cannot locate is barely a refusal.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// A name with no binder in scope.
    #[error("no binder named `{name}` is in scope")]
    UnknownName {
        /// The name as written.
        name: Name,
        /// Where it was written.
        at: Origin,
    },
    /// §2's `Switch` called conversion and conversion said no.
    #[error("{0}")]
    Mismatch(Box<Mismatch>),
    /// A metavariable was still unsolved when the declaration that created it
    /// ended. Never defaulted and never generalized (§2.1).
    #[error("could not determine {}", site.describe())]
    Unsolved {
        /// Which of §2.1's sites created it. Named `site` and not `source`
        /// because `thiserror` reads a field called `source` as the error this
        /// one wraps, and a [`MetaSource`] is not an error.
        site: MetaSource,
        /// The term whose elaboration created it.
        created: Origin,
        /// The term whose constraint was still postponed, when one was.
        blocked: Option<Origin>,
    },
    /// Something that is not a function was applied.
    #[error("this is applied to an argument, but its type is not a function type")]
    NotAFunction {
        /// The application.
        at: Origin,
        /// The type the applied term turned out to have.
        ty: Term,
    },
    /// An implicit argument was written in braces at an explicit binder, or an
    /// explicit binder was abstracted where the type wanted an implicit one.
    #[error("this argument is written implicitly, but the binder it fills is not")]
    PlicityMismatch {
        /// The argument or binder.
        at: Origin,
    },
    /// Something that is not a record was projected.
    #[error("this is projected at a field, but its type is not a record type")]
    NotARecord {
        /// The projection.
        at: Origin,
        /// The type the projected term turned out to have.
        ty: Term,
    },
    /// A record type has no such field.
    #[error("this record type has no field named `{field}`")]
    NoSuchField {
        /// The projection.
        at: Origin,
        /// The field asked for.
        field: Name,
    },
    /// A record literal's fields are not the record type's, in order.
    ///
    /// Order and not merely membership: a later field's type may mention an
    /// earlier field's value, so the telescope order is part of the type rather
    /// than a formatting preference.
    #[error("this record literal does not give the type's fields in order")]
    RecordShape {
        /// The literal.
        at: Origin,
        /// The fields the type declares, in telescope order.
        expected: Vec<Name>,
        /// The fields the literal writes, in the order written.
        found: Vec<Name>,
    },
    /// A term stood in type position whose own type is not a universe.
    #[error("this stands where a type is needed, but it is not one")]
    NotAType {
        /// The term.
        at: Origin,
        /// The type it turned out to have.
        ty: Term,
    },
    /// A constructor field mentions a family the declaration is declaring,
    /// somewhere §1.1's strict positivity does not allow.
    ///
    /// The occurrence is named rather than the constructor alone, because the
    /// edit an author makes is at the occurrence: the field is usually right and
    /// one argument of it is wrong.
    #[error("`{family}` occurs in `{constructor}` where a recursive occurrence is not allowed")]
    NonPositive {
        /// The offending occurrence.
        at: Origin,
        /// The family that occurs.
        family: Name,
        /// The constructor whose field it occurs in.
        constructor: Name,
    },
    /// A constructor chose a different number of index arguments than the family
    /// it belongs to declares.
    #[error("this constructor chooses {found} index arguments, but the family declares {expected}")]
    IndexCount {
        /// The constructor.
        at: Origin,
        /// How many indices the family declares.
        expected: usize,
        /// How many the constructor wrote.
        found: usize,
    },
    /// A pattern named something that is not a constructor of the type the
    /// subject it stands against has.
    ///
    /// One refusal rather than two, because "this type has no constructors at
    /// all" and "this type has constructors and not that one" are the same
    /// sentence to the author: the name they wrote does not build this.
    #[error("`{name}` is not a constructor of the type this pattern matches")]
    NoSuchConstructor {
        /// The pattern.
        at: Origin,
        /// The name it wrote.
        name: Name,
        /// The type the subject turned out to have.
        ty: Term,
    },
    /// A `match` left a constructor with no arm, at a subject the index
    /// constraints leave reachable (§6.2).
    #[error("this match has no arm for `{constructor}`")]
    IncompleteMatch {
        /// The match.
        at: Origin,
        /// A constructor no arm covers. One rather than all of them, because
        /// coverage is decided while the tree is built and the first gap is
        /// where the author's model went wrong.
        constructor: Name,
    },
    /// An arm no case the tree reaches can ever select.
    #[error("no value reaches this arm; an earlier one already covers it")]
    UnreachableBranch {
        /// The arm.
        at: Origin,
    },
    /// A subject whose type fixes an index to something other than a variable.
    ///
    /// The rule that would refine it is `02-core-calculus.md` §1.4's deletion,
    /// which requires K and is therefore not in this checker until a program
    /// needs it. The index is named because the edit is at the *subject's type*
    /// — generalize the index and match on it too — and not at the match.
    #[error("this match's subject fixes an index, which needs a unification rule this checker does not have")]
    ForcedIndex {
        /// The subject.
        at: Origin,
        /// The index argument that is not a variable.
        index: Term,
    },
    /// A recursive call the structural measure could not see decrease (§2.4).
    #[error("`{name}` calls itself on something this checker cannot see decrease")]
    UncheckedRecursion {
        /// The call.
        at: Origin,
        /// The definition being defined.
        name: Name,
    },
    /// An introduction form stood where a type had to be synthesized.
    ///
    /// §2's discipline in one variant: introduction forms *check*, so a record
    /// literal — or, in the re-checker, a λ, whose core form carries no domain
    /// (§1) — has no type of its own to report. The fix an author makes is to
    /// write one.
    #[error("this cannot be given a type on its own; write the type it should have")]
    Uninferable {
        /// The term.
        at: Origin,
    },
}

/// Two types that could not be made equal, and where the disagreement is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// The term whose type was being checked.
    pub at: Origin,
    /// The smallest subterm of the expected type that disagreed.
    pub expected: Term,
    /// The smallest subterm of the type actually found that disagreed.
    pub found: Term,
    /// The route from the two whole types to that pair, outermost step first.
    pub path: Vec<PathStep>,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            out.write_str("type mismatch")
        } else {
            out.write_str("type mismatch at ")?;
            for (position, step) in self.path.iter().enumerate() {
                if position > 0 {
                    out.write_str(", ")?;
                }
                write!(out, "{step}")?;
            }
            Ok(())
        }
    }
}

/// One step from the root of a type toward the subterm a mismatch is at.
///
/// Named after the *rôle* the subterm plays rather than after the enum variant
/// it sits in, because the reader has a type in front of them and not an AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStep {
    /// The argument type of a function type.
    Domain,
    /// The result type of a function type.
    Codomain,
    /// Under a λ.
    Body,
    /// The function of an application.
    Function,
    /// The argument of an application.
    Argument,
    /// A named field of a record type or literal.
    Field(Name),
    /// The record a projection is taken from.
    Projected,
    /// The type an identity is at.
    IdType,
    /// An identity's left endpoint.
    IdLeft,
    /// An identity's right endpoint.
    IdRight,
    /// The value `refl` witnesses.
    Witness,
}

impl fmt::Display for PathStep {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain => out.write_str("the argument type"),
            Self::Codomain => out.write_str("the result type"),
            Self::Body => out.write_str("the body"),
            Self::Function => out.write_str("the function"),
            Self::Argument => out.write_str("the argument"),
            Self::Field(field) => write!(out, "field `{field}`"),
            Self::Projected => out.write_str("the projected record"),
            Self::IdType => out.write_str("the type the identity is at"),
            Self::IdLeft => out.write_str("the identity's left side"),
            Self::IdRight => out.write_str("the identity's right side"),
            Self::Witness => out.write_str("the witnessed value"),
        }
    }
}
