//! Where a derived result came from
//! (`docs/rules/across-stages/02-derivation-diagrams.md`).
//!
//! A derivation is a **finite directed acyclic graph** whose leaves are
//! anchors of an input representation — not a list. §6 is the reason: one
//! result may combine several inputs, and one shared body may be used at
//! several sites, and a linear path can express neither. Composing two stages
//! grafts the earlier graph at each leaf of the later one; it does not
//! concatenate, and there is no helper here that flattens the graph into
//! `(source, target)` pairs, because that flattening loses exactly the two
//! things the graph exists to record.
//!
//! Acyclicity is by construction rather than by check: a node may only name
//! nodes already in the graph, so an id that could close a cycle does not
//! exist yet when the step that would close it is built. A dangling anchor is
//! refused the same way — [`Derivation::preserved`] and its siblings answer
//! `None` for an id the graph does not hold.
//!
//! Anchor ids are local to one [`PresentationRef`] and to one build. Nothing
//! here promises an identity across builds.

use crate::origin::SourceSpan;

/// Which stored representation an anchor belongs to (§1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PresentationKind {
    /// The `.musa` document the composer wrote.
    Source,
    /// The elaborated score track.
    ScoreTrack,
}

impl PresentationKind {
    /// The byte this kind is written as, for the canonical encoding.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    const fn byte(self) -> u8 {
        match self {
            Self::Source => b's',
            Self::ScoreTrack => b't',
        }
    }
}

/// A stored representation, at the version this build read (§1).
///
/// The version is part of the reference because §1 makes it part of the
/// reference: loading two records with one `PresentationRef` and different
/// descriptors is an error, and that can only be an error if the version is
/// carried rather than assumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PresentationRef {
    kind: PresentationKind,
    version: u32,
}

impl PresentationRef {
    /// The representation of `kind` at `version`.
    pub(crate) const fn at(kind: PresentationKind, version: u32) -> Self {
        Self { kind, version }
    }
}

/// One addressable item within one [`PresentationRef`] (§1).
///
/// Build-local: the pair `(PresentationRef, id)` is unambiguous within a
/// project, and means nothing outside one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Anchor {
    presentation: PresentationRef,
    id: u32,
}

impl Anchor {
    /// The `id`th addressable item of `presentation`.
    pub(crate) const fn at(presentation: PresentationRef, id: u32) -> Self {
        Self { presentation, id }
    }
}

/// One pass's exact descriptor (§2).
///
/// A pass id has one descriptor: two different descriptors may not share one
/// pass id, which is why the evidence and loss schema names live here rather
/// than being chosen per step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PassDescriptor {
    id: &'static str,
    source: PresentationKind,
    target: PresentationKind,
    version: u32,
    evidence_schema: &'static str,
    loss_schema: &'static str,
}

impl PassDescriptor {
    /// A descriptor for a pass from `source` to `target`.
    pub(crate) const fn new(
        id: &'static str,
        source: PresentationKind,
        target: PresentationKind,
        version: u32,
        evidence_schema: &'static str,
        loss_schema: &'static str,
    ) -> Self {
        Self {
            id,
            source,
            target,
            version,
            evidence_schema,
            loss_schema,
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    fn write_into(&self, out: &mut Vec<u8>) {
        push_text(out, self.id);
        out.push(self.source.byte());
        out.push(self.target.byte());
        out.extend_from_slice(&self.version.to_be_bytes());
        push_text(out, self.evidence_schema);
        push_text(out, self.loss_schema);
    }
}

/// The pass that turns a `.musa` document into a score track.
///
/// One descriptor rather than one per call site, because §2 says a pass id
/// has exactly one.
pub(crate) const ELABORATION: PassDescriptor = PassDescriptor::new(
    "musa.elaborate",
    PresentationKind::Source,
    PresentationKind::ScoreTrack,
    1,
    "musa.evidence.v1",
    "musa.loss.v1",
);

/// What a step offers under its pass's evidence schema (§2, §3).
///
/// Text rather than a structure because the schema is the pass's to fix and
/// this module is not the place that reads it; what matters here is that a
/// step carries evidence at all, and that the evidence takes part in the
/// graph's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Evidence(String);

impl Evidence {
    /// Evidence spelled `text`.
    pub(crate) fn of(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// A node's place in the graph. Build-local and monotone: a step may only
/// name ids smaller than its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(u32);

/// One step in an origin path (§3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// An ordinary relation: one source item produced this one.
    Preserved {
        /// What this result was made from.
        source: NodeId,
        /// Evidence under the pass's schema.
        evidence: Evidence,
    },
    /// Material made at a generation site — a repeat, an adapter expansion, a
    /// use of a shared body.
    ///
    /// It keeps **both** the shared source it instantiates and the site that
    /// instantiated it, which is what makes one body usable at several places
    /// without the uses becoming indistinguishable (§3).
    Generated {
        /// The shared source this instantiates.
        root: NodeId,
        /// The site that instantiated it.
        site: NodeId,
        /// Evidence under the pass's schema.
        evidence: Evidence,
    },
    /// A result made from several inputs, such as a label inferred from
    /// several notes, or a passage assembled from its parts.
    Combined {
        /// Every input, in order. All of them survive as parents.
        sources: Vec<NodeId>,
        /// Evidence under the pass's schema.
        evidence: Evidence,
    },
}

impl Step {
    /// Every node this step names, in order.
    fn parents(&self) -> Vec<NodeId> {
        match self {
            Self::Preserved { source, .. } => vec![*source],
            Self::Generated { root, site, .. } => vec![*root, *site],
            Self::Combined { sources, .. } => sources.clone(),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    fn write_into(&self, out: &mut Vec<u8>, renumber: &impl Fn(NodeId) -> u32) {
        let (tag, evidence) = match self {
            Self::Preserved { evidence, .. } => (b'p', evidence),
            Self::Generated { evidence, .. } => (b'g', evidence),
            Self::Combined { evidence, .. } => (b'c', evidence),
        };
        out.push(tag);
        let parents = self.parents();
        push_len(out, parents.len());
        for parent in parents {
            out.extend_from_slice(&renumber(parent).to_be_bytes());
        }
        push_text(out, &evidence.0);
    }
}

/// One node: the item it names, and how it came to be.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    anchor: Anchor,
    /// `None` for a leaf — an anchor of the input representation, which this
    /// graph does not explain and the earlier stage does.
    step: Option<Step>,
    /// Where in the source this node's item stands, when it stands anywhere.
    /// Reported, never compared: two nodes are the same node because they
    /// name the same anchor, not because they point at the same text.
    span: Option<SourceSpan>,
}

/// A finite acyclic derivation graph (§6).
///
/// Public as an opaque record, not as a shape: the graph's nodes, anchors and
/// step forms are the compiler's, and a consumer asks it questions. There is
/// deliberately no accessor that hands out `(source, target)` pairs — §6 says
/// what that flattening loses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    pass: PassDescriptor,
    nodes: Vec<Node>,
    /// Which node each score event ended at, in event order. Empty for a
    /// graph built by hand rather than read off a score.
    events: Vec<(crate::score::EventId, NodeId)>,
}

impl Derivation {
    /// An empty derivation for `pass`.
    pub(crate) const fn new(pass: PassDescriptor) -> Self {
        Self {
            pass,
            nodes: Vec::new(),
            events: Vec::new(),
        }
    }

    /// How many items this record explains.
    ///
    /// The count is what the laws below measure grafting by — that composing
    /// two derivations shares its common input rather than copying it. No
    /// product reader needs it, so it exists where it is used.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether this record explains nothing, which is what a piece with no
    /// music leaves behind.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Every place in the source that `event`'s derivation reaches, in the
    /// order it reaches them.
    ///
    /// This is the question a list-shaped path could not answer. A note
    /// instantiated from a shared body reaches **two** places at each use —
    /// the body it instantiates and the site that instantiated it — and two
    /// uses of one body reach the same body and different sites. Places are
    /// reported once each: where the graph shares a node, the answer shares
    /// a span.
    pub fn sources_of(&self, event: crate::score::EventId) -> Vec<SourceSpan> {
        let Some((_, start)) = self.events.iter().find(|(held, _)| *held == event) else {
            return Vec::new();
        };
        let mut order = Vec::new();
        self.visit(*start, &mut order);
        let mut spans: Vec<SourceSpan> = Vec::new();
        for id in order {
            let Some(node) = self.node(id) else { continue };
            if node.step.is_some() || node.anchor.presentation.kind != PresentationKind::Source {
                continue;
            }
            if let Some(span) = node.span
                && !spans.contains(&span)
            {
                spans.push(span);
            }
        }
        spans
    }

    /// Whether every item this record explains reaches a place in the source.
    ///
    /// §6's coverage obligation, asked rather than assumed: a result with no
    /// derivation is a defect in the pass that produced it.
    pub fn complete(&self) -> bool {
        self.covered()
    }

    /// The leaf for `anchor`, added if this graph does not already hold one.
    ///
    /// Interned rather than appended: where several results share one input,
    /// the shared input is one node with several children, which is the
    /// sharing §6 requires of a graft and the reason the graph is a graph.
    pub(crate) fn leaf(&mut self, anchor: Anchor, span: Option<SourceSpan>) -> NodeId {
        if let Some(found) = self.find(anchor) {
            return found;
        }
        self.push(Node {
            anchor,
            step: None,
            span,
        })
    }

    /// Record that `anchor` was preserved from `source`.
    ///
    /// `None` if `source` is not a node of this graph — a dangling anchor is
    /// refused rather than stored.
    pub(crate) fn preserved(
        &mut self,
        anchor: Anchor,
        span: Option<SourceSpan>,
        source: NodeId,
        evidence: Evidence,
    ) -> Option<NodeId> {
        self.step(anchor, span, Step::Preserved { source, evidence })
    }

    /// Record that `anchor` was generated at `site` from `root`.
    pub(crate) fn generated(
        &mut self,
        anchor: Anchor,
        span: Option<SourceSpan>,
        root: NodeId,
        site: NodeId,
        evidence: Evidence,
    ) -> Option<NodeId> {
        self.step(anchor, span, Step::Generated { root, site, evidence })
    }

    /// Record that `anchor` combines `sources`, all of which survive as
    /// parents.
    pub(crate) fn combined(
        &mut self,
        anchor: Anchor,
        span: Option<SourceSpan>,
        sources: Vec<NodeId>,
        evidence: Evidence,
    ) -> Option<NodeId> {
        self.step(anchor, span, Step::Combined { sources, evidence })
    }

    /// Every node `id` was made from, in the order the step names them.
    pub(crate) fn parents(&self, id: NodeId) -> Vec<NodeId> {
        self.node(id)
            .and_then(|node| node.step.as_ref())
            .map_or_else(Vec::new, Step::parents)
    }

    /// The anchor `id` names, if this graph holds it.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    pub(crate) fn anchor(&self, id: NodeId) -> Option<Anchor> {
        self.node(id).map(|node| node.anchor)
    }

    /// The node naming `anchor`, if this graph holds one.
    pub(crate) fn find(&self, anchor: Anchor) -> Option<NodeId> {
        self.nodes
            .iter()
            .position(|node| node.anchor == anchor)
            .and_then(|index| u32::try_from(index).ok())
            .map(NodeId)
    }

    /// Whether every node reaches a leaf in the pass's source representation
    /// (§6, coverage).
    ///
    /// A result with no derivation is a defect in the pass that produced it,
    /// so this is audited rather than assumed.
    pub(crate) fn covered(&self) -> bool {
        let mut reaches = Vec::with_capacity(self.nodes.len());
        // Nodes are monotone — a step only names earlier ids — so one forward
        // pass settles every answer without a fixed point.
        for node in &self.nodes {
            let answer = match node.step.as_ref() {
                None => node.anchor.presentation.kind == self.pass.source,
                Some(step) => step
                    .parents()
                    .iter()
                    .any(|parent| reaches.get(parent.0 as usize).copied().unwrap_or(false)),
            };
            reaches.push(answer);
        }
        reaches.iter().all(|reached| *reached)
    }

    /// This graph after `earlier`, grafted leaf by leaf (§6).
    ///
    /// Each leaf of `self` whose anchor `earlier` names is replaced by
    /// `earlier`'s graph rooted at that anchor. Where several results share
    /// one input the grafted subgraph is shared rather than copied, because
    /// each node of `earlier` is copied at most once; where one result has
    /// several inputs, all of them survive, because a step is rebuilt with
    /// every parent it had.
    ///
    /// `None` if the two passes do not meet: `earlier`'s target must be this
    /// pass's source, or the graft would join two representations that never
    /// touched.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    pub(crate) fn after(&self, earlier: &Self) -> Option<Self> {
        if earlier.pass.target != self.pass.source {
            return None;
        }
        let pass = PassDescriptor {
            id: self.pass.id,
            source: earlier.pass.source,
            target: self.pass.target,
            version: self.pass.version,
            evidence_schema: self.pass.evidence_schema,
            loss_schema: self.pass.loss_schema,
        };
        let mut grafted = Self::new(pass);
        // Where each node of `earlier`, then each node of `self`, ended up.
        let mut from_earlier: Vec<Option<NodeId>> = vec![None; earlier.nodes.len()];
        let mut from_later: Vec<Option<NodeId>> = vec![None; self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            let placed = match node.step.as_ref() {
                // A leaf `earlier` explains is replaced by `earlier`'s graph
                // rooted there; a leaf it does not is still a leaf.
                None => match earlier.find(node.anchor) {
                    Some(root) => earlier.copy_into(root, &mut grafted, &mut from_earlier)?,
                    None => grafted.leaf(node.anchor, node.span),
                },
                Some(step) => {
                    let rebuilt = rewrite(step, &from_later)?;
                    grafted.step(node.anchor, node.span, rebuilt)?
                }
            };
            *from_later.get_mut(index)? = Some(placed);
        }
        Some(grafted)
    }

    /// The bytes this graph is, for exact identity.
    ///
    /// Injective and order-independent: nodes are written in a fixed
    /// traversal from the graph's roots, renumbered as they are reached, so
    /// two graphs built in different orders that name the same derivation
    /// encode alike, and two different derivations never do. That is what
    /// lets associativity of grafting be an equality rather than a
    /// resemblance.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    pub(crate) fn canonical(&self) -> Vec<u8> {
        let order = self.traversal();
        let position = |id: NodeId| {
            order
                .iter()
                .position(|held| *held == id)
                .and_then(|index| u32::try_from(index).ok())
                .unwrap_or(u32::MAX)
        };
        let mut out = Vec::new();
        self.pass.write_into(&mut out);
        push_len(&mut out, order.len());
        for id in &order {
            let Some(node) = self.node(*id) else { continue };
            out.push(node.anchor.presentation.kind.byte());
            out.extend_from_slice(&node.anchor.presentation.version.to_be_bytes());
            out.extend_from_slice(&node.anchor.id.to_be_bytes());
            match node.step.as_ref() {
                None => out.push(b'-'),
                Some(step) => step.write_into(&mut out, &position),
            }
        }
        out
    }

    /// Depth-first from each root, parents before children, roots in node
    /// order. Every node is reached: a node that no step names is itself a
    /// root.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    fn traversal(&self) -> Vec<NodeId> {
        let mut named = vec![false; self.nodes.len()];
        for node in &self.nodes {
            for parent in node.step.as_ref().map(Step::parents).unwrap_or_default() {
                if let Some(flag) = named.get_mut(parent.0 as usize) {
                    *flag = true;
                }
            }
        }
        let mut order = Vec::with_capacity(self.nodes.len());
        for index in 0..self.nodes.len() {
            if named.get(index).copied().unwrap_or(false) {
                continue;
            }
            if let Ok(id) = u32::try_from(index) {
                self.visit(NodeId(id), &mut order);
            }
        }
        order
    }

    fn visit(&self, id: NodeId, order: &mut Vec<NodeId>) {
        if order.contains(&id) {
            return;
        }
        for parent in self.parents(id) {
            self.visit(parent, order);
        }
        order.push(id);
    }

    /// Copy `root` and everything it was made from into `into`, sharing
    /// whatever has already been copied.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
        )
    )]
    fn copy_into(&self, root: NodeId, into: &mut Self, placed: &mut Vec<Option<NodeId>>) -> Option<NodeId> {
        if let Some(Some(found)) = placed.get(root.0 as usize) {
            return Some(*found);
        }
        let node = self.node(root)?.clone();
        let copied = match node.step {
            None => into.leaf(node.anchor, node.span),
            Some(step) => {
                for parent in step.parents() {
                    self.copy_into(parent, into, placed)?;
                }
                let rebuilt = rewrite(&step, placed)?;
                into.step(node.anchor, node.span, rebuilt)?
            }
        };
        *placed.get_mut(root.0 as usize)? = Some(copied);
        Some(copied)
    }

    fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(usize::try_from(id.0).ok()?)
    }

    fn step(&mut self, anchor: Anchor, span: Option<SourceSpan>, step: Step) -> Option<NodeId> {
        if step.parents().iter().any(|parent| self.node(*parent).is_none()) {
            return None;
        }
        Some(self.push(Node {
            anchor,
            step: Some(step),
            span,
        }))
    }

    fn push(&mut self, node: Node) -> NodeId {
        let id = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        self.nodes.push(node);
        NodeId(id)
    }
}

/// `step` with every parent replaced by where it was placed.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
    )
)]
fn rewrite(step: &Step, placed: &[Option<NodeId>]) -> Option<Step> {
    let moved = |id: NodeId| placed.get(usize::try_from(id.0).ok()?).copied().flatten();
    Some(match step {
        Step::Preserved { source, evidence } => Step::Preserved {
            source: moved(*source)?,
            evidence: evidence.clone(),
        },
        Step::Generated { root, site, evidence } => Step::Generated {
            root: moved(*root)?,
            site: moved(*site)?,
            evidence: evidence.clone(),
        },
        Step::Combined { sources, evidence } => Step::Combined {
            sources: sources.iter().map(|id| moved(*id)).collect::<Option<_>>()?,
            evidence: evidence.clone(),
        },
    })
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
    )
)]
fn push_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&u64::try_from(len).unwrap_or(u64::MAX).to_be_bytes());
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "grafting and the exact encoding are proved by this module's law suite; their product caller is the second stage — notation and MIDI derivations — which prompt 127dd delivers"
    )
)]
fn push_text(out: &mut Vec<u8>, text: &str) {
    push_len(out, text.len());
    out.extend_from_slice(text.as_bytes());
}

/// The elaboration stage's derivation, read off the record each fact already
/// carries.
///
/// Every fact-making site in the elaborator writes an [`Origin`]: where the
/// note is written, and the steps that produced this instance of it. That
/// record is what a derivation is made of, so the graph is assembled from it
/// once, at the stage boundary, rather than at fourteen call sites that could
/// each forget. What the assembly adds is the *shape* the record cannot hold
/// on its own — sharing, and combined ancestry:
///
/// - **Event-track construction** gives each note a `Preserved` step from the
///   place it is written.
/// - **Reuse and repeat** give a `Generated` step keeping both the shared
///   body and the site that instantiated it, so two uses of one motif are two
///   derivations that agree on the root and differ in the site.
/// - **Payload mapping** — transposition, stretch, retrograde, inversion, a
///   checked pitch function, a lexical scale — gives a `Preserved` step whose
///   evidence names the transformation.
/// - **Succession and overlay** give a `Combined` step: a voice is made of
///   its events and a part of its voices, and all of them survive as parents.
///
/// Prefixes are interned, so two notes of one motif body share the nodes they
/// have in common instead of each carrying a private copy of the body's
/// history.
pub(crate) fn of_score(score: &crate::score::ScoreSnapshot) -> Derivation {
    let mut graph = Derivation::new(ELABORATION);
    let mut sources: Vec<SourceSpan> = Vec::new();
    let mut targets: Vec<Vec<u8>> = Vec::new();
    let mut parts = Vec::new();
    for (_, part) in score.parts().iter() {
        let mut voices = Vec::new();
        for (voice_id, voice) in part.voices() {
            let mut events = Vec::new();
            let mut reached = Vec::new();
            for event in voice.events() {
                if let Some(id) = of_event(&mut graph, &mut sources, &mut targets, &event.origin) {
                    events.push(id);
                    reached.push((event.id, id));
                }
            }
            graph.events.extend(reached);
            let key = format!("voice {} {}", part.id().0, voice_id.0);
            let Some(id) = graph.combined(
                intern_target(&mut targets, key.as_bytes()),
                None,
                events,
                Evidence::of("voice"),
            ) else {
                continue;
            };
            voices.push(id);
        }
        let key = format!("part {}", part.id().0);
        if let Some(id) = graph.combined(
            intern_target(&mut targets, key.as_bytes()),
            None,
            voices,
            Evidence::of("part"),
        ) {
            parts.push(id);
        }
    }
    // The score itself: overlay again, one level up. Without it a piece of
    // several parts would have several roots and no node standing for the
    // whole result, which is what §1's "root anchor for the whole
    // representation" asks for.
    graph.combined(
        intern_target(&mut targets, b"score"),
        None,
        parts,
        Evidence::of("score"),
    );
    graph
}

/// One event's derivation: the place it is written, then its expansion steps
/// innermost first, because that is the order they were applied in.
fn of_event(
    graph: &mut Derivation,
    sources: &mut Vec<SourceSpan>,
    targets: &mut Vec<Vec<u8>>,
    origin: &crate::origin::Origin,
) -> Option<NodeId> {
    let written = intern_source(sources, origin.definition_span);
    let mut node = graph.leaf(written, Some(origin.definition_span));
    let mut key = format!("{}:{}", origin.definition_span.start, origin.definition_span.end);
    for step in origin.expansion_path.iter().rev() {
        let spelled = step_key(step);
        key.push('/');
        key.push_str(&spelled);
        let anchor = intern_target(targets, key.as_bytes());
        let evidence = Evidence::of(spelled);
        node = match generation_site(step, origin) {
            Some(span) => {
                let site = intern_source(sources, span);
                let site = graph.leaf(site, Some(span));
                graph.generated(anchor, None, node, site, evidence)?
            }
            None => graph.preserved(anchor, None, node, evidence)?,
        };
    }
    Some(node)
}

/// Where a step instantiated material, when it is a generation site at all.
///
/// §3's distinction, stated once: a use, a template instance, a specialized
/// occurrence and a repeat iteration make new material at a place, and
/// everything else transforms material that is already there.
fn generation_site(step: &crate::origin::ExpansionStep, origin: &crate::origin::Origin) -> Option<SourceSpan> {
    use crate::origin::ExpansionStep;
    match *step {
        ExpansionStep::MotifApplication { call_site } => Some(call_site),
        ExpansionStep::TemplateInstance { site, .. } => Some(site),
        ExpansionStep::Specialization { override_site } => Some(override_site),
        ExpansionStep::RepeatIteration(_) => Some(origin.source_span),
        ExpansionStep::Transposition(_)
        | ExpansionStep::Stretch(_)
        | ExpansionStep::Retrograde
        | ExpansionStep::Inversion { .. }
        | ExpansionStep::MapNotePitches
        | ExpansionStep::ScaleContext { .. }
        | ExpansionStep::Assertion { .. }
        | ExpansionStep::KernelSplice { .. } => None,
    }
}

/// A step as evidence, and as the part of an anchor key that tells two
/// instances of one body apart.
///
/// Injective in what distinguishes instances: the iteration number, the call
/// site, the axis, the factor. Two events whose paths agree up to here share
/// the node, which is the sharing that makes this a graph.
fn step_key(step: &crate::origin::ExpansionStep) -> String {
    use crate::origin::ExpansionStep;
    match *step {
        ExpansionStep::MotifApplication { call_site } => format!("use@{}:{}", call_site.start, call_site.end),
        ExpansionStep::RepeatIteration(index) => format!("repeat#{index}"),
        ExpansionStep::Transposition(interval) => {
            format!("transpose({},{})", interval.diatonic_steps, interval.semitones)
        }
        ExpansionStep::Stretch(factor) => format!("stretch({}/{})", factor.numer(), factor.denom()),
        ExpansionStep::Retrograde => "retrograde".to_owned(),
        ExpansionStep::Inversion { ref axis } => format!("invert({axis})"),
        ExpansionStep::MapNotePitches => "map_note_pitches".to_owned(),
        ExpansionStep::ScaleContext { ref scale } => format!("scale({scale})"),
        ExpansionStep::TemplateInstance {
            ref template,
            ref alias,
            site,
            ..
        } => format!("make {template} as {alias}@{}:{}", site.start, site.end),
        ExpansionStep::Assertion { ref claim } => format!("assert({claim})"),
        ExpansionStep::KernelSplice { at } => format!("splice({}/{})", at.numer(), at.denom()),
        ExpansionStep::Specialization { override_site } => {
            format!("with@{}:{}", override_site.start, override_site.end)
        }
    }
}

fn intern_source(sources: &mut Vec<SourceSpan>, span: SourceSpan) -> Anchor {
    let id = match sources.iter().position(|held| *held == span) {
        Some(found) => found,
        None => {
            sources.push(span);
            sources.len().saturating_sub(1)
        }
    };
    Anchor::at(
        PresentationRef::at(PresentationKind::Source, 1),
        u32::try_from(id).unwrap_or(u32::MAX),
    )
}

fn intern_target(targets: &mut Vec<Vec<u8>>, key: &[u8]) -> Anchor {
    let id = match targets.iter().position(|held| held == key) {
        Some(found) => found,
        None => {
            targets.push(key.to_vec());
            targets.len().saturating_sub(1)
        }
    };
    Anchor::at(
        PresentationRef::at(PresentationKind::ScoreTrack, 1),
        u32::try_from(id).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
// A law suite reports a violated law by failing, and the helpers below take
// apart a value whose existence the law has already asserted.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const SOURCE: PresentationRef = PresentationRef::at(PresentationKind::Source, 1);
    const TRACK: PresentationRef = PresentationRef::at(PresentationKind::ScoreTrack, 1);

    /// A second stage, so that grafting has two passes to join.
    const RENDERING: PassDescriptor = PassDescriptor::new(
        "musa.render",
        PresentationKind::ScoreTrack,
        PresentationKind::ScoreTrack,
        1,
        "musa.evidence.v1",
        "musa.loss.v1",
    );

    fn source(id: u32) -> Anchor {
        Anchor::at(SOURCE, id)
    }

    fn track(id: u32) -> Anchor {
        Anchor::at(TRACK, id)
    }

    /// One body, used at two sites: the shape blocker 3 said a list could not
    /// hold.
    fn two_uses() -> Derivation {
        let mut graph = Derivation::new(ELABORATION);
        let body = graph.leaf(source(0), None);
        let first = graph.leaf(source(1), None);
        let second = graph.leaf(source(2), None);
        graph
            .generated(track(0), None, body, first, Evidence::of("use"))
            .expect("the body and the first site are in the graph");
        graph
            .generated(track(1), None, body, second, Evidence::of("use"))
            .expect("the body and the second site are in the graph");
        graph
    }

    #[test]
    fn two_uses_of_one_body_agree_on_the_root_and_differ_in_the_site() {
        let graph = two_uses();
        let first = graph.find(track(0)).expect("the first use is a node");
        let second = graph.find(track(1)).expect("the second use is a node");
        let (first, second) = (graph.parents(first), graph.parents(second));
        assert_eq!(
            first.first(),
            second.first(),
            "two uses of one body must instantiate one shared root"
        );
        assert_ne!(
            first.get(1),
            second.get(1),
            "two uses of one body must keep their two generation sites"
        );
    }

    #[test]
    fn a_combined_result_keeps_every_parent() {
        let mut graph = Derivation::new(ELABORATION);
        let notes: Vec<NodeId> = (0..4).map(|id| graph.leaf(source(id), None)).collect();
        let chord = graph
            .combined(track(0), None, notes.clone(), Evidence::of("chord"))
            .expect("every note is in the graph");
        assert_eq!(
            graph.parents(chord),
            notes,
            "a combined result that dropped a parent would not say which notes supported it"
        );
    }

    #[test]
    fn a_dangling_anchor_is_refused() {
        let mut graph = Derivation::new(ELABORATION);
        let absent = NodeId(7);
        assert!(
            graph.preserved(track(0), None, absent, Evidence::of("note")).is_none(),
            "a step naming a node the graph does not hold must be refused"
        );
        let held = graph.leaf(source(0), None);
        assert!(
            graph
                .combined(track(0), None, vec![held, absent], Evidence::of("chord"))
                .is_none(),
            "one absent parent is enough to refuse a combined step"
        );
    }

    #[test]
    fn a_cycle_cannot_be_written() {
        // Not a check that runs and fails: a step may only name ids the graph
        // already holds, and a node's own id does not exist until the step
        // that would name it is built. The law is that the ids a node could
        // name are all smaller than its own.
        let graph = two_uses();
        for index in 0..graph.len() {
            let id = NodeId(u32::try_from(index).expect("a small graph"));
            for parent in graph.parents(id) {
                assert!(
                    parent < id,
                    "a node named an id at or after its own, so the graph could cycle"
                );
            }
        }
    }

    /// The score-track stage, rooted at track anchors, feeding a later stage
    /// whose leaves are those same anchors.
    fn stages() -> (Derivation, Derivation, Derivation) {
        let earlier = two_uses();
        let mut middle = Derivation::new(RENDERING);
        let first = middle.leaf(track(0), None);
        let second = middle.leaf(track(1), None);
        middle
            .combined(track(2), None, vec![first, second], Evidence::of("beam"))
            .expect("both uses are in the graph");
        let mut later = Derivation::new(RENDERING);
        let beamed = later.leaf(track(2), None);
        later
            .preserved(track(3), None, beamed, Evidence::of("engrave"))
            .expect("the beam is in the graph");
        (earlier, middle, later)
    }

    #[test]
    fn grafting_covers_every_anchor() {
        let (earlier, middle, later) = stages();
        let composed = later
            .after(&middle)
            .and_then(|joined| joined.after(&earlier))
            .expect("the three passes meet");
        assert!(
            composed.covered(),
            "a composed result with an anchor that reaches no source anchor is a defect in the pass"
        );
    }

    #[test]
    fn grafting_is_associative() {
        let (earlier, middle, later) = stages();
        let left = later
            .after(&middle)
            .and_then(|joined| joined.after(&earlier))
            .expect("the three passes meet");
        let right = later
            .after(&middle.after(&earlier).expect("the first two passes meet"))
            .expect("the three passes meet");
        assert_eq!(
            left.canonical(),
            right.canonical(),
            "grafting three stages in either grouping must give one graph"
        );
    }

    #[test]
    fn grafting_shares_a_common_input_and_keeps_every_parent() {
        let (earlier, middle, _) = stages();
        let composed = middle.after(&earlier).expect("the two passes meet");
        let beam = composed.find(track(2)).expect("the beam survives the graft");
        let parents = composed.parents(beam);
        assert_eq!(parents.len(), 2, "the beam kept both of its inputs");
        let roots: Vec<Option<Anchor>> = parents
            .iter()
            .map(|parent| composed.parents(*parent).first().and_then(|id| composed.anchor(*id)))
            .collect();
        assert_eq!(
            roots.first(),
            roots.get(1),
            "the shared body must be grafted once and shared, not copied per use"
        );
        assert_eq!(
            composed.len(),
            earlier.len().saturating_add(1),
            "grafting copied a node twice, so the sharing the graph exists to record was lost"
        );
    }

    #[test]
    fn two_stages_that_do_not_meet_are_refused() {
        let (earlier, middle, _) = stages();
        assert!(
            earlier.after(&middle).is_none(),
            "grafting must refuse two passes whose representations never touched"
        );
    }

    #[test]
    fn a_graph_is_exactly_its_own_bytes() {
        let (earlier, middle, later) = stages();
        let composed = later
            .after(&middle)
            .and_then(|joined| joined.after(&earlier))
            .expect("the three passes meet");
        let rebuilt = later
            .after(&middle)
            .and_then(|joined| joined.after(&earlier))
            .expect("the three passes meet");
        assert_eq!(
            composed.canonical(),
            rebuilt.canonical(),
            "one derivation, built twice, must encode one way"
        );
        for other in [&earlier, &middle, &later] {
            assert_ne!(
                composed.canonical(),
                other.canonical(),
                "two different derivations must not encode alike"
            );
        }
        // Evidence takes part in identity: two graphs alike in shape and
        // different in what they claim are two graphs.
        let mut relabelled = Derivation::new(ELABORATION);
        let body = relabelled.leaf(source(0), None);
        let site = relabelled.leaf(source(1), None);
        relabelled
            .generated(track(0), None, body, site, Evidence::of("repeat"))
            .expect("both are in the graph");
        let mut original = Derivation::new(ELABORATION);
        let body = original.leaf(source(0), None);
        let site = original.leaf(source(1), None);
        original
            .generated(track(0), None, body, site, Evidence::of("use"))
            .expect("both are in the graph");
        assert_ne!(
            relabelled.canonical(),
            original.canonical(),
            "two steps with different evidence must not encode alike"
        );
    }

    #[test]
    fn an_uncovered_result_is_reported_rather_than_hidden() {
        let mut graph = Derivation::new(ELABORATION);
        // A leaf in the *target* representation that no earlier stage
        // explains: the shape a pass leaves behind when it forgets to record.
        graph.leaf(track(9), None);
        assert!(
            !graph.covered(),
            "a track anchor with no path to a source anchor must fail coverage"
        );
    }
}
