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

        impl Clone for $name {
            fn clone(&self) -> Self {
                Self(self.0.clone())
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

    /// The declared parameters, in order.
    pub fn params(&self) -> Vec<Param> {
        let mut params = Vec::new();
        let mut tokens = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| !token.kind().is_trivia())
            .peekable();
        // Skip `motif` and the name.
        drop(tokens.next());
        drop(tokens.next());
        while let Some(token) = tokens.next() {
            if token.kind() == SyntaxKind::LBrace {
                break;
            }
            if token.kind() != SyntaxKind::Identifier {
                continue;
            }
            let name = token.text().to_string();
            // `:` then the type keyword/identifier.
            drop(tokens.next());
            let kind = tokens.next().map_or_else(String::new, |kind| kind.text().to_string());
            let default = if tokens.peek().is_some_and(|next| next.kind() == SyntaxKind::Equals) {
                drop(tokens.next());
                tokens.next().map(|value| value.text().to_string())
            } else {
                None
            };
            params.push(Param { name, kind, default });
        }
        params
    }

    /// The motif's body items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// A declared motif parameter: `name: kind = default`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The parameter name (`root`).
    pub name: String,
    /// The declared kind (`pitch` or `duration`).
    pub kind: String,
    /// The default value text, when declared (`e5`, `1/8`).
    pub default: Option<String>,
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
        voice_items(&self.0)
    }
}

/// Extract the items of a voice-like block (voice body, motif body,
/// transpose block, repeat block).
fn voice_items(node: &SyntaxNode) -> Vec<VoiceItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::Block {
            items.extend(voice_items(&child));
            continue;
        }
        let kind = child.kind();
        let item = if kind == SyntaxKind::NoteStmt {
            NoteStmt::cast(child).map(VoiceItem::Note)
        } else if kind == SyntaxKind::RestStmt {
            RestStmt::cast(child).map(VoiceItem::Rest)
        } else if kind == SyntaxKind::ChordStmt {
            ChordStmt::cast(child).map(VoiceItem::Chord)
        } else if kind == SyntaxKind::UseStmt {
            UseStmt::cast(child).map(VoiceItem::Use)
        } else if kind == SyntaxKind::TransposeStmt {
            TransposeStmt::cast(child).map(VoiceItem::Transpose)
        } else if kind == SyntaxKind::RepeatStmt {
            RepeatStmt::cast(child).map(VoiceItem::Repeat)
        } else if kind == SyntaxKind::SlurStmt {
            SlurStmt::cast(child).map(VoiceItem::Slur)
        } else if kind == SyntaxKind::DynamicStmt {
            DynamicStmt::cast(child).map(VoiceItem::Dynamic)
        } else if kind == SyntaxKind::TupletStmt {
            TupletStmt::cast(child).map(VoiceItem::Tuplet)
        } else {
            None
        };
        items.extend(item);
    }
    items
}

/// One item in a voice or motif body.
#[derive(Clone)]
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
    /// `slur { ... }`
    Slur(SlurStmt),
    /// `dynamic p;`
    Dynamic(DynamicStmt),
    /// `tuplet 3/2 { ... }`
    Tuplet(TupletStmt),
}

/// The articulation names trailing a note or chord's duration.
fn articulation_names(node: &SyntaxNode) -> Vec<String> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::ArticulationList)
        .into_iter()
        .flat_map(|list| {
            list.children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a statement carries the postfix tie mark.
fn has_tie(node: &SyntaxNode) -> bool {
    find_token(node, SyntaxKind::Tilde).is_some()
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

    /// The articulation names written after the duration, in source order.
    pub fn articulations(&self) -> Vec<String> {
        articulation_names(&self.0)
    }

    /// Whether this note is tied to the statement that follows it.
    pub fn tied(&self) -> bool {
        has_tie(&self.0)
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

    /// The articulation names written after the duration, in source order.
    pub fn articulations(&self) -> Vec<String> {
        articulation_names(&self.0)
    }

    /// Whether this chord is tied to the statement that follows it.
    pub fn tied(&self) -> bool {
        has_tie(&self.0)
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

    /// The call arguments, in order (pitch literals, parameter references,
    /// or durations).
    pub fn args(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| {
                token.kind() == SyntaxKind::PitchLiteral
                    || token.kind() == SyntaxKind::Identifier
                    || token.kind() == SyntaxKind::Rational
                    || token.kind() == SyntaxKind::Integer
            })
            .skip(1) // the motif name
            .map(|token| token.text().to_string())
            .collect()
    }
}

/// `transpose down P5 { ... }`
pub struct TransposeStmt(SyntaxNode);
wrapper!(TransposeStmt, SyntaxKind::TransposeStmt);

impl TransposeStmt {
    /// Whether the direction keyword is `down`.
    pub fn is_down(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::DownKw)
    }

    /// The interval literal text (`P5`, `m3`).
    pub fn interval(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::IntervalLiteral)
    }

    /// The block's items.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `slur { ... }`
pub struct SlurStmt(SyntaxNode);
wrapper!(SlurStmt, SyntaxKind::SlurStmt);

impl SlurStmt {
    /// The slurred items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `dynamic mf;`
pub struct DynamicStmt(SyntaxNode);
wrapper!(DynamicStmt, SyntaxKind::DynamicStmt);

impl DynamicStmt {
    /// The marking text (`p`, `mf`, `ff`).
    pub fn mark(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// `tuplet 3/2 { ... }`
pub struct TupletStmt(SyntaxNode);
wrapper!(TupletStmt, SyntaxKind::TupletStmt);

impl TupletStmt {
    /// The ratio text (`3/2`), unreduced: `4/4` is not `1/1`.
    pub fn ratio(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational)
    }

    /// The block's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `repeat 4 { ... }`
pub struct RepeatStmt(SyntaxNode);
wrapper!(RepeatStmt, SyntaxKind::RepeatStmt);

impl RepeatStmt {
    /// The repeat count text.
    pub fn count(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Integer)
    }

    /// The block's items.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}
