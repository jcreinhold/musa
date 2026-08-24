//! One concern of the enclosing module; see its module docs.

use super::Eliminator;
use super::MachineOp;

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
    /// A finite syntax value ([`crate::quote::Syntax`]), by how it parses.
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
    Syntax(crate::quote::Cat),
    /// How the lexer classified one token ([`musa_syntax::SyntaxKind`]).
    ///
    /// Phase-local, and *generated*: its values are the lexer's own kinds, so
    /// there is no second table for a new token kind to be missing from. It
    /// replaces the `Text` that `syntax_token` took and every
    /// `text_equal(kind, "…")` an adapter wrote against it.
    TokenKind,
    /// How one group is delimited ([`crate::quote::Delimiter`]).
    ///
    /// Phase-local, four values, and what `syntax_group` used to claim to hide
    /// as "the fixed grouper's delimiter set".
    Delimiter,
    /// Where one node sits ([`crate::quote::NodePath`]).
    NodePath,
    /// Which name a binder declares ([`crate::quote::BindingPath`]).
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
    PitchBetween,
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
    Pc12Forget,
    Pc12Spelled,
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
            | Self::Voicing => true,
        }
    }
}

/// Whether every shape in a signature position is storable data.
///
/// Written as a recursion over the slice rather than a loop over indices
/// because it runs in a `const` context, where the registration is checked.
pub(crate) const fn all_storable(shapes: &[Shape]) -> bool {
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

pub(crate) const BOOL: Shape = Shape::Base(Base::Bool);
pub(crate) const NAT: Shape = Shape::Base(Base::Nat);

pub(crate) const RATIO: Shape = Shape::Base(Base::Ratio);
pub(crate) const TEXT: Shape = Shape::Base(Base::Text);

/// Every time operation is registered at written time, because written time is
/// the only coordinate the source language constructs a value of
/// (`02-core-calculus.md` §5.7 fixes it as the score side's). `PhysicalTime`
/// exists in [`Coordinate`] so that the day a physical duration reaches the
/// source it arrives as a *different type* rather than as the same one with a
/// different meaning; registering operations for it before anything can make
/// one would be names nothing could call.
pub(crate) const DURATION: Shape = Shape::Base(Base::Duration(Coordinate::WrittenTime));
pub(crate) const POSITION: Shape = Shape::Base(Base::Position(Coordinate::WrittenTime));

pub(crate) const PITCH: Shape = Shape::Base(Base::Pitch);
pub(crate) const CLASS: Shape = Shape::Base(Base::PitchClass);

pub(crate) const INTERVAL: Shape = Shape::Base(Base::Interval);
pub(crate) const KEY: Shape = Shape::Base(Base::Key);

pub(crate) const SCALE: Shape = Shape::Base(Base::Scale);
pub(crate) const DEGREE: Shape = Shape::Base(Base::Degree);

pub(crate) const FRAME: Shape = Shape::Base(Base::Frame);
pub(crate) const CHORD: Shape = Shape::Base(Base::ChordClass);

pub(crate) const TRIAD: Shape = Shape::Base(Base::Triad);
pub(crate) const ROMAN: Shape = Shape::Base(Base::Roman);

pub(crate) const VOICING: Shape = Shape::Base(Base::Voicing);
pub(crate) const TEXTS: Shape = Shape::List(&TEXT);
pub(crate) const MAYBE_TEXT: Shape = Shape::Option(&TEXT);

pub(crate) const PITCHES: Shape = Shape::List(&PITCH);

pub(crate) const INTERVALS: Shape = Shape::List(&INTERVAL);
pub(crate) const MAYBE_NAT: Shape = Shape::Option(&NAT);

pub(crate) const MAYBE_CLASS: Shape = Shape::Option(&CLASS);
pub(crate) const MAYBE_DEGREE: Shape = Shape::Option(&DEGREE);

pub(crate) const MAYBE_FRAME: Shape = Shape::Option(&FRAME);
pub(crate) const MAYBE_CHORD: Shape = Shape::Option(&CHORD);

pub(crate) const MAYBE_TRIAD: Shape = Shape::Option(&TRIAD);
pub(crate) const MAYBE_ROMAN: Shape = Shape::Option(&ROMAN);

pub(crate) const MAYBE_VOICING: Shape = Shape::Option(&VOICING);

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
pub(crate) const fn delta(arguments: &'static [Shape], result: Shape) -> Family {
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
