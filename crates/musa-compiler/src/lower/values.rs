//! A written expression read as a raw core term.
//!
//! The largest half of the module, and the one where "desugaring" is most of the
//! work: `01-surface.md` gives twenty-six expression node kinds and
//! `02-core-calculus.md` §1 gives about ten term formers, so most of what
//! follows is one written form becoming a shape the core already had.
//!
//! # Where a surface form has no core term
//!
//! Four kinds are read by the grammar and denote nothing yet, and each answers a
//! diagnostic rather than an unhelpful `None`:
//!
//! - `MusicExpr` and `KernelQuote` need `EventTrack`. Prompt 141h gave it a core
//!   shape — a base type and eight builtins over it — and deliberately left it
//!   unspellable; prompt 142 is where the source learns the word.
//! - `ProductType`'s value form, `(a, b)`, is written as a record here — the
//!   core has structural records and no separate pair, so a product is the
//!   record whose fields are its positions. That is a *reading*, not a new
//!   feature: nothing about which programs are admitted changes, and a surface
//!   product and the record it becomes have the same projections.
//! - A **named** call argument. The core applies positionally, and reordering a
//!   written argument list to match a declaration would mean resolving the
//!   callee — which is the core's, one pass later. Prompt 142's migration writes
//!   the arguments in order.

use musa_core::{Origin, Raw, RawArm, RawPattern};
use musa_language::{SyntaxKind, SyntaxNode, SyntaxToken};
use num_rational::Ratio;

use super::{
    Lowering, Question, applied, child, children, is_expr_node, listed, own_tokens, significant_tokens, whole, writes,
};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;

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
        let mut built = answer?;
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
                            Raw::var(origin, "Result.Err"),
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
            SyntaxKind::PathExpr => Some(Raw::var(origin, node.to_string().trim())),
            SyntaxKind::ProductExpr => self.product(node, origin),
            SyntaxKind::ListExpr => self.list(node, origin),
            SyntaxKind::OptionExpr => self.option(node, origin),
            SyntaxKind::ResultExpr => self.result(node, origin),
            SyntaxKind::ApplyExpr => self.application(node, origin),
            SyntaxKind::LambdaExpr => self.lambda(node, origin),
            SyntaxKind::BinaryExpr => self.operator(node, origin),
            SyntaxKind::IndexExpr => self.indexing(node, origin),
            SyntaxKind::PitchExpr => self.transposition(node, origin),
            SyntaxKind::StepExpr => self.scale_step(node, origin),
            SyntaxKind::ChordExpr => self.chord(node, origin),
            SyntaxKind::ScaleExpr => self.scale(node, origin),
            SyntaxKind::KeyExpr => self.key(node, origin),
            SyntaxKind::MatchExpr => self.match_on(node, origin),
            SyntaxKind::IfExpr => self.conditional(node, origin),
            SyntaxKind::RecordLiteralExpr => self.record(node, origin),
            SyntaxKind::RecordUpdateExpr => self.record_update(node, origin),
            SyntaxKind::QuestionExpr => self.question(node, origin),
            SyntaxKind::QuoteExpr => self.quote(node, origin),
            SyntaxKind::MusicExpr | SyntaxKind::KernelQuote => self.not_yet(node, "this form", "a track"),
            _ => None,
        }
    }

    // ---- naming and literals ----

    /// A written name.
    ///
    /// Three readings, and the node decides between them without resolving
    /// anything:
    ///
    /// - `TokenKind.Comma` and `Delimiter.Parentheses` are **literals** of the
    ///   phase's own enumerations, readable only where an adapter is read
    ///   (`02-core-calculus.md` §5.9). They are compiler-owned constants whose
    ///   spelling happens to contain a dot, so nothing an adapter declares can
    ///   collide with one and ordinary source cannot reach them at all.
    /// - `x.f` is a **projection**. `10-traits.md` §6 gives `::` to namespaces,
    ///   which leaves `.` meaning one thing in an expression rather than two.
    /// - Anything else is a variable, written through for the core to resolve.
    fn name(&self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let written = written_name(node)?;
        if self.in_phase
            && let Some(literal) = phase_literal(&written)
        {
            return Some(Raw::lit(origin, literal));
        }
        match written.split_once('.') {
            Some((record, field)) => Some(Raw::project(origin, Raw::var(origin, record), field)),
            None => Some(Raw::var(origin, written.as_str())),
        }
    }

    /// A scalar literal, at the base type its own token names.
    ///
    /// `02-core-calculus.md` §2's "a literal infers" is this function's whole
    /// shape: the token decides, and nothing is passed in. `Nat` and `Bool` are
    /// declared families rather than base types, so their literals are
    /// constructor applications — `3` is three `Nat.Succ`s — which is why they
    /// need no [`musa_core::Literal`] and no registered domain.
    fn literal(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let token = significant_tokens(node).next()?;
        Some(match token.kind() {
            SyntaxKind::TrueKw => Raw::var(origin, "Bool.True"),
            SyntaxKind::FalseKw => Raw::var(origin, "Bool.False"),
            SyntaxKind::Integer => whole(origin, self.whole_number(&token)?),
            SyntaxKind::Rational => plain_literal(origin, "Ratio", exact(&token)?),
            SyntaxKind::String => plain_literal(origin, "Text", musa_language::ast::unquote(token.text())),
            SyntaxKind::PitchLiteral => {
                let Some(pitch) = crate::WrittenPitch::parse(token.text()) else {
                    return self.not_a(&token, "pitch");
                };
                plain_literal(origin, "Pitch", pitch)
            }
            SyntaxKind::IntervalLiteral => {
                let Some(interval) = crate::Interval::parse(token.text(), false) else {
                    return self.not_a(&token, "interval");
                };
                plain_literal(origin, "Interval", interval)
            }
            _ => return None,
        })
    }

    /// `n`, as a natural number.
    fn whole_number(&mut self, token: &SyntaxToken) -> Option<u64> {
        token.text().parse().ok().or_else(|| {
            self.refuse(
                Diagnostic::error(Code::OutOfRange, "this natural number is too large")
                    .at(token_span(token), "outside Musa's exact natural range"),
            )
        })
    }

    /// A form the grammar reads and the core cannot yet be told about.
    ///
    /// A diagnostic rather than a bare [`None`], because the difference matters
    /// to the law beside this module: `None` means "this node is not an
    /// expression", and these nodes *are* expressions whose core shape a named
    /// prompt is about to supply.
    fn not_yet<T>(&mut self, node: &SyntaxNode, what: &str, needs: &str) -> Option<T> {
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
    fn not_a<T>(&mut self, token: &SyntaxToken, what: &str) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::NotAValue, format!("`{}` is not a {what}", token.text()))
                .at(token_span(token), format!("invalid {what} literal")),
        )
    }

    // ---- the built-up forms ----

    /// `(a, b)` — the anonymous product, as the record whose fields are its
    /// positions.
    fn product(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let members = self.every(node)?;
        let fields: Vec<(String, Raw)> = members
            .into_iter()
            .enumerate()
            .map(|(index, member)| (position_field(index), member))
            .collect();
        Some(Raw::record(
            origin,
            fields.iter().map(|(name, term)| (name.as_str(), term.clone())),
        ))
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
                Some(Raw::app(origin, Raw::var(origin, "Option.Some"), held))
            }
            None => Some(Raw::var(origin, "Option.None")),
        }
    }

    /// `Ok(e)` and `Err(e)`.
    fn result(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let constructor = if writes(node, SyntaxKind::ErrKw) {
            "Result.Err"
        } else {
            "Result.Ok"
        };
        let held = child(node, is_expr_node)?;
        let held = self.value(&held)?;
        Some(Raw::app(origin, Raw::var(origin, constructor), held))
    }

    /// `f(a, b)`, and `x.m(a)` with it.
    ///
    /// The two are one node kind because the grammar makes them one: `x.m` is a
    /// dotted `NameExpr` in head position, so what separates a method call from
    /// an ordinary call is whether the head has a dot — and that is a question
    /// about the written head, not about any type.
    fn application(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let head = child(node, is_expr_node)?;
        let arguments = self.arguments(node)?;
        // A dotted head is `10-traits.md` §6's method syntax, which resolves by
        // exact receiver *in the core*. Writing it as a projection applied would
        // be a different form — a record field that happened to be a function —
        // and would not find a trait's method at all.
        if head.kind() == SyntaxKind::NameExpr
            && let Some(written) = written_name(&head)
            && let Some((receiver, method)) = written.split_once('.')
            && !(self.in_phase && phase_literal(&written).is_some())
        {
            let receiver = Raw::var(self.origin(&head), receiver);
            return Some(applied(origin, Raw::method(origin, receiver, method), arguments));
        }
        let head = self.value(&head)?;
        Some(applied(origin, head, arguments))
    }

    /// The written arguments of an ordinary call, in order.
    fn arguments(&mut self, node: &SyntaxNode) -> Option<Vec<Raw>> {
        let Some(list) = child(node, |kind| kind == SyntaxKind::ExprArgList) else {
            return Some(Vec::new());
        };
        let mut arguments = Vec::new();
        for argument in children(&list, |kind| kind == SyntaxKind::ExprArg) {
            if own_tokens(&argument).any(|token| token.kind() == SyntaxKind::Colon) {
                return self.refuse(
                    Diagnostic::error(Code::UnsupportedLanguageStage, "an argument is passed by position")
                        .at(crate::resolve::trimmed_span(&argument), "named here")
                        .help("write the arguments in the order the declaration takes them"),
                );
            }
            let written = child(&argument, is_expr_node)?;
            arguments.push(self.value(&written)?);
        }
        Some(arguments)
    }

    /// `fn (x: τ, …) -> ρ { e }`.
    ///
    /// The result type becomes an annotation on the body rather than being
    /// dropped: it is what the author wrote about the answer, and
    /// [`RawShape::Annot`](musa_core::RawShape::Annot) is the one place the core
    /// admits a written type inside a term.
    pub(super) fn lambda(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parameters = child(node, |kind| kind == SyntaxKind::ParamList)
            .map(|list| children(&list, |kind| kind == SyntaxKind::Param))
            .unwrap_or_default();
        let body = child(node, is_expr_node)?;
        let mut built = self.expr(&body)?;
        if let Some(result) = super::child(node, super::is_type_node) {
            let annotation = self.ty(&result)?;
            built = Raw::annot(origin, built, annotation);
        }
        for parameter in parameters.iter().rev() {
            let bound = self.origin(parameter);
            let name = own_tokens(parameter)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
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

    /// `x ⊕ y` — one trait method, per `10-traits.md` §5's table.
    ///
    /// Written as method syntax rather than as a call to a named function,
    /// because that is what an operator *is* here: the receiver's concrete type
    /// selects the instance, and the core does the selecting.
    fn operator(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let operator = own_tokens(node).find_map(|token| operator_method(token.kind()))?;
        let parts = children(node, is_expr_node);
        let left = self.value(parts.first()?)?;
        let right = self.value(parts.get(1)?)?;
        Some(Raw::app(origin, Raw::method(origin, left, operator), right))
    }

    /// `xs[i]` — `Index<C, I, A>.at`, which is §5's last row.
    fn indexing(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let container = self.value(parts.first()?)?;
        let index = self.value(parts.get(1)?)?;
        Some(Raw::app(origin, Raw::method(origin, container, "at"), index))
    }

    // ---- the musical forms ----

    /// `p up M2` and `p down M2`.
    ///
    /// A method, because the receiver decides: a `Pitch` and a `PitchClass` are
    /// transposed by different operations answering different types, and
    /// `10-traits.md` §6's exact-receiver lookup is what tells them apart. The
    /// old checker asked the type itself, which is the same question one pass
    /// earlier and in the pass that no longer decides types.
    fn transposition(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let parts = children(node, is_expr_node);
        let pitch = self.value(parts.first()?)?;
        let interval = self.value(parts.get(1)?)?;
        let method = if writes(node, SyntaxKind::DownKw) {
            "transpose_down"
        } else {
            "transpose_up"
        };
        Some(Raw::app(origin, Raw::method(origin, pitch, method), interval))
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
        let Some(kind) = crate::chord::ChordType::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                    .at(crate::resolve::trimmed_span(node), "not a named chord type")
                    .note("a chord type is the content; the symbol written above the staff is a separate annotation"),
            );
        };
        Some(plain_literal(
            origin,
            "ChordClass",
            crate::chord::ChordClass::new(root, kind),
        ))
    }

    /// `scale c dorian` — a collection rooted on a spelled tonic class.
    fn scale(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let tonic = self.written_class(node)?;
        let word = trailing_word(node);
        let Some(collection) = crate::scale::Collection::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown collection `{word}`"))
                    .at(crate::resolve::trimmed_span(node), "not a named scale collection")
                    .note("a mode is a rotation of the diatonic collection; other collections are their own values"),
            );
        };
        Some(plain_literal(
            origin,
            "Scale",
            crate::scale::Scale::new(tonic, collection),
        ))
    }

    /// `key c minor` — the structural fact, read here as a value.
    fn key(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let tonic = self.written_class(node)?;
        let mode = match trailing_word(node).as_str() {
            "major" => crate::Mode::Major,
            "minor" => crate::Mode::Minor,
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
        Some(plain_literal(origin, "Key", crate::Key::new(tonic, mode)))
    }

    /// The single `PitchClass` node a `chord`, `scale`, or `key` literal spells.
    fn written_class(&mut self, node: &SyntaxNode) -> Option<crate::PitchClass> {
        let written = child(node, |kind| kind == SyntaxKind::PitchClass).map(|child| child.text().to_string());
        let read = written.as_deref().map(str::trim).and_then(crate::PitchClass::parse);
        match read {
            Some(class) => Some(class),
            None => self.refuse(Diagnostic::error(Code::NotAValue, "this tonic cannot be read").at(
                crate::resolve::trimmed_span(node),
                "expected a spelled pitch class such as `bf` or `g#`",
            )),
        }
    }

    // ---- the branching forms ----

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
            return self.quote_chain(origin, subject, &arms);
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

    /// `P { f = e, … }` — a record value, annotated with the type it names.
    ///
    /// The core's records are structural, so the written name is not part of the
    /// value; it is what says *which* record type this is, which is exactly an
    /// annotation. Writing it that way is what makes `01-surface.md` §9's
    /// nominal record a reading rather than a second kind of term.
    fn record(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let mut fields = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FieldInit) {
            let name = own_tokens(&written)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
            let value = child(&written, is_expr_node)?;
            fields.push((name, self.value(&value)?));
        }
        let built = Raw::record(origin, fields.iter().map(|(name, term)| (name.as_str(), term.clone())));
        match child(node, |kind| kind == SyntaxKind::NameExpr) {
            Some(named) => {
                let at = self.origin(&named);
                Some(Raw::annot(origin, built, Raw::var(at, named.to_string().trim())))
            }
            None => Some(built),
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
                .map(|token| token.text().to_owned())
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
    /// The grammar writes a pattern flat — a name, an optional parenthesized run
    /// of names, or a bracketed or parenthesized run — so this reads tokens
    /// rather than recursing, except through a record pattern's own fields.
    pub(crate) fn pattern(&mut self, node: &SyntaxNode) -> Option<RawPattern> {
        let origin = self.origin(node);
        if let Some(fields) = child(node, |kind| kind == SyntaxKind::RecordPattern) {
            return self.record_pattern(&fields, origin);
        }
        if let Some(quoted) = child(node, |kind| kind == SyntaxKind::QuotePattern) {
            return self.not_yet(&quoted, "a quote pattern", "a template");
        }
        let mut tokens = own_tokens(node);
        let head = tokens.next()?;
        let names: Vec<String> = tokens
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_owned())
            .collect();
        let bound = |names: &[String]| -> Vec<RawPattern> {
            names
                .iter()
                .map(|name| RawPattern::bind(origin, name.as_str()))
                .collect()
        };
        Some(match head.kind() {
            SyntaxKind::SomeKw => RawPattern::constructor(origin, "Option.Some", bound(&names)),
            SyntaxKind::NoneKw => RawPattern::constructor(origin, "Option.None", []),
            SyntaxKind::OkKw => RawPattern::constructor(origin, "Result.Ok", bound(&names)),
            SyntaxKind::ErrKw => RawPattern::constructor(origin, "Result.Err", bound(&names)),
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
            SyntaxKind::LBracket => match names.as_slice() {
                [] => RawPattern::constructor(origin, "List.Empty", []),
                [head, tail] => RawPattern::constructor(
                    origin,
                    "List.Cons",
                    [
                        RawPattern::bind(origin, head.as_str()),
                        RawPattern::bind(origin, tail.as_str()),
                    ],
                ),
                // The parser has already complained; a pattern read from a
                // bracket it could not finish would bind names nobody wrote.
                _ => return None,
            },
            // `(m, n)` matches the record a product is written as.
            SyntaxKind::LParen => {
                let fields: Vec<(String, RawPattern)> = names
                    .iter()
                    .enumerate()
                    .map(|(index, name)| (position_field(index), RawPattern::bind(origin, name.as_str())))
                    .collect();
                RawPattern::record(origin, fields.iter().map(|(name, held)| (name.as_str(), held.clone())))
            }
            SyntaxKind::Identifier => {
                if names.is_empty() && !head.text().chars().next().is_some_and(char::is_uppercase) {
                    // `_` is an ordinary binder whose name nothing refers to,
                    // which is what the core's own `RawPattern::Bind` documents.
                    RawPattern::bind(origin, head.text())
                } else {
                    RawPattern::constructor(origin, head.text(), bound(&names))
                }
            }
            _ => return None,
        })
    }

    /// `P { f = g }` — a record pattern binding the fields it names.
    fn record_pattern(&mut self, fields: &SyntaxNode, origin: Origin) -> Option<RawPattern> {
        let mut bound = Vec::new();
        for written in children(fields, |kind| kind == SyntaxKind::FieldPattern) {
            let name = own_tokens(&written)
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
            let held = child(&written, |kind| kind == SyntaxKind::Pattern)?;
            bound.push((name, self.pattern(&held)?));
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

/// `p/q`, as an exact rational.
fn exact(token: &SyntaxToken) -> Option<Ratio<i64>> {
    let (numerator, denominator) = token.text().split_once('/')?;
    let (Ok(numerator), Ok(denominator)) = (numerator.parse::<i64>(), denominator.parse::<i64>()) else {
        return None;
    };
    (denominator != 0).then(|| Ratio::new(numerator, denominator))
}

/// The last identifier a musical literal writes, which is the word naming
/// its collection, chord type, or mode.
fn trailing_word(node: &SyntaxNode) -> String {
    significant_tokens(node)
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .last()
        .map(|token| token.text().to_owned())
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

/// The field name a product's position is written as.
///
/// Leading underscore so that it cannot collide with a field a `record`
/// declaration wrote: `01-surface.md`'s identifiers do not start with one.
pub(super) fn position_field(index: usize) -> String {
    format!("_{index}")
}

/// The trait method an operator token stands for (`10-traits.md` §5).
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
/// The two namespaces `crate::core`'s own `phase_value` reads, and no others: a
/// dotted name outside them is a projection, which is why this answers [`None`]
/// rather than refusing.
fn phase_literal(written: &str) -> Option<musa_core::Literal> {
    let (namespace, case) = written.split_once('.')?;
    match namespace {
        "TokenKind" => crate::syntax::token_kind_named(case).map(crate::registry::token_kind_literal),
        "Delimiter" => crate::syntax::Delimiter::named(case)
            .map(|held| crate::registry::literal(crate::registry::plain_type("Delimiter"), held)),
        _ => None,
    }
}

/// The name a `NameExpr` writes: one identifier, or two joined by a dot.
fn written_name(node: &SyntaxNode) -> Option<String> {
    let mut tokens = significant_tokens(node).filter(|token| !matches!(token.kind(), SyntaxKind::Whitespace));
    let first = tokens.find(|token| token.kind() == SyntaxKind::Identifier)?;
    let mut written = first.text().to_owned();
    if tokens.next().is_some_and(|token| token.kind() == SyntaxKind::Dot)
        && let Some(member) = tokens.next().filter(|token| token.kind() == SyntaxKind::Identifier)
    {
        written.push('.');
        written.push_str(member.text());
    }
    Some(written)
}

/// Where a token was written.
fn token_span(token: &SyntaxToken) -> SourceSpan {
    let range = token.text_range();
    SourceSpan::new(range.start().into(), range.end().into())
}
