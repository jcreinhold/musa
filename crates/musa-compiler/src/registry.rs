//! The compiler's own operations, said in `musa-calculus`'s terms.
//!
//! `02-core-calculus.md` §5.8 splits every compiler-owned operation into four
//! families and gives each its own admissibility argument. Prompts 141b, 141c,
//! and 141d built the mechanism for them; this module fills its δ half. What was
//! a [`crate::phase::Type`] and a [`crate::phase::Shape`] becomes a
//! [`musa_calculus::Term`], and what was an arm of the old evaluator becomes a rule
//! over [`Datum`].
//!
//! # Nothing here decides anything new
//!
//! Every source signature is read off [`crate::phase::BUILTIN_OWNERSHIP`] rather
//! than retyped, and every phase signature off [`crate::phase::SyntaxOp`]'s own
//! scheme, because 106 signatures written a second time by hand is 106 chances
//! to write a different type from the one the old checker enforced. The rules are
//! the same arithmetic on the same domain modules; only the unwrapping of an
//! argument and the wrapping of an answer changed.
//!
//! # Why a base type and not a family
//!
//! A base type is *inert*: it contributes no ι-rule, so two closed values of it
//! are convertible exactly when the host says the payloads agree. That is the
//! right shape for a domain whose representation the compiler owns and no source
//! program takes apart — a `Pitch`, a `Scale`, a `Row12`. It is the wrong shape
//! for anything source pattern-matches on, which is why `Bool`, `Nat`, `Option`,
//! `List`, and `Result` are declared in [`crate::prelude`] instead.
//!
//! `Coordinate` and `Cat` are base types too, and that is D3 deciding rather than
//! taste. A δ-rule is a `fn` pointer, so the only type it can *write* is one it
//! can build with no context — a [`Base`] and a [`Literal`], both of which are
//! plain constructor calls, and never a family's constructor, which exists only
//! inside a [`Cx`]. `duration_of` answers a `Duration ⟨written⟩` and takes only a
//! `Ratio`, so if the index were a constructor it could not write down the type
//! of its own answer.
//!
//! # The order the context is built in
//!
//! Four stages, each needing the one before it, and written out because the
//! dependency is real rather than stylistic:
//!
//! 1. the structural families, which name no base type;
//! 2. a registry holding every base type and no builtin, so that a base type is
//!    nameable;
//! 3. the musical families, whose fields may name a base type — `RowFault` holds
//!    a `List Pc12`;
//! 4. the builtins, whose signatures may name anything above, and the full
//!    registry.
//!
//! # What is registered, and what is named and left
//!
//! `BUILTIN_OWNERSHIP` has 117 rows and 92 of them are δ. Eight more are §5.8's
//! *third* family and are registered in [`track`], over the `EventTrack` base
//! type this module registers for them. Eight of the nine machine rows are the
//! *fourth* family and are registered in [`machine`], over the `Machine` and
//! `Primitive` base types this module registers for them — as constructors,
//! because `03-machine-calculus.md` §2 gives them typing rules and no
//! reductions. The nine left are two kinds and neither is registrable here: the
//! eight collection eliminators are traversals over `Nat`, `List`, and
//! `Option`, which are *declared*, so [`musa_calculus::declare`] already generated
//! their recursors and [`Registry::new`] refuses a structural target that is
//! not a base type; and `primitive` is typed by a build-local registry rather
//! than by a signature, which [`machine::UNREGISTERED`] argues and prompt 142
//! owns.
//!
//! `SYNTAX_OWNERSHIP` has 17 rows and sixteen are registered: fourteen δ builders
//! here, and the two traversals in [`traversal`], which are §5.8's *second*
//! family — a structural eliminator over `Syntax`, which is a base type. The
//! seventeenth, `run_syntax_step`, is [`run_syntax_step`]: a projection, not a
//! compiler-owned operation, and its target is a declared family that
//! [`Registry::new`] would refuse. [`crate::registry::rules::UNREGISTERED`]
//! counts what is left, and the suite counts it again off the tables themselves.

#[cfg(test)]
mod laws;
mod machine;

/// The name the registration for one primitive unit is spelled by.
///
/// Re-exported because [`crate::lower::values`] is the one reader outside this
/// module: a written `primitive("scale", 1, c)` selects a signature, and the
/// selection is a name. The module stays private — nothing else in it is a
/// caller's business.
pub(crate) use machine::unit_spelling;

/// What a machine value is, read back: the ports its type decides, and the
/// nodes its normal form describes.
///
/// Two names rather than one because a caller asks them in that order and for
/// two different reasons. A machine's ports are settled by its *type*, so
/// [`machine_ports`] is what says a definition is a machine at all — and it
/// answers before the value has been normalized, which is what keeps a document
/// of a hundred definitions from evaluating all of them to find the two that
/// are machines.
pub(crate) use machine::{is_step_tag, nodes as machine_nodes, ports as machine_ports};
mod notation;
mod rules;
mod track;
mod traversal;

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, LazyLock};

use musa_calculus::{
    Base, Builtin, Cx, Datum, ElabError, Literal, Operator, Origin, Payload, Refusal, Registry, Sort, Term,
};
#[cfg(test)]
use musa_calculus::{Index, Raw, RawArm, RawPattern};

/// The old table's name for one inert domain, renamed on the way in.
///
/// `Base` means two things in this module — the registration `musa-calculus` holds,
/// and the leaf the signature table names — and this is the one that is going
/// away when prompt 142 deletes the old checker. Naming it for what it is keeps
/// the translation's two sides legible in a single line.
use crate::phase::Base as Leaf;
use crate::phase::{BUILTIN_OWNERSHIP, Coordinate, Family, SYNTAX_OWNERSHIP, Shape, Type};

/// Where a term this module builds comes from.
///
/// Compiler-owned, like [`crate::prelude`]'s: no source file wrote `pitch_of`'s
/// signature, so there is no line for a diagnostic to point at.
const HERE: Origin = Origin::UNKNOWN;

/// A musical value as a core literal's payload.
///
/// One implementation rather than one per domain. [`Payload`]'s three methods
/// each need one thing of the value it wraps — `PartialEq` for [`Payload::same`],
/// [`fmt::Display`] for [`Payload::shown`], and `'static` for
/// [`Payload::as_any`] — and every inert domain in this compiler already has all
/// three. Adding a domain is therefore adding a row to [`bases`] and nothing
/// else.
///
/// `Send + Sync` is inherited rather than chosen: a literal rides inside a
/// [`Term`], and terms cross threads wherever more than one document compiles at
/// once.
#[derive(Debug, PartialEq, Eq)]
struct Domain<T>(T);

impl<T> Payload for Domain<T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Two payloads agree when they are the same domain and the same value.
    ///
    /// The downcast can fail even though [`Literal`] compares types first,
    /// because two domains could in principle share a base type's name.
    /// Answering `false` there is the behaviour [`Payload::same`] documents and
    /// is an equivalence relation either way.
    fn same(&self, other: &dyn Payload) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| self == other)
    }

    fn shown(&self) -> String {
        self.0.to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// One `Ratio` literal, as the exact fraction §1.5's index language works in.
///
/// [`musa_calculus::Measures`] registered on the `Ratio` base type. The
/// downcast is the same one every δ-rule over a rational already does; `None`
/// is a literal that is not one, which the core reads as "not an index".
///
/// `Nat` registers nothing, and needs to: a counting family holds its count in
/// the core's own `Numeral`, so §5.8's opacity never arises for it.
fn exact_index(literal: &Literal) -> Option<(i128, i128)> {
    let value = literal
        .payload()
        .as_any()
        .downcast_ref::<Domain<num_rational::Ratio<i64>>>()?;
    Some((i128::from(*value.0.numer()), i128::from(*value.0.denom())))
}

/// The literal holding `value` at base type `ty`.
pub(crate) fn literal<T>(ty: Term, value: T) -> Literal
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    Literal::new(ty, Arc::new(Domain(value)))
}

/// The value inside a literal `Datum`, if it is of domain `T`.
///
/// The inverse of [`literal`], and the only way a rule reads its argument. It
/// answers `None` rather than panicking for the reason a rule answers `None` at
/// all: a builtin applied to something its signature does not admit is a caller
/// defect that the core reports, not a crash.
fn domain<T>(datum: &Datum) -> Option<&T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    let Datum::Lit(ref value) = *datum else {
        return None;
    };
    held(value)
}

/// The value inside a literal, if it is of domain `T`.
///
/// The same reading as [`domain`], one layer in. A δ-rule is handed a [`Datum`]
/// and a structural rewrite is handed the [`Literal`] it fired on, so both
/// spellings of "the target" reach the same downcast rather than two.
///
/// Visible to the crate because it is [`literal`]'s inverse and a normal form is
/// read the same way a rule's argument is: whoever puts a `Syntax` or a
/// `Template` into the core takes one back out of the term the core reduced to,
/// and a second downcast written beside this one would be a second answer to
/// "what domain is this literal of".
pub(crate) fn held<T>(value: &Literal) -> Option<&T>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    value.payload().as_any().downcast_ref::<Domain<T>>().map(|held| &held.0)
}

/// The value of domain `T` a *closed normal form* holds.
///
/// [`held`] one layer further out, and the outermost one there is: a δ-rule is
/// handed a [`Datum`], a structural rewrite a [`Literal`], and a caller that has
/// finished elaborating a [`Term`]. The step this adds is the canonicity
/// `02-core-calculus.md` §5 promises — a closed term at a registered base type
/// normalizes to a literal of that type — so whoever put a track or a `Syntax`
/// into the core takes one back out here rather than matching on a term's shape
/// by hand at each site.
///
/// # Errors
///
/// [`ElabError::Malformed`] when the normal form is not a literal, or holds
/// another host's datum. Both are defects in *this* crate rather than in the
/// source: the term was checked at `T`'s own type before it was normalized, so a
/// program cannot reach either by being wrong.
pub(crate) fn read_back<T>(normal: &Term) -> Result<&T, ElabError>
where
    T: PartialEq + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    // `type_name` rather than a name each caller passes: the sentence is about a
    // Rust domain that disagreed with its own registration, and a caller free to
    // name it could name it wrongly.
    let broken = || ElabError::from(musa_calculus::Malformed::NotALiteral(std::any::type_name::<T>().into()));
    let musa_calculus::Shape::Lit(musa_calculus::Constant::Payload(ref value)) = *normal.shape() else {
        return Err(broken());
    };
    held::<T>(value).ok_or_else(broken)
}

/// The claim argument a checked normal form holds, at the shape the claim
/// registry declares for it.
///
/// Here rather than beside the `assert` reading because this is the registry's
/// own knowledge: a scale is a base literal, a count is the prelude's unary
/// `Nat`, and a list of ranges is `List.Cons` over `Pair.Both` over two `Pitch`
/// literals. Three different readings behind one question, so a caller that has
/// finished elaborating an argument does not have to know which — the same
/// service [`read_back`] performs for the one shape that *is* a literal.
///
/// `None` for a `shape` that is a **word** rather than a value, and for a normal
/// form that does not hold what the shape declares. Both are defects in this
/// crate rather than in a program, for [`read_back`]'s reason and
/// [`musa_score::assert::Claim::build`]'s: the two words are read where the `assert`
/// is written and never reach here, and every other argument was checked at the
/// type this shape declares before it was normalized. `None` is the belt to
/// those braces.
pub(crate) fn argument(
    cx: &Cx,
    shape: musa_score::assert::ParamType,
    normal: &Term,
) -> Option<musa_score::assert::Argument> {
    use musa_score::assert::{Argument, ParamType};

    match shape {
        ParamType::Scale => Some(Argument::Scale(*read_back::<musa_score::scale::Scale>(normal).ok()?)),
        ParamType::Chord => Some(Argument::Chord(
            *read_back::<musa_score::chord::ChordClass>(normal).ok()?,
        )),
        ParamType::Count => Some(Argument::Count(rules::nat(&musa_calculus::canonical(cx, normal)?)?)),
        ParamType::Ranges => {
            let written = musa_calculus::canonical(cx, normal)?;
            let ranges = rules::items(&written)?
                .into_iter()
                .map(|range| {
                    let (low, high) = rules::halves(range)?;
                    Some((
                        rules::read::<musa_score::pitch::WrittenPitch>(low)?,
                        rules::read::<musa_score::pitch::WrittenPitch>(high)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Argument::Ranges(ranges))
        }
        ParamType::Policy | ParamType::Rule => None,
    }
}

/// The context every Musa program is elaborated in.
///
/// Answers the context and nothing else, for [`crate::prelude::constant`]'s
/// reason: a caller that wants `Option` or `Pitch` asks for it by name, the same
/// way source code does, rather than being handed a struct of resolved terms it
/// would have to keep in step.
///
/// # Errors
///
/// [`ElabError`] when a declaration or a signature in this compiler is wrong,
/// which is a compiler defect rather than a program's, and is returned rather
/// than panicked on for the reason `musa-calculus` returns
/// [`musa_calculus::Malformed`].
pub(crate) fn owned() -> Result<Cx, ElabError> {
    let mut cx = Cx::new();
    for declaration in crate::prelude::structural() {
        let group = musa_calculus::declare(&cx, &declaration)?;
        cx = cx.declaring(&group);
    }
    // Declared *by* a context standing in the phase module and added to one that
    // is not, which is what makes `SyntaxStep.Step` private: the group carries
    // the module it was written in, and the compiler's own context — standing
    // nowhere — is inside every module and may still mint a step.
    for declaration in crate::prelude::phase() {
        let group = musa_calculus::declare(&cx.in_module(crate::prelude::PHASE), &declaration)?;
        cx = cx.declaring(&group);
    }
    let bases = bases();
    cx = cx.with_externs(Arc::new(Registry::new(bases.clone(), Vec::new())?));
    for declaration in crate::prelude::musical() {
        let group = musa_calculus::declare(&cx, &declaration)?;
        cx = cx.declaring(&group);
    }
    let builtins = builtins(&cx)?;
    // The namespaced definitions last, because `Text.equal`'s body *is* a
    // δ-builtin's name — `text_equal` and nothing else — so it cannot be defined
    // until the registry that resolves that name is the context's. `List`'s
    // traversals name no builtin, but they name `List.Cons`, and one ordering
    // for all of them is one thing to remember rather than two.
    let cx = crate::prelude::equality(&cx.with_externs(Arc::new(Registry::new(bases, builtins)?)))?;
    crate::prelude::collections(&cx)
}

/// A base type at `Type 0`.
///
/// The one place a plain base type's kind is written, so that the registration
/// and the type a rule builds for its own answer are the same expression rather
/// than two that agree today.
fn plain(name: &'static str) -> Base {
    Base::new(name, type0())
}

/// A base type indexed by the values of another base type.
fn indexed(name: &'static str, index: &'static str) -> Base {
    Base::new(name, Term::pi(HERE, "index", plain(index).term(HERE), type0()))
}

/// A base type at `Type 0` whose representation this compiler guarantees is
/// storable data.
///
/// `02-core-calculus.md` §1.2 lets a compiler-owned type be storable "only when
/// its owner guarantees that its hidden representation contains no closure and
/// supplies the exact encoding", and this crate is that owner: every domain
/// spelled with this is a spelling and a number, is what a `ScoreFact` already
/// carries across the event track boundary, and is what `12-payload-admission.md`
/// admits. The guarantee is written at the registration rather than in a second
/// table, so a base type added without it is refused wherever storability is
/// required — which is §1.2's own safe direction.
///
/// What is *not* spelled with it is as much of the statement: the phase-local
/// types of §5.9 (`Syntax`, `Cat`, `TokenKind`, `Delimiter`, `NodePath`,
/// `BindingPath`), the two quote bodies held whole (`Template`, `EventsTerm`),
/// and `Coordinate`, which is an index nothing stores. A step tag is not
/// storable either, and could not be: §2 gives it no values.
fn storable(name: &'static str) -> Base {
    plain(name).storable()
}

/// A storable base type indexed by the values of another.
///
/// The three tagged by a coordinate. §1.2 names two of them outright and gives
/// the third conditionally — "`EventTrack C A` is storable when `A` is" — and
/// this build has one payload, so the condition is discharged at the
/// registration rather than carried as a constraint nothing could vary.
fn storable_indexed(name: &'static str, index: &'static str) -> Base {
    indexed(name, index).storable()
}

/// A base type indexed by a step tag and two ports, all three of them *types*.
///
/// `Machine` and `Primitive`, and nothing else. The other two indexed base types
/// are indexed by the *values* of a base type, because a δ-rule writes their
/// index and a `fn` pointer can build a literal and not a constructor. Nothing
/// writes one of these: §2 gives its forms no reductions, so every index here is
/// supplied by a constructor's caller, and the ports are then whatever storable
/// types the piece is wiring together — arbitrary, which is exactly what a
/// literal index cannot be.
fn ported(name: &'static str) -> Base {
    Base::new(
        name,
        Term::pi(
            HERE,
            "step",
            type0(),
            Term::pi(HERE, "input", type0(), Term::pi(HERE, "output", type0(), type0())),
        ),
    )
}

/// Every inert domain, with the kind it is registered at.
///
/// Needs no context, which is the point: a rule reaches the same three helpers
/// to write the type of its own answer, so a registered base type and a
/// rule-built one cannot be two different terms.
fn bases() -> Vec<Base> {
    vec![
        // The two numeric leaves §1 keeps inert. `Ratio` is signed and has no
        // least element to descend to, so it has no recursor and its arithmetic
        // is δ; `Text` is opaque printable text, which is what lets it be the
        // error half of a `Result` without giving a builtin a second way to say
        // what went wrong.
        // `Ratio` also carries the rule for reading one of its literals as an
        // **index** (`02-core-calculus.md` §1.5). D1 keeps a payload opaque to
        // the core, so the compiler that put the fraction in is the only party
        // that can take it out, and `Bar(3/4)` is the program that asks.
        storable("Ratio").measuring(exact_index),
        storable("Text"),
        // The index on the two tagged rationals: an enumeration in everything
        // but its representation, inert because nothing matches on it and a
        // literal because a `fn` rule can build one.
        plain("Coordinate"),
        // The two tagged rationals, indexed by their coordinate so that a
        // written beat and a number of seconds do not add.
        storable_indexed("Duration", "Coordinate"),
        storable_indexed("Position", "Coordinate"),
        // §5.7's event track, indexed by the same coordinate for the same
        // reason. Inert by D1's test rather than by convenience: an
        // `EventTrack` is normalized, carries a versioned exact identity
        // (`../../rules/events/05-normalization.md`), and is taken apart by
        // nothing the source language can write — every operation over it is
        // one of the eight compiler-owned builtins in [`track`]. A *declared*
        // family of tracks would have to expose constructors the event track's
        // normal form does not admit, which is the opposite of what a track's
        // identity is for.
        //
        // One index and not two. The payload would have to be an index too for
        // `EventTrack ⟨written⟩ ⟨score⟩` to mean anything, and
        // [`Registry::check_finite_data`] admits a base type at a *literal*
        // index and at nothing else — so a payload index is a base type with a
        // literal per payload, and this compiler has one payload. The
        // registration fixes it, and a performance payload earns the second
        // index in the prompt that has a caller for it.
        storable_indexed("EventTrack", "Coordinate"),
        // Why a constructed fact exists. Inert for D1's reason and not for
        // convenience either: an origin is a source span, a definition span, a
        // declaration ordinal, and an expansion path, no program takes one
        // apart, and the Origin view reads a *compiled projection* one stage
        // down rather than a value of this type. `play` is its only reader, and
        // [`crate::prelude`]'s `Scope` is the other half of what §5.7 requires
        // a constructed fact to carry — declared rather than registered,
        // because a scope is finite data with three cases and nothing hidden.
        storable("Origin"),
        // `03-machine-calculus.md` §2's machine, and §1's registered unit.
        // Inert for the reason a base type usually is not: not because the
        // compiler owns a representation source may not take apart, but because
        // §2 gives these forms *no reductions at all*. A machine's application
        // is its value, so there is nothing for an ι-rule to do and nothing for
        // a program to match on — `03-machine-calculus.md` §3 gives a machine
        // one step and §5 prepares it, and neither is a source operation.
        //
        // Indexed by three types rather than by a literal, which is
        // [`ported`]'s argument. The eight forms over them are in [`machine`];
        // `primitive` is the ninth row and is registered once per unit the
        // build knows, because its ports come from a build-local registry
        // rather than from a signature — see [`machine::primitives`].
        ported("Machine"),
        ported("Primitive"),
        // The written domains.
        storable("Pitch"),
        storable("PitchClass"),
        storable("Interval"),
        storable("Key"),
        storable("Scale"),
        storable("Degree"),
        storable("Frame"),
        storable("ChordClass"),
        storable("Triad"),
        storable("Roman"),
        storable("Voicing"),
        // The notated domains: the payloads of [`crate::prelude`]'s `Fact`, and
        // registered for the reason the written domains above are. A composer
        // writes a clef, a dynamic, or a metronome mark; none of them is taken
        // apart by a program, and each carries a spelling and a reading this
        // compiler owns — a `NotatedDuration` holds the pieces a tie is written
        // with, a `ChordSymbol` holds a parsed analysis, a `Progress` is a
        // events curve. They are here rather than in `Fact`'s own cases because
        // D1's test is inertness and not size: a case that spelled a
        // `NotatedDuration` out of a `Ratio` and a `Text` would let a program
        // build one whose spelling and value disagree.
        storable("Mode"),
        storable("Clef"),
        storable("NotatedDuration"),
        storable("FreeDuration"),
        storable("Mark"),
        storable("MarkArgument"),
        storable("DynamicMark"),
        storable("Progress"),
        storable("Metronome"),
        storable("Ramp"),
        storable("ChordSymbol"),
        // The twelve-tone domains.
        storable("Pc12"),
        storable("PcSet12"),
        storable("Row12"),
        // The phase-local domains (§5.9). Registered beside the musical ones
        // rather than in a second registry, because a base type is a base type;
        // what §5.9 keeps separate is the *operations*, and those are two tables
        // in [`builtins`].
        plain("Cat"),
        // The one base type with an acceptance rule. `11-quotation.md` §1's
        // forgetting rule is stated as the checker's and not as an operation an
        // author writes, so it is registered *with the type it is about* and the
        // elaborator asks it where a direction exists — see [`rules::forgets`]
        // and [`syntax_carrier`].
        indexed("Syntax", "Cat").accepting(rules::forgets),
        plain("TokenKind"),
        plain("Delimiter"),
        plain("NodePath"),
        plain("BindingPath"),
        // A quote's body, held whole. Inert by §5.8's D1 test and not by
        // convenience: a template contributes no ι-rule, no source program takes
        // one apart — `11-quotation.md` gives it no eliminator and no spelling
        // beyond the two forms that build and read it — and two of them agree
        // exactly when the host says the bodies do. What the alternative would
        // have been is the argument: a *declared* family of templates would make
        // a quote's body something a program could match on, and matching on it
        // is reading provenance, which §4's second rule forbids.
        plain("Template"),
        // An event track quote's term, held whole, for exactly [`plain("Template")`]'s
        // reason one stage down: `01-surface.md` §7 gives it no eliminator and
        // no spelling beyond the form that writes it, and two of them agree
        // when the terms do. What a *declared* family of event-track terms would buy
        // is a program that could match on the assembly a composer wrote by
        // hand, which is reading provenance by another route.
        plain("EventsTerm"),
    ]
    .into_iter()
    // The step tags this build's units count in, which are types with no
    // values and therefore nothing a list written here could name: see
    // [`machine::step_tags`].
    .chain(machine::step_tags())
    .collect()
}

/// The literal holding `value` at plain base type `name`, for a payload with no
/// `Display`.
///
/// [`literal`]'s companion, and the one entry the lowering has into
/// [`notation::Opaque`]. Nine of `Fact`'s payload types have no written
/// spelling, and a module that builds facts out of source needs to write them
/// without the wrapper becoming part of its vocabulary.
pub(crate) fn opaque_literal<T>(name: &'static str, value: T) -> Literal
where
    T: Clone + PartialEq + fmt::Debug + Send + Sync + 'static,
{
    literal(plain_type(name), notation::Opaque(value))
}

/// `nothing` — the empty track, as a literal a lowering can embed.
///
/// The re-export exists so that "the seed of a fold is `nothing`" is one name at
/// the call site rather than a path through the module that registers the words
/// beside it.
pub(crate) fn empty_track() -> Literal {
    notation::nothing()
}

/// `Type 0`, which every base type but the two indexed ones is registered at.
fn type0() -> Term {
    Term::universe(HERE, Sort::ZERO)
}

/// The registered base type named `name`.
///
/// [`bases`] is the one table and this is how a *term* reaches it, which the
/// three builders below need and [`plain`] cannot give them. A base type
/// carries its owner's rules — §1.2's storability and §1.5's reading of its
/// literals as index values — and [`Base`]'s equality is by *name*, so a second
/// `Base` spelled the same and registered with none of them is accepted
/// everywhere the registration is and then answers differently when asked.
///
/// That is what `Bar(3/4)` met. `3/4` lowered to a literal whose type was a
/// freshly built `Ratio` with `measuring` left behind, so §1.5's reader found a
/// literal at a base type that registers no measure and reported an exact
/// fraction as not an index — while `fn q() -> Ratio { 3/4 }` checked, because
/// conversion only ever compared the name.
///
/// A name with no registration is a defect in this module and never in a
/// program — every caller writes a `&'static str` this file also registers — so
/// it is asserted rather than reported, and every test run is where the
/// assertion is made. Release keeps the fresh `Base` the builder would have
/// made anyway, which is the behaviour this replaced.
fn registered(name: &'static str) -> Base {
    static TABLE: LazyLock<HashMap<Box<str>, Base>> = LazyLock::new(|| {
        bases()
            .into_iter()
            .map(|base| (Box::from(&**base.name()), base))
            .collect()
    });
    debug_assert!(TABLE.contains_key(name), "`{name}` is a registered base type");
    TABLE.get(name).cloned().unwrap_or_else(|| plain(name))
}

/// The term naming a plain base type.
pub(crate) fn plain_type(name: &'static str) -> Term {
    registered(name).term(HERE)
}

/// The term naming `Duration`, `Position`, or `EventTrack` at one coordinate.
pub(crate) fn tagged_type(name: &'static str, which: Coordinate) -> Term {
    Term::app(
        HERE,
        registered(name).term(HERE),
        literal(plain_type("Coordinate"), which).term(HERE),
    )
}

/// The term naming `Syntax` at one category.
pub(crate) fn syntax_type(cat: crate::quote::Cat) -> Term {
    Term::app(HERE, registered("Syntax").term(HERE), category_literal(cat).term(HERE))
}

/// The literal one coordinate is written as, at base type `Coordinate`.
///
/// The index of `Duration` and `Position`, and the one place a lowered
/// `Duration<WrittenTime>` and a registered signature's `DURATION` can agree —
/// [`tagged_type`] builds the same literal, so the two spellings are one
/// expression rather than two that match today.
pub(crate) fn coordinate_literal(which: Coordinate) -> Literal {
    literal(plain_type("Coordinate"), which)
}

/// The literal one syntax category is written as, at base type `Cat`.
pub(crate) fn category_literal(cat: crate::quote::Cat) -> Literal {
    literal(plain_type("Cat"), cat)
}

/// The literal one origin is written as, at base type `Origin`.
///
/// The only way into `play`'s first argument, and the reason `play` has one: a
/// source span, a definition span, and a `DeclarationId` are not things a `fn`
/// pointer can invent, so the caller supplies them exactly as it supplies
/// `instantiate_quote`'s anchor (`11-quotation.md` §3). `instanced` takes one
/// for the same reason and reads its expansion path.
///
/// The payload is [`track::Provenance`] and cannot be anything else, for
/// [`token_kind_literal`]'s reason: `play` reads its argument back at that type,
/// so a literal built from a bare [`musa_score::origin::Origin`] would be an origin
/// no rule can read. The wrapper is reachable only through this function —
/// beside [`opaque_literal`] rather than through it, because a `Provenance` has
/// a `Display` of its own showing the span it points at.
pub(crate) fn origin_literal(origin: musa_score::origin::Origin) -> Literal {
    literal(plain_type("Origin"), track::Provenance(origin))
}

/// The literal one quote's body is written as, at base type `Template`.
///
/// The construction site rides with the body, because that is what it is a
/// property of: `Derived`'s three fields are the anchor, the quotation, and the
/// position, and only the first is an argument. Two quotes with identical bodies
/// at one anchor must still build distinguishable nodes (`11-quotation.md` §3),
/// so the counter cannot be recovered from the template and cannot be shared.
pub(crate) fn template_literal(template: crate::quote::Template, quotation: u32) -> Literal {
    literal(plain_type("Template"), rules::Quotation { template, quotation })
}

/// The literal one events quote is written as, at base type `EventsTerm`.
///
/// The hole names ride with the term because they are what `spliced` binds: the
/// material arrives as a list, and the *i*th member is bound to the *i*th name.
/// Recovering them from the term instead would mean searching it for names this
/// compiler minted, which is the same fact stored twice and one place for the
/// two to disagree.
pub(crate) fn events_literal(term: track::Quoted, holes: Vec<String>) -> Literal {
    literal(plain_type("EventsTerm"), track::Assembly { term, holes })
}

/// The literal one token kind is written as, at base type `TokenKind`.
///
/// The payload is [`rules::Kind`] and cannot be anything else: `token_kind_equal`
/// reads its arguments back at that type, so a literal built from the bare
/// `musa_syntax::SyntaxKind` would compare equal to nothing. That is why the
/// wrapper is reachable only through this function — a caller outside the
/// registry can write the literal without being able to write a different one.
pub(crate) fn token_kind_literal(kind: musa_syntax::SyntaxKind) -> Literal {
    literal(plain_type("TokenKind"), rules::Kind(kind))
}

/// The core type a signature shape denotes.
///
/// This is the whole of "the signature table is reused rather than retyped".
/// [`Shape`] already says what each of the 92 δ entries takes and answers, with
/// no arrow and with storability checked where the table is written; what it
/// lacks is a spelling in the core's terms, and that is one recursion rather
/// than 92 declarations.
///
/// A base type applied to an index is an application, and nothing about it is
/// special-cased: `Duration` is registered at `Coordinate → Type 0`, so
/// `DURATION` denotes `Duration ⟨written⟩` by the same rule that makes
/// `Option Ratio` an application of a declared family.
fn shape_type(cx: &Cx, shape: Shape) -> Result<Term, ElabError> {
    match shape {
        Shape::Base(leaf) => base_type(cx, leaf),
        Shape::Option(member) => applied(cx, "Option", [shape_type(cx, *member)?]),
        Shape::List(member) => applied(cx, "List", [shape_type(cx, *member)?]),
        Shape::Result(value, error) => applied(cx, "Result", [shape_type(cx, *value)?, shape_type(cx, *error)?]),
        Shape::Fault => crate::prelude::constant(cx, "RowFault"),
    }
}

/// The core type one inert domain denotes.
///
/// The two tagged rationals are applications and every other leaf is a constant,
/// which is [`bases`]'s kinds read back the other way round.
fn base_type(cx: &Cx, leaf: Leaf) -> Result<Term, ElabError> {
    Ok(match leaf {
        Leaf::Ratio => plain_type("Ratio"),
        Leaf::Text => plain_type("Text"),
        Leaf::Duration(which) => tagged_type("Duration", which),
        Leaf::Position(which) => tagged_type("Position", which),
        Leaf::Pitch => plain_type("Pitch"),
        Leaf::PitchClass => plain_type("PitchClass"),
        Leaf::Interval => plain_type("Interval"),
        Leaf::Key => plain_type("Key"),
        Leaf::Scale => plain_type("Scale"),
        Leaf::Degree => plain_type("Degree"),
        Leaf::Frame => plain_type("Frame"),
        Leaf::ChordClass => plain_type("ChordClass"),
        Leaf::Triad => plain_type("Triad"),
        Leaf::Roman => plain_type("Roman"),
        Leaf::Voicing => plain_type("Voicing"),
        Leaf::Pc12 => plain_type("Pc12"),
        Leaf::PcSet12 => plain_type("PcSet12"),
        Leaf::Row12 => plain_type("Row12"),
        // Declared rather than registered, so they are read out of the prelude's
        // context and not out of this registry. §1 decides both by name: a `Nat`
        // with no recursor would leave `nat_fold` with nothing to fold, and
        // `Bool` is the same argument one constructor shorter.
        Leaf::Bool => crate::prelude::constant(cx, "Bool")?,
        Leaf::Nat => crate::prelude::constant(cx, "Nat")?,
    })
}

/// The core type one phase-operation type denotes.
///
/// The phase table has no [`Shape`], because its three traversals take function
/// arguments and `Shape` has no arrow; its signatures are written in the old
/// checker's [`Type`] instead. This is the same reuse by a different door — the
/// 14 builders' types are read off `SyntaxOp`'s own scheme rather than declared a
/// second time — and it covers exactly the first-order fragment those 14 use.
///
/// # Errors
///
/// [`unnameable`] for a type outside that fragment, which is how a traversal
/// reaches here by mistake: it is a defect in the caller's filter rather than in
/// the signature.
fn phase_type(cx: &Cx, ty: &Type) -> Result<Term, ElabError> {
    Ok(match *ty {
        Type::Bool => crate::prelude::constant(cx, "Bool")?,
        Type::Nat => crate::prelude::constant(cx, "Nat")?,
        Type::Ratio => plain_type("Ratio"),
        Type::Text => plain_type("Text"),
        Type::Syntax(cat) => syntax_type(cat),
        Type::TokenKind => plain_type("TokenKind"),
        Type::Delimiter => plain_type("Delimiter"),
        Type::NodePath => plain_type("NodePath"),
        Type::BindingPath => plain_type("BindingPath"),
        Type::Option(ref member) => applied(cx, "Option", [phase_type(cx, member)?])?,
        Type::List(ref member) => applied(cx, "List", [phase_type(cx, member)?])?,
        Type::Sum(ref value, ref error) => applied(cx, "Result", [phase_type(cx, value)?, phase_type(cx, error)?])?,
        Type::Function(ref arguments, ref result) => {
            let mut built = phase_type(cx, result)?;
            for argument in arguments.iter().rev() {
                built = Term::pi(HERE, "argument", phase_type(cx, argument)?, built);
            }
            built
        }
        // Spelled out rather than wildcarded, because the list is the claim.
        // Every one of these is a type the *old* checker had and the core is
        // deliberately not given: a type variable and a unit belong to the
        // checker prompt 142 deleted, and `SyntaxStep` is the sealed-call tag
        // an adapter's recursion carries, not a type a signature names. The
        // musical domains and the machine forms are registered from the
        // `Shape` table and the machine registry, not from `Type`. A variant
        // added to `Type` should stop here and be decided, not fall through a
        // `_`.
        Type::Var(_) | Type::Unit | Type::SyntaxStep { .. } => {
            return Err(unnameable("a phase type with no core spelling"));
        }
    })
}

/// `head` applied to `arguments`.
fn applied(cx: &Cx, head: &str, arguments: impl IntoIterator<Item = Term>) -> Result<Term, ElabError> {
    let head = crate::prelude::constant(cx, head)?;
    Ok(arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(HERE, function, argument)))
}

/// The signature of a δ entry: its arguments, then its result, as one Π type.
fn delta_type(cx: &Cx, arguments: &[Shape], result: Shape) -> Result<Term, ElabError> {
    Ok(arrow(
        arguments
            .iter()
            .map(|argument| shape_type(cx, *argument))
            .collect::<Result<Vec<_>, _>>()?,
        shape_type(cx, result)?,
    ))
}

/// `α₁ → … → αₙ → ρ`, which is what a first-order signature is.
///
/// The binder is named and unreferenced for [`crate::lower`]'s reason: a Π always
/// binds, and nothing in an arrow refers to its argument.
fn arrow(arguments: Vec<Term>, result: Term) -> Term {
    arguments
        .into_iter()
        .rev()
        .fold(result, |built, argument| Term::pi(HERE, "argument", argument, built))
}

/// The four operations quotation needs and neither ownership table names.
///
/// ```text
/// instantiate_quote : NodePath → Template → List (List (Syntax ⟨expr⟩)) → Syntax ⟨expr⟩
/// match_quote       : Syntax ⟨token-tree⟩ → Template → Bool
/// quote_hole        : Syntax ⟨token-tree⟩ → Template → Nat → Syntax ⟨token-tree⟩
/// quote_holes       : Syntax ⟨token-tree⟩ → Template → Nat → List (Syntax ⟨token-tree⟩)
/// quote_hole_expr   : Syntax ⟨expr⟩ → Template → Nat → Syntax ⟨expr⟩
/// quote_holes_expr  : Syntax ⟨expr⟩ → Template → Nat → List (Syntax ⟨expr⟩)
/// ```
///
/// # Why these are not table rows
///
/// [`BUILTIN_OWNERSHIP`] and [`SYNTAX_OWNERSHIP`] are the *old* checker's name
/// lookup, so a row in either would make `instantiate_quote` a word an adapter
/// could write today — and what an adapter writes is `quote at here { … }`, whose
/// whole point is that the anchor is the only number it supplies. They are
/// counted instead by [`rules::BEYOND`], and the accounting law reads them back
/// off the registry.
///
/// # Why the pattern side reads at a literal index, in one pair per category
///
/// The signature §4 asks for is `(c : Cat) → Syntax c → …`, and the finite-data
/// check refuses it: a δ signature admits a base type at a *literal* index and
/// nothing else, because a `fn` rule can build a literal and cannot build a
/// constructor. What the literal-index rule *admits* is one reader per
/// category, which is §4's sentence said another way — "a pattern is read at
/// the scrutinee's category" — so the readers come in a pair and the lowering
/// picks from the scrutinee's written annotation (prompt 142's repair; the
/// lowering runs before types exist, and the annotation is the category the
/// author stated). `match_quote` needs no twin: a `Syntax ⟨expr⟩` scrutinee
/// reaches its `⟨tokentree⟩` parameter through §1's forgetting rule, which
/// prompt 142 supplied as [`FORGOTTEN`](rules::FORGOTTEN) below.
///
/// # Errors
///
/// [`ElabError`] when `Nat`, `Bool`, or `List` is not declared in `cx`, which is
/// a compiler defect.
fn quotation(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let template = plain_type("Template");
    let expression = syntax_type(crate::quote::Cat::Expr);
    let read = syntax_type(crate::quote::Cat::TokenTree);
    let index = crate::prelude::constant(cx, "Nat")?;
    let splices = applied(cx, "List", [applied(cx, "List", [expression.clone()])?])?;
    let delta = |name: &'static str, arguments: Vec<Term>, result: Term, rule| {
        Builtin::new(name, arrow(arguments, result), musa_calculus::Family::Delta, rule)
    };
    Ok(vec![
        delta(
            rules::BEYOND[0],
            vec![plain_type("NodePath"), template.clone(), splices],
            expression.clone(),
            rules::INSTANTIATE,
        ),
        delta(
            rules::BEYOND[1],
            vec![read.clone(), template.clone()],
            crate::prelude::constant(cx, "Bool")?,
            rules::MATCHES,
        ),
        delta(
            rules::BEYOND[2],
            vec![read.clone(), template.clone(), index.clone()],
            read.clone(),
            rules::HOLE,
        ),
        delta(
            rules::BEYOND[3],
            vec![read.clone(), template.clone(), index.clone()],
            applied(cx, "List", [read])?,
            rules::HOLES,
        ),
        delta(
            rules::BEYOND[4],
            vec![expression.clone(), template.clone(), index.clone()],
            expression.clone(),
            rules::HOLE_EXPR,
        ),
        delta(
            rules::BEYOND[5],
            vec![expression.clone(), template, index],
            applied(cx, "List", [expression])?,
            rules::HOLES_EXPR,
        ),
    ])
}

/// `forget_category : Syntax ⟨expr⟩ → Syntax ⟨token-tree⟩`.
///
/// What `11-quotation.md` §1's acceptance rule elaborates to. The section calls
/// it "one acceptance rule in the checker rather than a `forget` an author
/// writes", and both halves of that are here: the rule is
/// [`rules::forgets`], carried on the `Syntax` registration and asked by
/// `musa-calculus`'s one directional site, and this operation is what the *checker*
/// inserts — it is in neither ownership table, so no name resolves to it and no
/// adapter can write it.
///
/// It is a δ-builtin and not bare acceptance because an elaborated term has to
/// re-check in the core. Without it the term the elaborator produced would hold
/// a `Syntax ⟨expr⟩` where its own type says `Syntax ⟨token-tree⟩`, and
/// [`musa_calculus::well_typed`] would refuse a term this compiler had accepted.
///
/// One direction and one signature, because [`crate::quote::Cat`] has two
/// cases. A third category is a case in [`rules::forgets`] and a second
/// registration here.
fn syntax_carrier() -> Builtin {
    Builtin::new(
        rules::FORGOTTEN,
        arrow(
            vec![syntax_type(crate::quote::Cat::Expr)],
            syntax_type(crate::quote::Cat::TokenTree),
        ),
        musa_calculus::Family::Delta,
        rules::FORGET,
    )
}

/// Every compiler-owned δ operation, as a core builtin.
///
/// Which of `02-core-calculus.md` §1.5's index operators an operation spells.
///
/// Five rows, and the absences carry as much as the entries.
///
/// `nat_sub` is not here because it answers `Option<Nat>`: a subtraction that
/// can fall off the floor is not a `Nat`, so it cannot stand in an index
/// position at all, and tagging it would claim a reading the type never admits.
/// `ratio_div` is not here because §1.5's grammar has no division —
/// multiplication is *by a literal*, which is what keeps an index a linear form.
/// `duration_add` and the rest of the tagged arithmetic are not here because a
/// `Duration` carries a coordinate **parameter** and is a different type from
/// the `Ratio` an index is drawn from.
///
/// Only an *open* application is ever read through one of these. A δ-rule fires
/// as soon as its arguments are canonical, so `2 + 3` has already reduced to `5`
/// before any index is compared; what stays stuck is `p + q` under a variable,
/// which is §1.5's `follow(a: Bar(p), b: Bar(q)) -> Bar(p + q)`.
///
/// Written as three questions rather than as one match over every operation,
/// because untagged is the *default* and not the leftover case: an operation
/// added later is not an index operator until someone decides it is one, and a
/// match that had to name all hundred-odd would make that decision look like an
/// oversight whenever it went unmade.
fn indexes(operation: crate::phase::Builtin) -> Option<Operator> {
    use crate::phase::Builtin as Operation;

    if matches!(operation, Operation::NatAdd | Operation::RatioAdd) {
        return Some(Operator::Add);
    }
    if matches!(operation, Operation::RatioSub) {
        return Some(Operator::Subtract);
    }
    if matches!(operation, Operation::NatMul | Operation::RatioMul) {
        return Some(Operator::Multiply);
    }
    None
}

/// Two tables, kept two. `BUILTIN_OWNERSHIP` is the source language's and
/// `SYNTAX_OWNERSHIP` is the expansion phase's, and §5.9 makes that separation
/// load-bearing: nothing in the phase registry is looked up when ordinary source
/// reads a name. What they share is this one loop, because a δ-builtin is a
/// δ-builtin whichever table names it.
///
/// A row whose rule is missing is a registration defect and refuses the whole
/// registry, rather than being skipped: a table with a hole in it would type-check
/// a program the evaluator then got stuck on.
fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let mut built = Vec::with_capacity(rules::REGISTERED);
    for entry in &BUILTIN_OWNERSHIP {
        // The rationale is load-bearing, not decoration: a row that cannot say
        // what it hides has not argued its ownership.
        debug_assert!(
            !entry.hidden_information.is_empty(),
            "`{}` does not say what it hides",
            entry.spelling
        );
        let Family::Delta { arguments, result } = entry.family else {
            continue;
        };
        let rule = rules::source(entry.operation).ok_or_else(|| missing(entry.spelling))?;
        let declared = Builtin::new(
            entry.spelling,
            delta_type(cx, arguments, result)?,
            musa_calculus::Family::Delta,
            rule,
        );
        built.push(match indexes(entry.operation) {
            Some(operator) => declared.indexing(operator),
            None => declared,
        });
    }
    let mut minter = crate::infer::Minter::default();
    for entry in &SYNTAX_OWNERSHIP {
        debug_assert!(
            !entry.hidden_information.is_empty(),
            "`{}` does not say what it hides",
            entry.spelling
        );
        let Some(rule) = rules::phase(entry.operation) else {
            continue;
        };
        built.push(Builtin::new(
            entry.spelling,
            phase_type(cx, &entry.operation.instantiate(&mut minter))?,
            musa_calculus::Family::Delta,
            rule,
        ));
    }
    built.extend(traversal::eliminators(cx)?);
    built.push(syntax_carrier());
    built.extend(quotation(cx)?);
    built.extend(track::builtins(cx)?);
    built.extend(notation::builtins(cx)?);
    built.extend(machine::builtins(cx)?);
    built.extend(machine::primitives(cx)?);
    Ok(built)
}

/// `run_syntax_step`, as a definition rather than a registration.
///
/// ```text
/// run_syntax_step
///   : (Context : Type 0) → (Answer : Type 0)
///   → Context → SyntaxStep Context Answer → Answer
/// run_syntax_step = fn (C, A, context, step) {
///     match step { SyntaxStep.Step(run) => run(context) }
/// }
/// ```
///
/// Seventeen rows in, sixteen registrations and this out. `run_syntax_step`
/// leaves the phase registry because a projection is not a compiler-owned
/// operation: it hides nothing, which is the test every `SYNTAX_OWNERSHIP` row
/// already states for itself. [`Registry::new`] would have refused it in any
/// case — its target is a *declared* family, and a rewrite over a declared family
/// is the second ι-rule that check exists to catch. That the ownership test and
/// the core's own check agree is evidence the design is right rather than a
/// coincidence to route around.
///
/// The step's `run` field is private to the phase module, so the `match` here
/// resolves only because this context stands in no module and is therefore inside
/// every one. A transformer's own module is not, which is the seal.
///
/// # Errors
///
/// [`ElabError`] when `SyntaxStep` is not declared in `cx`, or when the
/// definition does not check at its own type — a compiler defect either way.
#[cfg(test)]
pub(crate) fn run_syntax_step(cx: &Cx) -> Result<Definition, ElabError> {
    let sealed = crate::prelude::constant(cx, "SyntaxStep")?;
    let step = Term::app(
        HERE,
        Term::app(HERE, sealed, Term::var(HERE, Index(2))),
        Term::var(HERE, Index(1)),
    );
    let ty = Term::pi(
        HERE,
        "Context",
        type0(),
        Term::pi(
            HERE,
            "Answer",
            type0(),
            Term::pi(
                HERE,
                "context",
                Term::var(HERE, Index(1)),
                Term::pi(HERE, "step", step, Term::var(HERE, Index(2))),
            ),
        ),
    );
    let run = RawArm {
        patterns: vec![RawPattern::constructor(
            HERE,
            "SyntaxStep.Step",
            [RawPattern::bind(HERE, "run")],
        )],
        body: Raw::app(HERE, Raw::var(HERE, "run"), Raw::var(HERE, "context")),
    };
    let body = Raw::match_on(HERE, [Raw::var(HERE, "step")], vec![run]);
    let written = ["Context", "Answer", "context", "step"]
        .into_iter()
        .rev()
        .fold(body, |built, name| Raw::lam(HERE, name, built));
    let value = musa_calculus::check(cx, &ty, &written)?;
    Ok(Definition { ty, value })
}

/// A compiler-owned definition: what it is, and what it means.
///
/// Both halves, because a definition is both — [`Cx::define`] takes a type and a
/// value, and a λ has no inferable type, so handing back the value alone would
/// leave every caller to reconstruct the type this function already wrote.
#[cfg(test)]
pub(crate) struct Definition {
    /// Its type.
    pub(crate) ty: Term,
    /// Its meaning, checked at that type.
    pub(crate) value: Term,
}

/// A row of a compiler-owned table with no rule behind it.
fn missing(spelling: &str) -> ElabError {
    unnameable(spelling)
}

/// Something this module was asked for and has no core spelling of.
///
/// A compiler defect either way, and [`Refusal::UnknownName`] rather than a
/// [`musa_calculus::Malformed`] because what went wrong is a *name* with nothing
/// behind it — the same sentence a program earns for a name that is not in
/// scope, which is exactly the shape of this failure one level up.
fn unnameable(what: &str) -> ElabError {
    Refusal::UnknownName {
        name: Arc::from(what),
        at: HERE,
        candidates: Vec::new(),
    }
    .into()
}
