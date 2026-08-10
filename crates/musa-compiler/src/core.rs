//! The private total elaboration core (`docs/language/02-core-calculus.md`).
//!
//! This module owns lowering, checking, dependency validation, and evaluation
//! for elaboration values.  Its two crate-private entry points deliberately
//! return only success: the existing compiler facade remains the sole public
//! operation, diagnostics accumulate in the ordinary resolver, and no caller
//! can observe or orchestrate the pass representation.

use indexmap::{IndexMap, IndexSet};
use musa_language::ast::{AstNode as _, FnDecl, LetDecl};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
use num_rational::Ratio;

use crate::diagnose::{Code, Diagnostic};
use crate::imports::Libraries;
use crate::origin::{Interval, SourceSpan};
use crate::pitch::WrittenPitch;
use crate::resolve::{NameKind, Resolver};

/// Check and evaluate imported definitions followed by a piece's definitions.
pub(crate) fn check_piece(
    resolver: &mut Resolver,
    libraries: &Libraries,
    piece: &musa_language::ast::PieceDecl,
) -> bool {
    if !validate_imports(resolver, libraries) {
        return false;
    }
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(_, library)| declarations(library.syntax(), true))
            .chain(declarations(piece.syntax(), false)),
    )
    .is_some()
}

/// Check and evaluate imported definitions followed by an opened library's.
pub(crate) fn check_material(
    resolver: &mut Resolver,
    libraries: &Libraries,
    library: &musa_language::ast::LibraryDecl,
) -> bool {
    if !validate_imports(resolver, libraries) {
        return false;
    }
    check_and_evaluate(
        resolver,
        libraries
            .each()
            .flat_map(|(_, imported)| declarations(imported.syntax(), true))
            .chain(declarations(library.syntax(), false)),
    )
    .is_some()
}

/// Check each library with precisely the earlier libraries available to it.
/// A failure is then restated at the importing document's `use` span: spans
/// inside the foreign CST must never be published as spans in this document.
fn validate_imports(resolver: &mut Resolver, libraries: &Libraries) -> bool {
    let mut prefix = Vec::new();
    for (path, library, import_span) in libraries.each_with_import_span() {
        prefix.extend(declarations(library.syntax(), true));
        let mut foreign_resolver = Resolver::new();
        let evaluated = check_and_evaluate(&mut foreign_resolver, prefix.clone().into_iter()).is_some();
        let first_error = foreign_resolver
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == crate::diagnose::Severity::Error);
        if !evaluated || first_error.is_some() {
            let detail = first_error.map_or("elaboration evaluation failed", |diagnostic| {
                diagnostic.message.as_str()
            });
            resolver.report(
                Diagnostic::error(Code::Import, format!("`{path}` does not compile"))
                    .at(import_span, "imported here")
                    .note(format!("it says: {detail}"))
                    .help(format!("run `musa check {path}`")),
            );
            return false;
        }
    }
    true
}

fn declarations(owner: &SyntaxNode, foreign: bool) -> Vec<SurfaceDefinition> {
    owner
        .children()
        .filter_map(|node| {
            LetDecl::cast(node.clone())
                .map(|declaration| SurfaceDefinition::Let { declaration, foreign })
                .or_else(|| FnDecl::cast(node).map(|declaration| SurfaceDefinition::Function { declaration, foreign }))
        })
        .collect()
}

#[derive(Clone)]
enum SurfaceDefinition {
    Let { declaration: LetDecl, foreign: bool },
    Function { declaration: FnDecl, foreign: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Type {
    Unit,
    Bool,
    Nat,
    Ratio,
    Duration,
    Pitch,
    Interval,
    Product(Vec<Self>),
    Function(Vec<Self>, Box<Self>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => out.write_str("unit"),
            Self::Bool => out.write_str("bool"),
            Self::Nat => out.write_str("nat"),
            Self::Ratio => out.write_str("ratio"),
            Self::Duration => out.write_str("duration"),
            Self::Pitch => out.write_str("pitch"),
            Self::Interval => out.write_str("interval"),
            Self::Product(members) => {
                out.write_str("(")?;
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        out.write_str(", ")?;
                    }
                    write!(out, "{member}")?;
                }
                out.write_str(")")
            }
            Self::Function(parameters, result) => {
                if parameters.len() == 1 {
                    let parameter = parameters.first().unwrap_or(&Self::Unit);
                    if matches!(parameter, Self::Function(_, _)) {
                        write!(out, "({parameter}) -> {result}")
                    } else {
                        write!(out, "{parameter} -> {result}")
                    }
                } else {
                    out.write_str("(")?;
                    for (index, parameter) in parameters.iter().enumerate() {
                        if index > 0 {
                            out.write_str(", ")?;
                        }
                        write!(out, "{parameter}")?;
                    }
                    write!(out, ") -> {result}")
                }
            }
        }
    }
}

#[derive(Clone)]
struct RawParameter {
    name: String,
    ty: Type,
    default: Option<SyntaxNode>,
    span: SourceSpan,
}

struct RawDefinition {
    name: String,
    ty: Type,
    kind: RawDefinitionKind,
    name_span: SourceSpan,
    span: SourceSpan,
    foreign: bool,
}

enum RawDefinitionKind {
    Let {
        body: SyntaxNode,
    },
    Function {
        parameters: Vec<RawParameter>,
        body: SyntaxNode,
    },
}

impl RawDefinition {
    fn name_kind(&self) -> NameKind {
        match self.kind {
            RawDefinitionKind::Let { .. } => NameKind::Value,
            RawDefinitionKind::Function { .. } => NameKind::Function,
        }
    }
}

#[derive(Clone)]
struct Symbol {
    ty: Type,
    kind: NameKind,
    definition: usize,
}

#[derive(Clone)]
struct Expr {
    kind: ExprKind,
    ty: Type,
    span: SourceSpan,
}

#[derive(Clone)]
enum ExprKind {
    Literal(Value),
    Name(String),
    Product(Vec<Expr>),
    Apply {
        function: Box<Expr>,
        arguments: Vec<CallArgument>,
    },
}

#[derive(Clone)]
struct CallArgument {
    parameter: usize,
    value: Expr,
}

struct CheckedDefinition {
    name: String,
    ty: Type,
    kind: CheckedDefinitionKind,
    dependencies: IndexMap<String, SourceSpan>,
    span: SourceSpan,
    foreign: bool,
}

enum CheckedDefinitionKind {
    Let {
        body: Expr,
    },
    Function {
        parameters: Vec<CheckedParameter>,
        body: Expr,
    },
}

#[derive(Clone)]
struct CheckedParameter {
    name: String,
    ty: Type,
    default: Option<Expr>,
}

#[derive(Clone)]
enum Value {
    Bool(bool),
    Nat(u64),
    Ratio(Ratio<i64>),
    Duration(Ratio<i64>),
    Pitch(WrittenPitch),
    Interval(Interval),
    Product(Vec<Self>),
    Closure(Box<Closure>),
}

#[derive(Clone)]
struct Closure {
    parameters: Vec<CheckedParameter>,
    result: Type,
    body: Expr,
    captures: IndexMap<String, Value>,
}

impl Value {
    fn ty(&self) -> Type {
        match self {
            Self::Bool(_) => Type::Bool,
            Self::Nat(_) => Type::Nat,
            Self::Ratio(_) => Type::Ratio,
            Self::Duration(_) => Type::Duration,
            Self::Pitch(_) => Type::Pitch,
            Self::Interval(_) => Type::Interval,
            Self::Product(members) => Type::Product(members.iter().map(Self::ty).collect()),
            Self::Closure(closure) => Type::Function(
                closure
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                Box::new(closure.result.clone()),
            ),
        }
    }

    /// Read the whole finite value into a deterministic witness.  The result
    /// is not a semantic hash or cache key; it makes the executable
    /// normalization check traverse, rather than merely construct, the value
    /// before the private environment is discarded by this prompt's caller.
    fn normalization_witness(&self) -> u64 {
        match self {
            Self::Bool(value) => u64::from(*value),
            Self::Nat(value) => *value,
            Self::Ratio(value) | Self::Duration(value) => {
                value.numer().unsigned_abs().rotate_left(7) ^ value.denom().unsigned_abs()
            }
            Self::Pitch(value) => {
                u64::from(value.letter.steps().unsigned_abs()).rotate_left(8)
                    ^ u64::from(value.accidental.0.unsigned_abs()).rotate_left(4)
                    ^ u64::from(value.octave.unsigned_abs())
            }
            Self::Interval(value) => {
                u64::from(value.diatonic_steps.unsigned_abs()).rotate_left(8)
                    ^ u64::from(value.semitones.unsigned_abs())
            }
            Self::Product(members) => members.iter().fold(0u64, |witness, member| {
                witness.rotate_left(5) ^ member.normalization_witness()
            }),
            Self::Closure(closure) => closure.captures.values().fold(
                u64::try_from(closure.parameters.len()).unwrap_or(u64::MAX),
                |witness, captured| witness.rotate_left(5) ^ captured.normalization_witness(),
            ),
        }
    }
}

fn check_and_evaluate(
    resolver: &mut Resolver,
    declarations: impl Iterator<Item = SurfaceDefinition>,
) -> Option<IndexMap<String, Value>> {
    let mut raw = Vec::new();
    let mut names: IndexMap<String, (SourceSpan, bool)> = IndexMap::new();
    for declaration in declarations {
        let (name, name_span, span, foreign) = surface_identity(&declaration)?;
        if let Some((first, first_is_foreign)) = names.get(&name).copied() {
            let mut diagnostic = Diagnostic::error(Code::DuplicateName, format!("`{name}` is bound twice"))
                .at(name_span, "bound again here");
            if !first_is_foreign {
                diagnostic = diagnostic.also(first, "first bound here");
            }
            resolver.report(diagnostic.help("give one of the bindings a different name"));
            continue;
        }
        names.insert(name.clone(), (name_span, foreign));
        if let Some(definition) = lower_signature(resolver, declaration, name, name_span, span, foreign) {
            raw.push(definition);
        }
    }

    let mut symbols = IndexMap::new();
    for (index, definition) in raw.iter().enumerate() {
        symbols.insert(
            definition.name.clone(),
            Symbol {
                ty: definition.ty.clone(),
                kind: definition.name_kind(),
                definition: index,
            },
        );
        if !definition.foreign {
            resolver
                .references
                .declare(definition.name_kind(), &definition.name, definition.name_span);
        }
    }

    let mut checked = Vec::with_capacity(raw.len());
    let mut type_errors = false;
    for definition in &raw {
        let mut checker = Checker {
            resolver,
            definitions: &raw,
            symbols: &symbols,
            locals: IndexMap::new(),
            dependencies: IndexMap::new(),
            foreign: definition.foreign,
            failed: false,
        };
        let kind = match &definition.kind {
            RawDefinitionKind::Let { body } => checker
                .check(body, Some(&definition.ty))
                .map(|body| CheckedDefinitionKind::Let { body }),
            RawDefinitionKind::Function { parameters, body } => {
                let mut checked_parameters = Vec::with_capacity(parameters.len());
                let mut duplicate_parameters = IndexMap::<String, SourceSpan>::new();
                for parameter in parameters {
                    if let Some(first) = duplicate_parameters.get(&parameter.name).copied() {
                        checker.resolver.report(
                            Diagnostic::error(
                                Code::DuplicateName,
                                format!("parameter `{}` is bound twice", parameter.name),
                            )
                            .at(parameter.span, "bound again here")
                            .also(first, "first bound here"),
                        );
                        checker.failed = true;
                        continue;
                    }
                    duplicate_parameters.insert(parameter.name.clone(), parameter.span);
                    let default = parameter
                        .default
                        .as_ref()
                        .and_then(|expression| checker.check(expression, Some(&parameter.ty)));
                    checker.locals.insert(parameter.name.clone(), parameter.ty.clone());
                    checked_parameters.push(CheckedParameter {
                        name: parameter.name.clone(),
                        ty: parameter.ty.clone(),
                        default,
                    });
                }
                checker
                    .check(body, function_result(&definition.ty))
                    .map(|body| CheckedDefinitionKind::Function {
                        parameters: checked_parameters,
                        body,
                    })
            }
        };
        type_errors |= checker.failed || kind.is_none();
        if let Some(kind) = kind {
            checked.push(CheckedDefinition {
                name: definition.name.clone(),
                ty: definition.ty.clone(),
                kind,
                dependencies: checker.dependencies,
                span: definition.span,
                foreign: definition.foreign,
            });
        }
    }
    if type_errors || checked.len() != raw.len() {
        return None;
    }

    let order = dependency_order(resolver, &checked)?;
    evaluate(resolver, &checked, &order)
}

fn surface_identity(definition: &SurfaceDefinition) -> Option<(String, SourceSpan, SourceSpan, bool)> {
    let (syntax, name, foreign) = match definition {
        SurfaceDefinition::Let { declaration, foreign } => (declaration.syntax(), declaration.name(), *foreign),
        SurfaceDefinition::Function { declaration, foreign } => (declaration.syntax(), declaration.name(), *foreign),
    };
    let name = name?;
    let name_span = crate::resolve::token_span(syntax, SyntaxKind::Identifier)?;
    Some((name, name_span, crate::resolve::trimmed_span(syntax), foreign))
}

fn lower_signature(
    resolver: &mut Resolver,
    definition: SurfaceDefinition,
    name: String,
    name_span: SourceSpan,
    span: SourceSpan,
    foreign: bool,
) -> Option<RawDefinition> {
    match definition {
        SurfaceDefinition::Let { declaration, .. } => {
            let ty_node = child_of(declaration.syntax(), is_type_node)?;
            let body = child_of(declaration.syntax(), is_expr_node)?;
            let ty = parse_type(resolver, &ty_node)?;
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Let { body },
                name_span,
                span,
                foreign,
            })
        }
        SurfaceDefinition::Function { declaration, .. } => {
            let mut parameters = Vec::new();
            for parameter in declaration.params() {
                let Some(ty_node) = child_of(parameter.syntax(), is_type_node) else {
                    continue;
                };
                let Some(parameter_ty) = parse_type(resolver, &ty_node) else {
                    continue;
                };
                let parameter_name = parameter.name().unwrap_or_default();
                let parameter_span = crate::resolve::token_span(parameter.syntax(), SyntaxKind::Identifier)
                    .unwrap_or_else(|| crate::resolve::trimmed_span(parameter.syntax()));
                parameters.push(RawParameter {
                    name: parameter_name,
                    ty: parameter_ty,
                    default: child_of(parameter.syntax(), is_expr_node),
                    span: parameter_span,
                });
            }
            let result_node = declaration
                .syntax()
                .children()
                .filter(|node| is_type_node(node.kind()))
                .last()?;
            let result = parse_type(resolver, &result_node)?;
            let body = child_of(declaration.syntax(), is_expr_node)?;
            let ty = Type::Function(
                parameters.iter().map(|parameter| parameter.ty.clone()).collect(),
                Box::new(result),
            );
            Some(RawDefinition {
                name,
                ty,
                kind: RawDefinitionKind::Function { parameters, body },
                name_span,
                span,
                foreign,
            })
        }
    }
}

fn function_result(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Function(_, result) => Some(result),
        Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::Duration
        | Type::Pitch
        | Type::Interval
        | Type::Product(_) => None,
    }
}

fn parse_type(resolver: &mut Resolver, node: &SyntaxNode) -> Option<Type> {
    let kind = node.kind();
    if kind == SyntaxKind::TypeExpr {
        return child_of(node, is_type_node).and_then(|child| parse_type(resolver, &child));
    }
    if kind == SyntaxKind::TypeName {
        let text = node.to_string();
        let text = text.trim();
        return match text {
            "unit" => Some(Type::Unit),
            "bool" => Some(Type::Bool),
            "nat" => Some(Type::Nat),
            "ratio" => Some(Type::Ratio),
            "duration" => Some(Type::Duration),
            "pitch" => Some(Type::Pitch),
            "interval" => Some(Type::Interval),
            "music" => {
                stage_error(resolver, node, "`music` values arrive in prompt 97");
                None
            }
            _ => {
                resolver.report(
                    Diagnostic::error(Code::UnknownName, format!("unknown type `{text}`"))
                        .at(crate::resolve::trimmed_span(node), "not a value type")
                        .help("use `bool`, `nat`, `ratio`, `duration`, `pitch`, `interval`, a product, or a function type"),
                );
                None
            }
        };
    }
    if kind == SyntaxKind::ProductType {
        let members: Option<Vec<_>> = node
            .children()
            .filter(|child| is_type_node(child.kind()))
            .map(|child| parse_type(resolver, &child))
            .collect();
        return members.map(Type::Product);
    }
    if kind == SyntaxKind::FunctionType {
        let mut parts = node.children().filter(|child| is_type_node(child.kind()));
        let parameter = parts.next().and_then(|part| parse_type(resolver, &part))?;
        let result = parts.next().and_then(|part| parse_type(resolver, &part))?;
        return Some(Type::Function(vec![parameter], Box::new(result)));
    }
    if matches!(kind, SyntaxKind::OptionType | SyntaxKind::ListType) {
        stage_error(resolver, node, "finite options and lists arrive in prompt 96");
    }
    None
}

struct Checker<'a> {
    resolver: &'a mut Resolver,
    definitions: &'a [RawDefinition],
    symbols: &'a IndexMap<String, Symbol>,
    locals: IndexMap<String, Type>,
    dependencies: IndexMap<String, SourceSpan>,
    foreign: bool,
    failed: bool,
}

impl Checker<'_> {
    fn check(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let kind = node.kind();
        let checked = if kind == SyntaxKind::ParenExpr {
            child_of(node, is_expr_node).and_then(|child| self.check(&child, expected))
        } else if kind == SyntaxKind::LiteralExpr {
            self.literal(node, expected)
        } else if kind == SyntaxKind::NameExpr {
            self.name(node)
        } else if kind == SyntaxKind::ProductExpr {
            self.product(node, expected)
        } else if kind == SyntaxKind::ApplyExpr {
            self.application(node)
        } else if matches!(
            kind,
            SyntaxKind::ListExpr | SyntaxKind::OptionExpr | SyntaxKind::MatchExpr
        ) {
            stage_error(self.resolver, node, "finite data elimination arrives in prompt 96");
            self.failed = true;
            None
        } else if kind == SyntaxKind::MusicExpr {
            stage_error(self.resolver, node, "contextual `music` values arrive in prompt 97");
            self.failed = true;
            None
        } else {
            None
        }?;
        if let Some(expected) = expected
            && checked.ty != *expected
        {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("expected `{expected}`, found `{}`", checked.ty),
                )
                .at(span, format!("this has type `{}`", checked.ty)),
            );
            self.failed = true;
            return None;
        }
        Some(checked)
    }

    fn literal(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let token = significant_tokens(node).next()?;
        let span = crate::resolve::trimmed_span(node);
        let kind = token.kind();
        let value = if kind == SyntaxKind::TrueKw {
            Value::Bool(true)
        } else if kind == SyntaxKind::FalseKw {
            Value::Bool(false)
        } else if kind == SyntaxKind::Integer && expected == Some(&Type::Duration) {
            Value::Duration(Ratio::from_integer(parse_i64(self.resolver, &token)?))
        } else if kind == SyntaxKind::Rational && expected == Some(&Type::Duration) {
            Value::Duration(parse_ratio(self.resolver, &token)?)
        } else if kind == SyntaxKind::Integer {
            Value::Nat(parse_u64(self.resolver, &token)?)
        } else if kind == SyntaxKind::Rational {
            Value::Ratio(parse_ratio(self.resolver, &token)?)
        } else if kind == SyntaxKind::PitchLiteral {
            Value::Pitch(WrittenPitch::parse(token.text()).or_else(|| {
                self.resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{}` is not a pitch", token.text()))
                        .at(span, "invalid pitch literal"),
                );
                None
            })?)
        } else if kind == SyntaxKind::IntervalLiteral {
            Value::Interval(Interval::parse(token.text(), false).or_else(|| {
                self.resolver.report(
                    Diagnostic::error(Code::NotAValue, format!("`{}` is not an interval", token.text()))
                        .at(span, "invalid interval literal"),
                );
                None
            })?)
        } else {
            return None;
        };
        Some(Expr {
            ty: value.ty(),
            kind: ExprKind::Literal(value),
            span,
        })
    }

    fn name(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let token = significant_tokens(node).find(|token| token.kind() == SyntaxKind::Identifier)?;
        let name = token.text().to_owned();
        let span = crate::resolve::trimmed_span(node);
        if let Some(ty) = self.locals.get(&name) {
            return Some(Expr {
                kind: ExprKind::Name(name),
                ty: ty.clone(),
                span,
            });
        }
        if matches!(name.as_str(), "nat_fold" | "list_fold" | "option_fold") {
            stage_error(self.resolver, node, "structural folds arrive in prompt 96");
            self.failed = true;
            return None;
        }
        let Some(symbol) = self.symbols.get(&name) else {
            self.resolver.report(
                Diagnostic::error(Code::UnknownName, format!("cannot find value `{name}`"))
                    .at(span, "nothing binds this name"),
            );
            self.failed = true;
            return None;
        };
        self.dependencies.entry(name.clone()).or_insert(span);
        if !self.foreign {
            self.resolver.references.record_use(symbol.kind, &name, span);
        }
        Some(Expr {
            kind: ExprKind::Name(name),
            ty: symbol.ty.clone(),
            span,
        })
    }

    fn product(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let children: Vec<_> = node.children().filter(|child| is_expr_node(child.kind())).collect();
        let expected_members = match expected {
            Some(Type::Product(members)) if members.len() == children.len() => Some(members.as_slice()),
            _ => None,
        };
        let members: Option<Vec<_>> = children
            .iter()
            .enumerate()
            .map(|(index, child)| self.check(child, expected_members.and_then(|members| members.get(index))))
            .collect();
        let members = members?;
        Some(Expr {
            ty: Type::Product(members.iter().map(|member| member.ty.clone()).collect()),
            kind: ExprKind::Product(members),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn application(&mut self, node: &SyntaxNode) -> Option<Expr> {
        let mut children = node.children();
        let function_node = children.find(|child| is_expr_node(child.kind()))?;
        let function = self.check(&function_node, None)?;
        let Type::Function(parameter_types, result) = function.ty.clone() else {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{}` is not callable", function.ty))
                    .at(function.span, "this is a value, not a function"),
            );
            self.failed = true;
            return None;
        };
        let raw_arguments = node
            .children()
            .find(|child| child.kind() == SyntaxKind::ExprArgList)
            .map_or_else(Vec::new, |list| {
                list.children()
                    .filter(|child| child.kind() == SyntaxKind::ExprArg)
                    .collect()
            });
        let parameter_shape = self.parameter_shape(&function_node, &parameter_types);
        let mut occupied = IndexSet::new();
        let mut positional = 0usize;
        let mut arguments = Vec::with_capacity(raw_arguments.len());
        for argument in &raw_arguments {
            let named = argument_name(argument);
            let parameter = if let Some(named) = named {
                let Some(index) = parameter_shape
                    .iter()
                    .position(|parameter| parameter.name.as_deref() == Some(&named))
                else {
                    self.resolver.report(
                        Diagnostic::error(Code::WrongArity, format!("this function has no parameter `{named}`"))
                            .at(crate::resolve::trimmed_span(argument), "unknown named argument"),
                    );
                    self.failed = true;
                    continue;
                };
                index
            } else {
                while occupied.contains(&positional) {
                    positional = positional.saturating_add(1);
                }
                let index = positional;
                positional = positional.saturating_add(1);
                index
            };
            if parameter >= parameter_types.len() || !occupied.insert(parameter) {
                self.resolver.report(
                    Diagnostic::error(Code::WrongArity, "this call supplies the wrong arguments")
                        .at(crate::resolve::trimmed_span(argument), "extra or repeated argument"),
                );
                self.failed = true;
                continue;
            }
            let value_node = child_of(argument, is_expr_node)?;
            let value = self.check(&value_node, parameter_types.get(parameter))?;
            arguments.push(CallArgument { parameter, value });
        }
        let missing: Vec<_> = parameter_shape
            .iter()
            .enumerate()
            .filter(|(index, parameter)| !occupied.contains(index) && !parameter.has_default)
            .collect();
        if !missing.is_empty() {
            let names = missing
                .iter()
                .map(|(index, parameter)| {
                    parameter
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("argument {}", index.saturating_add(1)))
                })
                .collect::<Vec<_>>()
                .join(", ");
            self.resolver.report(
                Diagnostic::error(Code::WrongArity, format!("this call is missing {names}"))
                    .at(crate::resolve::trimmed_span(node), "not enough arguments"),
            );
            self.failed = true;
            return None;
        }
        Some(Expr {
            kind: ExprKind::Apply {
                function: Box::new(function),
                arguments,
            },
            ty: result.as_ref().clone(),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn parameter_shape(&self, function: &SyntaxNode, types: &[Type]) -> Vec<ParameterShape> {
        let global = name_of(function)
            .and_then(|name| self.symbols.get(&name))
            .and_then(|symbol| self.definitions.get(symbol.definition));
        match global.map(|definition| &definition.kind) {
            Some(RawDefinitionKind::Function { parameters, .. }) => parameters
                .iter()
                .map(|parameter| ParameterShape {
                    name: Some(parameter.name.clone()),
                    has_default: parameter.default.is_some(),
                })
                .collect(),
            _ => types
                .iter()
                .map(|_| ParameterShape {
                    name: None,
                    has_default: false,
                })
                .collect(),
        }
    }
}

struct ParameterShape {
    name: Option<String>,
    has_default: bool,
}

fn dependency_order(resolver: &mut Resolver, definitions: &[CheckedDefinition]) -> Option<Vec<usize>> {
    let indices: IndexMap<_, _> = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| (definition.name.as_str(), index))
        .collect();
    let mut colors = vec![0u8; definitions.len()];
    let mut stack = Vec::new();
    let mut order = Vec::with_capacity(definitions.len());
    let mut acyclic = true;
    for index in 0..definitions.len() {
        visit(
            index,
            definitions,
            &indices,
            &mut colors,
            &mut stack,
            &mut order,
            resolver,
            &mut acyclic,
        );
    }
    acyclic.then_some(order)
}

#[allow(clippy::too_many_arguments)]
fn visit(
    index: usize,
    definitions: &[CheckedDefinition],
    indices: &IndexMap<&str, usize>,
    colors: &mut [u8],
    stack: &mut Vec<usize>,
    order: &mut Vec<usize>,
    resolver: &mut Resolver,
    acyclic: &mut bool,
) {
    match colors.get(index).copied() {
        Some(2) => return,
        Some(1) => {
            *acyclic = false;
            return;
        }
        _ => {}
    }
    if let Some(color) = colors.get_mut(index) {
        *color = 1;
    }
    stack.push(index);
    let Some(definition) = definitions.get(index) else {
        return;
    };
    for (dependency, use_span) in &definition.dependencies {
        let Some(dependency_index) = indices.get(dependency.as_str()).copied() else {
            continue;
        };
        if colors.get(dependency_index) == Some(&1) {
            let start = stack.iter().position(|member| *member == dependency_index).unwrap_or(0);
            let mut names: Vec<_> = stack
                .iter()
                .skip(start)
                .filter_map(|member| definitions.get(*member))
                .map(|member| member.name.as_str())
                .collect();
            names.push(dependency);
            let mut diagnostic = Diagnostic::error(Code::DependencyCycle, "these definitions call each other")
                .at(*use_span, "the cycle closes here")
                .note(format!("the cycle is {}", names.join(" → ")))
                .help("pass the changing value as a parameter, or replace the recursion with a finite fold");
            if let Some(target) = definitions.get(dependency_index)
                && !target.foreign
            {
                diagnostic = diagnostic.also(target.span, format!("`{dependency}` is defined here"));
            }
            resolver.report(diagnostic);
            *acyclic = false;
            continue;
        }
        visit(
            dependency_index,
            definitions,
            indices,
            colors,
            stack,
            order,
            resolver,
            acyclic,
        );
    }
    stack.pop();
    if let Some(color) = colors.get_mut(index) {
        *color = 2;
    }
    order.push(index);
}

fn evaluate(
    resolver: &mut Resolver,
    definitions: &[CheckedDefinition],
    order: &[usize],
) -> Option<IndexMap<String, Value>> {
    let mut values = IndexMap::new();
    for index in order {
        let definition = definitions.get(*index)?;
        let value = match &definition.kind {
            CheckedDefinitionKind::Let { body } => eval(body, &values),
            CheckedDefinitionKind::Function { parameters, body } => {
                let mut captures = IndexMap::new();
                for dependency in definition.dependencies.keys() {
                    captures.insert(dependency.clone(), values.get(dependency)?.clone());
                }
                let result = function_result(&definition.ty)?.clone();
                Some(Value::Closure(Box::new(Closure {
                    parameters: parameters.clone(),
                    result,
                    body: body.clone(),
                    captures,
                })))
            }
        };
        let Some(value) = value else {
            resolver.report(
                Diagnostic::error(Code::TypeMismatch, "this checked expression could not be evaluated")
                    .at(definition.span, "evaluation stopped here")
                    .note("this is a compiler invariant failure, not a recoverable language effect"),
            );
            return None;
        };
        if value.ty() != definition.ty {
            resolver.report(
                Diagnostic::error(Code::TypeMismatch, "evaluation changed this expression's type")
                    .at(definition.span, "the preservation invariant failed here"),
            );
            return None;
        }
        let _normalization_witness = value.normalization_witness();
        values.insert(definition.name.clone(), value);
    }
    Some(values)
}

fn eval(expression: &Expr, environment: &IndexMap<String, Value>) -> Option<Value> {
    let value = match &expression.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Name(name) => environment.get(name).cloned(),
        ExprKind::Product(members) => members
            .iter()
            .map(|member| eval(member, environment))
            .collect::<Option<Vec<_>>>()
            .map(Value::Product),
        ExprKind::Apply { function, arguments } => {
            let Value::Closure(closure) = eval(function, environment)? else {
                return None;
            };
            let mut provided = vec![None; closure.parameters.len()];
            for argument in arguments {
                let slot = provided.get_mut(argument.parameter)?;
                *slot = Some(eval(&argument.value, environment)?);
            }
            let mut local = closure.captures.clone();
            for (index, parameter) in closure.parameters.iter().enumerate() {
                let value = provided
                    .get_mut(index)
                    .and_then(Option::take)
                    .or_else(|| parameter.default.as_ref().and_then(|default| eval(default, &local)))?;
                if value.ty() != parameter.ty {
                    return None;
                }
                local.insert(parameter.name.clone(), value);
            }
            eval(&closure.body, &local)
        }
    }?;
    (value.ty() == expression.ty).then_some(value)
}

fn parse_i64(resolver: &mut Resolver, token: &SyntaxToken) -> Option<i64> {
    token.text().parse().ok().or_else(|| {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this number is too large")
                .at(token_span(token), "outside Musa's exact integer range"),
        );
        None
    })
}

fn parse_u64(resolver: &mut Resolver, token: &SyntaxToken) -> Option<u64> {
    token.text().parse().ok().or_else(|| {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this natural number is too large")
                .at(token_span(token), "outside Musa's exact natural range"),
        );
        None
    })
}

fn parse_ratio(resolver: &mut Resolver, token: &SyntaxToken) -> Option<Ratio<i64>> {
    let (numerator, denominator) = token.text().split_once('/')?;
    let (Ok(numerator), Ok(denominator)) = (numerator.parse::<i64>(), denominator.parse::<i64>()) else {
        resolver.report(
            Diagnostic::error(Code::OutOfRange, "this ratio is too large")
                .at(token_span(token), "outside Musa's exact rational range"),
        );
        return None;
    };
    if denominator == 0 {
        resolver.report(
            Diagnostic::error(Code::NotAValue, "a ratio cannot have zero below the line")
                .at(token_span(token), "division by zero"),
        );
        return None;
    }
    Some(Ratio::new(numerator, denominator))
}

fn stage_error(resolver: &mut Resolver, node: &SyntaxNode, help: &str) {
    resolver.report(
        Diagnostic::error(
            Code::UnsupportedLanguageStage,
            "this expression is valid Musa syntax but belongs to a later elaboration stage",
        )
        .at(crate::resolve::trimmed_span(node), "not available in the scalar core")
        .help(help),
    );
}

fn child_of(node: &SyntaxNode, predicate: fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
    node.children().find(|child| predicate(child.kind()))
}

fn is_type_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::TypeExpr
            | SyntaxKind::TypeName
            | SyntaxKind::FunctionType
            | SyntaxKind::ProductType
            | SyntaxKind::OptionType
            | SyntaxKind::ListType
    )
}

fn is_expr_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpr
            | SyntaxKind::LiteralExpr
            | SyntaxKind::ParenExpr
            | SyntaxKind::ProductExpr
            | SyntaxKind::ListExpr
            | SyntaxKind::OptionExpr
            | SyntaxKind::ApplyExpr
            | SyntaxKind::MatchExpr
            | SyntaxKind::MusicExpr
    )
}

fn significant_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + '_ {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
}

fn name_of(node: &SyntaxNode) -> Option<String> {
    if node.kind() != SyntaxKind::NameExpr {
        return None;
    }
    significant_tokens(node)
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

fn argument_name(node: &SyntaxNode) -> Option<String> {
    let direct: Vec<_> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| !token.kind().is_trivia())
        .collect();
    match direct.as_slice() {
        [name, colon, ..] if name.kind() == SyntaxKind::Identifier && colon.kind() == SyntaxKind::Colon => {
            Some(name.text().to_owned())
        }
        _ => None,
    }
}

fn token_span(token: &SyntaxToken) -> SourceSpan {
    SourceSpan::new(
        u32::from(token.text_range().start()),
        u32::from(token.text_range().end()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    enum ReferenceTerm {
        Nat(u64),
        Identity(Box<Self>),
    }

    impl ReferenceTerm {
        fn source(&self) -> String {
            match self {
                Self::Nat(value) => value.to_string(),
                Self::Identity(argument) => format!("id({})", argument.source()),
            }
        }

        fn evaluate(&self) -> u64 {
            match self {
                Self::Nat(value) => *value,
                Self::Identity(argument) => argument.evaluate(),
            }
        }
    }

    fn values(source: &str) -> Option<IndexMap<String, Value>> {
        let parsed = musa_language::parse(source);
        let piece = musa_language::ast::PieceDecl::from_root(&parsed.syntax())?;
        let mut resolver = Resolver::new();
        check_and_evaluate(&mut resolver, declarations(piece.syntax(), false).into_iter())
    }

    #[test]
    fn production_evaluation_agrees_with_a_small_generated_reference() {
        for depth in 0..8 {
            for value in 0..16u64 {
                let mut reference = ReferenceTerm::Nat(value);
                for _ in 0..depth {
                    reference = ReferenceTerm::Identity(Box::new(reference));
                }
                let applied = reference.source();
                let source = format!("piece \"law\" {{ fn id(x: nat) -> nat = x; let result: nat = {applied}; }}");
                let actual = values(&source);
                assert!(actual.is_some(), "generated well-typed source was rejected: {source}");
                assert!(matches!(
                    actual.as_ref().and_then(|bindings| bindings.get("result")),
                    Some(Value::Nat(found)) if *found == reference.evaluate()
                ));
            }
        }
    }

    #[test]
    fn every_evaluated_binding_has_its_checked_type() {
        let source = "piece \"law\" { let pair: (nat, bool) = (3, true); fn keep(x: (nat, bool)) -> (nat, bool) = x; let result: (nat, bool) = keep(pair); }";
        let actual = values(source);
        assert!(actual.is_some(), "well-typed source was rejected");
        assert!(matches!(
            actual.as_ref().and_then(|bindings| bindings.get("result")),
            Some(Value::Product(members))
                if matches!(members.as_slice(), [Value::Nat(3), Value::Bool(true)])
        ));
    }
}
