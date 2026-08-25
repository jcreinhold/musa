//! See `ast` module docs; the items parsed in this family.

use super::AssertStmt;
use super::BarStmt;
use super::ChordStmt;
use super::ClefStmt;
use super::DynamicStmt;
use super::EndingStmt;
use super::GraceStmt;
use super::HairpinStmt;
use super::HarmonyDecl;
use super::InScaleStmt;
use super::InvertStmt;
use super::KeyStmt;
use super::MarkStmt;
use super::MeterStmt;
use super::NoteStmt;
use super::Param;
use super::PhraseStmt;
use super::Position;
use super::RepeatStmt;
use super::RestStmt;
use super::RetrogradeStmt;
use super::SenzaStmt;
use super::SlurStmt;
use super::StackStmt;
use super::StretchStmt;
use super::TempoStmt;
use super::TransposeStmt;
use super::TupletStmt;
use super::UseStmt;
use super::child;
use super::children;
use super::duration_text;
use super::token_text;
use super::unquote;
use super::voice_items;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode, SyntaxToken};

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
        // Descendants, not children: a parameter's type is a `TypeName` node,
        // the same one every other written type is, so the token is one level
        // down while the reading order is unchanged.
        let mut tokens = self
            .0
            .descendants_with_tokens()
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
            // A default is a spelling the parser rejects, but the tokens are
            // still in the tree so the error can span them. Step over them
            // here: a `Param` describes a parameter the language has, and a
            // parameter has no default.
            if tokens.peek().is_some_and(|next| next.kind() == SyntaxKind::Equals) {
                drop(tokens.next());
                drop(tokens.next());
            }
            params.push(Param { name, kind });
        }
        params
    }

    /// The motif's body items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
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

    /// The form markers written in the score, in source order.
    pub fn sections(&self) -> Vec<SectionStmt> {
        children(&self.0)
    }

    /// The harmony lanes, in source order. More than one is a diagnostic,
    /// not a parse error: the parser records what was written.
    pub fn harmonies(&self) -> Vec<HarmonyDecl> {
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

    /// The performance profile named for this part, if declared.
    pub fn profile(&self) -> Option<ProfileStmt> {
        child(&self.0)
    }

    /// The concise instrument/profile choice, if written.
    pub fn sound(&self) -> Option<SoundStmt> {
        child(&self.0)
    }

    /// The part's own meter, if it declares one — polymeter.
    ///
    /// Only a `meter` written directly in the part block: a `meter` inside a
    /// voice is the *piece's* meter from there, and `child`
    /// reads children rather than descendants precisely so the two do not
    /// blur into each other.
    pub fn meter(&self) -> Option<MeterStmt> {
        child(&self.0)
    }

    /// The part's own tempo, if it declares one — polytempo.
    pub fn tempo(&self) -> Option<TempoStmt> {
        child(&self.0)
    }
}

/// `profile violin;` inside a part.
pub struct ProfileStmt(SyntaxNode);
wrapper!(ProfileStmt, SyntaxKind::ProfileStmt);

impl ProfileStmt {
    /// The named profile.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// `sound solo_strings using lyrical;` inside a part.
pub struct SoundStmt(SyntaxNode);
wrapper!(SoundStmt, SyntaxKind::SoundStmt);

impl SoundStmt {
    fn names(&self) -> (String, String) {
        let mut instrument = String::new();
        let mut profile = String::new();
        let mut reading_profile = false;
        for token in self.0.children_with_tokens().filter_map(SyntaxElement::into_token) {
            if token.kind() == SyntaxKind::Identifier && token.text() == "using" {
                reading_profile = true;
            } else if matches!(
                token.kind(),
                SyntaxKind::Identifier | SyntaxKind::PerformanceKw | SyntaxKind::PitchKw | SyntaxKind::ScaleKw
            ) {
                if token.text() == "sound" && !reading_profile && instrument.is_empty() {
                    continue;
                }
                let name = if reading_profile { &mut profile } else { &mut instrument };
                if !name.is_empty() {
                    name.push_str("::");
                }
                name.push_str(token.text());
            }
        }
        (instrument, profile)
    }

    /// The chosen instrument name or qualified path.
    pub fn instrument(&self) -> Option<String> {
        let (instrument, _) = self.names();
        (!instrument.is_empty()).then_some(instrument)
    }

    /// The first token of the chosen instrument path.
    pub fn instrument_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .skip_while(|token| token.kind() != SyntaxKind::Identifier || token.text() != "sound")
            .skip(1)
            .find(|token| !token.kind().is_trivia())
    }

    /// The chosen performance profile name or qualified path.
    pub fn profile(&self) -> Option<String> {
        let (_, profile) = self.names();
        (!profile.is_empty()).then_some(profile)
    }

    /// The first token of the chosen profile path.
    pub fn profile_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .skip_while(|token| token.kind() != SyntaxKind::Identifier || token.text() != "using")
            .skip(1)
            .find(|token| !token.kind().is_trivia())
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

/// One item in a voice or motif body.
#[derive(Clone)]
pub enum VoiceItem {
    /// `<pitch-or-ref> <duration>;`
    Note(NoteStmt),
    /// `rest <duration>;`
    Rest(RestStmt),
    /// `[<pitch>…]<duration>`
    Chord(ChordStmt),
    /// `use name(...);`
    Use(UseStmt),
    /// `transpose ... { ... }`
    Transpose(TransposeStmt),
    /// `in scale <expr> { ... }`
    InScale(InScaleStmt),
    /// `stack c4 major7/2`
    Stack(StackStmt),
    /// `repeat n { ... }`
    Repeat(RepeatStmt),
    /// `bar { ... }` / `bar head { ... }`
    Bar(BarStmt),
    /// `assert pitches_in(scale c major) { ... }`
    Assert(AssertStmt),
    /// `senza { ... }` — unmeasured, and measured again after.
    Senza(SenzaStmt),
    /// `ending 1 { ... }`
    Ending(EndingStmt),
    /// `slur { ... }`
    Slur(SlurStmt),
    /// `dynamic p;`
    Dynamic(DynamicStmt),
    /// `tuplet 3/2 { ... }`
    Tuplet(TupletStmt),
    /// `stretch 3/2 { ... }`
    Stretch(StretchStmt),
    /// `retrograde { ... }`
    Retrograde(RetrogradeStmt),
    /// `invert around c5 { ... }`
    Invert(InvertStmt),
    /// `phrase "A" { ... }`
    Phrase(PhraseStmt),
    /// `mark breath;` / `mark pedal { ... }`
    Mark(MarkStmt),
    /// `grace { c5 d5 }` — the notes crushed before the next one.
    Grace(GraceStmt),
    /// `crescendo to f { ... }`
    Hairpin(HairpinStmt),
    /// `tempo 1/4 = 92;` — written where the music reaches it.
    Tempo(TempoStmt),
    /// `meter 3/4;` — likewise.
    Meter(MeterStmt),
    /// `key d minor;` — likewise.
    Key(KeyStmt),
    /// `clef bass;` — likewise.
    Clef(ClefStmt),
    /// `mobile { a; b; c; }`
    Mobile(MobileStmt),
    /// `improvise 8/1 over "Dm7 | G7";`
    Improvise(ImproviseStmt),
}

/// `fragment a { c5/4 e5/4 }`
pub struct FragmentDecl(SyntaxNode);
wrapper!(FragmentDecl, SyntaxKind::FragmentDecl);

impl FragmentDecl {
    /// The fragment name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The items in its body.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `mobile { a; b; c; }`
pub struct MobileStmt(SyntaxNode);
wrapper!(MobileStmt, SyntaxKind::MobileStmt);

impl MobileStmt {
    /// The fragment names it arranges, in written order.
    pub fn fragments(&self) -> Vec<String> {
        self.fragment_tokens()
            .iter()
            .map(|token| token.text().to_string())
            .collect()
    }

    /// The fragment names' own tokens, in written order — the spans an
    /// editor rewrites without touching the brackets around them.
    pub fn fragment_tokens(&self) -> Vec<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .collect()
    }
}

/// `improvise 8/1 over "Dm7 | G7";`
pub struct ImproviseStmt(SyntaxNode);
wrapper!(ImproviseStmt, SyntaxKind::ImproviseStmt);

impl ImproviseStmt {
    /// How long the frame lasts.
    pub fn duration(&self) -> Option<String> {
        duration_text(&self.0)
    }

    /// The changes to play over, if any were written.
    pub fn over(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }
}

/// `section "Exposition" at 1:1;`
pub struct SectionStmt(SyntaxNode);
wrapper!(SectionStmt, SyntaxKind::SectionStmt);

impl SectionStmt {
    /// The section's name, without its quotes.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }

    /// Where it is marked.
    pub fn position(&self) -> Option<Position> {
        children(&self.0).into_iter().next()
    }
}
