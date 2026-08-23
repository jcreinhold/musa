//! See `ast` module docs; the items parsed in this family.

use super::FnDecl;
use super::FragmentDecl;
use super::LetDecl;
use super::MotifDecl;
use super::PerformanceDecl;
use super::ScoreDecl;
use super::StudioDecl;
use super::child;
use super::children;
use super::token_text;
use super::unquote;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

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
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }

    /// The `tempo` statement in the header: the tempo the piece starts in.
    pub fn tempo(&self) -> Option<TempoStmt> {
        child(&self.0)
    }

    /// Every `tempo` statement in the header, in source order. More than one
    /// is an error the resolver reports; the parser does not editorialise.
    pub fn tempos(&self) -> Vec<TempoStmt> {
        children(&self.0)
    }

    /// The files this piece imports, in source order.
    pub fn imports(&self) -> Vec<ImportStmt> {
        children(&self.0)
    }

    /// The `meter` statement, if present.
    pub fn meter(&self) -> Option<MeterStmt> {
        child(&self.0)
    }

    /// The `key` statement, if present.
    pub fn key(&self) -> Option<KeyStmt> {
        child(&self.0)
    }

    /// The piece's front matter — composer, arranger, subtitle, copyright —
    /// in source order. A role written twice keeps both, and the resolver
    /// decides which wins; the parser does not editorialise.
    pub fn front_matter(&self) -> Vec<FrontMatterStmt> {
        children(&self.0)
    }

    /// All motif declarations.
    pub fn motifs(&self) -> Vec<MotifDecl> {
        children(&self.0)
    }

    /// All fragment declarations.
    pub fn fragments(&self) -> Vec<FragmentDecl> {
        children(&self.0)
    }

    /// Top-level elaboration value bindings.
    pub fn lets(&self) -> Vec<LetDecl> {
        children(&self.0)
    }

    /// Top-level elaboration functions.
    pub fn functions(&self) -> Vec<FnDecl> {
        children(&self.0)
    }

    /// The `score` block, if present.
    pub fn score(&self) -> Option<ScoreDecl> {
        child(&self.0)
    }

    /// The `performance` block, if present.
    pub fn performance(&self) -> Option<PerformanceDecl> {
        child(&self.0)
    }

    /// The `studio` block, if present.
    pub fn studio(&self) -> Option<StudioDecl> {
        child(&self.0)
    }
}

/// `tempo quarter = 72;`
pub struct TempoStmt(SyntaxNode);
wrapper!(TempoStmt, SyntaxKind::TempoStmt);

impl TempoStmt {
    /// The word printed with the marking (`"Allegro vivace"`), if the
    /// statement carries one.
    ///
    /// A marking may be a word, a metronome mark, or both, and the three
    /// combinations are three things a composer writes. The reader asks for
    /// each half separately because neither implies the other.
    pub fn text(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }

    /// Whether the statement states a metronome mark at all.
    ///
    /// `tempo "Andante";` does not, and the difference is the point: a word
    /// is printed and changes no clock.
    pub fn has_metronome(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::Equals)
    }

    /// The beat unit and the speed, as written: `1/4` and `92`.
    ///
    /// The first `Rational` and the first `Integer`, which is what they are —
    /// a ramp's `over` is a later `Rational` and its `to` a later `Integer`,
    /// so both halves are read by position rather than by kind alone.
    pub fn beat(&self) -> Option<String> {
        // Guarded on the `=`: without one, the first `Rational` in the
        // statement is a ramp's `over`, not a beat unit.
        self.has_metronome()
            .then(|| self.token_after(None, &[SyntaxKind::Rational, SyntaxKind::Identifier]))
            .flatten()
    }

    /// The speed in beats per minute, as written.
    pub fn bpm(&self) -> Option<String> {
        self.token_after(Some(SyntaxKind::Equals), &[SyntaxKind::Integer])
    }

    /// The speed a gradual change arrives at (`to 60`), as written.
    pub fn ramp_to(&self) -> Option<String> {
        self.token_after(Some(SyntaxKind::ToKw), &[SyntaxKind::Integer])
    }

    /// How far a gradual change reaches (`over 4/1`), as written.
    pub fn over(&self) -> Option<String> {
        self.token_after(Some(SyntaxKind::OverKw), &[SyntaxKind::Rational])
    }

    /// The first token of one of `wanted` after `keyword`, or after the start
    /// of the statement when `keyword` is `None`.
    fn token_after(&self, keyword: Option<SyntaxKind>, wanted: &[SyntaxKind]) -> Option<String> {
        let mut tokens = self.0.children_with_tokens().filter_map(SyntaxElement::into_token);
        if let Some(keyword) = keyword {
            tokens.find(|token| token.kind() == keyword)?;
        }
        tokens
            .find(|token| wanted.contains(&token.kind()))
            .map(|token| token.text().to_owned())
    }
}

/// `library { motif ...; studio { ... } }` — a file of shared declarations.
///
/// A library is not a piece: it has no score, and the parser refuses one, so
/// "what happens to the music in an imported file" is a question that cannot
/// be asked.
pub struct LibraryDecl(SyntaxNode);
wrapper!(LibraryDecl, SyntaxKind::LibraryDecl);

impl LibraryDecl {
    /// Cast the root node of a document to its library declaration.
    pub fn from_root(node: &SyntaxNode) -> Option<Self> {
        child(node)
    }

    /// The files this library imports, in source order.
    pub fn imports(&self) -> Vec<ImportStmt> {
        children(&self.0)
    }

    /// The motifs it declares.
    pub fn motifs(&self) -> Vec<MotifDecl> {
        children(&self.0)
    }

    /// The fragments it declares.
    pub fn fragments(&self) -> Vec<FragmentDecl> {
        children(&self.0)
    }

    /// Reusable elaboration value bindings.
    pub fn lets(&self) -> Vec<LetDecl> {
        children(&self.0)
    }

    /// Reusable elaboration functions.
    pub fn functions(&self) -> Vec<FnDecl> {
        children(&self.0)
    }

    /// Its `performance` block, if it has one.
    pub fn performance(&self) -> Option<PerformanceDecl> {
        child(&self.0)
    }

    /// Its `studio` block, if it has one.
    pub fn studio(&self) -> Option<StudioDecl> {
        child(&self.0)
    }
}

/// `mod tonal;` — one child of a package's module tree.
pub struct ModDecl(SyntaxNode);
wrapper!(ModDecl, SyntaxKind::ModDecl);

impl ModDecl {
    /// The children a module file declares, in the order it declares them.
    ///
    /// A package's tree is exactly this and nothing else: what files exist
    /// beside the declarations is a separate question, and the point of asking
    /// them separately is that the answers can disagree.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The name of the child.
    ///
    /// Read by position rather than by token kind: `mod list;` names a module,
    /// and the lexer writes `list` as a type keyword wherever the word appears.
    pub fn name(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| crate::parser::MODULE_NAME.contains(&token.kind()))
            .map(|token| token.text().to_owned())
    }
}

/// `import "../library/motifs.musa";` or `import std::core;`
pub struct ImportStmt(SyntaxNode);
wrapper!(ImportStmt, SyntaxKind::ImportStmt);

impl ImportStmt {
    /// The imports written at a document's lexical root, before its piece or
    /// library — what the file's templates may read.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// Whether this is `import syntax …` — the header statement that names
    /// which package reads an adapter region.
    ///
    /// A different statement from an ordinary import, not a modifier on one:
    /// an ordinary import brings a module's declarations into this file, and
    /// this one brings a *reader* into the compiler's expansion phase. The two
    /// are told apart by the word, which is why the word is there.
    pub fn changes_syntax(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::SyntaxKw)
    }

    /// The path as written, without quotes for a relative import.
    pub fn path(&self) -> Option<String> {
        if let Some(text) = token_text(&self.0, SyntaxKind::String) {
            return Some(unquote(&text));
        }
        let names = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            // The alias is a name too, and it is not part of the path.
            .take_while(|token| token.kind() != SyntaxKind::AsKw)
            .filter(|token| crate::parser::MODULE_NAME.contains(&token.kind()))
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>();
        // A namespace and at least one module. How many segments follow is a
        // fact about the package, so nothing here counts them.
        if names.len() < 2 {
            return None;
        }
        Some(names.join("::"))
    }

    /// The name written after `as`, which qualifies what the import binds.
    ///
    /// Absent on almost every import: binding is flat, and an alias is what
    /// resolves a collision between two modules that export the same name.
    pub fn alias(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .skip_while(|token| token.kind() != SyntaxKind::AsKw)
            .find(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_owned())
    }
}

/// `meter 4/4;`
pub struct MeterStmt(SyntaxNode);
wrapper!(MeterStmt, SyntaxKind::MeterStmt);

impl MeterStmt {
    /// The meter fraction text, e.g. `4/4`.
    ///
    /// `None` for `meter none;`, which states a meter without stating a
    /// fraction — see [`MeterStmt::is_unmeasured`].
    pub fn value(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational)
    }

    /// Whether this is `meter none;`: music with no barlines from here.
    ///
    /// The word is an identifier, not the absent case of an option: a meter
    /// that says there are no barlines is a meter, and `None` belongs to
    /// `Option`.
    pub fn is_unmeasured(&self) -> bool {
        token_text(&self.0, SyntaxKind::Identifier).is_some_and(|text| text == "none")
    }
}

/// `key a minor;`
pub struct KeyStmt(SyntaxNode);
wrapper!(KeyStmt, SyntaxKind::KeyStmt);

/// `clef bass;`
pub struct ClefStmt(SyntaxNode);
wrapper!(ClefStmt, SyntaxKind::ClefStmt);

impl ClefStmt {
    /// The clef's name, as written.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// Which line of front matter a [`FrontMatterStmt`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrontMatterRole {
    /// A second line under the title.
    Subtitle,
    /// Who wrote it.
    Composer,
    /// Who arranged it.
    Arranger,
    /// The notice at the foot of the first page.
    Copyright,
}

/// `composer "Name";` — one line of the piece's front matter.
pub struct FrontMatterStmt(SyntaxNode);
wrapper!(FrontMatterStmt, SyntaxKind::FrontMatterStmt);

impl FrontMatterStmt {
    /// Which of the four roles this line fills.
    pub fn role(&self) -> Option<FrontMatterRole> {
        // A table rather than a match: the node holds a `String` and a `;`
        // besides its keyword, so the question is which of exactly four
        // tokens opened it, not what every kind in the language means here.
        const ROLES: [(SyntaxKind, FrontMatterRole); 4] = [
            (SyntaxKind::SubtitleKw, FrontMatterRole::Subtitle),
            (SyntaxKind::ComposerKw, FrontMatterRole::Composer),
            (SyntaxKind::ArrangerKw, FrontMatterRole::Arranger),
            (SyntaxKind::CopyrightKw, FrontMatterRole::Copyright),
        ];
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find_map(|token| {
                ROLES
                    .iter()
                    .find(|(kind, _)| *kind == token.kind())
                    .map(|(_, role)| *role)
            })
    }

    /// The line as written, without its quotes.
    pub fn text(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }
}
