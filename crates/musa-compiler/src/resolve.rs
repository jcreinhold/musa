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
    AstNode as _, DynamicRule, FrontMatterRole, KeyStmt, MarkRule, PerformanceDecl, PieceDecl, ProfileDecl,
    SettingStmt, TempoStmt, VoiceItem,
};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;
use slotmap::{SlotMap, new_key_type};

use crate::diagnose::{Code, Diagnostic, nearest};
use crate::origin::{DeclarationId, ExpansionStep, Interval, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::profile::{ArticulationRealization, PerformanceProfile, ProfileSet};
use crate::score::{AnnotationStore, Clef, DynamicMark, EventId, Key, Meter, Mode, NotatedDuration, ScoreSnapshot};
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
    /// Material a *performance* arranges rather than a composer reuses: it
    /// may be reordered by a `mobile` and repeated a chosen number of times.
    /// One namespace with the other two, so `use` reaches all three.
    Fragment,
}

impl Material {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Motif => "motif",
            Self::Bar => "bar",
            Self::Fragment => "fragment",
        }
    }

    /// The kind the reference record files this material under.
    pub(crate) fn name_kind(self) -> NameKind {
        match self {
            Self::Motif => NameKind::Motif,
            Self::Bar => NameKind::Bar,
            Self::Fragment => NameKind::Fragment,
        }
    }
}

/// What kind of thing a recorded name names (prompt 78).
///
/// Values and functions share the elaboration-value namespace. Motifs, bars,
/// and fragments share the material namespace (see [`Material`]); parts,
/// voices, and patches are a namespace each. The kind is what a rename checks
/// a collision against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameKind {
    /// An immutable elaboration `let` binding.
    Value,
    /// A named elaboration function.
    Function,
    /// A `motif` declaration.
    Motif,
    /// A named `bar`.
    Bar,
    /// A `fragment` a mobile arranges.
    Fragment,
    /// A `part` in the score.
    Part,
    /// A `voice` in a part. Voices are declared, never used by name — their
    /// entries are declaration-only by construction.
    Voice,
    /// A `patch` in the studio.
    Patch,
}

/// One named thing and everywhere it is spoken in the compiled document
/// (prompt 78).
///
/// Spans are the *name tokens'* spans, not the statements': a rename rewrites
/// exactly these ranges and nothing around them. A name declared in an
/// imported library has no declaration here — its uses in the document are
/// recorded honestly and its declaration is `None`, which is what makes
/// cross-file rename impossible to ask for rather than silently wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameReference {
    /// The name as written.
    pub name: String,
    /// What it names.
    pub kind: NameKind,
    /// Where the declaration's name token is, when it is in this document.
    pub declaration: Option<SourceSpan>,
    /// Every resolved use's name token, in the order the resolver met them.
    /// A name that does not resolve records nothing: no uses, no entry.
    pub uses: Vec<SourceSpan>,
}

/// The reference record the resolver accumulates (prompt 78).
///
/// The resolver already knows every use's declaration at the moment it
/// resolves the name; this is that knowledge kept, not a second pass
/// re-derived afterwards. Entries are few — a piece names dozens of things —
/// so a `Vec` scanned linearly beats an index that has to be kept true.
pub(crate) struct ReferenceIndex {
    entries: Vec<NameReference>,
}

impl Default for ReferenceIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceIndex {
    pub(crate) fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Record a declaration in the compiled document.
    ///
    /// Always a new entry: declarations reach here only past the duplicate
    /// checks, so a repeated `(kind, name)` is two voices in two parts, not
    /// a collision.
    pub(crate) fn declare(&mut self, kind: NameKind, name: &str, span: SourceSpan) {
        self.entries.push(NameReference {
            name: name.to_owned(),
            kind,
            declaration: Some(span),
            uses: Vec::new(),
        });
    }

    /// Record one resolved use.
    ///
    /// The entry is found by kind and name — unique in every namespace that
    /// has uses (voices are declaration-only). A use whose declaration lives
    /// in an imported library has no entry yet: it is created with a `None`
    /// declaration, which is the record's way of saying "used here, spelled
    /// elsewhere".
    pub(crate) fn record_use(&mut self, kind: NameKind, name: &str, span: SourceSpan) {
        match self
            .entries
            .iter_mut()
            .find(|entry| entry.kind == kind && entry.name == name)
        {
            Some(entry) => entry.uses.push(span),
            None => self.entries.push(NameReference {
                name: name.to_owned(),
                kind,
                declaration: None,
                uses: vec![span],
            }),
        }
    }

    /// Everything recorded, in the order it was first met.
    pub(crate) fn entries(&self) -> &[NameReference] {
        &self.entries
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
    /// Whether the declaration lives in an imported library.
    ///
    /// The reference record only records spans in the compiled document's own
    /// text; a use inside a foreign body is elaborated through an
    /// [`ExpandCx`] this flag marks, so nothing foreign-text ever reaches the
    /// record.
    pub(crate) foreign: bool,
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
    /// The named place these items sit in — part, voice, motif, bar.
    ///
    /// The prefix of every [`crate::ChoicePath`] built here. A motif body
    /// resets it to the motif's own name, exactly as `path` is reset: the
    /// body is elaborated once and shared between call sites, so a decision
    /// inside it belongs to the *material* and not to any one use of it.
    pub(crate) choice: crate::ChoicePath,
    /// Whether the text being read belongs to an imported library rather
    /// than the compiled document.
    ///
    /// The reference record (prompt 78) records spans in the document's own
    /// text only, so a `use` read under a foreign context is resolved but
    /// never recorded. Set by `expand_material` when the body being expanded
    /// came from an import; the piece's own root context is never foreign.
    pub(crate) foreign: bool,
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
    pub(crate) meter: Meter,
    pub(crate) meter_written: bool,
    /// Where elaboration has reached, in the piece's own time.
    ///
    /// Maintained by `elaborate_items`, which is the one place that knows how
    /// long each item is; everything that needs to know *where* it is —
    /// a `meter` statement, a bar waiting to be measured — reads it here
    /// rather than recomputing a sum that has already been computed.
    pub(crate) cursor: crate::MusicalTime,
    /// Every mid-piece `meter`, in the order the voices were read.
    ///
    /// Collected rather than applied, because the meters have to be sorted
    /// and folded before any of them can be checked: whether a change lands
    /// on a barline is a question about the changes before it.
    pub(crate) meter_changes: Vec<(crate::MusicalTime, Meter, SourceSpan)>,
    /// The parts that declared a meter of their own — polymeter.
    ///
    /// Every check about barlines has to ask *whose* barlines, and the
    /// projection that would answer it does not exist until elaboration is
    /// over. Empty for every piece that is not polymetric, which is why the
    /// checks fall back to the piece's barlines rather than branching.
    pub(crate) part_meters: std::collections::BTreeMap<u32, Meter>,
    /// Every profile that declares a groove that is not straight, with the
    /// rule that declares it — kept so the check that a groove has a meter to
    /// swing against can point at the groove rather than at the music.
    pub(crate) groove_rules: Vec<(String, SourceSpan)>,
    /// Every mid-piece `key`, likewise.
    ///
    /// A modulation is a barline event, so it is checked against the
    /// barlines — which the meters decide — and therefore cannot be checked
    /// where it is written either.
    pub(crate) key_changes: Vec<(crate::MusicalTime, Key, SourceSpan)>,
    /// Bars whose length is still to be checked.
    ///
    /// A bar is checked against the meter in force where it sits, and where
    /// the meters change is not known until every voice has been read — so
    /// the check waits, and `elaborate_score` runs it once the barlines are
    /// known.
    pub(crate) pending_bars: Vec<crate::elaborate::PendingBar>,
    /// How many decision sites have been seen inside each named place, so the
    /// next one there knows its ordinal.
    pub(crate) sites: std::collections::BTreeMap<crate::ChoicePath, u32>,
    /// Every decision this compile took, in the order the sites were reached.
    pub(crate) decisions: Vec<crate::DecisionRecord>,
    /// The key the header wrote, on its way into the timeline.
    ///
    /// Staged here rather than on the snapshot because the snapshot's answer
    /// to "what key is this" is the *projection* of the timeline, and a field
    /// that held the header's reading until the projection overwrote it would
    /// be a second answer with a window in which it was the live one.
    pub(crate) key: Option<Key>,
    /// Where each voice's kernel timeline goes on its way to the adapter.
    ///
    /// `None` on every production path — nothing keeps a timeline after the
    /// snapshot is built. It is `Some` only under `crate::bench`, which needs
    /// the elaboration and projection stages separable to measure them apart
    /// (roadmap §17.7). One `Option` check per voice is the whole cost.
    pub(crate) timeline_sink: Option<Vec<crate::elaborate::VoiceTimeline>>,
    /// Every name reference resolved, kept for editors (prompt 78).
    pub(crate) references: ReferenceIndex,
    /// Which performance is being compiled (`docs/kernel/11-realization.md`).
    ///
    /// It lives here rather than being threaded through elaboration because a
    /// decision site can be anywhere a note can be, and every function on the
    /// way already carries the resolver.
    pub(crate) realization: crate::Realization,
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
            meter: Meter::default(),
            meter_written: false,
            cursor: crate::MusicalTime::ZERO,
            meter_changes: Vec::new(),
            part_meters: std::collections::BTreeMap::new(),
            groove_rules: Vec::new(),
            key_changes: Vec::new(),
            pending_bars: Vec::new(),
            sites: std::collections::BTreeMap::new(),
            references: ReferenceIndex::new(),
            decisions: Vec::new(),
            key: None,
            timeline_sink: None,
            realization: crate::Realization::deterministic(),
        }
    }

    /// The path of the next decision site inside `place`, and the decision
    /// the realization makes there.
    ///
    /// The ordinal is per named place, so a site in one voice is unaffected by
    /// sites added in another — and inside a place, a site added *below*
    /// leaves the ones above it alone. Both are the point of
    /// `docs/kernel/11-realization.md`'s path identity.
    pub(crate) fn decide_count(
        &mut self,
        place: &crate::ChoicePath,
        least: u32,
        most: u32,
        site: SourceSpan,
    ) -> (crate::ChoicePath, u32) {
        let path = self.site(place);
        let count = self.realization.count(&path, least, most);
        let passes = if count == 1 { "pass" } else { "passes" };
        self.decided(
            path.clone(),
            crate::Decision::Count(count),
            site,
            format!("{count} {passes}"),
        );
        (path, count)
    }

    /// The order a mobile's fragments are played in: a permutation of
    /// `0..fragments.len()`.
    pub(crate) fn decide_order(
        &mut self,
        place: &crate::ChoicePath,
        fragments: &[String],
        site: SourceSpan,
    ) -> Vec<u32> {
        let path = self.site(place);
        let count = u32::try_from(fragments.len()).unwrap_or(u32::MAX);
        let order = self.realization.order(&path, count);
        // By name, because `[2, 0, 1]` is not something to show anyone and
        // the names are only known here (`docs/interface/03-interaction.md` §7).
        let played: Vec<&str> = order
            .iter()
            .filter_map(|index| fragments.get(*index as usize).map(String::as_str))
            .collect();
        self.decided(path, crate::Decision::Order(order.clone()), site, played.join(", "));
        order
    }

    /// How long a freely-held note actually sounds, between the written value
    /// and the longest it may be held.
    pub(crate) fn decide_duration(
        &mut self,
        place: &crate::ChoicePath,
        least: Ratio<i64>,
        most: Ratio<i64>,
        site: SourceSpan,
    ) -> Ratio<i64> {
        let path = self.site(place);
        let held = self.realization.duration(&path, least, most);
        let answered = format!("held {}/{}", held.numer(), held.denom());
        self.decided(path, crate::Decision::Duration(held), site, answered);
        held
    }

    /// The path of the next decision site inside `place`.
    fn site(&mut self, place: &crate::ChoicePath) -> crate::ChoicePath {
        let ordinal = self.sites.entry(place.clone()).or_insert(0);
        let path = place.then(crate::ChoiceStep::Ordinal(*ordinal));
        *ordinal = ordinal.saturating_add(1);
        path
    }

    /// Record what was decided at a site.
    ///
    /// One site, one decision, however many voices reach it: the k-th site in
    /// every voice *is* the k-th site, which is the point of numbering them
    /// per voice rather than per path down from the part.
    fn decided(&mut self, path: crate::ChoicePath, decision: crate::Decision, site: SourceSpan, answered: String) {
        if let Some(already) = self.decisions.iter_mut().find(|record| record.path == path) {
            if !already.sites.contains(&site) {
                already.sites.push(site);
            }
            return;
        }
        let pinned = self.realization.is_pinned(&path);
        self.decisions.push(crate::DecisionRecord {
            path,
            decision,
            sites: vec![site],
            answered,
            pinned,
        });
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

/// Span of the first token of `kind` anywhere below `node`.
///
/// Most compiler nodes keep their named token as a direct child and should
/// use [`token_span`]. Expression syntax nests a `use` callee under
/// `NameExpr`/`ApplyExpr`, so that one bridge asks explicitly for descent.
pub(crate) fn descendant_token_span(node: &SyntaxNode, kind: SyntaxKind) -> Option<SourceSpan> {
    let range = node
        .descendants_with_tokens()
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

/// The span of a token, in the record's measure.
pub(crate) fn source_span_of(token: &musa_language::SyntaxToken) -> SourceSpan {
    let range = token.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
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
    crate::studio::resolve(
        studio.as_ref(),
        imported,
        &parts,
        &mut resolver.references,
        &mut resolver.diagnostics,
    )
}

/// Tempo, meter, key — plus registration of motif declarations (expansion
/// is prompt 06).
pub(crate) fn lower_header(resolver: &mut Resolver, piece: &PieceDecl, snapshot: &mut ScoreSnapshot) {
    snapshot.set_title(piece.name().unwrap_or_default());
    if piece.tempo().is_some() {
        resolver.declare(DeclInfo::Tempo);
    }
    // A second tempo in the header would leave two answers to "how fast does
    // this piece start" — the one thing a header may not do. The reading
    // itself happens in `elaborate.rs`, where the marking enters the timeline.
    for extra in piece.tempos().iter().skip(1) {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this piece already says how fast it starts")
                .at(trimmed_span(extra.syntax()), "a second starting tempo")
                .help("write the change in the voice that reaches it, like a meter or a key change")
                .note("a piece has one starting tempo; every later one is a place in the music"),
        );
    }
    if let Some(meter) = piece.meter() {
        resolver.declare(DeclInfo::Meter);
        if let Some(map) = parse_meter(&meter) {
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
            Some(map) => resolver.key = Some(map),
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
    register_fragments(resolver, snapshot, &piece.fragments(), None);
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
        if from.is_none()
            && !name.is_empty()
            && let Some(name_span) = token_span(motif.syntax(), SyntaxKind::Identifier)
        {
            resolver.references.declare(NameKind::Motif, &name, name_span);
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: motif.params(),
            body: motif.items(),
            declaration: ordinal(resolver, key),
            material: Material::Motif,
            span,
            foreign: from.is_some(),
        };
        snapshot.push_motif(crate::score::MotifDeclaration {
            name: name.clone(),
            span,
        });
        resolver.motifs.insert(name, definition);
    }
}

/// Register the fragment declarations, refusing to shadow.
///
/// Separate from [`register_motifs`] only because the AST nodes differ: a
/// fragment takes no parameters, so it is not a motif whose parameter list
/// happens to be empty — it is material with nothing to substitute into.
pub(crate) fn register_fragments(
    resolver: &mut Resolver,
    snapshot: &mut ScoreSnapshot,
    fragments: &[musa_language::ast::FragmentDecl],
    from: Option<&str>,
) {
    for fragment in fragments {
        let name = fragment.name().unwrap_or_default();
        let span = trimmed_span(fragment.syntax());
        if refuses_to_shadow(resolver, snapshot, &name, span, from) {
            continue;
        }
        if from.is_none()
            && !name.is_empty()
            && let Some(name_span) = token_span(fragment.syntax(), SyntaxKind::Identifier)
        {
            resolver.references.declare(NameKind::Fragment, &name, name_span);
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: Vec::new(),
            body: fragment.items(),
            declaration: ordinal(resolver, key),
            material: Material::Fragment,
            span,
            foreign: from.is_some(),
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
        if let Some(name_span) = token_span(bar.syntax(), SyntaxKind::Identifier) {
            resolver.references.declare(NameKind::Bar, &name, name_span);
        }
        let key = resolver.declare(DeclInfo::Motif);
        let definition = MotifDef {
            params: Vec::new(),
            body: bar.items(),
            declaration: ordinal(resolver, key),
            material: Material::Bar,
            span,
            foreign: false,
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
        for rule in declaration.marks() {
            let written = rule.name().unwrap_or_default();
            let Some(mark) = crate::Mark::parse(&written) else {
                resolver.report(
                    Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a mark"))
                        .at(trimmed_span(rule.syntax()), "unknown mark")
                        .help(suggest(&written, &crate::marks::names(), "marks")),
                );
                continue;
            };
            profile.set_mark(mark, mark_settings(resolver, &rule));
        }
        for rule in declaration.dynamics() {
            let written = rule.name().unwrap_or_default();
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
        if let Some((groove, span)) = groove_of(resolver, &declaration) {
            if !groove.is_straight() {
                resolver.groove_rules.push((profile.name().to_owned(), span));
            }
            profile.set_groove(groove);
        }
        if let Some(grace) = grace_of(resolver, &declaration) {
            profile.set_grace(grace);
        }
        set.insert(profile);
    }
    set
}

/// The one grace reading a profile declares, if it declares one.
///
/// More than one is refused for the same reason two grooves are: a reading of
/// a grace note is a single decision, and two of them composed in written
/// order would mean nothing a performer could act on.
fn grace_of(resolver: &mut Resolver, declaration: &ProfileDecl) -> Option<crate::GracePolicy> {
    let rules = declaration.graces();
    let (first, rest) = rules.split_first()?;
    for extra in rest {
        resolver.error(
            Code::DuplicateName,
            "this profile has more than one grace rule",
            trimmed_span(extra.syntax()),
            "a reading plays a grace note one way",
        );
    }
    let mut policy = crate::GracePolicy::DEFAULT;
    for setting in first.settings() {
        let name = setting.name().unwrap_or_default();
        match name.as_str() {
            "steal" => {
                if let Some(steal) = beat_setting(resolver, &setting) {
                    if steal <= Ratio::ZERO {
                        resolver.error(
                            Code::OutOfRange,
                            "`steal` is not a length",
                            trimmed_span(setting.syntax()),
                            "zero or less",
                        );
                    } else {
                        policy.steal = steal;
                    }
                }
            }
            "from" => {
                if let Some(from) = steal_from(resolver, &setting) {
                    policy.from = from;
                }
            }
            other => resolver.report(
                Diagnostic::error(
                    Code::UnknownWord,
                    format!("a grace rule has no setting called `{other}`"),
                )
                .at(trimmed_span(setting.syntax()), "unknown setting")
                .help(suggest(other, &["steal", "from"], "settings")),
            ),
        }
    }
    Some(policy)
}

/// `from = principal;` or `from = previous;`.
fn steal_from(resolver: &mut Resolver, setting: &SettingStmt) -> Option<crate::StealFrom> {
    let written = setting.word().or_else(|| setting.value())?;
    match written.as_str() {
        "principal" => Some(crate::StealFrom::Principal),
        "previous" => Some(crate::StealFrom::Previous),
        other => {
            resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{other}` is not a note to steal from"))
                    .at(trimmed_span(setting.syntax()), "unknown source")
                    .help(suggest(other, &["principal", "previous"], "sources"))
                    .note("a grace note takes its time from the note it leans on, or from the one before it"),
            );
            None
        }
    }
}

/// The one groove a profile declares, if it declares one.
///
/// More than one is refused rather than merged: a part has one beat, and two
/// grooves composed in written order would mean something no musician asked
/// for.
fn groove_of(resolver: &mut Resolver, declaration: &ProfileDecl) -> Option<(crate::Groove, SourceSpan)> {
    let rules = declaration.grooves();
    let (first, rest) = rules.split_first()?;
    for extra in rest {
        resolver.error(
            Code::DuplicateName,
            "this profile has more than one groove",
            trimmed_span(extra.syntax()),
            "a part has one beat",
        );
    }
    let written = first.name().unwrap_or_default();
    let Some(def) = crate::groove::lookup(&written) else {
        resolver.report(
            Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a groove"))
                .at(trimmed_span(first.syntax()), "unknown groove")
                .help(suggest(&written, &crate::groove::names(), "grooves")),
        );
        return None;
    };
    let mut values: indexmap::IndexMap<String, Ratio<i64>> = indexmap::IndexMap::new();
    for setting in first.settings() {
        let name = setting.name().unwrap_or_default();
        if !def.params.contains(&name.as_str()) {
            resolver.report(
                Diagnostic::error(
                    Code::UnknownWord,
                    format!("`{}` has no setting called `{name}`", def.name),
                )
                .at(trimmed_span(setting.syntax()), "unknown setting")
                .help(suggest(&name, def.params, "settings")),
            );
            continue;
        }
        if let Some(value) = beat_setting(resolver, &setting) {
            values.insert(name, value);
        }
    }
    let mut required = |param: &str| match values.get(param) {
        Some(value) => Some(*value),
        None => {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("`{}` needs a `{param}`", def.name))
                    .at(trimmed_span(first.syntax()), format!("no `{param}`"))
                    .help(example_of(def.name)),
            );
            None
        }
    };
    let groove = match def.name {
        "straight" => Some(crate::Groove::STRAIGHT),
        "swing" => crate::Groove::swing(required("ratio")?),
        "push" => crate::Groove::push(required("grid")?, required("by")?),
        other => {
            // `VOCABULARY` and this match are the same list. A row added to
            // one and not the other would be a groove the parser accepts and
            // the compiler quietly ignores, so it is reported rather than
            // dropped on the floor.
            resolver.error(
                Code::UnknownWord,
                format!("`{other}` is a groove musa does not know how to build"),
                trimmed_span(first.syntax()),
                "not implemented",
            );
            None
        }
    };
    if groove.is_none() {
        resolver.report(
            Diagnostic::error(
                Code::OutOfRange,
                format!("this `{}` cannot be laid over the beat", def.name),
            )
            .at(trimmed_span(first.syntax()), "out of range")
            .note("a groove displaces the beat inside a cell it never leaves: it may not move a note past its neighbour, and its cell must tile the whole note or the downbeat it fixes would drift from bar to bar")
            .help(example_of(def.name)),
        );
    }
    groove.map(|groove| (groove, trimmed_span(first.syntax())))
}

/// What a groove looks like written correctly, for the diagnostics above.
fn example_of(name: &str) -> &'static str {
    match name {
        "swing" => "write `groove swing { ratio = 2/3; }` — the first of each pair takes two thirds",
        "push" => "write `groove push { grid = 1/8; by = -1/64; }` — the offbeat eighths land early",
        _ => "write `groove straight {}`",
    }
}

/// A position or a length in whole notes, exact and possibly negative: a
/// swing ratio, a grid, or a displacement. Not a gate, so `0..=1` is not the
/// range, and not a time, so a unit is a category error.
fn beat_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note("a groove is written in beats, so it survives a tempo change"),
        );
        return None;
    }
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let (sign, magnitude) = written
        .strip_prefix('-')
        .map_or((Ratio::ONE, written.as_str()), |rest| (-Ratio::ONE, rest));
    let value = parse_ratio(magnitude).or_else(|| crate::profile::parse_decimal(magnitude));
    match value {
        Some(value) => Some(value * sign),
        None => {
            resolver.error(
                Code::NotAValue,
                format!("`{written}` is not a beat value"),
                span,
                "not a ratio",
            );
            None
        }
    }
}

/// `gate` (a ratio of the written value) and `attack` (a time).
fn mark_settings(resolver: &mut Resolver, rule: &MarkRule) -> ArticulationRealization {
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
            "hold" => {
                if let Some(hold) = hold_setting(resolver, &setting) {
                    realization.hold = hold;
                }
            }
            other => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("a mark has no setting called `{other}`"))
                    .at(trimmed_span(setting.syntax()), "unknown setting")
                    .help(suggest(other, &["gate", "attack", "hold"], "settings")),
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

/// A multiplier of at least one: how much longer than written a note is held.
///
/// Not [`ratio_setting`], and the difference is the whole point of the
/// setting. A gate is a *fraction* of the written value and so lives in
/// `0..=1`; a hold is a *multiple* of it, and a fermata that lasts twice as
/// long is the ordinary case. Reusing the gate's reader would make `hold = 2`
/// — the one value anybody writes first — an error.
///
/// Written as a ratio or as a decimal, like a groove's beat: `hold = 2/1` and
/// `hold = 1.5` are both a person saying the same kind of thing.
fn hold_setting(resolver: &mut Resolver, setting: &SettingStmt) -> Option<Ratio<i64>> {
    let name = setting.name().unwrap_or_default();
    let span = trimmed_span(setting.syntax());
    if let Some(unit) = setting.unit() {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` does not take a unit"))
                .at(span, format!("drop the `{unit}`"))
                .note("a hold is a multiple of the written value, not a length of time: it survives a tempo change"),
        );
        return None;
    }
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = parse_ratio(&written).or_else(|| crate::profile::parse_decimal(&written)) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a hold"),
            span,
            "not a number",
        );
        return None;
    };
    if value < Ratio::ONE {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, format!("`{name}` is less than 1"))
                .at(span, "shorter than written")
                .note("a hold lengthens a note; to shorten one, write a `gate`"),
        );
        return None;
    }
    Some(value)
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
    // Both spellings, and a value that will not parse is reported rather than
    // dropped. `gate = 1/2` used to resolve to nothing at all — a half-length
    // staccato that compiled clean and performed at full length.
    // A word reaches here too, so a setting given the wrong kind of value
    // is refused by name rather than dropped without a word (see `word`).
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = parse_ratio(&written).or_else(|| crate::profile::parse_decimal(&written)) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a fraction of the written value"),
            span,
            "not a number",
        );
        return None;
    };
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
    let written = setting.value().or_else(|| setting.word())?;
    let Some(value) = crate::profile::parse_decimal(&written) else {
        resolver.error(
            Code::NotAValue,
            format!("`{written}` is not a length of time"),
            span,
            "not a number",
        );
        return None;
    };
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

/// What a part says about itself before any of it is played.
///
/// One struct rather than a tuple because there are now four answers and
/// three of them are optional: a caller reading `context.meter` cannot
/// mistake it for `context.clef`, which a four-tuple invites.
pub(crate) struct PartContext {
    /// The clef the part is read in.
    pub(crate) clef: Option<(Clef, SourceSpan)>,
    /// The part's own meter — polymeter, when it differs from the piece's.
    pub(crate) meter: Option<(Meter, SourceSpan)>,
    /// The part's own tempo — polytempo.
    pub(crate) tempo: Option<(crate::elaborate::FactKind, SourceSpan)>,
    /// The performance profile that realizes the part.
    pub(crate) profile: Option<String>,
}

/// The part-level facts both semantic paths read the same way.
pub(crate) fn part_context(
    resolver: &mut Resolver,
    part: &musa_language::ast::PartDecl,
    profiles: &ProfileSet,
) -> PartContext {
    let mut clef: Option<(Clef, SourceSpan)> = None;
    for node in part.syntax().children() {
        if node.kind() != SyntaxKind::ClefStmt {
            continue;
        }
        let written = token_text(&node, SyntaxKind::Identifier).unwrap_or_default();
        let span = trimmed_span(&node);
        match Clef::parse(&written) {
            // A part reads in one clef until the grammar can say where a
            // second one starts, so a second declaration is two answers to
            // one question rather than a change of clef. Last-wins was
            // silent, which meant the composer found out by looking at the
            // page.
            Some(parsed) => match clef {
                Some((_, first)) => resolver.report(
                    Diagnostic::error(Code::DuplicateName, "this part already says what clef it is in")
                        .at(span, "declared again here")
                        .also(first, "first declared here")
                        .help("write one `clef` per part, and `clef bass;` in a voice where it changes"),
                ),
                None => clef = Some((parsed, span)),
            },
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
    PartContext {
        clef,
        meter: part.meter().and_then(|stmt| {
            let span = trimmed_span(stmt.syntax());
            match parse_meter(&stmt) {
                Some(meter) => Some((meter, span)),
                None => {
                    resolver.error(
                        Code::NotAValue,
                        "this meter cannot be read",
                        span,
                        "expected `4/4`, or `none`",
                    );
                    None
                }
            }
        }),
        tempo: part
            .tempo()
            .map(|stmt| (tempo_fact(resolver, &stmt), trimmed_span(stmt.syntax()))),
        profile,
    }
}

/// What one `tempo` statement says, as the timeline carries it.
///
/// The three forms are read here and nowhere else, because "does this marking
/// change the clock" is one question and every consumer asks it the same way:
/// by looking for a [`crate::score::Metronome`].
pub(crate) fn tempo_fact(resolver: &mut Resolver, tempo: &TempoStmt) -> crate::elaborate::FactKind {
    let syntax = tempo.syntax();
    let text = tempo.text();
    let metronome = tempo.has_metronome().then(|| {
        let beat = tempo
            .beat()
            .and_then(|text| parse_ratio(&text))
            // `quarter` (or another beat name) resolves to 1/4 for now.
            .unwrap_or_else(|| Ratio::new(1, 4));
        let bpm = tempo
            .bpm()
            .and_then(|text| text.parse::<u32>().ok())
            .unwrap_or_else(|| {
                resolver.report(
                    Diagnostic::error(Code::NotAValue, "this tempo has no speed")
                        .at(trimmed_span(syntax), "expected a number")
                        .help("write `tempo quarter = 72;`"),
                );
                120
            });
        crate::score::Metronome { beat, bpm }
    });
    // No check that the marking says *something*: the grammar refuses
    // `tempo;` outright, and a file with a syntax error never reaches
    // elaboration. A diagnostic here would be one nothing could produce.
    crate::elaborate::FactKind::Tempo {
        metronome,
        ramp: tempo_ramp(resolver, tempo, text.is_some()),
        text,
    }
}

/// The gradual half of a tempo marking, when it has one.
///
/// Two halves that only mean something together: `to` says where the change
/// arrives, `over` says how far it reaches. One without the other is refused
/// rather than guessed at, because both guesses would be wrong — a reach with
/// no destination is not a change, and a destination with no reach is a jump
/// already written more simply without the word `to`.
fn tempo_ramp(resolver: &mut Resolver, tempo: &TempoStmt, printed: bool) -> Option<crate::score::Ramp> {
    let syntax = tempo.syntax();
    let arrives = tempo.ramp_to().and_then(|text| text.parse::<u32>().ok());
    let Some(over) = tempo.over().and_then(|text| parse_ratio(&text)) else {
        if arrives.is_some() {
            resolver.report(
                Diagnostic::error(Code::Misplaced, "this gradual tempo change has no reach")
                    .at(trimmed_span(syntax), "`to` without `over`")
                    .help("say how far it takes, like `tempo quarter = 120 to 60 over 4/1;`")
                    .note("a change with no reach is a change at a point, written without `to`"),
            );
        }
        return None;
    };
    if arrives.is_none() && !printed {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this gradual tempo change goes nowhere")
                .at(trimmed_span(syntax), "`over` with neither a destination nor a word")
                .help("write where it arrives (`to 60`) or what to print (`\"rit.\"`)"),
        );
        return None;
    }
    Some(crate::score::Ramp {
        to: arrives,
        over: crate::time::MusicalDuration::new(over),
        // The grammar writes no shape, so every ramp is a straight line — in
        // seconds per beat, which is where the evenness a listener hears
        // lives. The value is in the timeline rather than invented during
        // lowering, so a second implementation integrates the same curve.
        shape: musa_kernel::Progress::linear(),
    })
}

pub(crate) fn parse_meter(meter: &musa_language::ast::MeterStmt) -> Option<Meter> {
    if meter.is_unmeasured() {
        return Some(Meter::NONE);
    }
    let text = meter.value()?;
    let (numerator, denominator) = text.split_once('/')?;
    let (numerator, denominator): (u32, u32) = (numerator.parse().ok()?, denominator.parse().ok()?);
    // `4/0` is not a meter and `0/4` is spelled `none`: a fraction here has to
    // name real measures, or the barlines would fall nowhere by accident
    // rather than on purpose.
    if numerator == 0 || denominator == 0 {
        return None;
    }
    Some(Meter::new(numerator, denominator))
}

pub(crate) fn parse_key(key: &KeyStmt) -> Option<Key> {
    let syntax = key.syntax();
    // The tonic is a node, not a token: `bb` is one identifier and `g#` is
    // two tokens, and only the node knows it is one pitch class either way.
    let tonic = PitchClass::parse(
        syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::PitchClass)?
            .text()
            .to_string()
            .trim(),
    )?;
    let mut identifiers = syntax
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Identifier);
    let mode = match identifiers.next()?.text() {
        "major" => Mode::Major,
        "minor" => Mode::Minor,
        _ => return None,
    };
    Some(Key::new(tonic, mode))
}

pub(crate) fn parse_ratio(text: &str) -> Option<Ratio<i64>> {
    let (numerator, denominator) = text.split_once('/')?;
    Some(Ratio::new(numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Parse a statement's duration (`1/2`, `3/8`, `1`, `/4`, `/4.`) into value +
/// spelling.
///
/// It reads the statement's `Duration` node rather than its first numeral,
/// which is what makes the short form safe: the `4` in `c4/4` is a numeral
/// like any other, and a search for one would find the octave and call every
/// quarter a whole note without ever failing.
///
/// The short form is spelled out — `/4.` records `3/8` — because that spelling
/// reaches diagnostics, the desktop inspector and every kernel golden, and one
/// duration must not arrive there under two names.
pub(crate) fn parse_duration(node: &SyntaxNode) -> Option<NotatedDuration> {
    let text = musa_language::ast::Duration::of(node)?.value()?;
    let value = if text.contains('/') {
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
                .note("a pitch is a letter, an optional `#` or `b`, and an octave: `c4`, `g#5`, `bb3`"),
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
    if let Some(text) = musa_language::ast::Duration::of(node).and_then(|duration| duration.parameter())
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
                        .note("a pitch is a letter, an optional `#` or `b`, and an octave: `c4`, `g#5`, `bb3`"),
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

/// A groove needs a meter to swing against.
///
/// A groove displaces the beat inside a cell named by the meter's denominator
/// (`groove.rs`), and unmeasured music has no denominator to name one — so a
/// swung cadenza is not a thing with a wrong answer, it is a question with
/// none. Refused rather than straightened silently, which would be the
/// composer's feel dropped without a word.
pub(crate) fn check_groove_has_a_meter(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    let rules = std::mem::take(&mut resolver.groove_rules);
    if rules.is_empty() {
        return;
    }
    for (id, part) in snapshot.parts().iter() {
        let bars = snapshot.bars(crate::Scope::Part { part: id.0 });
        let Some(profile) = snapshot.profiles().for_part(part.name()) else {
            continue;
        };
        let Some((_, span)) = rules.iter().find(|(name, _)| name == profile.name()) else {
            continue;
        };
        let unmeasured = part
            .voices()
            .flat_map(|(_, voice)| voice.events())
            .any(|event| !bars.meter_at(event.onset).is_measured());
        if !unmeasured {
            continue;
        }
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this groove has no beat to lay itself over")
                .at(*span, "the part playing this reaches unmeasured music")
                .help("end the unmeasured stretch before this part plays, or give the part a straight profile")
                .note("a groove displaces the beat inside a cell the meter names, and `meter none` names none"),
        );
    }
}

pub(crate) fn check_measure_sanity(resolver: &mut Resolver, snapshot: &ScoreSnapshot) {
    for (id, part) in snapshot.parts().iter() {
        // Whose barlines: a part in 7/8 stops short of *its* barline, and
        // measuring it against the piece's 4/4 would complain about music
        // that is right (polymeter, prompt 75).
        let bars = snapshot.bars(crate::Scope::Part { part: id.0 });
        for (voice_id, voice) in part.voices() {
            // A voice that holds a note as long as it likes is not measured
            // after that note: how far it reaches is the performance's
            // answer, so a barline arithmetic complaint about it would be a
            // complaint about the freedom rather than about a mistake.
            if voice.events().iter().any(|event| event.free.is_some()) {
                continue;
            }
            let span = voice.span();
            let end = crate::MusicalTime::ZERO + span;
            // Unmeasured music stops where it stops. "Part-way through a
            // measure" is a complaint about barlines, and there are none.
            if !bars.meter_at(end).is_measured() {
                continue;
            }
            let stops = bars.at(end);
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
