//! The lowering pass: CST → resolution + unit checks → `ScoreSnapshot`.
//! Private to the crate (roadmap §10.6: pass types never cross the
//! boundary).
//!
//! Time accumulation here uses the `MusicalTime`/`MusicalDuration` operators,
//! which are total for musa's magnitudes (see `time.rs`); the workspace
//! arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use indexmap::IndexMap;
use musa_language::ast::{AstNode as _, KeyStmt, PieceDecl, TempoStmt, VoiceItem};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;
use slotmap::{SlotMap, new_key_type};

use crate::compile::{Compilation, Diagnostic, SourceDocument};
use crate::origin::{DeclarationId, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::score::{
    Clef, EventId, KeyMap, MeterMap, Mode, NotatedDuration, Part, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot,
    TempoMap, Voice, VoiceId,
};
use crate::time::{MusicalDuration, MusicalTime};

new_key_type! {
    /// Transient arena key for the declaration table (roadmap §15.3:
    /// never serialized as a permanent identity).
    struct DeclKey;
}

/// A piece-level declaration discovered during the walk. Payloads (spans
/// for "declared here" diagnostics) arrive when motif resolution needs
/// them in prompt 06.
enum DeclInfo {
    Tempo,
    Meter,
    Key,
    Motif,
    Part,
    Voice,
}

/// Mutable lowering state.
struct Lowering {
    declarations: SlotMap<DeclKey, DeclInfo>,
    diagnostics: Vec<Diagnostic>,
    next_event: u64,
    next_part: u32,
}

impl Lowering {
    fn new() -> Self {
        Self {
            declarations: SlotMap::with_key(),
            diagnostics: Vec::new(),
            next_event: 0,
            next_part: 0,
        }
    }

    fn declare(&mut self, info: DeclInfo) -> DeclKey {
        self.declarations.insert(info)
    }

    fn error(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.diagnostics.push(Diagnostic::error(message, Some(span)));
    }

    fn warn(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.diagnostics.push(Diagnostic::warning(message, Some(span)));
    }

    fn event_id(&mut self) -> EventId {
        let id = EventId(self.next_event);
        self.next_event = self.next_event.saturating_add(1);
        id
    }
}

/// Convert a text range to a serializable span.
fn span_of(node: &SyntaxNode) -> SourceSpan {
    let range = node.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}

/// The span of a node's significant (non-trivia) content: provenance points
/// at the construct, not at the whitespace before it.
fn trimmed_span(node: &SyntaxNode) -> SourceSpan {
    let mut start = None;
    let mut end = None;
    for element in node.children_with_tokens() {
        if element.kind().is_trivia() {
            continue;
        }
        let range = element.text_range();
        if start.is_none() {
            start = Some(range.start());
        }
        end = Some(range.end());
    }
    match (start, end) {
        (Some(start), Some(end)) => SourceSpan::new(u32::from(start), u32::from(end)),
        _ => span_of(node),
    }
}

/// Declaration ordinal = its slot's position in insertion order.
fn ordinal(_lowering: &Lowering, key: DeclKey) -> DeclarationId {
    DeclarationId(u32::try_from(key.0.as_ffi() & 0xffff_ffff).unwrap_or(u32::MAX))
}

/// Text of the first token of `kind` under `node`.
fn token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == kind)
        .map(|token| token.text().to_string())
}

/// Lower a document into a compilation.
pub(crate) fn lower(source: &SourceDocument) -> Compilation {
    let document = musa_language::parse(source.text());
    let mut lowering = Lowering::new();
    for error in document.errors() {
        let range = error.range();
        lowering.error(
            format!("syntax: {}", error.message()),
            SourceSpan::new(u32::from(range.start()), u32::from(range.end())),
        );
    }
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, lowering.diagnostics);
    }
    let Some(piece) = PieceDecl::from_root(&document.syntax()) else {
        lowering.error("no `piece` declaration", SourceSpan::new(0, 0));
        return Compilation::new(None, lowering.diagnostics);
    };

    let mut snapshot = ScoreSnapshot::default();
    lower_header(&mut lowering, &piece, &mut snapshot);
    if let Some(score) = piece.score() {
        lower_score(&mut lowering, &score, &mut snapshot);
    }
    check_measure_sanity(&mut lowering, &snapshot);
    if lowering
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::compile::Severity::Error)
    {
        return Compilation::new(None, lowering.diagnostics);
    }
    Compilation::new(Some(snapshot), lowering.diagnostics)
}

/// Tempo, meter, key — plus registration of motif declarations (expansion
/// is prompt 06).
fn lower_header(lowering: &mut Lowering, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    if let Some(tempo) = piece.tempo() {
        lowering.declare(DeclInfo::Tempo);
        snapshot.tempo_map = parse_tempo(lowering, &tempo);
    }
    if let Some(meter) = piece.meter() {
        lowering.declare(DeclInfo::Meter);
        if let Some(map) = parse_meter(&meter) {
            snapshot.meter_map = map;
        } else {
            lowering.error("invalid meter", span_of(meter.syntax()));
        }
    }
    if let Some(key) = piece.key() {
        lowering.declare(DeclInfo::Key);
        match parse_key(&key) {
            Some(map) => snapshot.key_map = Some(map),
            None => lowering.error("invalid key declaration", span_of(key.syntax())),
        }
    }
    let mut seen_motifs = std::collections::HashSet::new();
    for motif in piece.motifs() {
        let name = motif.name().unwrap_or_default();
        if !seen_motifs.insert(name.clone()) {
            lowering.error(format!("duplicate motif `{name}`"), span_of(motif.syntax()));
            continue;
        }
        lowering.declare(DeclInfo::Motif);
        lowering.warn(
            format!("motif `{name}`: expansion is not yet implemented; applications will be skipped"),
            span_of(motif.syntax()),
        );
    }
}

fn parse_tempo(lowering: &mut Lowering, tempo: &TempoStmt) -> TempoMap {
    let syntax = tempo.syntax();
    let beat = token_text(syntax, SyntaxKind::Rational)
        .and_then(|text| parse_ratio(&text))
        // `quarter` (or another beat name) resolves to 1/4 for now.
        .unwrap_or_else(|| Ratio::new(1, 4));
    let bpm = token_text(syntax, SyntaxKind::Integer)
        .and_then(|text| text.parse::<u32>().ok())
        .unwrap_or_else(|| {
            lowering.error("tempo needs a bpm value", span_of(syntax));
            120
        });
    TempoMap { beat, bpm }
}

fn parse_meter(meter: &musa_language::ast::MeterStmt) -> Option<MeterMap> {
    let text = meter.value()?;
    let (numerator, denominator) = text.split_once('/')?;
    Some(MeterMap {
        numerator: numerator.parse().ok()?,
        denominator: denominator.parse().ok()?,
    })
}

fn parse_key(key: &KeyStmt) -> Option<KeyMap> {
    let syntax = key.syntax();
    let mut identifiers = syntax
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Identifier);
    let tonic = PitchClass::parse(identifiers.next()?.text())?;
    let mode = match identifiers.next()?.text() {
        "major" => Mode::Major,
        "minor" => Mode::Minor,
        _ => return None,
    };
    Some(KeyMap { tonic, mode })
}

fn parse_ratio(text: &str) -> Option<Ratio<i64>> {
    let (numerator, denominator) = text.split_once('/')?;
    Some(Ratio::new(numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Parse a duration token (`1/2`, `3/8`, `1`) into value + spelling.
fn parse_duration(node: &SyntaxNode) -> Option<NotatedDuration> {
    let (kind, text) = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Rational || token.kind() == SyntaxKind::Integer)
        .map(|token| (token.kind(), token.text().to_string()))?;
    let value = if kind == SyntaxKind::Rational {
        parse_ratio(&text)?
    } else {
        Ratio::from_integer(text.parse().ok()?)
    };
    Some(NotatedDuration {
        value: MusicalDuration::new(value),
        spelling: text,
    })
}

fn lower_score(lowering: &mut Lowering, score: &musa_language::ast::ScoreDecl, snapshot: &mut ScoreSnapshot) {
    for part in score.parts() {
        let name = part.name().unwrap_or_default();
        let part_key = lowering.declare(DeclInfo::Part);
        let _ = ordinal(lowering, part_key);
        if snapshot.parts.iter().any(|(_, existing)| existing.name == name) {
            lowering.error(format!("duplicate part `{name}`"), span_of(part.syntax()));
            continue;
        }
        let id = PartId(lowering.next_part);
        lowering.next_part = lowering.next_part.saturating_add(1);

        let mut clef = None;
        for node in part.syntax().children() {
            if node.kind() != SyntaxKind::ClefStmt {
                continue;
            }
            match token_text(&node, SyntaxKind::Identifier).and_then(|text| Clef::parse(&text)) {
                Some(parsed) => clef = Some(parsed),
                None => lowering.error("unknown clef", span_of(&node)),
            }
        }

        let mut voices = IndexMap::new();
        let mut voice_names = IndexMap::new();
        for (index, voice) in part.voices().iter().enumerate() {
            let voice_name = voice.name().unwrap_or_default();
            let voice_key = lowering.declare(DeclInfo::Voice);
            let declaration = ordinal(lowering, voice_key);
            if voice_names.values().any(|existing| *existing == voice_name) {
                lowering.error(
                    format!("duplicate voice `{voice_name}` in part `{name}`"),
                    span_of(voice.syntax()),
                );
                continue;
            }
            let voice_id = VoiceId(u32::try_from(index).unwrap_or(u32::MAX));
            let lowered = lower_voice(lowering, voice, declaration);
            voices.insert(voice_id, lowered);
            voice_names.insert(voice_id, voice_name);
        }

        snapshot.parts.insert(
            id,
            Part {
                id,
                name,
                clef,
                voices,
                voice_names,
            },
        );
    }
}

/// Lower one voice: sequential accumulation of onsets (roadmap §5.2's span
/// law by construction).
fn lower_voice(lowering: &mut Lowering, voice: &musa_language::ast::VoiceDecl, declaration: DeclarationId) -> Voice {
    let mut events = Vec::new();
    let mut onset = MusicalTime::ZERO;
    for item in voice.items() {
        match item {
            VoiceItem::Note(note) => {
                let Some(duration) = parse_duration(note.syntax()) else {
                    lowering.error("missing duration", span_of(note.syntax()));
                    continue;
                };
                let pitch_text = note.pitch().unwrap_or_default();
                let kind = match WrittenPitch::parse(&pitch_text) {
                    Some(pitch) => ScoreEventKind::Note { pitch },
                    None => {
                        lowering.warn(
                            format!("`{pitch_text}` is a motif parameter reference; parameter binding is not yet implemented"),
                            span_of(note.syntax()),
                        );
                        onset = onset + duration.value;
                        continue;
                    }
                };
                let origin = Origin {
                    source_span: trimmed_span(note.syntax()),
                    declaration,
                    expansion_path: Vec::new(),
                };
                events.push(ScoreEvent {
                    id: lowering.event_id(),
                    origin,
                    onset,
                    notated_duration: duration.clone(),
                    kind,
                });
                onset = onset + duration.value;
            }
            VoiceItem::Rest(rest) => {
                let Some(duration) = parse_duration(rest.syntax()) else {
                    lowering.error("missing duration", span_of(rest.syntax()));
                    continue;
                };
                let origin = Origin {
                    source_span: trimmed_span(rest.syntax()),
                    declaration,
                    expansion_path: Vec::new(),
                };
                events.push(ScoreEvent {
                    id: lowering.event_id(),
                    origin,
                    onset,
                    notated_duration: duration.clone(),
                    kind: ScoreEventKind::Rest,
                });
                onset = onset + duration.value;
            }
            VoiceItem::Chord(chord) => {
                let Some(duration) = parse_duration(chord.syntax()) else {
                    lowering.error("missing duration", span_of(chord.syntax()));
                    continue;
                };
                let pitches: Vec<WrittenPitch> = chord
                    .pitches()
                    .iter()
                    .filter_map(|text| WrittenPitch::parse(text))
                    .collect();
                let origin = Origin {
                    source_span: trimmed_span(chord.syntax()),
                    declaration,
                    expansion_path: Vec::new(),
                };
                events.push(ScoreEvent {
                    id: lowering.event_id(),
                    origin,
                    onset,
                    notated_duration: duration.clone(),
                    kind: ScoreEventKind::Chord { pitches },
                });
                onset = onset + duration.value;
            }
            VoiceItem::Use(call) => {
                let name = call.motif().unwrap_or_default();
                lowering.warn(
                    format!("`use {name}(...)`: motif expansion is not yet implemented; skipped"),
                    span_of(call.syntax()),
                );
            }
            VoiceItem::Transpose(transpose) => {
                lowering.warn(
                    "`transpose` is not yet implemented; contents skipped",
                    span_of(transpose.syntax()),
                );
            }
            VoiceItem::Repeat(repeat) => {
                lowering.warn(
                    "`repeat` is not yet implemented; contents skipped",
                    span_of(repeat.syntax()),
                );
            }
        }
    }
    Voice { events }
}

/// Warn when a voice's span is not a whole number of measures.
fn check_measure_sanity(lowering: &mut Lowering, snapshot: &ScoreSnapshot) {
    let measure = snapshot.meter_map.measure_len();
    if measure.is_zero() {
        return;
    }
    for (_, part) in snapshot.parts.iter() {
        for (voice_id, voice) in &part.voices {
            let span = voice.span().as_ratio();
            let measures = span / measure.as_ratio();
            if span != Ratio::ZERO && *measures.denom() != 1 {
                let name = part.voice_names.get(voice_id).map_or("?", String::as_str);
                lowering.diagnostics.push(Diagnostic::warning(
                    format!(
                        "voice `{}` in part `{}` spans {} whole notes, which is not a whole number of {} measures",
                        name,
                        part.name,
                        voice.span(),
                        snapshot.meter_map.numerator
                    ),
                    None,
                ));
            }
        }
    }
}
