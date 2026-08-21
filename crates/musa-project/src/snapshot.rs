//! The frontend's entire view of the session.

use crate::command::{DocumentId, Revision};
use crate::diagnostic::Diagnostic;
use crate::facts::ScoreFacts;

/// Everything a caller may observe, borrowed from the session.
///
/// Borrowed rather than owned because the engraved MEI of a large score is
/// substantial and a GUI takes a snapshot after every command; copying it
/// per observation would be waste with no purpose. Nothing here is a
/// compiler, renderer, or engine type — the frontend owns no semantics
/// (roadmap §14.2, §15.7).
#[derive(Clone, Copy, Debug)]
pub struct ProjectSnapshot<'session> {
    pub(crate) document: DocumentId,
    pub(crate) source: &'session str,
    pub(crate) name: &'session str,
    pub(crate) revision: Revision,
    pub(crate) diagnostics: &'session [Diagnostic],
    /// The closed world the last compilation was handed, for
    /// [`Self::cause_source`] and nothing else.
    pub(crate) imports: &'session musa_compiler::ImportSources,
    pub(crate) valid: &'session Option<ValidArtifacts>,
    pub(crate) compiles: bool,
    pub(crate) unsaved: bool,
    pub(crate) autosaved: bool,
    pub(crate) recovery: Option<&'session str>,
    pub(crate) midi_port: Option<&'session str>,
    pub(crate) playback: PlaybackState,
    pub(crate) kind: musa_compiler::DocumentKind,
    /// The project this piece is one of, when a [`Project`](crate::Project)
    /// took the snapshot. A session on its own knows nothing about the volume
    /// it is filed in, and says so.
    pub(crate) contents: Option<&'session crate::contents::ContentsFacts>,
}

/// The artifacts of the most recent *successful* compilation. They survive
/// an invalid edit untouched, which is what lets the score stay on screen
/// and playback keep running while the text is mid-thought (roadmap §14.7).
#[derive(Debug)]
pub(crate) struct ValidArtifacts {
    /// The engraved score, rendered once per successful compile rather than
    /// once per observation.
    pub(crate) mei: String,
    /// What MEI could not say about this score, once per kind — computed with
    /// the render and kept with it, because an export asked for later must
    /// report the same losses as the one that produced this text.
    pub(crate) mei_warnings: Vec<String>,
    /// The compiled score, kept for exports and playback preparation.
    pub(crate) score: musa_score::ScoreSnapshot,
    /// The source that produced it.
    ///
    /// Kept because one export — events text — is a projection of the
    /// *document*, not of the score snapshot: a term carries provenance the
    /// snapshot has already spent. One string per successful compile, beside
    /// a history that already holds one per edit.
    pub(crate) source: String,
    /// The compiled studio from the same compilation. Kept beside the score
    /// rather than inside it: they are two documents, and pairing them here
    /// is what stops a render from using one piece's sound with another's
    /// notes (§6.5).
    pub(crate) studio: musa_compiler::StudioSpec,
    /// Everything the interface displays about that score.
    pub(crate) facts: ScoreFacts,
    /// Everything the Sound and Mix workspaces display about that studio.
    pub(crate) studio_facts: crate::studio::StudioFacts,
    /// Every decision this compile took, for the pin command and for the
    /// Origin view's fourth step. Empty for a determinate piece,
    /// which is nearly every piece.
    pub(crate) decisions: Vec<musa_score::DecisionRecord>,
    /// Every name the resolver resolved, for an editor's references and
    /// rename.
    pub(crate) names: Vec<crate::facts::NameFact>,
    /// What each declaration says about itself, for an editor's hover,
    /// completion, signature help, and outline.
    pub(crate) items: Vec<crate::facts::ItemFact>,
    pub(crate) revision: Revision,
    /// What this score *means* (docs/rules/events/05 N6), so a consumer can ask
    /// whether an edit changed the music rather than only the text.
    pub(crate) identity: musa_compiler::SemanticHash,
}

impl<'session> ProjectSnapshot<'session> {
    /// The current source text — the canonical document (roadmap §11).
    pub fn source(&self) -> &str {
        self.source
    }

    /// The document's display name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Which of the two things this document is (roadmap §16).
    ///
    /// Material has no score and never will; a piece that has not compiled
    /// yet has none either. Those are different screens, which is why this is
    /// a separate question from [`Self::score`] being `None`.
    pub fn kind(&self) -> musa_compiler::DocumentKind {
        self.kind
    }

    /// The project this piece is one of, if the snapshot came from a
    /// [`Project`](crate::Project).
    pub fn contents(&self) -> Option<&crate::ContentsFacts> {
        self.contents
    }

    /// Which document this is a snapshot of.
    ///
    /// Read it before [`Self::revision`]: revisions count within a document,
    /// and two snapshots with different ids are not comparable at all.
    pub fn document(&self) -> DocumentId {
        self.document
    }

    /// The current revision, within [`Self::document`].
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Diagnostics for the *current* source, whether or not it compiles.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics
    }

    /// The text of the document a [`Cause`](crate::Cause) is about.
    ///
    /// A cause already says *where* in that document, in lines and columns, so
    /// this is only for a renderer that quotes the line as well — the terminal
    /// draws a caret under it. An interface that merely lists causes does not
    /// need it, and the desktop deliberately does not: the composer does not
    /// own that file (`docs/rules/desktop/05-states.md` §5).
    ///
    /// `None` for a key this compilation was not handed, which is also when
    /// the cause's own positions are absent.
    pub fn cause_source(&self, document: &str) -> Option<&str> {
        self.imports.get(document)
    }

    /// Whether the current source compiles.
    ///
    /// When false, [`Self::mei`] and playback describe
    /// [`Self::score_revision`], not [`Self::revision`] — the interface must
    /// say so rather than blanking the score (roadmap §14.7,
    /// `docs/rules/desktop/05-states.md` §4).
    pub fn compiles(&self) -> bool {
        self.compiles
    }

    /// The engraved score as MEI, from the last revision that compiled.
    /// `None` only before the piece has ever compiled.
    pub fn mei(&self) -> Option<&str> {
        self.valid.as_ref().map(|valid| valid.mei.as_str())
    }

    /// The musical facts the interface displays: title, tempo, key, parts,
    /// and every event's pitch, position, and provenance. From the same
    /// revision as [`Self::mei`].
    pub fn score(&self) -> Option<&ScoreFacts> {
        self.valid.as_ref().map(|valid| &valid.facts)
    }

    /// The studio facts the Sound and Mix workspaces display: patches and
    /// their stages, buses, sends, and which patch realizes each part. From
    /// the same revision as [`Self::mei`].
    pub fn studio(&self) -> Option<&crate::studio::StudioFacts> {
        self.valid.as_ref().map(|valid| &valid.studio_facts)
    }

    /// Every name the resolver resolved, with its declaration and use spans
    /// — what an editor's references and rename are built from.
    /// From the same revision as [`Self::mei`]: while the source is
    /// mid-edit, these describe the last text that compiled, which
    /// [`Self::score_revision`] says.
    pub fn names(&self) -> &'session [crate::facts::NameFact] {
        self.valid.as_ref().map_or(&[], |valid| valid.names.as_slice())
    }

    /// What each declaration of the last valid compile says about itself:
    /// kind, signature, summary, parameters, and where it is written.
    ///
    /// Declarations, not names — an imported declaration has a record whether
    /// or not this document happens to speak its name, and a name used but
    /// never declared has a [`NameFact`](crate::NameFact) and no record. Find
    /// one by the `(name, kind)` a reference carries. From the same revision
    /// as [`Self::mei`].
    pub fn items(&self) -> &'session [crate::facts::ItemFact] {
        self.valid.as_ref().map_or(&[], |valid| valid.items.as_slice())
    }

    /// The revision the engraved score and playback plan came from.
    pub fn score_revision(&self) -> Option<Revision> {
        self.valid.as_ref().map(|valid| valid.revision)
    }

    /// Whether the source differs from what is on disk.
    pub fn unsaved(&self) -> bool {
        self.unsaved
    }

    /// Whether a recovery copy of the unsaved text is on disk. Together with
    /// [`Self::unsaved`] this is the whole save state: saved, unsaved and
    /// caught, or unsaved and not caught (a piece with no file yet).
    pub fn autosaved(&self) -> bool {
        self.autosaved
    }

    /// Work a previous session left behind and this one has not resolved.
    ///
    /// Present only when the recovery copy differs from the file, so it is
    /// always a real choice: keep what was on disk, or take back what was
    /// being typed.
    pub fn recovery(&self) -> Option<&str> {
        self.recovery
    }

    /// The MIDI keyboard being listened to, if one is connected.
    pub fn midi_port(&self) -> Option<&str> {
        self.midi_port
    }

    /// What the transport is doing.
    pub fn playback(&self) -> PlaybackState {
        self.playback
    }
}

/// The snapshot as the interface receives it — one flat object, with the
/// last-valid artifacts lifted out of their internal container.
///
/// Serialization lives here rather than in the frontend because the shape of
/// this object is part of the facade: a fixture generated from this type
/// cannot drift from it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotWire<'a> {
    /// Which piece this is. A frontend compares it before `revision`, which
    /// counts within it and not across pieces.
    document: u64,
    name: &'a str,
    /// `piece` or `material` — which of the two things this file is.
    kind: &'static str,
    source: &'a str,
    revision: u64,
    compiles: bool,
    unsaved: bool,
    autosaved: bool,
    recovery: Option<&'a str>,
    midi_port: Option<&'a str>,
    diagnostics: &'a [Diagnostic],
    mei: Option<&'a str>,
    score: Option<&'a ScoreFacts>,
    studio: Option<&'a crate::studio::StudioFacts>,
    score_revision: Option<u64>,
    playback: PlaybackState,
    /// Absent for a session opened on its own; a project always sends it.
    contents: Option<&'a crate::contents::ContentsFacts>,
    /// Every declaration in scope, as `08-elaboration.md` §1 shows one.
    terms: Vec<TermWire<'a>>,
    /// Every resolved name, for definition and references.
    names: Vec<NameWire<'a>>,
}

/// One declaration, as the term panel and the completion list read it.
///
/// A deliberate projection of [`ItemFact`](crate::ItemFact) rather than the
/// fact itself, and the reason is the wire's own invariant: **an offset that
/// crosses the wire indexes the document that crossed with it.** Spans are
/// restated in UTF-16 code units by [`crate::utf16::translate_spans`], using
/// the open source's index, so a byte range into `std::pitch` sent in that
/// shape would be silently translated against the wrong text. A declaration
/// somewhere else therefore travels as a [`SiteWire::Library`] — the module's
/// URI and an opaque range the frontend only ever hands back — and the
/// frontend cannot mistake it for a place in the text it is showing.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TermWire<'a> {
    name: &'a str,
    kind: crate::facts::NameKind,
    /// `fn triad(root: NoteName) -> ChordClass`, as the source spells it.
    signature: &'a str,
    /// The comment block above the declaration, as one paragraph.
    summary: Option<&'a str>,
    /// What it evaluates to.
    result: Option<&'a crate::facts::TypeFact>,
    parameters: &'a [crate::facts::ParameterFact],
    /// What to write instead, when the declaration says it is deprecated.
    deprecation: Option<&'a str>,
    /// Whether an editor may write to it.
    read_only: bool,
    /// Where it is declared.
    site: SiteWire<'a>,
}

/// Where a declaration is written, as the wire may state it.
#[derive(serde::Serialize)]
#[serde(tag = "where", rename_all = "camelCase")]
enum SiteWire<'a> {
    /// In the document this snapshot carries. `span` indexes its `source`.
    Open {
        /// The declaration's own name token.
        span: crate::diagnostic::Span,
    },
    /// In another document — a bundled library module.
    ///
    /// `start` and `end` are byte offsets *in that module*, not code-unit
    /// offsets in anything the frontend is holding. They are a handle: the
    /// frontend hands them back when it asks to open the module, and the
    /// backend states them in that document's own measure then.
    Library {
        /// The module's readable virtual URI.
        uri: &'a str,
        start: u32,
        end: u32,
    },
}

/// One resolved name, for definition and references.
///
/// Same projection, same reason: a declaration in another document is named
/// by its URI, and the wire carries no offset into a text it did not send.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NameWire<'a> {
    name: &'a str,
    kind: crate::facts::NameKind,
    /// The declaration's name token, when it is in this document.
    declaration: Option<crate::diagnostic::Span>,
    /// The module it is declared in, when it is not.
    external: Option<&'a str>,
    /// Every resolved use's name token, in the order the resolver met them.
    uses: &'a [crate::diagnostic::Span],
}

impl ProjectSnapshot<'_> {
    /// The snapshot as a frontend receives it, with every span restated in
    /// UTF-16 code units.
    ///
    /// This is the *only* way to serialize a snapshot: `ProjectSnapshot` does
    /// not implement `Serialize`, so no caller can produce a wire snapshot
    /// that still measures the source in bytes. See [`crate::utf16`] for why
    /// the two measures differ and where the line between them is drawn.
    #[must_use]
    pub fn to_wire(&self) -> serde_json::Value {
        let offsets = crate::Utf16Offsets::new(self.source);
        let mut wire = serde_json::to_value(SnapshotWire {
            document: self.document.0,
            name: self.name,
            kind: match self.kind {
                musa_compiler::DocumentKind::Piece => "piece",
                musa_compiler::DocumentKind::Material => "material",
                musa_compiler::DocumentKind::Events => "events",
            },
            source: self.source,
            revision: self.revision.0,
            compiles: self.compiles,
            unsaved: self.unsaved,
            autosaved: self.autosaved,
            recovery: self.recovery,
            midi_port: self.midi_port,
            diagnostics: self.diagnostics,
            mei: self.mei(),
            score: self.score(),
            studio: self.studio(),
            score_revision: self.score_revision().map(|revision| revision.0),
            playback: self.playback,
            contents: self.contents,
            terms: self.items().iter().map(term).collect(),
            names: self.names().iter().map(name).collect(),
        })
        // `SnapshotWire` is a plain struct of strings, numbers, and derived
        // types; `to_value` fails only on things it cannot contain, such as a
        // map with non-string keys.
        .unwrap_or(serde_json::Value::Null);
        crate::utf16::translate_spans(&mut wire, &offsets);
        wire
    }
}

/// One declaration, projected onto the wire.
fn term(item: &crate::facts::ItemFact) -> TermWire<'_> {
    TermWire {
        name: &item.name,
        kind: item.kind,
        signature: &item.signature,
        summary: item.summary.as_deref(),
        result: item.result.as_ref(),
        parameters: &item.parameters,
        deprecation: item.deprecation.as_deref(),
        read_only: item.read_only,
        site: match item.uri {
            None => SiteWire::Open { span: item.span },
            Some(ref uri) => SiteWire::Library {
                uri,
                start: item.span.start,
                end: item.span.end,
            },
        },
    }
}

/// One resolved name, projected onto the wire.
fn name(reference: &crate::facts::NameFact) -> NameWire<'_> {
    NameWire {
        name: &reference.name,
        kind: reference.kind,
        declaration: reference.declaration,
        external: reference
            .external_declaration
            .as_ref()
            .map(|location| location.uri.as_str()),
        uses: &reference.uses,
    }
}

/// Transport state as the interface needs to display it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    /// Whether audio is running.
    pub playing: bool,
    /// The transport position in frames.
    pub position_frames: u64,
    /// The installed plan's total length in frames, including its tail.
    pub total_frames: u64,
    /// The sample rate the plan and stream run at.
    pub sample_rate: u32,
    /// The active loop region, if any.
    pub loop_region: Option<(u64, u64)>,
}
