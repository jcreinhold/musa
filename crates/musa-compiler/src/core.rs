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

use crate::core_budget::WorkMeter;
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
    Option(Box<Self>),
    List(Box<Self>),
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
            Self::Option(member) => write!(out, "option[{member}]"),
            Self::List(member) => write!(out, "list[{member}]"),
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
    Option(Option<Box<Expr>>),
    List(Vec<Expr>),
    Apply {
        function: Box<Expr>,
        arguments: Vec<CallArgument>,
    },
    Primitive {
        primitive: Primitive,
        arguments: Vec<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<CheckedArm>,
    },
}

#[derive(Clone, Copy)]
enum Primitive {
    NatFold,
    ListFold,
    OptionFold,
    Map,
    Filter,
    Range,
    Repeat,
}

impl Primitive {
    fn name(self) -> &'static str {
        match self {
            Self::NatFold => "nat_fold",
            Self::ListFold => "list_fold",
            Self::OptionFold => "option_fold",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Range => "range",
            Self::Repeat => "repeat",
        }
    }
}

#[derive(Clone)]
struct CheckedArm {
    pattern: Pattern,
    body: Expr,
}

#[derive(Clone)]
enum Pattern {
    Wildcard,
    Bind(String),
    Literal(Value),
    None,
    Some(String),
    EmptyList,
    Cons { head: String, tail: String },
    Product(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Coverage {
    CatchAll,
    True,
    False,
    None,
    Some,
    EmptyList,
    Cons,
    Literal(String),
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
    Option { member: Type, value: Option<Box<Self>> },
    List { member: Type, values: Vec<Self> },
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
            Self::Option { member, .. } => Type::Option(Box::new(member.clone())),
            Self::List { member, .. } => Type::List(Box::new(member.clone())),
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
            Self::Option { value, .. } => value.as_deref().map_or(0, Self::normalization_witness).rotate_left(1),
            Self::List { values, .. } => values.iter().fold(0u64, |witness, value| {
                witness.rotate_left(5) ^ value.normalization_witness()
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
    let mut meter = WorkMeter::default();
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
            meter: &mut meter,
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
    if meter.exhaustion().is_some() {
        report_exhaustion(resolver, &meter);
        return None;
    }
    if type_errors || checked.len() != raw.len() {
        return None;
    }

    let order = dependency_order(resolver, &checked)?;
    evaluate(resolver, &checked, &order, &mut meter)
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
        | Type::Product(_)
        | Type::Option(_)
        | Type::List(_) => None,
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
        let member = child_of(node, is_type_node).and_then(|child| parse_type(resolver, &child))?;
        return if kind == SyntaxKind::OptionType {
            Some(Type::Option(Box::new(member)))
        } else {
            Some(Type::List(Box::new(member)))
        };
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
    meter: &'a mut WorkMeter,
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
        } else if kind == SyntaxKind::ListExpr {
            self.list(node, expected)
        } else if kind == SyntaxKind::OptionExpr {
            self.option(node, expected)
        } else if kind == SyntaxKind::ApplyExpr {
            self.application(node, expected)
        } else if kind == SyntaxKind::MatchExpr {
            self.match_expression(node, expected)
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
        let token = significant_tokens(node)
            .find(|token| matches!(token.kind(), SyntaxKind::Identifier | SyntaxKind::RepeatKw))?;
        let name = token.text().to_owned();
        let span = crate::resolve::trimmed_span(node);
        if let Some(ty) = self.locals.get(&name) {
            return Some(Expr {
                kind: ExprKind::Name(name),
                ty: ty.clone(),
                span,
            });
        }
        if primitive_named(&name).is_some() {
            self.resolver.report(
                Diagnostic::error(
                    Code::TypeMismatch,
                    format!("`{name}` is a compiler-owned polymorphic operation"),
                )
                .at(span, "apply it directly so Musa can choose one concrete type"),
            );
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

    fn list(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let expected_member = match expected {
            Some(Type::List(member)) => Some(member.as_ref()),
            _ => None,
        };
        let children: Vec<_> = node.children().filter(|child| is_expr_node(child.kind())).collect();
        if children.is_empty() && expected_member.is_none() {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, "the element type of this empty list is unknown")
                    .at(crate::resolve::trimmed_span(node), "add a `list[...]` type annotation"),
            );
            self.failed = true;
            return None;
        }
        let mut values = Vec::with_capacity(children.len());
        let mut member = expected_member.cloned();
        for child in &children {
            let value = self.check(child, member.as_ref())?;
            if member.is_none() {
                member = Some(value.ty.clone());
            }
            values.push(value);
        }
        let member = member?;
        Some(Expr {
            kind: ExprKind::List(values),
            ty: Type::List(Box::new(member)),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn option(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let expected_member = match expected {
            Some(Type::Option(member)) => Some(member.as_ref()),
            _ => None,
        };
        let some = significant_tokens(node).any(|token| token.kind() == SyntaxKind::SomeKw);
        let value = child_of(node, is_expr_node);
        if !some && expected_member.is_none() {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, "the value type of `none` is unknown").at(
                    crate::resolve::trimmed_span(node),
                    "add an `option[...]` type annotation",
                ),
            );
            self.failed = true;
            return None;
        }
        let checked = if let Some(value) = value.as_ref() {
            Some(Box::new(self.check(value, expected_member)?))
        } else {
            None
        };
        let member = expected_member
            .cloned()
            .or_else(|| checked.as_ref().map(|value| value.ty.clone()))?;
        Some(Expr {
            kind: ExprKind::Option(checked),
            ty: Type::Option(Box::new(member)),
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn match_expression(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let scrutinee_node = child_of(node, is_expr_node)?;
        let scrutinee = self.check(&scrutinee_node, None)?;
        let mut coverage = IndexSet::new();
        let mut catch_all = false;
        let mut result = expected.cloned();
        let mut arms = Vec::new();
        for arm in node.children().filter(|child| child.kind() == SyntaxKind::MatchArm) {
            let pattern_node = arm.children().find(|child| child.kind() == SyntaxKind::Pattern)?;
            let (pattern, covered, bindings) = self.check_pattern(&pattern_node, &scrutinee.ty)?;
            if catch_all || is_exhaustive(&scrutinee.ty, &coverage) || !coverage.insert(covered.clone()) {
                self.resolver.report(
                    Diagnostic::error(Code::UnreachablePattern, "this match arm can never be selected")
                        .at(crate::resolve::trimmed_span(&pattern_node), "already covered above"),
                );
                self.failed = true;
            }
            catch_all |= covered == Coverage::CatchAll;
            let saved = self.locals.clone();
            self.locals.extend(bindings);
            let body_node = child_of(&arm, is_expr_node)?;
            let body = self.check(&body_node, result.as_ref())?;
            self.locals = saved;
            if result.is_none() {
                result = Some(body.ty.clone());
            }
            arms.push(CheckedArm { pattern, body });
        }
        if !is_exhaustive(&scrutinee.ty, &coverage) {
            self.resolver.report(
                Diagnostic::error(Code::NonExhaustiveMatch, "this match leaves a possible value uncovered")
                    .at(
                        crate::resolve::trimmed_span(node),
                        "add the missing constructor or a `_` fallback",
                    )
                    .note(format!("the matched value has type `{}`", scrutinee.ty)),
            );
            self.failed = true;
            return None;
        }
        Some(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            ty: result?,
            span: crate::resolve::trimmed_span(node),
        })
    }

    fn check_pattern(
        &mut self,
        node: &SyntaxNode,
        target: &Type,
    ) -> Option<(Pattern, Coverage, IndexMap<String, Type>)> {
        let tokens: Vec<_> = significant_tokens(node).collect();
        let first = tokens.first()?;
        let span = crate::resolve::trimmed_span(node);
        let mut bindings = IndexMap::new();
        if first.kind() == SyntaxKind::Identifier {
            if first.text() == "_" {
                return Some((Pattern::Wildcard, Coverage::CatchAll, bindings));
            }
            bindings.insert(first.text().to_owned(), target.clone());
            return Some((Pattern::Bind(first.text().to_owned()), Coverage::CatchAll, bindings));
        }
        if first.kind() == SyntaxKind::NoneKw {
            if !matches!(target, Type::Option(_)) {
                return self.pattern_type_error(span, target, "`none` needs an option");
            }
            return Some((Pattern::None, Coverage::None, bindings));
        }
        if first.kind() == SyntaxKind::SomeKw {
            let Type::Option(member) = target else {
                return self.pattern_type_error(span, target, "`some` needs an option");
            };
            let name = tokens
                .iter()
                .find(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())?;
            bindings.insert(name.clone(), member.as_ref().clone());
            return Some((Pattern::Some(name), Coverage::Some, bindings));
        }
        if first.kind() == SyntaxKind::LBracket {
            let Type::List(member) = target else {
                return self.pattern_type_error(span, target, "a list pattern needs a list");
            };
            let names: Vec<_> = tokens
                .iter()
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())
                .collect();
            return match names.as_slice() {
                [] => Some((Pattern::EmptyList, Coverage::EmptyList, bindings)),
                [head, tail] => {
                    bindings.insert(head.clone(), member.as_ref().clone());
                    bindings.insert(tail.clone(), Type::List(member.clone()));
                    Some((
                        Pattern::Cons {
                            head: head.clone(),
                            tail: tail.clone(),
                        },
                        Coverage::Cons,
                        bindings,
                    ))
                }
                _ => self.pattern_type_error(span, target, "write `[]` or `[head, ..tail]`"),
            };
        }
        if first.kind() == SyntaxKind::LParen {
            let Type::Product(members) = target else {
                return self.pattern_type_error(span, target, "a product pattern needs a product");
            };
            let names: Vec<_> = tokens
                .iter()
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .map(|token| token.text().to_owned())
                .collect();
            if names.len() != members.len() {
                return self.pattern_type_error(span, target, "the product pattern has the wrong number of bindings");
            }
            for (name, member) in names.iter().zip(members) {
                if bindings.insert(name.clone(), member.clone()).is_some() {
                    self.resolver.report(
                        Diagnostic::error(Code::DuplicateName, format!("pattern binding `{name}` is repeated"))
                            .at(span, "bind each product member once"),
                    );
                    self.failed = true;
                    return None;
                }
            }
            return Some((Pattern::Product(names), Coverage::CatchAll, bindings));
        }
        let value = self.pattern_literal(first, target, span)?;
        let covered = match value {
            Value::Bool(true) => Coverage::True,
            Value::Bool(false) => Coverage::False,
            Value::Nat(_)
            | Value::Ratio(_)
            | Value::Duration(_)
            | Value::Pitch(_)
            | Value::Interval(_)
            | Value::Product(_)
            | Value::Option { .. }
            | Value::List { .. }
            | Value::Closure(_) => Coverage::Literal(literal_key(&value)),
        };
        Some((Pattern::Literal(value), covered, bindings))
    }

    fn pattern_literal(&mut self, token: &SyntaxToken, target: &Type, span: SourceSpan) -> Option<Value> {
        let value = match (token.kind(), target) {
            (SyntaxKind::TrueKw, Type::Bool) => Value::Bool(true),
            (SyntaxKind::FalseKw, Type::Bool) => Value::Bool(false),
            (SyntaxKind::Integer, Type::Nat) => Value::Nat(parse_u64(self.resolver, token)?),
            (SyntaxKind::Integer, Type::Duration) => {
                Value::Duration(Ratio::from_integer(parse_i64(self.resolver, token)?))
            }
            (SyntaxKind::Rational, Type::Ratio) => Value::Ratio(parse_ratio(self.resolver, token)?),
            (SyntaxKind::Rational, Type::Duration) => Value::Duration(parse_ratio(self.resolver, token)?),
            (SyntaxKind::PitchLiteral, Type::Pitch) => Value::Pitch(WrittenPitch::parse(token.text())?),
            (SyntaxKind::IntervalLiteral, Type::Interval) => Value::Interval(Interval::parse(token.text(), false)?),
            _ => return self.pattern_type_error(span, target, "this literal cannot match that value type"),
        };
        Some(value)
    }

    fn pattern_type_error<T>(&mut self, span: SourceSpan, target: &Type, message: &str) -> Option<T> {
        self.resolver.report(
            Diagnostic::error(Code::TypeMismatch, message).at(span, format!("the matched value has type `{target}`")),
        );
        self.failed = true;
        None
    }

    fn application(&mut self, node: &SyntaxNode, expected: Option<&Type>) -> Option<Expr> {
        let mut children = node.children();
        let function_node = children.find(|child| is_expr_node(child.kind()))?;
        if let Some(primitive) = name_of(&function_node).as_deref().and_then(primitive_named) {
            return self.primitive_application(node, primitive, expected);
        }
        let function = self.check(&function_node, None)?;
        let Type::Function(parameter_types, result) = function.ty.clone() else {
            self.resolver.report(
                Diagnostic::error(Code::TypeMismatch, format!("`{}` is not callable", function.ty))
                    .at(function.span, "this is a value, not a function"),
            );
            self.failed = true;
            return None;
        };
        let raw_arguments = raw_arguments(node);
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

    fn primitive_application(
        &mut self,
        node: &SyntaxNode,
        primitive: Primitive,
        expected: Option<&Type>,
    ) -> Option<Expr> {
        let span = crate::resolve::trimmed_span(node);
        let raw = raw_arguments(node);
        if raw.iter().any(|argument| argument_name(argument).is_some()) {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` uses positional arguments", primitive.name()),
                )
                .at(span, "named arguments are not part of this prelude operation"),
            );
            self.failed = true;
            return None;
        }
        let wanted = match primitive {
            Primitive::NatFold | Primitive::ListFold | Primitive::OptionFold => 3,
            Primitive::Map | Primitive::Filter | Primitive::Repeat => 2,
            Primitive::Range => 1,
        };
        if raw.len() != wanted {
            self.resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{}` takes {wanted} arguments, found {}", primitive.name(), raw.len()),
                )
                .at(span, "wrong number of arguments"),
            );
            self.failed = true;
            return None;
        }
        if !self.meter.instantiate(primitive.name(), span) {
            self.failed = true;
            return None;
        }
        let nodes: Vec<_> = raw
            .iter()
            .filter_map(|argument| child_of(argument, is_expr_node))
            .collect();
        let (arguments, ty) = match primitive {
            Primitive::Range => {
                let count = self.check(nodes.first()?, Some(&Type::Nat))?;
                (vec![count], Type::List(Box::new(Type::Nat)))
            }
            Primitive::Repeat => {
                let value = self.check(nodes.first()?, None)?;
                let count = self.check(nodes.get(1)?, Some(&Type::Nat))?;
                let result = Type::List(Box::new(value.ty.clone()));
                (vec![value, count], result)
            }
            Primitive::Map => {
                let function = self.check(nodes.first()?, None)?;
                let Type::Function(parameters, result) = function.ty.clone() else {
                    return self.primitive_type_error(span, "`map` first needs a one-argument function");
                };
                let Some(parameter) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(span, "`map` first needs a one-argument function");
                };
                let values = self.check(nodes.get(1)?, Some(&Type::List(Box::new(parameter.clone()))))?;
                (vec![function, values], Type::List(result))
            }
            Primitive::Filter => {
                let function = self.check(nodes.first()?, None)?;
                let Type::Function(parameters, result) = function.ty.clone() else {
                    return self.primitive_type_error(span, "`filter` first needs a predicate");
                };
                let Some(parameter) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(span, "`filter` first needs a one-argument predicate");
                };
                if result.as_ref() != &Type::Bool {
                    return self.primitive_type_error(span, "a `filter` predicate must return `bool`");
                }
                let values = self.check(nodes.get(1)?, Some(&Type::List(Box::new(parameter.clone()))))?;
                (vec![function, values], Type::List(Box::new(parameter.clone())))
            }
            Primitive::NatFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let step = self.check(nodes.get(1)?, None)?;
                let wanted = Type::Function(vec![Type::Nat, zero.ty.clone()], Box::new(zero.ty.clone()));
                if step.ty != wanted {
                    return self.primitive_type_error(
                        step.span,
                        "a `nat_fold` step must accept the index and accumulator and return the accumulator type",
                    );
                }
                let count = self.check(nodes.get(2)?, Some(&Type::Nat))?;
                let result = zero.ty.clone();
                (vec![zero, step, count], result)
            }
            Primitive::ListFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let step = self.check(nodes.get(1)?, None)?;
                let Type::Function(parameters, result) = step.ty.clone() else {
                    return self.primitive_type_error(step.span, "a `list_fold` step must be a two-argument function");
                };
                let Some((member, accumulator)) = binary_parameters(&parameters) else {
                    return self.primitive_type_error(step.span, "a `list_fold` step must be a two-argument function");
                };
                if accumulator != &zero.ty || result.as_ref() != &zero.ty {
                    return self
                        .primitive_type_error(step.span, "a `list_fold` step must preserve the accumulator type");
                }
                let values = self.check(nodes.get(2)?, Some(&Type::List(Box::new(member.clone()))))?;
                let result = zero.ty.clone();
                (vec![zero, step, values], result)
            }
            Primitive::OptionFold => {
                let zero = self.check(nodes.first()?, expected)?;
                let some_case = self.check(nodes.get(1)?, None)?;
                let Type::Function(parameters, result) = some_case.ty.clone() else {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must be a one-argument function",
                    );
                };
                let Some(member) = unary_parameter(&parameters) else {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must be a one-argument function",
                    );
                };
                if result.as_ref() != &zero.ty {
                    return self.primitive_type_error(
                        some_case.span,
                        "an `option_fold` some-case must return the zero value's type",
                    );
                }
                let value = self.check(nodes.get(2)?, Some(&Type::Option(Box::new(member.clone()))))?;
                let result = zero.ty.clone();
                (vec![zero, some_case, value], result)
            }
        };
        Some(Expr {
            kind: ExprKind::Primitive { primitive, arguments },
            ty,
            span,
        })
    }

    fn primitive_type_error<T>(&mut self, span: SourceSpan, message: &str) -> Option<T> {
        self.resolver
            .report(Diagnostic::error(Code::TypeMismatch, message).at(span, "invalid prelude arguments"));
        self.failed = true;
        None
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

fn primitive_named(name: &str) -> Option<Primitive> {
    match name {
        "nat_fold" => Some(Primitive::NatFold),
        "list_fold" => Some(Primitive::ListFold),
        "option_fold" => Some(Primitive::OptionFold),
        "map" => Some(Primitive::Map),
        "filter" => Some(Primitive::Filter),
        "range" => Some(Primitive::Range),
        "repeat" => Some(Primitive::Repeat),
        _ => None,
    }
}

fn unary_parameter(parameters: &[Type]) -> Option<&Type> {
    match parameters {
        [parameter] => Some(parameter),
        _ => None,
    }
}

fn binary_parameters(parameters: &[Type]) -> Option<(&Type, &Type)> {
    match parameters {
        [first, second] => Some((first, second)),
        _ => None,
    }
}

fn raw_arguments(node: &SyntaxNode) -> Vec<SyntaxNode> {
    node.children()
        .find(|child| child.kind() == SyntaxKind::ExprArgList)
        .map_or_else(Vec::new, |list| {
            list.children()
                .filter(|child| child.kind() == SyntaxKind::ExprArg)
                .collect()
        })
}

fn is_exhaustive(target: &Type, coverage: &IndexSet<Coverage>) -> bool {
    coverage.contains(&Coverage::CatchAll)
        || match target {
            Type::Bool => coverage.contains(&Coverage::True) && coverage.contains(&Coverage::False),
            Type::Option(_) => coverage.contains(&Coverage::None) && coverage.contains(&Coverage::Some),
            Type::List(_) => coverage.contains(&Coverage::EmptyList) && coverage.contains(&Coverage::Cons),
            Type::Unit
            | Type::Nat
            | Type::Ratio
            | Type::Duration
            | Type::Pitch
            | Type::Interval
            | Type::Product(_)
            | Type::Function(_, _) => false,
        }
}

fn literal_key(value: &Value) -> String {
    match value {
        Value::Bool(value) => format!("bool:{value}"),
        Value::Nat(value) => format!("nat:{value}"),
        Value::Ratio(value) => format!("ratio:{}/{}", value.numer(), value.denom()),
        Value::Duration(value) => format!("duration:{}/{}", value.numer(), value.denom()),
        Value::Pitch(value) => format!("pitch:{}:{}:{}", value.letter.steps(), value.accidental.0, value.octave),
        Value::Interval(value) => format!("interval:{}:{}", value.diatonic_steps, value.semitones),
        Value::Product(_) | Value::Option { .. } | Value::List { .. } | Value::Closure(_) => "constructor".to_owned(),
    }
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
    meter: &mut WorkMeter,
) -> Option<IndexMap<String, Value>> {
    let mut values = IndexMap::new();
    for index in order {
        let definition = definitions.get(*index)?;
        if !meter.output("scalar elaboration", 0, definition.span) {
            report_exhaustion(resolver, meter);
            return None;
        }
        let value = match &definition.kind {
            CheckedDefinitionKind::Let { body } => eval(body, &values, meter),
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
            if meter.exhaustion().is_some() {
                report_exhaustion(resolver, meter);
                return None;
            }
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

fn eval(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<Value> {
    if !meter.step("expression evaluation", 1, expression.span) {
        return None;
    }
    let value = match &expression.kind {
        ExprKind::Literal(value) => Some(value.clone()),
        ExprKind::Name(name) => environment.get(name).cloned(),
        ExprKind::Product(members) => members
            .iter()
            .map(|member| eval(member, environment, meter))
            .collect::<Option<Vec<_>>>()
            .map(Value::Product),
        ExprKind::Option(value) => {
            let Type::Option(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            let value = if let Some(value) = value.as_deref() {
                Some(Box::new(eval(value, environment, meter)?))
            } else {
                None
            };
            Some(Value::Option { member, value })
        }
        ExprKind::List(values) => {
            let Type::List(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            let values = values
                .iter()
                .map(|value| eval(value, environment, meter))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::List { member, values })
        }
        ExprKind::Apply { function, arguments } => {
            let Value::Closure(closure) = eval(function, environment, meter)? else {
                return None;
            };
            let mut provided = vec![None; closure.parameters.len()];
            for argument in arguments {
                let slot = provided.get_mut(argument.parameter)?;
                *slot = Some(eval(&argument.value, environment, meter)?);
            }
            apply_closure(&closure, provided, meter, expression.span)
        }
        ExprKind::Primitive { primitive, arguments } => {
            eval_primitive(*primitive, arguments, environment, meter, expression)
        }
        ExprKind::Match { scrutinee, arms } => {
            let value = eval(scrutinee, environment, meter)?;
            let mut selected = None;
            for arm in arms {
                if !meter.step("match arm", 1, expression.span) {
                    return None;
                }
                if let Some(bindings) = match_pattern(&arm.pattern, &value) {
                    selected = Some((arm, bindings));
                    break;
                }
            }
            let (arm, bindings) = selected?;
            let mut local = environment.clone();
            local.extend(bindings);
            eval(&arm.body, &local, meter)
        }
    }?;
    let (nodes, bytes) = value_shape(&value);
    if value.ty() != expression.ty || !meter.construct("expression value", nodes, bytes, expression.span) {
        return None;
    }
    Some(value)
}

fn apply_closure(
    closure: &Closure,
    mut provided: Vec<Option<Value>>,
    meter: &mut WorkMeter,
    span: SourceSpan,
) -> Option<Value> {
    if !meter.step("function application", 1, span) {
        return None;
    }
    let mut local = closure.captures.clone();
    for (index, parameter) in closure.parameters.iter().enumerate() {
        let value = provided.get_mut(index).and_then(Option::take).or_else(|| {
            parameter
                .default
                .as_ref()
                .and_then(|default| eval(default, &local, meter))
        })?;
        if value.ty() != parameter.ty {
            return None;
        }
        local.insert(parameter.name.clone(), value);
    }
    eval(&closure.body, &local, meter)
}

fn eval_primitive(
    primitive: Primitive,
    arguments: &[Expr],
    environment: &IndexMap<String, Value>,
    meter: &mut WorkMeter,
    expression: &Expr,
) -> Option<Value> {
    let values = arguments
        .iter()
        .map(|argument| eval(argument, environment, meter))
        .collect::<Option<Vec<_>>>()?;
    match primitive {
        Primitive::Range => {
            let count = nat_value(values.first()?)?;
            let nodes = count.saturating_add(1);
            let bytes = count.saturating_mul(8);
            if !meter.step("range", count, expression.span)
                || !meter.preflight_construct("range", nodes, bytes, expression.span)
            {
                return None;
            }
            let capacity = usize::try_from(count).ok()?;
            let values = (0..count).map(Value::Nat).collect::<Vec<_>>();
            if values.len() != capacity {
                return None;
            }
            Some(Value::List {
                member: Type::Nat,
                values,
            })
        }
        Primitive::Repeat => {
            let value = values.first()?.clone();
            let count = nat_value(values.get(1)?)?;
            let (value_nodes, value_bytes) = value_shape(&value);
            let nodes = value_nodes.saturating_mul(count).saturating_add(1);
            let bytes = value_bytes.saturating_mul(count);
            if !meter.step("repeat", count, expression.span)
                || !meter.preflight_construct("repeat", nodes, bytes, expression.span)
            {
                return None;
            }
            let count = usize::try_from(count).ok()?;
            Some(Value::List {
                member: value.ty(),
                values: vec![value; count],
            })
        }
        Primitive::Map => {
            let Value::Closure(function) = values.first()? else {
                return None;
            };
            let Value::List { values: source, .. } = values.get(1)? else {
                return None;
            };
            if !meter.step("map", u64::try_from(source.len()).unwrap_or(u64::MAX), expression.span) {
                return None;
            }
            let mapped = source
                .iter()
                .cloned()
                .map(|value| apply_closure(function, vec![Some(value)], meter, expression.span))
                .collect::<Option<Vec<_>>>()?;
            let Type::List(member) = &expression.ty else {
                return None;
            };
            let member = member.as_ref().clone();
            Some(Value::List { member, values: mapped })
        }
        Primitive::Filter => {
            let Value::Closure(predicate) = values.first()? else {
                return None;
            };
            let Value::List { member, values } = values.get(1)? else {
                return None;
            };
            if !meter.step(
                "filter",
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            ) {
                return None;
            }
            let mut kept = Vec::new();
            for value in values {
                let decision = apply_closure(predicate, vec![Some(value.clone())], meter, expression.span)?;
                let Value::Bool(keep) = decision else {
                    return None;
                };
                if keep {
                    kept.push(value.clone());
                }
            }
            Some(Value::List {
                member: member.clone(),
                values: kept,
            })
        }
        Primitive::NatFold => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let count = nat_value(values.get(2)?)?;
            if !meter.step("nat_fold", count, expression.span) {
                return None;
            }
            for index in 0..count {
                accumulator = apply_closure(
                    step,
                    vec![Some(Value::Nat(index)), Some(accumulator)],
                    meter,
                    expression.span,
                )?;
            }
            Some(accumulator)
        }
        Primitive::ListFold => {
            let mut accumulator = values.first()?.clone();
            let Value::Closure(step) = values.get(1)? else {
                return None;
            };
            let Value::List { values, .. } = values.get(2)? else {
                return None;
            };
            if !meter.step(
                "list_fold",
                u64::try_from(values.len()).unwrap_or(u64::MAX),
                expression.span,
            ) {
                return None;
            }
            for value in values {
                accumulator = apply_closure(
                    step,
                    vec![Some(value.clone()), Some(accumulator)],
                    meter,
                    expression.span,
                )?;
            }
            Some(accumulator)
        }
        Primitive::OptionFold => {
            let zero = values.first()?.clone();
            let Value::Closure(some_case) = values.get(1)? else {
                return None;
            };
            let Value::Option { value, .. } = values.get(2)? else {
                return None;
            };
            match value {
                Some(value) => {
                    if !meter.step("option_fold", 1, expression.span) {
                        return None;
                    }
                    apply_closure(some_case, vec![Some(value.as_ref().clone())], meter, expression.span)
                }
                None => Some(zero),
            }
        }
    }
}

fn nat_value(value: &Value) -> Option<u64> {
    if let Value::Nat(value) = value {
        Some(*value)
    } else {
        None
    }
}

fn match_pattern(pattern: &Pattern, value: &Value) -> Option<IndexMap<String, Value>> {
    let mut bindings = IndexMap::new();
    let matched = match pattern {
        Pattern::Wildcard => true,
        Pattern::Bind(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        Pattern::Literal(expected) => literal_values_equal(expected, value),
        Pattern::None => matches!(value, Value::Option { value: None, .. }),
        Pattern::Some(name) => {
            if let Value::Option {
                value: Some(member), ..
            } = value
            {
                bindings.insert(name.clone(), member.as_ref().clone());
                true
            } else {
                false
            }
        }
        Pattern::EmptyList => matches!(value, Value::List { values, .. } if values.is_empty()),
        Pattern::Cons { head, tail } => {
            if let Value::List { member, values } = value
                && !values.is_empty()
            {
                let first = values.first()?.clone();
                let rest = values.get(1..)?.to_vec();
                bindings.insert(head.clone(), first);
                bindings.insert(
                    tail.clone(),
                    Value::List {
                        member: member.clone(),
                        values: rest,
                    },
                );
                true
            } else {
                false
            }
        }
        Pattern::Product(names) => {
            if let Value::Product(members) = value
                && names.len() == members.len()
            {
                bindings.extend(names.iter().cloned().zip(members.iter().cloned()));
                true
            } else {
                false
            }
        }
    };
    matched.then_some(bindings)
}

fn literal_values_equal(left: &Value, right: &Value) -> bool {
    if let (Value::Bool(left), Value::Bool(right)) = (left, right) {
        left == right
    } else if let (Value::Nat(left), Value::Nat(right)) = (left, right) {
        left == right
    } else if let (Value::Ratio(left), Value::Ratio(right)) = (left, right) {
        left == right
    } else if let (Value::Duration(left), Value::Duration(right)) = (left, right) {
        left == right
    } else if let (Value::Pitch(left), Value::Pitch(right)) = (left, right) {
        left == right
    } else if let (Value::Interval(left), Value::Interval(right)) = (left, right) {
        left == right
    } else {
        false
    }
}

fn value_shape(value: &Value) -> (u64, u64) {
    match value {
        Value::Bool(_) => (1, 1),
        Value::Nat(_) => (1, 8),
        Value::Ratio(_) | Value::Duration(_) => (1, 16),
        Value::Pitch(_) | Value::Interval(_) => (1, 12),
        Value::Product(members) => aggregate_shape(members.iter()),
        Value::Option { value, .. } => value.as_deref().map_or((1, 1), |value| {
            let (nodes, bytes) = value_shape(value);
            (nodes.saturating_add(1), bytes.saturating_add(1))
        }),
        Value::List { values, .. } => aggregate_shape(values.iter()),
        Value::Closure(closure) => aggregate_shape(closure.captures.values()),
    }
}

fn aggregate_shape<'a>(values: impl Iterator<Item = &'a Value>) -> (u64, u64) {
    values.fold((1u64, 0u64), |(nodes, bytes), value| {
        let (value_nodes, value_bytes) = value_shape(value);
        (nodes.saturating_add(value_nodes), bytes.saturating_add(value_bytes))
    })
}

fn report_exhaustion(resolver: &mut Resolver, meter: &WorkMeter) {
    let Some(exhaustion) = meter.exhaustion() else {
        return;
    };
    resolver.report(
        Diagnostic::error(
            Code::ResourceLimit,
            format!("`{}` exceeds the compilation budget", exhaustion.operation),
        )
        .at(
            exhaustion.span,
            format!(
                "attempted {} {}, limit {}",
                exhaustion.attempted, exhaustion.metric, exhaustion.limit
            ),
        )
        .note("the expression is finite; Musa rejected its size before publishing a partial value")
        .help("reduce the bound or split the generated material into smaller declarations"),
    );
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
        .find(|token| matches!(token.kind(), SyntaxKind::Identifier | SyntaxKind::RepeatKw))
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

    #[test]
    fn finite_primitives_agree_with_small_reference_folds() {
        for count in 0..16u64 {
            let source = format!(
                "piece \"law\" {{ \
                 fn latest(index: nat, accumulator: nat) -> nat = index; \
                 fn item(value: nat, accumulator: nat) -> nat = value; \
                 fn id(value: nat) -> nat = value; \
                 fn reject(value: nat) -> bool = false; \
                 fn from_option(value: option[nat]) -> nat = match value {{ none -> 0, some(found) -> found }}; \
                 let by_nat: nat = nat_fold(0, latest, {count}); \
                 let values: list[nat] = range({count}); \
                 let mapped: list[nat] = map(id, values); \
                 let filtered: list[nat] = filter(reject, mapped); \
                 let by_list: nat = list_fold(0, item, mapped); \
                 let selected: nat = from_option(some(by_list)); \
                 }}"
            );
            let actual = values(&source);
            assert!(actual.is_some(), "generated finite source was rejected: {source}");
            let expected = (0..count).last().unwrap_or(0);
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("by_nat")),
                Some(Value::Nat(found)) if *found == expected
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("by_list")),
                Some(Value::Nat(found)) if *found == expected
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("mapped")),
                Some(Value::List { values, .. })
                    if values.iter().enumerate().all(|(index, value)| {
                        matches!(value, Value::Nat(found) if usize::try_from(*found).ok() == Some(index))
                    })
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("filtered")),
                Some(Value::List { values, .. }) if values.is_empty()
            ));
            assert!(matches!(
                actual.as_ref().and_then(|bindings| bindings.get("selected")),
                Some(Value::Nat(found)) if *found == expected
            ));
        }
    }
}
