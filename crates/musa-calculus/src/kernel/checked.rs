//! The boundary type the kernel accepts, and the one thing it refuses to take.
//!
//! `TRUST.md` states the claim this crate makes: if elaboration has a bug the
//! kernel rejects the artifact, and only a bug in the kernel can make musa
//! accept an ill-typed program. That claim needs a place where the artifact
//! stops being *something elaboration produced* and starts being *something the
//! kernel accepted*, and a function signature is that place.
//!
//! So the kernel does not take a [`Term`]. It takes a [`Checked`], and the only
//! way to get one is [`Checked::try_from`], which fails on a term still holding
//! an unsolved metavariable. A boundary the compiler cannot forget to check is
//! worth more than a rule everyone agrees to follow.

use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::term::{Binder, Shape, Term};

/// A term with no unsolved metavariables left in it.
///
/// The newtype exists ahead of the bug it defends against, deliberately. §2.1
/// never defaults and never generalizes, so a metavariable that survives
/// elaboration is not an under-determined program — it is a solution that
/// escaped its scope, or a constraint still sitting in a queue nobody drained.
/// Prompt 153 is where the solver arrives and where that failure mode becomes
/// reachable; the wrapper is here so that 153 adds a caller rather than a
/// mechanism.
///
/// A `Checked` is *not* a claim that the term type-checks. It is the claim that
/// the term is finished, which is the precondition of asking whether it does.
/// [`recheck::recheck`](crate::kernel::recheck::recheck) answers the second
/// question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked(Term);

impl Checked {
    /// The term inside, for the kernel operation that was handed it.
    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.0
    }
}

impl TryFrom<Term> for Checked {
    type Error = CoreError;

    /// # Errors
    ///
    /// [`Malformed::UnsolvedMeta`] naming the first metavariable found, in the
    /// order the walk reaches them. First rather than all of them, because one
    /// is already a compiler defect and the list adds nothing a fix would use.
    fn try_from(term: Term) -> Result<Self, Self::Error> {
        match unsolved(&term) {
            Some(meta) => Err(Malformed::UnsolvedMeta(meta).into()),
            None => Ok(Self(term)),
        }
    }
}

/// The first metavariable in `term`, by a walk over every subterm.
fn unsolved(term: &Term) -> Option<u32> {
    match term.shape() {
        Shape::Meta(meta) => (!meta.is_solved()).then(|| meta.id()),
        Shape::Var(_) | Shape::Named { .. } | Shape::Lit(_) | Shape::Universe(_) => None,
        Shape::Bind { binder, body, .. } => binder_unsolved(binder).or_else(|| unsolved(body)),
        Shape::App { function, argument } => unsolved(function).or_else(|| unsolved(argument)),
        Shape::RecordType(fields) | Shape::Record(fields) => fields.iter().find_map(|field| unsolved(&field.term)),
        Shape::Project { record, .. } => unsolved(record),
        Shape::Indexed { ty, index } => unsolved(ty).or_else(|| unsolved(index)),
    }
}

/// The same, for what a binder carries beside its body.
fn binder_unsolved(binder: &Binder) -> Option<u32> {
    match binder {
        Binder::Lam => None,
        Binder::Pi { ty, .. } => unsolved(ty),
        Binder::Let { ty, value } => unsolved(ty).or_else(|| unsolved(value)),
    }
}
