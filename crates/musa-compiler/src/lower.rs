//! The surface CST read as a raw core term.
//!
//! `docs/rules/language/02-core-calculus.md` §2 elaborates a surface term into a
//! core term, and [`musa_calculus::Raw`] is what it reads. This module is the half
//! that knows about `.musa`: it walks the lossless CST that
//! [`musa_syntax`](musa_syntax) produced and writes the raw term that
//! `musa-calculus` checks. Nothing here decides a type — that is the core's, and the
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
//!   [`RawShape::Method`](musa_calculus::RawShape::Method) applied. The core has no
//!   operator table and never learns one.
//! - **Numbering.** §7 asks that every core term record the surface node it came
//!   from, and `musa-calculus` is a leaf that cannot know what a span is. [`Sites`]
//!   is the table that makes a [`musa_calculus::Origin`] mean something again when a
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
//! checking is exactly what moved to `musa-calculus`. The visible consequences are
//! two, and both are deliberate:
//!
//! - An unknown name is **written through** as a variable. The core holds the
//!   context, so the core answers
//!   [`Refusal::UnknownName`](musa_calculus::Refusal::UnknownName), at the origin
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

pub(crate) mod documented;
pub(crate) mod events;
pub(crate) mod items;
pub(crate) mod notation;
pub(crate) mod piece;
mod quotes;
pub(crate) mod refusals;
mod types;
mod values;

use std::collections::HashMap;

use num_rational::Ratio;

use musa_calculus::{ModuleId, Origin, Raw};
use musa_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use crate::resolve::Resolver;
use musa_score::diagnose::Diagnostic;
use musa_score::origin::SourceSpan;

/// Every surface node a raw term was read from, numbered.
///
/// `musa-calculus` carries a [`Origin`] on every term and says in its own module
/// documentation that the number is "the caller's": the core keeps it attached
/// to the right term and never interprets it. This is the caller's half — the
/// table that turns it back into a span when a [`musa_calculus::ElabError`] arrives
/// and has to be pointed at something a composer wrote.
///
/// # Why a table rather than a span in the term
///
/// An [`Origin`] is one word and `Copy`, which is what lets every term hold one
/// without provenance becoming a thing worth switching off. A `SourceSpan` is
/// two words and would have to cross into `musa-calculus`, which is a leaf that must
/// not learn what a file is. So the span stays here and the core carries an
/// index into here, which is the same arrangement
/// [`musa_score::derivation`](musa_score::derivation) already uses for the same reason.
#[derive(Debug, Default)]
pub(crate) struct Sites {
    spans: Vec<SourceSpan>,
    brought: Vec<Brought>,
    /// Each quote-pattern arm this document lowered: the origin of the arm's
    /// body, its span, and the identifiers the pattern matches *literally*.
    ///
    /// Recorded so [`lower::refusals::restate`] can answer the one confusing
    /// case of §4's "a quote pattern binds only its splices": an arm whose
    /// body uses a name the pattern wrote as a literal. The elaborator's
    /// `UnknownName` is true there and says nothing about the `$` that was
    /// left off, so the restatement asks this table whether the name was a
    /// pattern literal at that place and upgrades the report when it was.
    /// Recording rather than reporting at the lowering is what keeps the
    /// upgrade free of false positives: it fires only on a name the
    /// elaborator really could not resolve. The origin rides along because
    /// spans from two files are both byte ranges — containment means the body
    /// and the use are in the *same* file, which the site numbers know and
    /// the spans alone do not.
    quoted_arms: Vec<(Origin, SourceSpan, Vec<String>)>,
    /// Which file each [`ModuleId`] this document minted stands for.
    ///
    /// [`crate::document::elaborate`] numbers one module per file it reads from
    /// (`01-surface.md` §1.3: a Musa module is a file), and the core carries
    /// only the number — `musa-calculus` is a leaf that must not learn what a
    /// file is, exactly as it must not learn what a span is. So this is the
    /// other half of the same arrangement the span table already is, and it is
    /// what lets [`refusals::restate`] tell a reader *where* a private name
    /// lives rather than only that it is out of reach.
    ///
    /// A short list scanned linearly, because a document reads a handful of
    /// files and a map would be a data structure for a lookup that happens once
    /// per refusal.
    modules: Vec<(ModuleId, String)>,
    /// How many blocks of notation this document has entered.
    ///
    /// Beside the span table because it is the other thing a document numbers
    /// once and every walk of it shares: a [`Lowering`] lives for one
    /// declaration, and a [`musa_score::origin::DeclarationId`] has to be unique
    /// across all of them. See [`Sites::declaring`].
    declarations: u32,
}

/// A run of sites read out of an imported file, and the statement that brought
/// it in.
///
/// A *run* rather than a mark on every site, because a document is gathered one
/// source at a time: every site an import contributes is numbered before the
/// next source's first, so the whole fact is two numbers. A piece with no
/// imports carries none of these, and one with a dozen carries a dozen — the
/// scan in [`Sites::brought`] is over files, not over sites.
#[derive(Debug)]
struct Brought {
    /// The file, by the key the import resolved to. [`musa_score::diagnose::Cause`]
    /// names a document by this, which is how a consumer finds the text a
    /// foreign span is a span in.
    path: String,
    /// The `import` statement, in *this* document — the one span about that file
    /// a reader can be pointed at.
    at: SourceSpan,
    /// The half-open run of site numbers this file contributed.
    from: usize,
    upto: usize,
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

    /// Record a quote-pattern arm: its body's origin and span, and the
    /// identifiers its pattern matches literally. See the field for the why.
    pub(crate) fn quote_arm(&mut self, body: Origin, span: SourceSpan, literals: Vec<String>) {
        if !literals.is_empty() {
            self.quoted_arms.push((body, span, literals));
        }
    }

    /// Whether `at` — an unresolved name's origin — sits in the body of a
    /// quote-pattern arm that wrote `name` literally, in the same file.
    pub(crate) fn quoted_literal(&self, at: Origin, name: &str) -> bool {
        let Some(use_span) = self.span(at) else {
            return false;
        };
        self.quoted_arms.iter().any(|(body, body_span, literals)| {
            self.same_file(*body, at)
                && body_span.start <= use_span.start
                && use_span.end <= body_span.end
                && literals.iter().any(|literal| literal == name)
        })
    }

    /// Whether two origins were numbered out of the same file: both from the
    /// home document, or both from one import's run.
    fn same_file(&self, one: Origin, other: Origin) -> bool {
        let run_of = |origin: Origin| {
            let index = usize::try_from(origin.node_number()?).ok()?;
            self.brought
                .iter()
                .position(|run| run.from <= index && index < run.upto)
        };
        match (one.node_number(), other.node_number()) {
            (Some(_), Some(_)) => run_of(one) == run_of(other),
            _ => false,
        }
    }

    /// How many sites have been numbered, so a caller can say where a source's
    /// run began before reading it.
    pub(crate) fn counted(&self) -> usize {
        self.spans.len()
    }

    /// Every site numbered since `from` was read out of `path`, imported at `at`.
    ///
    /// Called by the document walk after each imported source, because that walk
    /// is the one thing that knows a source came from somewhere else. The
    /// alternative — a mode on this table, set and cleared around the read —
    /// would be an invariant a caller could break silently, and this is the same
    /// fact with nothing to forget.
    pub(crate) fn imported(&mut self, from: usize, path: &str, at: SourceSpan) {
        if from < self.spans.len() {
            self.brought.push(Brought {
                path: path.to_owned(),
                at,
                from,
                upto: self.spans.len(),
            });
        }
    }

    /// Where the term carrying `origin` was written, when this table numbered it.
    ///
    /// [`None`] for [`Origin::UNKNOWN`] and for a number this table did not
    /// hand out — the core mints neither, but a registered builtin's signature
    /// carries `UNKNOWN` by construction, so a refusal *about a signature* has
    /// nowhere of its own to point and says so rather than pointing at node one.
    ///
    /// The span may be a span in *another file*, which is why [`Sites::foreign`]
    /// exists beside this: a caller that publishes a span into a diagnostic has
    /// to ask which document it is a span in first.
    pub(crate) fn span(&self, origin: Origin) -> Option<SourceSpan> {
        origin
            .node_number()
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.spans.get(index))
            .copied()
    }

    /// Record that `module` is the number this document gave `path`.
    ///
    /// Called by the document walk as it mints the ids, for the reason
    /// [`Sites::imported`] is called there: that walk is the one thing that
    /// knows which file a source came out of.
    pub(crate) fn in_file(&mut self, module: ModuleId, path: &str) {
        self.modules.push((module, path.to_owned()));
    }

    /// The file `module` numbers, when this document minted it.
    ///
    /// [`None`] for a module this document did not number — the phase's own and
    /// the prelude's, which are written nowhere in particular and have no file
    /// to name.
    pub(crate) fn file_of(&self, module: ModuleId) -> Option<&str> {
        self.modules
            .iter()
            .find(|(numbered, _)| *numbered == module)
            .map(|(_, path)| path.as_str())
    }

    /// The next block of notation this document reads, numbered.
    ///
    /// A [`musa_score::origin::DeclarationId`] names the declaration a fact was
    /// written in, and the two places a block of notation begins ask for one:
    /// [`notation::Lowering::music`], which every motif, fragment, named bar,
    /// and `music` value goes through, and [`piece::Lowering::piece`], which
    /// enters a voice. Numbered from one, because zero is `factext`'s "no
    /// declaration to name" and stays the answer for a fact no block wrote.
    ///
    /// Handed out in reading order rather than derived from the node, which is
    /// what makes it an *ordinal* and not a second name for the span: two
    /// blocks are two numbers however alike their text.
    pub(crate) fn declaring(&mut self) -> musa_score::origin::DeclarationId {
        self.declarations = self.declarations.saturating_add(1);
        musa_score::origin::DeclarationId(self.declarations)
    }

    /// The file `origin` was read out of, when it was not this document's own
    /// text, with the `import` statement that brought it in.
    pub(crate) fn foreign(&self, origin: Origin) -> Option<(&str, SourceSpan)> {
        let index = origin.node_number().and_then(|index| usize::try_from(index).ok())?;
        self.brought
            .iter()
            .find(|brought| (brought.from..brought.upto).contains(&index))
            .map(|brought| (brought.path.as_str(), brought.at))
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
    /// Whether this walk is reading the *compiled* document.
    ///
    /// False while an imported module's own body is read. A use recorded then
    /// carries a span in that module's coordinates, and
    /// [`crate::resolve::Index`] holds one document's spans — an editor
    /// translates them against the open text, so a foreign one lands wherever
    /// the arithmetic puts it. The index says as much of every entry it keeps
    /// ("where the declaration's name token is, *when it is in this
    /// document*"); this is the same sentence for uses, said at the one walk
    /// that can know.
    here: bool,
    /// The modules this document writes, and how names read inside the
    /// declaration being walked. See [`Naming`].
    naming: Naming<'a>,
    /// The `?`s written since the last position that delimits an answer, in the
    /// order they were written. See [`Lowering::expr`].
    questions: Vec<Question>,
    /// The written `Syntax<…>` category of each annotated parameter in scope,
    /// innermost last.
    ///
    /// The lowering runs before types exist, and 11-quotation §4's "a pattern
    /// is read at the scrutinee's category" needs one anyway: the category is
    /// what the author wrote on the parameter the scrutinee stands behind.
    /// `true` is `Syntax<Expr>` written out; every other annotation and every
    /// other binding is `false` or absent, the token-tree readers being the
    /// default the chain already had. A `let` or pattern binding that reuses a
    /// parameter's name is not tracked, so a shadowed name can be answered
    /// stale — whose failure is a category mismatch naming both categories at
    /// the hole reader, never a wrongly accepted program.
    scrutinee_categories: Vec<(String, bool)>,
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
    /// The named place a decision site inside this walk is addressed under.
    ///
    /// `docs/rules/events/11-realization.md`'s path identity: a site is the
    /// named material enclosing it plus its ordinal among unnamed siblings, and
    /// the ordinal is [`Resolver::decide_count`]'s to mint. So what a walk
    /// carries is the names, and the empty path is a voice's own — which is
    /// deliberately the *same* place in every voice, because the k-th site in
    /// every voice is the k-th site.
    choice: musa_score::ChoicePath,
    /// How many passes each ranged repeat this walk has read plays, by the span
    /// of the statement that wrote it.
    ///
    /// A decision is made once and asked twice: [`notation::Lowering::repeat`]
    /// makes it while folding, and [`notation::extent`] asks again when an
    /// enclosing region measures how far its body reaches. Asking the
    /// realization a second time would mint a second site and could answer
    /// differently, so the answer is kept rather than re-derived — which is the
    /// same rule that puts a resolved meter in [`notation::Reading`] rather than
    /// re-reading the token.
    counts: HashMap<musa_score::origin::SourceSpan, u32>,
    /// How long each freely-held note this walk has read actually sounds, by the
    /// span of the statement that wrote it.
    ///
    /// [`Self::counts`]'s argument, for the other decision a notation statement
    /// can carry: `c5/4 to 2/1` is drawn as a quarter and sounds whatever the
    /// realization chose, and the statement's *sounding* length is what the
    /// region enclosing it has to measure.
    holds: HashMap<musa_score::origin::SourceSpan, Ratio<i64>>,
}

/// The separator between a namespace and one of its members.
///
/// A dot, because that is what the source writes. Names in the core's flat
/// namespace are plain identifiers, so no member name can ever collide with a
/// qualified one. Two rules build a dotted name — an `impl` block's prefix
/// ([`items::Lowering::namespace`]) and an import's `as` qualifier
/// ([`crate::document`]) — and this is what both write.
pub(crate) const DOT: char = '.';

/// Which whole spellings this walk reads as one name.
///
/// One type and one question, because the caller has one question:
/// [`values::Lowering::name`] is deciding whether `low.rise` is a name it
/// should write through or a projection out of a record, and the answer is
/// yes exactly when an import aliased something to `low`.
///
/// The default is a walk of ordinary source, where nothing decides anything —
/// which is every walk in every document that aliases no import, and is why
/// this is [`Default`] rather than a parameter every caller passes.
#[derive(Clone, Copy, Default)]
pub(crate) struct Naming<'a> {
    /// The `as` qualifiers this document's imports wrote, in no order.
    ///
    /// A slice rather than a set: an alias is required exactly where two
    /// modules collide, so this is empty in nearly every document and one entry
    /// long in the rest.
    aliases: &'a [String],
}

impl<'a> Naming<'a> {
    /// Names as they read in a document whose imports wrote these `as`
    /// qualifiers.
    pub(crate) fn under(aliases: &'a [String]) -> Self {
        Self { aliases }
    }

    /// Whether `written` is one name rather than a projection out of its first
    /// segment.
    ///
    /// An alias head is a name no declaration can have, `as` having taken it,
    /// so this cannot be wrong about a spelling. What it answers is the written
    /// name itself, because that is the name [`crate::document::Read::gather`]
    /// filed the definition under.
    fn reads_whole(&self, written: &str) -> bool {
        written
            .split_once(DOT)
            .is_some_and(|(head, _)| self.aliases.iter().any(|alias| alias == head))
    }
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
            here: true,
            naming: Naming::default(),
            questions: Vec::new(),
            scrutinee_categories: Vec::new(),
            minted: 0,
            claims: Vec::new(),
            choice: musa_score::ChoicePath::default(),
            counts: HashMap::new(),
            holds: HashMap::new(),
        }
    }

    /// The same walk, with a module's say in how its names read.
    ///
    /// A method rather than a parameter of [`Self::new`] because almost every
    /// walk has nothing to pass: a document that writes no `structure` reads
    /// every name the way it always has, and a constructor that asked all of
    /// them for the empty answer would put the module system's vocabulary in
    /// front of the passes that have no use for it.
    pub(crate) fn naming(mut self, naming: Naming<'a>) -> Self {
        self.naming = naming;
        self
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

    /// The same walk, reading a document this compilation only imported.
    ///
    /// A method rather than a second constructor for [`Self::naming`]'s reason:
    /// nearly every walk reads the document it is compiling, and the two that
    /// do not — an import's declarations and a bundled module's members — say
    /// so where they already hold the source that told them.
    pub(crate) fn elsewhere(mut self) -> Self {
        self.here = false;
        self
    }

    /// A walk of an adapter phase, where §5.9's vocabulary is readable.
    pub(crate) fn phase(resolver: &'a mut Resolver, sites: &'a mut Sites) -> Self {
        Self {
            in_phase: true,
            ..Self::new(resolver, sites)
        }
    }

    /// The origin numbering `node`.
    pub(crate) fn origin(&mut self, node: &SyntaxNode) -> Origin {
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

    /// Remember the written categories of a parameter list while its body is
    /// lowered, answering how many entries were pushed for
    /// [`Self::pop_syntax_categories`].
    pub(super) fn push_syntax_categories(&mut self, parameters: &[SyntaxNode]) -> usize {
        let mut pushed: usize = 0;
        for parameter in parameters {
            let Some(name) = own_tokens(parameter)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())
            else {
                continue;
            };
            // `Syntax<Expr>` written out, and nothing else: an applied type
            // whose head is the `Syntax` word and whose one index is the
            // `Expr` one — the same reading [`types`] gives it, kept syntactic
            // because the lowering has no types to ask.
            let expression = child(parameter, is_type_node).is_some_and(|written| {
                let parts = children(&written, is_type_node);
                matches!(
                    parts.split_first(),
                    Some((head, [index])) if head.to_string().trim() == "Syntax" && index.to_string().trim() == "Expr"
                )
            });
            self.scrutinee_categories.push((name, expression));
            pushed = pushed.saturating_add(1);
        }
        pushed
    }

    /// Forget what [`Self::push_syntax_categories`] remembered, on every way out.
    pub(super) fn pop_syntax_categories(&mut self, pushed: usize) {
        self.scrutinee_categories
            .truncate(self.scrutinee_categories.len().saturating_sub(pushed));
    }

    /// Whether `name`'s nearest remembered parameter was written `Syntax<Expr>`.
    pub(super) fn scrutinee_is_expression(&self, name: &str) -> bool {
        self.scrutinee_categories
            .iter()
            .rev()
            .find(|(taken, _)| taken == name)
            .is_some_and(|(_, expression)| *expression)
    }

    /// Number the next voice's own decision sites from zero again.
    ///
    /// Sites written among a voice's own items are numbered from zero in every
    /// voice, so the k-th of them is the *same* site in all of them — which is
    /// what makes a repeat the page can draw take one count rather than one per
    /// voice. See [`musa_score::ChoicePath`], and
    /// `docs/rules/events/11-realization.md` for why identity is the path.
    fn restart_sites(&mut self) {
        self.resolver.sites.remove(&musa_score::ChoicePath::default());
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
///
/// [`Raw::hosted`] and not [`Raw::var`], which is what makes an unannotated
/// `[1, 2]` infer. `List` has a parameter and a parameter is an explicit binder
/// at an ordinary use, so `List.Cons 1 …` written as an *author* would write it
/// means `Cons` at the type `1` — see `musa-calculus`'s `constructed_open`. The
/// reader knows it wrote no parameter, and `hosted` is how it says so.
fn listed(origin: Origin, members: Vec<Raw>) -> Raw {
    let mut built = Raw::hosted(origin, "List.Empty");
    for member in members.into_iter().rev() {
        built = applied(origin, Raw::hosted(origin, "List.Cons"), [member, built]);
    }
    built
}

/// `(m₁, …, mₙ)`, as nested `Pair`s — the anonymous product of any width.
///
/// Beside [`listed`] for its reason, and generic for one more: the *direction*
/// of the nesting has to be the same on the type side, the value side, and in a
/// pattern, and three copies of a fold are three chances for one of them to
/// lean the other way. All that differs between the three is which two-argument
/// form a pair is written as, so that is the argument.
///
/// **To the right**, so `(A, B, C)` is `Pair A (Pair B C)` and `(a, b, c)` is
/// `(a, (b, c))`. That is not a nesting nobody wrote: it is what every
/// implementation of a core whose product is binary writes, and it is the
/// choice under which the two spellings agree — a composer who nests by hand
/// gets the same type and the same value as one who writes the commas. Idris
/// folds a comma run right in its parser and applies a two-argument `Pair` at
/// desugaring, and the pair a program sees is binary either way.
///
/// The alternative was to refuse more than two, which reads as a smaller
/// language for no gain: the corpus writes wide products where a *count* is the
/// observation — the arity a sequence splice arrives with, which is the whole
/// of what `11-quotation.md` §2 says a position's grammar supplies — and there
/// is no narrower spelling of that.
///
/// Two or more is what the grammar can produce, since a parenthesized run with
/// no comma is a `ParenExpr`; the single-member fold below is therefore the
/// identity it should be rather than a case anybody reaches.
fn paired<T>(mut members: Vec<T>, pair: impl Fn(T, T) -> T) -> Option<T> {
    let mut built = members.pop()?;
    while let Some(before) = members.pop() {
        built = pair(before, built);
    }
    Some(built)
}

/// The origin a reading hands `instanced`: one expansion step, and the place
/// that produced it.
///
/// `at` twice and the step the reading minted, which is what an expansion is at
/// this point: a place in the source, and the identity a site was given there.
/// Only the path is read — a fact keeps the span of the text it was written as —
/// and the spans are the site's because an origin without one is a value no
/// diagnostic could restate.
///
/// Three readings mint one, and they are the ways facts get made somewhere
/// other than where they were written: [`events`] at a `${…}`, whose step is the
/// locus the hole stands at; [`notation::raw`] at a `use`, whose step is the use
/// site; and [`notation::materials`] at a spelling override, whose step is the
/// note the override names.
pub(crate) fn expansion(at: SourceSpan, step: musa_score::origin::ExpansionStep) -> musa_score::origin::Origin {
    musa_score::origin::Origin {
        source_span: at,
        definition_span: at,
        declaration: musa_score::origin::DeclarationId(0),
        expansion_path: vec![step],
    }
}

/// `n`, as one node at `Nat`.
///
/// `Nat` is a declared family rather than a base type (`02-core-calculus.md`
/// §5.8), and it is a *counting* family, so a written number elaborates to a
/// numeral rather than to that many applications of `Nat.Succ`. The numbers are
/// not small: `repeat 384` writes 384, and a whole note over a 384-tick division
/// writes 384. The tower cost one term node and one evaluator frame per unit,
/// which is why writing one is now writing a count.
fn whole(origin: Origin, value: u64) -> Raw {
    Raw::numeral(origin, "Nat", value)
}

/// The tokens of `node` that carry meaning, in order.
fn significant_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.descendants_with_tokens()
        .filter_map(musa_syntax::SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
}

/// The tokens `node` itself holds, without descending into its children.
///
/// The difference from [`significant_tokens`] is what keeps a form's own words
/// apart from its parts': `p step down n` writes `down` in the `StepExpr`, and a
/// `down` inside the expression `n` would otherwise answer for it.
fn own_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.children_with_tokens()
        .filter_map(musa_syntax::SyntaxElement::into_token)
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
/// Every form an expression may be written as, including the five prompts 136
/// and 137 added to the grammar and the old checker never learned to read.
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
            | SyntaxKind::MethodCallExpr
            | SyntaxKind::LambdaExpr
            | SyntaxKind::PitchExpr
            | SyntaxKind::ChordExpr
            | SyntaxKind::ScaleExpr
            | SyntaxKind::KeyExpr
            | SyntaxKind::StepExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::LetExpr
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
            | SyntaxKind::EventsQuote
    )
}

/// Whether a node kind is one of the written type forms.
///
/// The grammar's own list, asked rather than restated. A copy of it here went
/// stale the moment `IndexedType` joined the grammar: `Nat(12)` parsed, and
/// every reader that finds an annotation with this predicate looked straight
/// past it, so a parameter written at an indexed type lowered as a λ with no
/// domain and the author was told their parameter needed a type. The list that
/// decides what a type node *is* belongs to the crate that builds the nodes.
///
/// `is_expr_node` above is a deliberately different list and says so; this one
/// never had a reason to differ.
fn is_type_node(kind: SyntaxKind) -> bool {
    musa_syntax::ast::is_type(kind)
}
