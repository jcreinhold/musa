//! A core refusal, restated where it was written.
//!
//! `musa-calculus` answers a rejected program with a [`musa_calculus::ElabError`], which
//! names an [`Origin`] and nothing else about place: the core is a leaf that
//! must not learn what a file is (`02-core-calculus.md` §7). [`super::Sites`]
//! holds the other half of that arrangement, and this module is where the two
//! meet — a refusal in, a [`Diagnostic`] at the composer's own span out.
//!
//! # What this module decides, and what it does not
//!
//! It decides **which code** a refusal is filed under and **which node** it
//! points at. It does not decide how good the sentence is: the message is the
//! refusal's own [`Display`](std::fmt::Display), which `musa-calculus` wrote beside
//! the rule that raises it and can therefore name a normal form nobody wrote.
//! Prompt 165 owns rewriting those; this prompt owns that every one of them
//! arrives as a diagnostic, with a code that `musa explain` knows and a span a
//! reader can jump to. [`Refusal::BuiltinRefused`] is the one whose sentence
//! comes from further out still — a δ-rule in `musa-compiler`'s own registry
//! said it, and the core carried it here and added only the origin.
//!
//! # Why the match is written out
//!
//! Every variant, named once. The alternative — asking `musa-calculus` for a
//! refusal's origin and its severity through accessors — would put the same
//! arms in the core *as well*, because the code still has to be chosen here.
//! One list of the variants is the smaller arrangement, and a variant added to
//! the core fails to compile here until somebody says where it belongs.
//!
//! No count is written down. The three that were went stale as the core grew,
//! and a number in prose is a fact nothing checks — the exhaustive `match` is
//! the check, and it is the only one worth having.

use musa_calculus::{ElabError, Origin, Refusal};

use super::Sites;
use musa_score::diagnose::{Cause, Code, Diagnostic};
use musa_score::origin::SourceSpan;

/// A refusal filed: which code, where it happened, and the earlier place that
/// explains it when there is one.
struct Filed {
    code: Code,
    at: Origin,
    /// A second place and what to say about it. [`None`] for the refusals that
    /// are about one node.
    also: Option<(Origin, &'static str)>,
    /// What to say beside the node, when "here" is not the whole of it.
    ///
    /// A mismatch answers the node's *type*, and the type is what the reader
    /// needs beside the span — "this has type `Option<Degree>`" — because the
    /// message names the pair and the span alone leaves the finding unsaid.
    label: Option<String>,
    /// The repair, for a refusal whose repair is a *surface* spelling.
    ///
    /// Almost always [`None`]: what to do about a refusal is the refusal's own
    /// sentence, written in `musa-calculus` beside the rule that raises it, and
    /// prompt 165 owns how good those are. The exception is a repair the core
    /// cannot name because it is not the core's to know — a form the surface
    /// offers and the core has never heard of.
    help: Option<std::borrow::Cow<'static, str>>,
    /// The sentence, where the core's own names a thing the composer did not
    /// write.
    ///
    /// [`None`] for nearly all of them, and the module documentation says why:
    /// a refusal's wording is the core's, written beside the rule that raises
    /// it, and prompt 165 owns the pass over all of them. This is not that pass
    /// — it is the narrower case where the core's vocabulary and the language's
    /// are *different words for the same thing*, so forwarding the core's would
    /// teach a composer a term the language does not use.
    said: Option<String>,
}

/// What the elaborator said, as a diagnostic at the node that caused it.
///
/// Three outcomes reach here and stay three (`02-core-calculus.md` §4). A
/// **refusal** is the program's fault and carries the node it is about. An
/// **exhaustion** is not a judgment at all — the checker ran out of room — and
/// is filed under [`Code::ResourceLimit`] with no place, because the budget ends
/// wherever it happens to end and pointing at that node would blame it.
/// A **malformed** term is this compiler's own defect: `musa-calculus` says a term
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
            // The one `UnknownName` that is not the whole truth: a
            // quote-pattern arm whose body names a word the pattern matched
            // *literally*. "Cannot find it" is correct and unhelpful — what
            // the author left off is the `$` — so when the site table says the
            // name is a pattern literal where it was used, the report is the
            // old checker's `QuotedLiteralName` one. `11-quotation.md` §4:
            // a quote pattern binds only its splices.
            if let Refusal::UnknownName { name, at, .. } = refusal
                && sites.quoted_literal(*at, name)
            {
                let span = sites.span(*at);
                let upgraded = Diagnostic::error(
                    Code::QuotedLiteralName,
                    format!("the pattern matched `{name}` literally rather than binding it"),
                )
                .maybe_at(span, "nothing declares this name")
                .help(format!("write `${name}` in the pattern to bind what stands there"))
                .note("a quote pattern binds only its splices; every other word in it is matched as written");
                return match sites.foreign(*at) {
                    Some((path, foreign_at)) => imported(path, foreign_at, upgraded),
                    None => upgraded,
                };
            }
            let filed = file(refusal);
            let (also, text) = match filed.also {
                Some((origin, text)) => (sites.span(origin), text),
                None => (None, ""),
            };
            let said = filed.said.unwrap_or_else(|| refusal.to_string());
            let label = filed.label.as_deref().unwrap_or("here").to_owned();
            let restated = Diagnostic::error(filed.code, said)
                .maybe_at(sites.span(filed.at), &label)
                .maybe_also(also, text);
            let restated = match filed.help {
                Some(repair) => restated.help(repair),
                None => restated,
            };
            match sites.foreign(filed.at) {
                Some((path, at)) => imported(path, at, restated),
                None => restated,
            }
        }
        ElabError::Exhausted(exhausted) => Diagnostic::error(Code::ResourceLimit, exhausted.to_string())
            .note("the program was not judged: elaboration reached a deterministic limit before it could answer"),
        ElabError::Malformed(malformed) => Diagnostic::error(
            Code::UnsupportedLanguageStage,
            format!("the compiler built a core term musa-calculus could not have produced: {malformed}"),
        )
        .note("this is a defect in the compiler rather than in the source"),
    }
}

/// A refusal about text in `path`, restated at the `import` that brought it in.
///
/// A span inside a foreign CST means nothing against this document's text, so
/// the refusal cannot be published as it stands: the only span here that is
/// *about* that file is the statement that named it. What was going to be said
/// travels as a [`Cause`] instead, which is the channel prompt 141a built for
/// exactly this — it keeps the code, the sentence and the foreign labels
/// together, and `musa-project` resolves those labels against the import
/// sources it holds, so a reader still gets the line inside the library.
fn imported(path: &str, at: SourceSpan, refusal: Diagnostic) -> Diagnostic {
    Diagnostic::error(Code::Import, format!("`{path}` does not compile"))
        .at(at, "imported here")
        .help(format!("run `musa check {path}`"))
        .caused_by([Cause::of(path, refusal)])
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
        label: None,
        help: None,
        said: None,
    };
    let two = |code: Code, at: Origin, previous: Origin, text: &'static str| Filed {
        code,
        at,
        also: Some((previous, text)),
        label: None,
        help: None,
        said: None,
    };
    match refusal {
        // "cannot find", not "no binder named … is in scope". A binder is the
        // core's word and a composer writing `use nope()` wrote no binder — they
        // wrote a name, and the language has always called this "cannot find":
        // an unknown profile, an unknown import path, and an unknown mark all
        // say so already. Restating it here is what keeps one sentence for one
        // situation now that the name reaches the core to be resolved.
        Refusal::UnknownName { name, at, candidates } => Filed {
            said: Some(format!("cannot find `{name}`")),
            // The near-miss is the surface's sentence — `diagnose::nearest`'s
            // budget and tie rule — over the core's list: which names were in
            // scope is a fact of the context at the refusal, not something to
            // re-collect.
            help: musa_score::diagnose::nearest(name, candidates.iter().map(|name| &**name))
                .map(|near| std::borrow::Cow::Owned(format!("did you mean `{near}`?"))),
            ..one(Code::UnknownName, *at)
        },
        Refusal::Mismatch(mismatch) => {
            let (expected, found) = mismatch
                .whole
                .as_deref()
                .map_or((&mismatch.expected, &mismatch.found), |(expected, found)| {
                    (expected, found)
                });
            let said = match (spelled_type(expected), spelled_type(found)) {
                (Some(expected), Some(found)) => format!("expected `{expected}`, found `{found}`"),
                _ => mismatch.to_string(),
            };
            Filed {
                said: Some(said),
                label: spelled_type(found).map(|found| format!("this has type `{found}`")),
                help: crossing(expected, found).map(std::borrow::Cow::Borrowed),
                ..one(Code::ConversionMismatch, mismatch.at)
            }
        }
        // An index outside `02-core-calculus.md` §1.5's grammar, named at the
        // expression it could not read. The help states the grammar rather than
        // the linear form the solver would have built, because §1.5 asks for a
        // message about `Row(f x)` and a reader fixes the expression.
        Refusal::UnreadableIndex { shown, at } => Filed {
            said: Some(format!("`{shown}` is not an index")),
            label: Some("this stands in an index position".to_owned()),
            help: Some(std::borrow::Cow::Borrowed(
                "an index is a literal, an index variable, `+`, `-`, or `*` by a literal — a call, a `match`, a \
                 projection, and two variables multiplied are each outside it",
            )),
            ..one(Code::TypeMismatch, *at)
        },
        // §1.5's arity, from the two sides an author can be on. Both name the
        // *declaration* in their help, because that is what decides and what
        // has to be looked at: an index is written where a declaration said one
        // would be, and nowhere else.
        Refusal::NotIndexed { ty, at } => Filed {
            said: Some(format!("`{ty}` takes no index")),
            label: Some("written with one here".to_owned()),
            help: Some(std::borrow::Cow::Borrowed(
                "a type carries an index only when its declaration writes one, as `data Pc(n: Nat)`",
            )),
            ..one(Code::IndexArity, *at)
        },
        Refusal::MissingIndex { ty, binder, at } => Filed {
            said: Some(format!("`{ty}` carries an index, and none is written")),
            label: Some(format!("write the `{binder}` it was declared with, as `{ty}(…)`")),
            help: None,
            ..one(Code::IndexArity, *at)
        },
        // And the declaration's own mistake, which no use site can repair.
        Refusal::NotAnIndexSort { binder, ty, at } => Filed {
            said: Some(format!("`{binder}` is not a sort an index can be drawn from")),
            label: Some(format!("`{ty}` is declared to carry one of these")),
            help: Some(std::borrow::Cow::Borrowed(
                "an index is a whole number, an exact fraction, or one of a finite set of literals",
            )),
            ..one(Code::NotAnIndexSort, *at)
        },
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
            label: None,
            help: None,
            said: None,
        },
        Refusal::BareConstructor { at, .. } | Refusal::Uninferable { at, .. } => one(Code::UnsolvedMetavariable, *at),
        // Applying, projecting, or checking something whose type is not the
        // shape the position needs: one family, and the report says which shape
        // was wanted.
        Refusal::NotAFunction { at, .. }
        | Refusal::NotARecord { at, .. }
        | Refusal::NotAType { at, .. }
        | Refusal::BeyondUniverses { at, .. }
        | Refusal::NotStorable { at, .. }
        | Refusal::RecordShape { at, .. } => one(Code::TypeMismatch, *at),
        // Too many, too few, or the wrong kind of argument.
        Refusal::FillingMismatch { at, .. } | Refusal::ClassArity { at, .. } | Refusal::Underapplied { at, .. } => {
            one(Code::WrongArity, *at)
        }
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
        Refusal::NotANumeralFamily { at, .. } => one(Code::NotANumeralFamily, *at),
        Refusal::HandWrittenStorable { at, .. } => one(Code::HandWrittenStorable, *at),
        Refusal::BlanketInstance { at, .. } => one(Code::BlanketInstance, *at),
        Refusal::DuplicateInstance { at, previous, .. } => {
            two(Code::DuplicateInstance, *at, *previous, "already answered here")
        }
        Refusal::OrphanInstance { at, .. } => one(Code::OrphanInstance, *at),
        Refusal::SuperClass { at, .. } => one(Code::SuperClass, *at),
        Refusal::ConstrainedInstance { at, .. } => one(Code::ConstrainedInstance, *at),
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
            label: None,
            help: Some(std::borrow::Cow::Borrowed(
                "name the trait — `Trait::m(x, y)` resolves wherever its dictionary does, and a `where` clause on this signature is what supplies one",
            )),
            said: None,
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

/// A type term's surface spelling, when the surface has one.
///
/// The core's own [`Display`](std::fmt::Display) spells `Option Pitch` — the
/// term as the calculus sees it — and this answers `Option<Pitch>`, the
/// spelling the composer wrote. [`None`] for the shapes the surface cannot
/// write back (a dependent Π is the one a mismatch can still reach), which
/// leaves the core's own sentence standing.
fn spelled_type(term: &musa_calculus::Term) -> Option<String> {
    crate::lower::documented::spelled(term)
}

/// The type head's name and arguments: `Option<Pitch>` is `("Option", [Pitch])`.
///
/// Read off the term rather than off the spelling, because the recursion
/// below is structural — a `List<Option<Degree>>` mismatched against
/// `List<Option<Pitch>>` is the degree confusion twice wrapped, and the
/// sentence that answers it is the degree's own.
fn headed(term: &musa_calculus::Term) -> Option<(String, Vec<musa_calculus::Term>)> {
    let mut head = term;
    let mut arguments = Vec::new();
    while let musa_calculus::Shape::App { function, argument, .. } = head.shape() {
        arguments.push(argument.clone());
        head = function;
    }
    arguments.reverse();
    match head.shape() {
        musa_calculus::Shape::Base(base) => Some((base.to_string(), arguments)),
        musa_calculus::Shape::Const(constant) => Some((constant.to_string(), arguments)),
        _ => None,
    }
}

/// The one-sentence repair for two types that are neighbours in the *domain*
/// rather than in the calculus.
///
/// Ported from the rank-1 checker's `crossing_help`, whose doc said why the
/// table exists: most mismatches are slips and a slip needs no advice, but
/// the pairs below are places where the model has a gap a beginner cannot see
/// — a degree is not a pitch, a key is not a scale — and what the message has
/// to say is not "these differ" but *which operation crosses the gap*. The
/// table is one-directional per entry for the same reason it always was, and
/// every operation it names lives in `stdlib/src/`, so a rename that orphans
/// one of these strings shows up in `stdlib/reference.md` in the same commit.
fn crossing(expected: &musa_calculus::Term, found: &musa_calculus::Term) -> Option<&'static str> {
    // A container of the wrong element is the same confusion one layer out.
    // `Option<Roman>` where `Option<ChordClass>` was wanted is a numeral that
    // has not met a collection, and the sentence about that is the sentence
    // about numerals — the `Option` is not the problem and mentioning it
    // would bury the one that is.
    if let (Some((want, want_arguments)), Some((got, got_arguments))) = (headed(expected), headed(found))
        && want == got
        && matches!(want.as_str(), "Option" | "List")
        && let ([want_inner], [got_inner]) = (want_arguments.as_slice(), got_arguments.as_slice())
    {
        return crossing(want_inner, got_inner);
    }
    let expected_name = headed(expected).map(|(name, _)| name);
    let found_name = headed(found).map(|(name, _)| name);
    Some(match (expected_name.as_deref(), found_name.as_deref()) {
        (Some("EventTrack"), Some("ChordClass")) => {
            "a chord class has no register: `close_position` or `voiced_as` chooses the pitches, \
             and `sound_for` gives the result a duration"
        }
        (Some("EventTrack"), Some("Voicing")) => {
            "a voicing is pitches with no duration: `sound_for(chosen, held)` sounds it"
        }
        (Some("EventTrack"), Some("Pitch" | "PitchClass")) => {
            "a pitch is not music until it lasts: write the duration, as in `c4/4`"
        }
        (Some("Pitch"), Some("Degree")) => {
            "a degree is an ordinal with no octave: `frame_on` registers the collection, and \
             `frame_degree` reads a pitch out of the frame"
        }
        (Some("Pitch"), Some("PitchClass")) => {
            "a note name has no octave: write one (`c4`), or realize the class against a frame"
        }
        (Some("Degree"), Some("Pitch")) => {
            "`degree_in(collection, written)` locates a pitch in a collection, and is absent when \
             it is not a member"
        }
        (Some("PitchClass"), Some("Pc12")) => {
            "a `Pc12` has forgotten its spelling: `spelled_in` chooses one back, against the \
             collection that decides it"
        }
        (Some("Pc12"), Some("PitchClass")) => "`forget_spelling` is the map into `Pc12`, and it is total",
        (Some("Scale"), Some("Key")) => {
            "a key is not a collection — C minor is three of them: `key_scale` takes the \
             signature's own collection, or name the one you mean"
        }
        (Some("ChordClass"), Some("Roman")) => {
            "a numeral carries no collection: `numeral_chord(collection, written)` reads it in one"
        }
        (Some("ChordClass"), Some("Voicing")) => "`chord_of(chosen)` forgets a voicing down to its class",
        (Some("Voicing"), Some("ChordClass")) => {
            "`close_position(content, bass)` chooses the pitches, and the bass is yours to name"
        }
        // A function standing where its result was wanted. The core sees the
        // Π; the sentence is the surface's. A codomain that mentions the
        // binder is nobody's slip — it is a type that computes — and earns no
        // advice.
        _ if matches!(found.shape(), musa_calculus::Shape::Pi { .. }) => {
            return crossing_function(expected, found);
        }
        _ => return None,
    })
}

/// The function row of [`crossing`], kept apart because it reads the Π.
fn crossing_function(expected: &musa_calculus::Term, found: &musa_calculus::Term) -> Option<&'static str> {
    let musa_calculus::Shape::Pi { codomain, .. } = found.shape() else {
        return None;
    };
    // `expected` is closed here — a mismatch's sides are both elaborated
    // types — so it compares equal to a codomain that never mentions the
    // binder, and to nothing that does.
    if codomain != expected {
        return None;
    }
    Some("this is a function, not its result: apply it to its arguments")
}
