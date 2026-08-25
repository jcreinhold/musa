//! See `ast` module docs; the items parsed in this family.

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
