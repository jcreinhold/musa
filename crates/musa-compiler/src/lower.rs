//! The direct lowering pass: CST → resolution + unit checks → `ScoreSnapshot`.
//! Private to the crate (roadmap §10.6: pass types never cross the
//! boundary).
//!
//! **Frozen oracle (prompt 12):** the canonical semantic path is
//! `elaborate.rs` through the temporal kernel (course correction §30 Step 6).
//! This pass stays compiled in and runnable — it is the differential
//! regression oracle (tests/elaboration.rs). Bugs found here after the
//! switch are fixed in the elaboration path unless the differential suite
//! itself is at fault; do not grow this pass with new features.
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
use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::score::{
    Clef, EventId, KeyMap, MeterMap, Mode, NotatedDuration, Part, PartId, ScoreEvent, ScoreEventKind, ScoreSnapshot,
    TempoMap, Voice, VoiceId,
};
use crate::time::{MusicalDuration, MusicalTime};

new_key_type! {
    /// Transient arena key for the declaration table (roadmap §15.3:
    /// never serialized as a permanent identity).
    pub(crate) struct DeclKey;
}

/// A piece-level declaration discovered during the walk.
pub(crate) enum DeclInfo {
    Tempo,
    Meter,
    Key,
    Motif,
    Part,
    Voice,
}

/// A collected motif definition ready for expansion.
pub(crate) struct MotifDef {
    pub(crate) params: Vec<musa_language::ast::Param>,
    pub(crate) body: Vec<VoiceItem>,
    pub(crate) declaration: DeclarationId,
}

/// A parameter bound at a `use` site.
#[derive(Clone)]
pub(crate) enum BoundValue {
    Pitch(WrittenPitch),
    Duration(NotatedDuration),
}

/// Expansion context carried through blocks: bound parameters, the
/// transposition stack (outermost first), and the provenance path prefix.
#[derive(Clone)]
pub(crate) struct ExpandCx {
    pub(crate) params: IndexMap<String, BoundValue>,
    pub(crate) intervals: Vec<Interval>,
    pub(crate) path: Vec<ExpansionStep>,
    pub(crate) declaration: DeclarationId,
    /// Set when expanding a motif: every event's origin points at the call.
    pub(crate) origin_span: Option<SourceSpan>,
    /// Only motifs declared before this index are visible; this makes
    /// cyclic expansion impossible by construction (roadmap §6.5).
    pub(crate) max_motif: usize,
}

/// Mutable lowering state.
pub(crate) struct Lowering {
    pub(crate) declarations: SlotMap<DeclKey, DeclInfo>,
    pub(crate) motifs: IndexMap<String, MotifDef>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) next_event: u64,
    pub(crate) next_part: u32,
}

impl Lowering {
    pub(crate) fn new() -> Self {
        Self {
            declarations: SlotMap::with_key(),
            motifs: IndexMap::new(),
            diagnostics: Vec::new(),
            next_event: 0,
            next_part: 0,
        }
    }

    pub(crate) fn declare(&mut self, info: DeclInfo) -> DeclKey {
        self.declarations.insert(info)
    }

    pub(crate) fn error(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.diagnostics.push(Diagnostic::error(message, Some(span)));
    }

    pub(crate) fn event_id(&mut self) -> EventId {
        let id = EventId(self.next_event);
        self.next_event = self.next_event.saturating_add(1);
        id
    }
}

/// Convert a text range to a serializable span.
pub(crate) fn span_of(node: &SyntaxNode) -> SourceSpan {
    let range = node.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}

/// The span of a node's significant (non-trivia) content: provenance points
/// at the construct, not at the whitespace before it.
pub(crate) fn trimmed_span(node: &SyntaxNode) -> SourceSpan {
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
pub(crate) fn ordinal(_lowering: &Lowering, key: DeclKey) -> DeclarationId {
    DeclarationId(u32::try_from(key.0.as_ffi() & 0xffff_ffff).unwrap_or(u32::MAX))
}

/// Text of the first token of `kind` under `node`.
pub(crate) fn token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
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
pub(crate) fn lower_header(lowering: &mut Lowering, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    snapshot.title = piece.name().unwrap_or_default();
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
    for motif in piece.motifs() {
        let name = motif.name().unwrap_or_default();
        if lowering.motifs.contains_key(&name) {
            lowering.error(format!("duplicate motif `{name}`"), span_of(motif.syntax()));
            continue;
        }
        let key = lowering.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: motif.params(),
            body: motif.items(),
            declaration: ordinal(lowering, key),
        };
        lowering.motifs.insert(name, definition);
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

pub(crate) fn parse_ratio(text: &str) -> Option<Ratio<i64>> {
    let (numerator, denominator) = text.split_once('/')?;
    Some(Ratio::new(numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Parse a duration token (`1/2`, `3/8`, `1`) into value + spelling.
pub(crate) fn parse_duration(node: &SyntaxNode) -> Option<NotatedDuration> {
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
/// Lower one voice from a root expansion context: sequential accumulation
/// of onsets (roadmap §5.2's span law by construction).
fn lower_voice(lowering: &mut Lowering, voice: &musa_language::ast::VoiceDecl, declaration: DeclarationId) -> Voice {
    let mut events = Vec::new();
    let mut onset = MusicalTime::ZERO;
    let cx = ExpandCx {
        params: IndexMap::new(),
        intervals: Vec::new(),
        path: Vec::new(),
        declaration,
        origin_span: None,
        max_motif: usize::MAX,
    };
    lower_items(lowering, &voice.items(), &cx, &mut events, &mut onset);
    Voice { events }
}

/// Lower a sequence of voice items under an expansion context.
fn lower_items(
    lowering: &mut Lowering,
    items: &[VoiceItem],
    cx: &ExpandCx,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
) {
    for item in items {
        match item {
            VoiceItem::Note(note) => {
                let Some(duration) = resolve_duration(lowering, note.syntax(), cx) else {
                    continue;
                };
                let pitch_text = note.pitch().unwrap_or_default();
                let Some(pitch) = resolve_pitch(lowering, &pitch_text, note.syntax(), cx) else {
                    continue;
                };
                push_event(
                    lowering,
                    events,
                    onset,
                    cx,
                    trimmed_span(note.syntax()),
                    duration,
                    ScoreEventKind::Note { pitch },
                );
            }
            VoiceItem::Rest(rest) => {
                let Some(duration) = resolve_duration(lowering, rest.syntax(), cx) else {
                    continue;
                };
                push_event(
                    lowering,
                    events,
                    onset,
                    cx,
                    trimmed_span(rest.syntax()),
                    duration,
                    ScoreEventKind::Rest,
                );
            }
            VoiceItem::Chord(chord) => {
                let Some(duration) = resolve_duration(lowering, chord.syntax(), cx) else {
                    continue;
                };
                let mut pitches = Vec::new();
                for text in chord.pitches() {
                    match WrittenPitch::parse(&text).map(|pitch| apply_intervals(lowering, pitch, cx, chord.syntax())) {
                        Some(Some(pitch)) => pitches.push(pitch),
                        Some(None) => break,
                        None => {
                            lowering.error(format!("invalid chord pitch `{text}`"), span_of(chord.syntax()));
                        }
                    }
                }
                push_event(
                    lowering,
                    events,
                    onset,
                    cx,
                    trimmed_span(chord.syntax()),
                    duration,
                    ScoreEventKind::Chord { pitches },
                );
            }
            VoiceItem::Use(call) => lower_use(lowering, call, cx, events, onset),
            VoiceItem::Transpose(transpose) => {
                let text = transpose.interval().unwrap_or_default();
                let Some(interval) = Interval::parse(&text, transpose.is_down()) else {
                    lowering.error(format!("unknown interval `{text}`"), span_of(transpose.syntax()));
                    continue;
                };
                let mut inner = cx.clone();
                inner.intervals.push(interval);
                inner.path.push(ExpansionStep::Transposition(interval));
                lower_items(lowering, &transpose.items(), &inner, events, onset);
            }
            VoiceItem::Repeat(repeat) => {
                let count: u32 = repeat.count().and_then(|text| text.parse().ok()).unwrap_or(0);
                for iteration in 0..count {
                    let mut inner = cx.clone();
                    inner.path.push(ExpansionStep::RepeatIteration(iteration));
                    lower_items(lowering, &repeat.items(), &inner, events, onset);
                }
            }
        }
    }
}

/// Append an event and advance the onset.
fn push_event(
    lowering: &mut Lowering,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
    cx: &ExpandCx,
    span: SourceSpan,
    duration: NotatedDuration,
    kind: ScoreEventKind,
) {
    let origin = Origin {
        source_span: cx.origin_span.unwrap_or(span),
        declaration: cx.declaration,
        expansion_path: cx.path.clone(),
    };
    let value = duration.value;
    events.push(ScoreEvent {
        id: lowering.event_id(),
        origin,
        onset: *onset,
        notated_duration: duration,
        kind,
    });
    *onset = *onset + value;
}

/// Resolve a pitch token: a literal, or a bound `pitch` parameter.
pub(crate) fn resolve_pitch(
    lowering: &mut Lowering,
    text: &str,
    node: &SyntaxNode,
    cx: &ExpandCx,
) -> Option<WrittenPitch> {
    let pitch = if let Some(pitch) = WrittenPitch::parse(text) {
        pitch
    } else if let Some(BoundValue::Pitch(pitch)) = cx.params.get(text) {
        *pitch
    } else {
        lowering.error(format!("unknown pitch reference `{text}`"), span_of(node));
        return None;
    };
    apply_intervals(lowering, pitch, cx, node)
}

/// Apply the transposition stack, outermost first.
fn apply_intervals(
    lowering: &mut Lowering,
    pitch: WrittenPitch,
    cx: &ExpandCx,
    node: &SyntaxNode,
) -> Option<WrittenPitch> {
    let mut current = pitch;
    for interval in &cx.intervals {
        let Some(next) = current.transpose(*interval) else {
            lowering.error(
                format!("transposition of `{current}` needs more than a double accidental"),
                span_of(node),
            );
            return None;
        };
        current = next;
    }
    Some(current)
}

/// Resolve a duration token: a literal, or a bound `duration` parameter.
pub(crate) fn resolve_duration(lowering: &mut Lowering, node: &SyntaxNode, cx: &ExpandCx) -> Option<NotatedDuration> {
    if let Some(duration) = parse_duration(node) {
        return Some(duration);
    }
    if let Some(text) = token_text(node, SyntaxKind::Identifier)
        && let Some(BoundValue::Duration(duration)) = cx.params.get(&text)
    {
        return Some(duration.clone());
    }
    lowering.error("missing or unresolved duration", span_of(node));
    None
}

/// Expand a `use` statement: bind arguments, then lower the motif body.
fn lower_use(
    lowering: &mut Lowering,
    call: &musa_language::ast::UseStmt,
    cx: &ExpandCx,
    events: &mut Vec<ScoreEvent>,
    onset: &mut MusicalTime,
) {
    let name = call.motif().unwrap_or_default();
    let found = lowering
        .motifs
        .get_full(&name)
        .map(|(index, _, motif)| (index, motif.params.clone(), motif.body.clone(), motif.declaration));
    let Some((index, motif_params, body, declaration)) = found else {
        lowering.error(format!("unknown motif `{name}`"), span_of(call.syntax()));
        return;
    };
    if index >= cx.max_motif {
        lowering.error(
            format!("motif `{name}` can only reference motifs declared before it"),
            span_of(call.syntax()),
        );
        return;
    }
    let args = call.args();
    if args.len() > motif_params.len() {
        lowering.error(
            format!(
                "motif `{name}` takes {} arguments, got {}",
                motif_params.len(),
                args.len()
            ),
            span_of(call.syntax()),
        );
        return;
    }
    let mut params = IndexMap::new();
    for (position, param) in motif_params.iter().enumerate() {
        let text = args.get(position).cloned().or_else(|| param.default.clone());
        let Some(text) = text else {
            lowering.error(
                format!("motif `{name}`: missing argument `{}`", param.name),
                span_of(call.syntax()),
            );
            return;
        };
        let Some(value) = bind_argument(lowering, &name, param, &text, cx, call.syntax()) else {
            return;
        };
        params.insert(param.name.clone(), value);
    }
    let call_span = trimmed_span(call.syntax());
    let inner = ExpandCx {
        params,
        intervals: cx.intervals.clone(),
        path: cx
            .path
            .iter()
            .cloned()
            .chain(std::iter::once(ExpansionStep::MotifApplication {
                call_site: call_span,
            }))
            .collect(),
        declaration,
        origin_span: Some(call_span),
        max_motif: index,
    };
    lower_items(lowering, &body, &inner, events, onset);
}

/// Bind one argument text to a parameter kind.
pub(crate) fn bind_argument(
    lowering: &mut Lowering,
    motif: &str,
    param: &musa_language::ast::Param,
    text: &str,
    cx: &ExpandCx,
    node: &SyntaxNode,
) -> Option<BoundValue> {
    match param.kind.as_str() {
        "pitch" => {
            if let Some(pitch) = WrittenPitch::parse(text) {
                Some(BoundValue::Pitch(pitch))
            } else if let Some(BoundValue::Pitch(pitch)) = cx.params.get(text) {
                Some(BoundValue::Pitch(*pitch))
            } else {
                lowering.error(format!("motif `{motif}`: `{text}` is not a pitch"), span_of(node));
                None
            }
        }
        "duration" => {
            if let Some(value) = parse_ratio(text).or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer)) {
                Some(BoundValue::Duration(NotatedDuration {
                    value: MusicalDuration::new(value),
                    spelling: text.to_string(),
                }))
            } else if let Some(BoundValue::Duration(duration)) = cx.params.get(text) {
                Some(BoundValue::Duration(duration.clone()))
            } else {
                lowering.error(format!("motif `{motif}`: `{text}` is not a duration"), span_of(node));
                None
            }
        }
        other => {
            lowering.error(
                format!("motif `{motif}`: unknown parameter kind `{other}`"),
                span_of(node),
            );
            None
        }
    }
}

pub(crate) fn check_measure_sanity(lowering: &mut Lowering, snapshot: &ScoreSnapshot) {
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
