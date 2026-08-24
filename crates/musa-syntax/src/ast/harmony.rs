//! See `ast` module docs; the items parsed in this family.

use super::children;
use super::element_text;
use super::token_text;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// `harmony { ... }`
pub struct HarmonyDecl(SyntaxNode);
wrapper!(HarmonyDecl, SyntaxKind::HarmonyDecl);

impl HarmonyDecl {
    /// The chords in the lane, in source order.
    pub fn chords(&self) -> Vec<HarmonyStmt> {
        children(&self.0)
    }
}

/// `at 1:1 am;`
pub struct HarmonyStmt(SyntaxNode);
wrapper!(HarmonyStmt, SyntaxKind::HarmonyStmt);

impl HarmonyStmt {
    /// Where the chord is written.
    pub fn position(&self) -> Option<Position> {
        children(&self.0).into_iter().next()
    }

    /// The chord symbol, as written.
    pub fn symbol(&self) -> Option<ChordSymbol> {
        children(&self.0).into_iter().next()
    }
}

/// `1:1` — a measure:beat position.
pub struct Position(SyntaxNode);
wrapper!(Position, SyntaxKind::Position);

impl Position {
    /// The measure number, as written.
    pub fn measure(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Integer)
    }

    /// The beat within the measure, as written (`1`, `3/2`).
    pub fn beat(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational).or_else(|| {
            self.0
                .children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(|token| token.kind() == SyntaxKind::Integer)
                .nth(1)
                .map(|token| token.text().to_string())
        })
    }
}

/// `fmaj7` — a chord symbol, as written.
pub struct ChordSymbol(SyntaxNode);
wrapper!(ChordSymbol, SyntaxKind::ChordSymbol);

impl ChordSymbol {
    /// The symbol's significant text, with nothing between its parts.
    pub fn text(&self) -> String {
        self.0
            .children_with_tokens()
            .filter(|part| !part.kind().is_trivia())
            .map(|part| element_text(&part))
            .collect()
    }

    /// Whether the symbol really is one word. `fmaj 7` parses as the same
    /// tokens as `fmaj7` and means nothing musical; the compiler rejects it
    /// rather than quietly reading a chord out of two words.
    pub fn is_one_word(&self) -> bool {
        self.0.text().to_string().trim() == self.text()
    }
}
