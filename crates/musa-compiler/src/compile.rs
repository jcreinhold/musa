//! The compiler facade (roadmap §10.6, §15.3): one deep operation. Passes
//! (resolution, units, resolver) are private; callers see `Compilation`.

use musa_score::diagnose::{Diagnostic, Severity};
use musa_score::score::ScoreSnapshot;
use std::sync::Arc;

/// A source document to compile.
pub struct SourceDocument {
    name: String,
    text: String,
}

impl SourceDocument {
    /// Create a document from text and a display name (used in diagnostics).
    pub fn new(text: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }

    /// Load a document from a `.musa` file.
    ///
    /// # Errors
    /// Returns the I/O error when the file cannot be read as UTF-8.
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)?;
        Ok(Self::new(text, path.to_string_lossy()))
    }

    /// The source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The document name used in diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Options controlling compilation. Currently empty: defaults live inside.
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    /// The text of every file this compilation may `use` (roadmap §16),
    /// keyed by the path an importer resolves to. The compiler reads no
    /// files: whoever owns the filesystem fills this in.
    pub imports: crate::imports::ImportSources,
    /// Which performance to compile.
    ///
    /// A piece that leaves nothing open never consults this, so the default —
    /// `Realization::deterministic()` — is the absence of a question rather
    /// than a choice of answer (`docs/rules/events/11-realization.md`).
    pub realization: musa_score::realize::Realization,
}

/// Which of the shapes a musa file may be (roadmap §16).
///
/// The grammar has always had the first two — a `piece` sounds, a `library`
/// declares — but the compiler used to accept only the first at a document's
/// root, which made "material has no score" indistinguishable from "this
/// failed to compile". Callers ask this when they need to tell those apart;
/// callers that only want a score still ask for one and still get `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocumentKind {
    /// `piece "…" { … }` — the thing that has a score.
    #[default]
    Piece,
    /// Declarations and no `piece` — a file for other files to import.
    Material,
    /// `mod …;` and nothing else — a package's module tree, at its root
    /// (`lib.musa`) or one directory down (`mod.musa`).
    ///
    /// Neither of the two above: it has no score, and it exports nothing for
    /// an importer either, because a module file is a path segment rather
    /// than a module of its own (`docs/rules/language/04-templates-and-modules.md`).
    /// A caller deciding what to show needs that told apart from material,
    /// which has nothing to show and plenty to import.
    Modules,
    /// `% musa-events-3` — an events interchange file, read as itself
    /// (`docs/rules/language/01-surface.md` §7).
    ///
    /// A third kind rather than a second flavour of `Piece`, because the two
    /// differ in what a caller may *do*: an events document has no surface
    /// syntax tree, so nothing that edits structure, completes a name, or
    /// renames a motif applies to it. It has a score, which is why it is not
    /// `Material` either.
    Events,
}

/// The result of compiling a document: diagnostics always, a snapshot and a
/// studio when no error-severity diagnostic was produced.
pub struct Compilation {
    kind: DocumentKind,
    snapshot: Option<ScoreSnapshot>,
    studio: crate::studio_model::SurfaceStudio,
    studio_source: Option<musa_calculus::CheckedSource>,
    studio_spans: StudioSpans,
    machines: Vec<(String, musa_score::MachineSpec)>,
    diagnostics: Vec<Diagnostic>,
    identity: musa_events::SemanticHash,
    decisions: Vec<musa_score::DecisionRecord>,
    references: crate::resolve::ReferenceIndex,
    derivation: Option<musa_score::derivation::Derivation>,
    barline_items: Vec<BarlineSourceItem>,
}

/// How one direct checked voice item participates in pipe-bar syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarlineSourceRole {
    /// An item a leading `|` may enclose, such as a note, rest, chord, `use`,
    /// tuplet, mark, or point context statement.
    Loose,
    /// A `bar` or `|` assertion already written by the author.
    ExistingBar,
    /// A direct statement that ends pipe-bar syntax and is not itself a
    /// candidate for insertion.
    Boundary,
}

/// How adapter restoration classified the item's boundary and contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BarlineLineage {
    Authored,
    GeneratedInsertion,
    GeneratedContent,
}

/// One compiler-proved direct source-item extent for semantic source edits.
///
/// This is intentionally not a public pass or a second score model. It is the
/// immutable join key between the lossless source and the exact duration and
/// scoped-barline answers the successful compilation already computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarlineSourceItem {
    pub(crate) span: musa_score::origin::SourceSpan,
    pub(crate) scope: musa_score::Scope,
    pub(crate) start: musa_score::MusicalTime,
    pub(crate) end: musa_score::MusicalTime,
    pub(crate) measured: bool,
    pub(crate) start_on_barline: bool,
    pub(crate) end_on_barline: bool,
    pub(crate) internal_boundary: Option<musa_score::MusicalTime>,
    pub(crate) role: BarlineSourceRole,
    pub(crate) lineage: BarlineLineage,
    /// Definition spans of the occurrences the item produced, retained until
    /// adapter restoration can distinguish authored definitions from text the
    /// compiler generated.
    pub(crate) definitions: Vec<musa_score::origin::SourceSpan>,
}

impl BarlineSourceItem {
    /// The exact source range of the direct statement.
    pub fn span(&self) -> musa_score::origin::SourceSpan {
        self.span
    }

    /// The part/voice whose barlines govern the item.
    pub fn scope(&self) -> musa_score::Scope {
        self.scope
    }

    /// Its exact written-time onset.
    pub fn start(&self) -> musa_score::MusicalTime {
        self.start
    }

    /// Its exact exclusive written-time end.
    pub fn end(&self) -> musa_score::MusicalTime {
        self.end
    }

    /// Whether a measured meter is in force at the item.
    pub fn is_measured(&self) -> bool {
        self.measured
    }

    /// Whether the onset is a barline in the item's scope.
    pub fn starts_on_barline(&self) -> bool {
        self.start_on_barline
    }

    /// Whether the exclusive end is a barline in the item's scope.
    pub fn ends_on_barline(&self) -> bool {
        self.end_on_barline
    }

    /// The first barline strictly inside the item, if one exists.
    pub fn internal_boundary(&self) -> Option<musa_score::MusicalTime> {
        self.internal_boundary
    }

    /// Its pipe-bar syntax role.
    pub fn role(&self) -> BarlineSourceRole {
        self.role
    }

    /// Whether this insertion boundary belongs to authored source rather than
    /// text produced by an adapter expansion.
    pub fn is_authored(&self) -> bool {
        self.lineage != BarlineLineage::GeneratedInsertion
    }

    /// Whether evaluating the item reached material produced by an adapter.
    pub fn contains_generated_content(&self) -> bool {
        self.lineage == BarlineLineage::GeneratedContent
    }
}

impl Compilation {
    pub(crate) fn new(snapshot: Option<ScoreSnapshot>, diagnostics: Vec<Diagnostic>) -> Self {
        // Read off the snapshot here rather than threaded through
        // elaboration, so that a score and the record of where it came from
        // cannot be built from two different pictures of the same piece.
        let derivation = snapshot
            .as_ref()
            .map(|snapshot| musa_score::derivation::of_score(snapshot, &[]));
        Self {
            kind: DocumentKind::Piece,
            snapshot,
            studio: crate::studio_model::SurfaceStudio::default(),
            studio_source: None,
            studio_spans: StudioSpans::default(),
            machines: Vec::new(),
            diagnostics,
            identity: musa_events::SemanticHash::default(),
            decisions: Vec::new(),
            references: crate::resolve::ReferenceIndex::new(),
            derivation,
            barline_items: Vec::new(),
        }
    }

    /// Say everything this compilation learned about the text the *composer*
    /// wrote, rather than about the text the compiler read.
    ///
    /// The one seam adapter expansion creates, closed in one place. A document
    /// with no region has an identity map and nothing here runs; a document
    /// with one has every span translated and the derivation rebuilt, so that a
    /// note an adapter produced is a `Generated` step anchored at its region
    /// rather than a note apparently written in text that does not exist.
    pub(crate) fn restore(&mut self, expansion: &crate::expand::Expansion) {
        self.diagnostics.extend(expansion.diagnostics.iter().cloned());
        if expansion.map.is_identity() {
            return;
        }
        for diagnostic in &mut self.diagnostics {
            diagnostic.remap_spans(&expansion.map);
        }
        for item in &mut self.barline_items {
            let generated_insertion = expansion
                .map
                .replacements
                .iter()
                .any(|replacement| item.span.start >= replacement.from && item.span.end <= replacement.to);
            item.span = expansion.map.span(item.span);
            let generated_content = item.definitions.iter().any(|span| {
                expansion
                    .map
                    .replacements
                    .iter()
                    .any(|replacement| span.start >= replacement.from && span.end <= replacement.to)
            });
            item.lineage = if generated_insertion {
                BarlineLineage::GeneratedInsertion
            } else if generated_content {
                BarlineLineage::GeneratedContent
            } else {
                BarlineLineage::Authored
            };
            for span in &mut item.definitions {
                *span = expansion.map.span(*span);
            }
        }
        if let Some(snapshot) = self.snapshot.as_mut() {
            snapshot.remap_spans(&expansion.map);
        }
        self.studio.remap_spans(&expansion.map);
        self.studio_spans.remap(&expansion.map);
        for decision in &mut self.decisions {
            decision.remap_spans(&expansion.map);
        }
        self.references.remap_spans(&expansion.map);
        let anchors = expansion.anchors();
        self.derivation = self
            .snapshot
            .as_ref()
            .map(|snapshot| musa_score::derivation::of_score(snapshot, &anchors));
    }

    pub(crate) fn into_material(mut self) -> Self {
        self.kind = DocumentKind::Material;
        self
    }

    pub(crate) fn into_modules(mut self) -> Self {
        self.kind = DocumentKind::Modules;
        self
    }

    pub(crate) fn into_events(mut self) -> Self {
        self.kind = DocumentKind::Events;
        self
    }

    /// Which shape this document is (roadmap §16).
    ///
    /// Material and a module file compile to no score and that is not a
    /// failure: a caller deciding what to *show* needs the distinction,
    /// because "no score yet" and "no score ever" are different screens.
    pub fn kind(&self) -> DocumentKind {
        self.kind
    }

    pub(crate) fn with_studio(mut self, studio: crate::studio_model::SurfaceStudio) -> Self {
        self.studio = studio;
        self
    }

    pub(crate) fn with_studio_source(
        mut self,
        source: musa_calculus::CheckedSource,
        spans: Vec<Option<musa_score::origin::SourceSpan>>,
    ) -> Self {
        self.studio_source = Some(source);
        self.studio_spans = StudioSpans(spans.into());
        self
    }

    pub(crate) fn with_machines(mut self, machines: Vec<(String, musa_score::MachineSpec)>) -> Self {
        self.machines = machines;
        self
    }

    pub(crate) fn with_identity(mut self, identity: musa_events::SemanticHash) -> Self {
        self.identity = identity;
        self
    }

    pub(crate) fn with_decisions(mut self, decisions: Vec<musa_score::DecisionRecord>) -> Self {
        self.decisions = decisions;
        self
    }

    pub(crate) fn with_references(mut self, references: crate::resolve::ReferenceIndex) -> Self {
        self.references = references;
        self
    }

    pub(crate) fn with_barline_items(mut self, items: Vec<BarlineSourceItem>) -> Self {
        self.barline_items = items;
        self
    }

    /// Direct checked source items against their governing scoped barlines.
    ///
    /// Empty for non-piece documents and failed compilations.
    pub fn barline_items(&self) -> &[BarlineSourceItem] {
        &self.barline_items
    }

    /// Every name the resolver resolved, and everywhere it is spoken.
    ///
    /// This is the resolver's own knowledge kept, not a re-scan of the text:
    /// a name that did not resolve has no entry, and a name declared in an
    /// imported library has no declaration span here — its uses are honest
    /// and its home is `None`.
    pub fn references(&self) -> &[crate::resolve::NameReference] {
        self.references.entries()
    }

    /// What each checked declaration says about itself, for an editor to
    /// restate (`crate::docs`).
    ///
    /// Declarations, not names: a name used but never declared has a
    /// reference and no record, and an imported declaration has a record
    /// whether or not this document happens to speak its name. Unique by name
    /// and kind, so a consumer may look one up by the name a reference
    /// carries.
    pub fn items(&self) -> &[crate::docs::ItemDoc] {
        self.references.items()
    }

    /// Every decision this compilation took, in the order the sites were
    /// reached (`docs/rules/events/11-realization.md`).
    ///
    /// The realization holds only what a composer *pinned*; this is what the
    /// piece actually asked and what it was answered, which is what a header
    /// line records and what the Origin view shows. A determinate piece
    /// returns nothing, under every seed.
    pub fn decisions(&self) -> &[musa_score::DecisionRecord] {
        &self.decisions
    }

    /// What this compilation *means*, as a digest of the piece's track
    /// (docs/rules/events/05 N6).
    ///
    /// Two compilations with the same identity are the same music, whatever
    /// their sources looked like; two with different identities differ in
    /// something a listener or an engraver would see. Provenance is part of
    /// the track's payloads, so moving a note's text without changing the
    /// note changes the identity — the question it answers is "is this the
    /// same compiled piece", not "does it sound the same".
    ///
    /// A compilation that produced no score has the identity of the empty
    /// piece, which is what a caller keying on it wants: nothing to install.
    pub fn identity(&self) -> musa_events::SemanticHash {
        self.identity
    }

    /// The exact checked source value denoted by the compatibility studio
    /// spelling. This generic artifact is the production semantic handoff;
    /// consumers decode their own read-only projection from it.
    pub fn studio_source(&self) -> Option<&musa_calculus::CheckedSource> {
        self.studio_source.as_ref()
    }

    /// Resolve one checked studio anchor into this compilation's source.
    /// Forged or source-generated ordinals deliberately name no range.
    #[must_use]
    pub fn studio_span(&self, anchor: u64) -> Option<musa_score::origin::SourceSpan> {
        self.studio_spans.resolve(anchor)
    }

    /// The immutable lineage table paired with [`Self::studio_source`].
    #[must_use]
    pub fn studio_spans(&self) -> &StudioSpans {
        &self.studio_spans
    }

    /// The machine a name denotes, if this document names one
    /// (`docs/rules/across-stages/03-machine-calculus.md` §2).
    ///
    /// A [`musa_score::MachineSpec`] is immutable, flat, and exact, and it is the
    /// only form a machine leaves the compiler in. Its audio consumer prepares
    /// one into something that can be stepped.
    pub fn machine(&self, name: &str) -> Option<&musa_score::MachineSpec> {
        self.machines
            .iter()
            .find(|(named, _)| named == name)
            .map(|(_, machine)| machine)
    }

    /// Where this compilation's score came from
    /// (`docs/rules/across-stages/02-derivation-diagrams.md`).
    ///
    /// A finite acyclic graph, not a list of pairs: a note instantiated from
    /// a shared body reaches both the body and the site that instantiated it,
    /// and a passage assembled from several inputs keeps all of them. `None`
    /// for a document that compiled to no score, which is the same answer
    /// [`Self::snapshot`] gives and for the same reason.
    ///
    /// Its named consumer is `musa-project`, which fills the Origin row's
    /// list of supporting places from it.
    pub fn derivation(&self) -> Option<&musa_score::derivation::Derivation> {
        self.derivation.as_ref()
    }

    /// Every machine this document names, in the order it declares them.
    pub fn machine_names(&self) -> Vec<&str> {
        self.machines.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// The expanded score, if compilation succeeded.
    pub fn snapshot(&self) -> Option<&ScoreSnapshot> {
        self.snapshot.as_ref()
    }

    /// The score snapshot, consuming the compilation.
    pub fn into_snapshot(self) -> Option<ScoreSnapshot> {
        self.snapshot
    }

    /// All diagnostics (parse and semantic), in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Whether any error-severity diagnostic was produced.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

/// Compiler-owned source lineage for ordinals embedded in a checked studio.
///
/// The table is immutable and has no public constructor: source declarations
/// carry only stable ordinals, while the compiler remains the sole authority
/// for mapping those ordinals back into the lossless input document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StudioSpans(Arc<[Option<musa_score::origin::SourceSpan>]>);

impl StudioSpans {
    fn remap(&mut self, map: &musa_score::origin::SourceMap) {
        self.0 = self.0.iter().map(|span| map.maybe(*span)).collect();
    }

    /// Resolve one checked ordinal. Forged and source-generated ordinals name
    /// no range.
    #[must_use]
    pub fn resolve(&self, anchor: u64) -> Option<musa_score::origin::SourceSpan> {
        self.0.get(usize::try_from(anchor).ok()?).copied().flatten()
    }
}

/// Compile `source` into a [`Compilation`] (roadmap §10.6, §15.3; course
/// correction §26).
///
/// Pipeline: parse → expansion-aware elaboration (motifs, repeat, transpose)
/// → event-track → `ScoreSnapshot` adapter. There is one semantic path.
///
/// A document written in the *events* alternative
/// (`docs/rules/language/01-surface.md` §7) joins that path later rather than
/// running beside it: it has no surface syntax to elaborate, so reading and
/// checking the term replaces everything up to the event track, and the projection
/// and every backend after it are shared. Which alternative a text is, is a
/// question about its first line and is asked by `musa-syntax`.
pub fn compile(source: &SourceDocument, options: &CompileOptions) -> Compilation {
    // Create the span on the caller's thread, where its subscriber lives, and
    // enter the same span on the room thread so every pipeline event remains
    // its child. The fields are the caller's own arguments: nothing is
    // computed to fill them.
    let span = tracing::info_span!("compile", document = source.name(), bytes = source.text().len());
    musa_calculus::with_stack_room(|| {
        let _entered = span.enter();
        compile_in_room(source, options)
    })
}

/// [`compile`] after its one stack-room boundary has been arranged.
fn compile_in_room(source: &SourceDocument, options: &CompileOptions) -> Compilation {
    let alternative = musa_syntax::alternative(source.text());
    let compilation = match alternative {
        musa_syntax::DocumentAlternative::Events => crate::events_text::compile_events(source),
        musa_syntax::DocumentAlternative::Surface => {
            // Step 4 of the fixed order, and the only place it happens. What
            // `elaborate` then reads is a text with every adapter region
            // replaced by the expression its adapter answered with; what the
            // caller is told about is the text they wrote.
            let expansion = crate::expand::expand(source, options);
            let mut compilation = crate::elaborate::elaborate(&expansion.document, options);
            compilation.restore(&expansion);
            compilation
        }
    };
    // Which alternative a text was read as is decided from its first line and
    // is invisible afterwards, so a piece that is silently treated as events
    // interchange has no other way of saying so.
    tracing::debug!(
        alternative = ?alternative,
        diagnostics = compilation.diagnostics().len(),
        compiled = compilation.snapshot().is_some(),
        "compiled"
    );
    compilation
}

/// Lay `text` out canonically, whichever alternative it is written in.
///
/// One entry because there is one question — "how should this document be
/// written down" — and two answers only because there are two languages. A
/// caller that dispatched itself would be a caller that could get the
/// dispatch wrong, and the whole toolchain formats through here.
///
/// Events text is laid out by the event track's own printer, which is what makes
/// `musa format` idempotent on a file `musa events` produced. `None` means
/// the text cannot be read at all, and an unreadable document is left exactly
/// as its author has it.
pub fn format_document(text: &str, spacing: musa_syntax::BarSpacing) -> Option<String> {
    let span = tracing::debug_span!("format", bytes = text.len());
    let _entered = span.enter();
    match musa_syntax::alternative(text) {
        musa_syntax::DocumentAlternative::Events => crate::events_text::format_events(text),
        musa_syntax::DocumentAlternative::Surface => {
            let document = musa_syntax::parse(text);
            Some(musa_syntax::format(&document, spacing).text().to_owned())
        }
    }
}

#[cfg(test)]
mod barline_projection_laws {
    #![allow(clippy::expect_used)]

    use super::*;
    use musa_score::origin::{Replacement, SourceMap, SourceSpan};

    #[test]
    fn restoration_marks_generated_content_without_disowning_the_use_site() {
        let mut compilation = Compilation::new(None, Vec::new()).with_barline_items(vec![BarlineSourceItem {
            span: SourceSpan::new(80, 94),
            scope: musa_score::Scope::Voice { part: 0, voice: 0 },
            start: musa_score::MusicalTime::ZERO,
            end: musa_score::MusicalTime::new(num_rational::Ratio::from_integer(2)),
            measured: true,
            start_on_barline: true,
            end_on_barline: true,
            internal_boundary: Some(musa_score::MusicalTime::new(num_rational::Ratio::ONE)),
            role: BarlineSourceRole::Loose,
            lineage: BarlineLineage::Authored,
            definitions: vec![SourceSpan::new(20, 24)],
        }]);
        let expansion = crate::expand::Expansion {
            document: SourceDocument::new("", "law.musa"),
            map: SourceMap {
                replacements: vec![Replacement {
                    from: 10,
                    to: 40,
                    original: SourceSpan::new(5, 15),
                }],
            },
            records: Vec::new(),
            charges: crate::expand::Charges::default(),
            diagnostics: Vec::new(),
        };

        compilation.restore(&expansion);
        let item = compilation.barline_items().first().expect("one item");
        assert!(item.is_authored(), "the use statement remains authored");
        assert!(
            item.contains_generated_content(),
            "its evaluated definition came from the adapter"
        );
    }
}
