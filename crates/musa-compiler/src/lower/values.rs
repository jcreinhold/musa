//! A written expression read as a raw core term.
//!
//! The largest half of the module, and the one where "desugaring" is most of the
//! work: `01-surface.md` gives twenty-six expression node kinds and
//! `02-core-calculus.md` §1 gives about ten term formers, so most of what
//! follows is one written form becoming a shape the core already had.
//!
//! # Where a surface form has no core term
//!
//! One kind is read by the grammar and denotes nothing yet, and it answers a
//! diagnostic rather than an unhelpful `None`. `MusicExpr` and `EventsQuote`
//! were here and are gone: prompt 141k folded the first and
//! [`super::events`] reads the second. The anonymous product was here too, and
//! prompt 142 gave it a spelling at every width — [`super::paired`] nests it.
//!
//! - A **named** call argument. The core applies positionally, and reordering a
//!   written argument list to match a declaration would mean resolving the
//!   callee — which is the core's, one pass later. Prompt 142's migration writes
//!   the arguments in order.

use musa_calculus::{Origin, Raw, RawArm, RawPattern, Sort};
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use super::items::declared_name;
use super::{
    Lowering, Question, applied, child, children, is_expr_node, is_type_node, listed, own_tokens, paired,
    significant_tokens, whole, writes,
};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

impl Lowering<'_> {
    /// A position whose value is an answer, lowered.
    ///
    /// The difference from [`Lowering::value`] is `?`. A question mark says
    /// "propagate this failure out of the answer I am part of", and this is the
    /// function that decides which answer that is: a function body, a lambda
    /// body, a match arm, and an `if` branch each delimit one, so each calls
    /// this and each drains the questions written inside it.
    ///
    /// # Why the boundary is here and not at the function
    ///
    /// A `?` inside a match arm whose match wrapped the *function* would evaluate
    /// its subject whether or not the arm was taken, which is a different program
    /// from the one that was written. Draining at the arm keeps the failure where
    /// the author put it, and costs nothing: the arm's type is then a `Result`,
    /// and the core is what says so.
    pub(crate) fn expr(&mut self, node: &SyntaxNode) -> Option<Raw> {
        let outer = std::mem::take(&mut self.questions);
        let answer = self.value(node);
        let asked = std::mem::replace(&mut self.questions, outer);
        self.answered(answer?, asked)
    }

    /// `built`, with `asked`'s questions caught around it.
    ///
    /// Held apart from [`Lowering::expr`] because a notated declaration delimits
    /// an answer without going through `value`: a motif's body is a fold rather
    /// than an expression node, and it catches its own failures for the same
    /// reason a function body does.
    pub(super) fn answered(&mut self, built: Raw, asked: Vec<Question>) -> Option<Raw> {
        let mut built = built;
        // Outermost first, so that a later question may mention an earlier
        // question's binder — `f(x)?.g()?` is the ordinary case.
        for Question {
            subject,
            binder,
            origin,
        } in asked.into_iter().rev()
        {
            let failure = self.mint("failure");
            built = Raw::match_on(
                origin,
                [subject],
                vec![
                    RawArm {
                        patterns: vec![RawPattern::constructor(
                            origin,
                            "Result.Ok",
                            [RawPattern::bind(origin, binder.as_str())],
                        )],
                        body: built,
                    },
                    RawArm {
                        patterns: vec![RawPattern::constructor(
                            origin,
                            "Result.Err",
                            [RawPattern::bind(origin, failure.as_str())],
                        )],
                        body: Raw::app(
                            origin,
                            Raw::hosted(origin, "Result.Err"),
                            Raw::var(origin, failure.as_str()),
                        ),
                    },
                ],
            );
        }
        Some(built)
    }

    /// Any expression position, lowered.
    ///
    /// A `?` written inside adds to [`Lowering::questions`] rather than being
    /// handled here, because the form that catches it is above.
    pub(crate) fn value(&mut self, node: &SyntaxNode) -> Option<Raw> {
        let origin = self.origin(node);
        match node.kind() {
            // `⟦(e)⟧ = ⟦{ e }⟧ = ⟦e⟧`. Braces delimit one expression and hold no
            // sequence, so a block adds a shape to the grammar and no case here.
            SyntaxKind::ParenExpr | SyntaxKind::BlockExpr => {
                let inner = child(node, is_expr_node)?;
                self.value(&inner)
            }
            SyntaxKind::LiteralExpr => self.literal(node, origin),
            SyntaxKind::NameExpr => self.name(node, origin),
            SyntaxKind::PathExpr => self.path(node, origin),
            SyntaxKind::ProductExpr => self.product(node, origin),
            SyntaxKind::ListExpr => self.list(node, origin),
            SyntaxKind::OptionExpr => self.option(node, origin),
            SyntaxKind::ResultExpr => self.result(node, origin),
            SyntaxKind::ApplyExpr => self.application(node, origin),
            SyntaxKind::MethodCallExpr => self.method_call(node, origin),
            SyntaxKind::LambdaExpr => self.lambda(node, origin),
            SyntaxKind::BinaryExpr => self.operator(node, origin),
            SyntaxKind::IndexExpr => self.indexing(node, origin),
            SyntaxKind::PitchExpr => self.transposition(node, origin),
            SyntaxKind::StepExpr => self.scale_step(node, origin),
            SyntaxKind::ChordExpr => self.chord(node, origin),
            SyntaxKind::ScaleExpr => self.scale(node, origin),
            SyntaxKind::KeyExpr => self.key(node, origin),
            SyntaxKind::MatchExpr => self.match_on(node, origin),
            SyntaxKind::LetExpr => self.local_binding(node, origin),
            SyntaxKind::IfExpr => self.conditional(node, origin),
            SyntaxKind::RecordLiteralExpr => self.record(node, origin),
            SyntaxKind::RecordUpdateExpr => self.record_update(node, origin),
            SyntaxKind::QuestionExpr => self.question(node, origin),
            SyntaxKind::QuoteExpr => self.quote(node, origin),
            SyntaxKind::MusicExpr => self.music(node),
            SyntaxKind::EventsQuote => self.events_quote(node, origin),
            _ => None,
        }
    }

    // ---- naming and literals ----

    /// A written name.
    ///
    /// Three readings, and only one of them asks anything to be resolved:
    ///
    /// - `TokenKind.Comma` and `Delimiter.Parentheses` are **literals** of the
    ///   phase's own enumerations, readable only where an adapter is read
    ///   (`02-core-calculus.md` §5.9). They are compiler-owned constants whose
    ///   spelling happens to contain a dot, so nothing an adapter declares can
    ///   collide with one and ordinary source cannot reach them at all.
    /// - `x.f` is a **projection**. `10-traits.md` §6 gives `::` to namespaces,
    ///   which leaves `.` meaning one thing in an expression rather than two.
    /// - Anything else is a variable, written through for the core to resolve.
    fn name(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let written = written_name(node)?;
        // A `_` that reached here is one no argument list caught, so it stands
        // where a value stands and there is no call for it to leave a slot in.
        // Said as its own refusal rather than as "unknown name `_`", because
        // the author wrote a form the language has and wrote it somewhere the
        // form does not go.
        if written == "_" {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "`_` is not a value")
                    .at(crate::resolve::trimmed_span(node), "written here")
                    .help("write `_` in a call's argument list — `transposed_by(3, _)` is the function that still needs the second argument")
                    .note("a `_` elsewhere has no call to leave a slot in, so there is nothing for it to stand for"),
            );
        }
        if self.in_phase
            && let Some(literal) = phase_literal(&written)
        {
            return Some(Raw::lit(origin, literal));
        }
        // Types are terms in Musa, so a type name has the same core spelling
        // wherever it occurs. In particular, an argument of `Option(NoteName)`
        // is parsed by the one application grammar as a `NameExpr`; it must not
        // lose the surface-to-core spelling that a bare annotation receives.
        // This is deliberately not application-head-specific: `Type` and every
        // compiler-owned type may be passed as an ordinary dependent argument.
        if written == "Type" {
            return Some(Raw::universe(origin, Sort::ZERO));
        }
        if let Some(spelled) = super::types::compiler_type(&written) {
            return Some(Raw::var(origin, spelled));
        }
        if self.naming.reads_whole(&written) {
            if self.here {
                self.resolver
                    .references
                    .speak(&written, crate::resolve::trimmed_span(node));
            }
            return Some(Raw::var(origin, written.as_str()));
        }
        // Every segment after the first, and not just the second: §1.2 lets a
        // record hold a record, so `moved.region.anchor` is two projections and
        // reading it as one field named `region.anchor` would look for a field
        // no declaration has. The parser already declined to say where a path
        // stops and a projection starts — this is where the answer is, and the
        // alias reading above is the half that had to go first.
        let mut segments = written.split('.');
        let subject = segments.next()?;
        if subject.len() == written.len() {
            // Told to the index rather than resolved here: what this document
            // declares was recorded before any body was read, and the core
            // answers what the name means.
            if self.here {
                self.resolver
                    .references
                    .speak(&written, crate::resolve::trimmed_span(node));
            }
            return Some(Raw::var(origin, subject));
        }
        Some(segments.fold(Raw::var(origin, subject), |record, field| {
            Raw::project(origin, record, field)
        }))
    }

    /// `std::tonal::TokenKind::PitchLiteral` — an item in a type's namespace.
    ///
    /// The `.` reading above and this one never meet: `10-traits.md` §6 gives
    /// `::` to namespaces, which is what leaves `.` meaning projection or a
    /// method call and nothing else. So a path is *not* read as a projection
    /// however many dots its core name ends up containing.
    fn path(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let segments = musa_syntax::ast::PathExpr::cast(node.clone())?.segments();
        let written = self.qualified(&segments, crate::resolve::trimmed_span(node))?;
        if self.in_phase
            && let Some(literal) = phase_literal(&written)
        {
            return Some(Raw::lit(origin, literal));
        }
        Some(Raw::var(origin, written.as_str()))
    }

    /// The core name `segments` write, or the one refusal a *reading* can raise
    /// about a path.
    ///
    /// # Capitalization decides, and nothing is looked up
    ///
    /// `01-surface.md` §1.5 fixes the whole reading and leaves none of it open:
    /// "lowercase segments are modules, the first capitalized segment names a
    /// type, and exactly one segment follows it." So the modules are
    /// dropped — what a module name *reaches* is `use` and `import`'s question,
    /// and this module resolves nothing — and what is left is joined with `.`,
    /// which is how `musa-calculus` spells a qualified name: `Nat.Succ`,
    /// `Bool.True`, `Pitch.act`. The result goes through as a variable like every
    /// other name, so an item nothing declares is the core's `UnknownName` at
    /// the origin this node was numbered with, and `Lowering`'s "No scope"
    /// stays true.
    ///
    /// # What is refused and what is not
    ///
    /// More than one segment after the capitalized one, and only that: it is a
    /// claim about the written path, which is the one kind of claim a reading
    /// can make without becoming a checker.
    ///
    /// A path *ending* at the capitalized segment is not refused — it names the
    /// type itself, and §1.5's refusal-table row is about "the extra
    /// segment". Nor is a lowercase item: `Duration::of(r)` is the row below it
    /// and `Head::member(x)` is §1.5's own sentence, so a blanket ban on a
    /// lowercase segment after a capitalized one would forbid the two spellings
    /// the section exists to make available.
    ///
    /// A path with no capitalized segment at all is every-segment-a-module and a
    /// name: `std::tonal::sixth_over` is `sixth_over`, for the same reason and
    /// by the same rule.
    fn qualified(&mut self, segments: &[String], span: SourceSpan) -> Option<String> {
        let named = segments
            .iter()
            .position(|segment| segment.starts_with(char::is_uppercase))
            .unwrap_or(segments.len().checked_sub(1)?);
        match segments.get(named..)? {
            [held] => Some(held.clone()),
            [held, item] => Some(format!("{held}.{item}")),
            [held, item, extra, ..] => self.refuse(
                Diagnostic::error(Code::QualifiedPath, "a type namespace holds one item")
                    .at(span, format!("`{extra}` is written inside `{held}::{item}`"))
                    .help(format!("write the path of the type `{extra}` belongs to")),
            ),
            [] => None,
        }
    }

    /// A scalar literal, at the base type its own token names.
    ///
    /// `02-core-calculus.md` §2's "a literal infers" is this function's whole
    /// shape: the token decides, and nothing is passed in. `Nat` and `Bool` are
    /// declared families rather than base types, so their literals are
    /// constructor applications — `3` is three `Nat.Succ`s — which is why they
    /// need no [`musa_calculus::Literal`] and no registered domain.
    fn literal(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let token = significant_tokens(node).next()?;
        Some(match token.kind() {
            SyntaxKind::TrueKw => Raw::hosted(origin, "Bool.True"),
            SyntaxKind::FalseKw => Raw::hosted(origin, "Bool.False"),
            SyntaxKind::Integer => whole(origin, self.whole_number(&token)?),
            SyntaxKind::Rational => plain_literal(origin, "Ratio", self.exact(&token)?),
            SyntaxKind::String => plain_literal(origin, "Text", musa_syntax::ast::unquote(&super::lexeme_text(&token))),
            SyntaxKind::PitchLiteral => {
                let Some(pitch) = musa_score::WrittenPitch::parse(&super::lexeme_text(&token)) else {
                    return self.not_a(&token, "pitch");
                };
                plain_literal(origin, "Pitch", pitch)
            }
            SyntaxKind::IntervalLiteral => {
                let Some(interval) = musa_score::Interval::parse(&super::lexeme_text(&token), false) else {
                    return self.not_a(&token, "interval");
                };
                plain_literal(origin, "Interval", interval)
            }
            _ => return None,
        })
    }

    /// `n`, as a natural number.
    fn whole_number(&mut self, token: &musa_syntax::SyntaxElement) -> Option<u64> {
        super::lexeme_text(token).parse().ok().or_else(|| {
            self.refuse(
                Diagnostic::error(Code::OutOfRange, "this natural number is too large")
                    .at(token_span(token), "outside Musa's exact natural range"),
            )
        })
    }

    /// `p/q`, as an exact rational, or [`None`] with the reason it is not one.
    ///
    /// A refusal rather than a bare [`None`], for the reason [`Lowering::not_yet`]
    /// gives: an empty `None` out of [`Lowering::literal`] means "this node is not
    /// an expression", and a `Rational` token always is one. Returning it silently
    /// made `999999999999999999999/1` compile clean — the overflow reached the `?`
    /// in the literal arm and left with it, and the piece was accepted holding no
    /// value for the number the author wrote.
    ///
    /// Two ways to fail and two messages, because they are two mistakes: a
    /// numerator or denominator past `i64` is a number outside the exact range
    /// musical time is measured in, and a zero denominator is not a number at all.
    fn exact(&mut self, token: &musa_syntax::SyntaxElement) -> Option<Ratio<i64>> {
        let written = super::lexeme_text(token);
        let (numerator, denominator) = written.split_once('/')?;
        let (Ok(numerator), Ok(denominator)) = (numerator.parse::<i64>(), denominator.parse::<i64>()) else {
            return self.refuse(
                Diagnostic::error(Code::OutOfRange, "this rational number is too large")
                    .at(token_span(token), "outside Musa's exact rational range")
                    .note("musical time is exact, so a ratio is held as a pair of 64-bit integers"),
            );
        };
        if denominator == 0 {
            return self.refuse(
                Diagnostic::error(Code::OutOfRange, "this rational number is divided by zero")
                    .at(token_span(token), "the denominator is zero")
                    .help("write a denominator other than `0`"),
            );
        }
        Some(Ratio::new(numerator, denominator))
    }

    /// A form the grammar reads and the core cannot yet be told about.
    ///
    /// A diagnostic rather than a bare [`None`], because the difference matters
    /// to the law beside this module: `None` means "this node is not an
    /// expression", and these nodes *are* expressions whose core shape a named
    /// prompt is about to supply.
    pub(super) fn not_yet<T>(&mut self, node: &SyntaxNode, what: &str, needs: &str) -> Option<T> {
        self.refuse(
            Diagnostic::error(
                Code::UnsupportedLanguageStage,
                format!("{what} has no core spelling yet"),
            )
            .at(crate::resolve::trimmed_span(node), format!("{needs} is written here"))
            .note("both reached the core in prompts 141ga and 141h; prompt 142 teaches the source to spell them"),
        )
    }

    /// "`x` is not a τ", at the token that wrote it.
    fn not_a<T>(&mut self, token: &musa_syntax::SyntaxElement, what: &str) -> Option<T> {
        self.refuse(
            Diagnostic::error(
                Code::NotAValue,
                format!("`{}` is not a {what}", super::lexeme_text(token)),
            )
            .at(token_span(token), format!("invalid {what} literal")),
        )
    }

    // ---- the built-up forms ----

    /// `(a, b, …)` — the anonymous product, as the `Pair`s the prelude declares.
    ///
    /// A constructor application and not a structural record, which makes a
    /// written product *canonical data*: `Pair.Both a b` is a
    /// [`musa_calculus::Datum::Case`], so a δ-rule and a claim's argument can read
    /// one back, and a record cannot be read back at all. That is the whole of
    /// why this is `Pair` — see [`crate::prelude`] for why the halves stay
    /// positional, and [`super::paired`] for why a wider one nests to the right.
    fn product(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        paired(self.every(node)?, |first, second| {
            applied(origin, Raw::hosted(origin, "Pair.Both"), [first, second])
        })
    }

    /// `[a, b]` — a cons list, built from its tail.
    fn list(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        Some(listed(origin, self.every(node)?))
    }

    /// `some(e)` and `none`.
    fn option(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        match child(node, is_expr_node) {
            Some(held) => {
                let held = self.value(&held)?;
                Some(Raw::app(origin, Raw::hosted(origin, "Option.Some"), held))
            }
            None => Some(Raw::hosted(origin, "Option.None")),
        }
    }

    /// `Ok(e)` and `Err(e)`.
    ///
    /// [`Raw::hosted`] rather than [`Raw::var`], for the reason [`Self::option`]
    /// one function up is: `Ok` is a form of the grammar and not a path the
    /// author qualified, so the single written argument is the *field* and there
    /// are no parameters to misread it as. That is exactly the promise
    /// `RawShape::Hosted` carries, and it is what lets `Ok(x)` infer where the
    /// expected type is still a metavariable — an adapter's `syntax_fold_from_leaves`
    /// branch is checked against the fold's answer parameter long before the
    /// annotation on the transformer solves it.
    fn result(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let constructor = if writes(node, SyntaxKind::ErrKw) {
            "Result.Err"
        } else {
            "Result.Ok"
        };
        let held = child(node, is_expr_node)?;
        let held = self.value(&held)?;
        Some(Raw::app(origin, Raw::hosted(origin, constructor), held))
    }

    /// `f(a, b)`, and `x.m(a)` with it.
    ///
    /// The two are one node kind because the grammar makes them one: `x.m` is a
    /// dotted `NameExpr` in head position, so what separates a method call from
    /// an ordinary call is whether the head has a dot — and that is a question
    /// about the written head, not about any type.
    fn application(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let head = child(node, is_expr_node)?;
        if head.kind() == SyntaxKind::NameExpr
            && let Some(written) = written_name(&head)
        {
            let written_arguments = written_arguments(node);
            if let Some((index, help)) = self.indexed_base(&written) {
                return self.written_index(index, &written, node, &written_arguments, help);
            }
            if matches!(written.as_str(), "Machine" | "Primitive") {
                return self.machine_type(node, origin, &written, &head, &written_arguments);
            }
        }
        let Written {
            arguments,
            slots,
            supplied,
        } = self.arguments(node)?;
        // The one call whose arguments are not exactly what was written.
        // §5.7 requires every constructed fact to carry an origin and a scope,
        // `BUILTIN_OWNERSHIP` already calls both of them `play`'s hidden
        // information, and a composer has neither to give: an origin is where
        // this call *is*, which is the reading's to know and not the caller's.
        // So the reading supplies them, exactly as `Lowering::sounded` supplies
        // them for a notation statement. The arity is what makes this
        // unambiguous — the registered `play` takes four, so a written two is
        // this and nothing else.
        if arguments.len() == 2 && head.kind() == SyntaxKind::NameExpr && written_name(&head).as_deref() == Some("play")
        {
            let supplied = [
                // Unplaced, for the same reason the scope below is `Piece`: a
                // `fn` body stands at no one place in the piece, so where its
                // facts *came from* is each call site's to fill in. Under no
                // declaration by the same argument — this is a call in a
                // function and not a block of notation, so there is no motif,
                // bar, or voice to name.
                self.provenance_at(origin, false, musa_score::origin::DeclarationId::default()),
                // The scope a `music` block starts at, because a `fn` body is
                // inside no voice and no part. `stdlib/src/voicing.musa:57`'s
                // `sound_for` is the caller this is written for, and the voice
                // that eventually sounds it is the fold's to say.
                super::notation::scope_of(origin, musa_score::Scope::Piece),
            ];
            return Some(sectioned(
                applied(
                    origin,
                    Raw::hosted(origin, "play"),
                    supplied.into_iter().chain(arguments),
                ),
                slots,
            ));
        }
        // The other call whose head is not what was written. `primitive` has no
        // type of its own: `03-machine-calculus.md` §1 lets the *name and
        // version* decide the step, both ports, and the type of the
        // configuration, so what is registered is one closed signature per unit
        // this build knows (`crate::registry::machine::primitives`) and the word
        // is registered nowhere. Reading it here is what turns the written pair
        // into the signature it selects — which is also the whole of the
        // refusal, since a pair the build does not register selects nothing.
        if arguments.len() == 3
            && head.kind() == SyntaxKind::NameExpr
            && written_name(&head).as_deref() == Some("primitive")
        {
            let unit = self.unit(node, origin)?;
            return Some(sectioned(Raw::app(origin, unit, arguments.into_iter().nth(2)?), slots));
        }
        // A dotted head is `10-traits.md` §6's method syntax, which resolves by
        // exact receiver *in the core*. Writing it as a projection applied would
        // be a different form — a record field that happened to be a function —
        // and would not find a trait's method at all.
        //
        // Unless [`Naming`](crate::lower::Naming) reads the whole spelling as one
        // name, in which case there is no receiver: `low.rise()` calls what an
        // aliased import brought in. That reading goes first here for the reason
        // [`Self::name`] gives for putting it first there. Falling through hands
        // the head to [`Self::value`], which asks the same question and is the
        // one place the answer is turned into a term.
        if head.kind() == SyntaxKind::NameExpr
            && let Some(written) = written_name(&head)
            && let Some((receiver, method)) = written.split_once('.')
            && !self.naming.reads_whole(&written)
            && !(self.in_phase && phase_literal(&written).is_some())
        {
            let receiver = Raw::var(self.origin(&head), receiver);
            return Some(sectioned(
                Raw::call_supplying(
                    origin,
                    Raw::method(origin, receiver, method),
                    arguments,
                    supplying(&supplied),
                ),
                slots,
            ));
        }
        let head = self.value(&head)?;
        // `Raw::call` and not `applied`, at the two sites that read an argument
        // list an author wrote: `02-core-calculus.md` §1.3 makes a call complete,
        // and the core can only say so where it can see the whole list. The
        // reader's own constructions above stay applications, because an arity
        // sentence about `play`'s four arguments would be about this reading
        // rather than about the two the composer wrote.
        Some(sectioned(
            Raw::call_supplying(origin, head, arguments, supplying(&supplied)),
            slots,
        ))
    }

    /// The registered unit `primitive("name", version, …)` selects.
    ///
    /// Read off the *tokens* and not off the lowered arguments, because that is
    /// what selecting a signature means here: the name and the version are
    /// consulted by the compiler before there is a type to check anything at,
    /// so they have to be written out at the call and cannot be computed. A
    /// composer who wants one chosen at compile time writes the two calls.
    fn unit(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let written = written_arguments(node);
        let [name, version, _] = written.as_slice() else {
            return None;
        };
        let span = crate::resolve::trimmed_span(node);
        let (Some(id), Some(number)) = (text_of(name), whole_of(version)) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a registered unit is named by a written name and version")
                    .at(span, "expected `primitive(\"name\", version, configuration)`")
                    .note("the name and the version decide the ports and the configuration, so this compiler reads them before it checks anything"),
            );
        };
        let Some(descriptor) = musa_score::machine::descriptor(&id, number) else {
            return self.refuse(unknown_unit(&id, number, span));
        };
        Some(Raw::var(
            origin,
            crate::registry::unit_spelling(descriptor.id(), descriptor.version()),
        ))
    }

    /// The written arguments of an ordinary call, in order, and the sections
    /// among them.
    ///
    /// A written `_` is not an expression to lower: `02-core-calculus.md` §1.3
    /// keeps a call complete, and a section is the surface's way of saying
    /// which slot the completed call is *waiting* for. So the `_` becomes a
    /// fresh variable here and [`sectioned`] binds it outside the call, which
    /// is Peyton Jones ch. 3's enrichment — a convenience translated away
    /// before anything types it — rather than a new term shape.
    fn arguments(&mut self, node: &SyntaxNode) -> Option<Written> {
        let Some(list) = child(node, |kind| kind == SyntaxKind::ExprArgList) else {
            return Some(Written::default());
        };
        let mut written = Written::default();
        for supplied in children(&list, |kind| kind == SyntaxKind::SuppliedArg) {
            let name = own_tokens(&supplied)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| super::lexeme_text(&token))?;
            let value = self.value(&child(&supplied, is_expr_node)?)?;
            written.supplied.push((name, value));
        }
        for argument in children(&list, |kind| kind == SyntaxKind::ExprArg) {
            // A label at a *use* is refused rather than checked against the
            // declaration's field name, because checking it would need the
            // declaration here and the core is what knows one. Every argument is
            // positional, so a label is either agreeing with the position it is
            // already in or contradicting it, and neither is worth a second way
            // to pass an argument (`01-surface.md` §1.2 names fields at the
            // declaration, which is where the name does work).
            if own_tokens(&argument).any(|token| token.kind() == SyntaxKind::Colon) {
                return self.refuse(
                    Diagnostic::error(Code::UnsupportedLanguageStage, "an argument cannot be labelled here")
                        .at(crate::resolve::trimmed_span(&argument), "labelled here")
                        .help("drop the label — arguments go in the order the declaration takes them")
                        .note("a field's name is written at the declaration; a use passes values by position"),
                );
            }
            let expression = child(&argument, |kind| is_expr_node(kind) || super::is_type_node(kind))?;
            if is_section_hole(&expression) {
                // Minted rather than named after the parameter, because the
                // parameter's name is the callee's and this reading has no
                // callee. `Lowering::mint` prefixes `?`, which no written
                // identifier can spell, so a section cannot capture a name the
                // author had in scope.
                let at = self.origin(&expression);
                let bound = self.mint("section");
                written.arguments.push(Raw::var(at, bound.as_str()));
                written.slots.push((at, bound));
                continue;
            }
            written.arguments.push(if super::is_type_node(expression.kind()) {
                self.ty(&expression)?
            } else {
                self.value(&expression)?
            });
        }
        Some(written)
    }

    /// `fn (x: τ, …) -> ρ { e }`.
    ///
    /// The result type becomes an annotation on the body rather than being
    /// dropped: it is what the author wrote about the answer, and
    /// [`RawShape::Annot`](musa_calculus::RawShape::Annot) is the one place the core
    /// admits a written type inside a term.
    pub(super) fn lambda(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parameters = self.parameters(node)?;
        let body = child(node, is_expr_node)?;
        // The body's quote patterns read these back off the scrutinee's
        // written category — 11-quotation §4 through the lowering's one type
        // fact. Popped on every way out, which the ?-style of this walk makes
        // an explicit call rather than a guard.
        let remembered = self.push_syntax_categories(&parameters);
        let built = self.expr(&body);
        self.pop_syntax_categories(remembered);
        let mut built = built?;
        if let Some(result) = super::child(node, super::is_type_node) {
            let annotation = self.ty(&result)?;
            built = Raw::annot(origin, built, annotation);
        }
        for parameter in parameters.iter().rev() {
            let bound = self.origin(parameter);
            let name = own_tokens(parameter)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| super::lexeme_text(&token))?;
            built = match child(parameter, super::is_type_node) {
                Some(written) => {
                    let domain = self.ty(&written)?;
                    Raw::annotated_lam(bound, name.as_str(), domain, built)
                }
                // An unannotated parameter is `01-surface.md`'s optional
                // annotation, and the core solves it: a bare λ is checkable
                // against a Π, which is exactly the position a lambda is in.
                None => Raw::lam(bound, name.as_str(), built),
            };
        }
        Some(built)
    }

    /// `x ⊕ y` — one member call on the left operand, per `01-surface.md`
    /// §1.5's table.
    ///
    /// Method syntax and not a written-out path, so that `x == y` and
    /// `x.equal(y)` are the same term and resolve by the same rule. There is no
    /// longer a case where the two would differ: the receiver's head is what
    /// names the definition, and a receiver with no rigid head is refused with
    /// the qualified spelling as the repair.
    fn operator(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let method = own_tokens(node).find_map(|token| operator_method(token.kind()))?;
        let parts = children(node, is_expr_node);
        let left = self.value(parts.first()?)?;
        let right = self.value(parts.get(1)?)?;
        Some(Raw::app(origin, Raw::method(origin, left, method), right))
    }

    /// `e.m(y)` where `e` is not a name — `[1, 2].collect()`, `f(x).m(y)`,
    /// `xs[i].m(y)`.
    ///
    /// The other half of [`Self::application`]'s dotted-head reading, and a
    /// separate case for the reason the *parser* makes it one: `low.rise()` is
    /// three tokens that may be a module member or a method, so the name is read
    /// whole and the question is deferred; a receiver with no name has nothing
    /// to defer, so the parser commits and this is where the commitment is
    /// honored. Both readings end at the same [`Raw::method`], which is what
    /// makes "resolves by exact receiver" one rule rather than two.
    ///
    /// Written out because the corpus writes it. `option_fold(fallback, present,
    /// value)` becomes `value.fold_from_end(fallback, present)`, and `value` at
    /// most of `staff.musa`'s call sites is a field read or a call's answer
    /// rather than a bound name — so a reading that took only named receivers
    /// would make the trait unreachable from exactly the code it was introduced
    /// for.
    fn method_call(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let receiver = self.value(&child(node, is_expr_node)?)?;
        let named = own_tokens(node).find(|token| token.kind() == SyntaxKind::Identifier)?;
        let method = Raw::method(origin, receiver, super::lexeme_text(&named));
        let Written {
            arguments,
            slots,
            supplied,
        } = self.arguments(node)?;
        Some(sectioned(
            Raw::call_supplying(origin, method, arguments, supplying(&supplied)),
            slots,
        ))
    }

    /// `xs[i]` — `xs.at(i)`, which is §1.5's last row.
    fn indexing(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let container = self.value(parts.first()?)?;
        let index = self.value(parts.get(1)?)?;
        Some(Raw::app(origin, Raw::method(origin, container, "at"), index))
    }

    // ---- the musical forms ----

    /// `p up M2` and `p down M2`.
    ///
    /// `std::algebra`'s `Action::act`, which is what these two words have always
    /// meant: a written interval acting on a carrier. A method, because the
    /// receiver decides — a `Pitch` and a `PitchClass` are moved by different
    /// operations answering different types, and `10-traits.md` §6's
    /// exact-receiver lookup is what tells them apart. The old checker asked the
    /// type itself, which is the same question one pass earlier and in the pass
    /// that no longer decides types.
    ///
    /// One method and not two, because `down` is a fact about the *interval*.
    /// `p down M2` is `p` moved by the interval that undoes an `M2` — the
    /// `inverse` of `Group<Interval>`, which is what
    /// [`musa_score::Interval::inverse`] answers and what
    /// [`super::notation::Lowering::interval_of`] already writes for the same
    /// two words inside a `music` block. A `transpose_down` beside a
    /// `transpose_up` would be a second method every implementor had to write
    /// and every implementor would write the same way.
    fn transposition(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let pitch = self.value(parts.first()?)?;
        let interval = self.value(parts.get(1)?)?;
        let interval = if writes(node, SyntaxKind::DownKw) {
            Raw::app(origin, Raw::hosted(origin, "interval_inverse"), interval)
        } else {
            interval
        };
        Some(Raw::app(origin, Raw::method(origin, pitch, "act"), interval))
    }

    /// `p step n` and `p step down n`.
    ///
    /// The scale the steps are counted in is *ambient* in the old language, and
    /// an ambient is what the new core does not have: nothing flows into an
    /// expression that the expression does not name. So the reading is the
    /// method, and supplying the scale is prompt 142's migration — the same
    /// change, and for the same reason, as placement moving to the enclosing
    /// voice's fold.
    fn scale_step(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let base = self.value(parts.first()?)?;
        let steps = self.value(parts.get(1)?)?;
        let method = if writes(node, SyntaxKind::DownKw) {
            "step_down"
        } else {
            "step_up"
        };
        Some(Raw::app(origin, Raw::method(origin, base, method), steps))
    }

    /// `chord c# minor` — a root and the content stacked on it.
    fn chord(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let root = self.written_class(node)?;
        let word = trailing_word(node);
        let Some(kind) = musa_score::chord::ChordType::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                    .at(crate::resolve::trimmed_span(node), "not a named chord type")
                    .note("a chord type is the content; the symbol written above the staff is a separate annotation"),
            );
        };
        Some(plain_literal(
            origin,
            "ChordClass",
            musa_score::chord::ChordClass::new(root, kind),
        ))
    }

    /// `scale c dorian` — a collection rooted on a spelled tonic class.
    fn scale(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let tonic = self.written_class(node)?;
        let word = trailing_word(node);
        let Some(collection) = musa_score::scale::Collection::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown collection `{word}`"))
                    .at(crate::resolve::trimmed_span(node), "not a named scale collection")
                    .note("a mode is a rotation of the diatonic collection; other collections are their own values"),
            );
        };
        Some(plain_literal(
            origin,
            "Scale",
            musa_score::scale::Scale::new(tonic, collection),
        ))
    }

    /// `key c minor` — the structural fact, read here as a value.
    fn key(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let tonic = self.written_class(node)?;
        let mode = match trailing_word(node).as_str() {
            "major" => musa_score::Mode::Major,
            "minor" => musa_score::Mode::Minor,
            _ => {
                return self.refuse(
                    Diagnostic::error(Code::NotAValue, "this key cannot be read")
                        .at(
                            crate::resolve::trimmed_span(node),
                            "expected a tonic and a mode, like `key a minor`",
                        )
                        .help("a key names a signature and a mode; a collection is written `scale a dorian`"),
                );
            }
        };
        Some(plain_literal(origin, "Key", musa_score::Key::new(tonic, mode)))
    }

    /// The single `PitchClass` node a `chord`, `scale`, or `key` literal spells.
    fn written_class(&mut self, node: &SyntaxNode) -> Option<musa_score::PitchClass> {
        let written = child(node, |kind| kind == SyntaxKind::PitchClass).map(|child| child.text().to_string());
        let read = written
            .as_deref()
            .map(str::trim)
            .and_then(musa_score::PitchClass::parse);
        match read {
            Some(class) => Some(class),
            None => self.refuse(Diagnostic::error(Code::NotAValue, "this tonic cannot be read").at(
                crate::resolve::trimmed_span(node),
                "expected a spelled pitch class such as `bf` or `g#`",
            )),
        }
    }

    // ---- the branching forms ----

    /// `let x : A = v; e` — the core's own `let`, given a spelling.
    ///
    /// Nothing is desugared here, which is the point: `02-core-calculus.md` §2
    /// has had the term since the beginning and `01-surface.md` §9.1's path
    /// update already elaborates to one. What was missing was a way to write
    /// it, so this reads the two expression children and hands them over.
    ///
    /// Both parts go through [`Lowering::value`] and not [`Lowering::expr`]: a
    /// `?` inside a binding's value belongs to the function the binding stands
    /// in, exactly as it would if the value had been written where the name is
    /// used. A boundary here would catch it one level too early and answer the
    /// binding with a `Result` nobody asked for.
    fn local_binding(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let name = declared_name(node)?;
        let parts = children(node, is_expr_node);
        let value = self.value(parts.first()?)?;
        let body = self.value(parts.get(1)?)?;
        match child(node, is_type_node) {
            Some(written) => {
                let ty = self.ty(&written)?;
                Some(Raw::annotated_bind(origin, name, ty, value, body))
            }
            None => Some(Raw::bind(origin, name, value, body)),
        }
    }

    /// `if c { a } else { b }` — the two-arm boolean match the core already has.
    fn conditional(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let condition = self.value(parts.first()?)?;
        let consequent = self.expr(parts.get(1)?)?;
        let alternative = self.expr(parts.get(2)?)?;
        Some(Raw::match_on(
            origin,
            [condition],
            vec![
                RawArm {
                    patterns: vec![RawPattern::constructor(origin, "Bool.True", [])],
                    body: consequent,
                },
                RawArm {
                    patterns: vec![RawPattern::constructor(origin, "Bool.False", [])],
                    body: alternative,
                },
            ],
        ))
    }

    /// `match s { … }`.
    ///
    /// One subject, because the grammar writes one. Two shapes come out, and
    /// which one depends on whether any arm matches against a **literal of a
    /// base type**: `RawPattern` has no literal case, and that is
    /// `02-core-calculus.md` §5.8's D1 rather than an omission — an inert type
    /// has no eliminator, so there is nothing to split on. Matching against such
    /// a value is decidable equality, so the match becomes a chain of `Bool`
    /// matches on `s == ℓ`, with the subject bound once so it is evaluated once.
    fn match_on(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let subject = child(node, is_expr_node)?;
        let subject = self.value(&subject)?;
        let arms = children(node, |kind| kind == SyntaxKind::MatchArm);
        // A quote pattern first, because the two tests cannot both be about one
        // arm — `matches_a_literal` reads the `Pattern`'s *own* tokens, and a
        // quote pattern holds its body in a child node — and because the quote
        // chain is the one that refuses a match written with both.
        if arms.iter().any(matches_a_quote) {
            return self.quote_chain(node, origin, subject, &arms);
        }
        if arms.iter().any(matches_a_literal) {
            return self.equality_chain(origin, subject, &arms);
        }
        let mut lowered = Vec::with_capacity(arms.len());
        for arm in &arms {
            let written = child(arm, |kind| kind == SyntaxKind::Pattern)?;
            let pattern = self.pattern(&written)?;
            let body = child(arm, is_expr_node)?;
            let body = self.expr(&body)?;
            lowered.push(RawArm {
                patterns: vec![pattern],
                body,
            });
        }
        Some(Raw::match_on(origin, [subject], lowered))
    }

    /// A match against inert values, as nested equality tests.
    fn equality_chain(&mut self, origin: Origin, subject: Raw, arms: &[SyntaxNode]) -> Option<Raw> {
        let bound = self.mint("subject");
        let mut fallback = None;
        let mut tests = Vec::new();
        for arm in arms {
            let written = child(arm, |kind| kind == SyntaxKind::Pattern)?;
            let body = child(arm, is_expr_node)?;
            let body = self.expr(&body)?;
            let at = self.origin(&written);
            match child(&written, |kind| kind == SyntaxKind::LiteralExpr)
                .or_else(|| literal_of(&written))
                .as_ref()
                .and_then(|held| self.literal(held, at))
            {
                Some(literal) => tests.push((at, literal, body)),
                // The first catch-all ends the chain: an arm after it is
                // unreachable, and the core is what says so when it sees the
                // `Bool` match this becomes.
                None => {
                    fallback.get_or_insert(body);
                }
            }
        }
        let mut built = fallback?;
        for (at, literal, body) in tests.into_iter().rev() {
            let same = Raw::app(at, Raw::method(at, Raw::var(at, bound.as_str()), "equal"), literal);
            built = Raw::match_on(
                at,
                [same],
                vec![
                    RawArm {
                        patterns: vec![RawPattern::constructor(at, "Bool.True", [])],
                        body,
                    },
                    RawArm {
                        patterns: vec![RawPattern::constructor(at, "Bool.False", [])],
                        body: built,
                    },
                ],
            );
        }
        Some(Raw::bind(origin, bound.as_str(), subject, built))
    }

    // ---- records ----

    /// `P { f = e, … }` — a record value, headed by the family it names.
    ///
    /// The head rides on the literal rather than in an annotation around it.
    /// An annotation is elaborated as a *type*, and a parameterized record's
    /// family is not one — `Cell : Type 0 → Type 0` — so `Cell { … }` used to
    /// be refused with "this stands where a type is needed, but it is not one"
    /// and every parameterized `record` in the library was a declaration
    /// nothing could construct. Carried this way the head names a family, whose
    /// parameters the fields solve.
    fn record(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let mut fields = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FieldInit) {
            let name = own_tokens(&written)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| super::lexeme_text(&token))?;
            let value = child(&written, is_expr_node)?;
            fields.push((name, self.value(&value)?));
        }
        let fields = fields.iter().map(|(name, term)| (name.as_str(), term.clone()));
        match child(node, |kind| kind == SyntaxKind::NameExpr) {
            Some(named) => {
                let at = self.origin(&named);
                let head = Raw::var(at, named.to_string().trim());
                Some(Raw::headed_record(origin, head, fields))
            }
            None => Some(Raw::record(origin, fields)),
        }
    }

    /// `s with { a.b = e, … }`.
    ///
    /// One [`Raw::update`] rather than a `let` and a record literal per path
    /// segment: the core's update takes the whole path and evaluates the subject
    /// once, so the desugaring `01-surface.md` §9.1 describes is what the core
    /// already does.
    fn record_update(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let subject = child(node, is_expr_node)?;
        let subject = self.value(&subject)?;
        let mut updates: Vec<(Vec<String>, Raw)> = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FieldUpdate) {
            let path = child(&written, |kind| kind == SyntaxKind::FieldPath)?;
            let path: Vec<String> = significant_tokens(&path)
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| super::lexeme_text(&token))
                .collect();
            let value = child(&written, is_expr_node)?;
            updates.push((path, self.value(&value)?));
        }
        let updates: Vec<(Vec<&str>, Raw)> = updates
            .iter()
            .map(|(path, value)| (path.iter().map(String::as_str).collect(), value.clone()))
            .collect();
        Some(Raw::update(
            origin,
            subject,
            updates.iter().map(|(path, value)| (path.as_slice(), value.clone())),
        ))
    }

    /// `e?` — the failure this propagates, recorded for the answer above.
    fn question(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let subject = child(node, is_expr_node)?;
        let subject = self.value(&subject)?;
        let binder = self.mint("held");
        self.questions.push(Question {
            subject,
            binder: binder.clone(),
            origin,
        });
        Some(Raw::var(origin, binder.as_str()))
    }

    // ---- patterns ----

    /// One written pattern.
    ///
    /// The head is read off this node's own tokens — a name, a keyword, a
    /// literal, a bracket, a `::` path — and every *sub-position* is a nested
    /// `Pattern` node, read by recursion. `01-surface.md` §1's "a sub-position
    /// holds another pattern rather than only a binder" is that recursion, and
    /// a bare lowercase name reaching this function one level down is the
    /// binder reading rather than a second rule.
    pub(crate) fn pattern(&mut self, node: &SyntaxNode) -> Option<RawPattern> {
        let origin = self.origin(node);
        if let Some(fields) = child(node, |kind| kind == SyntaxKind::RecordPattern) {
            return self.record_pattern(&fields, origin);
        }
        if let Some(quoted) = child(node, |kind| kind == SyntaxKind::QuotePattern) {
            return self.not_yet(&quoted, "a quote pattern", "a template");
        }
        let written: Vec<musa_syntax::SyntaxElement> = own_tokens(node).collect();
        // `Bass | Tenor` — one arm reached from several branches
        // (`01-surface.md` §1). The alternatives are the node's `Pattern`
        // children and the `|` is its own token, which is what tells this shape
        // from a constructor's sub-positions: those have a head token before
        // them and this has none.
        if written.iter().any(|token| token.kind() == SyntaxKind::Pipe) {
            let alternatives: Vec<RawPattern> = children(node, |kind| kind == SyntaxKind::Pattern)
                .iter()
                .map(|held| self.pattern(held))
                .collect::<Option<_>>()?;
            return Some(RawPattern::or(origin, alternatives));
        }
        let head = written.first()?.clone();
        // The `::` path the head writes, and the bindings after it. Splitting
        // them is the whole of `Tying::Untied`'s repair: reading every later
        // identifier as a binding made the case name one, so the pattern bound
        // `Untied` and matched a constructor named `Tying`.
        let (path, _after) = path_segments(&written);
        // The sub-positions, each already a whole pattern. A field written as a
        // bare lowercase name arrives here as a `Bind`, which is what it always
        // meant; nothing reads a sub-position's spelling any more.
        let nested: Vec<RawPattern> = children(node, |kind| kind == SyntaxKind::Pattern)
            .iter()
            .map(|held| self.pattern(held))
            .collect::<Option<_>>()?;
        // Whether the pattern was *written applied*. A constructor with no
        // fields and a binder are the same tokens — `Silence` — and the
        // uppercase reading below is what tells them apart; a lowercase name
        // with a `(` after it is a constructor whatever its case, and asking
        // the token stream is what keeps that from becoming a second rule.
        let applied = written.iter().any(|token| token.kind() == SyntaxKind::LParen);
        Some(match head.kind() {
            SyntaxKind::SomeKw => RawPattern::constructor(origin, "Option.Some", nested),
            SyntaxKind::NoneKw => RawPattern::constructor(origin, "Option.None", []),
            SyntaxKind::OkKw => RawPattern::constructor(origin, "Result.Ok", nested),
            SyntaxKind::ErrKw => RawPattern::constructor(origin, "Result.Err", nested),
            SyntaxKind::TrueKw => RawPattern::constructor(origin, "Bool.True", []),
            SyntaxKind::FalseKw => RawPattern::constructor(origin, "Bool.False", []),
            // `3` counts down to `Nat.Zero`, which is an ordinary constructor
            // pattern and needs no literal case: `Nat` is a declared family.
            SyntaxKind::Integer => {
                let mut built = RawPattern::constructor(origin, "Nat.Zero", []);
                for _ in 0..self.whole_number(&head)? {
                    built = RawPattern::constructor(origin, "Nat.Succ", [built]);
                }
                built
            }
            // `[]` and `[head, ..tail]` are the two list patterns the grammar
            // admits, and they are exactly `List`'s two constructors: the
            // bracket is a spelling of a cons cell rather than of a list of
            // known length, so there is no chain to build here.
            SyntaxKind::LBracket => match nested.as_slice() {
                [] => RawPattern::constructor(origin, "List.Empty", []),
                [head, tail] => RawPattern::constructor(origin, "List.Cons", [head.clone(), tail.clone()]),
                // The parser has already complained; a pattern read from a
                // bracket it could not finish would bind names nobody wrote.
                _ => return None,
            },
            // `(m, n)` matches the constructor a product is written as, and a
            // wider one matches the nesting [`super::paired`] writes. A `(` with
            // no name under it is the [`SyntaxKind::LBracket`] case again: the
            // parser has already complained.
            SyntaxKind::LParen => paired(nested, |first, second| {
                RawPattern::constructor(origin, "Pair.Both", [first, second])
            })?,
            // `Tying::Untied` — §1.5's path, read exactly as it is in an
            // expression, because a case named in its type's namespace is the
            // same name written in the same way.
            SyntaxKind::Identifier if path.len() > 1 => {
                let name = self.qualified(&path, crate::resolve::trimmed_span(node))?;
                RawPattern::constructor(origin, name.as_str(), nested)
            }
            SyntaxKind::Identifier => {
                let spelling = super::lexeme_text(&head);
                if !applied && !spelling.chars().next().is_some_and(char::is_uppercase) {
                    // `_` is an ordinary binder whose name nothing refers to,
                    // which is what the core's own `RawPattern::Bind` documents.
                    RawPattern::bind(origin, spelling)
                } else {
                    RawPattern::constructor(origin, spelling, nested)
                }
            }
            _ => return None,
        })
    }

    /// `P { f = g }` — a record pattern binding the fields it names.
    ///
    /// A field written without an `=` is the shorthand `01-surface.md` §9.2
    /// gives patterns and withholds from literals: `Dotted { total }` binds the
    /// field to its own name. The grammar records it by writing no nested
    /// pattern at all ([`musa_syntax::ast::FieldPattern::pattern`] answers
    /// `None`), so the binding is made here rather than read.
    fn record_pattern(&mut self, fields: &SyntaxNode, origin: Origin) -> Option<RawPattern> {
        let mut bound = Vec::new();
        for written in children(fields, |kind| kind == SyntaxKind::FieldPattern) {
            let at = self.origin(&written);
            let name = own_tokens(&written)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| super::lexeme_text(&token))?;
            let held = match child(&written, |kind| kind == SyntaxKind::Pattern) {
                Some(nested) => self.pattern(&nested)?,
                None => RawPattern::bind(at, name.as_str()),
            };
            bound.push((name, held));
        }
        Some(RawPattern::record(
            origin,
            bound.iter().map(|(name, held)| (name.as_str(), held.clone())),
        ))
    }

    /// Every expression child of `node`, lowered in order.
    fn every(&mut self, node: &SyntaxNode) -> Option<Vec<Raw>> {
        children(node, is_expr_node)
            .iter()
            .map(|member| self.value(member))
            .collect()
    }
}

/// A literal of a plain base type, written the way [`crate::registry`] writes
/// one.
///
/// Through [`crate::registry::plain_type`] rather than a term assembled here,
/// so that a lowered `Text` and a registered signature's `Text` are one
/// expression rather than two that agree today.
fn plain_literal<T>(origin: Origin, base: &'static str, value: T) -> Raw
where
    T: PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    Raw::lit(
        origin,
        crate::registry::literal(crate::registry::plain_type(base), value),
    )
}

/// The last identifier a musical literal writes, which is the word naming
/// its collection, chord type, or mode.
fn trailing_word(node: &SyntaxNode) -> String {
    significant_tokens(node)
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .last()
        .map(|token| super::lexeme_text(&token))
        .unwrap_or_default()
}

/// Whether an arm's pattern is a literal of a base type.
pub(super) fn matches_a_literal(arm: &SyntaxNode) -> bool {
    child(arm, |kind| kind == SyntaxKind::Pattern).is_some_and(|pattern| {
        own_tokens(&pattern).any(|token| {
            matches!(
                token.kind(),
                SyntaxKind::String | SyntaxKind::Rational | SyntaxKind::PitchLiteral | SyntaxKind::IntervalLiteral
            )
        })
    })
}

/// Whether an arm matches against a quote pattern (`11-quotation.md` §4).
fn matches_a_quote(arm: &SyntaxNode) -> bool {
    child(arm, |kind| kind == SyntaxKind::Pattern)
        .is_some_and(|pattern| child(&pattern, |kind| kind == SyntaxKind::QuotePattern).is_some())
}

/// The `LiteralExpr` a bare literal pattern would have been, when the
/// grammar wrote the token straight into the `Pattern` node.
fn literal_of(pattern: &SyntaxNode) -> Option<SyntaxNode> {
    own_tokens(pattern)
        .any(|token| {
            matches!(
                token.kind(),
                SyntaxKind::String | SyntaxKind::Rational | SyntaxKind::PitchLiteral | SyntaxKind::IntervalLiteral
            )
        })
        .then(|| pattern.clone())
}

/// The `::` path a pattern's tokens open with, and everything after it.
///
/// A pattern is written flat: `Tying::Untied(why)` is a run of tokens directly
/// under the node, because `constructor_bindings` bumps its names into the
/// pattern rather than starting a node for them. So the case's path and the
/// bindings after it have to be told apart here, and the `::` is what tells
/// them apart — the same question `01-surface.md` §1.5 answers in an expression,
/// asked where the grammar happens to give no node to ask it of.
///
/// A head that is not an identifier — `Some`, `[`, `(`, a number — opens no
/// path, so the segments are empty and every token is "after", which is what
/// the shapes below already read.
fn path_segments(written: &[musa_syntax::SyntaxElement]) -> (Vec<String>, &[musa_syntax::SyntaxElement]) {
    let mut segments = Vec::new();
    let mut rest = written;
    while let [name, tail @ ..] = rest
        && name.kind() == SyntaxKind::Identifier
    {
        segments.push(super::lexeme_text(name));
        match tail {
            [first, second, beyond @ ..] if first.kind() == SyntaxKind::Colon && second.kind() == SyntaxKind::Colon => {
                rest = beyond;
            }
            _ => return (segments, tail),
        }
    }
    (segments, rest)
}

/// The member an operator token stands for — `01-surface.md` §1.5's table.
///
/// The member alone, because that is now all there is to say: `x == y` is
/// `x.equal(y)`, which the receiver's head resolves to `Text.equal` or
/// `Ratio.equal` or whichever definition the type names. The old table wrote a
/// trait beside each word so that an operator under a `where` could resolve
/// against the supplied dictionary; `where` is deleted, so the trait half has
/// nothing left to say and the two spellings became one path.
fn operator_method(kind: SyntaxKind) -> Option<&'static str> {
    Some(match kind {
        SyntaxKind::EqualsEquals => "equal",
        SyntaxKind::Less => "less",
        SyntaxKind::Plus => "add",
        SyntaxKind::Minus => "sub",
        SyntaxKind::Star => "mul",
        SyntaxKind::Slash => "div",
        _ => return None,
    })
}

/// The literal a phase enumeration's written name denotes.
///
/// The two namespaces `crate::phase`'s own `phase_value` reads, and no others: a
/// dotted name outside them is a projection, which is why this answers [`None`]
/// rather than refusing.
fn phase_literal(written: &str) -> Option<musa_calculus::Literal> {
    let (namespace, case) = written.split_once('.')?;
    match namespace {
        "TokenKind" => crate::quote::token_kind_named(case).map(crate::registry::token_kind_literal),
        "Delimiter" => crate::quote::Delimiter::named(case)
            .map(|held| crate::registry::literal(crate::registry::plain_type("Delimiter"), held)),
        _ => None,
    }
}

/// One call's written arguments, and the `_`s among them.
///
/// Two lists rather than one list of an enumeration, because the two answer
/// different questions and are used at different moments: the arguments go
/// inside the call, and the slots go around it.
#[derive(Default)]
struct Written {
    /// The arguments in written order, a section's `_` already standing as the
    /// variable [`sectioned`] will bind.
    arguments: Vec<Raw>,
    /// One binder per written `_`, left to right — so `g(_, b, _)` reads as
    /// `fn (x, y) { g(x, b, y) }` and not the other way round.
    slots: Vec<(Origin, String)>,
    /// `{A = Nat}` — the type parameters this call supplies by name.
    ///
    /// Carried rather than placed, because the name is the *callee's* binder
    /// name and nothing in this reading knows a callee's binders. The core
    /// places each one at the binder that bears it.
    supplied: Vec<(String, Raw)>,
}

/// A [`Written::supplied`] list as [`Raw::call_supplying`] wants it.
fn supplying(supplied: &[(String, Raw)]) -> impl Iterator<Item = (&str, Raw)> {
    supplied.iter().map(|(name, term)| (name.as_str(), term.clone()))
}

/// A call, wrapped in the binders its written `_`s stand for.
///
/// The identity when nothing was written, which is every call in the language
/// before this reading existed and almost every call after it.
fn sectioned(call: Raw, slots: Vec<(Origin, String)>) -> Raw {
    slots
        .into_iter()
        .rev()
        .fold(call, |body, (at, bound)| Raw::lam(at, bound.as_str(), body))
}

/// Whether an argument is a written `_`.
///
/// `_` lexes as an ordinary identifier — the regex is `[a-zA-Z_][a-zA-Z_0-9]*`
/// — so this is a question about the one spelling and not about a token kind
/// the grammar would have to grow. It is asked of an argument only: everywhere
/// else `_` keeps meaning what it already meant, an ignored binding, and
/// [`Lowering::name`] says so where a `_` is written as a value.
fn is_section_hole(node: &SyntaxNode) -> bool {
    node.kind() == SyntaxKind::NameExpr && written_name(node).as_deref() == Some("_")
}

/// The name a `NameExpr` writes: one identifier, or several joined by dots.
///
/// As many as were written, because the parser reads as many as were written:
/// `held.region.anchor` is one node with three identifiers, and stopping at the
/// second would hand [`Lowering::name`] a name whose last segment is silently
/// missing. Which of the segments is a module, which is the value, and which are
/// projections is that reader's question — see its documentation.
///
/// A transformation's word counts as an identifier here, because the parser
/// already says so: `expr_atom` routes `transpose`, `stretch`, `retrograde`,
/// `invert`, and `repeat` through `name_or_record_literal`, so `transpose(P5, e)`
/// is a `NameExpr` applied and reading only [`SyntaxKind::Identifier`] would
/// leave it unlowerable. `00-semantics.md` §3 requires that spelling to exist —
/// "a call must be complete … `transpose(i)` is written as a function that takes
/// its track argument" — and it is the half of §3's implementation theorem that
/// is not written in a block.
fn written_name(node: &SyntaxNode) -> Option<String> {
    let mut tokens = significant_tokens(node).filter(|token| !matches!(token.kind(), SyntaxKind::Whitespace));
    let first = tokens.find(|token| {
        matches!(
            token.kind(),
            SyntaxKind::Identifier
                | SyntaxKind::ListKw
                | SyntaxKind::OptionKw
                | SyntaxKind::ResultKw
                | SyntaxKind::TransposeKw
                | SyntaxKind::StretchKw
                | SyntaxKind::RetrogradeKw
                | SyntaxKind::InvertKw
                | SyntaxKind::RepeatKw
        )
    })?;
    let mut written = super::lexeme_text(&first);
    while tokens.next().is_some_and(|token| token.kind() == SyntaxKind::Dot) {
        let Some(member) = tokens.next().filter(|token| token.kind() == SyntaxKind::Identifier) else {
            break;
        };
        written.push('.');
        written.push_str(&super::lexeme_text(&member));
    }
    Some(written)
}

/// The text a written string literal spells, if that is what this node is.
fn text_of(node: &SyntaxNode) -> Option<String> {
    let token = significant_tokens(node).next()?;
    (token.kind() == SyntaxKind::String).then(|| musa_syntax::ast::unquote(&super::lexeme_text(&token)))
}

/// The natural a written integer literal spells, if that is what this node is.
fn whole_of(node: &SyntaxNode) -> Option<u32> {
    let token = significant_tokens(node).next()?;
    (token.kind() == SyntaxKind::Integer).then(|| super::lexeme_text(&token).parse().ok())?
}

/// The expression node each written argument holds, in written order.
fn written_arguments(node: &SyntaxNode) -> Vec<SyntaxNode> {
    let Some(list) = child(node, |kind| kind == SyntaxKind::ExprArgList) else {
        return Vec::new();
    };
    children(&list, |kind| kind == SyntaxKind::ExprArg)
        .into_iter()
        .filter_map(|argument| child(&argument, |kind| is_expr_node(kind) || is_type_node(kind)))
        .collect()
}

/// "this build registers no such unit", with what it does register.
fn unknown_unit(id: &str, version: u32, span: SourceSpan) -> Diagnostic {
    let versions = musa_score::machine::versions_of(id);
    if versions.is_empty() {
        return Diagnostic::error(Code::UnknownWord, format!("`{id}` is not a unit this build registers"))
            .at(span, "unknown unit")
            .help(crate::resolve::suggest(
                id,
                &musa_score::machine::registered_ids(),
                "units",
            ));
    }
    let spelled: Vec<String> = versions.iter().map(u32::to_string).collect();
    Diagnostic::error(
        Code::UnknownWord,
        format!("this build registers no version {version} of `{id}`"),
    )
    .at(span, "unknown version")
    .help(format!("it registers {}", spelled.join(", ")))
    .note("a unit's version is part of its identity: two versions are two units with two projections")
}

/// Where a token was written.
fn token_span(lexeme: &musa_syntax::SyntaxElement) -> SourceSpan {
    let range = lexeme.text_range();
    SourceSpan::new(range.start().into(), range.end().into())
}
