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

    /// The `performance` block, if present.
    pub fn performance(&self) -> Option<PerformanceDecl> {
        child(&self.0)
    }

    /// The `studio` block, if present.
    pub fn studio(&self) -> Option<StudioDecl> {
        child(&self.0)
    }
}

/// `performance { profile violin { ... } }` — how marks are realized.
pub struct PerformanceDecl(SyntaxNode);
wrapper!(PerformanceDecl, SyntaxKind::PerformanceDecl);

impl PerformanceDecl {
    /// The declared profiles, in source order.
    pub fn profiles(&self) -> Vec<ProfileDecl> {
        children(&self.0)
    }
}

/// `profile violin { articulation staccato { ... } dynamic p { ... } }`
pub struct ProfileDecl(SyntaxNode);
wrapper!(ProfileDecl, SyntaxKind::ProfileDecl);

impl ProfileDecl {
    /// The profile name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The articulation rules, in source order.
    pub fn articulations(&self) -> Vec<ArticulationRule> {
        children(&self.0)
    }

    /// The dynamic rules, in source order.
    pub fn dynamics(&self) -> Vec<DynamicRule> {
        children(&self.0)
    }
}

/// One rule's head name and its settings; `articulation` and `dynamic` rules
/// share the shape, so they share the accessors.
macro_rules! rule_wrapper {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <name> { <setting>* }` inside a profile.")]
        pub struct $name(SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            #[doc = concat!("The ", $what, " this rule realizes.")]
            pub fn mark(&self) -> Option<String> {
                token_text(&self.0, SyntaxKind::Identifier)
            }

            /// The rule's settings, in source order.
            pub fn settings(&self) -> Vec<SettingStmt> {
                children(&self.0)
            }
        }
    };
}

rule_wrapper!(ArticulationRule, SyntaxKind::ArticulationRule, "articulation");
rule_wrapper!(DynamicRule, SyntaxKind::DynamicRule, "dynamic");

/// `gate = 0.55;`, `attack = 8 ms;`
pub struct SettingStmt(SyntaxNode);
wrapper!(SettingStmt, SyntaxKind::SettingStmt);

impl SettingStmt {
    /// The setting name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The written number, without its unit.
    pub fn value(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Float).or_else(|| token_text(&self.0, SyntaxKind::Integer))
    }

    /// The unit written after the number (`ms`, `s`), if any.
    pub fn unit(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::UnitMs).or_else(|| token_text(&self.0, SyntaxKind::UnitS))
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

    /// The performance profile named for this part, if declared.
    pub fn profile(&self) -> Option<ProfileStmt> {
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

// --- Studio (roadmap §7.1) --------------------------------------------------

/// One item of a `studio` block. The variants are the block's whole
/// vocabulary, so a `match` over this type is a match over the studio
/// language.
pub enum StudioItem {
    /// `patch name { ... }`
    Patch(PatchDecl),
    /// `bus name { ... }`
    Bus(BusDecl),
    /// `name = chain;`
    Signal(SignalBinding),
    /// `modulate a -> b.c.d;`
    Modulate(ModulateStmt),
    /// `assign part -> patch;`
    Assign(AssignStmt),
    /// `route a -> b;`
    Route(RouteStmt),
    /// `send a -> b at -18 dB;`
    Send(SendStmt),
}

/// `studio { ... }` — the sound side of a piece.
pub struct StudioDecl(SyntaxNode);
wrapper!(StudioDecl, SyntaxKind::StudioDecl);

impl StudioDecl {
    /// The block's items, in source order. Order matters: a `route` may name
    /// a bus declared after it, but diagnostics read better in source order.
    pub fn items(&self) -> Vec<StudioItem> {
        // Each `cast` is its own kind check, so the dispatch is a chain of
        // attempts rather than a match: a node that is none of these — an
        // error node, say — is simply not an item.
        self.0
            .children()
            .filter_map(|node| {
                PatchDecl::cast(node.clone())
                    .map(StudioItem::Patch)
                    .or_else(|| BusDecl::cast(node.clone()).map(StudioItem::Bus))
                    .or_else(|| SignalBinding::cast(node.clone()).map(StudioItem::Signal))
                    .or_else(|| ModulateStmt::cast(node.clone()).map(StudioItem::Modulate))
                    .or_else(|| AssignStmt::cast(node.clone()).map(StudioItem::Assign))
                    .or_else(|| RouteStmt::cast(node.clone()).map(StudioItem::Route))
                    .or_else(|| SendStmt::cast(node.clone()).map(StudioItem::Send))
            })
            .collect()
    }
}

/// A patch or a bus: a named container of signal bindings and chains. The two
/// differ in what they may contain semantically, not syntactically, so one
/// macro gives both the same accessors.
macro_rules! chain_container {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <name> { <binding-or-chain>* }`")]
        pub struct $name(SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            #[doc = concat!("The ", $what, "'s name.")]
            pub fn name(&self) -> Option<String> {
                token_text(&self.0, SyntaxKind::Identifier)
            }

            /// The named signals declared inside, in source order.
            pub fn signals(&self) -> Vec<SignalBinding> {
                children(&self.0)
            }

            /// The unnamed chains, in source order.
            pub fn chains(&self) -> Vec<ChainStmt> {
                children(&self.0)
            }
        }
    };
}

chain_container!(PatchDecl, SyntaxKind::PatchDecl, "patch");
chain_container!(BusDecl, SyntaxKind::BusDecl, "bus");

/// `carrier = oscillator(sine);`
pub struct SignalBinding(SyntaxNode);
wrapper!(SignalBinding, SyntaxKind::SignalBinding);

impl SignalBinding {
    /// The signal's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The chain it is bound to.
    pub fn chain(&self) -> Option<SignalChain> {
        child(&self.0)
    }
}

/// `mix(a, b) |> lowpass(cutoff: 1400 Hz) |> output;`
pub struct ChainStmt(SyntaxNode);
wrapper!(ChainStmt, SyntaxKind::ChainStmt);

impl ChainStmt {
    /// The chain.
    pub fn chain(&self) -> Option<SignalChain> {
        child(&self.0)
    }
}

/// A `|>`-separated sequence of stages.
pub struct SignalChain(SyntaxNode);
wrapper!(SignalChain, SyntaxKind::SignalChain);

impl SignalChain {
    /// The stages, left to right.
    pub fn stages(&self) -> Vec<SignalStage> {
        self.0
            .children()
            .filter_map(|node| SignalStage::cast_any(&node))
            .collect()
    }
}

/// One stage of a chain, or one argument value.
pub enum SignalStage {
    /// `lowpass(cutoff: 1400 Hz)`
    Call(CallExpr),
    /// `carrier`, `output`, `sine`
    Name(NameRef),
    /// `-15 dB`, `0.65`
    Literal(ValueLiteral),
}

impl SignalStage {
    fn cast_any(node: &SyntaxNode) -> Option<Self> {
        CallExpr::cast(node.clone())
            .map(Self::Call)
            .or_else(|| NameRef::cast(node.clone()).map(Self::Name))
            .or_else(|| ValueLiteral::cast(node.clone()).map(Self::Literal))
    }

    /// The span of whichever form this is, for diagnostics.
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Call(call) => call.syntax(),
            Self::Name(name) => name.syntax(),
            Self::Literal(literal) => literal.syntax(),
        }
    }
}

/// `lowpass(cutoff: 1400 Hz, q: 0.7)`
pub struct CallExpr(SyntaxNode);
wrapper!(CallExpr, SyntaxKind::CallExpr);

impl CallExpr {
    /// The processor being constructed.
    pub fn callee(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Its arguments, in source order.
    pub fn args(&self) -> Vec<Arg> {
        child::<ArgList>(&self.0)
            .map(|list| children(&list.0))
            .unwrap_or_default()
    }
}

/// The parenthesized arguments of a call.
pub struct ArgList(SyntaxNode);
wrapper!(ArgList, SyntaxKind::ArgList);

/// `cutoff: 1400 Hz` or a positional `sine`.
pub struct Arg(SyntaxNode);
wrapper!(Arg, SyntaxKind::Arg);

impl Arg {
    /// The argument's name, if it was written with one.
    ///
    /// The name is the identifier *before* the colon; a positional `sine`
    /// has no colon and so no name, which is what tells the two apart.
    pub fn name(&self) -> Option<String> {
        find_token(&self.0, SyntaxKind::Colon)?;
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The argument's value.
    pub fn value(&self) -> Option<SignalStage> {
        self.0.children().find_map(|node| SignalStage::cast_any(&node))
    }
}

/// `1400 Hz`, `-15 dB`, `0.65`
pub struct ValueLiteral(SyntaxNode);
wrapper!(ValueLiteral, SyntaxKind::ValueLiteral);

impl ValueLiteral {
    /// The written number including its sign, without the unit.
    pub fn number(&self) -> Option<String> {
        let digits = token_text(&self.0, SyntaxKind::Float)
            .or_else(|| token_text(&self.0, SyntaxKind::Integer))
            .or_else(|| token_text(&self.0, SyntaxKind::Rational))?;
        Some(if find_token(&self.0, SyntaxKind::Minus).is_some() {
            format!("-{digits}")
        } else {
            digits
        })
    }

    /// The unit suffix, if one was written.
    pub fn unit(&self) -> Option<String> {
        [
            SyntaxKind::UnitHz,
            SyntaxKind::UnitMs,
            SyntaxKind::UnitS,
            SyntaxKind::UnitDb,
        ]
        .into_iter()
        .find_map(|kind| token_text(&self.0, kind))
    }
}

/// A bare name in a signal position: another signal, a waveform, or `output`.
pub struct NameRef(SyntaxNode);
wrapper!(NameRef, SyntaxKind::NameRef);

impl NameRef {
    /// The written name.
    pub fn text(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| !token.kind().is_trivia())
            .map(|token| token.text().to_string())
    }
}

/// `modulate lfo -> glass_pad.lowpass.cutoff;`
pub struct ModulateStmt(SyntaxNode);
wrapper!(ModulateStmt, SyntaxKind::ModulateStmt);

impl ModulateStmt {
    /// The modulating signal's name.
    pub fn source(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The dotted path to the modulated parameter.
    pub fn target(&self) -> Vec<String> {
        child::<ParamPath>(&self.0)
            .map(|path| {
                path.0
                    .children_with_tokens()
                    .filter_map(SyntaxElement::into_token)
                    .filter(|token| token.kind() == SyntaxKind::Identifier)
                    .map(|token| token.text().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `glass_pad.lowpass.cutoff`
pub struct ParamPath(SyntaxNode);
wrapper!(ParamPath, SyntaxKind::ParamPath);

/// `assign violin -> glass_pad;` and `route violin -> master;` — two names
/// and an arrow, which is the same wrapper twice.
macro_rules! binding_wrapper {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <source> -> <destination>;`")]
        pub struct $name(SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            /// The left-hand name.
            pub fn source(&self) -> Option<String> {
                binding_names(&self.0).first().cloned()
            }

            /// The right-hand name (`master` included).
            pub fn destination(&self) -> Option<String> {
                binding_names(&self.0).get(1).cloned()
            }

            /// The right-hand name's own token, for an editor that means to
            /// rewrite the name and nothing else around it.
            pub fn destination_token(&self) -> Option<SyntaxToken> {
                binding_tokens(&self.0).into_iter().nth(1)
            }
        }
    };
}

/// The names on either side of a binding arrow, in written order. `master` is
/// a keyword rather than an identifier, so reading tokens by kind alone would
/// silently drop it.
fn binding_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| matches!(token.kind(), SyntaxKind::Identifier | SyntaxKind::MasterKw))
        .collect()
}

fn binding_names(node: &SyntaxNode) -> Vec<String> {
    binding_tokens(node)
        .iter()
        .map(|token| token.text().to_string())
        .collect()
}

binding_wrapper!(AssignStmt, SyntaxKind::AssignStmt, "assign");
binding_wrapper!(RouteStmt, SyntaxKind::RouteStmt, "route");

/// `send violin -> hall at -18 dB;`
pub struct SendStmt(SyntaxNode);
wrapper!(SendStmt, SyntaxKind::SendStmt);

impl SendStmt {
    /// The sending source's name.
    pub fn source(&self) -> Option<String> {
        binding_names(&self.0).first().cloned()
    }

    /// The receiving bus's name.
    pub fn destination(&self) -> Option<String> {
        binding_names(&self.0).get(1).cloned()
    }

    /// The send level written after `at`.
    pub fn level(&self) -> Option<ValueLiteral> {
        child(&self.0)
    }
}
