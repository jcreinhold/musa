//! See `ast` module docs; the items parsed in this family.

use super::AstNode;
use super::binding_wrapper;
use super::chain_container;
use super::child;
use super::children;
use super::find_token;
use super::token_text;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode, SyntaxToken};

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

/// `lowpass(cutoff: 1400 Hz, resonance: 0.7)`
pub struct CallExpr(SyntaxNode);
wrapper!(CallExpr, SyntaxKind::CallExpr);

impl CallExpr {
    /// The processor being constructed.
    ///
    /// `scale` is a keyword in the notation language and a processor in the
    /// studio, so the studio's name may arrive as either token.
    pub fn callee(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier).or_else(|| token_text(&self.0, SyntaxKind::ScaleKw))
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

    /// The argument-name token, when this is a named argument.
    ///
    /// Diagnostics use its exact range to replace a removed spelling without
    /// rewriting the value or its comments.
    pub fn name_token(&self) -> Option<SyntaxToken> {
        find_token(&self.0, SyntaxKind::Colon)?;
        find_token(&self.0, SyntaxKind::Identifier)
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

/// The names on either side of a binding arrow, in written order. `master` is
/// a keyword rather than an identifier, so reading tokens by kind alone would
/// silently drop it.
pub(crate) fn binding_tokens(node: &SyntaxNode) -> Vec<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| matches!(token.kind(), SyntaxKind::Identifier | SyntaxKind::MasterKw))
        .collect()
}

pub(crate) fn binding_names(node: &SyntaxNode) -> Vec<String> {
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

    /// The sending source's own token, for an editor that means to rewrite
    /// the name and nothing else around it.
    pub fn source_token(&self) -> Option<SyntaxToken> {
        binding_tokens(&self.0).into_iter().next()
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
