use crate::compile::SourceDocument;
use crate::diagnose::Diagnostic;
use crate::origin::SourceSpan;

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
    pub(crate) input: crate::syntax::Syntax,
    /// What it answered with.
    pub(crate) output: crate::syntax::Syntax,
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

/// Where one stretch of the expanded text came from in the composer's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Replacement {
    /// The stretch, in expanded coordinates.
    pub(crate) from: u32,
    pub(crate) to: u32,
    /// The region it replaced, in original coordinates.
    pub(crate) original: SourceSpan,
}

/// The translation from the text the compiler read to the text the composer
/// wrote.
///
/// Two rules and no more. A position outside every expansion moves by the
/// accumulated difference in length of the expansions before it, which is
/// exact. A position *inside* an expansion becomes the region that produced it,
/// because there is no finer answer that is true: the character is not in the
/// composer's file at all, and pointing at the region is pointing at the only
/// text they can edit to change it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceMap {
    pub(crate) replacements: Vec<Replacement>,
}

impl SourceMap {
    /// Whether this map changes anything, which it does not for the enormous
    /// majority of files: no region, no translation, no walk over a snapshot.
    pub(crate) fn is_identity(&self) -> bool {
        self.replacements.is_empty()
    }

    /// Where `offset` in the expanded text stands in the original.
    fn offset(&self, offset: u32) -> u32 {
        let mut shift: i64 = 0;
        for replacement in &self.replacements {
            if offset < replacement.from {
                break;
            }
            if offset < replacement.to {
                return replacement.original.start;
            }
            let generated = i64::from(replacement.to).saturating_sub(i64::from(replacement.from));
            let original = i64::from(replacement.original.end).saturating_sub(i64::from(replacement.original.start));
            shift = shift.saturating_add(original.saturating_sub(generated));
        }
        u32::try_from(i64::from(offset).saturating_add(shift)).unwrap_or(u32::MAX)
    }

    /// Where `span` in the expanded text stands in the original.
    ///
    /// A span that lies inside one expansion becomes that expansion's region,
    /// whole: half of a generated call is not a place, and a caret under it
    /// would be a caret under nothing.
    pub(crate) fn span(&self, span: SourceSpan) -> SourceSpan {
        for replacement in &self.replacements {
            if span.start >= replacement.from && span.end <= replacement.to {
                return replacement.original;
            }
        }
        SourceSpan::new(self.offset(span.start), self.offset(span.end))
    }

    /// The same, for a span a caller may not have.
    pub(crate) fn maybe(&self, span: Option<SourceSpan>) -> Option<SourceSpan> {
        span.map(|span| self.span(span))
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
    pub(crate) fn anchors(&self) -> Vec<crate::derivation::ExpansionAnchor> {
        self.records
            .iter()
            .map(|record| crate::derivation::ExpansionAnchor {
                site: record.use_site,
                adapter: record.adapter.clone(),
            })
            .collect()
    }
}
