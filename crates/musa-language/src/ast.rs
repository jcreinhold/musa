//! Typed views over the concrete syntax tree (roadmap §10.5).
//!
//! Each wrapper is a thin cast around a [`SyntaxNode`] with accessors for its
//! significant children. Wrappers cache nothing and perform no semantic
//! resolution: `NoteStmt::pitch` returns the written text `gs4`, not a
//! computed pitch. New wrappers are cheap; create them with
//! [`AstNode::cast`].

use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode, SyntaxToken};

/// A typed view over a [`SyntaxNode`].
pub trait AstNode: Sized {
    /// Wrap `node` if it has the expected kind.
    fn cast(node: SyntaxNode) -> Option<Self>;
    /// The underlying untyped node.
    fn syntax(&self) -> &SyntaxNode;
}

fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

fn children<N: AstNode>(node: &SyntaxNode) -> Vec<N> {
    node.children().filter_map(N::cast).collect()
}

fn find_token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == kind)
}

fn token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    find_token(node, kind).map(|token| token.text().to_string())
}

macro_rules! wrapper {
    ($name:ident, $kind:expr) => {
        impl AstNode for $name {
            fn cast(node: SyntaxNode) -> Option<Self> {
                (node.kind() == $kind).then_some(Self(node))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

/// `piece "name" { ... }` — the root declaration of a document.
pub struct PieceDecl(SyntaxNode);
wrapper!(PieceDecl, SyntaxKind::PieceDecl);

impl PieceDecl {
    /// Cast the root node of a document to its piece declaration.
    pub fn from_root(node: &SyntaxNode) -> Option<Self> {
        child(node)
    }

    /// The piece title from its string literal (without quotes).
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| text.trim_matches('"').to_string())
    }

    /// The `tempo` statement, if present.
    pub fn tempo(&self) -> Option<TempoStmt> {
        child(&self.0)
    }

    /// The `meter` statement, if present.
    pub fn meter(&self) -> Option<MeterStmt> {
        child(&self.0)
    }

    /// The `key` statement, if present.
    pub fn key(&self) -> Option<KeyStmt> {
        child(&self.0)
    }

    /// All motif declarations.
    pub fn motifs(&self) -> Vec<MotifDecl> {
        children(&self.0)
    }

    /// The `score` block, if present.
    pub fn score(&self) -> Option<ScoreDecl> {
        child(&self.0)
    }
}

/// `tempo quarter = 72;`
pub struct TempoStmt(SyntaxNode);
wrapper!(TempoStmt, SyntaxKind::TempoStmt);

/// `meter 4/4;`
pub struct MeterStmt(SyntaxNode);
wrapper!(MeterStmt, SyntaxKind::MeterStmt);

impl MeterStmt {
    /// The meter fraction text, e.g. `4/4`.
    pub fn value(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational)
    }
}

/// `key a minor;`
pub struct KeyStmt(SyntaxNode);
wrapper!(KeyStmt, SyntaxKind::KeyStmt);

/// `motif sigh(root: pitch = e5) { ... }`
pub struct MotifDecl(SyntaxNode);
wrapper!(MotifDecl, SyntaxKind::MotifDecl);

impl MotifDecl {
    /// The motif name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// `score { ... }`
pub struct ScoreDecl(SyntaxNode);
wrapper!(ScoreDecl, SyntaxKind::ScoreDecl);

impl ScoreDecl {
    /// All parts in the score.
    pub fn parts(&self) -> Vec<PartDecl> {
        children(&self.0)
    }
}

/// `part violin { ... }`
pub struct PartDecl(SyntaxNode);
wrapper!(PartDecl, SyntaxKind::PartDecl);

impl PartDecl {
    /// The part name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The part's voices.
    pub fn voices(&self) -> Vec<VoiceDecl> {
        children(&self.0)
    }
}

/// `voice lead { ... }`
pub struct VoiceDecl(SyntaxNode);
wrapper!(VoiceDecl, SyntaxKind::VoiceDecl);

impl VoiceDecl {
    /// The voice name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The voice's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        let mut items = Vec::new();
        for node in self.0.children() {
            let kind = node.kind();
            let item = if kind == SyntaxKind::NoteStmt {
                NoteStmt::cast(node).map(VoiceItem::Note)
            } else if kind == SyntaxKind::RestStmt {
                RestStmt::cast(node).map(VoiceItem::Rest)
            } else if kind == SyntaxKind::ChordStmt {
                ChordStmt::cast(node).map(VoiceItem::Chord)
            } else if kind == SyntaxKind::UseStmt {
                UseStmt::cast(node).map(VoiceItem::Use)
            } else if kind == SyntaxKind::TransposeStmt {
                TransposeStmt::cast(node).map(VoiceItem::Transpose)
            } else if kind == SyntaxKind::RepeatStmt {
                RepeatStmt::cast(node).map(VoiceItem::Repeat)
            } else {
                None
            };
            items.extend(item);
        }
        items
    }
}

/// One item in a voice or motif body.
pub enum VoiceItem {
    /// `<pitch-or-ref> <duration>;`
    Note(NoteStmt),
    /// `rest <duration>;`
    Rest(RestStmt),
    /// `chord [...] <duration>;`
    Chord(ChordStmt),
    /// `use name(...);`
    Use(UseStmt),
    /// `transpose ... { ... }`
    Transpose(TransposeStmt),
    /// `repeat n { ... }`
    Repeat(RepeatStmt),
}

/// `<pitch-or-ref> <duration>;`
pub struct NoteStmt(SyntaxNode);
wrapper!(NoteStmt, SyntaxKind::NoteStmt);

impl NoteStmt {
    /// The written pitch (`gs4`) or pitch reference (`root`).
    pub fn pitch(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral).or_else(|| token_text(&self.0, SyntaxKind::Identifier))
    }

    /// The duration text (`1/2`, `3/8`, `1`).
    pub fn duration(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational).or_else(|| token_text(&self.0, SyntaxKind::Integer))
    }
}

/// `rest <duration>;`
pub struct RestStmt(SyntaxNode);
wrapper!(RestStmt, SyntaxKind::RestStmt);

impl RestStmt {
    /// The duration text.
    pub fn duration(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational).or_else(|| token_text(&self.0, SyntaxKind::Integer))
    }
}

/// `chord [a3, c4, e4] 1/2;`
pub struct ChordStmt(SyntaxNode);
wrapper!(ChordStmt, SyntaxKind::ChordStmt);

impl ChordStmt {
    /// The chord's written pitches, in source order.
    pub fn pitches(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::PitchLiteral)
            .map(|token| token.text().to_string())
            .collect()
    }
}

/// `use sigh();`
pub struct UseStmt(SyntaxNode);
wrapper!(UseStmt, SyntaxKind::UseStmt);

impl UseStmt {
    /// The referenced motif name.
    pub fn motif(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// `transpose down P5 { ... }`
pub struct TransposeStmt(SyntaxNode);
wrapper!(TransposeStmt, SyntaxKind::TransposeStmt);

/// `repeat 4 { ... }`
pub struct RepeatStmt(SyntaxNode);
wrapper!(RepeatStmt, SyntaxKind::RepeatStmt);

impl RepeatStmt {
    /// The repeat count text.
    pub fn count(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Integer)
    }
}
