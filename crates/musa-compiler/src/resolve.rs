//! Resolution: names, declarations, ordinals, motifs, and the readings of
//! the header that everything downstream depends on.
//!
//! This is the vocabulary the semantic path speaks *before* anything becomes
//! an occurrence — a pitch text resolved against the transposition stack, a
//! `use` argument bound to a parameter, a motif looked up in the table, a
//! profile or a clef read off a declaration — together with the diagnostic
//! sink those readings report into. Private to the crate (roadmap §10.6:
//! pass types never cross the boundary).
//!
//! It was carved out of `lower.rs` at prompt 41, which had become two modules
//! wearing one name: this vocabulary, and one of the two implementations that
//! spoke it (`PoSD` ch. 16's *conjoined methods*). Separating them first is
//! what let the frozen lowerer be deleted as a deletion.
//!
//! Time accumulation here uses the `MusicalTime`/`MusicalDuration` operators,
//! which are total for musa's magnitudes (see `time.rs`); the workspace
//! arithmetic lint is allowed at module scope for that reason.
#![allow(clippy::arithmetic_side_effects)]

use indexmap::IndexMap;
use musa_language::ast::{
    ArticulationRule, AstNode as _, DynamicRule, KeyStmt, PerformanceDecl, PieceDecl, SettingStmt, TempoStmt, VoiceItem,
};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;
use slotmap::{SlotMap, new_key_type};

use crate::compile::Diagnostic;
use crate::origin::{DeclarationId, ExpansionStep, Interval, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::profile::{ArticulationRealization, PerformanceProfile, ProfileSet};
use crate::score::{
    AnnotationStore, ArticulationMark, Clef, DynamicMark, EventId, KeyMap, MeterMap, Mode, NotatedDuration,
    ScoreSnapshot, TempoMap,
};
use crate::time::MusicalDuration;

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
    /// How the enclosing tuplets scale the durations written here (`2/3`
    /// inside a `3/2` tuplet). Always `1` on the direct path, which rejects
    /// tuplets outright.
    pub(crate) scale: Ratio<i64>,
}

/// What resolution accumulates while a piece is read: the tables names are
/// resolved against, the counters that issue identities, the annotations
/// resolved to those identities, and the diagnostics for what could not be
/// resolved at all.
///
/// One `&mut` threaded through the pass rather than a return value per
/// helper, because every one of these is append-only and read by later
/// helpers — a resolution that has to be merged is a resolution that can
/// disagree with itself.
pub(crate) struct Resolver {
    pub(crate) declarations: SlotMap<DeclKey, DeclInfo>,
    pub(crate) motifs: IndexMap<String, MotifDef>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) next_event: u64,
    pub(crate) next_part: u32,
    pub(crate) annotations: AnnotationStore,
    /// Where each voice's kernel timeline goes on its way to the adapter.
    ///
    /// `None` on every production path — nothing keeps a timeline after the
    /// snapshot is built. It is `Some` only under `crate::bench`, which needs
    /// the elaboration and projection stages separable to measure them apart
    /// (roadmap §17.7). One `Option` check per voice is the whole cost.
    pub(crate) timeline_sink: Option<Vec<crate::elaborate::VoiceTimeline>>,
}

impl Resolver {
    pub(crate) fn new() -> Self {
        Self {
            declarations: SlotMap::with_key(),
            motifs: IndexMap::new(),
            diagnostics: Vec::new(),
            next_event: 0,
            next_part: 0,
            annotations: AnnotationStore::default(),
            timeline_sink: None,
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
pub(crate) fn ordinal(_resolver: &Resolver, key: DeclKey) -> DeclarationId {
    DeclarationId(u32::try_from(key.0.as_ffi() & 0xffff_ffff).unwrap_or(u32::MAX))
}

/// Text of the first token of `kind` under `node`.
pub(crate) fn token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == kind)
        .map(|token| token.text().to_string())
}

/// Resolve the piece's `studio` block, if it has one.
///
/// Shared by both semantic paths: the studio says nothing about notes, so
/// there is nothing here for the two elaborations to disagree about, and one
/// implementation is one fewer place for them to drift.
pub(crate) fn lower_studio(
    resolver: &mut Resolver,
    piece: &PieceDecl,
    snapshot: &ScoreSnapshot,
    imported: &[musa_language::ast::StudioDecl],
) -> crate::studio::StudioSpec {
    let studio = piece.studio();
    if studio.is_none() && imported.is_empty() {
        return crate::studio::StudioSpec::default();
    }
    let parts: Vec<String> = snapshot.parts.iter().map(|(_, part)| part.name.clone()).collect();
    crate::studio::resolve(studio.as_ref(), imported, &parts, &mut resolver.diagnostics)
}

/// Tempo, meter, key — plus registration of motif declarations (expansion
/// is prompt 06).
pub(crate) fn lower_header(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    snapshot.title = piece.name().unwrap_or_default();
    if let Some(tempo) = piece.tempo() {
        resolver.declare(DeclInfo::Tempo);
        snapshot.tempo_map = parse_tempo(resolver, &tempo);
    }
    // A second tempo without a position would leave two answers to "how fast
    // does this piece start" — the one thing a header may not do.
    for extra in piece.tempos().iter().filter(|tempo| tempo.position().is_none()).skip(1) {
        resolver.error(
            "the piece already has a starting tempo; write `at <measure>:<beat>` to change it",
            span_of(extra.syntax()),
        );
    }
    if let Some(meter) = piece.meter() {
        resolver.declare(DeclInfo::Meter);
        if let Some(map) = parse_meter(&meter) {
            snapshot.meter_map = map;
        } else {
            resolver.error("invalid meter", span_of(meter.syntax()));
        }
    }
    if let Some(key) = piece.key() {
        resolver.declare(DeclInfo::Key);
        match parse_key(&key) {
            Some(map) => snapshot.key_map = Some(map),
            None => resolver.error("invalid key declaration", span_of(key.syntax())),
        }
    }
    if let Some(performance) = piece.performance() {
        let profiles = parse_profiles(resolver, &performance);
        merge_profiles(resolver, snapshot, &profiles, span_of(performance.syntax()), None);
    }
    register_motifs(resolver, snapshot, &piece.motifs(), None);
}

/// Register a set of motif declarations, refusing to shadow.
///
/// `from` names the library a declaration was imported from; `None` is the
/// piece's own. Two motifs with one name is always an error — an imported
/// name that quietly loses to a local one would make a piece sound different
/// depending on what it imported.
pub(crate) fn register_motifs(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    motifs: &[musa_language::ast::MotifDecl],
    from: Option<&str>,
) {
    for motif in motifs {
        let name = motif.name().unwrap_or_default();
        if resolver.motifs.contains_key(&name) {
            resolver.error(duplicate(&format!("motif `{name}`"), from), span_of(motif.syntax()));
            continue;
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: motif.params(),
            body: motif.items(),
            declaration: ordinal(resolver, key),
        };
        snapshot.motifs.push(crate::score::MotifDeclaration {
            name: name.clone(),
            span: trimmed_span(motif.syntax()),
        });
        resolver.motifs.insert(name, definition);
    }
}

/// Merge a `performance` block's profiles into the snapshot, refusing to
/// shadow for the same reason motifs do.
pub(crate) fn merge_profiles(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    profiles: &ProfileSet,
    span: SourceSpan,
    from: Option<&str>,
) {
    let names: Vec<String> = profiles.names().map(str::to_owned).collect();
    for name in names {
        if snapshot.profiles.declares(&name) {
            resolver.error(duplicate(&format!("profile `{name}`"), from), span);
            continue;
        }
        if let Some(profile) = profiles.get(&name) {
            snapshot.profiles.insert(profile.clone());
        }
    }
}

/// "duplicate X" — and, when it came from a library, which one.
fn duplicate(what: &str, from: Option<&str>) -> String {
    from.map_or_else(
        || format!("duplicate {what}"),
        |path| format!("`{path}` declares {what}, which this piece already has"),
    )
}

/// Read the `performance` block into a [`ProfileSet`]. Declarations only —
/// nothing here is applied until `lower_performance` (roadmap §6.4).
pub(crate) fn parse_profiles(resolver: &mut Resolver, performance: &PerformanceDecl) -> ProfileSet {
    let mut set = ProfileSet::default();
    for declaration in performance.profiles() {
        let name = declaration.name().unwrap_or_default();
        if set.declares(&name) {
            resolver.error(
                format!("duplicate profile `{name}`"),
                trimmed_span(declaration.syntax()),
            );
            continue;
        }
        let mut profile = PerformanceProfile::named(&name);
        for rule in declaration.articulations() {
            let written = rule.mark().unwrap_or_default();
            let Some(mark) = ArticulationMark::parse(&written) else {
                resolver.error(format!("unknown articulation `{written}`"), trimmed_span(rule.syntax()));
                continue;
            };
            profile.set_articulation(mark, articulation_settings(resolver, &rule));
        }
        for rule in declaration.dynamics() {
            let written = rule.mark().unwrap_or_default();
            let Some(mark) = DynamicMark::parse(&written) else {
                resolver.error(
                    format!("unknown dynamic marking `{written}`"),
                    trimmed_span(rule.syntax()),
                );
                continue;
            };
            if let Some(amplitude) = dynamic_settings(resolver, &rule) {
                profile.set_dynamic(mark, amplitude);
            }
        }
        set.insert(profile);
    }
    set
}

/// `gate` (a ratio of the written value) and `attack` (a time).
fn articulation_settings(resolver: &mut Resolver, rule: &ArticulationRule) -> ArticulationRealization {
    let mut realization = ArticulationRealization::NEUTRAL;
    for setting in rule.settings() {
        let name = setting.name().unwrap_or_default();
        match name.as_str() {
            "gate" => {
                if let Some(gate) = ratio_setting(resolver, &setting) {
                    realization.gate = gate;
                }
            }
            "attack" => {
                if let Some(attack) = time_setting(resolver, &setting) {
                    realization.attack = attack;
                }
            }
            other => resolver.error(
                format!("unknown setting `{other}` (expected `gate` or `attack`)"),
                trimmed_span(setting.syntax()),
            ),
        }
    }
    realization
}

/// `amplitude` — abstract loudness, not decibels (§2).
fn dynamic_settings(resolver: &mut Resolver, rule: &DynamicRule) -> Option<Ratio<i64>> {
    let mut amplitude = None;
    for setting in rule.settings() {
        let name = setting.name().unwrap_or_default();
        if name == "amplitude" {
            amplitude = ratio_setting(resolver, &setting);
        } else {
            resolver.error(
                format!("unknown setting `{name}` (expected `amplitude`)"),
                trimmed_span(setting.syntax()),
            );
        }
    }
    if amplitude.is_none() {
        resolver.error("a `dynamic` rule needs an `amplitude`", trimmed_span(rule.syntax()));
    }
    amplitude
}

/// A unitless ratio in `0..=1`. A unit here is a category error: a gate is a
/// fraction of the written value, not a length of time.
fn ratio_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.error(format!("`{name}` is a ratio, not a `{unit}` value"), span);
        return None;
    }
    let value = crate::profile::parse_decimal(&setting.value()?)?;
    if value < Ratio::ZERO || value > Ratio::ONE {
        resolver.error(format!("`{name}` must be between 0 and 1"), span);
        return None;
    }
    Some(value)
}

/// A time in seconds, written with its unit (roadmap §7.2: units are syntax).
fn time_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    let value = crate::profile::parse_decimal(&setting.value()?)?;
    let seconds = match setting.unit().as_deref() {
        Some("ms") => value / 1000,
        Some("s") => value,
        _ => {
            resolver.error(format!("`{name}` needs a time unit (`ms` or `s`)"), span);
            return None;
        }
    };
    if seconds < Ratio::ZERO {
        resolver.error(format!("`{name}` cannot be negative"), span);
        return None;
    }
    Some(seconds)
}

/// The part-level facts both semantic paths read the same way: the written
/// clef and the profile that realizes the part.
pub(crate) fn part_metadata(
    resolver: &mut Resolver,
    part: &musa_language::ast::PartDecl,
    profiles: &ProfileSet,
) -> (Option<Clef>, Option<String>) {
    let mut clef = None;
    for node in part.syntax().children() {
        if node.kind() != SyntaxKind::ClefStmt {
            continue;
        }
        match token_text(&node, SyntaxKind::Identifier).and_then(|text| Clef::parse(&text)) {
            Some(parsed) => clef = Some(parsed),
            None => resolver.error("unknown clef", span_of(&node)),
        }
    }
    let mut profile = None;
    if let Some(statement) = part.profile() {
        let name = statement.name().unwrap_or_default();
        if profiles.declares(&name) {
            profile = Some(name);
        } else {
            resolver.error(format!("unknown profile `{name}`"), trimmed_span(statement.syntax()));
        }
    }
    (clef, profile)
}

/// The beat unit and bpm a `tempo` statement writes.
pub(crate) fn tempo_reading(resolver: &mut Resolver, tempo: &TempoStmt) -> (Ratio<i64>, u32) {
    let syntax = tempo.syntax();
    let beat = token_text(syntax, SyntaxKind::Rational)
        .and_then(|text| parse_ratio(&text))
        // `quarter` (or another beat name) resolves to 1/4 for now.
        .unwrap_or_else(|| Ratio::new(1, 4));
    let bpm = token_text(syntax, SyntaxKind::Integer)
        .and_then(|text| text.parse::<u32>().ok())
        .unwrap_or_else(|| {
            resolver.error("tempo needs a bpm value", span_of(syntax));
            120
        });
    (beat, bpm)
}

fn parse_tempo(resolver: &mut Resolver, tempo: &TempoStmt) -> TempoMap {
    let (beat, bpm) = tempo_reading(resolver, tempo);
    TempoMap {
        beat,
        bpm,
        changes: Vec::new(),
    }
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
    Some(NotatedDuration::single(MusicalDuration::new(value), text))
}

/// Resolve a pitch token: a literal, or a bound `pitch` parameter.
pub(crate) fn resolve_pitch(
    resolver: &mut Resolver,
    text: &str,
    node: &SyntaxNode,
    cx: &ExpandCx,
) -> Option<WrittenPitch> {
    let pitch = if let Some(pitch) = WrittenPitch::parse(text) {
        pitch
    } else if let Some(BoundValue::Pitch(pitch)) = cx.params.get(text) {
        *pitch
    } else {
        resolver.error(format!("unknown pitch reference `{text}`"), span_of(node));
        return None;
    };
    apply_intervals(resolver, pitch, cx, node)
}

/// Apply the transposition stack, outermost first.
pub(crate) fn apply_intervals(
    resolver: &mut Resolver,
    pitch: WrittenPitch,
    cx: &ExpandCx,
    node: &SyntaxNode,
) -> Option<WrittenPitch> {
    let mut current = pitch;
    for interval in &cx.intervals {
        let Some(next) = current.transpose(*interval) else {
            resolver.error(
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
pub(crate) fn resolve_duration(resolver: &mut Resolver, node: &SyntaxNode, cx: &ExpandCx) -> Option<NotatedDuration> {
    if let Some(duration) = parse_duration(node) {
        return Some(duration);
    }
    if let Some(text) = token_text(node, SyntaxKind::Identifier)
        && let Some(BoundValue::Duration(duration)) = cx.params.get(&text)
    {
        return Some(duration.clone());
    }
    resolver.error("missing or unresolved duration", span_of(node));
    None
}

/// Bind one argument text to a parameter kind.
pub(crate) fn bind_argument(
    resolver: &mut Resolver,
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
                resolver.error(format!("motif `{motif}`: `{text}` is not a pitch"), span_of(node));
                None
            }
        }
        "duration" => {
            if let Some(value) = parse_ratio(text).or_else(|| text.parse::<i64>().ok().map(Ratio::from_integer)) {
                Some(BoundValue::Duration(NotatedDuration::single(
                    MusicalDuration::new(value),
                    text,
                )))
            } else if let Some(BoundValue::Duration(duration)) = cx.params.get(text) {
                Some(BoundValue::Duration(duration.clone()))
            } else {
                resolver.error(format!("motif `{motif}`: `{text}` is not a duration"), span_of(node));
                None
            }
        }
        other => {
            resolver.error(
                format!("motif `{motif}`: unknown parameter kind `{other}`"),
                span_of(node),
            );
            None
        }
    }
}

pub(crate) fn check_measure_sanity(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
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
                resolver.diagnostics.push(Diagnostic::warning(
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
