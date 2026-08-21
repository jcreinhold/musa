//! Statement lowering: `music`, `motif`, `fragment`, `note`, and the fold that turns written statements into claims.

use musa_calculus::{Origin, Raw};
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::lower::{Lowering, applied, child, is_type_node, listed, whole, writes};
use musa_score::diagnose::{Code, Diagnostic};

use super::*;
impl Lowering<'_> {
    /// `music { … }` — a block of notation, as the track it denotes.
    ///
    /// The entry [`Lowering::value`] calls, and the entry both declarations call.
    /// Reads at [`Reading::free`], because a written `music` expression is a
    /// value and a value has no voice.
    ///
    /// # A block asks nothing
    ///
    /// It used to. Every constructor this module reaches for answered
    /// `Result τ Text`, so the fold was written in the surface's own `?` and
    /// drained at the brace, and a block denoted `Result<EventTrack ⟨written⟩,
    /// Text>` — a type §2 never wrote and no author could use without unwrapping
    /// it first. Prompt 141m gave a δ-rule somewhere to say no, so a fact that
    /// cannot sound refuses the program at its own span instead of answering a
    /// failure nobody can act on, and the constructors went total.
    ///
    /// What is left is the fold itself: a block denotes `EventTrack ⟨written⟩`,
    /// which is what §2's twenty signatures say it denotes. The questions
    /// machinery stays where it belongs, serving a `?` the author wrote.
    pub(crate) fn music(&mut self, node: &SyntaxNode) -> Option<Raw> {
        // The one place a block of notation that is not a voice begins, so the
        // one place that asks for its number: a motif, a fragment, a named bar,
        // and a `music` value all arrive here, and each is a declaration a fact
        // can name.
        let declaration = self.sites.declaring();
        self.notated(node, Reading::free(declaration))
    }

    /// `motif turn(root: Pitch) { … }` — the function §2 says it is.
    ///
    /// "A named `fn turn(...) -> EventTrack[WrittenTime, ScoreFact] { music {
    /// body } }` with a `Motif` role retained for lints, extraction, editing,
    /// and Origin", and the role is `Sites`' already: the block's statements are
    /// numbered from the motif's own nodes, so a fact built here points at the
    /// note inside the motif rather than at the `use` that expanded it.
    ///
    /// # Why no written return type
    ///
    /// §2 spells `-> EventTrack[WrittenTime, ScoreFact]`, and it is now the type
    /// the body has. Spelling it here would still be *checking*, which this
    /// module does not do: the core infers it from the body, which is the one
    /// place that can tell, and a written annotation that agreed with the
    /// inference would be the same fact stated twice.
    ///
    /// # Why the parameters come from the typed AST
    ///
    /// A `fn`'s parameter is a [`SyntaxKind::Param`] node and a motif's is not:
    /// the motif grammar writes the name as a bare token beside a `TypeName`,
    /// which is why [`crate::lower::items::written_parameters`] finds none here.
    /// [`musa_syntax::ast::MotifDecl::params`] is the grammar's own reading of
    /// that shape, so it is used rather than re-derived — and because it yields
    /// names without nodes, each binder is numbered at the declaration. That is
    /// the honest answer: there is no node to point at, and inventing a
    /// plausible one would point a reader at code that is not the cause.
    /// `motif name(param: τ, …) { … }` — the `fn` `01-surface.md` §2 says it
    /// desugars to, so the parameters are [`crate::lower::items::written_parameters`]
    /// like a `fn`'s and each annotation that was written binds: a `d:
    /// Duration<WrittenTime>` the grammar reads is not a spelling the λ drops.
    pub(crate) fn motif(&mut self, node: &SyntaxNode) -> Option<crate::lower::items::Definition> {
        let origin = self.origin(node);
        let name = crate::lower::items::declared_name(node)?;
        let parameters = crate::lower::items::written_parameters(node);
        let mut value = self.music(node)?;
        for parameter in parameters.iter().rev() {
            let at = self.origin(parameter);
            let bound = crate::lower::items::declared_name(parameter)?;
            value = match child(parameter, is_type_node) {
                Some(written) => {
                    let domain = self.ty(&written)?;
                    musa_calculus::Raw::annotated_lam(at, bound, domain, value)
                }
                None => musa_calculus::Raw::lam(at, bound, value),
            };
        }
        Some(crate::lower::items::Definition {
            origin,
            name,
            ty: None,
            value,
        })
    }

    /// `fragment name { … }` — the `let` §2 says it is.
    ///
    /// No parameters, because the grammar gives it none: a fragment that took
    /// one would be a motif, which is the distinction §2 draws by giving them
    /// two words.
    pub(crate) fn fragment(&mut self, node: &SyntaxNode) -> Option<crate::lower::items::Definition> {
        Some(crate::lower::items::Definition {
            origin: self.origin(node),
            name: crate::lower::items::declared_name(node)?,
            ty: None,
            value: self.music(node)?,
        })
    }

    /// `bar refrain { … }` — the same `let` a fragment is, for the bar that has
    /// a name.
    ///
    /// A named bar is a declaration written where it sounds. The braces sound in
    /// place, which is [`Lowering::bar`]'s reading and states the measure claim;
    /// the name binds the same passage for every `use` that answers it. The
    /// caller has already read the name off the statement and hands it over —
    /// there is nothing here to re-derive from the tokens.
    ///
    /// # Why the passage is read a second time rather than shared
    ///
    /// The two readings are at two scopes. What sounds between the braces is the
    /// *voice's*, read under the meter and collection in force where it stands;
    /// what a `use` plays is material relabelled to wherever it is played, which
    /// is [`Lowering::music`]'s `Reading::free` and the same reading a fragment
    /// gets. Sharing one term would have to pick one of them, and either choice
    /// puts a fact in a scope its author did not write.
    ///
    /// # Why it is a definition and not a binder over the rest of the voice
    ///
    /// A binder introduced inside the fold is invisible to the *claims* the fold
    /// raises. A [`Claimed`] holds `before` and `passage` as terms that
    /// [`crate::document::Document::passage`] elaborates in the document's own
    /// context, so a claim written after `use refrain;` — which
    /// `examples/refrain.musa` writes — would carry a free `refrain` into a
    /// context that never bound it. Making the bar a declaration is what the
    /// replaced core did, and it is the reading under which one name-resolution
    /// mechanism answers every use of the name, claims included.
    pub(crate) fn named_bar(&mut self, node: &SyntaxNode, name: &str) -> Option<crate::lower::items::Definition> {
        Some(crate::lower::items::Definition {
            origin: self.origin(node),
            name: musa_calculus::Name::from(name),
            ty: None,
            value: self.music(node)?,
        })
    }

    /// The left fold of `node`'s statements, seeded with `nothing`.
    ///
    /// Every statement is read even after one is refused, so a block with three
    /// bad statements reports three diagnostics rather than the first. The
    /// answer is [`None`] if any of them was refused, because a fold that
    /// quietly dropped a statement would answer a *different piece of music*
    /// than the one written.
    ///
    /// A claim raised while a statement is read is placed relative to *this*
    /// block, so every claim that came out of that statement has the music
    /// standing before it prepended before the fold moves on. One level of that
    /// at each nesting is what makes [`Claimed::before`] absolute by the time a
    /// voice is finished, without any block having to know where it stands.
    pub(crate) fn notated(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        self.folded(node, statements(node), reading)
    }

    /// The same fold over a chosen subsequence of `node`'s statements.
    ///
    /// One caller passes anything but every statement: [`Lowering::repeat`]
    /// leaves the endings out of the body, because a volta is not part of what
    /// the passes have in common. `node` is still handed over, because the
    /// origin a claim is placed against belongs to the block rather than to the
    /// statements that survived a filter.
    pub(crate) fn folded(
        &mut self,
        node: &SyntaxNode,
        statements: impl Iterator<Item = SyntaxNode>,
        reading: Reading,
    ) -> Option<Raw> {
        let origin = self.origin(node);
        let mut placed = Placed::default();
        let mut whole = true;
        let mut reading = reading;
        for statement in statements {
            // A `meter` is in force from where it is written, so it is read
            // *before* the statement that wrote it is folded and stays in force
            // for everything after — which is the whole of what makes the
            // restoring half of `senza` the meter a composer expects.
            if let Some(written) = musa_syntax::ast::MeterStmt::cast(statement.clone())
                && let Some(meter) = crate::resolve::parse_meter(&written)
            {
                reading = reading.metered(meter);
            }
            // A `key` is read the same way and for the same reason: it is in
            // force from where it is written, and what it puts in force for a
            // `step` is the collection it suggests. Two statements read
            // lexically, and both write only forward.
            if let Some(written) = musa_syntax::ast::KeyStmt::cast(statement.clone())
                && let Some(key) = crate::resolve::parse_key(&written)
            {
                reading = reading.keyed(key);
            }
            let raised = self.claims.len();
            match self.statement(&statement, reading) {
                Some(next) => {
                    // Asked for only when a claim was raised: the prefix is a
                    // term of its own, and building one per statement would
                    // spend nodes on blocks that claim nothing.
                    if self.claims.len() > raised {
                        let before = placed.built(origin);
                        for claim in self.claims.iter_mut().skip(raised) {
                            let inside = claim.before.clone();
                            claim.before = applied(origin, Raw::hosted(origin, "follow"), [before.clone(), inside]);
                        }
                    }
                    placed.place(origin, next);
                }
                None => whole = false,
            }
        }
        whole.then(|| placed.built(origin))
    }

    /// `node`'s body under one transformation, and every claim raised inside it
    /// under the same one.
    ///
    /// A claim is about the passage *as instantiated* (`05-verification.md`), and
    /// a transformation block is part of how it was instantiated:
    /// `transpose up m2 { assert pitches_in(scale c major) { … } }` claims about
    /// the transposed notes, which is what makes one motif under two
    /// transpositions two verdicts. [`Claimed`] holds terms rather than notes —
    /// the notes do not exist until the document is elaborated — so the
    /// transformation is applied to the *terms*, here, where the reading still
    /// knows which one it is.
    ///
    /// Both of a claim's terms travel, for two reasons that happen to agree.
    /// `passage` is the music the claim is about, so a `transpose` changes what
    /// sounds in it; `before` is read only for its duration, so a `stretch`
    /// changes where the passage begins. [`Lowering::folded`] threads the
    /// *prefix* into `before` for the same reason this threads the
    /// *transformation* into both, and the two compose in written order — the
    /// fold prepends outside whatever this wrapped inside.
    ///
    /// The four transformation blocks are the whole of it, because they are the
    /// four statements that change what their body sounds. `in scale`, `senza`,
    /// a region, and a repeat each enclose a body and leave what is inside the
    /// braces sounding exactly as written, so a claim under one of those is
    /// already about the right music.
    pub(crate) fn transforming(
        &mut self,
        node: &SyntaxNode,
        reading: Reading,
        under: impl Fn(Raw) -> Raw,
    ) -> Option<Raw> {
        let raised = self.claims.len();
        let body = self.notated(node, reading)?;
        for claim in self.claims.iter_mut().skip(raised) {
            claim.before = under(claim.before.clone());
            claim.passage = under(claim.passage.clone());
        }
        Some(under(body))
    }

    /// One notation statement, as the track it denotes.
    ///
    /// The table this module exists for. Every arm answers a track; the ones
    /// that refuse say why at the node that caused it, and the two kinds of
    /// refusal are deliberately different words — see [`Lowering::misplaced`].
    pub(crate) fn statement(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        let origin = self.origin(node);
        let span = crate::resolve::trimmed_span(node);
        match node.kind() {
            SyntaxKind::NoteStmt => self.note(node, origin, reading),
            SyntaxKind::RestStmt => {
                let (field, held, free) = self.notated_duration(node, span, reading)?;
                let fact = applied(
                    origin,
                    Raw::hosted(origin, "Fact.Rest"),
                    [
                        field,
                        listed(origin, Vec::new()),
                        optional(origin, "FreeDuration", free),
                    ],
                );
                Some(self.sounded_term(origin, reading, fact, held))
            }
            SyntaxKind::ChordStmt => self.chord_statement(node, origin, reading),
            SyntaxKind::StackStmt => self.stack(node, origin, reading),
            SyntaxKind::GraceStmt => self.grace(node, origin, reading),
            SyntaxKind::UseStmt => self.used(node, origin, reading),

            // The transformation blocks: each is one track builtin applied to
            // the fold of its body, which is the whole of §3's "the function and
            // block spellings invoke the same semantic action".
            SyntaxKind::TransposeStmt => {
                let text = musa_syntax::ast::TransposeStmt::cast(node.clone())
                    .and_then(|stmt| stmt.interval())
                    .unwrap_or_default();
                let Some(interval) = musa_score::Interval::parse(&text, writes(node, SyntaxKind::DownKw)) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                            .at(span, "unknown interval")
                            .note("a quality and a number: `P5`, `M3`, `m6`, `A4`, `d5`"),
                    );
                };
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "transpose"),
                        [plain(origin, "Interval", interval), body],
                    )
                })
            }
            SyntaxKind::StretchStmt => {
                let text = musa_syntax::ast::StretchStmt::cast(node.clone())
                    .and_then(|stmt| stmt.factor())
                    .unwrap_or_default();
                let factor = crate::resolve::parse_ratio(&text)
                    .or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer))
                    .filter(|factor| *factor > Ratio::ZERO);
                let Some(factor) = factor else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a stretch factor"))
                            .at(span, "expected a positive number, like `2` or `3/2`"),
                    );
                };
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "stretch"),
                        [plain(origin, "Ratio", factor), body],
                    )
                })
            }
            SyntaxKind::RetrogradeStmt => self.transforming(node, reading, |body| {
                Raw::app(origin, Raw::hosted(origin, "retrograde"), body)
            }),
            SyntaxKind::InvertStmt => {
                let text = musa_syntax::ast::InvertStmt::cast(node.clone())
                    .and_then(|stmt| stmt.axis())
                    .unwrap_or_default();
                let Some(axis) = musa_score::WrittenPitch::parse(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
                            .at(span, "inversion needs a pitch to mirror about"),
                    );
                };
                self.transforming(node, reading, |body| {
                    applied(
                        origin,
                        Raw::hosted(origin, "invert"),
                        [plain(origin, "Pitch", axis), body],
                    )
                })
            }

            // `in scale` changes what a `step` reads and denotes its body. It is
            // the one statement that contributes no fact of its own — and the
            // one that still has to *say* it was here, because a spelling like
            // `eb4` is the same written pitch whether the source wrote it or a
            // step arrived at it, and Origin view is where a reader asks which.
            SyntaxKind::InScaleStmt => {
                let Some(written) = musa_syntax::ast::InScaleStmt::cast(node.clone()).and_then(|s| s.scale_expr())
                else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "`in scale` needs a scale")
                            .at(span, "expected a scale, such as `c dorian`"),
                    );
                };
                let scale = self.counting(&written)?;
                let body = self.notated(node, reading.stepping(scale))?;
                Some(Self::under_scale(node, origin, &written, body))
            }

            // The region annotations: a fact over the span its body covers.
            SyntaxKind::SlurStmt => self.region(node, origin, reading, Raw::hosted(origin, "Fact.Slur")),
            SyntaxKind::PhraseStmt => {
                let name = musa_syntax::ast::PhraseStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Phrase"), plain(origin, "Text", name));
                self.region(node, origin, reading, fact)
            }
            SyntaxKind::TupletStmt => {
                let text = musa_syntax::ast::TupletStmt::cast(node.clone())
                    .and_then(|stmt| stmt.ratio())
                    .unwrap_or_default();
                let Some((num, den)) = tuplet_ratio(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a tuplet ratio"))
                            .at(span, "expected something like `3/2`"),
                    );
                };
                let fact = applied(
                    origin,
                    Raw::hosted(origin, "Fact.Tuplet"),
                    [whole(origin, u64::from(num)), whole(origin, u64::from(den))],
                );
                // The one region that changes what its body *means* rather than
                // only annotating it: `tuplet 3/2` is three in the time of two,
                // so an eighth written inside lasts a twelfth. The factor is put
                // in the reading rather than applied to the finished track,
                // because a track has no duration to rescale — the values are
                // already literals in the facts by then — and because reading it
                // lexically is what makes tuplets nest without either statement
                // knowing about the other.
                self.region(node, origin, reading.inside(tuplet_factor(node)), fact)
            }
            SyntaxKind::HairpinStmt => {
                let statement = musa_syntax::ast::HairpinStmt::cast(node.clone())?;
                let text = statement.target().unwrap_or_default();
                let Some(target) = musa_score::score::DynamicMark::parse(&text) else {
                    return self.refuse(Self::not_a_dynamic(&text, span));
                };
                let grows = if statement.grows() { "Bool.True" } else { "Bool.False" };
                let fact = applied(
                    origin,
                    Raw::hosted(origin, "Fact.Hairpin"),
                    [
                        Raw::var(origin, grows),
                        payload(origin, "DynamicMark", target),
                        // The grammar writes `cres.`/`dim.` and nothing about
                        // shape, so every hairpin is a straight line — the same
                        // value the old checker put in the timeline, put in the
                        // term instead of invented downstream.
                        payload(origin, "Progress", musa_events::Progress::linear()),
                    ],
                );
                self.region(node, origin, reading, fact)
            }
            SyntaxKind::RepeatStmt => self.repeat(node, origin, reading),
            SyntaxKind::EndingStmt => self.ending(node, origin, reading),
            SyntaxKind::MobileStmt => self.mobile(node, origin, reading),
            SyntaxKind::ImproviseStmt => self.improvise(node, origin, reading),

            // The point annotations: a fact at the instant it is written.
            SyntaxKind::DynamicStmt => {
                let text = musa_syntax::ast::DynamicStmt::cast(node.clone())
                    .and_then(|stmt| stmt.mark())
                    .unwrap_or_default();
                let Some(mark) = musa_score::score::DynamicMark::parse(&text) else {
                    return self.refuse(Self::not_a_dynamic(&text, span));
                };
                let fact = Raw::app(
                    origin,
                    Raw::hosted(origin, "Fact.Dynamic"),
                    payload(origin, "DynamicMark", mark),
                );
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::SectionStmt => {
                let name = musa_syntax::ast::SectionStmt::cast(node.clone())
                    .and_then(|stmt| stmt.name())
                    .unwrap_or_default();
                let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Section"), plain(origin, "Text", name));
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::HarmonyStmt => {
                let text = musa_syntax::ast::HarmonyStmt::cast(node.clone())
                    .and_then(|stmt| stmt.symbol())
                    .map(|symbol| symbol.text())
                    .unwrap_or_default();
                let Some(symbol) = musa_score::harmony::ChordSymbol::parse(&text) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a chord symbol"))
                            .at(span, "expected something like `Cmaj7` or `F#m7b5`"),
                    );
                };
                let fact = Raw::app(
                    origin,
                    Raw::hosted(origin, "Fact.Harmony"),
                    payload(origin, "ChordSymbol", symbol),
                );
                Some(self.sounded(origin, reading, fact, Ratio::ZERO))
            }
            SyntaxKind::MarkStmt => self.marked(node, origin, reading),

            // §3's own list: "from here onward" has no unique meaning in a value
            // usable at several places — so each of the four is refused where
            // there is no single here, and is a point where there is one.
            SyntaxKind::KeyStmt => self.context(node, origin, reading, span, Context::Key),
            SyntaxKind::MeterStmt => self.context(node, origin, reading, span, Context::Meter),
            SyntaxKind::TempoStmt => self.context(node, origin, reading, span, Context::Tempo),
            SyntaxKind::ClefStmt => self.context(node, origin, reading, span, Context::Clef),

            SyntaxKind::BarStmt => self.bar(node, origin, reading),
            SyntaxKind::SenzaStmt => self.senza(node, origin, reading, span),

            SyntaxKind::AssertStmt => self.asserted(node, origin, reading),
            _ => None,
        }
    }

    /// `c5/4`, `(c5 step 2)/8 staccato`, `g4/4 to 2/1 ~`.
    ///
    /// One `play` of a one-note voicing rather than a `sounded` of a `Fact.Note`,
    /// for the reason 141j's Design gives: `play` is what turns written pitches
    /// into `Note` facts, and a note is a chord of one.
    pub(crate) fn note(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::NoteStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let pitch = self.pitch_term(&statement, node, origin, reading)?;
        let (field, held, free) = self.notated_duration(node, span, reading)?;
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Note"),
            [
                pitch,
                field,
                self.articulations(origin, &statement.articulations(), span),
                optional(origin, "FreeDuration", free),
            ],
        );
        Some(continuing(
            origin,
            statement.tied(),
            self.sounded_term(origin, reading, fact, held),
        ))
    }

    /// `[c4 e4 g4]/2` — the written pitches sounding together.
    ///
    /// Every pitch is its own `Note` fact over the same span, which is what a
    /// chord *is* at this layer (§3's `map_note_pitches` clause says so from the
    /// other side). Read as one `sounded` per pitch folded with `together`, and
    /// not as a `play`, because `play` takes a `Voicing`, a `Voicing` takes a
    /// `ChordClass`, and a written simultaneity is not required to spell one:
    /// `[c4 c#4]` names no chord class and is still a sounding. `stack` is the
    /// statement that names one.
    pub(crate) fn chord_statement(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::ChordStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (field, held, free) = self.notated_duration(node, span, reading)?;
        let articulations = self.articulations(origin, &statement.articulations(), span);
        let written = statement.pitches();
        if written.is_empty() {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a chord with no notes is a rest")
                    .at(span, "no pitches inside these brackets")
                    .help("write `rest` for silence, or the pitches this chord sounds"),
            );
        }
        let mut sounding = None;
        for text in written {
            let Some(pitch) = musa_score::WrittenPitch::parse(&text) else {
                return self.refuse(Self::not_a_pitch(&text, span));
            };
            let fact = applied(
                origin,
                Raw::hosted(origin, "Fact.Note"),
                [
                    plain(origin, "Pitch", pitch),
                    field.clone(),
                    articulations.clone(),
                    optional(origin, "FreeDuration", free),
                ],
            );
            let one = self.sounded_term(origin, reading, fact, held.clone());
            sounding = Some(match sounding {
                None => one,
                Some(built) => applied(origin, Raw::hosted(origin, "together"), [built, one]),
            });
        }
        // Outside the `together` and not on each note, because a chord's `~` is
        // written once and is about the chord: marking the notes separately
        // would say the same thing as many times as there are pitches.
        sounding.map(|track| continuing(origin, statement.tied(), track))
    }

    /// `stack c4 major7/2` — a chord class voiced from a written bass.
    ///
    /// The one statement that reaches `play`'s `Voicing` argument, because it is
    /// the one that names a chord class: `[c4 e4 g4]` is three pitches and
    /// `stack c4 major/2` is a C major triad voiced upward from `c4`.
    pub(crate) fn stack(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::StackStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let text = statement.root().unwrap_or_default();
        let Some(bass) = musa_score::WrittenPitch::parse(&text) else {
            // A pitch class is refused with its own sentence rather than the
            // generic one. `stack c major7` is not a typo for a pitch: it names
            // a class, and the answer is that stacking sounds notes and a class
            // chooses no octave. Supplying one would be Musa deciding a
            // register the composer did not write.
            if statement.root_is_class() {
                return self.refuse(
                    Diagnostic::error(Code::NotAValue, "a stacked chord needs a register")
                        .at(span, "expected a written pitch here")
                        .note("`stack c4 major7/2` sounds notes, and a pitch class chooses no octave"),
                );
            }
            return self.refuse(Self::not_a_pitch(&text, span));
        };
        let word = statement.chord_type().unwrap_or_default();
        let Some(kind) = musa_score::chord::ChordType::named(&word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown chord type `{word}`"))
                    .at(span, "not a named chord type")
                    .note("a chord type is the content; the symbol written above the staff is a separate annotation"),
            );
        };
        let (_, held, _) = self.notated_duration(node, span, reading)?;
        let class = musa_score::chord::ChordClass::new(bass.pitch_class(), kind);
        let Ok(voicing) = musa_score::chord::Voicing::close_position(class, bass) else {
            return self.refuse(
                Diagnostic::error(Code::OutOfRange, "this chord does not stack above that bass")
                    .at(span, "the written coordinates leave Musa's exact range"),
            );
        };
        let call = applied(
            origin,
            Raw::hosted(origin, "play"),
            [
                self.provenance(origin, span, reading.placed, reading.declaration),
                scope_of(origin, reading.scope),
                // `plain` and not `payload`: `play` reads a `Voicing` and not an
                // `Opaque<Voicing>`, and a literal at the wrong Rust type
                // downcasts to nothing, which the core reports as this
                // compiler's table disagreeing with itself.
                plain(origin, "Voicing", voicing),
                held,
            ],
        );
        Some(call)
    }

    /// `grace { c5 d5 }` — the notes crushed before the one they lean on.
    ///
    /// Each grace note is a *point* `Fact.Grace` carrying its index, which is
    /// load-bearing rather than decorative: every grace note in one group shares
    /// a start and an end, so without the index `grace { c5 d5 }` and
    /// `grace { d5 c5 }` normalize to the same timeline.
    pub(crate) fn grace(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let span = crate::resolve::trimmed_span(node);
        let mut built = Raw::lit(origin, crate::registry::empty_track());
        for (index, note) in musa_syntax::ast::GraceStmt::cast(node.clone())?
            .notes()
            .into_iter()
            .enumerate()
        {
            let text = note.pitch().unwrap_or_default();
            let Some(pitch) = musa_score::WrittenPitch::parse(&text) else {
                return self.refuse(Self::not_a_pitch(&text, span));
            };
            let at = self.origin(note.syntax());
            let fact = applied(
                at,
                Raw::hosted(at, "Fact.Grace"),
                [
                    plain(at, "Pitch", pitch),
                    self.articulations(at, &note.articulations(), span),
                    whole(at, index as u64),
                ],
            );
            let one = self.sounded(at, reading, fact, Ratio::ZERO);
            built = applied(origin, Raw::hosted(origin, "follow"), [built, one]);
        }
        Some(built)
    }
}
