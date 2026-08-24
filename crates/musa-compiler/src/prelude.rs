//! The declarations every Musa program is read against, and the context that
//! holds them.
//!
//! `musa-calculus` is a leaf that knows nothing about pitch, time, or notation
//! (roadmap §15.12), so *something* has to say that `Pitch` is an inert base
//! type and that `Option` is a family with two constructors. This module is that
//! something. It is the host half of `02-core-calculus.md` §5.8: the core owns
//! the mechanism, and the table of what is registered lives here, where the
//! musical domains already do.
//!
//! # Why the families are declared before the registry is built
//!
//! §5.8's D1 admits an argument or result type that is "a base type **or a
//! finite constructor over base types**", and 41% of the δ-builtins in
//! [`crate::phase`]'s table answer an `Option`, a `Result`, or a `List`. A
//! signature that says so has to *name* those families, and a family constant
//! exists only after [`musa_calculus::declare`] has run. So the order is fixed and
//! not a preference: declare the families in a bare context, read their
//! constants back out of it, build the registry against them, and only then hand
//! the whole thing to the rest of the compiler.
//!
//! # Which types are families and which are base types
//!
//! A base type is *inert*: it contributes no ι-rule, so two closed values of it
//! are convertible exactly when the host says the payloads agree. That is the
//! right shape for a domain whose representation the compiler owns and no source
//! program takes apart — a `Pitch`, a `Scale`, a `Row12`.
//!
//! It is the wrong shape for anything source code pattern-matches on, and
//! `02-core-calculus.md` decides two of those by name. `Nat` "is in the language
//! to be the *inductive* numeric type: `zero | succ` is well founded, so its
//! recursor terminates by construction" — a `Nat` with no recursor would leave
//! `nat_fold` with nothing to fold. `Bool` is the same argument one constructor
//! shorter, and `Option`, `List`, and `Result` are what §1's `τ + τ` and its
//! relatives become once families exist. All five are declared here rather than
//! registered, and the eight collection eliminators prompt 141c left out of the
//! core's registry are the traversals over three of them — written in `.musa`,
//! over the constructors declared here, in `stdlib/src/{nat,list,option}.musa`.
//!
//! `Ratio` stays a base type, and that is not an inconsistency with `Nat`: an
//! exact rational has no least element to descend to, which is the same reason
//! `02-core-calculus.md` gives for admitting no signed integer. Arithmetic on it
//! is δ, not ι.
//!
//! # What is *not* declared here, and why
//!
//! `Coordinate` and `Cat` look like enumerations and are registered as base
//! types with literal values instead. They index `Duration`, `Position`, and
//! `Syntax`, and a δ-rule is a `fn` pointer: it can build a
//! [`musa_calculus::Literal`] with no context and cannot build a constructor at all,
//! so `duration_of` — which answers a `Duration` and takes only a `Ratio` —
//! could not write down the type of its own answer if the index were a
//! constructor. Nothing is lost by it, because neither is pattern-matched by
//! source, which is §1's own test for inertness. See [`crate::registry`].

use std::sync::Arc;

use musa_calculus::{
    Cx, ElabError, ModuleId, Origin, Raw, RawArm, RawBinder, RawConstructor, RawData, RawFamily, RawPattern,
    RawProgram, RawTopLevel, Sort, Term, Visibility,
};

/// Where a declaration this module writes comes from.
///
/// Every term here is compiler-owned: no source file wrote `data Nat`, and a
/// diagnostic that pointed at one would be pointing at a line that does not
/// exist. [`Origin::UNKNOWN`] is the honest answer, and the same one
/// `musa-calculus`'s own suites use for a term the host assembled.
const HERE: Origin = Origin::UNKNOWN;

/// `data Bool { False, True }`.
///
/// Declared rather than registered because `if` is a `match`, and a `match`
/// needs constructors to split on. A base type with two opaque values would make
/// every conditional in the language a δ-builtin.
fn bool_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Bool",
            vec![constructor("False", Vec::new()), constructor("True", Vec::new())],
        )],
    )
}

/// `data Nat { Zero, Succ(Nat) }`.
///
/// `02-core-calculus.md`'s own argument for keeping `Nat` and refusing `Int`:
/// `zero | succ` is well founded, so the recursor terminates by construction.
/// Everything the language counts with — repeat counts, list lengths, range
/// bounds — is nonnegative, so nothing musical is lost by having no predecessor
/// below zero.
fn nat_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Nat",
            vec![
                constructor("Zero", Vec::new()),
                constructor("Succ", vec![binder("earlier", var("Nat"))]),
            ],
        )],
    )
}

/// `data Option (A : Type 0) { None, Some(A) }`.
fn option_data() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "Option",
            vec![
                constructor("None", Vec::new()),
                constructor("Some", vec![binder("value", var("A"))]),
            ],
        )],
    )
}

/// `data List (A : Type 0) { Empty, Cons(A, List A) }`.
fn list_data() -> RawData {
    data(
        vec![binder("A", type0())],
        vec![family(
            "List",
            vec![
                constructor("Empty", Vec::new()),
                constructor(
                    "Cons",
                    vec![binder("first", var("A")), binder("rest", applied("List", [var("A")]))],
                ),
            ],
        )],
    )
}

/// `data Result (T : Type 0) (E : Type 0) { Ok(T), Err(E) }`.
///
/// §1's binary sum `τ + τ`, with the spelling it is actually used for. A builtin
/// with more than one way to fail says which one happened by answering this;
/// `Option` says only that it did.
fn result_data() -> RawData {
    data(
        vec![binder("T", type0()), binder("E", type0())],
        vec![family(
            "Result",
            vec![
                constructor("Ok", vec![binder("value", var("T"))]),
                constructor("Err", vec![binder("error", var("E"))]),
            ],
        )],
    )
}

/// `data Scope { Piece; Part(part : Nat); Voice(part : Nat, voice : Nat) }`.
///
/// Where a constructed fact sits in the score's *structure* — the same three
/// places [`musa_score::scope::Scope`] names, and never a position in time.
///
/// # Why this is declared and `Origin` is registered
///
/// `02-core-calculus.md` §5.7 requires that every fact a track builtin
/// constructs has "the requested scope" and "a complete `Origin`", and neither
/// is something a `fn` rule can invent — so `play` takes both. They arrive by
/// different doors because they are different kinds of thing. A scope is finite
/// data with three cases and nothing hidden behind them, so it is *declared* and
/// a program may match on it. An origin is a source span, a definition span, a
/// declaration ordinal, and an expansion path over a growing set of steps: the
/// compiler owns its representation, no program takes one apart, and §5.8's D1
/// inertness test therefore puts it in the registry. That is the same test that
/// declared `Bool` and registered `Pitch`, applied to the two arguments of one
/// builtin.
fn scope_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "Scope",
            vec![
                constructor("Piece", Vec::new()),
                constructor("Part", vec![binder("part", var("Nat"))]),
                constructor("Voice", vec![binder("part", var("Nat")), binder("voice", var("Nat"))]),
            ],
        )],
    )
}

/// `data Unit { Only }`.
///
/// One constructor and no fields, so `Unit` has exactly one value.
/// `../../rules/across-stages/03-machine-calculus.md` §2 needs a type with that
/// property in one place and one only: `drop : Machine<K,A,Unit>` is how a
/// machine says its output carries no information, and a result type with two
/// values would let a `drop` be observed.
///
/// Declared rather than registered for `Bool`'s reason one constructor shorter:
/// a base type with one opaque value would make `Unit`'s value unwritable, and
/// §2's `feedback` threads a stored value through a machine — a piece that
/// stores nothing has to be able to write the value it stores.
fn unit_data() -> RawData {
    data(Vec::new(), vec![family("Unit", vec![constructor("Only", Vec::new())])])
}

/// `data Pair (A : Type 0) (B : Type 0) { Both(first : A, second : B) }`.
///
/// `../../rules/across-stages/03-machine-calculus.md` §2's `(A, D)`.  Four of
/// its eight forms are written over it — `beside` pairs both ports, `copy`
/// answers one, `swap` exchanges a pair's halves, and `feedback` threads the
/// stored value through one on both sides — and the compiler has had it as
/// `Type::Product` with no source spelling since the old checker.
///
/// # Why a declared family and not a record type
///
/// Prompt 136 gave the core structural record types, and `{ first : A, second :
/// B }` would spell this without a declaration. It is the wrong shape, because
/// §2's pairs are *wiring*: `swap : Machine<K,(A,B),(B,A)>` says which side goes
/// where and says nothing about what the sides are called. Field names would be
/// invented here and then read by every program that matched on one, which is
/// the surface reading a wiring diagram as a record — and a record's fields are
/// its interface, so two pieces of wiring that named theirs differently would
/// stop being the same type. A declared family keeps the halves positional,
/// which is what they are, and gives a program a constructor to match on.
///
/// The field names exist anyway because a constructor's fields are binders, and
/// `first`/`second` are the two words that add nothing: they name the position,
/// which is the only thing here that is true.
fn pair_data() -> RawData {
    data(
        vec![binder("A", type0()), binder("B", type0())],
        vec![family(
            "Pair",
            vec![constructor(
                "Both",
                vec![binder("first", var("A")), binder("second", var("B"))],
            )],
        )],
    )
}

/// `data RowFault { Fault(List Nat, List Pc12) }`.
///
/// Why the twelve-tone row's failure has a name rather than a tuple: a rule
/// answers a [`musa_calculus::Datum`], which is a literal or a constructor, and an
/// anonymous pair is neither. That is the mechanism noticing something true —
/// the pair was a domain concept wearing a tuple. *Open Music Theory*
/// `108-basics-of-twelve-tone-theory.md` says what the two halves are: the order
/// positions whose pitch class already appeared, and the pitch classes the
/// sequence never names. Both, rather than a choice between them, because a
/// sequence of the wrong length can have either without the other.
///
/// It is the one declaration here that is musical rather than structural, and it
/// belongs to the compiler for the same reason `Pitch` does.
fn row_fault_data() -> RawData {
    data(
        Vec::new(),
        vec![family(
            "RowFault",
            vec![constructor(
                "Fault",
                vec![
                    binder("repeated", applied("List", [var("Nat")])),
                    binder("missing", applied("List", [var("Pc12")])),
                ],
            )],
        )],
    )
}

/// `data Fact { Note(…); Rest(…); … }` — the nineteen things a score says.
///
/// One case per [`crate::elaborate::FactKind`] case, with the same name and the
/// same fields in the same order. That correspondence is the declaration's whole
/// content and is checked by a law rather than trusted: a twentieth kind added
/// to the enum makes the law's exhaustive `match` fail to compile, so notation
/// cannot quietly become unwritable in the core.
///
/// # Why declared rather than registered
///
/// `02-core-calculus.md` §5.8's D1 test is inertness, and a fact fails it in the
/// direction that matters: it is finite data whose cases are the vocabulary of
/// notation itself, and `07-analysis.md`'s analyses are `match`es over exactly
/// this. `Scope` settled the same question for `play`'s other argument — "finite
/// data with three cases and nothing hidden behind them" — and this is that
/// answer nineteen cases wider.
///
/// Its *payloads* go the other way and are registered base types, because each
/// is a value this compiler owns a representation and a reading of: a
/// `NotatedDuration` records which noteheads spell a span, a `ChordSymbol` is a
/// parsed analysis, a `Progress` is an event track curve. A case that spelled one out
/// of its parts would let a program build a duration whose value and spelling
/// disagree, which is a fact no notation can be.
fn fact_data() -> RawData {
    let list = |name: &str| applied("List", [var(name)]);
    let option = |name: &str| applied("Option", [var(name)]);
    data(
        Vec::new(),
        vec![family(
            "Fact",
            vec![
                constructor(
                    "Note",
                    vec![
                        binder("pitch", var("Pitch")),
                        binder("duration", var("NotatedDuration")),
                        binder("articulations", list("Mark")),
                        binder("free", option("FreeDuration")),
                    ],
                ),
                constructor(
                    "Rest",
                    vec![
                        binder("duration", var("NotatedDuration")),
                        binder("articulations", list("Mark")),
                        binder("free", option("FreeDuration")),
                    ],
                ),
                constructor(
                    "Mark",
                    vec![binder("mark", var("Mark")), binder("argument", option("MarkArgument"))],
                ),
                constructor(
                    "Grace",
                    vec![
                        binder("pitch", var("Pitch")),
                        binder("articulations", list("Mark")),
                        binder("index", var("Nat")),
                    ],
                ),
                constructor("Slur", Vec::new()),
                constructor("Phrase", vec![binder("name", var("Text"))]),
                constructor("Tuplet", vec![binder("num", var("Nat")), binder("den", var("Nat"))]),
                constructor("Dynamic", vec![binder("mark", var("DynamicMark"))]),
                constructor(
                    "Hairpin",
                    vec![
                        binder("grows", var("Bool")),
                        binder("target", var("DynamicMark")),
                        binder("shape", var("Progress")),
                    ],
                ),
                // One `Key` rather than a tonic and a mode, because `key k;`
                // names a key a template was handed and the two halves of a
                // bound one cannot be taken apart by any word this compiler
                // registers. A written `key g major;` reaches the same
                // constructor through the same reading, so there is one answer
                // to what a key statement means rather than two.
                constructor("Key", vec![binder("key", var("Key"))]),
                constructor(
                    "Meter",
                    vec![binder("numerator", var("Nat")), binder("denominator", var("Nat"))],
                ),
                constructor("Clef", vec![binder("clef", var("Clef"))]),
                constructor(
                    "Tempo",
                    vec![
                        binder("metronome", option("Metronome")),
                        binder("text", option("Text")),
                        binder("ramp", option("Ramp")),
                    ],
                ),
                constructor("Section", vec![binder("name", var("Text"))]),
                constructor("Harmony", vec![binder("symbol", var("ChordSymbol"))]),
                constructor(
                    "Repeat",
                    vec![
                        binder("times", var("Nat")),
                        binder("range", applied("Option", [applied("Pair", [var("Nat"), var("Nat")])])),
                    ],
                ),
                constructor(
                    "Mobile",
                    vec![binder("fragments", list("Text")), binder("order", list("Nat"))],
                ),
                constructor("Improvise", vec![binder("over", option("Text"))]),
                constructor(
                    "Ending",
                    vec![binder("bracket", var("Nat")), binder("pass", var("Nat"))],
                ),
            ],
        )],
    )
}

/// `data SyntaxStep (Context : Type 0) (Answer : Type 0) { private Step(run : Context -> Answer) }`.
///
/// One suspended recursive call into a proper child, sealed —
/// `11-quotation.md` §1's sealed step, moved off a base type and onto a family
/// whose constructor is private to [`PHASE`].
///
/// # Why a family, and why it holds a function
///
/// A step has to *capture*: which child it descends to, and under which
/// algebra. The only sound capture in this calculus is a closure, because the
/// core evaluates a rewrite's answer in the environment of the spine's
/// arguments and a λ in that answer becomes a value holding them. A base type
/// cannot hold a closure — its payload is opaque host data, and a payload
/// carrying a de Bruijn index would be meaningless the moment it left the spine
/// it was minted in. A declared family can hold a field of function type, so it
/// does.
///
/// # Why the seal survives
///
/// It used to be "this type is opaque because the compiler says so". It becomes
/// "this constructor is private to the phase module", which is 136a's own
/// mechanism, checked by the same filter as every other `private`. What a
/// transformer may do is unchanged: it receives steps and runs them, and there
/// is no spelling with which it could mint one for a node it chose. What is
/// *gained* is that the claim is now checked rather than asserted.
///
/// The family itself is public, because a transformer's own signature has to be
/// able to say `List (SyntaxStep C A)`. Hiding the type as well would hide the
/// argument type of the branch that receives it.
fn syntax_step_data() -> RawData {
    data(
        vec![binder("Context", type0()), binder("Answer", type0())],
        vec![family(
            "SyntaxStep",
            vec![sealed(
                "Step",
                vec![binder("run", Raw::pi(HERE, "context", var("Context"), var("Answer")))],
            )],
        )],
    )
}

/// The module the expansion phase's own declarations are written in.
///
/// One module and one number, because there is one seal. It is `1` rather than
/// `0` so that a context which has not said where it is standing — [`ModuleId`]
/// is an `Option` in a context — is never confused with this one by arithmetic.
pub(crate) const PHASE: ModuleId = ModuleId::new(1);

/// The module the file being compiled is written in.
///
/// A context standing nowhere is inside every module, which is right for this
/// compiler's own — [`crate::registry::owned`] declares the phase's families and
/// then keeps minting steps for the traversals. It is wrong for a document a
/// person wrote: standing nowhere, an adapter module could write
/// `SyntaxStep::Step(Text, Text, run)` and mint the seal `11-quotation.md` §1
/// says only the recursor mints. Naming a module for source is what turns
/// [`syntax_step_data`]'s `private` from a word into a check.
///
/// This is the *home* document's number, and prompt 162a is why it is only one
/// of several. `01-surface.md` §1.3 scopes `private` to "the module that
/// declares it" and a Musa module is a file, so each file a document reads from
/// gets an id of its own — [`crate::document::elaborate`] mints them from
/// [`IMPORTED`] as it walks. The file being compiled keeps this constant
/// because it is also the *viewer*: the question `private` answers is "may this
/// file name it", and this file is the one under the compiler.
pub(crate) const SOURCE: ModuleId = ModuleId::new(2);

/// The first number a document may mint for a file it imported.
///
/// The three reserved ids are stated together on purpose: `0` is a context
/// standing nowhere, [`PHASE`] is the expansion phase's own declarations, and
/// [`SOURCE`] is the file being compiled. Everything from here up is minted per
/// elaboration by [`crate::document::elaborate`] and means nothing outside it —
/// nothing persists one, nothing compares two documents' ids, and no hash,
/// snapshot, or export ever sees one.
pub(crate) const IMPORTED: u32 = 3;

/// The families the expansion phase declares, in the module that seals them.
///
/// Separate from [`structural`] not because the elaboration differs but because
/// the *context* does: these must be declared by a `Cx` standing in [`PHASE`],
/// or `private` would be a word with nothing behind it — 136a's own rule is that
/// a declaration written in no module hides from nobody.
pub(crate) fn phase() -> Vec<RawData> {
    vec![syntax_step_data()]
}

/// The families that name no base type, in dependency order.
///
/// These are declarable in a bare context, which is what makes them first: a
/// base type's *kind* may mention one — `Duration : Coordinate → Type 0` does —
/// so the registry cannot exist until these do.
///
/// Order matters within the group as well, because a later declaration may
/// mention an earlier one, and each is its own `data` group rather than one
/// large one: a group is the unit of mutual recursion, and putting seven
/// unrelated families in one would make positivity a question about all of them
/// at once.
pub(crate) fn structural() -> Vec<RawData> {
    vec![
        bool_data(),
        nat_data(),
        option_data(),
        list_data(),
        result_data(),
        scope_data(),
        unit_data(),
        pair_data(),
    ]
}

/// The families that name a base type, and so must be declared after one exists.
///
/// One today. It is a separate list rather than a comment on an ordering because
/// the ordering is a real constraint that would otherwise be discoverable only
/// by breaking it: `RowFault` holds a `List Pc12`, and `Pc12` is registered
/// rather than declared.
pub(crate) fn musical() -> Vec<RawData> {
    vec![row_fault_data(), fact_data()]
}

/// `Text.equal`, and its four siblings — the definitions `==` resolves to.
///
/// # Why the compiler owns them rather than the standard library
///
/// A **literal pattern** is core syntax. `match kind { "at_the_fifth" -> P5, … }`
/// has no constructors to split on — `Text` is a base type — so
/// [`crate::lower::values`] lowers it to a chain of equality tests, and
/// `x == y` lowers to `x.equal(y)` by `01-surface.md` §1.5's table. Both are
/// readings the compiler performs on source that imported nothing, and
/// `examples/named-answer.musa` is a piece that writes one and imports nothing. A
/// definition in `stdlib/` would make that piece's meaning depend on a line it
/// did not write — and the piece cannot even be told to write it, because the
/// line it would have to write is an import of the thing that gives `==` its
/// meaning.
///
/// # Why five names rather than one
///
/// The lowering does not know the subject's type: `3/8` is a `Ratio` token that
/// may stand at `Duration<WrittenTime>`, and a pattern is checked against the
/// type of what it matches rather than against its own spelling. Method syntax
/// runs *after* the subject is inferred, which is the only place the question
/// has an answer. So the lowering writes `equal` on the receiver and the
/// receiver's head picks the definition — which is also what prompt 164 needs
/// standing before it can collapse `text_equal` and its four siblings onto `==`.
///
/// The list is closed by the grammar rather than by taste:
/// `Lowering::matches_a_literal` fires for a string, a rational, a pitch, and an
/// interval, and a rational stands at `Ratio` or at `Duration<WrittenTime>`.
/// `Bool` and `Nat` are absent because they are *declared* families — `true` and
/// `7` are constructor patterns, split by the case tree, and a definition for
/// them would be a second way to ask a question ι already answers.
///
/// `Position<WrittenTime>` is absent for the opposite reason: `position_equal`
/// exists, but no literal spells a position and no source program can therefore
/// reach it. Prompt 164 adds it in the commit that gives `==` its meaning, where
/// it will have a caller.
///
/// Each body is the builtin itself rather than a λ around it. `Text.equal`'s
/// type is `text_equal`'s type, so η-contraction is not a trick here — the two
/// are the same term, and writing `λx y. text_equal(x, y)` would only add a
/// redex for the evaluator to undo.
fn equalities() -> Vec<RawTopLevel> {
    vec![
        namespaced("Text", "equal", None, var("text_equal")),
        namespaced("Ratio", "equal", None, var("ratio_equal")),
        namespaced("Duration", "equal", None, var("duration_equal")),
        namespaced("Pitch", "equal", None, var("pitch_equal")),
        namespaced("Interval", "equal", None, var("interval_equal")),
    ]
}

/// `List.fold_from_start` and `List.fold_from_end`, and the same two for
/// `Option`.
///
/// # Why the compiler owns these too
///
/// [`equalities`]'s argument, applied to a traversal. `List` and `Option` are
/// *prelude* families — no source file writes `data List` — and a definition in
/// a type's namespace belongs with the type. Declaring `List` here and
/// `List.fold_from_end` in `stdlib/` would put a family and its one lawful
/// traversal in two packages, and would make a piece that folds a list depend on
/// a line it did not write.
///
/// # Why two names rather than four
///
/// They replace `list_fold_from_start`, `list_fold_from_end`, `option_fold`, and
/// `nat_fold` — four spellings of one operation, which existed because the
/// checker they were written for had no source-level type parameters. A reader
/// who has learned `xs.fold_from_end(zero, step)` has learned every container
/// the language will ever add, because every container spells its traversal in
/// its own namespace under the same two words.
///
/// The step of a fold takes the accumulator as well as the element, so an
/// `Option` reader that only wants the held value writes a step that ignores one
/// argument. That is the price of two words over four and it is the right one:
/// the alternative reading — a container-shaped `Option` eliminator beside the
/// fold — is the per-type function this replaces, one type later.
///
/// Prompt 156 gives every declared family a generated eliminator, and these four
/// are then the recursor with its arguments in the order a reader writes them.
/// They are written out here because that prompt has not run.
fn traversals() -> Vec<RawTopLevel> {
    vec![
        namespaced(
            "List",
            "fold_from_start",
            Some(folding(list_of(var("A")), arrow(var("B"), arrow(var("A"), var("B"))))),
            list_from_start(),
        ),
        namespaced(
            "List",
            "fold_from_end",
            Some(folding(list_of(var("A")), arrow(var("A"), arrow(var("B"), var("B"))))),
            list_from_end(),
        ),
        namespaced("List", "map", Some(mapping()), list_map()),
        namespaced(
            "Option",
            "fold_from_start",
            Some(folding(option_of(var("A")), arrow(var("B"), arrow(var("A"), var("B"))))),
            option_fold(applied("step", [var("zero"), var("found")])),
        ),
        namespaced(
            "Option",
            "fold_from_end",
            Some(folding(option_of(var("A")), arrow(var("A"), arrow(var("B"), var("B"))))),
            option_fold(applied("step", [var("found"), var("zero")])),
        ),
    ]
}

/// `{A : Type} → {B : Type} → List A → (A → B) → List B`.
///
/// Beside the two folds because it is the one thing a fold does not say as well
/// — not in cost, which is the same walk, but in what the reader has to write.
/// `stdlib/src/list.musa` says the rest of it: an answer built by folding and
/// *appending* costs the square of the length, so what belongs in a namespace is
/// the traversal that costs the length.
fn mapping() -> Raw {
    let body = arrow(list_of(var("A")), arrow(arrow(var("A"), var("B")), list_of(var("B"))));
    Raw::parameter_pi(HERE, "A", type0(), Raw::parameter_pi(HERE, "B", type0(), body))
}

/// `List.map`, written over `List.fold_from_end` and `Cons`.
///
/// Over the fold rather than over a second `rec`, and the reason is the claim
/// prompt 146 makes: the two folds are enough to write the library. A `map` that
/// recursed again would be a second traversal of a family that has one, which is
/// the shape the deleted trait's derived methods existed to avoid and the shape
/// prompt 156's generated recursor will make impossible to want.
fn list_map() -> Raw {
    Raw::parameter_lam(
        HERE,
        "A",
        Raw::parameter_lam(
            HERE,
            "B",
            lam(
                "source",
                lam(
                    "shape",
                    applied(
                        "List.fold_from_end",
                        [
                            var("source"),
                            applied("List.Empty", [var("B")]),
                            lam(
                                "first",
                                lam(
                                    "rest",
                                    applied("List.Cons", [var("B"), applied("shape", [var("first")]), var("rest")]),
                                ),
                            ),
                        ],
                    ),
                ),
            ),
        ),
    )
}

/// `{A : Type} → {B : Type} → container → B → step → B`.
///
/// The element and the answer are both implicit, and both are read off the
/// arguments a call supplies: `xs.fold_from_end(seed, combine)` learns `A` from
/// the receiver and `B` from `seed`. Writing either would make the receiver's
/// own type something a call had to repeat.
fn folding(container: Raw, step: Raw) -> Raw {
    let body = arrow(container, arrow(var("B"), arrow(step, var("B"))));
    Raw::parameter_pi(HERE, "A", type0(), Raw::parameter_pi(HERE, "B", type0(), body))
}

/// `λ{A}. λ{B}. λsource. λzero. λstep. …`, the shape all four traversals share.
fn traversal(body: Raw) -> Raw {
    Raw::parameter_lam(
        HERE,
        "A",
        Raw::parameter_lam(HERE, "B", lam("source", lam("zero", lam("step", body)))),
    )
}

/// `List.fold_from_start`: accumulate forwards, so the accumulator travels with
/// the traversal and a reader written this way runs in the direction the source
/// is written in.
fn list_from_start() -> Raw {
    let walk_ty = arrow(list_of(var("A")), arrow(var("B"), var("B")));
    traversal(Raw::annotated_bind(
        HERE,
        "walk",
        walk_ty.clone(),
        Raw::rec(
            HERE,
            "walk",
            walk_ty,
            lam(
                "xs",
                lam(
                    "built",
                    matching(
                        var("xs"),
                        vec![
                            arm(vec![con("List.Empty", [])], var("built")),
                            arm(
                                vec![con("List.Cons", [held("first"), held("rest")])],
                                applied("walk", [var("rest"), applied("step", [var("built"), var("first")])]),
                            ),
                        ],
                    ),
                ),
            ),
        ),
        applied("walk", [var("source"), var("zero")]),
    ))
}

/// `List.fold_from_end`: the catamorphism, which is what the generated recursor
/// will be.
fn list_from_end() -> Raw {
    let fold_ty = arrow(list_of(var("A")), var("B"));
    traversal(Raw::annotated_bind(
        HERE,
        "walk",
        fold_ty.clone(),
        Raw::rec(
            HERE,
            "walk",
            fold_ty,
            lam(
                "xs",
                matching(
                    var("xs"),
                    vec![
                        arm(vec![con("List.Empty", [])], var("zero")),
                        arm(
                            vec![con("List.Cons", [held("first"), held("rest")])],
                            applied("step", [var("first"), applied("walk", [var("rest")])]),
                        ),
                    ],
                ),
            ),
        ),
        applied("walk", [var("source")]),
    ))
}

/// Either fold over `Option`: the container that holds at most one thing.
///
/// This is what `option_fold(fallback, present, value)` was. Neither fold
/// recurses, because there is nothing to recurse into — an `Option` is one step
/// of a list — and the two differ only in which side of `step` the held value
/// arrives on, which is what `present` says. A reader that wants the held value
/// alone writes `value.fold_from_end(fallback, fn (found, _) { … })`, and the
/// ignored argument is the accumulator that a one-element container has no
/// second use for.
fn option_fold(present: Raw) -> Raw {
    traversal(matching(
        var("source"),
        vec![
            arm(vec![con("Option.None", [])], var("zero")),
            arm(vec![con("Option.Some", [held("found")])], present),
        ],
    ))
}

/// One definition in a type's namespace: `Head.member`, spelled as the dotted
/// name `01-surface.md` §1.5 resolves.
///
/// The dot is the whole mechanism. There is no instance table, no dictionary,
/// and no binding form here that an ordinary definition does not already have —
/// `x.member(…)` finds this by taking the head of `x`'s type and looking up one
/// name.
fn namespaced(head: &str, member: &str, ty: Option<Raw>, value: Raw) -> RawTopLevel {
    RawTopLevel {
        origin: HERE,
        name: Arc::from(format!("{head}.{member}")),
        visibility: Visibility::Public,
        module: None,
        ty,
        value,
    }
}

/// `cx` with every [`equalities`] definition in it.
///
/// Last in [`crate::registry::owned`] and necessarily so: each body is a
/// δ-builtin's name, and a name resolves only once the registry holding it is
/// the context's.
///
/// # Errors
///
/// As [`musa_calculus::declare_program`] — in practice never, since the
/// definitions are this module's own and a failure here is a compiler defect
/// rather than a program's.
pub(crate) fn equality(cx: &Cx) -> Result<Cx, ElabError> {
    defining(cx, equalities())
}

/// `cx` with every [`traversals`] definition in it.
///
/// # Errors
///
/// As [`equality`].
pub(crate) fn collections(cx: &Cx) -> Result<Cx, ElabError> {
    defining(cx, traversals())
}

/// `cx` extended by one group of compiler-owned definitions.
///
/// One [`RawProgram`] rather than one call each, because a program is the unit
/// that orders its own definitions: `List.fold_from_end` may name
/// `List.fold_from_start` without this module having to say which comes first.
fn defining(cx: &Cx, definitions: Vec<RawTopLevel>) -> Result<Cx, ElabError> {
    let declared = musa_calculus::declare_program(
        cx,
        &RawProgram {
            families: Vec::new(),
            definitions,
        },
    )?;
    Ok(cx.defining(&declared))
}

/// `cx` with the expansion phase's one definition in scope.
///
/// ```text
/// run_syntax_step
///   : {Context : Type 0} → {Answer : Type 0}
///   → Context → SyntaxStep Context Answer → Answer
/// run_syntax_step = fn (context, step) {
///     match step { SyntaxStep.Step(run) => run(context) }
/// }
/// ```
///
/// Seventeen phase rows in, sixteen registrations and this out.
/// `run_syntax_step` is not a registered builtin because a projection is not a
/// compiler-owned operation: it hides nothing, which is the test every
/// `SYNTAX_OWNERSHIP` row states for itself, and
/// [`Registry::new`](musa_calculus::Registry::new) would have refused it in any
/// case — its target is a *declared* family, and a rewrite over one is the
/// second ι-rule that check exists to catch. It is defined instead, by the same
/// [`musa_calculus::declare_program`] that reads a library's own definitions.
///
/// **Only where a phase reads.** §5.9 keeps the two vocabularies apart, so a
/// piece that wrote `run_syntax_step` must get the unknown name it earned;
/// [`crate::document::elaborate`] calls this for a reading that holds a phase
/// source and for no other.
///
/// The step's `run` field is private to the phase module, so the `match`
/// resolves only because `cx` stands in no module and is therefore inside every
/// one. A transformer's own module is not, which is the seal.
///
/// # Errors
///
/// [`ElabError`] when `SyntaxStep` is not declared in `cx`, or when the
/// definition does not check at its own type — a compiler defect either way.
pub(crate) fn expansion(cx: &Cx) -> Result<Cx, ElabError> {
    defining(cx, vec![run_syntax_step()])
}

/// `run_syntax_step`, written out. See [`expansion`].
fn run_syntax_step() -> RawTopLevel {
    let sealed = calling(var("SyntaxStep"), [var("Context"), var("Answer")]);
    let ty = Raw::parameter_pi(
        HERE,
        "Context",
        type0(),
        Raw::parameter_pi(
            HERE,
            "Answer",
            type0(),
            Raw::pi(
                HERE,
                "context",
                var("Context"),
                Raw::pi(HERE, "step", sealed, var("Answer")),
            ),
        ),
    );
    let ran = arm(
        vec![RawPattern::constructor(
            HERE,
            "SyntaxStep.Step",
            [RawPattern::bind(HERE, "run")],
        )],
        calling(var("run"), [var("context")]),
    );
    // The implicit binders are written rather than left to insertion: a λ is
    // checked against the Π it stands at, and the two type parameters are the
    // ones the body's `SyntaxStep Context Answer` names.
    let value = Raw::parameter_lam(
        HERE,
        "Context",
        Raw::parameter_lam(
            HERE,
            "Answer",
            lam("context", lam("step", matching(var("step"), vec![ran]))),
        ),
    );
    RawTopLevel {
        origin: HERE,
        name: Arc::from("run_syntax_step"),
        visibility: Visibility::Public,
        // Written *in* [`PHASE`], because its body matches on the private case
        // and nothing else may. Public, so a transformer standing in
        // [`SOURCE`] calls it — which is the whole shape of the seal: the one
        // operation that opens a step is the module's own, and it exports the
        // answer rather than the constructor.
        module: Some(PHASE),
        ty: Some(ty),
        value,
    }
}

/// The term naming `name` in `cx`, for a caller assembling a builtin's type.
///
/// This is how a δ signature spells `Option Ratio`: the family constant is read
/// back out of the context that declared it, exactly as a source program's
/// `Option` is, so there is one path from a spelling to a constant and not two.
///
/// # Errors
///
/// As [`musa_calculus::infer`] — in practice [`musa_calculus::Refusal::Unbound`] for a
/// name this module did not declare.
pub(crate) fn constant(cx: &Cx, name: &str) -> Result<Term, ElabError> {
    musa_calculus::infer(cx, &Raw::var(HERE, name)).map(|(term, _)| term)
}

// ---- raw-syntax helpers ----
//
// The same shapes `musa-calculus`'s own suites build, spelled once here so that the
// declarations above read as declarations rather than as struct literals.

fn binder(name: &str, ty: Raw) -> RawBinder {
    RawBinder {
        name: Arc::from(name),
        ty,
    }
}

fn constructor(name: &str, fields: Vec<RawBinder>) -> RawConstructor {
    RawConstructor {
        origin: HERE,
        name: Arc::from(name),
        visibility: Visibility::Public,
        fields,
        chosen: Vec::new(),
    }
}

/// A constructor nothing outside its own module may write.
///
/// The one place `private` appears in this module, and the only kind of seal
/// this compiler has now: [`syntax_step_data`]'s `Step`.
fn sealed(name: &str, fields: Vec<RawBinder>) -> RawConstructor {
    RawConstructor {
        visibility: Visibility::Private,
        ..constructor(name, fields)
    }
}

fn family(name: &str, constructors: Vec<RawConstructor>) -> RawFamily {
    RawFamily {
        name: Arc::from(name),
        visibility: Visibility::Public,
        indices: Vec::new(),
        constructors,
    }
}

fn data(params: Vec<RawBinder>, families: Vec<RawFamily>) -> RawData {
    RawData {
        origin: HERE,
        params,
        families,
    }
}

fn var(name: &str) -> Raw {
    Raw::var(HERE, name)
}

fn applied(head: &str, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    calling(var(head), arguments)
}

fn calling(head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |function, argument| Raw::app(HERE, function, argument))
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(HERE, "_", domain, codomain)
}

fn lam(name: &str, body: Raw) -> Raw {
    Raw::lam(HERE, name, body)
}

fn matching(subject: Raw, arms: Vec<RawArm>) -> Raw {
    Raw::match_on(HERE, [subject], arms)
}

fn arm(patterns: Vec<RawPattern>, body: Raw) -> RawArm {
    RawArm { patterns, body }
}

fn con(name: &str, fields: impl IntoIterator<Item = RawPattern>) -> RawPattern {
    RawPattern::constructor(HERE, name, fields)
}

fn held(name: &str) -> RawPattern {
    RawPattern::bind(HERE, name)
}

fn list_of(element: Raw) -> Raw {
    applied("List", [element])
}

fn option_of(element: Raw) -> Raw {
    applied("Option", [element])
}

fn type0() -> Raw {
    Raw::universe(HERE, Sort::ZERO)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        clippy::panic,
        reason = "a law that cannot fail loudly is not a law"
    )]

    use musa_calculus::{Cx, Raw, Sort, Term};

    use super::{HERE, applied, constant, structural, var};

    /// The structural families elaborate in a bare context.
    ///
    /// Worth a test rather than a comment because positivity, universe levels,
    /// and constructor-field scoping are all checked by `musa-calculus` and all
    /// silently absent until something declares them. It is also the claim
    /// [`structural`] makes by being separate from [`super::musical`]: these
    /// name no base type, so no registry is needed to declare them.
    ///
    /// The musical group is not tested here, because it is not declarable here.
    /// Its laws are in the registry's suite, where a base type exists.
    #[test]
    fn the_structural_families_elaborate_with_no_registry() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_calculus::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        for name in ["Bool", "Nat", "Option", "List", "Result"] {
            constant(&cx, name).unwrap_or_else(|_| panic!("`{name}` is declared"));
        }
    }

    /// A constructor of each structural family is nameable by its qualified
    /// spelling.
    ///
    /// `Datum::Case` names a constructor as `Family.Case` and nothing else, so a
    /// δ-rule answering `Option.Some` is writing this string. If the qualified
    /// spelling here and the one `musa-calculus` builds ever disagreed, every such
    /// rule would answer `MisfitAnswer` at reduction rather than failing to
    /// build.
    #[test]
    fn each_constructor_is_nameable_by_its_qualified_spelling() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_calculus::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        for name in [
            "Bool.True",
            "Nat.Succ",
            "Option.Some",
            "List.Cons",
            "Result.Ok",
            "Result.Err",
        ] {
            constant(&cx, name).unwrap_or_else(|_| panic!("`{name}` is declared"));
        }
    }

    /// The prelude's families are writable the way source writes them: a
    /// constructor names its case and supplies its fields, and never its
    /// family's parameters.
    ///
    /// `02-core-calculus.md` §2 states the rule and `musa-calculus`'s own suite
    /// proves it over families that suite declares. What this law adds is that
    /// the families *this* module declares are the shape it fires on. Three of
    /// the five carry parameters, and a parameterized family whose constructors
    /// could only be written `Option.Some Nat 0` is one no `.musa` file could
    /// use — `None` and `Some(register)` are what
    /// `stdlib/src/context.musa` actually writes.
    #[test]
    fn a_prelude_constructor_is_written_without_its_family_s_parameters() {
        let mut cx = Cx::new();
        for declaration in structural() {
            let group = musa_calculus::declare(&cx, &declaration).expect("a compiler declaration elaborates");
            cx = cx.declaring(&group);
        }
        let ty = |raw: &Raw| {
            musa_calculus::check(&cx, &Term::universe(HERE, Sort::ZERO), raw).expect("a prelude family is a type")
        };
        let zero = var("Nat.Zero");
        let programs: &[(&str, Raw, Raw)] = &[
            ("None", applied("Option", [var("Nat")]), var("None")),
            (
                "Some(0)",
                applied("Option", [var("Nat")]),
                applied("Some", [zero.clone()]),
            ),
            (
                "Ok(0)",
                applied("Result", [var("Nat"), var("Bool")]),
                applied("Ok", [zero.clone()]),
            ),
            (
                "Err(True)",
                applied("Result", [var("Nat"), var("Bool")]),
                applied("Err", [var("Bool.True")]),
            ),
            (
                "List.Cons(0, List.Cons(0, List.Empty))",
                applied("List", [var("Nat")]),
                applied(
                    "List.Cons",
                    [zero.clone(), applied("List.Cons", [zero, var("List.Empty")])],
                ),
            ),
        ];
        for (name, at, program) in programs {
            musa_calculus::check(&cx, &ty(at), program).unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }
}
