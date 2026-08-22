//! What this crate answers when it cannot answer.
//!
//! Two shapes, and the distinction is the point. **Exhaustion** is a language
//! outcome: `docs/rules/language/02-core-calculus.md` §4 names it as the third
//! outcome beside acceptance and refusal, and a program that exhausts one
//! budget may be accepted under another. **Malformation** is a caller defect:
//! this crate does not type-check — prompt 134's elaborator does — so a term
//! that projects a field from a function is not a program that was refused, it
//! is a term nobody should have handed over.
//!
//! Reporting the second rather than panicking on it is deliberate. A total
//! language that aborts has replaced a diagnostic with a crash, and none of the
//! three outcomes describes what happened.

use crate::kernel::budget::ResourceError;
use crate::kernel::term::{Index, Name};

/// Why a core operation did not produce a term.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// The deterministic budget ended the operation (§4).
    #[error(transparent)]
    Exhausted(#[from] ResourceError),
    /// The caller handed over a term that does not fit its stated type, or a
    /// context it does not belong to. A compiler defect, reported as one.
    #[error("malformed core term: {0}")]
    Malformed(#[from] Malformed),
    /// A δ-rule rejected the program (§4's first outcome).
    ///
    /// Deliberately *not* a [`Malformed`], which is the mistake this variant
    /// exists to stop making: the term was well formed and it was the
    /// composer's own arguments the rule said no to. A stretch factor of zero
    /// and a chord asked to sound for no time are programs to fix, and a
    /// sentence blaming the compiler for one is a failed diagnostic.
    ///
    /// It carries a sentence and a place rather than a [`crate::Refusal`]
    /// because this module is below that one: [`crate::ElabError`]'s conversion
    /// is where the two meet, and is the only lift.
    #[error("{message}")]
    Refused {
        /// What the rule said, in its own words.
        message: String,
        /// The application that fired.
        at: crate::kernel::origin::Origin,
    },
}

/// A term that does not fit where it was used.
///
/// Each variant names a *shape* mismatch rather than a typing verdict, because
/// a typing verdict needs a type-checker and this crate is not one. What these
/// say is that evaluation reached a point where the term's own structure made
/// the next step meaningless.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Malformed {
    /// A variable named a binder that is not in scope.
    #[error("variable {0:?} is not bound in this context")]
    UnboundVariable(Index),
    /// A term named something no declaration, definition, or registration in
    /// scope answers to.
    ///
    /// A caller defect and not an unknown-name refusal: the elaborator resolved
    /// every name once already (`02-core-calculus.md` §6), so a term that
    /// reaches evaluation with an unresolvable name was built or moved wrong.
    /// Reported rather than resolved past, because the alternative — falling
    /// through to a different declaration of the same spelling — is the one way
    /// a name reaching the context could change what a program means without a
    /// test noticing.
    #[error("nothing in scope is named `{0}`")]
    UndeclaredName(Name),
    /// Something that is not a function was applied.
    #[error("applied a value that is not a function")]
    NotAFunction,
    /// Something that is not a record was projected.
    #[error("projected a value that is not a record")]
    NotARecord,
    /// A record was projected at a field it does not have.
    #[error("record has no field named `{0}`")]
    NoSuchField(Name),
    /// A value stood where a type was needed.
    #[error("a value that is not a type stood in type position")]
    NotAType,
    /// Quotation reached a level that its own depth does not name, which can
    /// only mean levels and indices were confused somewhere above.
    #[error("quotation reached a variable outside the scope it was quoting in")]
    EscapedVariable,
    /// A metavariable was solved twice. Solutions are write-once (§2.1), so the
    /// second attempt is a conversion checker defect rather than a program's fault.
    #[error("metavariable ?{0} was solved twice")]
    AlreadySolved(u32),
    /// A δ-rule answered nothing at closed literal arguments of its declared
    /// types.
    ///
    /// `docs/rules/language/02-core-calculus.md` §5.8's D2 promises that "for
    /// every tuple of closed values of the declared argument types it yields a
    /// closed value of the declared result type". This is that promise broken,
    /// and it belongs here rather than beside the elaborator's refusals for the
    /// reason this module opens with: the program was well typed and the *table*
    /// was wrong, which is a caller defect. Reported rather than left as a stuck
    /// term, because a silently neutral application surfaces later as an
    /// inscrutable conversion failure somewhere else entirely.
    #[error("builtin `{0}` computed nothing at arguments it declares it accepts")]
    BuiltinStuck(Name),
    /// A refined type reached conversion carrying an index §1.5's grammar cannot
    /// read.
    ///
    /// A caller defect and not a verdict, which is the whole of what prompt
    /// 142da changed. [`crate::Refusal::UnreadableIndex`] refuses such a type
    /// where it is formed, so a comparison that meets one was handed a type that
    /// should not exist. Answering "these two are different" instead is what
    /// made a type inconvertible with itself.
    #[error("a refined type reached conversion carrying an index that cannot be read")]
    UnreadableIndex,
    /// A δ-rule answered data that does not fit its own declared result type.
    ///
    /// Three ways to earn it and one sentence for all of them: the constructor
    /// named is not a case of the type the builtin answers at, or it is applied
    /// to a number of fields the declaration does not have, or the result type
    /// is not a declared family at all. Each is the same defect — the host's
    /// rule and the host's signature disagree — and each is a caller defect for
    /// [`Self::BuiltinStuck`]'s reason: the program was well typed and the
    /// *table* was wrong.
    #[error("builtin answered `{0}`, which does not fit the type it answers at")]
    MisfitAnswer(Name),
    /// A closed normal form was read for a host datum and did not hold one.
    ///
    /// `docs/rules/language/02-core-calculus.md` §5's canonicity says a closed
    /// term at a registered base type normalizes to a *literal* of that type, so
    /// a host that put a datum into the core is entitled to take one back out of
    /// the term the core reduced to. This is that entitlement failing: either
    /// the normal form is not a literal at all, or the literal it is holds some
    /// other host's datum.
    ///
    /// One sentence for both, for [`Self::MisfitAnswer`]'s reason — the term was
    /// checked at the type before it was normalized, so the program was well
    /// typed and the *registration* was wrong.
    #[error("a closed normal form does not hold a `{0}`")]
    NotALiteral(Name),
    /// A base type's [`Accepts`](crate::Accepts) named an operation the registry
    /// does not hold.
    ///
    /// The host said one of its index positions accepts a value at another, and
    /// then named something that is not a builtin of the same registry to carry
    /// it across. [`Self::MisfitAnswer`]'s defect one step earlier: the rule and
    /// the table disagree, and the program that reached it was well typed.
    #[error("no builtin named `{0}` to carry a value into the position that accepts it")]
    UnregisteredCarrier(Name),
}
