//! Score scaffolding: score/part/voice declarations, meter/key, blocks, and the motif/fragment constructs.

use super::engine::Parser;
use crate::SyntaxKind;

/// What recovery anchors a broken score body.
const SCORE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::RBrace,
    SyntaxKind::PartKw,
    SyntaxKind::SectionKw,
    SyntaxKind::HarmonyKw,
];

/// What recovery anchors a broken part body.
const PART_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    SyntaxKind::ClefKw,
    SyntaxKind::MeterKw,
    SyntaxKind::TempoKw,
    SyntaxKind::ProfileKw,
    SyntaxKind::VoiceKw,
];

/// What recovery anchors a broken voice body.
const VOICE_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::Semicolon,
    SyntaxKind::RBrace,
    // `|` cannot appear inside an event, so it is the strongest anchor a
    // voice has: one malformed note poisons its own bar and no more.
    SyntaxKind::Pipe,
    SyntaxKind::LBracket,
    SyntaxKind::PitchLiteral,
    SyntaxKind::RestKw,
    SyntaxKind::UseKw,
    SyntaxKind::TransposeKw,
    SyntaxKind::RepeatKw,
    SyntaxKind::BarKw,
    SyntaxKind::AssertKw,
    SyntaxKind::SenzaKw,
    SyntaxKind::TempoKw,
    SyntaxKind::MeterKw,
    SyntaxKind::KeyKw,
    SyntaxKind::ClefKw,
    SyntaxKind::EndingKw,
    SyntaxKind::SlurKw,
    SyntaxKind::PhraseKw,
    SyntaxKind::GraceKw,
    SyntaxKind::CrescendoKw,
    SyntaxKind::DiminuendoKw,
    SyntaxKind::DynamicKw,
    SyntaxKind::TupletKw,
    SyntaxKind::StretchKw,
    SyntaxKind::RetrogradeKw,
    SyntaxKind::InvertKw,
    SyntaxKind::MobileKw,
    SyntaxKind::ImproviseKw,
    SyntaxKind::MarkKw,
];

impl Parser<'_> {
    /// `meter <n>/<d>;` or `meter none;`
    pub(super) fn meter_stmt(&mut self) {
        self.start(SyntaxKind::MeterStmt);
        self.bump(); // meter
        // `none` is a value of the meter, not a mechanism beside it: music
        // with no barlines is music whose meter says there are none. It is
        // spelled with the identifier rather than a keyword because there is
        // nothing else `meter` can be followed by.
        if self.at(SyntaxKind::Identifier) {
            self.bump();
        } else {
            self.expect(SyntaxKind::Rational, "a meter such as `4/4`, or `none`");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `key <pitch-class> <mode>;`
    pub(super) fn key_stmt(&mut self) {
        self.start(SyntaxKind::KeyStmt);
        self.bump(); // key
        // `key a minor;` writes the key out; `key k;` and `key M.k;` name one
        // a template was given or a module holds. One word before the `;`
        // cannot be both a tonic and a mode, so it is a name — and `key a;`
        // was never valid, so nothing that parsed before parses differently
        // now.
        let named = self.at(SyntaxKind::Identifier)
            && (self.nth_significant(1) == Some(SyntaxKind::Semicolon)
                || (self.nth_significant(1) == Some(SyntaxKind::Dot)
                    && self.nth_significant(3) == Some(SyntaxKind::Semicolon)));
        if named {
            self.expr();
        } else {
            self.pitch_class();
            self.expect(SyntaxKind::Identifier, "a mode (`major` or `minor`)");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `senza { ... }` — an unmeasured stretch, and the meter back after it.
    pub(super) fn senza_stmt(&mut self) {
        self.start(SyntaxKind::SenzaStmt);
        self.bump(); // senza
        self.block();
        self.finish();
    }

    /// `motif name(param: type, ...) { ... }`
    pub(super) fn motif_decl(&mut self) {
        self.start(SyntaxKind::MotifDecl);
        self.bump(); // motif
        self.expect(SyntaxKind::Identifier, "a motif name");
        // The one parameter list a `fn` writes: `01-surface.md` §2 desugars a
        // motif to a `fn`, so its parameters take the same types the same way
        // — an indexed `Duration<WrittenTime>` included — rather than a loop
        // of its own that admitted only a bare `Pitch` or `Duration`.
        self.param_list();
        self.block();
        self.finish();
    }

    /// `fragment name { ... }` — material a performance may reorder.
    ///
    /// A motif without parameters, tagged differently: a motif is material a
    /// *composer* reuses, a fragment is material a *performance* arranges. The
    /// namespace is one namespace, so `use` reaches both and a name collision
    /// is the diagnostic the resolver already writes.
    pub(super) fn fragment_decl(&mut self) {
        self.start(SyntaxKind::FragmentDecl);
        self.bump(); // fragment
        self.expect(SyntaxKind::Identifier, "a fragment name");
        self.block();
        self.finish();
    }

    /// `mobile { a; b; c; }` — its fragments in an order the performance
    /// chooses.
    ///
    /// A list of names, not a block of music: what a mobile arranges is
    /// *material*, and writing the notes inline would mean the same figure
    /// could not be reached from anywhere else.
    pub(super) fn mobile_stmt(&mut self) {
        self.start(SyntaxKind::MobileStmt);
        self.bump(); // mobile
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`mobile` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::Identifier) {
                self.bump();
                self.expect(SyntaxKind::Semicolon, "`;`");
            } else {
                self.expected_with_help("a fragment name", "a mobile lists fragments: `mobile { a; b; c; }`");
                self.recover(&[SyntaxKind::RBrace, SyntaxKind::Identifier, SyntaxKind::Semicolon]);
            }
        }
        self.finish();
    }

    /// `improvise 8/1 over "Dm7 | G7";` — a frame with unnotated contents.
    pub(super) fn improvise_stmt(&mut self) {
        self.start(SyntaxKind::ImproviseStmt);
        self.bump(); // improvise
        self.duration();
        if self.at(SyntaxKind::OverKw) {
            self.bump();
            self.expect(SyntaxKind::String, "the changes to play over, in quotes");
        }
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `score { part ... }`
    pub(super) fn score_decl(&mut self) {
        self.start(SyntaxKind::ScoreDecl);
        self.bump(); // score
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`score` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::PartKw) {
                self.part_decl();
            } else if self.at(SyntaxKind::SectionKw) {
                self.section_stmt();
            } else if self.at(SyntaxKind::HarmonyKw) {
                self.harmony_decl();
            } else {
                self.expected("a `part`, `section`, or `harmony` declaration");
                self.recover(SCORE_RECOVERY);
            }
        }
        self.finish();
    }

    /// `part name { clef ...; meter ...; tempo ...; voice ... }`
    ///
    /// A `meter` or a `tempo` here is the part's own, in force for the whole
    /// part: polymeter and polytempo. The part is the granularity because it
    /// is the one every consumer can express — a staff has one set of
    /// barlines in MEI, in `MusicXML` and on paper, so a per-voice barline
    /// grid would be a document nothing could draw.
    pub(super) fn part_decl(&mut self) {
        self.start(SyntaxKind::PartDecl);
        self.bump(); // part
        self.expect(SyntaxKind::Identifier, "a part name");
        self.expect(SyntaxKind::LBrace, "`{`");
        loop {
            if self.at(SyntaxKind::RBrace) {
                self.bump();
                break;
            }
            if self.current().is_none() {
                if let Some(error) = self.unclosed("`part` block") {
                    self.errors.push(error);
                }
                break;
            }
            if self.at(SyntaxKind::ClefKw) {
                self.clef_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                self.tempo_stmt();
            } else if self.at(SyntaxKind::ProfileKw) {
                self.profile_stmt();
            } else if self.at_word("sound") {
                self.sound_stmt();
            } else if self.at(SyntaxKind::VoiceKw) {
                self.voice_decl();
            } else {
                self.expected("`clef`, `meter`, `tempo`, `profile`, `sound`, or `voice`");
                self.recover(PART_RECOVERY);
            }
        }
        self.finish();
    }

    /// `clef <name>;`
    pub(super) fn clef_stmt(&mut self) {
        self.start(SyntaxKind::ClefStmt);
        self.bump(); // clef
        self.expect(SyntaxKind::Identifier, "a clef name");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `profile <name>;` — which performance profile realizes this part.
    pub(super) fn profile_stmt(&mut self) {
        self.start(SyntaxKind::ProfileStmt);
        self.bump(); // profile
        self.expect(SyntaxKind::Identifier, "a profile name");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    /// `sound <instrument> using <profile>;` — the ordinary one-action sound choice.
    pub(super) fn sound_stmt(&mut self) {
        self.start(SyntaxKind::SoundStmt);
        self.bump(); // sound
        self.qualified_sound_name("an instrument name");
        if self.at_word("using") {
            self.bump();
        } else {
            self.expected("`using`");
        }
        self.qualified_sound_name("a performance profile name");
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }

    fn qualified_sound_name(&mut self, what: &str) {
        if self.at_any(super::documents::MODULE_NAME) {
            self.bump();
            while self.at(SyntaxKind::Colon) && self.nth_significant(1) == Some(SyntaxKind::Colon) {
                self.bump();
                self.bump();
                if self.at_any(super::documents::MODULE_NAME) {
                    self.bump();
                } else {
                    self.expected(what);
                    break;
                }
            }
        } else {
            self.expected(what);
        }
    }

    /// `voice name { ... }`
    pub(super) fn voice_decl(&mut self) {
        self.start(SyntaxKind::VoiceDecl);
        self.bump(); // voice
        self.expect(SyntaxKind::Identifier, "a voice name");
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// Items of a voice or motif body, stopped by `}` or end of input.
    pub(super) fn voice_items(&mut self) {
        loop {
            if self.at(SyntaxKind::RBrace) || self.current().is_none() {
                break;
            }
            // The exit is checked before the dispatch, which is the whole of
            // `| a | b`: the pipe ends the bar it is in, and the voice one
            // level up is what opens the next one.
            if self.in_pipe_bar && !self.continues_a_bar() {
                break;
            }
            if self.at(SyntaxKind::Pipe) {
                self.pipe_bar_stmt();
            } else if self.at_any(&[SyntaxKind::PitchLiteral, SyntaxKind::Identifier, SyntaxKind::LParen]) {
                self.note_stmt();
            } else if self.at(SyntaxKind::RestKw) {
                self.rest_stmt();
            } else if self.at(SyntaxKind::LBracket) {
                self.chord_stmt();
            } else if self.at(SyntaxKind::Semicolon) {
                self.stray_semicolon();
            } else if self.at(SyntaxKind::UseKw) {
                self.use_stmt();
            } else if self.at(SyntaxKind::InKw) {
                self.in_scale_stmt();
            } else if self.at(SyntaxKind::StackKw) {
                self.stack_stmt();
            } else if self.at(SyntaxKind::TransposeKw) {
                self.transpose_stmt();
            } else if self.at(SyntaxKind::RepeatKw) {
                self.repeat_stmt();
            } else if self.at(SyntaxKind::BarKw) {
                self.bar_stmt();
            } else if self.at(SyntaxKind::AssertKw) {
                self.assert_stmt();
            } else if self.at(SyntaxKind::SenzaKw) {
                self.senza_stmt();
            } else if self.at(SyntaxKind::GraceKw) {
                self.grace_stmt();
            } else if self.at(SyntaxKind::MobileKw) {
                self.mobile_stmt();
            } else if self.at(SyntaxKind::ImproviseKw) {
                self.improvise_stmt();
            } else if self.at(SyntaxKind::TempoKw) {
                // The same statements the header and the part write, written
                // where the music reaches them: one kind, one node, two
                // places.
                self.tempo_stmt();
            } else if self.at(SyntaxKind::MeterKw) {
                self.meter_stmt();
            } else if self.at(SyntaxKind::KeyKw) {
                self.key_stmt();
            } else if self.at(SyntaxKind::ClefKw) {
                self.clef_stmt();
            } else if self.at(SyntaxKind::EndingKw) {
                self.ending_stmt();
            } else if self.at(SyntaxKind::SlurKw) {
                self.slur_stmt();
            } else if self.at(SyntaxKind::DynamicKw) {
                self.dynamic_stmt();
            } else if self.at(SyntaxKind::TupletKw) {
                self.tuplet_stmt();
            } else if self.at(SyntaxKind::StretchKw) {
                self.stretch_stmt();
            } else if self.at(SyntaxKind::RetrogradeKw) {
                self.retrograde_stmt();
            } else if self.at(SyntaxKind::InvertKw) {
                self.invert_stmt();
            } else if self.at(SyntaxKind::PhraseKw) {
                self.phrase_stmt();
            } else if self.at(SyntaxKind::MarkKw) {
                self.mark_stmt();
            } else if self.at_any(&[SyntaxKind::CrescendoKw, SyntaxKind::DiminuendoKw]) {
                self.hairpin_stmt();
            } else {
                self.expected_with_help(
                    "something to play",
                    "a voice holds notes (`c5/4`), chords (`[c5 e5]/4`), `rest`, `|`, and `use` — run `musa explain syntax` for the rest",
                );
                self.recover(VOICE_RECOVERY);
            }
        }
    }

    /// Whether the statement the parser is on belongs to the bar it is in.
    ///
    /// A bar holds the events of one measure and the marks written among
    /// them. Everything else a voice can write is *at least* a bar long — a
    /// repeat, an ending, a named bar, an unmeasured stretch, an
    /// improvisation — and notation draws those around barlines rather than
    /// inside one, so meeting one closes the bar the way a barline would.
    ///
    /// `meter` and `key` close it too, for the other reason: a time signature
    /// and a key signature are *printed* at a barline, which is why
    /// `examples/modulation.musa` says every change lands on one and why a
    /// meter written mid-bar is already an error.
    ///
    /// A whitelist rather than a blacklist: a statement kind added next year
    /// ends the bar, which is wrong in a way the composer sees, instead of
    /// lengthening it, which is wrong in a way only the bar-length check
    /// notices.
    pub(super) fn continues_a_bar(&self) -> bool {
        self.current().is_some_and(|kind| {
            matches!(
                kind,
                SyntaxKind::PitchLiteral
                    | SyntaxKind::Identifier
                    | SyntaxKind::LParen
                    | SyntaxKind::RestKw
                    | SyntaxKind::LBracket
                    | SyntaxKind::Semicolon
                    | SyntaxKind::UseKw
                    | SyntaxKind::DynamicKw
                    | SyntaxKind::ClefKw
                    | SyntaxKind::TempoKw
                    | SyntaxKind::MarkKw
                    | SyntaxKind::CrescendoKw
                    | SyntaxKind::DiminuendoKw
                    | SyntaxKind::TupletKw
                    | SyntaxKind::SlurKw
                    | SyntaxKind::GraceKw
            )
        })
    }

    /// `{ ... }` body of a motif, transpose, or repeat.
    pub(super) fn block(&mut self) {
        // A brace opens a context of its own: a tuplet written inside a `|`
        // bar reads its items as a block, not as more of the bar.
        let enclosing = std::mem::replace(&mut self.in_pipe_bar, false);
        self.block_inner();
        self.in_pipe_bar = enclosing;
    }

    pub(super) fn block_inner(&mut self) {
        self.start(SyntaxKind::Block);
        self.expect(SyntaxKind::LBrace, "`{`");
        self.voice_items();
        self.expect(SyntaxKind::RBrace, "`}`");
        self.finish();
    }

    /// `modulate <signal> -> <patch>.<stage>.<parameter>;`
    pub(super) fn modulate_stmt(&mut self) {
        self.start(SyntaxKind::ModulateStmt);
        self.bump(); // modulate
        self.expect(SyntaxKind::Identifier, "a signal name");
        self.expect(SyntaxKind::Arrow, "`->`");
        self.start(SyntaxKind::ParamPath);
        self.expect(SyntaxKind::Identifier, "a patch name");
        while self.at(SyntaxKind::Dot) {
            self.bump();
            self.expect(SyntaxKind::Identifier, "a stage or parameter name");
        }
        self.finish();
        self.expect(SyntaxKind::Semicolon, "`;`");
        self.finish();
    }
}
