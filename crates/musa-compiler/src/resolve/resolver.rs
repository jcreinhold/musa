#![allow(clippy::arithmetic_side_effects)]
use indexmap::IndexSet;
use musa_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;
use musa_score::score::{AnnotationStore, EventId, Meter};

use super::ReferenceIndex;

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
    pub(crate) motifs: IndexSet<String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) next_event: u64,
    /// Which construction site the next `quote at …` is
    /// (`docs/rules/language/11-quotation.md` §3).
    ///
    /// The middle component of a [`crate::quote::Derived`] path, and the one
    /// number in it the elaborator has to allocate rather than derive: the
    /// origin comes from the anchor and the path from the quote's own tree,
    /// but "which quote wrote this" is a fact about the program and not about
    /// either. It lives here, beside [`Self::next_event`] and
    /// [`Self::next_part`], because a resolver is one per compile — a counter
    /// on the per-definition checker would restart, and two helpers in two
    /// modules quoting at the same anchor would mint the same identity for
    /// different nodes.
    pub(crate) next_quotation: u32,
    pub(crate) annotations: AnnotationStore,
    /// The prevailing meter, and whether the piece actually wrote it.
    ///
    /// Read-only once the header is lowered, and here rather than threaded
    /// through the expansion context because it is a fact about the piece
    /// rather than about the block being expanded. What a `bar` is checked
    /// against.
    pub(crate) meter: Meter,
    pub(crate) meter_written: bool,
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
    /// How many decision sites have been seen inside each named place, so the
    /// next one there knows its ordinal.
    pub(crate) sites: std::collections::BTreeMap<musa_score::ChoicePath, u32>,
    /// Every decision this compile took, in the order the sites were reached.
    pub(crate) decisions: Vec<musa_score::DecisionRecord>,
    /// Where each voice's kernel track goes on its way to the adapter.
    ///
    /// `None` on every production path — nothing keeps a track after the
    /// snapshot is built. It is `Some` only under `crate::bench`, which needs
    /// the elaboration and projection stages separable to measure them apart
    /// (roadmap §17.7). One `Option` check per voice is the whole cost.
    pub(crate) track_sink: Option<Vec<crate::elaborate::VoiceTrack>>,
    /// Every name reference resolved, kept for editors.
    pub(crate) references: ReferenceIndex,
    /// Which performance is being compiled (`docs/rules/kernel/11-realization.md`).
    ///
    /// It lives here rather than being threaded through elaboration because a
    /// decision site can be anywhere a note can be, and every function on the
    /// way already carries the resolver.
    pub(crate) realization: musa_score::Realization,
}

impl Resolver {
    pub(crate) fn new() -> Self {
        Self {
            motifs: IndexSet::new(),
            diagnostics: Vec::new(),
            next_event: 0,
            next_quotation: 0,
            annotations: AnnotationStore::default(),
            meter: Meter::default(),
            meter_written: false,
            part_meters: std::collections::BTreeMap::new(),
            groove_rules: Vec::new(),
            sites: std::collections::BTreeMap::new(),
            references: ReferenceIndex::new(),
            decisions: Vec::new(),
            track_sink: None,
            realization: musa_score::Realization::deterministic(),
        }
    }

    /// The path of the next decision site inside `place`, and the decision
    /// the realization makes there.
    ///
    /// The ordinal is per named place, so a site in one voice is unaffected by
    /// sites added in another — and inside a place, a site added *below*
    /// leaves the ones above it alone. Both are the point of
    /// `docs/rules/kernel/11-realization.md`'s path identity.
    pub(crate) fn decide_count(
        &mut self,
        place: &musa_score::ChoicePath,
        least: u32,
        most: u32,
        site: SourceSpan,
    ) -> (musa_score::ChoicePath, u32) {
        let path = self.site(place);
        let count = self.realization.count(&path, least, most);
        let passes = if count == 1 { "pass" } else { "passes" };
        self.decided(
            path.clone(),
            musa_score::Decision::Count(count),
            site,
            format!("{count} {passes}"),
        );
        (path, count)
    }

    /// The order a mobile's fragments are played in: a permutation of
    /// `0..fragments.len()`.
    pub(crate) fn decide_order(
        &mut self,
        place: &musa_score::ChoicePath,
        fragments: &[String],
        site: SourceSpan,
    ) -> Vec<u32> {
        let path = self.site(place);
        let count = u32::try_from(fragments.len()).unwrap_or(u32::MAX);
        let order = self.realization.order(&path, count);
        // By name, because `[2, 0, 1]` is not something to show anyone and
        // the names are only known here (`docs/rules/desktop/03-interaction.md` §7).
        let played: Vec<&str> = order
            .iter()
            .filter_map(|index| fragments.get(*index as usize).map(String::as_str))
            .collect();
        self.decided(
            path,
            musa_score::Decision::Order(order.clone()),
            site,
            played.join(", "),
        );
        order
    }

    /// How long a freely-held note actually sounds, between the written value
    /// and the longest it may be held.
    pub(crate) fn decide_duration(
        &mut self,
        place: &musa_score::ChoicePath,
        least: Ratio<i64>,
        most: Ratio<i64>,
        site: SourceSpan,
    ) -> Ratio<i64> {
        let path = self.site(place);
        let held = self.realization.duration(&path, least, most);
        let answered = format!("held {}/{}", held.numer(), held.denom());
        self.decided(path, musa_score::Decision::Duration(held), site, answered);
        held
    }

    /// The path of the next decision site inside `place`.
    fn site(&mut self, place: &musa_score::ChoicePath) -> musa_score::ChoicePath {
        let ordinal = self.sites.entry(place.clone()).or_insert(0);
        let path = place.then(musa_score::ChoiceStep::Ordinal(*ordinal));
        *ordinal = ordinal.saturating_add(1);
        path
    }

    /// Record what was decided at a site.
    ///
    /// One site, one decision, however many voices reach it: the k-th site in
    /// every voice *is* the k-th site, which is the point of numbering them
    /// per voice rather than per path down from the part.
    fn decided(
        &mut self,
        path: musa_score::ChoicePath,
        decision: musa_score::Decision,
        site: SourceSpan,
        answered: String,
    ) {
        if let Some(already) = self.decisions.iter_mut().find(|record| record.path == path) {
            if !already.sites.contains(&site) {
                already.sites.push(site);
            }
            return;
        }
        let pinned = self.realization.is_pinned(&path);
        self.decisions.push(musa_score::DecisionRecord {
            path,
            decision,
            sites: vec![site],
            answered,
            pinned,
        });
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
///
/// Walked inwards from both ends rather than across, because the answer is
/// only ever the first significant child and the last. Trivia clusters at the
/// edges — leading indent, a trailing newline — so both walks stop within a
/// step or two, where enumerating the children materialized a cursor per child
/// of every node whose span anything ever asked for.
pub(crate) fn trimmed_span(node: &SyntaxNode) -> SourceSpan {
    let mut forward = node.first_child_or_token();
    while forward.as_ref().is_some_and(|element| element.kind().is_trivia()) {
        forward = forward.and_then(|element| element.next_sibling_or_token());
    }
    let mut backward = node.last_child_or_token();
    while backward.as_ref().is_some_and(|element| element.kind().is_trivia()) {
        backward = backward.and_then(|element| element.prev_sibling_or_token());
    }
    match (forward, backward) {
        (Some(first), Some(last)) => SourceSpan::new(
            u32::from(first.text_range().start()),
            u32::from(last.text_range().end()),
        ),
        _ => span_of(node),
    }
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

/// The span of a token, in the record's measure.
pub(crate) fn source_span_of(token: &musa_syntax::SyntaxToken) -> SourceSpan {
    let range = token.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}
