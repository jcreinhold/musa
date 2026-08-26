//! Typed views over the concrete syntax tree (roadmap §10.5).
//!
//! Each wrapper is a thin cast around a [`SyntaxNode`] with accessors for its
//! significant children. Wrappers cache nothing and perform no semantic
//! resolution: `NoteStmt::pitch` returns the written text `g#4`, not a
//! computed pitch. New wrappers are cheap; create them with
//! [`AstNode::cast`].
//!
//! The wrappers are grouped by the family of syntax they read, one module
//! each:
//!
//! - `document` — file shapes: piece/library/module bodies, imports,
//!   templates, makes, front matter
//! - `performance` — performance declarations and their rule settings
//! - `declarations` — `let`/`fn`/signature/structure/data/record/enum/impl
//!   declarations and their members
//! - `types` — type expressions
//! - `expressions` — the expression grammar and match patterns
//! - `score` — score/part/voice scaffolding and the motif/fragment constructs
//! - `voice` — note items and the statements a voice holds
//! - `harmony` — harmonic analysis
//! - `quotation` — quotes, splices, and events holes
//! - `studio` — the studio graph

use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode, SyntaxToken};

mod declarations;
mod document;
mod expressions;
mod harmony;
mod performance;
mod quotation;
mod score;
mod studio;
mod types;
mod voice;

// ---------------------------------------------------------------------------
// The shared vocabulary: the AstNode trait, the accessor helpers every
// wrapper's methods parse with, the string literal encoding, and the macros
// that stamp out a wrapper. All paths inside the macro bodies are spelled
// `$crate::` so an invocation does not depend on names being imported at the
// call site.
// ---------------------------------------------------------------------------

/// A typed view over a [`SyntaxNode`].
pub trait AstNode: Sized {
    /// Wrap `node` if it has the expected kind.
    fn cast(node: SyntaxNode) -> Option<Self>;
    /// The underlying untyped node.
    fn syntax(&self) -> &SyntaxNode;
}

pub(crate) fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

pub(crate) fn children<N: AstNode>(node: &SyntaxNode) -> Vec<N> {
    node.children().filter_map(N::cast).collect()
}

pub(crate) fn find_token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|token| token.kind() == kind)
}

/// The source text an element covers: a token's own text, or a node's whole
/// text.
///
/// The three composite literals — `c#5`, `M3`, `3/8` — are nodes whose
/// children are the parts the lexer's pattern already found
/// (`parser/literals.rs`), so the lexeme a reader wants is the *node's* text.
/// Everything else that a reader asks for by kind is still a token, and a
/// token's text is its own.
pub(crate) fn element_text(element: &SyntaxElement) -> String {
    match element {
        SyntaxElement::Node(node) => node.text().to_string(),
        SyntaxElement::Token(token) => token.text().to_string(),
    }
}

/// One lexeme of `kind` among a node's own children — a token of that kind,
/// or the composite literal node standing for one.
///
/// A reader asking for a `PitchLiteral` wants `g#4`, and whether the CST
/// spells that as one token or as a node over its parts is not a question the
/// reader is asking.
pub(crate) fn find_lexeme(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxElement> {
    node.children_with_tokens().find(|element| element.kind() == kind)
}

pub(crate) fn token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    find_lexeme(node, kind).as_ref().map(element_text)
}

/// Whether a declaration carries the `private` marker
/// (`01-surface.md` §1.3).
///
/// A token of the declaration's own node rather than a wrapper around it, so
/// every accessor a declaration already had still reads the same node and the
/// answer costs one child lookup.
pub(crate) fn is_private(node: &SyntaxNode) -> bool {
    find_token(node, SyntaxKind::PrivateKw).is_some()
}

pub(crate) fn descendant_token_text(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.descendants_with_tokens()
        .find(|element| element.kind() == kind)
        .as_ref()
        .map(element_text)
}

/// A token's byte range, as the compiler's spans are counted.
pub(crate) fn span_of(token: &SyntaxToken) -> (u32, u32) {
    let range = token.text_range();
    (u32::from(range.start()), u32::from(range.end()))
}

/// The text a string literal stands for: one pair of quotes off, escapes
/// resolved.
///
/// The lexer's string pattern admits `\\` escapes, so a title that contains a
/// quotation mark reaches the CST as `\"` and has to come back out as `"` —
/// otherwise reading a piece's own name and writing it back is not the
/// identity, which is exactly what an editable title field does on every
/// keystroke. An unknown escape keeps its character rather than its
/// backslash: this resolves what the lexer accepts and invents nothing.
///
/// **`\n` is the one escape that names a character rather than removing a
/// backslash**, and it is here because the lexer's string body excludes a raw
/// line feed (`lexer.rs`'s `[^"\\\n]`) — so without it there is no text
/// containing a newline that [`quote`] could write at all, and the pair below
/// would be an inverse only on the texts that happen not to hold one. A tab
/// needs no such treatment: the body pattern admits it raw.
///
/// With [`quote`] this pair is `Text`'s exact encoding
/// (`docs/rules/language/02-core-calculus.md` §1.1): `unquote(quote(t))` is
/// `t` for **every** text, so a text value written into source and read back
/// is the same value.
#[must_use]
pub fn unquote(literal: &str) -> String {
    let body = literal
        .strip_prefix('"')
        .map_or(literal, |rest| rest.strip_suffix('"').unwrap_or(rest));
    let mut out = String::with_capacity(body.len());
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('n') => out.push('\n'),
                Some(escaped) => out.push(escaped),
                None => {}
            }
        } else {
            out.push(character);
        }
    }
    out
}

/// `text` as a string literal the lexer will read back as `text`.
///
/// The inverse of [`unquote`], and the only correct way to write a value a
/// composer typed into the source.
///
/// Three characters are written as escapes and the rest stand for themselves:
/// a quotation mark and a backslash because they would end the literal or
/// start an escape, and a line feed because the lexer's string body refuses
/// one raw. That set is decided by the lexer's pattern rather than chosen —
/// escaping a character the pattern already admits would make the spelling
/// longer and the reading no different.
#[must_use]
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for character in text.chars() {
        match character {
            '"' | '\\' => {
                out.push('\\');
                out.push(character);
            }
            '\n' => out.push_str("\\n"),
            _ => out.push(character),
        }
    }
    out.push('"');
    out
}

macro_rules! wrapper {
    ($name:ident, $kind:expr) => {
        impl $crate::ast::AstNode for $name {
            fn cast(node: $crate::language::SyntaxNode) -> Option<Self> {
                (node.kind() == $kind).then_some(Self(node))
            }

            fn syntax(&self) -> &$crate::language::SyntaxNode {
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
pub(crate) use wrapper;

/// One rule's head name and its settings; `mark` and `dynamic` rules
/// share the shape, so they share the accessors.
macro_rules! rule_wrapper {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <name> { <setting>* }` inside a profile.")]
        pub struct $name($crate::language::SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            #[doc = concat!("The ", $what, " this rule names.")]
            pub fn name(&self) -> Option<String> {
                $crate::ast::token_text(&self.0, $crate::SyntaxKind::Identifier)
            }

            /// The rule's settings, in source order.
            pub fn settings(&self) -> Vec<$crate::ast::SettingStmt> {
                $crate::ast::children(&self.0)
            }
        }
    };
}
pub(crate) use rule_wrapper;

/// A patch or a bus: a named container of signal bindings and chains. The two
/// differ in what they may contain semantically, not syntactically, so one
/// macro gives both the same accessors.
macro_rules! chain_container {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <name> { <binding-or-chain>* }`")]
        pub struct $name($crate::language::SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            #[doc = concat!("The ", $what, "'s name.")]
            pub fn name(&self) -> Option<String> {
                $crate::ast::token_text(&self.0, $crate::SyntaxKind::Identifier)
            }

            /// The named signals declared inside, in source order.
            pub fn signals(&self) -> Vec<$crate::ast::SignalBinding> {
                $crate::ast::children(&self.0)
            }

            /// The unnamed chains, in source order.
            pub fn chains(&self) -> Vec<$crate::ast::ChainStmt> {
                $crate::ast::children(&self.0)
            }
        }
    };
}
pub(crate) use chain_container;

/// `assign violin -> glass_pad;` and `route violin -> master;` — two names
/// and an arrow, which is the same wrapper twice.
macro_rules! binding_wrapper {
    ($name:ident, $kind:expr, $what:literal) => {
        #[doc = concat!("`", $what, " <source> -> <destination>;`")]
        pub struct $name($crate::language::SyntaxNode);
        wrapper!($name, $kind);

        impl $name {
            /// The left-hand name.
            pub fn source(&self) -> Option<String> {
                $crate::ast::binding_names(&self.0).first().cloned()
            }

            /// The right-hand name (`master` included).
            pub fn destination(&self) -> Option<String> {
                $crate::ast::binding_names(&self.0).get(1).cloned()
            }

            /// The left-hand name's own token, for an editor that means to
            /// rewrite the name and nothing else around it.
            pub fn source_token(&self) -> Option<$crate::language::SyntaxToken> {
                $crate::ast::binding_tokens(&self.0).into_iter().next()
            }

            /// The right-hand name's own token, for an editor that means to
            /// rewrite the name and nothing else around it.
            pub fn destination_token(&self) -> Option<$crate::language::SyntaxToken> {
                $crate::ast::binding_tokens(&self.0).into_iter().nth(1)
            }
        }
    };
}
pub(crate) use binding_wrapper;

/// Whether `kind` is a type-expression node.
///
/// A type is several node kinds rather than one wrapper, so every reader of an
/// annotation asks this rather than casting.
#[must_use]
pub fn is_type(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::TypeExpr | SyntaxKind::TypeName | SyntaxKind::FunctionType | SyntaxKind::ProductType
    )
}

pub(crate) use declarations::params_of;
pub(crate) use studio::{binding_names, binding_tokens};
pub(crate) use voice::{duration_text, voice_items};

// ---------------------------------------------------------------------------
// The public surface, re-exported so `musa_syntax::ast::NoteStmt` and the
// rest keep the paths the rest of the workspace reads them by.
// ---------------------------------------------------------------------------

pub use document::{
    ClefStmt, Document, FrontMatterRole, FrontMatterStmt, ImportStmt, KeyStmt, MeterStmt, ModDecl, PieceDecl, TempoStmt,
};

pub use performance::{DynamicRule, GraceRule, GrooveRule, MarkRule, PerformanceDecl, ProfileDecl, SettingStmt};

pub use declarations::{
    DataDecl, DataField, DataVariant, EnumCase, EnumDecl, FieldDecl, FnDecl, FnParam, ImplDecl, LetDecl, Param,
    ParamList, RecordDecl,
};

pub use types::{FunctionType, ProductType, TypeExpr, TypeName};

pub use expressions::{
    ApplyExpr, ExprArg, ExprArgList, FieldInit, FieldPath, FieldPattern, FieldUpdate, IfExpr, LambdaExpr, ListExpr,
    LiteralExpr, MatchArm, MatchExpr, NameExpr, OptionExpr, ParenExpr, PathExpr, Pattern, ProductExpr, QuestionExpr,
    RecordLiteralExpr, RecordPattern, RecordUpdateExpr, ResultExpr,
};

pub use score::{
    CueStmt, FragmentDecl, ImproviseStmt, MobileStmt, MotifDecl, PartDecl, ProfileStmt, ScoreDecl, SectionStmt,
    SoundStmt, VoiceDecl, VoiceItem,
};

pub use voice::{
    AssertStmt, BarStmt, ChordStmt, Duration, DynamicStmt, EndingStmt, GraceNote, GraceStmt, HairpinStmt, InScaleStmt,
    InvertStmt, MarkStmt, MusicExpr, NoteStmt, OverrideStmt, PhraseStmt, RepeatStmt, RestStmt, RetrogradeStmt,
    SenzaStmt, SlurStmt, StackStmt, StretchStmt, TransposeStmt, TupletStmt, UseStmt,
};

pub use harmony::{ChordSymbol, HarmonyDecl, HarmonyStmt, Position};

pub use quotation::{EventsHole, EventsQuote, QuoteExpr, QuotePattern, SequenceSplice, Splice};

pub use studio::{
    Arg, ArgList, AssignStmt, BusDecl, CallExpr, ChainStmt, ClipDecl, FixedMediaDecl, InstrumentDecl,
    InstrumentImplementation, ModulateStmt, NameRef, ParamPath, PatchDecl, RoomDecl, RouteStmt, SendStmt,
    SignalBinding, SignalChain, SignalStage, StudioDecl, StudioItem, ValueLiteral,
};
