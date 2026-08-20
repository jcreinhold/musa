use indexmap::IndexSet;
use musa_core::Raw;
use musa_language::ast::AstNode as _;
use musa_language::{SyntaxKind, SyntaxNode};

use crate::diagnose::{Code, Diagnostic};
use crate::infer::Minter;
use crate::origin::SourceSpan;
use crate::resolve::Resolver;

/// Which clock a duration or a position is measured against
/// (`docs/rules/language/02-core-calculus.md` §1's `C`).
///
/// A closed pair, and deliberately not a kind: `C` ranges over exactly these
/// two, so a coordinate-polymorphic builtin would be machinery for a
/// two-element domain. Theorem 5 applies to a finite family of inert leaves
/// without it, and promoting the index to a kind later is additive because the
/// tags are already written down.
///
/// The point of carrying it is that `Duration<WrittenTime>` and
/// `Duration<PhysicalTime>` do not unify, so adding a written beat to a number
/// of seconds is not a mistake this language can express.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Coordinate {
    /// Positions and durations as the page counts them.
    WrittenTime,
    /// Positions and durations as a clock counts them, after a time map has
    /// been applied. Nothing in the source language constructs one yet; the
    /// tag exists so that the day one arrives it cannot be quietly mixed with
    /// written time.
    PhysicalTime,
}

impl Coordinate {
    /// The word this coordinate is written with, in a type and in a
    /// diagnostic.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::WrittenTime => "WrittenTime",
            Self::PhysicalTime => "PhysicalTime",
        }
    }
}

/// A coordinate prints as the word a type is written with.
///
/// It is the index of `Duration` and `Position` in [`crate::registry`], so this
/// is what a diagnostic shows when one of those types is printed.
impl std::fmt::Display for Coordinate {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.spelling())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    /// A type inference has not decided yet, named by the
    /// [`crate::infer::Minter`] that made it. It exists only while one
    /// declaration is being checked: every type that leaves the checker has
    /// been resolved, and a variable that survives that is a diagnostic.
    Var(crate::infer::TypeVar),
    Unit,
    Bool,
    Nat,
    Ratio,
    /// Opaque printable text. Storable data, and not a way in: nothing
    /// reads structure out of it, so it cannot carry what a type would
    /// otherwise have to say.
    Text,
    /// The binary sum `τ + τ`, written `Result<T, E>`.
    ///
    /// There is no `Type::Result`, and that is the point of the spelling:
    /// `Result` is what a sum is *used for* here, not a second kind of
    /// thing the compiler privileges
    /// (`docs/rules/language/02-core-calculus.md` §1).
    Sum(Box<Self>, Box<Self>),
    Option(Box<Self>),
    List(Box<Self>),
    /// A finite syntax value ([`crate::syntax::Syntax`]), by how it parses.
    ///
    /// Phase-local: `../rules/language/02-core-calculus.md` §5 closes the
    /// source type grammar and says the source language has no syntax value,
    /// and this does not widen it. The spelling `Syntax<Expr>` is read only
    /// where a transformer is checked, and the operations over it are offered
    /// only there.
    ///
    /// The index is a claim about how the tree parses, not about how it is
    /// built: the representation is unchanged, and `Syntax` is that
    /// representation with nothing claimed about it
    /// (`../rules/language/11-quotation.md` §1). Forgetting the claim is
    /// [`Checker::reconcile`]'s acceptance rule and not an operation, and
    /// establishing one is `as_expression`'s checked parse.
    Syntax(crate::syntax::Cat),
    /// How the lexer classified one token ([`musa_language::SyntaxKind`]).
    ///
    /// Phase-local, and *generated*: its values are the lexer's own kinds, so
    /// there is no second table for a new token kind to be missing from. It
    /// replaces the `Text` that `syntax_token` took and every
    /// `text_equal(kind, "…")` an adapter wrote against it.
    TokenKind,
    /// How one group is delimited ([`crate::syntax::Delimiter`]).
    ///
    /// Phase-local, four values, and what `syntax_group` used to claim to hide
    /// as "the fixed grouper's delimiter set".
    Delimiter,
    /// Where one node sits ([`crate::syntax::NodePath`]).
    NodePath,
    /// Which name a binder declares ([`crate::syntax::BindingPath`]).
    BindingPath,
    /// `SyntaxStep<C, A>` — one suspended recursive call, sealed to the child
    /// it descends to and the algebra that exposed it.
    ///
    /// Phase-local like [`Type::Syntax`], and the one phase type with
    /// arguments, so `C` and `A` unify the way any other member does. It has
    /// no source constructor: a step is minted only by the recursor's group
    /// case and consumed only by `run_syntax_step`, which is what makes
    /// `docs/rules/language/02-core-calculus.md` §5.9's association lemma a
    /// fact about the value rather than a check someone has to run.
    ///
    /// **Never storable data**, at any depth, for a stronger reason than an
    /// arrow's: what it hides *is* an algebra of source closures. The
    /// exclusion is enforced structurally in [`crate::infer::Unifier`], beside
    /// the arrow's.
    SyntaxStep {
        context: Box<Self>,
        answer: Box<Self>,
    },
    Function(Vec<Self>, Box<Self>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // A variable prints as a lowercase name, the way a signature
            // would write it if the file had written one: `a`, `b`, … and
            // `a26` onwards once the letters run out.
            Self::Var(variable) => match u8::try_from(*variable) {
                Ok(index) if index < 26 => write!(out, "{}", char::from(b'a'.saturating_add(index))),
                _ => write!(out, "a{variable}"),
            },
            Self::Unit => out.write_str("Unit"),
            Self::Bool => out.write_str("Bool"),
            Self::Nat => out.write_str("Nat"),
            Self::Ratio => out.write_str("Ratio"),
            Self::Text => out.write_str("Text"),
            Self::Sum(value, error) => write!(out, "Result<{value}, {error}>"),
            Self::Option(member) => write!(out, "Option<{member}>"),
            Self::List(member) => write!(out, "List<{member}>"),
            // These print in a transformer's diagnostics in the spelling an
            // adapter writes them, and nowhere else: ordinary source may write
            // none of them, because they are read only where `in_phase` holds.
            Self::Syntax(category) => write!(out, "Syntax<{}>", category.name()),
            Self::NodePath => out.write_str("NodePath"),
            Self::BindingPath => out.write_str("BindingPath"),
            Self::TokenKind => out.write_str("TokenKind"),
            Self::Delimiter => out.write_str("Delimiter"),
            Self::SyntaxStep { context, answer } => write!(out, "SyntaxStep<{context}, {answer}>"),
            Self::Function(parameters, result) => {
                if parameters.len() == 1 {
                    let parameter = parameters.first().unwrap_or(&Self::Unit);
                    if matches!(parameter, Self::Function(_, _)) {
                        write!(out, "({parameter}) -> {result}")
                    } else {
                        write!(out, "{parameter} -> {result}")
                    }
                } else {
                    out.write_str("(")?;
                    for (index, parameter) in parameters.iter().enumerate() {
                        if index > 0 {
                            out.write_str(", ")?;
                        }
                        write!(out, "{parameter}")?;
                    }
                    write!(out, ") -> {result}")
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Builtin {
    NatFold,
    ListFoldFromStart,
    ListFoldFromEnd,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
    RatioAdd,
    RatioSub,
    RatioMul,
    RatioDiv,
    RatioLess,
    RatioEqual,
    TextEqual,
    /// `text_join(pieces)` — the pieces of a text, run together into one.
    ///
    /// The only operation that *builds* a text, and the whole reason a printer
    /// can be written at all (`26-language-design-decision.md` §4). A join of a
    /// list rather than a binary concatenation because a printer assembles a
    /// sequence and wants one answer: folding a pair-wise concatenation over
    /// `n` pieces allocates `n` intermediate texts and charges the §4 byte
    /// meter `O(n²)` for a result of size `O(n)`.
    TextJoin,
    /// `nat_literal(count)` — the source literal that names a whole number.
    NatLiteral,
    /// `ratio_literal(value)` — the source literal that names an exact
    /// rational, or nothing when it has none.
    ///
    /// Nothing below zero. The grammar has no negative numeric literal, and a
    /// spelling the reader would not read back is not a literal — which is the
    /// one law this family has.
    RatioLiteral,
    /// `pitch_literal(pitch)` — the source literal that names a written pitch.
    PitchLiteral,
    /// `key_literal(key)` — the source literal that names a key.
    KeyLiteral,
    /// `interval_literal(interval)` — the source literal that names a written
    /// interval, or nothing when it has none.
    ///
    /// Written interval names run out: a size and quality outside the named
    /// grid has no literal, and D2 puts that in the result type rather than in
    /// a fabricated pair. It is also the one honest `PrintLoss` a printer of
    /// notation has.
    IntervalLiteral,
    NatAdd,
    NatMul,
    NatSub,
    DurationOf,
    DurationRatio,
    DurationAdd,
    DurationScale,
    DurationLess,
    DurationEqual,
    PositionOf,
    PositionRatio,
    PositionShift,
    PositionBetween,
    PositionLess,
    PositionEqual,
    IntervalAdd,
    IntervalInverse,
    IntervalEqual,
    PitchTransposed,
    PitchEqual,
    PitchClassTransposed,
    PitchClassOf,
    SignatureScale,
    ScaleOn,
    ScaleTonic,
    ScaleSize,
    ScalePitch,
    ScaleClass,
    ScaleChord,
    PitchFrame,
    FrameScale,
    FrameTonic,
    FramePitch,
    DegreeOf,
    DegreeStepUp,
    DegreeStepDown,
    DegreeRaised,
    DegreeLowered,
    ChordOn,
    ChordRoot,
    ChordBass,
    ChordMembers,
    ChordInversion,
    ChordOver,
    ChordTriad,
    TriadChord,
    TriadMajor,
    RomanOf,
    RomanOrdinal,
    RomanSize,
    RomanInversion,
    VoicingOf,
    VoicingPitches,
    VoicingBass,
    VoicingChord,
    VoicingPosition,
    CloseVoicing,
    DropVoicing,
    OmitVoicing,
    Pc12Of,
    Pc12Number,
    Pc12Forget,
    Pc12Transposed,
    Pc12Inverted,
    Pc12Spelled,
    PcSet12Of,
    PcSet12Members,
    PcSet12Transposed,
    PcSet12Inverted,
    PcSet12Normal,
    PcSet12Prime,
    PcSet12Vector,
    Row12Of,
    Row12Pcs,
    Row12Head,
    Row12Transposed,
    Row12Inverted,
    Row12Retrograde,
    Row12Matrix,
    Row12Forms,
    Row12Symmetries,
    Row12Repeats,
    Row12Missing,
    Transpose,
    Stretch,
    Retrograde,
    Invert,
    Shift,
    Together,
    MapNotePitches,
    Play,
    Primitive,
    Machine,
    Identity,
    Connect,
    Beside,
    Feedback,
    Copy,
    Drop,
    Swap,
}

/// A base type as a builtin signature names it.
///
/// These are the inert types of `docs/rules/language/02-core-calculus.md` §5.8: a closed value of one is
/// an opaque constant, no reduction rule inspects its structure, and everything observable about it
/// is observed by applying a builtin. That is condition D1, and it holds here by construction —
/// there is no variant for a type with an eliminator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Base {
    Bool,
    Nat,
    /// An exact rational. Signed, and the ordinary arithmetic base: §1's
    /// commentary says the refinements live at the constructors of the tagged
    /// types rather than in a second numeric type.
    Ratio,
    /// Opaque printable text. Inert in D1's sense — nothing reads structure
    /// out of it — which is what lets it be the error half of a `Result`
    /// without giving a builtin a second way to say what went wrong.
    Text,
    Duration(Coordinate),
    Position(Coordinate),
    Pitch,
    PitchClass,
    Interval,
    Key,
    Scale,
    Degree,
    Frame,
    ChordClass,
    Triad,
    Roman,
    Voicing,
    Pc12,
    PcSet12,
    Row12,
}

/// An argument or result type of a δ-builtin.
///
/// There is deliberately no arrow constructor. §5.8's no-arrow premise is therefore true of every
/// declared δ signature by construction rather than by inspection, and a builtin that wanted a
/// function argument could not be spelled here at all — it would have to join the eliminators,
/// which is exactly the classification the theorem depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shape {
    Base(Base),
    Option(&'static Self),
    List(&'static Self),
    /// `Result<value, error>` — how a builtin with more than one way to
    /// fail says which one happened. `Option` says only *that* it did.
    Result(&'static Self, &'static Self),
    /// Why a sequence of pitch classes is not a twelve-tone row.
    ///
    /// The one signature in the table that is neither a base type nor a
    /// container over one, and the reason it is spelled as a name rather than as
    /// an anonymous pair: a δ-rule answers a `musa_core::Datum`, which is a
    /// literal or a constructor, so a bare product is the one thing it cannot
    /// write. That is the mechanism noticing something true — the pair was a
    /// domain concept wearing a tuple — so it is declared in
    /// [`crate::prelude`] and named here.
    ///
    /// It is still a product to the old checker, because [`Self::ty`] still has
    /// to answer one; that half leaves with the old checker in prompt 142.
    Fault,
}

impl Shape {
    /// Whether this shape denotes storable data (`02-core-calculus.md` §1.1).
    ///
    /// A registered builtin's arguments and result must be storable: it is
    /// handed values that could equally have been written in a data field, and
    /// it hands one back. That is what makes it *foreign* rather than a second
    /// evaluator — it cannot receive a closure to call, a music value to walk,
    /// or anything else whose meaning depends on the elaboration around it.
    ///
    /// Today this is true of every shape that can be spelled, because [`Base`]
    /// names only storable domains and [`Shape`] has no arrow. It is written
    /// out rather than assumed so that the day a base is added the question is
    /// asked at the registration and answered in the build, not discovered at
    /// a call.
    const fn is_storable(self) -> bool {
        match self {
            Self::Base(base) => base.is_storable(),
            Self::Option(member) | Self::List(member) => member.is_storable(),
            Self::Fault => NATS.is_storable() && PC12S.is_storable(),
            Self::Result(value, error) => value.is_storable() && error.is_storable(),
        }
    }
}

impl Base {
    /// Every base a δ signature can name is storable data.
    ///
    /// Exhaustive rather than `true`, so that a base for a non-storable domain
    /// — music, a running signal, anything holding a function — has to answer
    /// this question before it can be registered.
    const fn is_storable(self) -> bool {
        match self {
            Self::Bool
            | Self::Nat
            | Self::Ratio
            | Self::Text
            | Self::Duration(_)
            | Self::Position(_)
            | Self::Pitch
            | Self::PitchClass
            | Self::Interval
            | Self::Key
            | Self::Scale
            | Self::Degree
            | Self::Frame
            | Self::ChordClass
            | Self::Triad
            | Self::Roman
            | Self::Voicing
            | Self::Pc12
            | Self::PcSet12
            | Self::Row12 => true,
        }
    }
}

/// Whether every shape in a signature position is storable data.
///
/// Written as a recursion over the slice rather than a loop over indices
/// because it runs in a `const` context, where the registration is checked.
const fn all_storable(shapes: &[Shape]) -> bool {
    match shapes {
        [] => true,
        [first, rest @ ..] => first.is_storable() && all_storable(rest),
    }
}

/// Which of `02-core-calculus.md` §5.8's families a builtin belongs to.
///
/// The families are disjoint and exhaustive, which is what lets Theorem 5 be stated once instead of
/// once per musical domain. A new domain is admissible when its operations can be declared here as
/// `Delta` and discharge D1–D4; it does not get a new induction.
///
/// All four of §5.8's families are spelled here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Family {
    /// §5.8's **δ-builtins**: first-order and arrow-free. Covered by Theorem 5 once D1–D4 hold,
    /// and the declared signature is the single statement of the operation's type: the checker
    /// reads argument and result types from it rather than restating them.
    Delta { arguments: &'static [Shape], result: Shape },
    /// §5.8's **structural eliminators**: takes a function argument or carries a rank-1 scheme.
    /// Covered by §5.6, and checked by hand because its type depends on its arguments' types.
    Eliminator(Eliminator),
    /// §5.8's **track builtins**: constructs or transforms an event track. Covered by §5.7.
    Track,
    /// §5.8's **machine builtins**: the nine forms that build a finite machine description
    /// (`../across-stages/03-machine-calculus.md` §2). Each carries a rank-1 scheme, the way the
    /// eliminators do, except `primitive`, whose type the build-local registry supplies.
    Machine(MachineOp),
}

/// The nine machine builtins of `../across-stages/03-machine-calculus.md` §2.
///
/// A closed set, for the same reason [`Eliminator`] is one: §2's admissible forms are exactly
/// these, and a tenth would be a change to the calculus rather than an addition to a library.
/// Seven of them are pure wiring and say nothing about what is being wired; `primitive` names a
/// registered unit, and `machine` is how one becomes a machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MachineOp {
    /// `primitive(name, version, configuration)` — one instance of a registered unit.
    Primitive,
    /// `machine(p)` — §2's lifting of a registered unit into a machine.
    Machine,
    Identity,
    Connect,
    Beside,
    Feedback,
    Copy,
    Drop,
    Swap,
}

/// The eight structural eliminators of `02-core-calculus.md` §5.6.
///
/// They are named as a closed set rather than matched out of [`Builtin`] because §5.6's proof is
/// about exactly these eight. Naming them here is what lets the checker's remaining hand-written
/// arms be exhaustive: once a builtin's family is an `Eliminator`, which one it is has already
/// been decided, and no arm is left over for the sixty-two δ-builtins to fall into by accident.
///
/// `list` has two because both directions have demonstrated consumers: its outermost cons holds
/// the *first* element, so folding from the outside in and accumulating from the start can
/// disagree. The projection step `s(x,a) = x` is one witness. The two folds share one type, which
/// is exactly why the direction has to be in the name. Other structures may admit ordered
/// traversals too; they keep one canonical eliminator until another primitive earns a caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Eliminator {
    NatFold,
    ListFoldFromStart,
    ListFoldFromEnd,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
}

/// The fourteen phase-local syntax operations.
///
/// A closed set, like [`Eliminator`] and [`MachineOp`], and deliberately *not* a
/// [`Family`]: §5.8's four families classify the builtins ordinary source can
/// name, and none of these is one of those. They are reachable only where
/// [`Checker::expansion`] is set, which is the whole of what "phase-local"
/// means here — one core, one evaluator, and an environment that offers more
/// names in one place.
///
/// Every path a transformer holds was *derived*: [`Self::Recurse`] hands each
/// input node its own structural path, and [`Self::Built`] and
/// [`Self::Binding`] derive a new one from a path already held. Nothing here
/// takes a number and returns a path, and nothing mints a fresh id, which is
/// the repair `37-final-blocker.md` §1 and `34-proof-review.md` asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyntaxOp {
    /// `recurse_syntax(missing, token, identifier, group, context, subject)` —
    /// the way into a syntax value, with each branch receiving the inherited
    /// context and the node's own path.
    ///
    /// The group branch is handed a `List<SyntaxStep<C,A>>` rather than a
    /// `List<A>`: it decides whether, in what order, and under what context
    /// each child is read. Prompt 127da's "the fold is the only way in" is
    /// superseded here — `Syntax` is still opaque, paths are still derived,
    /// and what changed is only that an adapter may look at a node before
    /// reading its children (`../rules/language/02-core-calculus.md` §5.9).
    Recurse,
    /// `run_syntax_step(context, next)` — resume one sealed step.
    ///
    /// A builtin and not callable syntax, for two reasons that point the same
    /// way. A step spelled `next(c)` would *be* a function type, so any `C ->
    /// A` would unify with it and sealing would stop being a fact about the
    /// type; and an arrow-shaped step could not be excluded from `d` for the
    /// reason it must be.
    Run,
    /// `syntax_fold_from_leaves(missing, token, identifier, group, subject)` —
    /// the derived bottom-up fold: every child is read, in source order,
    /// before its group's branch runs.
    ///
    /// Named for which end it runs from, because that is the behaviour a
    /// caller has to plan around and it cannot be in the type — the same
    /// argument prompt 127dcfaa made for the two list folds. It is
    /// [`Self::Recurse`] at a context nothing reads, and the traversal below
    /// is literally the same function in its other mode.
    Fold,
    /// `syntax_at(subject, path)` — the input node at `path`, if there is one.
    /// How a transformer preserves input with its source information intact.
    At,
    /// `syntax_anchor(subject, path)` — the anchor of the input node at
    /// `path`, as a node built at the place the node itself determines.
    ///
    /// An anchor is how a value remembers where it came from: the number this
    /// builds into the emitted expression is an index into the expansion
    /// record's table of the region's own ranges, so a package function
    /// complaining about the fourth connection three passes later is still
    /// complaining about the fourth connection. It answers with a *node* and
    /// not with the number because a transformer may emit a place and may not
    /// read one (`26-language-design-decision.md` §3.4) — this hands back
    /// something to splice, and nothing to compare. The answer stands at
    /// [`crate::syntax::anchor_place`] of the anchored node: the place derives
    /// from the arguments alone, because a δ rule is a function of its
    /// arguments and nothing else (§5.8's D3), and the reservation it uses is
    /// [`crate::syntax::DELTA_QUOTATION`]'s.
    Anchor,
    /// `syntax_number(node)` — the exact rational a numeric token spells.
    ///
    /// The reader has already read it: the lexer keeps `3/8` whole as one
    /// `Rational` token and `4` as one `Integer`, so this hands back a reading
    /// the compiler performed rather than making a transformer re-derive one
    /// from text. Nothing is revealed that the fold did not already reveal —
    /// the transformer can see the same token's spelling — and what is saved is
    /// a conversion the phase has no operation for and no finite table could
    /// stand in for, since a written span is an arbitrary rational.
    Number,
    /// `syntax_built(path, role, child)` — an output path derived from `path`.
    Built,
    /// `syntax_binding(path, role)` — the binding `path` declares at `role`.
    Binding,
    /// `syntax_token(path, kind, text)`.
    Token,
    /// `syntax_identifier(path, name)` — a name the composer's own source binds.
    Identifier,
    /// `syntax_group(path, delimiter, children)`.
    Group,
    /// `syntax_binder(binding, name)` — the declaration of a name the expansion
    /// introduces.
    Binder,
    /// `syntax_reference(path, binding, name)` — a use of one.
    Reference,
    /// `token_kind_equal(one, other)` — whether two token kinds are one kind.
    KindEqual,
    /// `delimiter_equal(one, other)` — whether two delimiters are one.
    DelimiterEqual,
    /// `as_expression(subject)` — the checked parse
    /// (`../rules/language/11-quotation.md` §1).
    ///
    /// The one operation that establishes the finer claim, and the reason the
    /// index is worth having: an adapter that lifts a node out of the
    /// composer's own region holds a tree nobody parsed and has to put it where
    /// an expression stands. Answering here is what puts the diagnostic on the
    /// composer's line rather than on the region, after a malformed tree has
    /// already reached the gate.
    AsExpression,
    /// `checked_expression(subject)` — the gate, answering with the value or
    /// with what is wrong with it.
    Checked,
}

/// What kind of phase-local operation a [`SyntaxOp`] is.
///
/// Two cases, not four: this registry is small on purpose, and the split that
/// matters is between the operations that eliminate a syntax value and the
/// total first-order builders around it. "Descent into syntax happens in
/// exactly one place" is then a fact the registry states rather than a claim a
/// reader has to count out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PhaseFamily {
    /// Descent: [`SyntaxOp::Recurse`], [`SyntaxOp::Run`], and the derived
    /// [`SyntaxOp::Fold`]. Each takes function arguments and carries a rank-1
    /// scheme, so §5.6's account of an eliminator applies to them unchanged,
    /// and all three run the *same* traversal — [`recurse_syntax`] — so there
    /// is one descent in the implementation and not three.
    Fold,
    /// A total first-order operation over syntax values and paths. Each is a
    /// function of its displayed arguments and nothing else — no counter, no
    /// clock, no compiler state — which is what makes two runs agree exactly.
    Builder,
}

impl SyntaxOp {
    /// This operation's type, as a rank-1 scheme, instantiated fresh.
    ///
    /// Only the fold has a variable in it — what the transformer is folding
    /// *to* — and that variable is **ordinary**: a fold may build a list of
    /// functions as readily as a list of syntax. Everything else is
    /// monomorphic, because a builder's argument and result types are decided
    /// by which builder it is.
    pub(crate) fn instantiate(self, minter: &mut Minter) -> Type {
        // Every operation here reads and builds at `TokenTree`: reading claims
        // nothing about a node it descends into, and a builder's result is a
        // tree nobody has parsed. `Expr` is reached only by `as_expression` and
        // by the gate, which are the two operations that run the real parser
        // (`../rules/language/11-quotation.md` §1).
        let syntax = || Type::Syntax(crate::syntax::Cat::TokenTree);
        let expression = || Type::Syntax(crate::syntax::Cat::Expr);
        let path = || Type::NodePath;
        match self {
            Self::Fold => {
                let to = minter.fresh();
                let step = |arguments: Vec<Type>| Type::Function(arguments, Box::new(to.clone()));
                Type::Function(
                    vec![
                        step(vec![path()]),
                        step(vec![path(), Type::TokenKind, Type::Text]),
                        step(vec![path(), Type::Text]),
                        step(vec![path(), Type::Delimiter, Type::List(Box::new(to.clone()))]),
                        syntax(),
                    ],
                    Box::new(to),
                )
            }
            // Two variables, both **ordinary**: an adapter may inherit a
            // function and answer with one, and program five of prompt
            // 127dcfae's trial does both at once. Rank stays 1 — they are
            // quantified here, at the outside of this one scheme, and
            // `SyntaxStep<C, A>` is a type constructor over them rather than
            // a quantifier of its own.
            Self::Recurse => {
                let context = minter.fresh();
                let to = minter.fresh();
                let branch = |mut arguments: Vec<Type>| {
                    arguments.insert(0, context.clone());
                    Type::Function(arguments, Box::new(to.clone()))
                };
                let sealed = Type::SyntaxStep {
                    context: Box::new(context.clone()),
                    answer: Box::new(to.clone()),
                };
                Type::Function(
                    vec![
                        branch(vec![path()]),
                        branch(vec![path(), Type::TokenKind, Type::Text]),
                        branch(vec![path(), Type::Text]),
                        branch(vec![path(), Type::Delimiter, Type::List(Box::new(sealed))]),
                        context,
                        syntax(),
                    ],
                    Box::new(to),
                )
            }
            // The context comes first for the reason the folding use makes
            // plain: `fn (kid, later) { run_syntax_step(later, kid) }` puts
            // the context where the accumulator is.
            Self::Run => {
                let context = minter.fresh();
                let to = minter.fresh();
                let sealed = Type::SyntaxStep {
                    context: Box::new(context.clone()),
                    answer: Box::new(to.clone()),
                };
                Type::Function(vec![context, sealed], Box::new(to))
            }
            // The answer is optional for one reason both operations share: an
            // adapter points at a node it holds, and a path it derived
            // addresses no input node at all. The anchor's second argument is
            // the node it is *about*; the place its answer stands at is no
            // argument at all, because a δ rule is a function of its
            // arguments (§5.8's D3) and the place derives from them —
            // `crate::syntax::anchor_place` is the derivation.
            Self::At | Self::Anchor => {
                Type::Function(vec![syntax(), path()], Box::new(Type::Option(Box::new(syntax()))))
            }
            // `Option` because a node that is not a numeric token is not a
            // number, and `Ratio` because one operation covering both numeric
            // kinds is one operation an adapter has to learn.
            Self::Number => Type::Function(vec![syntax()], Box::new(Type::Option(Box::new(Type::Ratio)))),
            Self::Built => Type::Function(vec![path(), Type::Nat, Type::Nat], Box::new(Type::NodePath)),
            Self::Binding => Type::Function(vec![path(), Type::Nat], Box::new(Type::BindingPath)),
            Self::Token => Type::Function(vec![path(), Type::TokenKind, Type::Text], Box::new(syntax())),
            Self::Identifier => Type::Function(vec![path(), Type::Text], Box::new(syntax())),
            Self::Group => Type::Function(
                vec![path(), Type::Delimiter, Type::List(Box::new(syntax()))],
                Box::new(syntax()),
            ),
            Self::Binder => Type::Function(vec![Type::BindingPath, Type::Text], Box::new(syntax())),
            Self::Reference => Type::Function(vec![path(), Type::BindingPath, Type::Text], Box::new(syntax())),
            // Two comparisons, and only two: the phase's own types have no
            // `match`, and equality against a named constant is the whole of
            // what an adapter asks of a kind or a delimiter. They are members
            // of the `text_equal`/`nat_equal` family, and prompt 143 collapses
            // that family behind `Eq` with these inside it.
            Self::KindEqual => Type::Function(vec![Type::TokenKind, Type::TokenKind], Box::new(Type::Bool)),
            Self::DelimiterEqual => Type::Function(vec![Type::Delimiter, Type::Delimiter], Box::new(Type::Bool)),
            // The checked parse, and the only introduction form for `Expr`
            // that does not go through a quote. `Option` rather than `Result`
            // because there is one way to fail — this tree is not an
            // expression — and the parser's own diagnostic belongs on the
            // composer's line, not in an adapter's error value.
            Self::AsExpression => Type::Function(vec![syntax()], Box::new(Type::Option(Box::new(expression())))),
            // The gate says which of several things is wrong, so it answers
            // with a `Result` rather than an `Option`, exactly as a δ-builtin
            // with more than one way to fail does.
            Self::Checked => Type::Function(
                vec![syntax()],
                Box::new(Type::Sum(Box::new(expression()), Box::new(Type::Text))),
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BuiltinOwnership<T, F = Family> {
    pub(crate) operation: T,
    pub(crate) spelling: &'static str,
    pub(crate) hidden_information: &'static str,
    pub(crate) family: F,
}

/// The phase-local registry.
///
/// Separate from [`BUILTIN_OWNERSHIP`] so that §5.8's four families stay the
/// four families of the source core: nothing here is a δ-builtin, an
/// eliminator, a track builtin, or a machine builtin, and nothing here is
/// looked up when ordinary source reads a name. Each entry says what it hides,
/// for the same reason the source entries do — an operation earns a place in a
/// compiler-owned registry by hiding something a library could not.
pub(crate) const SYNTAX_OWNERSHIP: [BuiltinOwnership<SyntaxOp, PhaseFamily>; 17] = [
    BuiltinOwnership {
        operation: SyntaxOp::Recurse,
        spelling: "recurse_syntax",
        hidden_information: "the reader's node representation, each node's structural path, and the suspended entry \
                             into a proper child",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Run,
        spelling: "run_syntax_step",
        hidden_information: "which child and which algebra a step was minted for",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Fold,
        spelling: "syntax_fold_from_leaves",
        hidden_information: "the reader's node representation and each node's structural path",
        family: PhaseFamily::Fold,
    },
    BuiltinOwnership {
        operation: SyntaxOp::At,
        spelling: "syntax_at",
        hidden_information: "descent into the reader's node representation, and a node's untouched source information",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Anchor,
        spelling: "syntax_anchor",
        hidden_information: "a node's position in the region's own reading order, which is the only name the compiler \
                             and a later package function both have for it",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Number,
        spelling: "syntax_number",
        hidden_information: "the reader's own numeric reading of a literal token, which a transformer has no operation \
                             to derive from that token's text",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Built,
        spelling: "syntax_built",
        hidden_information: "path derivation, which keeps output paths disjoint from input paths by construction",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Binding,
        spelling: "syntax_binding",
        hidden_information: "name identity as a derived coordinate rather than an allocated fresh id",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Token,
        spelling: "syntax_token",
        hidden_information: "generated source information, which an adapter can carry but not forge",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Identifier,
        spelling: "syntax_identifier",
        hidden_information: "generated source information, and the absence of a hygiene scope",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Group,
        spelling: "syntax_group",
        // It used to claim the delimiter set as well. `Delimiter` hides that
        // now, and a builder that hides only its source information is what
        // prompt 143 collapses.
        hidden_information: "generated source information, which an adapter can carry but not forge",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Binder,
        spelling: "syntax_binder",
        hidden_information: "the opaque hygiene scope a binding carries, which no operation constructs",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Reference,
        spelling: "syntax_reference",
        hidden_information: "the opaque hygiene scope a binding carries, which no operation constructs",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::KindEqual,
        spelling: "token_kind_equal",
        hidden_information: "which of the lexer's kinds two tokens were given, which the phase has no other way to \
                             compare now that a kind is not text",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::DelimiterEqual,
        spelling: "delimiter_equal",
        hidden_information: "which of the fixed grouper's four delimiters a group carries, for the same reason",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::AsExpression,
        spelling: "as_expression",
        hidden_information: "the real parser, run over a tree the adapter holds, which is the only thing that can \
                             establish that the tree parses as an expression",
        family: PhaseFamily::Builder,
    },
    BuiltinOwnership {
        operation: SyntaxOp::Checked,
        spelling: "checked_expression",
        // It used to claim the delimiter set here too, in the shape of "this
        // group names a real delimiter". `Delimiter` answers that where the
        // value is made, so what is left is output well-formedness alone.
        hidden_information: "output well-formedness: unique generated paths, and one binder per binding",
        family: PhaseFamily::Builder,
    },
];

const BOOL: Shape = Shape::Base(Base::Bool);
const NAT: Shape = Shape::Base(Base::Nat);
const RATIO: Shape = Shape::Base(Base::Ratio);
const TEXT: Shape = Shape::Base(Base::Text);
/// Every time operation is registered at written time, because written time is
/// the only coordinate the source language constructs a value of
/// (`02-core-calculus.md` §5.7 fixes it as the score side's). `PhysicalTime`
/// exists in [`Coordinate`] so that the day a physical duration reaches the
/// source it arrives as a *different type* rather than as the same one with a
/// different meaning; registering operations for it before anything can make
/// one would be names nothing could call.
const DURATION: Shape = Shape::Base(Base::Duration(Coordinate::WrittenTime));
const POSITION: Shape = Shape::Base(Base::Position(Coordinate::WrittenTime));
const PITCH: Shape = Shape::Base(Base::Pitch);
const CLASS: Shape = Shape::Base(Base::PitchClass);
const INTERVAL: Shape = Shape::Base(Base::Interval);
const KEY: Shape = Shape::Base(Base::Key);
const SCALE: Shape = Shape::Base(Base::Scale);
const DEGREE: Shape = Shape::Base(Base::Degree);
const FRAME: Shape = Shape::Base(Base::Frame);
const CHORD: Shape = Shape::Base(Base::ChordClass);
const TRIAD: Shape = Shape::Base(Base::Triad);
const ROMAN: Shape = Shape::Base(Base::Roman);
const VOICING: Shape = Shape::Base(Base::Voicing);
const PC12: Shape = Shape::Base(Base::Pc12);
const PCSET12: Shape = Shape::Base(Base::PcSet12);
const ROW12: Shape = Shape::Base(Base::Row12);

const TEXTS: Shape = Shape::List(&TEXT);
const MAYBE_TEXT: Shape = Shape::Option(&TEXT);

pub(crate) const NATS: Shape = Shape::List(&NAT);
const PITCHES: Shape = Shape::List(&PITCH);
const INTERVALS: Shape = Shape::List(&INTERVAL);
pub(crate) const PC12S: Shape = Shape::List(&PC12);
const ROW12S: Shape = Shape::List(&ROW12);

const MAYBE_NAT: Shape = Shape::Option(&NAT);
const MAYBE_CLASS: Shape = Shape::Option(&CLASS);
const MAYBE_DEGREE: Shape = Shape::Option(&DEGREE);
const MAYBE_FRAME: Shape = Shape::Option(&FRAME);
const MAYBE_CHORD: Shape = Shape::Option(&CHORD);
const MAYBE_TRIAD: Shape = Shape::Option(&TRIAD);
const MAYBE_ROMAN: Shape = Shape::Option(&ROMAN);
const MAYBE_VOICING: Shape = Shape::Option(&VOICING);

/// A row, or the two exact reasons a sequence is not one: the order positions
/// whose pitch class already appeared, and the pitch classes it never names
/// (*Open Music Theory*, `108-basics-of-twelve-tone-theory.md`). Both, rather
/// than a choice between them, because a sequence of the wrong length can have
/// either without the other.
const ROW_FAULT: Shape = Shape::Fault;
const ROW12_OR_FAULT: Shape = Shape::Result(&ROW12, &ROW_FAULT);

/// Register a first-order signature, checking it as it is written.
///
/// Every δ entry is built here, so this is the registration site, and a
/// violation is a build error rather than a call that fails once a composer
/// finds it. Four conditions hold of a registered builtin
/// (`docs/rules/language/02-core-calculus.md` §§1.1 and 5.8):
///
/// - **First-order**, and **no closure argument**: [`Shape`] has no arrow
///   constructor, so a signature that wanted a function could not be spelled.
///   An operation that needs one is an eliminator, checked by §5.6.
/// - **Data-only**: every argument and the result is storable data, asserted
///   here.
/// - **Total**: the operation answers on every input its signature admits.
///   Registration cannot see this, so it is the sampling law's D2, which reads
///   [`Shape::admits_absence`] and rejects an evaluator that declined to
///   answer where the shape promised a value.
/// - **Failing by value**: a builtin with one way to fail says so with
///   `Option`, and one with several says which with `Result`. Both injections
///   of a `Result` are values, so answering with one is still total.
const fn delta(arguments: &'static [Shape], result: Shape) -> Family {
    assert!(
        all_storable(arguments),
        "a registered builtin takes storable data; one of these arguments is not"
    );
    assert!(
        result.is_storable(),
        "a registered builtin answers with storable data; this result does not"
    );
    Family::Delta { arguments, result }
}

pub(crate) const BUILTIN_OWNERSHIP: [BuiltinOwnership<Builtin>; 121] = [
    BuiltinOwnership {
        operation: Builtin::NatFold,
        spelling: "nat_fold",
        hidden_information: "the evaluator's finite natural representation and structural work budget",
        family: Family::Eliminator(Eliminator::NatFold),
    },
    BuiltinOwnership {
        operation: Builtin::ListFoldFromStart,
        spelling: "list_fold_from_start",
        hidden_information: "the evaluator's finite list representation and structural work budget",
        family: Family::Eliminator(Eliminator::ListFoldFromStart),
    },
    BuiltinOwnership {
        operation: Builtin::ListFoldFromEnd,
        spelling: "list_fold_from_end",
        hidden_information: "the evaluator's finite list representation, reverse traversal, and structural work budget",
        family: Family::Eliminator(Eliminator::ListFoldFromEnd),
    },
    BuiltinOwnership {
        operation: Builtin::OptionFold,
        spelling: "option_fold",
        hidden_information: "the evaluator's hidden option representation and total case dispatch",
        family: Family::Eliminator(Eliminator::OptionFold),
    },
    BuiltinOwnership {
        operation: Builtin::Map,
        spelling: "map",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Map),
    },
    BuiltinOwnership {
        operation: Builtin::Filter,
        spelling: "filter",
        hidden_information: "rank-1 monomorphization over the evaluator's hidden finite list representation",
        family: Family::Eliminator(Eliminator::Filter),
    },
    BuiltinOwnership {
        operation: Builtin::Range,
        spelling: "range",
        hidden_information: "bounded construction governed by the evaluator's structural work budget",
        family: Family::Eliminator(Eliminator::Range),
    },
    BuiltinOwnership {
        operation: Builtin::Repeat,
        spelling: "repeat",
        hidden_information: "rank-1 finite-list construction governed by the structural work budget",
        family: Family::Eliminator(Eliminator::Repeat),
    },
    BuiltinOwnership {
        operation: Builtin::RatioAdd,
        spelling: "ratio_add",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::RatioSub,
        spelling: "ratio_sub",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::RatioMul,
        spelling: "ratio_mul",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::RatioDiv,
        spelling: "ratio_div",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[RATIO, RATIO], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::RatioLess,
        spelling: "ratio_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[RATIO, RATIO], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::RatioEqual,
        spelling: "ratio_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[RATIO, RATIO], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::TextEqual,
        spelling: "text_equal",
        hidden_information: "the encoding two texts are compared in, which no source expression can inspect",
        family: delta(&[TEXT, TEXT], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::TextJoin,
        spelling: "text_join",
        hidden_information: "how a text is stored and grown, which no source expression can inspect — a program can \
                             build a text and compare two, and has no operation that takes one apart",
        family: delta(&[TEXTS], TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::NatLiteral,
        spelling: "nat_literal",
        hidden_information: "the reader's own numeral grammar, which this is the inverse of rather than a second copy \
                             of",
        family: delta(&[NAT], TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::RatioLiteral,
        spelling: "ratio_literal",
        hidden_information: "the reduced form an exact rational is written in, and the reader's numeral grammar this \
                             is the inverse of",
        family: delta(&[RATIO], MAYBE_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::PitchLiteral,
        spelling: "pitch_literal",
        hidden_information: "the reader's pitch-literal grammar — letter, accidental run, and octave — which this is \
                             the inverse of and which no theory's presentation of a pitch is",
        family: delta(&[PITCH], TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::KeyLiteral,
        spelling: "key_literal",
        hidden_information: "the reader's key-literal grammar, which this is the inverse of",
        family: delta(&[KEY], TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalLiteral,
        spelling: "interval_literal",
        hidden_information: "which sizes and qualities the written interval grammar names, which is where this runs \
                             out and says so",
        family: delta(&[INTERVAL], MAYBE_TEXT),
    },
    BuiltinOwnership {
        operation: Builtin::NatAdd,
        spelling: "nat_add",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::NatMul,
        spelling: "nat_mul",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::NatSub,
        spelling: "nat_sub",
        hidden_information: "the representable range a whole number must stay inside",
        family: delta(&[NAT, NAT], MAYBE_NAT),
    },
    BuiltinOwnership {
        operation: Builtin::DurationOf,
        spelling: "duration_of",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[RATIO], DURATION),
    },
    BuiltinOwnership {
        operation: Builtin::DurationRatio,
        spelling: "duration_ratio",
        hidden_information: "the exact rational a duration is measured by, and its coordinate tag",
        family: delta(&[DURATION], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::DurationAdd,
        spelling: "duration_add",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[DURATION, DURATION], DURATION),
    },
    BuiltinOwnership {
        operation: Builtin::DurationScale,
        spelling: "duration_scale",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[DURATION, RATIO], DURATION),
    },
    BuiltinOwnership {
        operation: Builtin::DurationLess,
        spelling: "duration_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[DURATION, DURATION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::DurationEqual,
        spelling: "duration_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[DURATION, DURATION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PositionOf,
        spelling: "position_of",
        hidden_information: "the origin an instant is measured from, and its coordinate tag",
        family: delta(&[RATIO], POSITION),
    },
    BuiltinOwnership {
        operation: Builtin::PositionRatio,
        spelling: "position_ratio",
        hidden_information: "the origin an instant is measured from, and its coordinate tag",
        family: delta(&[POSITION], RATIO),
    },
    BuiltinOwnership {
        operation: Builtin::PositionShift,
        spelling: "position_shift",
        hidden_information: "exact rational reduction, and whether the reduced result is representable at all",
        family: delta(&[POSITION, DURATION], POSITION),
    },
    BuiltinOwnership {
        operation: Builtin::PositionBetween,
        spelling: "position_between",
        hidden_information: "the nonnegativity every duration constructor checks",
        family: delta(&[POSITION, POSITION], DURATION),
    },
    BuiltinOwnership {
        operation: Builtin::PositionLess,
        spelling: "position_less",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[POSITION, POSITION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PositionEqual,
        spelling: "position_equal",
        hidden_information: "the reduced form two exact rationals share, which no source expression can inspect",
        family: delta(&[POSITION, POSITION], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalAdd,
        spelling: "interval_add",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL, INTERVAL], INTERVAL),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalInverse,
        spelling: "interval_inverse",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL], INTERVAL),
    },
    BuiltinOwnership {
        operation: Builtin::IntervalEqual,
        spelling: "interval_equal",
        hidden_information: "the evaluator's exact written-interval coordinate representation",
        family: delta(&[INTERVAL, INTERVAL], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PitchTransposed,
        spelling: "pitch_transposed",
        hidden_information: "the two coordinates a written pitch moves on at once, and the range either may leave",
        family: delta(&[PITCH, INTERVAL], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::PitchEqual,
        spelling: "pitch_equal",
        hidden_information: "the letter, alteration, and octave a written pitch is stored as",
        family: delta(&[PITCH, PITCH], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::PitchClassTransposed,
        spelling: "pitchclass_transposed",
        hidden_information: "the spelling-preserving quotient a class is transposed in, and the range it may leave",
        family: delta(&[CLASS, INTERVAL], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::PitchClassOf,
        spelling: "pitchclass_of",
        hidden_information: "the written pitch's octave coordinate and spelling-preserving quotient",
        family: delta(&[PITCH], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::SignatureScale,
        spelling: "signature_scale",
        hidden_information: "the compiler's table of named collections, which no source text can enumerate",
        family: delta(&[KEY], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleOn,
        spelling: "scale_on",
        hidden_information: "the scale's private ordered offset cycle, re-rooted without being exposed",
        family: delta(&[SCALE, CLASS], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleTonic,
        spelling: "scale_tonic",
        hidden_information: "the scale's private tonic field",
        family: delta(&[SCALE], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleSize,
        spelling: "scale_size",
        hidden_information: "the length of the scale's private offset cycle",
        family: delta(&[SCALE], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::ScalePitch,
        spelling: "scale_pitch",
        hidden_information: "spelled membership against the scale's private offset cycle",
        family: delta(&[SCALE, PITCH], MAYBE_DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleClass,
        spelling: "scale_class",
        hidden_information: "the scale's private offset cycle, read without a register",
        family: delta(&[SCALE, DEGREE], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ScaleChord,
        spelling: "scale_chord",
        hidden_information: "the scale's offset cycle and the chord vocabulary's member table at once",
        family: delta(&[SCALE, DEGREE, NAT], MAYBE_CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::PitchFrame,
        spelling: "pitch_frame",
        hidden_information: "the register frame's representation invariant, which only the compiler can enforce",
        family: delta(&[SCALE, PITCH], MAYBE_FRAME),
    },
    BuiltinOwnership {
        operation: Builtin::FrameScale,
        spelling: "frame_scale",
        hidden_information: "the frame's private scale field",
        family: delta(&[FRAME], SCALE),
    },
    BuiltinOwnership {
        operation: Builtin::FrameTonic,
        spelling: "frame_tonic",
        hidden_information: "the frame's private registered tonic field",
        family: delta(&[FRAME], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::FramePitch,
        spelling: "frame_pitch",
        hidden_information: "Euclidean division of a degree through the scale's private period",
        family: delta(&[FRAME, DEGREE], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeOf,
        spelling: "degree_of",
        hidden_information: "the degree's private signed coordinate, which is not the written ordinal",
        family: delta(&[NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeStepUp,
        spelling: "degree_step_up",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeStepDown,
        spelling: "degree_step_down",
        hidden_information: "the degree's private signed coordinate and its machine-integer bound",
        family: delta(&[DEGREE, NAT], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeRaised,
        spelling: "degree_raised",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::DegreeLowered,
        spelling: "degree_lowered",
        hidden_information: "the degree's private chromatic alteration and its machine-integer bound",
        family: delta(&[DEGREE], DEGREE),
    },
    BuiltinOwnership {
        operation: Builtin::ChordOn,
        spelling: "chord_on",
        hidden_information: "the compiler's table of chord types, re-rooted without being exposed",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordRoot,
        spelling: "chord_root",
        hidden_information: "the chord class's private root field",
        family: delta(&[CHORD], CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordBass,
        spelling: "chord_bass",
        hidden_information: "the chord class's private bass designation, which is absent and not the root",
        family: delta(&[CHORD], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordMembers,
        spelling: "chord_members",
        hidden_information: "the private spelled member stack, which no source text can enumerate",
        family: delta(&[CHORD], INTERVALS),
    },
    BuiltinOwnership {
        operation: Builtin::ChordInversion,
        spelling: "chord_inversion",
        hidden_information: "membership of the private member stack, which is what makes an inversion true",
        family: delta(&[CHORD, NAT], MAYBE_CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordOver,
        spelling: "chord_over",
        hidden_information: "the chord class's private bass designation",
        family: delta(&[CHORD, CLASS], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::ChordTriad,
        spelling: "chord_triad",
        hidden_information: "the triad refinement's representation invariant, which only the compiler can enforce",
        family: delta(&[CHORD], MAYBE_TRIAD),
    },
    BuiltinOwnership {
        operation: Builtin::TriadChord,
        spelling: "triad_chord",
        hidden_information: "the triad refinement's private witness",
        family: delta(&[TRIAD], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::RomanOf,
        spelling: "roman_of",
        hidden_information: "the numeral's representation invariant, which only the compiler can enforce",
        family: delta(&[NAT, NAT, NAT], MAYBE_ROMAN),
    },
    BuiltinOwnership {
        operation: Builtin::RomanOrdinal,
        spelling: "roman_ordinal",
        hidden_information: "the numeral's private ordinal",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::RomanSize,
        spelling: "roman_size",
        hidden_information: "the numeral's private member count",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::RomanInversion,
        spelling: "roman_inversion",
        hidden_information: "the numeral's private bass designation, which is a position and not a pitch",
        family: delta(&[ROMAN], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::TriadMajor,
        spelling: "triad_major",
        hidden_information: "the chord class's private type, which is the only place the two triads differ",
        family: delta(&[TRIAD], BOOL),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingOf,
        spelling: "voicing_of",
        hidden_information: "the voicing's representation invariant: ascending distinct pitches drawn from the class",
        family: delta(&[CHORD, PITCHES], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingPitches,
        spelling: "voicing_pitches",
        hidden_information: "the voicing's private ordered pitch sequence",
        family: delta(&[VOICING], PITCHES),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingBass,
        spelling: "voicing_bass",
        hidden_information: "the voicing's private lowest pitch, held apart from the rest",
        family: delta(&[VOICING], PITCH),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingChord,
        spelling: "voicing_chord",
        hidden_information: "the voicing's private association to the class it voices",
        family: delta(&[VOICING], CHORD),
    },
    BuiltinOwnership {
        operation: Builtin::VoicingPosition,
        spelling: "voicing_position",
        hidden_information: "membership of the private member stack, which is what classifies an inversion",
        family: delta(&[VOICING], MAYBE_NAT),
    },
    BuiltinOwnership {
        operation: Builtin::CloseVoicing,
        spelling: "close_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::DropVoicing,
        spelling: "drop_voicing",
        hidden_information: "the private member stack walked upward, and the voicing invariant it must satisfy",
        family: delta(&[CHORD, PITCH, NAT], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::OmitVoicing,
        spelling: "omit_voicing",
        hidden_information: "the private member stack, which is what says which pitch an omission removes",
        family: delta(&[VOICING, NAT], MAYBE_VOICING),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Of,
        spelling: "pc12_of",
        hidden_information: "the canonical representative of a residue class modulo twelve",
        family: delta(&[NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Number,
        spelling: "pc12_number",
        hidden_information: "the canonical representative, which is the only number a residue class has",
        family: delta(&[PC12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Forget,
        spelling: "pc12_forget",
        hidden_information: "the chromatic coordinate of a spelled pitch class, taken modulo twelve",
        family: delta(&[CLASS], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Transposed,
        spelling: "pc12_transposed",
        hidden_information: "modular addition, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Inverted,
        spelling: "pc12_inverted",
        hidden_information: "modular subtraction, which a `nat` without subtraction cannot express",
        family: delta(&[PC12, NAT], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Pc12Spelled,
        spelling: "pc12_spelled",
        hidden_information: "the collection's spelled members, searched for the one this class forgets to",
        family: delta(&[PC12, SCALE], MAYBE_CLASS),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Of,
        spelling: "pcset12_of",
        hidden_information: "the twelve-bit membership word that makes duplication unrepresentable",
        family: delta(&[PC12S], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Members,
        spelling: "pcset12_members",
        hidden_information: "the membership word, read out ascending",
        family: delta(&[PCSET12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Transposed,
        spelling: "pcset12_transposed",
        hidden_information: "the membership word, rotated by the index without unpacking it",
        family: delta(&[PCSET12, NAT], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Inverted,
        spelling: "pcset12_inverted",
        hidden_information: "the membership word, reflected about the index without unpacking it",
        family: delta(&[PCSET12, NAT], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Normal,
        spelling: "pcset12_normal",
        hidden_information: "every rotation of the set and the compactness order that chooses between them",
        family: delta(&[PCSET12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Prime,
        spelling: "pcset12_prime",
        hidden_information: "the normal orders of the set and its inversion, and which of the two reads lower",
        family: delta(&[PCSET12], PCSET12),
    },
    BuiltinOwnership {
        operation: Builtin::PcSet12Vector,
        spelling: "pcset12_vector",
        hidden_information: "every unordered pair of members and the interval class each realizes",
        family: delta(&[PCSET12], NATS),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Of,
        spelling: "row12_of",
        hidden_information: "the permutation invariant: twelve order positions and each pitch class once",
        family: delta(&[PC12S], ROW12_OR_FAULT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Pcs,
        spelling: "row12_pcs",
        hidden_information: "the private order-position array",
        family: delta(&[ROW12], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Head,
        spelling: "row12_head",
        hidden_information: "order position zero of the private array, which the finite list eliminators cannot index",
        family: delta(&[ROW12], PC12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Transposed,
        spelling: "row12_transposed",
        hidden_information: "modular addition, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Inverted,
        spelling: "row12_inverted",
        hidden_information: "modular subtraction, and the finite-closure lemma that keeps the result a row",
        family: delta(&[ROW12, NAT], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Retrograde,
        spelling: "row12_retrograde",
        hidden_information: "reversal of the order positions, which the finite list eliminators cannot express",
        family: delta(&[ROW12], ROW12),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Matrix,
        spelling: "row12_matrix",
        hidden_information: "the classical construction: the inversion about the row's own head, read as starting pitches",
        family: delta(&[ROW12], ROW12S),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Forms,
        spelling: "row12_forms",
        hidden_information: "the forty-eight labelled forms, compared for equality and counted once each",
        family: delta(&[ROW12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Symmetries,
        spelling: "row12_symmetries",
        hidden_information: "the forty-eight labelled forms, counted where they fix the row",
        family: delta(&[ROW12], NAT),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Repeats,
        spelling: "row12_repeats",
        hidden_information: "pitch-class equality, which the surface has no operator for",
        family: delta(&[PC12S], NATS),
    },
    BuiltinOwnership {
        operation: Builtin::Row12Missing,
        spelling: "row12_missing",
        hidden_information: "pitch-class equality against the whole finite domain",
        family: delta(&[PC12S], PC12S),
    },
    BuiltinOwnership {
        operation: Builtin::Transpose,
        spelling: "transpose",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Stretch,
        spelling: "stretch",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Retrograde,
        spelling: "retrograde",
        hidden_information: "contextual music extent, occurrence provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Invert,
        spelling: "invert",
        hidden_information: "contextual music representation, written-pitch provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Shift,
        spelling: "shift",
        hidden_information: "contextual music representation, exact-time provenance, and kernel construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Together,
        spelling: "together",
        hidden_information: "contextual music representation, origin paths, and kernel stacking construction",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::MapNotePitches,
        spelling: "map_note_pitches",
        hidden_information: "controlled traversal of contextual notes while preserving non-note facts and provenance",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Play,
        spelling: "play",
        hidden_information: "contextual music construction: the voicing's private pitches become sounded occurrences with provenance",
        family: Family::Track,
    },
    BuiltinOwnership {
        operation: Builtin::Primitive,
        spelling: "primitive",
        hidden_information: "the build-local registry: which unit a name and version select, and that unit's private state layout, start, and step",
        family: Family::Machine(MachineOp::Primitive),
    },
    BuiltinOwnership {
        operation: Builtin::Machine,
        spelling: "machine",
        hidden_information: "the exact configuration encoding a registered unit is instantiated with",
        family: Family::Machine(MachineOp::Machine),
    },
    BuiltinOwnership {
        operation: Builtin::Identity,
        spelling: "identity",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Identity),
    },
    BuiltinOwnership {
        operation: Builtin::Connect,
        spelling: "connect",
        hidden_information: "the machine description's node representation and the order its children are stored in",
        family: Family::Machine(MachineOp::Connect),
    },
    BuiltinOwnership {
        operation: Builtin::Beside,
        spelling: "beside",
        hidden_information: "the machine description's node representation and the order its children are stored in",
        family: Family::Machine(MachineOp::Beside),
    },
    BuiltinOwnership {
        operation: Builtin::Feedback,
        spelling: "feedback",
        hidden_information: "the exact encoding of the stored value the first step reads",
        family: Family::Machine(MachineOp::Feedback),
    },
    BuiltinOwnership {
        operation: Builtin::Copy,
        spelling: "copy",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Copy),
    },
    BuiltinOwnership {
        operation: Builtin::Drop,
        spelling: "drop",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Drop),
    },
    BuiltinOwnership {
        operation: Builtin::Swap,
        spelling: "swap",
        hidden_information: "the machine description's node representation",
        family: Family::Machine(MachineOp::Swap),
    },
];

/// The type a written name denotes in an adapter module, and nowhere else.
///
/// Deliberately absent from `musa-language`'s `BASE_TYPES`: these are not
/// spellings the parser offers, the language server completes, or a composer
/// can write. They are read only where [`crate::data::TypeScope::in_phase`]
/// holds, which is the same boundary [`Reading::Expansion`] draws for the
/// phase's operations — one line between the two languages rather than two.
///
/// The four here take no arguments. `Syntax<Cat>` and `SyntaxStep<C, A>` do, so
/// they are read where the other applied forms are, under the same `in_phase`
/// gate.
///
/// `Syntax` is deliberately absent from the bare list. It takes a category, so
/// the bare word names no type — the same reason `Duration` is absent from
/// [`named_type`], and for the same reason an adapter that writes it is told
/// what to write instead rather than being handed one category by default.
fn phase_type(text: &str) -> Option<Type> {
    match text {
        "NodePath" => Some(Type::NodePath),
        "BindingPath" => Some(Type::BindingPath),
        "TokenKind" => Some(Type::TokenKind),
        "Delimiter" => Some(Type::Delimiter),
        _ => None,
    }
}

fn child_of(node: &SyntaxNode, predicate: fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
    node.children().find(|child| predicate(child.kind()))
}

fn is_type_node(kind: SyntaxKind) -> bool {
    musa_language::ast::is_type(kind)
}

/// The type a declaration or parameter annotates, as a node.
pub(crate) fn type_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_type_node)
}

/// The expression a wrapper node holds, as a node.
pub(crate) fn expr_node_of(node: &SyntaxNode) -> Option<SyntaxNode> {
    child_of(node, is_expr_node)
}

fn is_expr_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::BlockExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ResultExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::LambdaExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ChordExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::IfExpr
            | SyntaxKind::RecordUpdateExpr
            | SyntaxKind::QuestionExpr
            | SyntaxKind::MusicExpr
            | SyntaxKind::KernelQuote
            | SyntaxKind::QuoteExpr
    )
}

/// One adapter module, elaborated in the phase environment.
///
/// **This is the phase environment**, and the only place a [`crate::document::Source`]
/// is read [`in_phase`](crate::document::Source::in_phase). Everything about it
/// is the ordinary machinery: the same declarations, the same bidirectional
/// elaborator, the same core, the same budget. What is phase-local is the
/// *environment* — [`crate::prelude::phase`]'s vocabulary answers a name here and
/// nowhere else — which is what keeps `02-core-calculus.md` §5's closed source
/// type grammar and its "no syntax value" sentence true of the language a
/// composer writes.
///
/// A module and not an expression. An adapter's operations read the module's
/// own `let`, `fn`, and `data`, because they are declarations of the module
/// those operations are declared in, and because a reader written without local
/// definitions is a reader nobody can follow (Peyton Jones ch. 3).
///
/// # What the phase no longer supplies
///
/// The types of `expand` and `edit`. They used to be a table here, matched
/// against what the checker inferred; now the adapter writes them, and the
/// application below is what checks that it wrote the right one. The reason is
/// the reason [`Printer`] already gives for `print`: a bidirectional elaborator
/// settles a `fn` where the `fn` stands, so a definition whose type is held in a
/// table beside the file is a definition the file cannot be read without. Every
/// adapter in `stdlib/src/adapters/` now writes both signatures.
pub(crate) struct AdapterModule {
    /// The module's declarations, elaborated once.
    read: crate::document::Document,
    /// `print`, which is the one declaration not read with the module.
    ///
    /// A printer's argument is the *package's* type — what its regions produce
    /// — and the package a region belongs to is not a module the adapter
    /// imports, so a standalone check of the printer would have nothing to
    /// settle its parameter against. So the printer is read where it is run,
    /// against the value it is handed ([`print_value`]), which is also why it is
    /// the one operation that never sees the phase environment.
    ///
    /// It is still read *with* the module declarations it names, because "not
    /// in the phase environment" is a statement about what is in scope and not
    /// a licence to take the module away. See [`Printer`].
    printer: Option<Printer>,
}

impl AdapterModule {
    /// What checking this module charged.
    ///
    /// `26-language-design-decision.md` §3.5's checking counters, for a run to
    /// add to its own. The module is where an adapter's *checking* actually
    /// happens — the run itself is one application of an operation whose type
    /// the module already fixed — so a phase that reported only the run would
    /// charge nothing for the expensive half and let a file buy unbounded
    /// checking by arranging to be refused.
    pub(crate) const fn spend(&self) -> musa_core::Spend {
        self.read.spend()
    }

    /// The text a declaration holds, when it holds one.
    ///
    /// How `level` is read: the declared level is a `Text` the module
    /// evaluates to, so asking for it is asking the module for one of its own
    /// values rather than matching the shape of its source.
    pub(crate) fn text(&self, name: &str) -> Option<String> {
        let (normal, _) = self.read.value(name).ok()?;
        crate::registry::read_back::<String>(&normal).ok().cloned()
    }

    /// Whether the module declares `name` at all.
    pub(crate) fn declares(&self, name: &str) -> bool {
        self.read.names().iter().any(|bound| &**bound == name) || (name == "print" && self.printer.is_some())
    }

    /// The printer, for the one operation read at its use site.
    pub(crate) fn printer(&self) -> Option<&Printer> {
        self.printer.as_ref()
    }

    /// Apply the operation `name` to `arguments`, and normalize the answer.
    ///
    /// One step where there used to be three. The operation's type was compared
    /// against a table, its value was pulled out as a closure, and the closure
    /// was applied by a second evaluator; now the call is written as a term and
    /// elaborated in the module's own context, so "is this an `expand`?" and
    /// "what does it answer here?" are the one question the elaborator already
    /// answers. An adapter whose `expand` takes the wrong thing is refused by
    /// the conversion check at the argument, which says which type it found and
    /// where — the sentence [`not_the_operation`] used to approximate.
    ///
    /// The arguments are terms rather than values because that is the phase's
    /// side of the boundary: a region is a literal at `Syntax ⟨token-tree⟩`, a
    /// command is a literal at `Text`, and an anchor is the prelude's `Nat`.
    fn run(&self, name: &str, arguments: Vec<Raw>) -> Result<(musa_core::Datum, musa_core::Spend), Unrun> {
        if !self.declares(name) {
            return Err(Unrun::Undeclared);
        }
        let here = musa_core::Origin::UNKNOWN;
        let call = Raw::call(here, Raw::var(here, name), arguments);
        let ((normal, _), spend) = self.read.term_metered(&call).map_err(|error| match error {
            musa_core::ElabError::Exhausted(_) => Unrun::Stopped,
            error @ (musa_core::ElabError::Refused(_) | musa_core::ElabError::Malformed(_)) => {
                Unrun::Refused(vec![crate::lower::refusals::restate(self.read.sites(), &error)])
            }
        })?;
        // Canonicity, and the whole of what it is for: the call was checked
        // before it was normalized, so a closed answer at a declared family *is*
        // one of its constructors. A term that is not one is this crate's defect
        // rather than an adapter's, which is why the two callers report it as
        // "no answer" rather than as a refusal with the adapter's name on it.
        let datum = musa_core::canonical(&normal).ok_or(Unrun::NoAnswer)?;
        Ok((datum, spend))
    }
}

/// Why an operation produced no answer.
///
/// Four cases and not one because the two callers restate them into two
/// different vocabularies — a transformer that is not a transformer and an
/// editor that is not an editor are different sentences — and because
/// [`ExpansionFailure`] and [`EditFailure`] each already keep a stop apart from a
/// fault, which this has to be able to tell them.
enum Unrun {
    /// The module declares nothing under that name.
    Undeclared,
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The application did not elaborate.
    Refused(Vec<Diagnostic>),
    /// It elaborated and its normal form was not canonical, which is a defect
    /// in this crate rather than in the adapter.
    NoAnswer,
}

/// Where an adapter module's own imports resolve.
///
/// Two things and not one because an import is resolved *relative to the module
/// that writes it*: `document` is the key the syntax import itself resolved to,
/// and `sources` is the text of every file this compilation may read. A phase
/// that took only the second would resolve `./sibling.musa` against whichever
/// file happened to be compiling.
///
/// Bundled `std::` modules answer out of [`crate::imports::ImportSources`]
/// without having been inserted into it, so a caller that hands over a default
/// one is still handing over the standard library rather than an empty world.
#[derive(Clone, Copy)]
pub(crate) struct PhaseImports<'a> {
    document: &'a str,
    sources: &'a crate::imports::ImportSources,
}

impl<'a> PhaseImports<'a> {
    /// The closure of the module stored under `document`.
    pub(crate) fn at(document: &'a str, sources: &'a crate::imports::ImportSources) -> Self {
        Self { document, sources }
    }
}

#[cfg(test)]
impl PhaseImports<'static> {
    /// The bundled standard library, for a module written in a test rather than
    /// read out of a package.
    ///
    /// A default [`crate::imports::ImportSources`] is not an empty world: it
    /// falls back to the embedded `std::` modules, which is exactly the closure
    /// a test module that writes `import std::list;` should get.
    pub(crate) fn bundled() -> Self {
        static SOURCES: std::sync::LazyLock<crate::imports::ImportSources> =
            std::sync::LazyLock::new(crate::imports::ImportSources::default);
        Self {
            document: "adapter.musa",
            sources: &SOURCES,
        }
    }
}

/// Check and evaluate one adapter module, in the phase environment.
///
/// The diagnostics come back rather than being reported: they are about the
/// adapter package's own document, and publishing a span inside it as a span in
/// the composer's file is exactly what the source map exists to prevent. The
/// caller decides which of its own spans to restate them at.
pub(crate) fn read_adapter_module(source: &str, imports: PhaseImports<'_>) -> Result<AdapterModule, ModuleFault> {
    let parsed = musa_language::parse(source);
    if let Some(error) = parsed.errors().first() {
        return Err(ModuleFault::Broken(vec![Diagnostic::error(
            Code::Expansion,
            format!("it does not parse: {}", error.message()),
        )]));
    }
    let root = parsed.syntax();
    let Some(library) = musa_language::ast::LibraryDecl::from_root(&root) else {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "an adapter module is a `library`").help(
                "write the module as `library { let level = …; let expand = …; }`, the way `stdlib/src/adapters/` does",
            ),
        ]));
    };
    // The adapter-free bootstrap, checked rather than assumed. An adapter whose
    // own definition needed an adapter would put the expansion order back into
    // a cycle, and this pair of refusals is the whole of what prevents it —
    // which is also why termination is structural and needs no rank arithmetic.
    // An *ordinary* import is not in the cycle and is loaded below: it brings a
    // module's declarations in, which is the one thing `02-core-calculus.md`
    // §5.9 says the phase adds rather than takes away.
    // Root and library both, for the reason [`ordinary_imports`] reads both: an
    // import stands at a document's lexical root or inside its `library`, and
    // which one an author chose is not something the phase should depend on.
    let written: Vec<musa_language::ast::ImportStmt> = root
        .descendants()
        .filter_map(musa_language::ast::ImportStmt::cast)
        .collect();
    if let Some(reader) = written.iter().find(|import| import.changes_syntax()) {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "it is written with an adapter of its own")
                .at(crate::resolve::trimmed_span(reader.syntax()), "this syntax import")
                .help("an adapter is written in the adapter-free bootstrap: no region, no syntax import"),
        ]));
    }
    if root.descendants().any(|node| node.kind() == SyntaxKind::SyntaxRegion) {
        return Err(ModuleFault::Broken(vec![
            Diagnostic::error(Code::Expansion, "it is written with an adapter of its own")
                .help("an adapter is written in the adapter-free bootstrap: no region, no syntax import"),
        ]));
    }
    let mut resolver = Resolver::new();
    let libraries = crate::imports::load(&mut resolver, imports.document, &written, imports.sources);
    if resolver
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
    {
        return Err(ModuleFault::Broken(resolver.diagnostics));
    }
    let printer = printer_source(library.syntax());
    let sources: Vec<crate::document::Source> = libraries
        .each()
        .map(|(from, imported)| crate::document::Source::imported(imported.syntax(), from).in_phase())
        .chain(std::iter::once(
            crate::document::Source::own(library.syntax())
                .in_phase()
                .without(printer.iter().flat_map(Printer::left_to_it)),
        ))
        .collect();
    let Some(read) = crate::document::elaborate(&mut resolver, &sources, None) else {
        return Err(module_fault(resolver.diagnostics));
    };
    // A document that came back is not yet a module that checks: `elaborate`
    // answers `Some` for a document whose *declarations* all landed, and the
    // refusals a declaration world reports — a field storing an arrow, a name
    // declared twice — are in the resolver beside it. Answering `Ok` while it
    // holds an error would be an adapter running with a declaration the checker
    // had already refused.
    let refusals: Vec<Diagnostic> = resolver
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error)
        .collect();
    if !refusals.is_empty() {
        return Err(module_fault(refusals));
    }
    Ok(AdapterModule { read, printer })
}

/// A stop when a limit was crossed, and the module's own complaints when not.
///
/// Read off the diagnostics rather than off a meter, because the meter is
/// [`musa_core`]'s now and it reports exhaustion the way it reports everything
/// else — as a refusal, filed under [`Code::ResourceLimit`] by
/// [`crate::lower::refusals::restate`].
fn module_fault(diagnostics: Vec<Diagnostic>) -> ModuleFault {
    if diagnostics.iter().any(|it| it.code == Code::ResourceLimit) {
        ModuleFault::Stopped
    } else {
        ModuleFault::Broken(diagnostics)
    }
}

/// Why a module could not be read as an adapter module.
///
/// Two cases and not one, for the reason [`ExpansionFailure`] separates the
/// same pair: a compilation that ran out of budget has said nothing about the
/// module, and reporting it as a broken adapter would make a narrowed budget
/// look like a package that does not compile.
pub(crate) enum ModuleFault {
    /// A compilation limit was crossed before the module finished checking.
    Stopped,
    /// It is not an adapter module, and these say why.
    Broken(Vec<Diagnostic>),
}

/// `print`, and the module declarations it reads.
///
/// The one operation read as text rather than as a value, for the reason
/// [`AdapterModule::printer`] gives — and read *with* its module rather than
/// alone. A musa block holds exactly one expression and the language has no
/// `let` expression, so an operation read as a bare expression cannot bind a
/// single local name: it would be the language minus local definitions, which
/// is the sublanguage by subtraction root `AGENTS.md` forbids and the shape
/// Peyton Jones ch. 3 enriches the calculus to avoid. `expand` and `edit` never
/// felt it because they are checked with the module; this is what gives `print`
/// the same.
///
/// The declarations it *names*, transitively, and not the whole module — which
/// makes a module two parts rather than one. A helper written for the printer
/// names the package's types, and the phase has no such types, so a module
/// checked as one piece could not hold one at all: the phase would refuse the
/// helper before the printer was ever read. So a declaration belongs to
/// whichever operations name it. `level`, `expand` and `edit` are the phase's
/// roots; `print` is the printer's; a declaration both reach is checked twice,
/// once under each reading, which is the honest answer for a helper that is
/// genuinely both. One neither reaches is dead, and stays with the phase so
/// that it is still checked rather than quietly ignored.
///
/// # Why the lambda is kept in two pieces
///
/// [`run_printer`] splices it back as a `fn` declaration rather than as a `let`
/// bound to a lambda, and needs the halves apart to do it. The reason is
/// bidirectional: `let printer = fn (value: Text) { match value { … } };` gives
/// the elaborator a domain and no codomain, so the arms are *inferred*, and
/// `Ok("hello")` on its own infers `Result Text ?e` with nothing to solve `?e`
/// — an unsolved metavariable in a printer that is perfectly well typed. Written
/// as `fn printer(value: Text) -> Result<Text, Text> { … }`, the same arms are
/// *checked*, and the error type comes from the annotation `04-adapters.md` §4
/// already fixes for every printer. Algorithm W did not need this because it
/// solved the whole piece at once; a checker that reads a definition where it
/// stands needs the type written where it stands.
pub(crate) struct Printer {
    /// The parameter list, verbatim — `(value: Text)`.
    params: String,
    /// Everything after it: the body, and the `-> T` before it when the adapter
    /// wrote one.
    answer: String,
    /// Whether it did. When it did not, [`run_printer`] supplies the return type
    /// §4 fixes; when it did, the adapter's own words stand and `printed`'s
    /// annotation is what checks the two agree.
    says_answer: bool,
    /// The module declarations the body reaches, in the order the module writes
    /// them, verbatim.
    reached: Vec<String>,
    /// The names of those the phase must *not* check, being the printer's alone.
    private: IndexSet<String>,
}

impl Printer {
    /// Every name the phase leaves to the printer, `print` included.
    ///
    /// What [`crate::document::Source::without`] is handed. `print` is on the
    /// list because it is the operation itself and the rest because they are
    /// reached from nowhere else, and both halves are the same sentence: a
    /// declaration whose type only the *package* names cannot be read in a phase
    /// that has no package.
    fn left_to_it(&self) -> impl Iterator<Item = String> {
        std::iter::once("print".to_owned()).chain(self.private.iter().cloned())
    }
}

/// The declarations a set of root names reaches, transitively.
///
/// By repetition rather than by recursion: each pass takes the declarations the
/// names so far reach and adds what *they* name, until a pass adds nothing.
fn reached_by(module: &[(Vec<String>, IndexSet<String>, String)], roots: IndexSet<String>) -> Vec<bool> {
    let mut wanted = roots;
    let mut taken = vec![false; module.len()];
    loop {
        let mut grew = false;
        for (reached, (binds, names, _)) in taken.iter_mut().zip(module) {
            if *reached || !binds.iter().any(|name| wanted.contains(name)) {
                continue;
            }
            *reached = true;
            wanted.extend(names.iter().cloned());
            grew = true;
        }
        if !grew {
            return taken;
        }
    }
}

fn printer_source(library: &SyntaxNode) -> Option<Printer> {
    let whole = library.to_string();
    let base = usize::try_from(u32::from(library.text_range().start())).ok()?;
    let written = |from: u32, to: u32| -> Option<String> {
        let from = usize::try_from(from).ok()?.checked_sub(base)?;
        let to = usize::try_from(to).ok()?.checked_sub(base)?;
        Some(whole.get(from..to)?.trim().to_owned())
    };
    // What each declaration binds, what it names, and its own source — in the
    // order the module writes them, because that is the order they have to be
    // spliced back in.
    let mut module: Vec<(Vec<String>, IndexSet<String>, String)> = Vec::new();
    let mut printer = None;
    for node in library.children() {
        let range = node.text_range();
        if matches!(node.kind(), SyntaxKind::LetDecl | SyntaxKind::FnDecl) {
            let Some(name) = declared_name(&node) else { continue };
            if node.kind() == SyntaxKind::LetDecl && name == "print" {
                printer = Some(printer_body(&node, &written)?);
            } else {
                module.push((
                    vec![name],
                    names_in(&node),
                    written(range.start().into(), range.end().into())?,
                ));
            }
        } else if let Some(declaration) = musa_language::ast::DataDecl::cast(node.clone()) {
            // A `data` is reached by its type, by any of its constructors, or
            // by the fold generated for it: those are the whole of what naming
            // it can look like from a printer.
            let mut binds: Vec<String> = declaration.variants().iter().filter_map(|it| it.name()).collect();
            if let Some(name) = declaration.name() {
                binds.push(crate::data::fold_name(&name));
                binds.push(name);
            }
            module.push((
                binds,
                names_in(&node),
                written(range.start().into(), range.end().into())?,
            ));
        }
    }
    let (params, answer, says_answer) = printer?;
    let printers = reached_by(&module, names_in_text(&format!("fn {params} {answer}")));
    let phases = reached_by(
        &module,
        ["level", "expand", "edit"].into_iter().map(str::to_owned).collect(),
    );
    let mut reached = Vec::new();
    let mut private = IndexSet::new();
    for ((binds, names, text), (&mine, &theirs)) in module.into_iter().zip(printers.iter().zip(&phases)) {
        if !mine || names_a_phase_type(&names) {
            continue;
        }
        if !theirs {
            private.extend(binds);
        }
        reached.push(text);
    }
    Some(Printer {
        params,
        answer,
        says_answer,
        reached,
        private,
    })
}

/// Whether a declaration writes down one of the phase's own types.
///
/// The one place the printer's reachability may not over-reach. A declaration
/// naming `Syntax`, `NodePath`, `BindingPath` or `SyntaxStep` is the phase's:
/// the ordinary reading has no such type, so splicing it does not cost "a
/// little checking" — it refuses the whole piece, and refuses it with a
/// complaint about a declaration the printer never asked for. So the phase
/// keeps it, and a printer that genuinely reached one is refused for the name
/// it wrote rather than for a type it did not.
///
/// This is the same boundary [`Printer`] states, applied to the declaration
/// rather than to the type: a `data` is reached by any of its constructors, and
/// a constructor name an adapter shares with the package it reads — `Untied` is
/// both `Tying`'s and `Tie`'s — is exactly where the over-approximation
/// otherwise drags the phase's half into the printer's piece.
fn names_a_phase_type(names: &IndexSet<String>) -> bool {
    names
        .iter()
        .any(|name| phase_type(name).is_some() || matches!(name.as_str(), "Syntax" | "SyntaxStep"))
}

/// `let print = fn (…) …;` split where the parameter list ends.
///
/// [`None`] when the declaration's value is not a lambda at all, which refuses
/// the module as "not a printer" — the same answer §4 gives for a `print` that
/// is not `fn (value: T) { … }`, reached one step earlier.
fn printer_body(
    declaration: &SyntaxNode,
    written: &impl Fn(u32, u32) -> Option<String>,
) -> Option<(String, String, bool)> {
    let lambda = declaration
        .children()
        .find(|node| node.kind() == SyntaxKind::LambdaExpr)?;
    let params = lambda
        .children()
        .find(|node| node.kind() == SyntaxKind::ParamList)?
        .text_range();
    Some((
        written(params.start().into(), params.end().into())?,
        written(params.end().into(), lambda.text_range().end().into())?,
        lambda.children_with_tokens().any(|it| it.kind() == SyntaxKind::Arrow),
    ))
}

/// Every name written anywhere inside a declaration.
///
/// Deliberately more than the names it *reads*: a parameter, a field, and a
/// bound pattern variable all land here too. Over-reaching mostly splices a
/// declaration the printer did not need, which costs a little checking;
/// under-reaching would leave a name unbound and refuse a printer that was
/// correct. The one case where over-reaching is not cheap is a declaration
/// written in the phase's language, and [`names_a_phase_type`] is what keeps
/// that one out.
fn names_in(node: &SyntaxNode) -> IndexSet<String> {
    node.descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
        .collect()
}

/// The same over source text, for the printer's own body, which is text by the
/// time anything asks what it names.
fn names_in_text(source: &str) -> IndexSet<String> {
    names_in(&musa_language::parse(&format!("library {{ let named = {source}; }}")).syntax())
}

/// The name a `let` declares.
fn declared_name(declaration: &SyntaxNode) -> Option<String> {
    declaration
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

/// What running a transformer over a region produced, or why it did not.
///
/// The three failures are different mistakes and a transformer author reading
/// one should be told which they made, which is why this is a type rather than
/// a `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExpansionFailure {
    /// The adapter read the region and would not have it.
    ///
    /// Not a fault: a transformer that answers `Err` has *worked*, and what it
    /// says is the adapter package's sentence about the composer's text. The
    /// node is how it points — `26-language-design-decision.md` §3.4 gives a
    /// transformer no way to read a source range, so it hands back a node it
    /// was given and the range is read from that.
    Refused {
        /// The adapter's own sentence.
        message: String,
        /// Where it pointed, when it pointed at something the composer wrote.
        /// `None` means the adapter pointed at a node it built itself, which
        /// has no text under it.
        at: Option<SourceSpan>,
    },
    /// A compilation limit was crossed before the run finished.
    ///
    /// Not a fault in the adapter and not a fault in the region: a transformer
    /// is total, so this is the meter stopping a run rather than a run that
    /// would not have stopped. It is its own case because
    /// `docs/rules/language/00-semantics.md` §2 makes the difference matter —
    /// a stop must not read as a file that is not well-typed.
    Stopped,
    /// The transformer did not check as `Syntax -> Syntax`.
    NotATransformer(Vec<Diagnostic>),
    /// It checked and then did not answer — a budget crossed, or the evaluator
    /// and the checker disagreeing, which is a compiler fault rather than a
    /// language effect.
    NoAnswer,
    /// It answered with something that is not a well-formed expression.
    NotAnExpression(crate::syntax::NotAnExpression),
}

/// Run one transformer expression over one region, in the phase environment.
///
/// A test helper, and the one place a bare `expand` expression is still wrapped
/// into a module for the phase to read: a law about the fold or about a builder
/// is about that expression, and making each such test write a whole `library`
/// around it would bury the law in ceremony. Everything else — the compiler's
/// own path, and every test about an adapter *module* — hands the phase a
/// module.
///
/// `region` is source text, read by the fixed reader Musa already has: this
/// driver does not extend the lexer or the grouper.
#[cfg(test)]
pub(crate) fn expand_region(
    transformer: &str,
    region: &str,
    expansion: crate::syntax::ExpansionPath,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let subject = crate::syntax::read_region(&musa_language::parse(region).syntax(), expansion);
    expand_syntax(
        &format!("library {{\n    let level = \"readable\";\n\n    let expand = {transformer};\n}}\n"),
        PhaseImports::bundled(),
        &subject,
    )
    .0
}

/// The refusal an `Err(Both(node, message))` carries.
///
/// The span comes from the node the adapter handed back, never from anything
/// the adapter computed: `SourceInfo` has no eliminator, so an adapter can
/// point at a node it holds and cannot say where a node is.
fn refusal_of(held: &musa_core::Datum) -> Option<ExpansionFailure> {
    let musa_core::Datum::Case {
        ref constructor,
        ref fields,
    } = *held
    else {
        return None;
    };
    if &**constructor != "Pair.Both" {
        return None;
    }
    let [musa_core::Datum::Lit(ref node), ref message] = fields[..] else {
        return None;
    };
    let node = crate::registry::held::<crate::syntax::Syntax>(node)?;
    let at = match node.info() {
        crate::syntax::SourceInfo::Original { span, .. } => Some(*span),
        crate::syntax::SourceInfo::Generated(_) => None,
    };
    Some(ExpansionFailure::Refused {
        message: said(message)?,
        at,
    })
}

/// What one expansion charged the phases it used.
///
/// Two of `CompilerLimits`' four counters (`26-language-design-decision.md`
/// §3.5); the other two are the phase's own and are counted by
/// [`crate::expand`]. Read off [`musa_core::Spend`], because a transformer is
/// elaborated and normalized by the ordinary machinery and a separate accounting
/// of the same work would be a second opinion about it.
pub(crate) struct PhaseWork {
    pub(crate) evaluation_steps: u64,
}

impl PhaseWork {
    /// §3.5's checking counter, from what the core charged.
    ///
    /// `evaluation_steps` is the core's reduction count, which is the same
    /// quantity under the same name — and the whole of the checking half's
    /// cost: the flat dictionary law postpones no constraint and retries none,
    /// so there is no second quantity to carry. The phase's four counters are
    /// three, and §3.5's determinism-and-growth requirement is met by the one.
    ///
    /// The cost of reading the adapter *module* is in here too, added by
    /// [`AdapterModule::spend`] before the run. It has to be: an adapter whose
    /// operations carry their own signatures is checked without raising a
    /// single metavariable, so a phase that counted only the run's own
    /// elaboration would report zero for the checking half and charge nothing
    /// for a module of any size. The number is the core's own rather than an
    /// estimate on this side, which is what the paragraph above requires.
    const fn of(spent: musa_core::Spend) -> Self {
        Self {
            evaluation_steps: spent.steps,
        }
    }
}

/// Why an adapter's `edit` produced no patch.
///
/// The same three-way distinction [`ExpansionFailure`] makes, for the same
/// reasons, minus the two cases an editor cannot reach: an editor answers with
/// replacements rather than with syntax, so there is nothing for the expression
/// gate to reject. It is its own type rather than a reuse because the sentences
/// a reader gets differ — "this is not a transformer" is not what to tell
/// somebody whose `edit` is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditFailure {
    /// The adapter read the command and would not serve it.
    ///
    /// The package's own sentence, exactly as a refusal during expansion is.
    /// It carries no node: a command an adapter does not know is not about a
    /// place in the region, and pointing at one would be inventing a subject.
    Refused(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `edit` did not check as an editor.
    NotAnEditor(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// One replacement an adapter asked for: the anchor of the node to replace, and
/// the text to put there.
///
/// An anchor and not a range. An adapter has no operation for reading a source
/// range and gains none here — it names a node it was given, in the one
/// vocabulary it and an editor share (prompt 127dcc), and the compiler owns the
/// translation to bytes. That makes §4's locality a fact about the type rather
/// than a property the phase has to hope for and check afterwards.
pub(crate) type AdapterPatch = (u64, String);

/// Run one adapter's `edit` over one already-read region.
///
/// The second of `26-language-design-decision.md` §4's declared operations, in
/// the same phase environment [`expand_syntax`] runs in and by the same
/// machinery: one ordinary expression, checked by Algorithm W against a wanted
/// type, evaluated by the total evaluator, metered by the ordinary meter. The
/// command is spelled as its name, the anchor it is about, and one text
/// argument, because the phase is type-blind and may not learn a package's
/// command type.
pub(crate) fn edit_syntax(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
) -> (Result<Vec<AdapterPatch>, EditFailure>, PhaseWork) {
    let mut spent = musa_core::Spend::default();
    let answer = run_editor(adapter_source, imports, subject, command, anchor, argument, &mut spent);
    (answer, PhaseWork::of(spent))
}

fn run_editor(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    command: &str,
    anchor: u64,
    argument: &str,
    spent: &mut musa_core::Spend,
) -> Result<Vec<AdapterPatch>, EditFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => EditFailure::Stopped,
        ModuleFault::Broken(diagnostics) => EditFailure::NotAnEditor(diagnostics),
    })?;
    *spent = spent.and(module.spend());
    // `edit(region, command, anchor, argument)`, written as a term and read in
    // the module's own context. The anchor arrives as a number rather than
    // inside the argument text because a command spelled as one string would be
    // one an adapter could not take apart.
    let here = musa_core::Origin::UNKNOWN;
    let (answer, spend) = module
        .run(
            "edit",
            vec![
                region(subject),
                text(command),
                Raw::numeral(here, "Nat", anchor),
                text(argument),
            ],
        )
        .map_err(|unrun| match unrun {
            Unrun::Undeclared => EditFailure::NotAnEditor(vec![not_the_operation("edit")]),
            Unrun::Stopped => EditFailure::Stopped,
            Unrun::Refused(diagnostics) => EditFailure::NotAnEditor(diagnostics),
            Unrun::NoAnswer => EditFailure::NoAnswer,
        })?;
    *spent = spent.and(spend);
    patches(&answer).ok_or(EditFailure::NoAnswer)?
}

/// The `Result<List<Pair<Nat, Text>>, Text>` an editor answered.
///
/// [`None`] when the normal form is not one, which is this crate's defect rather
/// than an adapter's for [`answered`]'s reason: the call was checked before it
/// was normalized, so an editor cannot reach it by being wrong.
fn patches(answer: &musa_core::Datum) -> Option<Result<Vec<AdapterPatch>, EditFailure>> {
    let musa_core::Datum::Case {
        ref constructor,
        ref fields,
    } = *answer
    else {
        return None;
    };
    let [ref held] = fields[..] else { return None };
    match &**constructor {
        "Result.Ok" => Some(Ok(listed(held, patch)?)),
        "Result.Err" => Some(Err(EditFailure::Refused(said(held)?))),
        _ => None,
    }
}

/// One `Pair<Nat, Text>` an editor asked for.
fn patch(field: &musa_core::Datum) -> Option<AdapterPatch> {
    let musa_core::Datum::Case {
        ref constructor,
        ref fields,
    } = *field
    else {
        return None;
    };
    if &**constructor != "Pair.Both" {
        return None;
    }
    let [musa_core::Datum::Count { count, .. }, ref text] = fields[..] else {
        return None;
    };
    Some((count, said(text)?))
}

/// The elements of a `List`, each read by `each`.
///
/// The prelude spelling and not a builtin one: `01-surface.md`'s `List` is
/// `Empty`/`Cons`, so a closed list is that chain and reading it is walking it.
/// [`None`] the moment an element is not what `each` expects, which keeps the
/// whole answer one decision rather than a vector with a hole in it.
fn listed<T>(held: &musa_core::Datum, each: impl Fn(&musa_core::Datum) -> Option<T>) -> Option<Vec<T>> {
    let mut read = Vec::new();
    let mut rest = held;
    loop {
        let musa_core::Datum::Case {
            ref constructor,
            ref fields,
        } = *rest
        else {
            return None;
        };
        match (&**constructor, &fields[..]) {
            ("List.Empty", []) => return Some(read),
            ("List.Cons", [head, tail]) => {
                read.push(each(head)?);
                rest = tail;
            }
            _ => return None,
        }
    }
}

/// The text a literal holds.
fn said(held: &musa_core::Datum) -> Option<String> {
    let musa_core::Datum::Lit(ref value) = *held else {
        return None;
    };
    crate::registry::held::<String>(value).cloned()
}

/// One region, as the term the phase hands an operation.
fn region(subject: crate::syntax::Syntax) -> Raw {
    Raw::lit(
        musa_core::Origin::UNKNOWN,
        crate::registry::literal(crate::registry::syntax_type(crate::syntax::Cat::TokenTree), subject),
    )
}

/// One `Text` argument, the same way.
fn text(said: &str) -> Raw {
    Raw::lit(
        musa_core::Origin::UNKNOWN,
        crate::registry::literal(crate::registry::plain_type("Text"), said.to_owned()),
    )
}

/// Why an adapter's `print` produced no source text.
///
/// [`Self::Loss`] is `26-language-design-decision.md` §4's `PrintLoss`, and it
/// is the one of these that is not a fault at all: the printer was handed a
/// value carrying something it cannot write down, and said so. A printer that
/// quietly dropped that detail would make the round-trip law true by making the
/// value smaller, which is why the operation answers with a `Result` rather
/// than with text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PrintFailure {
    /// The adapter read the value and could not spell it — its own sentence.
    Loss(String),
    /// A compilation limit was crossed before the run finished.
    Stopped,
    /// The `print` did not check against the value it was handed.
    NotAPrinter(Vec<Diagnostic>),
    /// It checked and then did not answer.
    NoAnswer,
}

/// Run one adapter's `print` over one ordinary value.
///
/// The third of `26-language-design-decision.md` §4's declared operations, and
/// the only one that does **not** run in the phase environment: its input is an
/// ordinary evaluated value rather than syntax, so it is an ordinary total
/// package function and is read under the ordinary reading. A printer that
/// could reach [`SYNTAX_OWNERSHIP`] would be a second way to build syntax, from
/// a value, outside the one place expansion happens.
///
/// `value` is the value as an ordinary expression, because that is the one
/// spelling of a value this crate shares with anything outside it. `A` is what
/// the printer says it takes, and the application against the value is what
/// checks the two agree. A printer *can* say it now: its parameter type is the
/// package's, and the package is in scope where the printer is read. The phase
/// still never learns that type — it is named in the adapter's `print` and read
/// at the use site, never in the module the phase checks.
pub(crate) fn print_value(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    at: &crate::compile::SourceDocument,
    value: &str,
) -> Result<String, PrintFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => PrintFailure::Stopped,
        ModuleFault::Broken(diagnostics) => PrintFailure::NotAPrinter(diagnostics),
    })?;
    let Some(printer) = module.printer() else {
        return Err(PrintFailure::NotAPrinter(vec![not_the_operation("print")]));
    };
    run_printer(printer, adapter_source, imports, at, value)
}

/// The one small piece a printer and its subject are read in.
///
/// Ordinary compilation and nothing else: the same declarations, the same
/// elaborator, the same core, and the same readback that every other document in
/// this crate gets — so [`crate::prelude::phase`]'s vocabulary is out of scope
/// exactly as the empty scope had it out. A printer that could build syntax
/// would be a second way to make an expansion, out of a value, away from the one
/// place expansion happens.
///
/// "Ordinary compilation" is [`crate::document::elaborate`] because that is what
/// the words now mean. The piece below is a document like any other, and a
/// printer read by a second checker would be a printer the language does not
/// agree about: `Ok(text_join([…]))` would have to mean here what it means
/// everywhere, and two checkers is two chances for it not to.
///
/// What the piece holds, and why each part of it is there:
///
/// - **`at`'s ordinary imports, and the adapter's.** `Document(…)` is not a name
///   an empty world holds, so neither the subject nor a `match` over it could be
///   checked without `at`'s: `at` is the document the region will be written
///   into, which makes those the same scope the printed region will itself be
///   read in — the two sides of the round-trip law, read the same way. The
///   adapter's are here for the other half of the same claim: the printer's
///   reached declarations are checked *twice*, once in the phase and once here,
///   and a second reading that could not see the modules the first one saw would
///   be a second program rather than a second reading of one. A syntax import is
///   left out for the reason [`crate::imports::load`] leaves it out: it named a
///   reader, not a module. A line both documents write is written once, because
///   two identical imports in one piece is a collision the composer did not
///   make.
/// - **The module declarations the printer names**, so that a printer may have
///   local definitions. See [`Printer`].
/// - **`printer` as a `fn` declaration, then `printed`, with the value written
///   *at* the call.** A `fn` and not a `let` for the reason [`Printer`] gives:
///   §4 fixes a printer's answer at `Result<Text, Text>`, and writing it is what
///   turns the arms from inferred into checked. `A` is still settled by checking
///   the argument against the printer's own domain rather than by declaring it:
///   what a package's regions produce is the package's business, and a phase
///   that had to be told it would be a phase that knows a type. The value is not
///   bound to a name first, because a bidirectional
///   elaborator has nothing to check a bare `let subject = Alto;` against —
///   `Alto` is a constructor of some family and the binding says of which one
///   only if the type is written, which is the very thing this must not write.
///   In argument position the domain is already known, so the same text needs no
///   annotation at all.
fn run_printer(
    printer: &Printer,
    adapter_source: &str,
    imports: PhaseImports<'_>,
    at: &crate::compile::SourceDocument,
    value: &str,
) -> Result<String, PrintFailure> {
    let mut source = String::from("piece \"print\" {\n");
    let mut written = IndexSet::new();
    written.extend(ordinary_imports(at.text()));
    let mine = written.len();
    written.extend(ordinary_imports(adapter_source));
    for import in &written {
        source.push_str(import);
        source.push('\n');
    }
    for declaration in &printer.reached {
        source.push_str(declaration);
        source.push('\n');
    }
    source.push_str("fn printer");
    source.push_str(&printer.params);
    if !printer.says_answer {
        source.push_str(" -> Result<Text, Text>");
    }
    source.push(' ');
    source.push_str(&printer.answer);
    source.push_str("\nlet printed: Result<Text, Text> = printer(");
    source.push_str(value);
    source.push_str(");\n}\n");

    let parsed = musa_language::parse(&source);
    if let Some(error) = parsed.errors().first() {
        return Err(PrintFailure::NotAPrinter(vec![Diagnostic::error(
            Code::Expansion,
            format!("the printer and the value do not parse together: {}", error.message()),
        )]));
    }
    let root = parsed.syntax();
    let Some(piece) = musa_language::ast::PieceDecl::from_root(&root) else {
        return Err(PrintFailure::NotAPrinter(vec![not_the_operation("print")]));
    };
    let mut resolver = Resolver::new();
    // The statements come back in the order they were written above, so the
    // split is where `mine` said it was.
    let statements = musa_language::ast::ImportStmt::all_at_root(piece.syntax());
    let (here, there) = statements.split_at(mine.min(statements.len()));
    let composers = crate::imports::load(&mut resolver, at.name(), here, imports.sources);
    let adapters = crate::imports::load(&mut resolver, imports.document, there, imports.sources);
    // One library per document, however many closures reached it: a piece that
    // reads both a composer's `std::list` and an adapter's `std::list` reaches
    // the same module twice, and declaring it twice would be a collision
    // neither of them wrote.
    let mut seen = IndexSet::new();
    let mut sources = Vec::new();
    for (importer, libraries) in [(at.name(), &composers), (imports.document, &adapters)] {
        for (from, library) in libraries.each() {
            let document = crate::imports::resolve_import(importer, from.path);
            if seen.insert((document, from.qualifier.map(str::to_owned))) {
                sources.push(crate::document::Source::imported(library.syntax(), from));
            }
        }
    }
    sources.push(crate::document::Source::own(&root));
    sources.push(crate::document::Source::own(piece.syntax()));
    let Some(document) = crate::document::elaborate(&mut resolver, &sources, None) else {
        return Err(print_failure(resolver.diagnostics));
    };
    let printed = match document.value("printed") {
        Ok((printed, _)) => printed,
        Err(musa_core::ElabError::Exhausted(_)) => return Err(PrintFailure::Stopped),
        Err(error) => {
            return Err(PrintFailure::NotAPrinter(vec![crate::lower::refusals::restate(
                document.sites(),
                &error,
            )]));
        }
    };
    answered(&printed).ok_or(PrintFailure::NoAnswer)?
}

/// The `Result<Text, Text>` a printer answered, as this module's own outcome.
///
/// [`None`] when the normal form is not one, which is a defect in this crate
/// rather than in an adapter: the term was checked at `Result<Text, Text>` above
/// before it was normalized, so a printer cannot reach it by being wrong.
///
/// [`PrintFailure::Loss`] is what the error arm *means* — the printer read the
/// value and said which part of it it could not spell — so the two arms are not
/// success and failure here. They are the two answers §4 declares.
fn answered(printed: &musa_core::Term) -> Option<Result<String, PrintFailure>> {
    let musa_core::Datum::Case {
        ref constructor,
        ref fields,
    } = musa_core::canonical(printed)?
    else {
        return None;
    };
    let [musa_core::Datum::Lit(ref said)] = fields[..] else {
        return None;
    };
    let said = crate::registry::held::<String>(said)?.clone();
    match &**constructor {
        "Result.Ok" => Some(Ok(said)),
        "Result.Err" => Some(Err(PrintFailure::Loss(said))),
        _ => None,
    }
}

/// The import statements of a document that bring a module's declarations in,
/// written back out verbatim.
///
/// Verbatim because an import means what it says: re-spelling `import std::x as
/// y;` from its parts would be this function deciding what the composer wrote.
fn ordinary_imports(text: &str) -> Vec<String> {
    let parsed = musa_language::parse(text);
    let root = parsed.syntax();
    root.descendants()
        .filter_map(musa_language::ast::ImportStmt::cast)
        .collect::<Vec<_>>()
        .iter()
        .filter(|import| !import.changes_syntax())
        .map(|import| import.syntax().text().to_string().trim().to_owned())
        .collect()
}

/// A stop when a limit was crossed, and the checker's complaints when not.
///
/// Read off the diagnostics for [`module_fault`]'s reason, and separate from it
/// because the two sentences differ: a printer that does not check is not an
/// adapter module that does not check.
fn print_failure(diagnostics: Vec<Diagnostic>) -> PrintFailure {
    if diagnostics.iter().any(|it| it.code == Code::ResourceLimit) {
        PrintFailure::Stopped
    } else {
        PrintFailure::NotAPrinter(diagnostics)
    }
}

/// The text an ordinary expression evaluates to, for a law that compares two
/// values rather than two spellings of one.
///
/// The round-trip law of §4 is about *values*, and the fixture that carries it
/// states its equality as text equality — which is the fixture's own equality
/// function, not a structural comparison of the printed source, because
/// printing is allowed to normalize.
#[cfg(test)]
pub(crate) fn evaluate_text(expression: &str) -> Option<String> {
    let parsed = musa_language::parse(&format!("library {{\n  let it: Text = {expression};\n}}"));
    if !parsed.errors().is_empty() {
        return None;
    }
    let library = musa_language::ast::LibraryDecl::from_root(&parsed.syntax())?;
    let mut resolver = Resolver::new();
    let document = crate::document::elaborate(&mut resolver, &[crate::document::Source::own(library.syntax())], None)?;
    let (normal, _ty) = document
        .term(&musa_core::Raw::var(musa_core::Origin::UNKNOWN, "it"))
        .ok()?;
    crate::registry::read_back::<String>(&normal).ok().cloned()
}

/// Run one transformer over one already-read region, in the phase environment.
///
/// The work is reported whichever way the run came out, because what a run
/// cost does not depend on what it answered: an adapter that reads a whole
/// region and then refuses it has read a whole region, and a phase that
/// charged nothing for that would let a file buy unbounded reading by
/// arranging to be refused.
pub(crate) fn expand_syntax(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: &crate::syntax::Syntax,
) -> (Result<crate::syntax::Syntax, ExpansionFailure>, PhaseWork) {
    // Built here and lent inwards rather than made on the other side of
    // `with_room`: what a run charged is reported by the caller that asked for
    // it, and a total kept on a scoped thread would go out of scope with it.
    let mut spent = musa_core::Spend::default();
    let answer = with_room(adapter_source, imports, subject, &mut spent);
    (answer, PhaseWork::of(spent))
}

/// Run one transformer with enough stack for the nesting the budget allows.
///
/// The budget is the guard: `Budget::LANGUAGE`'s nesting limit is what turns a
/// region too deep to read into a refusal that names an operation and a place
/// (`../rules/language/02-core-calculus.md` §4). This is what makes that
/// refusal *reachable*. A limit no host can afford to run up to is a limit the
/// process dies before hitting, and not dying is the whole exercise. Neither
/// half stands in for the other: room alone only moves the cliff, and a limit
/// alone only promises a diagnostic the machine may not live to print.
///
/// The room is `NESTING × FRAME_CEILING`, derived rather than picked, so
/// raising the published limit cannot quietly outrun the stack that honours it.
///
/// A host with no threads — the wasm shell — takes the second path and runs on
/// the stack it was linked with, where the room is a link-time setting instead.
/// The budget does not move, so both hosts accept and refuse exactly the same
/// programs; they differ only in what they survive.
fn with_room(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: &crate::syntax::Syntax,
    spent: &mut musa_core::Spend,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let room = usize::try_from(crate::core_budget::NESTING.saturating_mul(crate::core_budget::FRAME_CEILING))
        .unwrap_or(usize::MAX);
    let mut answer = None;
    if !cfg!(target_family = "wasm") {
        std::thread::scope(|scope| {
            let run = || answer = Some(run_transformer(adapter_source, imports, subject.clone(), spent));
            if let Ok(running) = std::thread::Builder::new().stack_size(room).spawn_scoped(scope, run)
                && let Err(panic) = running.join()
            {
                // A panic inside is a compiler fault. Resuming it on this side
                // keeps it looking like one, rather than like a phase that
                // quietly answered nothing.
                std::panic::resume_unwind(panic);
            }
        });
    }
    answer.unwrap_or_else(|| run_transformer(adapter_source, imports, subject.clone(), spent))
}

fn run_transformer(
    adapter_source: &str,
    imports: PhaseImports<'_>,
    subject: crate::syntax::Syntax,
    spent: &mut musa_core::Spend,
) -> Result<crate::syntax::Syntax, ExpansionFailure> {
    let module = read_adapter_module(adapter_source, imports).map_err(|fault| match fault {
        ModuleFault::Stopped => ExpansionFailure::Stopped,
        ModuleFault::Broken(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
    })?;
    *spent = spent.and(module.spend());
    let (answer, spend) = module
        .run("expand", vec![region(subject)])
        .map_err(|unrun| match unrun {
            Unrun::Undeclared => ExpansionFailure::NotATransformer(vec![not_the_operation("expand")]),
            Unrun::Stopped => ExpansionFailure::Stopped,
            Unrun::Refused(diagnostics) => ExpansionFailure::NotATransformer(diagnostics),
            Unrun::NoAnswer => ExpansionFailure::NoAnswer,
        })?;
    *spent = spent.and(spend);
    let produced = expanded(&answer).ok_or(ExpansionFailure::NoAnswer)??;
    // The gate again, here rather than only in `checked_expression`: a
    // transformer that never called the builtin has still produced output the
    // rest of the compiler will have to anchor diagnostics against.
    crate::syntax::check_expression(&produced).map_err(ExpansionFailure::NotAnExpression)?;
    Ok(produced)
}

/// The `Result<Syntax, Pair<Syntax, Text>>` a transformer answered.
///
/// [`None`] for [`answered`]'s reason, and the error arm is not a fault for
/// [`ExpansionFailure::Refused`]'s: a transformer that answers `Err` has
/// *worked*, and what it says is the adapter package's sentence about the
/// composer's text.
fn expanded(answer: &musa_core::Datum) -> Option<Result<crate::syntax::Syntax, ExpansionFailure>> {
    let musa_core::Datum::Case {
        ref constructor,
        ref fields,
    } = *answer
    else {
        return None;
    };
    let [ref held] = fields[..] else { return None };
    match &**constructor {
        "Result.Ok" => {
            let musa_core::Datum::Lit(ref produced) = *held else {
                return None;
            };
            Some(Ok(crate::registry::held::<crate::syntax::Syntax>(produced)?.clone()))
        }
        "Result.Err" => Some(Err(refusal_of(held)?)),
        _ => None,
    }
}

/// The complaint for a module that declares nothing under a name the phase
/// runs.
///
/// One case where there were two. A module whose `expand` is not a transformer
/// used to be told the type the phase found instead; now the *call* is what
/// checks it, so that author is told which argument did not fit and where, by
/// the conversion check itself. What is left here is the case there is no call
/// to make.
fn not_the_operation(name: &str) -> Diagnostic {
    Diagnostic::error(Code::Expansion, format!("it declares no `{name}`")).help(match name {
        "expand" => {
            "an adapter declares `let expand = fn (region) { … };`, answering `Ok(syntax)` or `Err((node, why))`"
        }
        "edit" => "an adapter declares `let edit = fn (region, command, anchor, argument) { … };`",
        _ => "an adapter declares `let print = fn (value) { … };`, answering `Ok(text)` or `Err(loss)`",
    })
}
