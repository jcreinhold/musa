//! See `ast` module docs; the items parsed in this family.

use super::AstNode;
use super::child;
use super::is_type;
use super::wrapper;
use crate::SyntaxKind;
use crate::language::SyntaxNode;

/// A parenthesized type expression.
pub struct TypeExpr(SyntaxNode);
wrapper!(TypeExpr, SyntaxKind::TypeExpr);

/// A base or named type.
pub struct TypeName(SyntaxNode);
wrapper!(TypeName, SyntaxKind::TypeName);

/// A right-associative function type.
pub struct FunctionType(SyntaxNode);
wrapper!(FunctionType, SyntaxKind::FunctionType);

/// A product type.
pub struct ProductType(SyntaxNode);
wrapper!(ProductType, SyntaxKind::ProductType);

/// `option[type]`.
pub struct OptionType(SyntaxNode);
wrapper!(OptionType, SyntaxKind::OptionType);

/// `list[type]`.
pub struct ListType(SyntaxNode);
wrapper!(ListType, SyntaxKind::ListType);

/// `Result<value, error>` — the binary sum.
pub struct ResultType(SyntaxNode);
wrapper!(ResultType, SyntaxKind::ResultType);

/// `Tree<Nat>` — a declared type applied to its arguments.
pub struct AppliedType(SyntaxNode);
wrapper!(AppliedType, SyntaxKind::AppliedType);

/// `Pc(12)` — a type carrying an index.
pub struct IndexedType(SyntaxNode);
wrapper!(IndexedType, SyntaxKind::IndexedType);

impl IndexedType {
    /// The type's name.
    pub fn name(&self) -> Option<String> {
        child::<TypeName>(&self.0).map(|name| name.syntax().to_string().trim().to_owned())
    }

    /// The index it carries, as the expression node it was written as.
    pub fn index(&self) -> Option<SyntaxNode> {
        self.0.children().find(|node| !is_type(node.kind()))
    }
}

impl AppliedType {
    /// The declaration's name.
    pub fn name(&self) -> Option<String> {
        child::<TypeName>(&self.0).map(|name| name.syntax().to_string().trim().to_owned())
    }

    /// The types it is applied to, in source order.
    pub fn arguments(&self) -> Vec<SyntaxNode> {
        self.0.children().skip(1).filter(|node| is_type(node.kind())).collect()
    }
}
