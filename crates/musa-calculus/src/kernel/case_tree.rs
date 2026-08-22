//! A compiled case tree: what a `match` is, before it is a term.
//!
//! `docs/rules/language/02-core-calculus.md` §6.2: "Surface `match` is compiled
//! to a **case tree** and then to nested applications of the generated
//! eliminators, which is where coverage is decided." Both halves are here — the
//! tree is [`CaseTree`], the emission is [`CaseTree::emitted`], and coverage is
//! [`CaseTree::uncovered`], asked of the *finished* tree rather than of the
//! builder's bookkeeping.
//!
//! # Why the tree is a value and not a shape of the recursion
//!
//! The builder in [`crate::elaboration::case`] already walked a tree; what it
//! did not do was ever hold one. Coverage was then a fact about the order the
//! builder happened to visit constructors in, which is the kind of invariant
//! that is true until someone reorders a loop. With the tree in hand it is
//! re-derived from the declaration group, and the two answers have to agree.
//!
//! It is also what prompt 157 needs: a record projection is "a generated
//! function whose body is a one-branch case tree" (§1.1), and a body is a value
//! somebody stores.
//!
//! # Three nodes
//!
//! [`CaseTree::Answer`] is a leaf, [`CaseTree::Split`] is a case analysis, and
//! [`CaseTree::Impossible`] is the branch index unification ruled out. Idris2's
//! `Core/Case/CaseTree.idr` carries a **fourth** node, for a `match` that falls
//! off the end with no alternative matching. Musa forbids exactly that case, so
//! there is no node for it and no name for one here: coverage cannot fail while
//! a program runs, because [`CaseTree::uncovered`] decides it before one does.
//!
//! `Impossible` has **no producer until prompt 156**. With no indices there is
//! nothing for unification to refute, so a tree this crate builds today holds
//! none, and emission reports one as a defect rather than inventing a term.

use std::sync::Arc;

use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::family::{Constant, Group};
use crate::kernel::origin::Origin;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Name, Term};

/// A `match`, compiled.
pub(crate) enum CaseTree {
    /// The branch is this term, elaborated under the binders the splits above
    /// it introduced.
    Answer(Term),
    /// A case analysis. Boxed because a [`Split`] holds a whole telescope and
    /// most nodes are leaves.
    Split(Box<Split>),
    /// Index unification refuted this branch, so it has no body and needs none.
    #[expect(
        dead_code,
        reason = "no producer until prompt 156: with no indices there is nothing for \
                  unification to refute. The lint retires itself when 156 builds one."
    )]
    Impossible,
}

/// One case analysis: what is analysed, and what each case answers.
pub(crate) struct Split {
    /// Where the `match` was written (§7).
    pub(crate) origin: Origin,
    /// The declaration group the analysed family belongs to.
    pub(crate) group: Arc<Group>,
    /// Which family of the group is analysed.
    pub(crate) family: u32,
    /// The family's parameters, as terms at the depth this split stands at.
    pub(crate) params: Arc<[Term]>,
    /// One motive per family of the group, each `λ(t : N p⃗). …` (§1.1).
    pub(crate) motives: Arc<[Term]>,
    /// The universe the motives land in, chosen per use site (§1.3).
    pub(crate) level: Sort,
    /// The subject, as a term at the depth this split stands at.
    pub(crate) on: Term,
    /// One alternative per constructor of **every** family of the group, in
    /// declaration order — the order the generated eliminator takes its methods
    /// in, so emission is a fold rather than a search.
    pub(crate) alternatives: Arc<[Alternative]>,
}

/// One constructor's case.
pub(crate) struct Alternative {
    /// The constructor this is the case for, qualified by its family.
    pub(crate) constructor: Name,
    /// The binders the body stands under: one per field, then one per recursive
    /// field. The names are for a reader of the emitted term; what a row's body
    /// may *write* is bound by the builder from that row's own pattern.
    pub(crate) binders: Arc<[Name]>,
    /// What this case answers.
    pub(crate) body: CaseTree,
}

impl CaseTree {
    /// The first constructor this tree fails to analyse, if there is one.
    ///
    /// **Re-derived from the declaration group**, not from the builder: the
    /// question is "does the group have a constructor no alternative names",
    /// and the group is the only thing that can answer it. A builder that
    /// silently dropped a case, reordered its loops, or matched a name loosely
    /// is caught here rather than by the emitted term happening to type-check.
    pub(crate) fn uncovered(&self) -> Option<Name> {
        let Self::Split(split) = self else {
            return None;
        };
        for wanted in split.constructors() {
            if !split
                .alternatives
                .iter()
                .any(|alternative| *alternative.constructor == *wanted)
            {
                return Some(wanted);
            }
        }
        split
            .alternatives
            .iter()
            .find_map(|alternative| alternative.body.uncovered())
    }

    /// The tree as a term: nested applications of the generated eliminators
    /// (§6.2).
    ///
    /// # Errors
    ///
    /// [`Malformed::UnreachableAlternative`] for an [`Self::Impossible`] node,
    /// which nothing builds until prompt 156 and which has no term to be.
    pub(crate) fn emitted(&self) -> Result<Term, CoreError> {
        match self {
            Self::Answer(term) => Ok(term.clone()),
            Self::Impossible => Err(Malformed::UnreachableAlternative.into()),
            Self::Split(split) => split.emitted(),
        }
    }
}

impl Split {
    /// Every constructor of every family of the group, qualified, in the order
    /// the eliminator takes its methods in.
    fn constructors(&self) -> Vec<Name> {
        let mut wanted = Vec::new();
        for family in 0..self.group.arity() {
            let count = self
                .group
                .family_at(family)
                .map_or(0, |declared| declared.constructors.len());
            for which in 0..count {
                let which = u32::try_from(which).unwrap_or(u32::MAX);
                wanted.push(Constant::constructor(&self.group, family, which).name());
            }
        }
        wanted
    }

    /// `N.elim p⃗ motives methods… subject`.
    fn emitted(&self) -> Result<Term, CoreError> {
        let mut applied = Constant::recursor(&self.group, self.family, self.level.clone()).term(self.origin);
        for param in self.params.iter() {
            applied = Term::app(self.origin, applied, param.clone());
        }
        for motive in self.motives.iter() {
            applied = Term::app(self.origin, applied, motive.clone());
        }
        for alternative in self.alternatives.iter() {
            applied = Term::app(self.origin, applied, alternative.emitted(self.origin)?);
        }
        Ok(Term::app(self.origin, applied, self.on.clone()))
    }
}

impl Alternative {
    /// The method this alternative is: its body, under one λ per binder.
    fn emitted(&self, origin: Origin) -> Result<Term, CoreError> {
        let body = self.body.emitted()?;
        Ok(self
            .binders
            .iter()
            .rev()
            .fold(body, |built, name| Term::lam(origin, Arc::clone(name), built)))
    }
}
