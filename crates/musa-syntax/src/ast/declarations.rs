//! See `ast` module docs; the items parsed in this family.

use super::AstNode;
use super::TypeExpr;
use super::child;
use super::children;
use super::is_private;
use super::is_type;
use super::token_text;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::{SyntaxElement, SyntaxNode};

/// A declared motif parameter: `name: kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The parameter name (`root`).
    pub name: String,
    /// The declared kind (`pitch` or `duration`).
    pub kind: String,
}

/// The annotated parameters of a declaration or function, in source order.
pub(crate) fn params_of(node: &SyntaxNode) -> Vec<FnParam> {
    node.children()
        .find_map(ParamList::cast)
        .map_or_else(Vec::new, |list| children(&list.0))
}

/// `signature TonalContext { let key: key; }` — the members a module must
/// provide, and their types.
pub struct SignatureDecl(SyntaxNode);
wrapper!(SignatureDecl, SyntaxKind::SignatureDecl);

impl SignatureDecl {
    /// Every signature declared at a document's lexical root, in source
    /// order.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The name modules and template parameters refer to it by.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Its members, in source order.
    pub fn members(&self) -> Vec<SignatureMember> {
        children(&self.0)
    }
}

/// `let key: key;` — one member of a signature: a name and the type a
/// module's definition of it must have.
pub struct SignatureMember(SyntaxNode);
wrapper!(SignatureMember, SyntaxKind::SignatureMember);

impl SignatureMember {
    /// The member's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The type a module's definition of it must have.
    ///
    /// A written type is one of several node kinds rather than one wrapper,
    /// so this answers with the node itself, the way every other reader of a
    /// type annotation takes it — through [`is_type`], so that a kind added to
    /// the type grammar is a type here too. Spelling the list again is how
    /// `EventTrack<WrittenTime>` came to be unreadable in a signature and
    /// nowhere else.
    pub fn ty(&self) -> Option<SyntaxNode> {
        self.0.children().find(|node| is_type(node.kind()))
    }
}

/// `structure CMajor : TonalContext { ... }` — a named group of declarations,
/// reached from outside as `CMajor.member`.
///
/// A `template structure` parameterizes one over other structures; the
/// parameter list is the only difference in the node, and [`TemplateDecl`] is
/// what says which of the two this is.
pub struct StructureDecl(SyntaxNode);
wrapper!(StructureDecl, SyntaxKind::StructureDecl);

impl StructureDecl {
    /// Every structure declared directly at a document's lexical root, in
    /// source order — not the ones a `template structure` parameterizes.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The structure's name: the qualifier its members are reached through.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The name of the signature it claims to provide.
    ///
    /// The second identifier, because `structure M : S { ... }` writes the
    /// structure's own name first and the parameters, when there are any, live
    /// inside a [`ParamList`] rather than among these tokens.
    pub fn signature(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| token.kind() == SyntaxKind::Identifier)
            .nth(1)
            .map(|token| token.text().to_string())
    }

    /// Its parameters, in source order — empty unless a `template` wraps it.
    pub fn params(&self) -> Vec<FnParam> {
        params_of(&self.0)
    }

    /// The values it defines, in source order.
    pub fn lets(&self) -> Vec<LetDecl> {
        children(&self.0)
    }

    /// The functions it defines, in source order.
    pub fn fns(&self) -> Vec<FnDecl> {
        children(&self.0)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    ///
    /// About the structure itself and not about its members: a member the
    /// signature does not list is already private *to the structure*, which is
    /// sealing by listing rather than by marking, and the two do not overlap.
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `let name = expression;`, with an optional `: type` before the `=`.
pub struct LetDecl(SyntaxNode);
wrapper!(LetDecl, SyntaxKind::LetDecl);

impl LetDecl {
    /// The declared name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `fn name(parameters) { expression }`, with an optional `-> type`.
pub struct FnDecl(SyntaxNode);
wrapper!(FnDecl, SyntaxKind::FnDecl);

impl FnDecl {
    /// The function's declared name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Its parameters in source order, annotated or not.
    pub fn params(&self) -> Vec<FnParam> {
        params_of(&self.0)
    }

    /// Its body, or `None` for a file the parser has already complained
    /// about.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == SyntaxKind::BlockExpr)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// One function parameter, with or without its `: type`.
pub struct FnParam(SyntaxNode);
wrapper!(FnParam, SyntaxKind::Param);

impl FnParam {
    /// The parameter name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }
}

/// A function declaration's parenthesized parameters.
pub struct ParamList(SyntaxNode);
wrapper!(ParamList, SyntaxKind::ParamList);

/// `data Motive { Silence, Sounded(pitch: Pitch) }` — one nominal declaration.
pub struct DataDecl(SyntaxNode);
wrapper!(DataDecl, SyntaxKind::DataDecl);

impl DataDecl {
    /// Every `data` declaration among this node's children, in source order.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The type's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The names of its type parameters, in source order.
    pub fn parameters(&self) -> Vec<String> {
        type_parameters(&self.0)
    }

    /// Its constructors, in source order.
    pub fn variants(&self) -> Vec<DataVariant> {
        children(&self.0)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `record Pending { read: Reading; dots: Dots; }` — a declaration of named
/// fields.
pub struct RecordDecl(SyntaxNode);
wrapper!(RecordDecl, SyntaxKind::RecordDecl);

impl RecordDecl {
    /// Every `record` declaration among this node's children, in source order.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The type's name.
    ///
    /// What diagnostics say, and nothing more: a record *is* its fields
    /// (`01-surface.md` §1.2), so two declarations with the same fields at the
    /// same types denote one type whatever they are called.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The names of its type parameters, in source order.
    pub fn parameters(&self) -> Vec<String> {
        type_parameters(&self.0)
    }

    /// Its fields, in declaration order — which is the order they are
    /// evaluated in and the order a telescope reads them.
    pub fn fields(&self) -> Vec<FieldDecl> {
        children(&self.0)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `read: Reading;` — one declared field of a record or of a named enum case.
pub struct FieldDecl(SyntaxNode);
wrapper!(FieldDecl, SyntaxKind::FieldDecl);

impl FieldDecl {
    /// The field's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Its written type.
    pub fn ty(&self) -> Option<TypeExpr> {
        child(&self.0)
    }
}

/// `enum Tying { Untied, TiedOn }` — a nominal sum.
pub struct EnumDecl(SyntaxNode);
wrapper!(EnumDecl, SyntaxKind::EnumDecl);

impl EnumDecl {
    /// Every `enum` declaration among this node's children, in source order.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The type's name, which is also the namespace its cases live in.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The names of its type parameters, in source order.
    pub fn parameters(&self) -> Vec<String> {
        type_parameters(&self.0)
    }

    /// Its cases, in source order. An enum may have none.
    pub fn cases(&self) -> Vec<EnumCase> {
        children(&self.0)
    }

    /// Whether it is marked `private`, and so nameable only inside the module
    /// that declares it (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `Refused { at: NodePath, why: Text }` — one case of an enum.
pub struct EnumCase(SyntaxNode);
wrapper!(EnumCase, SyntaxKind::EnumCase);

impl EnumCase {
    /// The case's name, unqualified: the type it belongs to is the enclosing
    /// declaration, and writing it again here would be writing it twice.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The types a positional case carries, in source order. Empty for an
    /// empty case and for one that names its fields.
    ///
    /// Read by [`is_type`] rather than as a [`TypeExpr`], because `TiedOn(Nat)`
    /// writes a [`SyntaxKind::TypeName`] and only a parenthesized type is a
    /// `TypeExpr`: a positional list holds whatever the type grammar produced.
    pub fn positional(&self) -> Vec<SyntaxNode> {
        self.0.children().filter(|node| is_type(node.kind())).collect()
    }

    /// The fields a named case declares, in source order.
    pub fn named(&self) -> Vec<FieldDecl> {
        children(&self.0)
    }

    /// Whether the case is marked `private`, so that the *type* stays public
    /// and the constructor is the declaring module's to build
    /// (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `impl Duration { … }` — one type's namespace, opened.
///
/// The block declares nothing a top-level `fn` does not: each item inside it is
/// the definition `Duration.name`, and `01-surface.md` §1.5 reaches it from
/// `d.name(…)`, from `Duration::name(…)`, and from the operator that spells it.
pub struct ImplDecl(SyntaxNode);
wrapper!(ImplDecl, SyntaxKind::ImplDecl);

impl ImplDecl {
    /// Every `impl` block among this node's children, in source order.
    pub fn all_at_root(node: &SyntaxNode) -> Vec<Self> {
        children(node)
    }

    /// The type whose namespace this block opens.
    pub fn head(&self) -> Option<SyntaxNode> {
        written_type(&self.0)
    }

    /// The functions it declares, in source order.
    pub fn methods(&self) -> Vec<FnDecl> {
        children(&self.0)
    }

    /// Whether it is marked `private` (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// The one written type among `node`'s children.
///
/// A type is several node kinds rather than one wrapper ([`is_type`]), so a
/// place that holds exactly one asks for it this way rather than casting to
/// the kind it happened to be written in.
pub(crate) fn written_type(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.children().find(|child| is_type(child.kind()))
}

/// `data Motive;` — a signature member naming a type without its
/// constructors.
pub struct DataMember(SyntaxNode);
wrapper!(DataMember, SyntaxKind::DataMember);

impl DataMember {
    /// The type's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The names of its type parameters, in source order.
    pub fn parameters(&self) -> Vec<String> {
        type_parameters(&self.0)
    }
}

/// The type parameters written on a declaration or a signature member.
pub(crate) fn type_parameters(node: &SyntaxNode) -> Vec<String> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::TypeParams)
        .map(|params| {
            params
                .children()
                .filter(|child| child.kind() == SyntaxKind::TypeParam)
                .filter_map(|param| token_text(&param, SyntaxKind::Identifier))
                .collect()
        })
        .unwrap_or_default()
}

/// `private Sounded(pitch: Pitch, Duration)` — one constructor.
pub struct DataVariant(SyntaxNode);
wrapper!(DataVariant, SyntaxKind::DataVariant);

impl DataVariant {
    /// The constructor's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// Its named fields, in source order. A constructor with no fields, and one
    /// that writes its arguments positionally, has none.
    pub fn fields(&self) -> Vec<DataField> {
        children(&self.0)
    }

    /// The types it carries positionally, in source order.
    ///
    /// The other half of [`Self::fields`], and both may be written in one
    /// constructor: `01-surface.md` §1.3's positional form names types and not
    /// fields, so what a reader gets back from it is a type and nothing else.
    /// Read the way [`EnumCase::positional`] is read, and for the same reason.
    pub fn positional(&self) -> Vec<SyntaxNode> {
        self.0.children().filter(|node| is_type(node.kind())).collect()
    }

    /// Whether the constructor is marked `private`, so that the *type* stays
    /// public and the case is the declaring module's to build
    /// (`01-surface.md` §1.3).
    pub fn is_private(&self) -> bool {
        is_private(&self.0)
    }
}

/// `pitch: Pitch` — one named field.
pub struct DataField(SyntaxNode);
wrapper!(DataField, SyntaxKind::DataField);

impl DataField {
    /// The field's name.
    pub fn name(&self) -> Option<String> {
        token_text(&self.0, SyntaxKind::Identifier)
    }

    /// The type it stores.
    pub fn ty(&self) -> Option<SyntaxNode> {
        self.0.children().find(|node| is_type(node.kind()))
    }
}
