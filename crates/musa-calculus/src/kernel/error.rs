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
use crate::kernel::term::{Index, Name, Term};

/// Why a core operation did not produce a term.
///
/// # Which enum a case belongs to
///
/// This one and [`crate::Refusal`] divide by two questions, asked in order.
///
/// **Who is the sentence addressed to?** A [`crate::Refusal`] is addressed to
/// someone who *wrote* something — an author writing `.musa`, or a host author
/// writing a registration. A [`Malformed`] is addressed to whoever maintains
/// this compiler: nobody wrote the term, it was assembled wrong.
///
/// **When was it discovered?** The same host mistake can be either, and the
/// boundary is whether the thing that was written is still in hand. A δ
/// signature that §5.8's D1 does not admit is caught at registration, where the
/// registration can be named, so it is [`crate::Refusal::HigherOrderDelta`]. A δ
/// *rule* that answers nothing at arguments it declared it accepts is caught
/// mid-evaluation, long after D2's promise was accepted and with nothing left
/// to point at, so it is [`Malformed::BuiltinStuck`].
///
/// [`Self::Exhausted`] is neither: §4 makes it the third outcome rather than a
/// verdict. [`Self::Refused`] is a δ-rule's verdict on the author's own
/// arguments, which is why it carries a sentence and a place, and why
/// [`crate::ElabError`]'s conversion from this enum is the one crossing.
///
/// A claim that is true on both sides of the line is stated on both sides, in
/// each side's vocabulary, rather than moved down: `NotAFunction`,
/// `NotARecord`, `NoSuchField`, `NotAType` and `Uninferable` are each a
/// [`Malformed`] *and* a [`crate::Refusal`], and
/// [`Malformed::Mistyped`] stands beside [`crate::Refusal::Mismatch`] the same
/// way. Two copies of a distinction that is genuinely two distinctions is
/// not duplication.
///
/// Unification belongs here. *These two terms have no solution* and *this
/// constraint is still blocked* are facts about two terms with no writer to
/// address, discovered by the kernel, so the variants prompt 153 adds are
/// [`Malformed`] cases rather than a third enum.
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
    /// A term reached the kernel still holding a metavariable nobody solved.
    ///
    /// §2.1 never defaults and never generalizes, so this is not an
    /// under-determined program — the elaborator would have named it
    /// [`crate::Refusal::Unsolved`] where it was written. It is a solution that
    /// escaped its scope or a constraint left in a queue, which is why
    /// [`Checked`](crate::Checked) is a type and not a convention.
    #[error("metavariable ?{0} reached the kernel unsolved")]
    UnsolvedMeta(u32),
    /// A term did not have the type the term around it required.
    ///
    /// The re-checker's one verdict, and the reason it is a [`Malformed`] and
    /// not a refusal: elaboration accepted this term already, so the two
    /// answers disagreeing is a defect in this compiler rather than a fault in
    /// the program. `TRUST.md` is where that reading is written down.
    #[error("re-checking derived a type the term around it does not accept")]
    Mistyped {
        /// Where the term whose type disagreed was written.
        at: crate::kernel::origin::Origin,
        /// The type the surrounding term required, as a normal form.
        expected: Term,
        /// The type the re-checker derived, as a normal form.
        found: Term,
    },
    /// An introduction form was asked for a type it does not have.
    ///
    /// A λ carries no domain and a record literal no field types, so §2 makes
    /// both checking forms: the type such a term "obviously" has is a guess. Not
    /// a defect on its own, but a defect where it is raised — the re-checker
    /// only ever infers a subterm whose surroundings gave it no type, and one of
    /// these standing there means the term was assembled wrong.
    #[error("a lambda or a record literal has no type of its own to derive")]
    Uninferable,
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
    /// A metavariable would have had to occur in its own solution.
    ///
    /// The occurs check, and it is a defect rather than a program's fault for
    /// this module's opening reason: two values with no author between them.
    /// The *program* that produced the pair is refused by the elaborator with a
    /// sentence about what it could not determine; this is what the check
    /// itself answers.
    #[error("metavariable ?{0} would have to occur in its own solution")]
    Cyclic(u32),
    /// A metavariable's type did not have the telescope its arity claims.
    ///
    /// `meta.rs` states the invariant: a meta created under `n` binders has a
    /// closed type of `n` nested Π binders around its goal, and every
    /// occurrence applies exactly those `n` variables. This is that invariant
    /// broken — a meta assembled by something that did not build its type, or
    /// an occurrence carrying a spine of the wrong length.
    #[error("metavariable ?{0} does not have the scope its occurrences apply")]
    MetaTelescope(u32),
    /// A metavariable's solution named a variable from outside its scope.
    ///
    /// `02-core-calculus.md` §2.1 admits a solution only when it "mentions no
    /// variable outside" the metavariable's scope, and
    /// [`crate::kernel::unify`] enforces that where it writes one. This variant
    /// is the *second* check, made by the re-checker over a finished term:
    /// verification that trusts the pass it verifies verifies nothing, and this
    /// prompt's failure mode is otherwise silent.
    #[error("metavariable ?{0} was solved with a term that names a variable outside its scope")]
    EscapedSolution(u32),
    /// A use of a level-polymorphic definition named the wrong number of
    /// levels.
    ///
    /// §1's parameters are generalized at the declaration and instantiated at
    /// the use, so a term carrying some other count was assembled by something
    /// that did not read the declaration it names — [`Self::UndeclaredName`]'s
    /// defect one step later, and a caller defect for the same reason.
    #[error("`{0}` was instantiated at the wrong number of universe levels")]
    LevelArity(Name),
}
