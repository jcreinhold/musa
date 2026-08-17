//! A core refusal, restated where it was written.
//!
//! `musa-core` answers a rejected program with a [`musa_core::ElabError`], which
//! names an [`Origin`] and nothing else about place: the core is a leaf that
//! must not learn what a file is (`02-core-calculus.md` §7). [`super::Sites`]
//! holds the other half of that arrangement, and this module is where the two
//! meet — a refusal in, a [`Diagnostic`] at the composer's own span out.
//!
//! # What this module decides, and what it does not
//!
//! It decides **which code** a refusal is filed under and **which node** it
//! points at. It does not decide how good the sentence is: the message is the
//! refusal's own [`Display`](std::fmt::Display), which `musa-core` wrote beside
//! the rule that raises it and can therefore name a normal form nobody wrote.
//! Prompt 144 owns rewriting those; this prompt owns that every one of them
//! arrives as a diagnostic, with a code that `musa explain` knows and a span a
//! reader can jump to. [`Refusal::BuiltinRefused`] is the one whose sentence
//! comes from further out still — a δ-rule in `musa-compiler`'s own registry
//! said it, and the core carried it here and added only the origin.
//!
//! # Why the match is written out
//!
//! Fifty-three variants, each named once. The alternative — asking `musa-core` for
//! a refusal's origin and its severity through accessors — would put the same
//! fifty-four arms in the core *as well*, because the code still has to be chosen
//! here. One list of the variants is the smaller arrangement, and a variant
//! added to the core fails to compile here until somebody says where it belongs.

use musa_core::{ElabError, Origin, Refusal};

use super::Sites;
use crate::diagnose::{Code, Diagnostic};

/// A refusal filed: which code, where it happened, and the earlier place that
/// explains it when there is one.
struct Filed {
    code: Code,
    at: Origin,
    /// A second place and what to say about it. [`None`] for the refusals that
    /// are about one node.
    also: Option<(Origin, &'static str)>,
    /// The repair, for a refusal whose repair is a *surface* spelling.
    ///
    /// Almost always [`None`]: what to do about a refusal is the refusal's own
    /// sentence, written in `musa-core` beside the rule that raises it, and
    /// prompt 144 owns how good those are. The exception is a repair the core
    /// cannot name because it is not the core's to know — a form the surface
    /// offers and the core has never heard of.
    help: Option<&'static str>,
}

/// What the elaborator said, as a diagnostic at the node that caused it.
///
/// Three outcomes reach here and stay three (`02-core-calculus.md` §4). A
/// **refusal** is the program's fault and carries the node it is about. An
/// **exhaustion** is not a judgment at all — the checker ran out of room — and
/// is filed under [`Code::ResourceLimit`] with no place, because the budget ends
/// wherever it happens to end and pointing at that node would blame it.
/// A **malformed** term is this compiler's own defect: `musa-core` says a term
/// handed to it is one it could not have produced, so the report says that
/// rather than dressing it as a source error.
///
/// The span is `Option` throughout, and that is the honest shape. A refusal
/// about a *registered signature* carries [`Origin::UNKNOWN`] by construction —
/// nobody wrote it in a file — and [`Sites::span`] answers `None` rather than
/// pointing at node one.
pub(crate) fn restate(sites: &Sites, error: &ElabError) -> Diagnostic {
    match error {
        ElabError::Refused(refusal) => {
            let filed = file(refusal);
            let (also, text) = match filed.also {
                Some((origin, text)) => (sites.span(origin), text),
                None => (None, ""),
            };
            let restated = Diagnostic::error(filed.code, refusal.to_string())
                .maybe_at(sites.span(filed.at), "here")
                .maybe_also(also, text);
            match filed.help {
                Some(repair) => restated.help(repair),
                None => restated,
            }
        }
        ElabError::Exhausted(exhausted) => Diagnostic::error(Code::ResourceLimit, exhausted.to_string())
            .note("the program was not judged: elaboration reached a deterministic limit before it could answer"),
        ElabError::Malformed(malformed) => Diagnostic::error(
            Code::UnsupportedLanguageStage,
            format!("the compiler built a core term musa-core could not have produced: {malformed}"),
        )
        .note("this is a defect in the compiler rather than in the source"),
    }
}

/// Which code a refusal belongs under, and which nodes it is about.
///
/// The mapping is nearly one-to-one, because `musa-compiler`'s codes were named
/// for these rules as they were built. Where several refusals share a code they
/// share a *family* — a thing a reader looks up once — and the doc comment on
/// each such arm says which family, since that is the judgment and not the
/// mechanics.
fn file(refusal: &Refusal) -> Filed {
    let one = |code: Code, at: Origin| Filed {
        code,
        at,
        also: None,
        help: None,
    };
    let two = |code: Code, at: Origin, previous: Origin, text: &'static str| Filed {
        code,
        at,
        also: Some((previous, text)),
        help: None,
    };
    match refusal {
        Refusal::UnknownName { at, .. } => one(Code::UnknownName, *at),
        Refusal::Mismatch(mismatch) => one(Code::ConversionMismatch, mismatch.at),
        Refusal::Private { at, .. } => one(Code::PrivateName, *at),
        Refusal::MixedVisibility { at, .. } => one(Code::MixedVisibility, *at),
        Refusal::AbstractMatch { at, .. } => one(Code::AbstractMatch, *at),
        // The three ways elaboration can fail to *determine* something the
        // program did not say. A metavariable nothing solved is the general
        // case; a bare constructor and a term with no inference rule are the
        // two the surface reaches by writing less than a type needs.
        Refusal::Unsolved { created, blocked, .. } => Filed {
            code: Code::UnsolvedMetavariable,
            at: *created,
            also: blocked.map(|origin| (origin, "still waiting on this")),
            help: None,
        },
        Refusal::BareConstructor { at, .. } | Refusal::Uninferable { at, .. } => one(Code::UnsolvedMetavariable, *at),
        // Applying, projecting, or checking something whose type is not the
        // shape the position needs: one family, and the report says which shape
        // was wanted.
        Refusal::NotAFunction { at, .. }
        | Refusal::NotARecord { at, .. }
        | Refusal::NotAType { at, .. }
        | Refusal::RecordShape { at, .. } => one(Code::TypeMismatch, *at),
        // Too many, too few, or the wrong kind of argument.
        Refusal::PlicityMismatch { at, .. }
        | Refusal::IndexCount { at, .. }
        | Refusal::ClassArity { at, .. }
        | Refusal::Underapplied { at, .. } => one(Code::WrongArity, *at),
        // A name the declaration it is read against does not have.
        Refusal::NoSuchField { at, .. } | Refusal::NoSuchConstructor { at, .. } => one(Code::UnknownName, *at),
        // One name written twice, in a record type, an enum, or a `with`.
        Refusal::DuplicateField { at, previous, .. }
        | Refusal::DuplicateCase { at, previous, .. }
        | Refusal::OverlappingUpdate { at, previous, .. } => {
            two(Code::DuplicateName, *at, *previous, "first written here")
        }
        Refusal::NonPositive { at, .. } => one(Code::NonPositiveOccurrence, *at),
        Refusal::IncompleteMatch { at, .. } => one(Code::IncompleteMatch, *at),
        Refusal::UnreachableBranch { at, .. } => one(Code::UnreachableBranch, *at),
        Refusal::ForcedIndex { at, .. } => one(Code::ForcedIndex, *at),
        Refusal::UncheckedRecursion { at, .. } => one(Code::UncheckedRecursion, *at),
        Refusal::UntypedRecursion { at, .. } => one(Code::UntypedRecursion, *at),
        // `02-core-calculus.md` §2.4's graph rule, under the code the old
        // checker filed the same mistake under: a reader who has seen
        // `dependency-cycle` once has seen this.
        Refusal::DefinitionCycle { at, .. } => one(Code::DependencyCycle, *at),
        Refusal::ReservedClass { at, .. } => one(Code::ReservedClass, *at),
        Refusal::HeadlessClass { at, .. } => one(Code::HeadlessClass, *at),
        Refusal::ConstrainedField { at, .. } => one(Code::ConstrainedField, *at),
        Refusal::DuplicateMethod { at, previous, .. } => {
            two(Code::DuplicateMethod, *at, *previous, "first declared here")
        }
        Refusal::HandWrittenStorable { at, .. } => one(Code::HandWrittenStorable, *at),
        Refusal::BlanketInstance { at, .. } => one(Code::BlanketInstance, *at),
        Refusal::DuplicateInstance { at, previous, .. } => {
            two(Code::DuplicateInstance, *at, *previous, "already answered here")
        }
        Refusal::OrphanInstance { at, .. } => one(Code::OrphanInstance, *at),
        Refusal::UnboundedInstance { at, .. } => one(Code::UnboundedInstance, *at),
        Refusal::DerivedMethod { at, .. } => one(Code::DerivedMethod, *at),
        Refusal::NoSuchMethod { at, .. } => one(Code::NoSuchMethod, *at),
        Refusal::MissingMethod { at, .. } => one(Code::MissingMethod, *at),
        Refusal::UnresolvedInstance { at, .. } => one(Code::UnresolvedInstance, *at),
        Refusal::UnconstrainedVariable { at, .. } => one(Code::UnconstrainedVariable, *at),
        // The one refusal whose repair the core cannot name. `01-surface.md`
        // §1.5's refusal table gives both halves — "with the `where` or
        // `Trait::m(x, y)` as the fix" — and `Trait::m` is a *surface* spelling
        // the core has never seen: it reaches the core already read, as the
        // qualified name `Trait.m`. So the sentence belongs here, and it is
        // true here because this prompt is what gives the path a reading.
        Refusal::MethodOnVariable { at, .. } => Filed {
            code: Code::MethodOnVariable,
            at: *at,
            also: None,
            help: Some(
                "name the trait — `Trait::m(x, y)` resolves wherever its dictionary does, and a `where` clause on this signature is what supplies one",
            ),
        },
        Refusal::NoMethodForType { at, .. } => one(Code::NoMethodForType, *at),
        Refusal::AmbiguousMethod { at, .. } => one(Code::AmbiguousMethod, *at),
        Refusal::UnkeyedConstraint { at, .. } => one(Code::UnkeyedConstraint, *at),
        // The registry's own five. Reachable from source only through a
        // compiler defect — nobody writes a δ-builtin in `.musa` — but filed
        // rather than folded together, because the person who reads one of
        // these is the person editing `BUILTIN_OWNERSHIP`.
        Refusal::DuplicateExtern { at, .. } => one(Code::DuplicateExtern, *at),
        Refusal::HigherOrderDelta { at, .. } => one(Code::HigherOrderDelta, *at),
        Refusal::UnknownBase { at, .. } => one(Code::UnknownBase, *at),
        Refusal::BaseNotMatchable { at, .. } => one(Code::BaseNotMatchable, *at),
        Refusal::TargetOutsideSignature { at, .. } => one(Code::TargetOutsideSignature, *at),
        Refusal::TargetNotABase { at, .. } => one(Code::TargetNotABase, *at),
        Refusal::NotFiniteData { at, .. } => one(Code::NotFiniteData, *at),
        // The vocabulary saying no. One code for all of them, because what a
        // reader needs here is the sentence the operation said and not a page
        // per operation, and the node is the application the composer wrote.
        Refusal::BuiltinRefused { at, .. } => one(Code::OperationRefused, *at),
    }
}
