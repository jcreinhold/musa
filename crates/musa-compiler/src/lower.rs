//! The surface CST read as a raw core term.
//!
//! `docs/rules/language/02-core-calculus.md` §2 elaborates a surface term into a
//! core term, and [`musa_core::Raw`] is what it reads. This module is the half
//! that knows about `.musa`: it walks the lossless CST that
//! [`musa_language`](musa_language) produced and writes the raw term that
//! `musa-core` checks. Nothing here decides a type — that is the core's, and the
//! whole reason this module is a fraction of the size of the checker it replaces.
//!
//! # What "lowering" is, and what it is not
//!
//! Three jobs, and each is one the core cannot do because it would need the
//! grammar to do it:
//!
//! - **Reading.** A `TypeName` node holds text; `Duration<WrittenTime>` is an
//!   application of a registered base type to a literal coordinate. Turning one
//!   into the other is reading.
//! - **Desugaring.** `if c { a } else { b }` is `match c { Bool.True → a ; …}`,
//!   `x == y` is `Eq.equal x y` (`10-traits.md` §5), and `x.m(y)` is
//!   [`RawShape::Method`](musa_core::RawShape::Method) applied. The core has no
//!   operator table and never learns one.
//! - **Numbering.** §7 asks that every core term record the surface node it came
//!   from, and `musa-core` is a leaf that cannot know what a span is. [`Sites`]
//!   is the table that makes a [`musa_core::Origin`] mean something again when a
//!   refusal comes back.
//!
//! What is *not* here: name resolution to an index, unification, implicit
//! insertion, coverage, positivity, termination. Those are the core's, and a
//! second implementation of any of them beside it is the second path
//! `02-core-calculus.md` §5's audit exists to catch.
//!
//! # The one direction
//!
//! Lowering never asks what type a position wants. That is not an economy, it is
//! the boundary: a function that took an expected type would be *checking*, and
//! checking is exactly what moved to `musa-core`. The visible consequences are
//! two, and both are deliberate:
//!
//! - An unknown name is **written through** as a variable. The core holds the
//!   context, so the core answers
//!   [`Refusal::UnknownName`](musa_core::Refusal::UnknownName), at the origin
//!   this module gave the node.
//! - A literal's base type comes from **the surface node**, never from a type
//!   flowing in. `02-core-calculus.md` §2 says a literal infers, so `3` is a
//!   `Nat` wherever it is written and `chord c# minor` supplies `NoteName`
//!   because a `ChordExpr` is what it is. Where the old checker read an
//!   `Integer` as a `Duration` because a `Duration` was expected, the reading is
//!   now a `Nat` and the conversion is written in the source — prompt 142's
//!   migration owns that change.

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "every dispatch here is over `SyntaxKind`'s three hundred variants, of which the forms this module reads are a named minority; `is_expr_node` and `is_type_node` are those lists, and writing either out a second time per match would hide the reading rather than check it"
)]

#[cfg(test)]
mod laws;

pub(crate) mod items;
pub(crate) mod notation;
pub(crate) mod piece;
mod quotes;
pub(crate) mod refusals;
mod types;
mod values;

use musa_core::{Origin, Raw};
use musa_language::{SyntaxKind, SyntaxNode, SyntaxToken};

use crate::diagnose::Diagnostic;
use crate::origin::SourceSpan;
use crate::resolve::Resolver;

/// Every surface node a raw term was read from, numbered.
///
/// `musa-core` carries a [`Origin`] on every term and says in its own module
/// documentation that the number is "the caller's": the core keeps it attached
/// to the right term and never interprets it. This is the caller's half — the
/// table that turns it back into a span when a [`musa_core::ElabError`] arrives
/// and has to be pointed at something a composer wrote.
///
/// # Why a table rather than a span in the term
///
/// An [`Origin`] is one word and `Copy`, which is what lets every term hold one
/// without provenance becoming a thing worth switching off. A `SourceSpan` is
/// two words and would have to cross into `musa-core`, which is a leaf that must
/// not learn what a file is. So the span stays here and the core carries an
/// index into here, which is the same arrangement
/// [`crate::derivation`](crate::derivation) already uses for the same reason.
#[derive(Debug, Default)]
pub(crate) struct Sites {
    spans: Vec<SourceSpan>,
}

impl Sites {
    /// The origin naming `span`, adding it if it is new to this table.
    ///
    /// Not deduplicated: two nodes with the same span are two nodes, and a table
    /// that merged them would report the second one's failure at the first one's
    /// place. The span is small and the table is per-pass.
    pub(crate) fn at(&mut self, span: SourceSpan) -> Origin {
        let index = u32::try_from(self.spans.len()).unwrap_or(u32::MAX);
        self.spans.push(span);
        Origin::node(index)
    }

    /// The origin naming `node`'s written span.
    pub(crate) fn node(&mut self, node: &SyntaxNode) -> Origin {
        self.at(crate::resolve::trimmed_span(node))
    }

    /// Where the term carrying `origin` was written, when this table numbered it.
    ///
    /// [`None`] for [`Origin::UNKNOWN`] and for a number this table did not
    /// hand out — the core mints neither, but a registered builtin's signature
    /// carries `UNKNOWN` by construction, so a refusal *about a signature* has
    /// nowhere of its own to point and says so rather than pointing at node one.
    pub(crate) fn span(&self, origin: Origin) -> Option<SourceSpan> {
        origin
            .node_number()
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.spans.get(index))
            .copied()
    }
}

/// One walk of a declaration's surface, and everything it needs to write raw
/// terms for it.
///
/// # Why a struct rather than four parameters
///
/// The three fields are threaded through every function below and none is ever
/// passed without the others: a diagnostic goes to the resolver, a node's origin
/// comes from the table, and both are needed wherever a form is read. What is
/// *not* here is as deliberate:
///
/// - **No scope.** A local binder, a type parameter, and a library name are all
///   written through as [`Raw::var`] for the core to resolve, so there is
///   nothing for this to remember. The compiler's own vocabulary — `Pitch`,
///   `Nat`, the phase's types — is a fixed table asked before anything else,
///   which is why a `data` declaration cannot quietly become a second reading of
///   `Pitch` and why a type parameter cannot shadow one either.
/// - **No expected type.** See the module documentation: a lowering that took
///   one would be a second checker.
///
/// [`Lowering::in_phase`] is the one thing a walk carries that is not derivable
/// from the node, and it is a property of the *declaration* rather than of any
/// expression inside it: `02-core-calculus.md` §5.9 keeps the phase's types and
/// constants in a scope ordinary source cannot name, so whether `Syntax<Expr>`
/// is a type at all depends on which of the two is being read.
pub(crate) struct Lowering<'a> {
    resolver: &'a mut Resolver,
    sites: &'a mut Sites,
    /// Whether the phase's own vocabulary is readable here (§5.9).
    in_phase: bool,
    /// The `?`s written since the last position that delimits an answer, in the
    /// order they were written. See [`Lowering::expr`].
    questions: Vec<Question>,
    /// How many unspellable binders this walk has minted.
    minted: u32,
    /// The claims written over passages in this walk, in source order.
    ///
    /// Accumulated here rather than answered by [`Lowering::notated`] because a
    /// claim is not part of the music: the fold answers a track, and an
    /// assertion that changed its answer would be the thing
    /// `examples/theory-assertions.musa` promises it is not. See
    /// [`notation::Claimed`] for why each one holds two terms.
    claims: Vec<notation::Claimed>,
}

/// One `?`, waiting for the answer it was written inside of.
struct Question {
    /// The subject, lowered once. It becomes the scrutinee, which is what makes
    /// the surface's promise that `e?` evaluates `e` exactly once true by
    /// construction rather than by care.
    subject: Raw,
    /// The unspellable name standing where the `?` was written.
    binder: String,
    /// The `?` itself, so a refusal points at the question that propagated a
    /// failure rather than at a match nobody wrote.
    origin: Origin,
}

impl<'a> Lowering<'a> {
    /// A walk of ordinary source.
    pub(crate) fn new(resolver: &'a mut Resolver, sites: &'a mut Sites) -> Self {
        Self {
            resolver,
            sites,
            in_phase: false,
            questions: Vec::new(),
            minted: 0,
            claims: Vec::new(),
        }
    }

    /// The claims written over passages in this walk, taken away from it.
    ///
    /// Drained rather than borrowed because a walk reads one declaration and a
    /// claim outlives it: the piece decides where its barlines fall once every
    /// voice has been read, so what is recorded here is carried to that point
    /// and proved there.
    pub(crate) fn claimed(&mut self) -> Vec<notation::Claimed> {
        std::mem::take(&mut self.claims)
    }

    /// A walk of an adapter phase, where §5.9's vocabulary is readable.
    pub(crate) fn phase(resolver: &'a mut Resolver, sites: &'a mut Sites) -> Self {
        Self {
            in_phase: true,
            ..Self::new(resolver, sites)
        }
    }

    /// The origin numbering `node`.
    fn origin(&mut self, node: &SyntaxNode) -> Origin {
        self.sites.node(node)
    }

    /// Report `complaint` and answer [`None`], which is what every refusal here
    /// does.
    fn refuse<T>(&mut self, complaint: Diagnostic) -> Option<T> {
        self.resolver.report(complaint);
        None
    }

    /// Run `read`, and answer [`None`] if it complained.
    ///
    /// [`crate::resolve`]'s readings report on the resolver and answer whatever
    /// they could still make of the source; everything here answers [`Option`]
    /// and lets the caller decide what a refusal costs. This is the adapter
    /// between the two, and it exists so a reading the replaced path already
    /// has — [`crate::resolve::part_facts`] is the first — can be *called* from
    /// here rather than written a second time.
    fn heard<T>(&mut self, read: impl FnOnce(&mut Resolver) -> T) -> Option<T> {
        let before = self.resolver.diagnostics.len();
        let answer = read(self.resolver);
        (self.resolver.diagnostics.len() == before).then_some(answer)
    }

    /// A binder no source file can write.
    ///
    /// `?` is not an identifier start in this grammar, so a name beginning with
    /// one cannot collide with anything a composer wrote — which is what lets a
    /// desugaring bind a value without shadowing.
    fn mint(&mut self, hint: &str) -> String {
        self.minted = self.minted.saturating_add(1);
        format!("?{hint}{}", self.minted)
    }
}

/// `head a₁ … aₙ`, left-associated, which is what the core's one-argument
/// application means for a written argument list.
fn applied(origin: Origin, head: Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    arguments
        .into_iter()
        .fold(head, |function, argument| Raw::app(origin, function, argument))
}

/// `[m₁, …, mₙ]`, as the prelude's `List` spells one.
///
/// Beside [`applied`] rather than inside one reading, because three of them
/// build a list from members they already hold: a written `[…]`, a quote's
/// splices, and the run one hole of a quote pattern stands for. One fold, so a
/// list a lowering writes has one shape.
fn listed(origin: Origin, members: Vec<Raw>) -> Raw {
    let mut built = Raw::var(origin, "List.Empty");
    for member in members.into_iter().rev() {
        built = applied(origin, Raw::var(origin, "List.Cons"), [member, built]);
    }
    built
}

/// `n`, counted up from `Nat.Zero`.
///
/// `Nat` is a declared family rather than a base type (`02-core-calculus.md`
/// §5.8), so a written number is a unary tower and not a literal. Small by
/// construction wherever this is called: a scalar the source wrote, or a hole's
/// index in a quote pattern.
fn whole(origin: Origin, value: u64) -> Raw {
    let mut built = Raw::var(origin, "Nat.Zero");
    for _ in 0..value {
        built = Raw::app(origin, Raw::var(origin, "Nat.Succ"), built);
    }
    built
}

/// The tokens of `node` that carry meaning, in order.
fn significant_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.descendants_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
}

/// The tokens `node` itself holds, without descending into its children.
///
/// The difference from [`significant_tokens`] is what keeps a form's own words
/// apart from its parts': `p step down n` writes `down` in the `StepExpr`, and a
/// `down` inside the expression `n` would otherwise answer for it.
fn own_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.children_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
}

/// Whether `node` writes one of the form's own words.
fn writes(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    own_tokens(node).any(|token| token.kind() == kind)
}

/// The first child of `node` whose kind `wanted` admits.
fn child(node: &SyntaxNode, wanted: impl Fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
    node.children().find(|child| wanted(child.kind()))
}

/// Every child of `node` whose kind `wanted` admits, in order.
fn children(node: &SyntaxNode, wanted: impl Fn(SyntaxKind) -> bool + Copy) -> Vec<SyntaxNode> {
    node.children().filter(|child| wanted(child.kind())).collect()
}

/// Whether a node kind is one of the written expression forms.
///
/// The list `crate::core`'s own `is_expr_node` holds, plus the five forms
/// prompts 136 and 137 added to the grammar and the old checker never learned to
/// read. That the two lists differ is the shape of what this module is for.
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
            | SyntaxKind::RecordLiteralExpr
            | SyntaxKind::QuestionExpr
            | SyntaxKind::QuoteExpr
            | SyntaxKind::BinaryExpr
            | SyntaxKind::IndexExpr
            | SyntaxKind::PathExpr
            | SyntaxKind::Splice
            | SyntaxKind::SequenceSplice
            | SyntaxKind::MusicExpr
            | SyntaxKind::KernelQuote
    )
}

/// Whether a node kind is one of the written type forms.
fn is_type_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::TypeExpr
            | SyntaxKind::TypeName
            | SyntaxKind::AppliedType
            | SyntaxKind::OptionType
            | SyntaxKind::ListType
            | SyntaxKind::ResultType
            | SyntaxKind::FunctionType
            | SyntaxKind::ProductType
    )
}
