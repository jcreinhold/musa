//! Rowan language plumbing: the `Language` marker and node type aliases.
//!
//! Exposing Rowan's node types at this boundary is intentional (roadmap
//! §15.2); green-node *construction* details stay private to the parser.

use crate::SyntaxKind;

/// The Rowan [`rowan::Language`] marker for `.musa` source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MusaLanguage {}

impl rowan::Language for MusaLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(u16::from(kind))
    }
}

/// A concrete syntax node.
pub type SyntaxNode = rowan::SyntaxNode<MusaLanguage>;
/// A concrete syntax token.
pub type SyntaxToken = rowan::SyntaxToken<MusaLanguage>;
/// A node or token.
pub type SyntaxElement = rowan::SyntaxElement<MusaLanguage>;
