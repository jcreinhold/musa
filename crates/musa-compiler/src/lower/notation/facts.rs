//! Facts: `context`, `fact`, `region`, `notated_duration`, `held`, the pitch and interval readings, and the written scale.

use musa_calculus::{Origin, Raw};
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::lower::{Lowering, applied, child, children, is_expr_node, writes};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::score::NotatedDuration;

use super::*;
impl Lowering<'_> {
    /// One of the four statements whose meaning is "from here onward".
    ///
    /// Refused where there is no single here — a `music` value is "usable at
    /// several places" — and a **point** where there is one, which is what the
    /// replaced path made of a context statement written among a voice's items
    /// too. A point rather than a region because a left fold cannot see what
    /// comes after the statement it is reading, and because "from here onward"
    /// is answered by the context rules in [`musa_score::scope`] reading the
    /// occurrences in order rather than by the extent written on any one of them.
    pub(crate) fn context(
        &mut self,
        node: &SyntaxNode,
        origin: Origin,
        reading: Reading,
        span: SourceSpan,
        which: Context,
    ) -> Option<Raw> {
        if !reading.permanent() {
            return self.misplaced(which.what(), span);
        }
        // Where the *fact* belongs, which is not where it was written: a key,
        // a meter, and a tempo are things the piece does, and a clef is one
        // player's staff. Both rules are `elaborate.rs`'s, unchanged.
        let scope = match which {
            Context::Key | Context::Meter | Context::Tempo => musa_score::Scope::Piece,
            Context::Clef => match reading.scope {
                musa_score::Scope::Part { part } | musa_score::Scope::Voice { part, .. } => {
                    musa_score::Scope::Part { part }
                }
                musa_score::Scope::Piece => {
                    return self.refuse(
                        Diagnostic::error(Code::Misplaced, "a clef change belongs to a part")
                            .at(span, "written outside any part")
                            .help("write it in the part, or in a voice of that part")
                            .note("a clef is one player's staff, and the piece as a whole is read on none"),
                    );
                }
            },
        };
        let fact = self.fact(node, origin, span, which)?;
        Some(self.sounded_at(origin, scope, reading.placed, reading.declaration, fact, Ratio::ZERO))
    }

    /// The `Fact` one of the four states, without the placement around it.
    ///
    /// Shared with [`super::piece`], which writes the same four facts over the
    /// region a header covers rather than at the point a body reached. Reading
    /// them twice would be two answers to what `key g major;` means.
    pub(crate) fn fact(&mut self, node: &SyntaxNode, origin: Origin, span: SourceSpan, which: Context) -> Option<Raw> {
        match which {
            Context::Key => {
                let statement = musa_syntax::ast::KeyStmt::cast(node.clone())?;
                // `key k;` names a key rather than spelling one, and the parser
                // wrote the name as an expression child for exactly this
                // reading. The ordinary value reading answers it, so a key a
                // template was handed reaches `Fact.Key` the same way a written
                // one does.
                if let Some(written) = child(node, is_expr_node) {
                    let named = self.value(&written)?;
                    return Some(Raw::app(origin, Raw::hosted(origin, "Fact.Key"), named));
                }
                let Some(key) = crate::resolve::parse_key(&statement) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "this key cannot be read")
                            .at(span, "expected a note and a mode, like `a minor`"),
                    );
                };
                Some(keyed(origin, key))
            }
            Context::Meter => {
                let statement = musa_syntax::ast::MeterStmt::cast(node.clone())?;
                let Some(meter) = crate::resolve::parse_meter(&statement) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, "this meter cannot be read")
                            .at(span, "expected `4/4`, or `none`"),
                    );
                };
                Some(metered(origin, meter))
            }
            Context::Tempo => {
                let statement = musa_syntax::ast::TempoStmt::cast(node.clone())?;
                let marking = crate::resolve::tempo_marking(self.resolver, &statement);
                Some(tempo(origin, &marking))
            }
            Context::Clef => {
                let written = musa_syntax::ast::ClefStmt::cast(node.clone())
                    .and_then(|statement| statement.name())
                    .unwrap_or_default();
                let Some(clef) = musa_score::score::Clef::parse(&written) else {
                    return self.refuse(
                        Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a clef"))
                            .at(span, "not a clef musa reads")
                            .help(crate::resolve::suggest(
                                &written,
                                musa_score::score::Clef::NAMES,
                                "clefs",
                            )),
                    );
                };
                Some(clefed(origin, clef))
            }
        }
    }

    /// A fact over the region its body covers, and the body under it.
    ///
    /// `together(sounded(…, duration(body)), body)` is what "over the region it
    /// spans" means with no `duration` query available: the fold already knows
    /// what it built, so the extent is computed here and written as a literal
    /// rather than asked of the term. That is 141j's argument for registering
    /// `follow` rather than `duration`, used.
    ///
    /// `inside` is the reading the *body* is read under, which for every region
    /// but a tuplet is the reading the region itself stands in. The marker is
    /// built under it too, because the one thing [`Lowering::sounded`] takes from
    /// a reading is its scope and a region never changes that. What the marker
    /// spans is [`Lowering::lasts`] rather than [`Lowering::extent`], because a
    /// tuplet's own ratio is part of how long the *statement* lasts and no part
    /// of how long its body reaches.
    pub(crate) fn region(&mut self, node: &SyntaxNode, origin: Origin, inside: Reading, fact: Raw) -> Option<Raw> {
        let body = self.notated(node, inside)?;
        let over = self.lasts(node);
        let marker = self.sounded(origin, inside, fact, over);
        Some(applied(origin, Raw::hosted(origin, "together"), [marker, body]))
    }

    /// A written duration and the freedom written on it, as long as it lasts
    /// where it stands.
    ///
    /// The reading is read last rather than first: [`Lowering::held`] decides
    /// how far a `to` is taken in the time the composer *wrote*, on the
    /// realization's own sixteenth-note grid, and [`Reading::lasting`] then
    /// carries the decided value into the tuplet's time. Deciding first and
    /// scaling after is what keeps [`Lowering::lasts`] a measurement of the
    /// written tree plus the decisions already recorded, rather than one that
    /// depends on a tuplet's body having been lowered before the region above it
    /// asks how far it reaches.
    /// A statement's duration as the two *terms* the fact and the span need:
    /// the `NotatedDuration` field of the fact, and the `Duration ⟨written⟩`
    /// the fact sounds for. A written fraction bakes both in as literals; a
    /// parameter is a term already, so the field goes through the
    /// `notated_duration` δ word, which evaluation fires once the call has
    /// given the parameter its value.
    ///
    /// The pair is read here rather than at each caller for
    /// [`NotatedDuration::spelled`]'s reason: one duration has one spelling,
    /// and six statements reading one duration six ways is six spellings of
    /// it.
    ///
    /// Inside a tuplet a parameter's term is scaled the way
    /// [`Reading::lasting`] scales a written one, and the spelling the rule
    /// then derives is of the *sounding* value: a computed duration has no
    /// written symbol for the annotation to point back at, which is the one
    /// asymmetry with the written case a parameter cannot avoid.
    pub(crate) fn notated_duration(
        &mut self,
        node: &SyntaxNode,
        span: SourceSpan,
        reading: Reading,
    ) -> Option<(Raw, Raw, Option<musa_score::score::FreeDuration>)> {
        let origin = self.origin(node);
        let Some(duration) = crate::resolve::parse_duration(node) else {
            if let Some(parameter) = musa_syntax::ast::Duration::of(node).and_then(|written| written.parameter()) {
                let term = if reading.tuplet == Ratio::ONE {
                    Raw::var(origin, parameter)
                } else {
                    applied(
                        origin,
                        Raw::hosted(origin, "duration_scale"),
                        [Raw::var(origin, parameter), plain(origin, "Ratio", reading.tuplet)],
                    )
                };
                let field = applied(origin, Raw::hosted(origin, "notated_duration"), [term.clone()]);
                return Some((field, term, None));
            }
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "this statement has no duration")
                    .at(span, "expected a duration")
                    .note("a duration is a fraction or a whole number of whole notes: `1/4`, `3/8`, `1`"),
            );
        };
        let (duration, free) = match musa_syntax::ast::Duration::of(node).and_then(|written| written.held_to()) {
            None => reading.lasting((duration, None)),
            Some(most) => reading.lasting(self.held(duration, &most, span)?),
        };
        let held = written_duration(origin, duration.value.as_ratio());
        Some((payload(origin, "NotatedDuration", duration), held, free))
    }

    /// `g4/4 to 2/1` — a quarter the performer may hold to a double whole.
    ///
    /// Roadmap §2's row with both values kept rather than one standing in for
    /// the other: what comes back as the duration is what the note *sounds*, so
    /// everything after it lands where it should, and the
    /// [`musa_score::score::FreeDuration`] beside it is what recovers the symbol the
    /// engraver draws.
    ///
    /// The decided length is remembered under `span` for the same reason
    /// [`Lowering::passes`] remembers a count: [`Lowering::lasts`] asks a second
    /// time when an enclosing region measures how far its body reaches, and
    /// asking the realization again would mint a second site.
    pub(crate) fn held(
        &mut self,
        duration: NotatedDuration,
        most: &str,
        span: SourceSpan,
    ) -> Option<(NotatedDuration, Option<musa_score::score::FreeDuration>)> {
        let Some(written) = crate::resolve::parse_ratio(most) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{most}` is not a duration"))
                    .at(span, "expected the longest this note may be held")
                    .help("write a duration such as `2/1`"),
            );
        };
        let written = musa_score::MusicalDuration::new(written);
        if written.as_ratio() < duration.value.as_ratio() {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a held note counts upwards")
                    .at(span, format!("`{}` is longer than `{most}`", duration.spelling))
                    .help("write the written value first and the longest hold second"),
            );
        }
        let least = duration.value;
        let sounds = self
            .resolver
            .decide_duration(&self.choice, least.as_ratio(), written.as_ratio(), span);
        self.holds.insert(span, sounds);
        Some((
            NotatedDuration {
                value: musa_score::MusicalDuration::new(sounds),
                spelling: duration.spelling,
                pieces: vec![musa_score::MusicalDuration::new(sounds)],
            },
            Some(musa_score::score::FreeDuration { least, most: written }),
        ))
    }

    /// The `Pitch` a note statement sounds, as a term.
    ///
    /// A spelled pitch folds to a literal, which is what lets the fold finish a
    /// note into a value; a *named* one does not, because a parameter has no
    /// value until an instance site supplies one. Both answer a `Raw` at
    /// `Pitch`, so the `Fact.Note` written around them is one expression rather
    /// than two — what the second costs is that the `sounded` enclosing it stays
    /// a neutral term until the site applies it, which is the ordinary behaviour
    /// of a builtin under an unapplied binder (`02-core-calculus.md` §5.8).
    ///
    /// Which of the two a statement wrote is the parser's answer and not a
    /// second grammar here: `c4` lexes as a pitch literal and `root` as an
    /// identifier, and the statement's own tokens say which is there.
    pub(crate) fn pitch_term(
        &mut self,
        statement: &musa_syntax::ast::NoteStmt,
        node: &SyntaxNode,
        origin: Origin,
        reading: Reading,
    ) -> Option<Raw> {
        if let Some(written) = statement.pitch_expr() {
            return self.pitch_of(&written, reading);
        }
        let text = statement.pitch().unwrap_or_default();
        if writes(node, SyntaxKind::Identifier) {
            return Some(Raw::var(origin, text.as_str()));
        }
        let Some(pitch) = musa_score::WrittenPitch::parse(&text) else {
            return self.refuse(Self::not_a_pitch(&text, crate::resolve::trimmed_span(node)));
        };
        Some(plain(origin, "Pitch", pitch))
    }

    /// A pitch expression as a term, whatever it names.
    ///
    /// `up`/`down` becomes `pitch_transposed`, which reduces to a literal when
    /// both arguments are ones and stays a neutral spine when either is a
    /// binder — so one reading serves `c5/4` and `(root up M2)/4` and the fold
    /// never asks which it got. `step` is the exception: no registered
    /// operation walks a frame of a collection, so it still folds here and
    /// still needs a base it can read.
    pub(crate) fn pitch_of(&mut self, node: &SyntaxNode, reading: Reading) -> Option<Raw> {
        let origin = self.origin(node);
        match node.kind() {
            SyntaxKind::ParenExpr | SyntaxKind::BlockExpr => {
                let inner = child(node, is_expr_node)?;
                self.pitch_of(&inner, reading)
            }
            SyntaxKind::NameExpr | SyntaxKind::PathExpr => self.value(node),
            SyntaxKind::PitchExpr => {
                let parts = children(node, is_expr_node);
                let base = self.pitch_of(parts.first()?, reading)?;
                let interval = self.interval_of(parts.get(1)?, writes(node, SyntaxKind::DownKw))?;
                Some(applied(
                    origin,
                    Raw::hosted(origin, "pitch_transposed"),
                    [base, interval],
                ))
            }
            _ => {
                let pitch = self.written_pitch(node, reading)?;
                Some(plain(origin, "Pitch", pitch))
            }
        }
    }

    /// The interval a transposition moves by, as a term.
    ///
    /// A named one is inverted by `interval_inverse` rather than by parsing the
    /// spelling with a sign, because `down` is a direction the source wrote and
    /// the value it applies to may not arrive until an instance site.
    pub(crate) fn interval_of(&mut self, node: &SyntaxNode, down: bool) -> Option<Raw> {
        let origin = self.origin(node);
        if named(node) {
            let held = self.value(node)?;
            return Some(if down {
                Raw::app(origin, Raw::hosted(origin, "interval_inverse"), held)
            } else {
                held
            });
        }
        let text = node.to_string().trim().to_owned();
        let Some(interval) = musa_score::Interval::parse(&text, down) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                    .at(crate::resolve::trimmed_span(node), "unknown interval"),
            );
        };
        Some(plain(origin, "Interval", interval))
    }

    /// A written pitch, finished under the scale in force.
    ///
    /// The one place `in scale` is read. `p step n` needs a scale and says so
    /// when there is none; `p up M3` does not and never asks.
    pub(crate) fn written_pitch(&mut self, node: &SyntaxNode, reading: Reading) -> Option<musa_score::WrittenPitch> {
        let span = crate::resolve::trimmed_span(node);
        match node.kind() {
            SyntaxKind::ParenExpr | SyntaxKind::BlockExpr => {
                let inner = child(node, is_expr_node)?;
                self.written_pitch(&inner, reading)
            }
            SyntaxKind::LiteralExpr | SyntaxKind::NameExpr => {
                let text = node.to_string().trim().to_owned();
                musa_score::WrittenPitch::parse(&text).or_else(|| self.refuse(Self::not_a_pitch(&text, span)))
            }
            SyntaxKind::PitchExpr => {
                let parts = children(node, is_expr_node);
                let base = self.written_pitch(parts.first()?, reading)?;
                let text = parts.get(1)?.to_string().trim().to_owned();
                let Some(interval) = musa_score::Interval::parse(&text, writes(node, SyntaxKind::DownKw)) else {
                    return self.refuse(
                        Diagnostic::error(Code::NotAValue, format!("`{text}` is not an interval"))
                            .at(span, "unknown interval"),
                    );
                };
                base.transpose(interval).or_else(|| self.out_of_range(span))
            }
            SyntaxKind::StepExpr => {
                let parts = children(node, is_expr_node);
                let base = self.written_pitch(parts.first()?, reading)?;
                let written = parts.get(1)?.to_string().trim().to_owned();
                let Ok(count) = written.parse::<i64>() else {
                    return self.refuse(Self::not_a_count(&written, span));
                };
                let steps = if writes(node, SyntaxKind::DownKw) {
                    let Some(down) = count.checked_neg() else {
                        return self.out_of_range(span);
                    };
                    down
                } else {
                    count
                };
                let scale = match reading.scale {
                    Some(Counting::Written(scale)) => scale,
                    Some(Counting::Bound) => {
                        return self.refuse(
                            Diagnostic::error(Code::UnsupportedLanguageStage, "`step` needs a scale it can count")
                                .at(span, "the enclosing `in scale` names a scale rather than spelling one")
                                .help("write the collection out, as `in scale c major { … }`")
                                .note(
                                    "a step walks a frame of the collection around this pitch, and no registered \
                                     operation builds one",
                                ),
                        );
                    }
                    None => {
                        return self.refuse(
                            Diagnostic::error(Code::Misplaced, "`step` needs a scale to count in")
                                .at(span, "no `in scale` encloses this")
                                .help("put the passage in `in scale c major { … }`, naming the collection this steps through")
                                .note("an absent scale is never an implicit C major: a step is a coordinate move and a coordinate needs a system"),
                        );
                    }
                };
                self.stepped(base, scale, steps, span)
            }
            _ => self.refuse(
                Diagnostic::error(Code::NotAValue, "this is not a written pitch")
                    .at(span, "expected a pitch, a transposition, or a scale step"),
            ),
        }
    }

    /// `base`, moved `steps` degrees through `scale`.
    ///
    /// The old checker's arithmetic, called where the old checker called it. It
    /// is `musa_score::scale`'s and none of it moves — what changes is only that the
    /// answer becomes a literal here rather than being deferred to a stage that
    /// no longer exists.
    pub(crate) fn stepped(
        &mut self,
        base: musa_score::WrittenPitch,
        scale: musa_score::scale::Scale,
        steps: i64,
        span: SourceSpan,
    ) -> Option<musa_score::WrittenPitch> {
        let Some(frame) = musa_score::scale::Frame::around(scale, base) else {
            return self.out_of_range(span);
        };
        let Some(degree) = frame.locate(base) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{base}` is not in {scale}"))
                    .at(span, "this scale has no degree there")
                    .note("a step counts through the collection in force, so the pitch it starts from has to be one of its degrees"),
            );
        };
        let moved = degree.step(steps)?;
        frame.pitch(moved).or_else(|| self.out_of_range(span))
    }

    /// What an `in scale` put in force: a spelled collection, or a bound name.
    ///
    /// The two are told apart by the node the parser built and not by trying to
    /// read the text both ways: `in scale c dorian` writes a scale expression
    /// and `in scale mode` writes a name, and asking the CST which one is there
    /// is the same reading [`super::values`] does one statement over.
    pub(crate) fn counting(&mut self, node: &SyntaxNode) -> Option<Counting> {
        if named(node) {
            // Read for its refusals — an unbound name is still an error here —
            // and discarded, because `in scale` contributes no fact of its own
            // and a `step` under a bound scale is refused where it is written.
            self.value(node)?;
            return Some(Counting::Bound);
        }
        self.written_scale(node).map(Counting::Written)
    }

    /// `body`, with one [`musa_score::origin::ExpansionStep::ScaleContext`] step on
    /// every fact it made.
    ///
    /// [`stamped`] is the operation, and it is [`Lowering::used`]'s and
    /// [`Lowering::asserted`]'s too.
    ///
    /// The step carries the collection *as the source spells it*, which is what
    /// Origin view prints and what `factext` parses back. A collection this
    /// reading could not spell — a bound `in scale mode { … }` — records the
    /// words the author wrote, because the step is a record of the source and
    /// not of the value.
    pub(crate) fn under_scale(node: &SyntaxNode, origin: Origin, written: &SyntaxNode, body: Raw) -> Raw {
        stamped(
            origin,
            crate::resolve::trimmed_span(node),
            musa_score::origin::ExpansionStep::ScaleContext {
                scale: format!(
                    "scale {}",
                    written.to_string().trim().trim_start_matches("scale").trim()
                ),
            },
            body,
        )
    }

    /// `scale c dorian`, as the collection it names.
    pub(crate) fn written_scale(&mut self, node: &SyntaxNode) -> Option<musa_score::scale::Scale> {
        let span = crate::resolve::trimmed_span(node);
        let written = node.to_string();
        let mut words = written.split_whitespace().skip_while(|word| *word == "scale");
        let tonic = words.next().unwrap_or_default();
        let Some(tonic) = musa_score::PitchClass::parse(tonic) else {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, format!("`{tonic}` is not a pitch class"))
                    .at(span, "expected a spelled tonic, such as `c` or `f#`"),
            );
        };
        let word = words.next().unwrap_or_default();
        let Some(collection) = musa_score::scale::Collection::named(word) else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown collection `{word}`"))
                    .at(span, "not a named scale collection")
                    .note("a mode is a rotation of the diatonic collection; other collections are their own values"),
            );
        };
        Some(musa_score::scale::Scale::new(tonic, collection))
    }
}
