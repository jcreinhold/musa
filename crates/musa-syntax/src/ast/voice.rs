//! See `ast` module docs; the items parsed in this family.

use super::AstNode;
use super::ClefStmt;
use super::ExprArg;
use super::ExprArgList;
use super::ImproviseStmt;
use super::KeyStmt;
use super::MeterStmt;
use super::MobileStmt;
use super::TempoStmt;
use super::VoiceItem;
use super::child;
use super::children;
use super::descendant_token_text;
use super::find_token;
use super::span_of;
use super::token_text;
use super::unquote;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode, SyntaxToken};

/// `crescendo to f { ... }` / `diminuendo to p { ... }`
pub struct HairpinStmt(SyntaxNode);
wrapper!(HairpinStmt, SyntaxKind::HairpinStmt);

impl HairpinStmt {
    /// Whether it grows or fades.
    pub fn grows(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::CrescendoKw)
    }

    /// The mark it arrives at, as written.
    pub fn target(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The music it covers, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `senza { ... }`
///
/// An unmeasured stretch with a scope. It is the same two meter changes a
/// composer could write by hand — `meter none;` and the meter back — with the
/// second one impossible to forget, which is the whole of what it adds.
pub struct SenzaStmt(SyntaxNode);
wrapper!(SenzaStmt, SyntaxKind::SenzaStmt);

impl SenzaStmt {
    /// The unmeasured items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// Extract the items of a voice-like block (voice body, motif body,
/// transpose block, repeat block).
pub(crate) fn voice_items(node: &SyntaxNode) -> Vec<VoiceItem> {
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
        } else if kind == SyntaxKind::InScaleStmt {
            InScaleStmt::cast(child).map(VoiceItem::InScale)
        } else if kind == SyntaxKind::StackStmt {
            StackStmt::cast(child).map(VoiceItem::Stack)
        } else if kind == SyntaxKind::TransposeStmt {
            TransposeStmt::cast(child).map(VoiceItem::Transpose)
        } else if kind == SyntaxKind::RepeatStmt {
            RepeatStmt::cast(child).map(VoiceItem::Repeat)
        } else if kind == SyntaxKind::BarStmt {
            BarStmt::cast(child).map(VoiceItem::Bar)
        } else if kind == SyntaxKind::AssertStmt {
            AssertStmt::cast(child).map(VoiceItem::Assert)
        } else if kind == SyntaxKind::SenzaStmt {
            SenzaStmt::cast(child).map(VoiceItem::Senza)
        } else if kind == SyntaxKind::EndingStmt {
            EndingStmt::cast(child).map(VoiceItem::Ending)
        } else if kind == SyntaxKind::SlurStmt {
            SlurStmt::cast(child).map(VoiceItem::Slur)
        } else if kind == SyntaxKind::DynamicStmt {
            DynamicStmt::cast(child).map(VoiceItem::Dynamic)
        } else if kind == SyntaxKind::TupletStmt {
            TupletStmt::cast(child).map(VoiceItem::Tuplet)
        } else if kind == SyntaxKind::StretchStmt {
            StretchStmt::cast(child).map(VoiceItem::Stretch)
        } else if kind == SyntaxKind::RetrogradeStmt {
            RetrogradeStmt::cast(child).map(VoiceItem::Retrograde)
        } else if kind == SyntaxKind::InvertStmt {
            InvertStmt::cast(child).map(VoiceItem::Invert)
        } else if kind == SyntaxKind::PhraseStmt {
            PhraseStmt::cast(child).map(VoiceItem::Phrase)
        } else if kind == SyntaxKind::MarkStmt {
            MarkStmt::cast(child).map(VoiceItem::Mark)
        } else if kind == SyntaxKind::GraceStmt {
            GraceStmt::cast(child).map(VoiceItem::Grace)
        } else if kind == SyntaxKind::HairpinStmt {
            HairpinStmt::cast(child).map(VoiceItem::Hairpin)
        } else if kind == SyntaxKind::TempoStmt {
            TempoStmt::cast(child).map(VoiceItem::Tempo)
        } else if kind == SyntaxKind::MeterStmt {
            MeterStmt::cast(child).map(VoiceItem::Meter)
        } else if kind == SyntaxKind::KeyStmt {
            KeyStmt::cast(child).map(VoiceItem::Key)
        } else if kind == SyntaxKind::ClefStmt {
            ClefStmt::cast(child).map(VoiceItem::Clef)
        } else if kind == SyntaxKind::MobileStmt {
            MobileStmt::cast(child).map(VoiceItem::Mobile)
        } else if kind == SyntaxKind::ImproviseStmt {
            ImproviseStmt::cast(child).map(VoiceItem::Improvise)
        } else {
            None
        };
        items.extend(item);
    }
    items
}

/// The articulation names trailing a note or chord's duration.
pub(crate) fn articulation_names(node: &SyntaxNode) -> Vec<String> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::ArticulationList)
        .into_iter()
        .flat_map(|list| {
            list.children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter_map(|token| articulation_name(&token))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// One articulation, by whichever of its two spellings was written.
///
/// A mark is the word: `a5/4>` and `a5/4 accent` are the same articulation,
/// named here rather than downstream, so everything that reads articulations
/// keeps reading names and never learns there was a second spelling.
pub(crate) fn articulation_name(token: &SyntaxToken) -> Option<String> {
    let kind = token.kind();
    if kind == SyntaxKind::Identifier {
        return Some(token.text().to_string());
    }
    if kind == SyntaxKind::Greater {
        return Some("accent".to_owned());
    }
    if kind == SyntaxKind::Caret {
        return Some("marcato".to_owned());
    }
    None
}

/// `1/2`, `3/8`, `1`, `/4`, `/4.`, or a duration parameter's name — how long
/// one note, rest or chord lasts.
pub struct Duration(SyntaxNode);
wrapper!(Duration, SyntaxKind::Duration);

impl Duration {
    /// The duration inside a note, rest, chord or improvised frame.
    pub fn of(statement: &SyntaxNode) -> Option<Self> {
        child(statement)
    }

    /// The notated duration as a fraction of a whole note, with the short
    /// form spelled out: `/4` reads `1/4` and `/4.` reads `3/8`.
    ///
    /// One duration has one spelling on purpose. `c4/4.` and `c4 3/8` are the
    /// same note, and a reader that could tell them apart would be a reader
    /// that could disagree with itself about how long the note lasts.
    ///
    /// `None` when the duration is a parameter reference, which only the
    /// caller holding the binding can resolve.
    pub fn value(&self) -> Option<String> {
        spell(&mut self.parts())
    }

    /// The name of the duration parameter this stands for, when it is one
    /// (`root 1/8` binds `root`; `use ostinato(long)` binds a duration).
    ///
    /// A statement's pitch reference is a bare identifier too, so the name is
    /// read from inside the duration and nowhere else.
    pub fn parameter(&self) -> Option<String> {
        self.parts()
            .next()
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .map(|token| token.text().to_string())
    }

    /// The longest this may be held, when the statement gives the performer a
    /// range (`g4/4 to 2/1`), spelled the same way.
    pub fn held_to(&self) -> Option<String> {
        let mut after = self
            .parts()
            .skip_while(|token| token.kind() != SyntaxKind::ToKw)
            .skip(1);
        spell(&mut after)
    }

    fn parts(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| !token.kind().is_trivia())
    }
}

/// Read one duration value off the front of a token run.
pub(crate) fn spell(tokens: &mut impl Iterator<Item = SyntaxToken>) -> Option<String> {
    let first = tokens.next()?;
    if matches!(first.kind(), SyntaxKind::Rational | SyntaxKind::Integer) {
        return Some(first.text().to_string());
    }
    if first.kind() != SyntaxKind::Slash {
        // An identifier: a duration parameter, resolved somewhere that knows
        // what it is bound to.
        return None;
    }
    let value = tokens.next().filter(|token| token.kind() == SyntaxKind::Integer)?;
    let dots = tokens.take_while(|token| token.kind() == SyntaxKind::Dot).count();
    dotted(value.text().parse().ok()?, dots)
}

/// `/N` with `d` augmentation dots, as a fraction: `(1/N)·(2 − 2⁻ᵈ)`.
///
/// Eight dots is the ceiling because the ninth would be a note held for
/// 511/512 of the value of one that is already unplayable; past it the
/// duration is not a duration and the caller says so.
pub(crate) fn dotted(value: i64, dots: usize) -> Option<String> {
    if value <= 0 || dots > 8 {
        return None;
    }
    let scale = 1_i64.checked_shl(u32::try_from(dots).ok()?)?;
    let numerator = scale.checked_mul(2)?.checked_sub(1)?;
    let denominator = value.checked_mul(scale)?;
    let divisor = gcd(numerator, denominator);
    let numerator = numerator.checked_div(divisor)?;
    let denominator = denominator.checked_div(divisor)?;
    // A whole note is written `1`, not `1/1`: the long form spells it that
    // way, and the two forms must not disagree about one duration.
    Some(if denominator == 1 {
        numerator.to_string()
    } else {
        format!("{numerator}/{denominator}")
    })
}

pub(crate) fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let rest = a.checked_rem(b).unwrap_or(0);
        a = b;
        b = rest;
    }
    a.max(1)
}

/// The duration written after `to`, when the statement gives the performer a
/// range (`g4/4 to 2/1`).
pub(crate) fn held_to(node: &SyntaxNode) -> Option<String> {
    Duration::of(node).and_then(|duration| duration.held_to())
}

/// How long a note, rest, chord or improvised frame lasts, as written.
pub(crate) fn duration_text(node: &SyntaxNode) -> Option<String> {
    Duration::of(node).and_then(|duration| duration.value())
}

/// Whether a statement carries the postfix tie mark.
pub(crate) fn has_tie(node: &SyntaxNode) -> bool {
    find_token(node, SyntaxKind::Tilde).is_some()
}

/// `<pitch-or-ref> <duration>;`
pub struct NoteStmt(SyntaxNode);
wrapper!(NoteStmt, SyntaxKind::NoteStmt);

impl NoteStmt {
    /// The written pitch (`g#4`) or pitch reference (`root`).
    pub fn pitch(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral).or_else(|| token_text(&self.0, SyntaxKind::Identifier))
    }

    /// The expression which computes this note's written pitch.
    pub fn pitch_expr(&self) -> Option<SyntaxNode> {
        self.0.children().find(|child| {
            matches!(
                child.kind(),
                SyntaxKind::NameExpr
                    | SyntaxKind::LiteralExpr
                    | SyntaxKind::ParenExpr
                    | SyntaxKind::ApplyExpr
                    | SyntaxKind::PitchExpr
                    | SyntaxKind::StepExpr
            )
        })
    }

    /// The duration text (`1/2`, `3/8`, `1`).
    pub fn duration(&self) -> Option<String> {
        duration_text(&self.0)
    }

    /// The articulation names written after the duration, in source order.
    pub fn articulations(&self) -> Vec<String> {
        articulation_names(&self.0)
    }

    /// The longest this note may be held, when the performer was given a
    /// range (`g4/4 to 2/1`).
    pub fn held_to(&self) -> Option<String> {
        held_to(&self.0)
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
        duration_text(&self.0)
    }

    /// The longest this rest may be held.
    pub fn held_to(&self) -> Option<String> {
        held_to(&self.0)
    }
}

/// `[a3 c4 e4]/2`
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
        descendant_token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The call arguments, in order (pitch literals, parameter references,
    /// or durations).
    pub fn args(&self) -> Vec<String> {
        let Some(expression) = self.0.children().find(|child| child.kind() != SyntaxKind::WithClause) else {
            return Vec::new();
        };
        expression
            .descendants_with_tokens()
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

    /// The overrides specializing this occurrence, in source order. Empty
    /// for an ordinary call — a `with` clause is the only thing that makes
    /// one occurrence differ from its siblings (roadmap §9).
    pub fn overrides(&self) -> Vec<OverrideStmt> {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::WithClause)
            .flat_map(|clause| clause.children().filter_map(OverrideStmt::cast).collect::<Vec<_>>())
            .collect()
    }
}

/// One override inside a `with { ... }` clause: `note 2 = d5;`.
pub struct OverrideStmt(SyntaxNode);
wrapper!(OverrideStmt, SyntaxKind::OverrideStmt);

impl OverrideStmt {
    /// Which note of the occurrence this respells, counting from one.
    pub fn position(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Integer)
    }

    /// The pitch it is respelled to.
    pub fn pitch(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral)
    }
}

/// `stretch 3/2 { ... }`
pub struct StretchStmt(SyntaxNode);
wrapper!(StretchStmt, SyntaxKind::StretchStmt);

impl StretchStmt {
    /// The factor text (`3/2`, `2`), unreduced.
    pub fn factor(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Rational).or_else(|| token_text(&self.0, SyntaxKind::Integer))
    }

    /// The block's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `retrograde { ... }`
pub struct RetrogradeStmt(SyntaxNode);
wrapper!(RetrogradeStmt, SyntaxKind::RetrogradeStmt);

impl RetrogradeStmt {
    /// The block's items, in source order — the order they are *written*,
    /// not the order they sound.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `invert around c5 { ... }`
pub struct InvertStmt(SyntaxNode);
wrapper!(InvertStmt, SyntaxKind::InvertStmt);

impl InvertStmt {
    /// The axis pitch the block is mirrored about.
    pub fn axis(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral)
    }

    /// The block's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
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

/// `in scale <expr> { ... }` — the enclosed music read in a scale.
pub struct InScaleStmt(SyntaxNode);
wrapper!(InScaleStmt, SyntaxKind::InScaleStmt);

impl InScaleStmt {
    /// The expression that computes the scale.
    pub fn scale_expr(&self) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() != SyntaxKind::Block)
    }

    /// The items read in that scale.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `stack <pitch> <type><duration>` — a chord sounded in close position.
pub struct StackStmt(SyntaxNode);
wrapper!(StackStmt, SyntaxKind::StackStmt);

impl StackStmt {
    /// The written root, when one was written. A pitch class parses here and
    /// answers `None`, which is what the register diagnostic reads.
    pub fn root(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral)
    }

    /// The chord type, as written.
    pub fn chord_type(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Whether a pitch class was written where a pitch belongs.
    pub fn root_is_class(&self) -> bool {
        self.0.children().any(|child| child.kind() == SyntaxKind::PitchClass)
    }

    /// The articulation names trailing the duration.
    pub fn articulations(&self) -> Vec<String> {
        articulation_names(&self.0)
    }

    /// Whether the chord is tied into what follows.
    pub fn tied(&self) -> bool {
        has_tie(&self.0)
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

/// `phrase "A" { ... }`
pub struct PhraseStmt(SyntaxNode);
wrapper!(PhraseStmt, SyntaxKind::PhraseStmt);

impl PhraseStmt {
    /// The phrase's name, without its quotes.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }

    /// The music the phrase covers, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `mark breath;` / `mark text "dolce";` / `mark pedal { ... }`
///
/// One node for every mark that is not written on a note. Which of the three
/// written forms is legal for a given name is the vocabulary's answer, not the
/// grammar's, so all three parse here and the compiler reports the mismatch.
pub struct MarkStmt(SyntaxNode);
wrapper!(MarkStmt, SyntaxKind::MarkStmt);

impl MarkStmt {
    /// The mark's name, as written.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The quoted argument, without its quotes.
    pub fn text(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::String).map(|text| unquote(&text))
    }

    /// The numeric argument with its sign, as written.
    ///
    /// A string, like [`SettingStmt::value`] and for the same reason: the
    /// minus is a token of its own, and a reader wants one value.
    pub fn number(&self) -> Option<String> {
        let magnitude = token_text(&self.0, SyntaxKind::Integer)?;
        Some(match token_text(&self.0, SyntaxKind::Minus) {
            Some(_) => format!("-{magnitude}"),
            None => magnitude,
        })
    }

    /// Whether a block was written, which is how a span is told from a point
    /// before the vocabulary is consulted.
    pub fn has_block(&self) -> bool {
        self.0.children().any(|child| child.kind() == SyntaxKind::Block)
    }

    /// The music inside the block, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `grace { c5 d5 }`
pub struct GraceStmt(SyntaxNode);
wrapper!(GraceStmt, SyntaxKind::GraceStmt);

impl GraceStmt {
    /// The grace notes, in written order.
    ///
    /// The order is the value: `grace { c5 d5 }` and `grace { d5 c5 }`
    /// are different music, and nothing downstream can recover the difference
    /// once this sequence is lost.
    pub fn notes(&self) -> Vec<GraceNote> {
        self.0.children().filter_map(GraceNote::cast).collect()
    }
}

/// One pitch inside a `grace` block.
pub struct GraceNote(SyntaxNode);
wrapper!(GraceNote, SyntaxKind::GraceNote);

impl GraceNote {
    /// The written pitch (`g#4`) or pitch reference (`root`).
    pub fn pitch(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::PitchLiteral).or_else(|| token_text(&self.0, SyntaxKind::Identifier))
    }

    /// The articulation names written after the pitch, in source order.
    pub fn articulations(&self) -> Vec<String> {
        articulation_names(&self.0)
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

    /// Where the count is written, as start and end byte offsets.
    ///
    /// A diagnostic about how many passes there are has to underline the
    /// number, not the whole block.
    pub fn count_span(&self) -> Option<(u32, u32)> {
        find_token(&self.0, SyntaxKind::Integer).map(|token| span_of(&token))
    }

    /// The largest number of passes, when the count is a range
    /// (`repeat 4 to 16`). `None` for the exact form.
    pub fn most(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .filter(|token| token.kind() == SyntaxKind::Integer)
            .nth(1)
            .map(|token| token.text().to_string())
    }

    /// Where the whole count is written — both numbers and the `to` between
    /// them, when there is one.
    pub fn range_span(&self) -> Option<(u32, u32)> {
        let mut integers = self
            .0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .filter(|token| token.kind() == SyntaxKind::Integer);
        let first = span_of(&integers.next()?);
        Some(integers.next().map_or(first, |last| (first.0, span_of(&last).1)))
    }

    /// The block's items.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `bar { c5/4 e5/4 g5/4 e5/4 }`
///
/// A measure, written down. The braces are an assertion — the contents are
/// checked against the prevailing meter and then erased — and a name on the
/// front makes the same measure playable again from anywhere below it.
pub struct BarStmt(SyntaxNode);
wrapper!(BarStmt, SyntaxKind::BarStmt);

impl BarStmt {
    /// The name this bar binds, when it has one.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The bar's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }

    /// Where the bar's contents end: after the last thing written in it and
    /// before the whitespace in front of the closing `}`.
    ///
    /// Where something added to the bar goes. Inserting at the `}` instead
    /// would put the new statement after the line break that closes the block,
    /// which is a different-looking edit on a one-line bar and a
    /// wrong-looking one on a bar that broke.
    pub fn content_end(&self) -> Option<u32> {
        content_end(&self.0)
    }
}

/// Where a braced body's contents end: after the last thing written in it and
/// before the whitespace in front of the closing `}`.
pub(crate) fn content_end(node: &SyntaxNode) -> Option<u32> {
    let mut end = None;
    for element in node.descendants_with_tokens() {
        let SyntaxElement::Token(token) = element else { continue };
        if token.kind().is_trivia() || token.kind() == SyntaxKind::RBrace {
            continue;
        }
        end = Some(u32::from(token.text_range().end()));
    }
    end
}

/// `assert pitches_in(scale c major) { c5/4 e5/4 g5/2 }`
///
/// A claim about the passage inside it. The braces hold ordinary music and
/// contribute nothing to it: an assertion that holds returns exactly what was
/// written, and one that does not is a diagnostic. Which claims exist is the
/// compiler's registry, so everything here is shape — a name, its arguments,
/// and a body.
pub struct AssertStmt(SyntaxNode);
wrapper!(AssertStmt, SyntaxKind::AssertStmt);

impl AssertStmt {
    /// The claim's name, as written.
    pub fn claim(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Where the claim's name is written, as start and end byte offsets.
    pub fn claim_span(&self) -> Option<(u32, u32)> {
        find_token(&self.0, SyntaxKind::Identifier).map(|token| span_of(&token))
    }

    /// The claim's arguments, in source order.
    pub fn args(&self) -> Vec<ExprArg> {
        self.0
            .children()
            .find_map(ExprArgList::cast)
            .map_or_else(Vec::new, |list| children(list.syntax()))
    }

    /// The asserted music, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }

    /// Where the asserted music ends, for a fix that adds to it. The same
    /// place a bar's is, because the edit is the same edit.
    pub fn content_end(&self) -> Option<u32> {
        content_end(&self.0)
    }
}

/// `ending 1 { ... }`
///
/// What a repeat plays on one of its passes and prints once, under a volta
/// bracket. Legal only directly inside a `repeat`; the compiler is what says
/// so.
pub struct EndingStmt(SyntaxNode);
wrapper!(EndingStmt, SyntaxKind::EndingStmt);

impl EndingStmt {
    /// Which pass this ending belongs to, as written.
    pub fn number(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Integer)
    }

    /// Where that number is written, as start and end byte offsets.
    pub fn number_span(&self) -> Option<(u32, u32)> {
        find_token(&self.0, SyntaxKind::Integer).map(|token| span_of(&token))
    }

    /// The ending's items, in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}

/// `music { ... }`, a contextual notation-first music value.
pub struct MusicExpr(SyntaxNode);
wrapper!(MusicExpr, SyntaxKind::MusicExpr);

impl MusicExpr {
    /// Its notation statements in source order.
    pub fn items(&self) -> Vec<VoiceItem> {
        voice_items(&self.0)
    }
}
