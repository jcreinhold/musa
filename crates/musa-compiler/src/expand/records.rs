use crate::compile::SourceDocument;
use musa_score::diagnose::Diagnostic;
use musa_score::origin::{SourceMap, SourceSpan};

/// The phase-tagged logical charges of `26-language-design-decision.md` §3.5.
///
/// Four counters, each charged by the phase that incurs it, and every one of
/// them a function of the *result* rather than of the work done to reach it.
/// That is what makes a cache hit cost what the miss it replaces cost: the
/// second of two identical regions is charged from the record, not from the
/// evaluation it skipped, so cache warmth cannot change whether a file is
/// accepted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Charges {
    /// One per region expanded, hit or miss.
    pub(crate) expansion_steps: u64,
    /// Nodes a transformer built rather than preserved.
    pub(crate) generated_syntax_nodes: u64,
    /// Equations the transformer's own checking solved.
    /// Reductions the transformer's own evaluation took.
    pub(crate) evaluation_steps: u64,
}

impl Charges {
    pub(crate) fn add(&mut self, other: Self) {
        self.expansion_steps = self.expansion_steps.saturating_add(other.expansion_steps);
        self.generated_syntax_nodes = self.generated_syntax_nodes.saturating_add(other.generated_syntax_nodes);
        self.evaluation_steps = self.evaluation_steps.saturating_add(other.evaluation_steps);
    }
}

/// What one successful expansion leaves behind
/// (`26-language-design-decision.md` §3.4).
///
/// A compiler source map, and — as
/// `docs/rules/across-stages/02-derivation-diagrams.md` §6 puts it — a source of
/// `Generated` steps and nothing more. It says that an expression was produced
/// at a site by a named adapter at an exact version. It does not say that the
/// music the expression later makes was derived from anything: that claim is
/// the derivation graph's, and it starts at the use site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpansionRecord {
    /// The module path the header named: `std::adapters::doubled`.
    pub(crate) adapter: String,
    /// The exact source the adapter package was read at, as hex.
    pub(crate) version: String,
    /// The region, in the composer's own text.
    pub(crate) use_site: SourceSpan,
    /// What the adapter was handed.
    pub(crate) input: crate::quote::Syntax,
    /// What it answered with.
    pub(crate) output: crate::quote::Syntax,
    /// Every node of the region, by range, in the order `syntax_anchor`
    /// numbers them.
    ///
    /// The compiler's half of an anchor. The adapter emits a number into the
    /// value it produces; a later ordinary package function complaining about
    /// that value carries the number along, and this is what turns it back into
    /// a place in the composer's text. It lives on the record rather than in a
    /// compilation-wide map because a number means nothing without the region
    /// that minted it, and the record is what a reader already holds.
    pub(crate) anchors: Vec<SourceSpan>,
    /// The expansion this one was produced inside, if any.
    ///
    /// Always `None` today, and a field rather than an omission because the
    /// record's shape is fixed by §3.4 and a reader should not have to know
    /// which parts of it this build can reach. An adapter cannot emit a
    /// region, so an expansion has no children; if that rule is ever relaxed,
    /// this is where the parent goes.
    pub(crate) parent: Option<usize>,
}

impl ExpansionRecord {
    /// The range an anchor names, or `None` for a number this region never
    /// minted.
    ///
    /// Total for the reason the anchor is a number and not a range: a forged
    /// anchor addresses nothing here, so the worst it can do is leave a
    /// package's complaint without a place — which is a mislocated sentence,
    /// not a way to read text the adapter was never handed.
    ///
    /// Exercised by this prompt's tests; the trials of prompts 127dcf and
    /// 127dcg are what call it in earnest, when a package's `validate` starts
    /// carrying anchors into complaints a piece has to place.
    #[cfg(test)]
    pub(crate) fn anchor(&self, number: u64) -> Option<SourceSpan> {
        self.anchors.get(usize::try_from(number).ok()?).copied()
    }
}

/// The phase's whole result.
pub(crate) struct Expansion {
    /// The text steps 5 onwards read.
    pub(crate) document: SourceDocument,
    /// How to say where any of it came from.
    pub(crate) map: SourceMap,
    /// Every successful expansion, in source order.
    pub(crate) records: Vec<ExpansionRecord>,
    /// What the phase charged.
    pub(crate) charges: Charges,
    /// Everything that went wrong, already anchored in the composer's text.
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Expansion {
    /// The result for a document with no region in it, which is every document
    /// the language has had until now.
    pub(crate) fn unchanged(source: &SourceDocument) -> Self {
        Self {
            document: SourceDocument::new(source.text(), source.name()),
            map: SourceMap::default(),
            records: Vec::new(),
            charges: Charges::default(),
            diagnostics: Vec::new(),
        }
    }

    /// The regions, as the derivation graph anchors a generated event at them.
    pub(crate) fn anchors(&self) -> Vec<musa_score::derivation::ExpansionAnchor> {
        self.records
            .iter()
            .map(|record| musa_score::derivation::ExpansionAnchor {
                site: record.use_site,
                adapter: record.adapter.clone(),
            })
            .collect()
    }
}
