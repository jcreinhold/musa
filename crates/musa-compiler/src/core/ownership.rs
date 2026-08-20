//! One concern of the enclosing module; see its module docs.

use crate::infer::Minter;

use super::Builtin;
use super::Family;
use super::Type;
use super::{
    BOOL, CHORD, CLASS, DEGREE, DURATION, FRAME, INTERVAL, INTERVALS, KEY, MAYBE_CHORD, MAYBE_CLASS, MAYBE_DEGREE,
    MAYBE_FRAME, MAYBE_NAT, MAYBE_ROMAN, MAYBE_TEXT, MAYBE_TRIAD, MAYBE_VOICING, NAT, NATS, PC12, PC12S, PCSET12,
    PITCH, PITCHES, POSITION, RATIO, ROMAN, ROW12, ROW12_OR_FAULT, ROW12S, SCALE, TEXT, TEXTS, TRIAD, VOICING, delta,
};

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
