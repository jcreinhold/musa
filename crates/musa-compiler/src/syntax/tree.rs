//! The syntax value itself: what a node is, where it came from, and the
//! reads over a whole tree that later stages ask for.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::category::Delimiter;
use super::path::{NodePath, PathStep, Scope};
use super::print::print;
use crate::origin::SourceSpan;

/// A syntax value, shown as the text it prints to.
///
/// [`print`] is the authority on that and is reused rather than approximated: a
/// diagnostic that showed a tree one way while the expansion emitted it another
/// would be describing a program nobody wrote. The generated-name report [`print`]
/// also carries is a gate's concern and not a reader's, so it is dropped here.
impl std::fmt::Display for Syntax {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&print(self).text)
    }
}

/// Where a node came from.
///
/// It has no eliminator on purpose. A transformer holds a node in order to
/// point at it — a diagnostic, a preserved use — and cannot read the range,
/// so it cannot claim a position it did not receive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceInfo {
    /// Text the composer wrote, at the structural path the reader gave it.
    Original { span: SourceSpan, path: NodePath },
    /// Something an expansion made, at the path it made it.
    Generated(NodePath),
}

impl SourceInfo {
    /// Where this node sits.
    ///
    /// A node carries its own path rather than having one reconstructed for it,
    /// which is what lets the fold hand a step function the path of the node it
    /// is looking at without a second traversal — and what makes the path of a
    /// preserved input node the same path however deep in the output it ends up.
    pub(crate) const fn path(&self) -> &NodePath {
        match self {
            Self::Original { path, .. } | Self::Generated(path) => path,
        }
    }
}

/// A finite syntax value.
///
/// The four forms of `26-language-design-decision.md` §3.3, with the source
/// information and the hygiene scopes the compiler owns held inside rather than
/// handed out. It contains no arrow at any depth, so it is storable data and
/// the kind system refuses a syntax value that hides a closure without being
/// asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// A node the reader expected and did not find. Kept rather than dropped,
    /// so that a transformer over a half-written region still has a shape to
    /// fold and a node to point a diagnostic at.
    Missing(SourceInfo),
    /// A token, by the kind the reader gave it and its exact text.
    ///
    /// The kind *is* the lexer's own, rather than a name printed from it and
    /// compared back as text. That is what makes the phase's `TokenKind` a
    /// generated type rather than a second table to keep in step: there is no
    /// second table.
    Token {
        info: SourceInfo,
        kind: musa_language::SyntaxKind,
        text: String,
    },
    /// A name, with the hygiene scopes it carries.
    Identifier {
        info: SourceInfo,
        name: String,
        scopes: Vec<Scope>,
    },
    /// A delimited or layout group and its children, in source order.
    Group {
        info: SourceInfo,
        delimiter: Delimiter,
        children: Vec<Self>,
    },
}

impl Syntax {
    /// Where this node came from.
    pub(crate) const fn info(&self) -> &SourceInfo {
        match self {
            Self::Missing(info)
            | Self::Token { info, .. }
            | Self::Identifier { info, .. }
            | Self::Group { info, .. } => info,
        }
    }

    /// The sub-node at `path`, if this value has one there.
    ///
    /// How a transformer preserves input: the fold reveals a node's path, and
    /// this turns that path back into the node, with its own source
    /// information intact. It reveals nothing the fold did not already, and it
    /// answers `None` for a path from another expansion or another shape,
    /// which is why it is total.
    pub(crate) fn at(&self, path: &NodePath) -> Option<&Self> {
        let root = self.info().path();
        if root.expansion != path.expansion || !root.steps.is_empty() {
            return None;
        }
        self.descend(&path.steps)
    }

    fn descend(&self, steps: &[PathStep]) -> Option<&Self> {
        let Some((first, rest)) = steps.split_first() else {
            return Some(self);
        };
        let PathStep::Child(index) = *first else {
            return None;
        };
        let Self::Group { children, .. } = self else {
            return None;
        };
        children.get(usize::try_from(index).ok()?)?.descend(rest)
    }

    /// The anchor of the sub-node at `path`: its position in this value's own
    /// reading order, counting this node as zero.
    ///
    /// A function of this value alone — not a counter, not the region's ordinal
    /// in the file, and not anything the compiler allocates. That is what keeps
    /// prompt 127dc's law intact: two identical regions anywhere in a file mint
    /// the same anchors, so one region still has one answer and the cache still
    /// replays a miss's charge. `None` where `at` is `None`, and for the same
    /// reason — an adapter anchors a node it was given, and a node it built has
    /// no place in the composer's text to be anchored to.
    pub(crate) fn anchor(&self, path: &NodePath) -> Option<u64> {
        let root = self.info().path();
        if root.expansion != path.expansion || !root.steps.is_empty() {
            return None;
        }
        self.count_to(&path.steps, 0)
    }

    fn count_to(&self, steps: &[PathStep], here: u64) -> Option<u64> {
        let Some((first, rest)) = steps.split_first() else {
            return Some(here);
        };
        let PathStep::Child(index) = *first else {
            return None;
        };
        let Self::Group { children, .. } = self else {
            return None;
        };
        let index = usize::try_from(index).ok()?;
        // Pre-order: this node, then each earlier sibling's whole subtree,
        // then the wanted child. `shape` already counts a subtree's nodes.
        let mut reached = here.saturating_add(1);
        for earlier in children.get(..index)? {
            reached = reached.saturating_add(earlier.shape().0);
        }
        children.get(index)?.count_to(rest, reached)
    }

    /// Every node's source range, in the order [`Syntax::anchor`] numbers.
    ///
    /// The compiler-owned half of the anchor: the number alone means nothing
    /// without this table, which is what keeps a forged anchor a mislocated
    /// sentence rather than a way to read a range the adapter was never given.
    /// A region read from source is `Original` throughout, so `fallback` is
    /// reached only by a value that was built rather than read.
    pub(crate) fn spans(&self, fallback: SourceSpan) -> Vec<SourceSpan> {
        let mut out = Vec::new();
        self.push_spans(fallback, &mut out);
        out
    }

    fn push_spans(&self, fallback: SourceSpan, out: &mut Vec<SourceSpan>) {
        out.push(match self.info() {
            SourceInfo::Original { span, .. } => *span,
            SourceInfo::Generated(_) => fallback,
        });
        if let Self::Group { children, .. } = self {
            for child in children {
                child.push_spans(fallback, out);
            }
        }
    }

    /// How much of the evaluator's budget this value occupies: one node per
    /// node, and the text it holds as its size.
    pub(crate) fn shape(&self) -> (u64, u64) {
        let size = |text: &str| u64::try_from(text.len()).unwrap_or(u64::MAX);
        match self {
            Self::Missing(_) => (1, 0),
            // A kind and a delimiter are two bytes and one, now that neither
            // is a string. Charging them their stored size rather than their
            // spelling's is the same rule prompt 127dcec set — a value is
            // charged what it occupies — applied to a value that shrank.
            Self::Token { text, .. } => (1, size(text).saturating_add(2)),
            Self::Identifier { name, .. } => (1, size(name)),
            Self::Group { children, .. } => children.iter().fold((1, 1), |(nodes, bytes), child| {
                let (theirs, their_bytes) = child.shape();
                (nodes.saturating_add(theirs), bytes.saturating_add(their_bytes))
            }),
        }
    }
}
