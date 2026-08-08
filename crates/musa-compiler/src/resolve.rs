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
    ArticulationRule, AstNode as _, DynamicRule, FrontMatterRole, KeyStmt, PerformanceDecl, PieceDecl, SettingStmt,
    TempoStmt, VoiceItem,
};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;
use slotmap::{SlotMap, new_key_type};

use crate::diagnose::{Code, Diagnostic, nearest};
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

/// What kind of material a name was bound to.
///
/// Motifs and bars share one namespace because "material with a name" is one
/// idea, and a composer who mistypes a name should get one diagnostic that
/// knows about both. They differ only in how a diagnostic refers to them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Material {
    Motif,
    Bar,
}

impl Material {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Motif => "motif",
            Self::Bar => "bar",
        }
    }
}

/// A collected motif or named bar, ready for expansion.
pub(crate) struct MotifDef {
    pub(crate) params: Vec<musa_language::ast::Param>,
    pub(crate) body: Vec<VoiceItem>,
    pub(crate) declaration: DeclarationId,
    pub(crate) material: Material,
    /// Where the declaration is written.
    ///
    /// Motifs are all declared above the score, so nothing can use one before
    /// it exists. A bar is declared in the middle of the music, so the same
    /// guarantee has to be checked: a `use` that starts before this span ends
    /// is either forward reference or the bar quoting itself, and both are the
    /// same mistake seen from different sides.
    pub(crate) span: SourceSpan,
}

/// A parameter bound at a `use` site.
#[derive(Clone, Debug)]
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
    /// The prevailing meter, and whether the piece actually wrote it.
    ///
    /// Read-only once the header is lowered, and here rather than threaded
    /// through the expansion context because it is a fact about the piece
    /// rather than about the block being expanded. What a `bar` is checked
    /// against.
    pub(crate) meter: MeterMap,
    pub(crate) meter_written: bool,
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
            meter: MeterMap::default(),
            meter_written: false,
            timeline_sink: None,
        }
    }

    pub(crate) fn declare(&mut self, info: DeclInfo) -> DeclKey {
        self.declarations.insert(info)
    }

    /// The common shape: a code, the claim, the place, and what is wrong
    /// there. Anything that also wants help, a note, a second place, or a fix
    /// builds the diagnostic and hands it to [`Self::report`].
    pub(crate) fn error(&mut self, code: Code, message: impl Into<String>, span: SourceSpan, label: impl Into<String>) {
        self.report(Diagnostic::error(code, message).at(span, label));
    }

    pub(crate) fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
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
/// The span of a node's first token of `kind`.
///
/// What a label wants: `soprano`, not the whole `clef soprano;` statement with
/// the newline in front of it.
pub(crate) fn token_span(node: &SyntaxNode, kind: SyntaxKind) -> Option<SourceSpan> {
    let range = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == kind)?
        .text_range();
    Some(SourceSpan::new(u32::from(range.start()), u32::from(range.end())))
}

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
    let parts: Vec<String> = snapshot
        .parts()
        .iter()
        .map(|(_, part)| part.name().to_string())
        .collect();
    crate::studio::resolve(studio.as_ref(), imported, &parts, &mut resolver.diagnostics)
}

/// Tempo, meter, key — plus registration of motif declarations (expansion
/// is prompt 06).
pub(crate) fn lower_header(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    snapshot.set_title(piece.name().unwrap_or_default());
    if let Some(tempo) = piece.tempo() {
        resolver.declare(DeclInfo::Tempo);
        *snapshot.tempo_mut() = parse_tempo(resolver, &tempo);
    }
    // A second tempo without a position would leave two answers to "how fast
    // does this piece start" — the one thing a header may not do.
    for extra in piece.tempos().iter().filter(|tempo| tempo.position().is_none()).skip(1) {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this piece already says how fast it starts")
                .at(trimmed_span(extra.syntax()), "a second starting tempo")
                .help("write `at <measure>:<beat>` to change the tempo partway through")
                .note("a piece has one starting tempo; every later one is a change, and a change needs a place"),
        );
    }
    if let Some(meter) = piece.meter() {
        resolver.declare(DeclInfo::Meter);
        if let Some(map) = parse_meter(&meter) {
            snapshot.set_meter(map);
            resolver.meter = map;
            resolver.meter_written = true;
        } else {
            resolver.error(
                Code::NotAValue,
                "this meter cannot be read",
                span_of(meter.syntax()),
                "expected two numbers, like `4/4`",
            );
        }
    }
    if let Some(key) = piece.key() {
        resolver.declare(DeclInfo::Key);
        match parse_key(&key) {
            Some(map) => snapshot.set_key(Some(map)),
            None => resolver.error(
                Code::NotAValue,
                "this key cannot be read",
                span_of(key.syntax()),
                "expected a note and a mode, like `a minor`",
            ),
        }
    }
    lower_front_matter(resolver, piece, snapshot);
    if let Some(performance) = piece.performance() {
        let profiles = parse_profiles(resolver, &performance);
        merge_profiles(resolver, snapshot, &profiles, span_of(performance.syntax()), None);
    }
    register_motifs(resolver, snapshot, &piece.motifs(), None);
}

/// `composer`, `arranger`, `subtitle`, `copyright`.
///
/// Written twice is an error rather than a silent last-wins: a page whose
/// composer depends on which line the engraver read is worse than a page that
/// refuses to compile.
fn lower_front_matter(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    for statement in piece.front_matter() {
        let Some(role) = statement.role() else { continue };
        let text = statement.text().unwrap_or_default();
        let slot = match role {
            FrontMatterRole::Subtitle => &mut snapshot.front_matter_mut().subtitle,
            FrontMatterRole::Composer => &mut snapshot.front_matter_mut().composer,
            FrontMatterRole::Arranger => &mut snapshot.front_matter_mut().arranger,
            FrontMatterRole::Copyright => &mut snapshot.front_matter_mut().copyright,
        };
        if slot.is_some() {
            let word = match role {
                FrontMatterRole::Subtitle => "subtitle",
                FrontMatterRole::Composer => "composer",
                FrontMatterRole::Arranger => "arranger",
                FrontMatterRole::Copyright => "copyright",
            };
            resolver.error(
                Code::Misplaced,
                format!("this piece already names a {word}"),
                span_of(statement.syntax()),
                format!("a second {word}"),
            );
            continue;
        }
        *slot = Some(text);
    }
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
        let span = trimmed_span(motif.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, from) {
            continue;
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: motif.params(),
            body: motif.items(),
            declaration: ordinal(resolver, key),
            material: Material::Motif,
            span,
        };
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name, definition);
    }
}

/// Register every named `bar` in the score, in source order.
///
/// A bar is declared where it sounds, which is inside a voice, so this walks
/// the score rather than reading a list off the piece. Registration happens
/// before any voice is elaborated for the same reason a motif's does: a name
/// has to exist before the thing that plays it is read. Whether a `use` is
/// *allowed* to reach a given bar is a separate question, and one the spans
/// answer — see [`MotifDef::span`].
pub(crate) fn register_bars(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    score: &musa_language::ast::ScoreDecl,
) {
    for bar in named_bars(score) {
        let Some(name) = bar.name() else { continue };
        let span = trimmed_span(bar.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, None) {
            continue;
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: Vec::new(),
            body: bar.items(),
            declaration: ordinal(resolver, key),
            material: Material::Bar,
            span,
        };
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name, definition);
    }
}

/// Every named bar in the score, outermost first and in source order.
fn named_bars(score: &musa_language::ast::ScoreDecl) -> Vec<musa_language::ast::BarStmt> {
    let mut found = Vec::new();
    for node in score.syntax().descendants() {
        if node.kind() == SyntaxKind::BarStmt
            && let Some(bar) = musa_language::ast::BarStmt::cast(node)
            && bar.name().is_some()
        {
            found.push(bar);
        }
    }
    found
}

/// Report a name that is already taken, and say where by.
///
/// Two declarations of one name is always an error — an imported name that
/// quietly lost to a local one would make a piece sound different depending on
/// what it imported — and a bar shares the rule because it shares the
/// namespace.
fn refuses_to_shadow(
    resolver: &mut Resolver,
    snapshot: &ScoreSnapshot,
    name: &str,
    span: SourceSpan,
    from: Option<&str>,
) -> bool {
    if !resolver.motifs.contains_key(name) {
        return false;
    }
    let first = snapshot
        .motifs()
        .iter()
        .find(|declared| declared.name == name)
        .map(|declared| declared.span);
    resolver.report(
        Diagnostic::error(
            Code::DuplicateName,
            match from {
                Some(path) => format!("`{path}` also declares `{name}`"),
                None => format!("`{name}` is declared twice"),
            },
        )
        .at(span, "declared again here")
        .maybe_also(first, "first declared here")
        .help("rename one of them, or delete this declaration")
        .note("musa has no shadowing: a name means one thing everywhere the piece can see it"),
    );
    true
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
        if snapshot.profiles().declares(&name) {
            resolver.report(
                Diagnostic::error(
                    Code::DuplicateName,
                    match from {
                        Some(path) => format!("`{path}` also declares profile `{name}`"),
                        None => format!("profile `{name}` is declared twice"),
                    },
                )
                .at(span, "declared again here")
                .help("rename one of them, or delete this declaration"),
            );
            continue;
        }
        if let Some(profile) = profiles.get(&name) {
            snapshot.profiles_mut().insert(profile.clone());
        }
    }
}

/// "did you mean", or the list, or nothing.
///
/// One suggestion when one candidate stands out; otherwise the vocabulary
/// itself, which for a closed set is short enough to print and is what the
/// reader actually needs. A list of more than six is neither, and says so.
pub(crate) fn suggest(written: &str, known: &[&str], plural: &str) -> String {
    if let Some(near) = nearest(written, known.iter().copied()) {
        return format!("did you mean `{near}`?");
    }
    if known.is_empty() {
        return format!("this piece declares no {plural}");
    }
    if known.len() > 6 {
        return format!("run `musa explain unknown-word` for the {plural} musa reads");
    }
    let quoted: Vec<String> = known.iter().map(|name| format!("`{name}`")).collect();
    format!("musa reads {}", quoted.join(", "))
}

/// The same, for a name the *composer* chose rather than a word musa knows.
///
/// The difference is whose vocabulary is at fault. `musa reads f, mf, p` is
/// the right answer for a misspelled dynamic and the wrong one for a
/// misspelled motif, where the list is the piece's own.
pub(crate) fn suggest_name(written: &str, known: &[&str]) -> String {
    if let Some(near) = nearest(written, known.iter().copied()) {
        return format!("did you mean `{near}`?");
    }
    if known.is_empty() {
        return "this piece declares no motifs and no bars".to_owned();
    }
    if known.len() > 6 {
        return "check the spelling against the declaration".to_owned();
    }
    let quoted: Vec<String> = known.iter().map(|name| format!("`{name}`")).collect();
    format!("this piece declares {}", quoted.join(", "))
}

/// Read the `performance` block into a [`ProfileSet`]. Declarations only —
/// nothing here is applied until `lower_performance` (roadmap §6.4).
pub(crate) fn parse_profiles(resolver: &mut Resolver, performance: &PerformanceDecl) -> ProfileSet {
    let mut set = ProfileSet::default();
    for declaration in performance.profiles() {
        let name = declaration.name().unwrap_or_default();
        if set.declares(&name) {
            resolver.error(
                Code::DuplicateName,
                format!("profile `{name}` is declared twice"),
                trimmed_span(declaration.syntax()),
                "declared again here",
            );
            continue;
        }
        let mut profile = PerformanceProfile::named(&name);
        for rule in declaration.articulations() {
            let written = rule.mark().unwrap_or_default();
            let Some(mark) = ArticulationMark::parse(&written) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{written}` is not an articulation"))
                        .at(trimmed_span(rule.syntax()), "unknown articulation")
                        .help(suggest(&written, ArticulationMark::NAMES, "articulations")),
                );
                continue;
            };
            profile.set_articulation(mark, articulation_settings(resolver, &rule));
        }
        for rule in declaration.dynamics() {
            let written = rule.mark().unwrap_or_default();
            let Some(mark) = DynamicMark::parse(&written) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a dynamic marking"))
                        .at(trimmed_span(rule.syntax()), "unknown marking")
                        .help(suggest(&written, DynamicMark::NAMES, "markings")),
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
            other => resolver.report(
                Diagnostic::error(
                    Code::UnknownWord,
                    format!("an articulation has no setting called `{other}`"),
                )
                .at(trimmed_span(setting.syntax()), "unknown setting")
                .help(suggest(other, &["gate", "attack"], "settings")),
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
            resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("a dynamic has no setting called `{name}`"))
                    .at(trimmed_span(setting.syntax()), "unknown setting")
                    .help("a dynamic rule sets `amplitude`, and nothing else"),
            );
        }
    }
    if amplitude.is_none() {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "this dynamic rule says nothing")
                .at(trimmed_span(rule.syntax()), "no amplitude")
                .help("give it an amplitude, like `amplitude = 0.6;`")
                .note("amplitude is abstract loudness between 0 and 1, not decibels"),
        );
    }
    amplitude
}

/// A unitless ratio in `0..=1`. A unit here is a category error: a gate is a
/// fraction of the written value, not a length of time.
fn ratio_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note(format!(
                    "`{name}` is a fraction of the written value, not a length of time"
                )),
        );
        return None;
    }
    let value = crate::profile::parse_decimal(&setting.value()?)?;
    if value < Ratio::ZERO || value > Ratio::ONE {
        resolver.error(
            Code::OutOfRange,
            format!("`{name}` is outside 0 to 1"),
            span,
            "out of range",
        );
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
            resolver.report(
                Diagnostic::error(Code::OutOfRange, format!("`{name}` is a length of time"))
                    .at(span, "no unit here")
                    .help(format!("write `{name} = 30 ms` or `{name} = 0.03 s`")),
            );
            return None;
        }
    };
    if seconds < Ratio::ZERO {
        resolver.error(
            Code::OutOfRange,
            format!("`{name}` cannot be negative"),
            span,
            "below zero",
        );
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
        let written = token_text(&node, SyntaxKind::Identifier).unwrap_or_default();
        match Clef::parse(&written) {
            Some(parsed) => clef = Some(parsed),
            None => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a clef"))
                    .at(
                        token_span(&node, SyntaxKind::Identifier).unwrap_or_else(|| trimmed_span(&node)),
                        "not a clef musa reads",
                    )
                    .help(suggest(&written, Clef::NAMES, "clefs")),
            ),
        }
    }
    let mut profile = None;
    if let Some(statement) = part.profile() {
        let name = statement.name().unwrap_or_default();
        if profiles.declares(&name) {
            profile = Some(name);
        } else {
            let known: Vec<&str> = profiles.names().collect();
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("cannot find profile `{name}`"))
                    .at(trimmed_span(statement.syntax()), "not declared in this piece")
                    .help(suggest(&name, &known, "profiles")),
            );
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
            resolver.report(
                Diagnostic::error(Code::NotAValue, "this tempo has no speed")
                    .at(trimmed_span(syntax), "expected a number")
                    .help("write `tempo quarter = 72;`"),
            );
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
    Some(MeterMap::new(numerator.parse().ok()?, denominator.parse().ok()?))
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
    Some(KeyMap::new(tonic, mode))
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
        resolver.report(
            Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
                .at(trimmed_span(node), "expected a pitch")
                .note("a pitch is a letter, an optional `s` or `f`, and an octave: `c4`, `gs5`, `bf3`"),
        );
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
            resolver.report(
                Diagnostic::error(
                    Code::OutOfRange,
                    format!("`{current}` cannot be spelled after this transposition"),
                )
                .at(trimmed_span(node), "would need a triple accidental")
                .help("transpose by a different interval, or write the passage out"),
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
    resolver.report(
        Diagnostic::error(Code::NotAValue, "this note has no duration")
            .at(trimmed_span(node), "expected a duration")
            .note("a duration is a fraction or a whole number of whole notes: `1/4`, `3/8`, `1`"),
    );
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
                resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
                        .at(trimmed_span(node), format!("passed to motif `{motif}`"))
                        .note("a pitch is a letter, an optional `s` or `f`, and an octave: `c4`, `gs5`, `bf3`"),
                );
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
                resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{text}` is not a duration"))
                        .at(trimmed_span(node), format!("passed to motif `{motif}`")),
                );
                None
            }
        }
        other => {
            resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{other}` is not a kind of parameter"))
                    .at(trimmed_span(node), format!("declared by motif `{motif}`"))
                    .help("a motif parameter is a `pitch` or a `duration`"),
            );
            None
        }
    }
}

pub(crate) fn check_measure_sanity(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    let bars = snapshot.bars();
    if !bars.is_measured() {
        return;
    }
    for (_, part) in snapshot.parts().iter() {
        for (voice_id, voice) in part.voices() {
            let span = voice.span();
            let stops = bars.at(crate::MusicalTime::ZERO + span);
            if span.as_ratio() != Ratio::ZERO && stops.into != crate::MusicalDuration::ZERO {
                let name = part.voice_name(voice_id).unwrap_or("?");
                // No span: this is a fact about a whole voice, and pointing
                // at its first note would send the reader somewhere the
                // mistake probably is not.
                // How far past the last barline the voice stops, said in the
                // units the composer writes durations in.
                let measure = bars.measure_at(crate::MusicalTime::ZERO + span);
                let finished = stops.measure;
                let short = (measure.end - (crate::MusicalTime::ZERO + span)).as_ratio();
                resolver.report(
                    Diagnostic::warning(
                        Code::DoesNotAddUp,
                        format!(
                            "voice `{name}` in part `{}` stops part-way through measure {}",
                            part.name(),
                            finished.max(1),
                        ),
                    )
                    .help(format!(
                        "it is {short} short — add a rest, or check the durations above"
                    ))
                    .note("a short last measure puts every part after it out of step"),
                );
            }
        }
    }
}
