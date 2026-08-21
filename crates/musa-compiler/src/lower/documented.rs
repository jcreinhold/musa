//! What a reader is told about a declaration, stated where it is read.
//!
//! One record per declaration ([`crate::docs::ItemDoc`]), and hover, completion,
//! signature help, the outline, and `stdlib/reference.md` are all readers of it.
//!
//! # Why the signature is the author's own words
//!
//! Because a signature is a thing a reader *writes*. `fn walked(items: StaffItem)
//! -> (Position<WrittenTime> -> Result<Realization, Text>)` is what the file
//! says, and a reader who copies it back out has written a declaration that
//! compiles. The elaborated type is the same type spelled in the core's
//! vocabulary — a base type applied rather than in angle brackets, an arrow with
//! a binder name, `Machine K A B` where the surface writes `Machine<K, A, B>`
//! (`02-core-calculus.md` §7) — and printing that back would be a second
//! translation, out of the surface and then guessing its way back in.
//!
//! This is not the line-scanning generator prompt 127 removed. That one read the
//! *text* of a file it did not otherwise compile, so it published a function's
//! body as its signature and truncated a declaration whose parameters wrapped.
//! What reads here is the declaration's own tree, in the walk that lowers it, so
//! a parameter is a parameter node and a result type is the type child — the two
//! things a scanner could not tell apart.
//!
//! # And where the author wrote nothing
//!
//! `let held = unchanged(c4);` has a type and did not write one, so the type is
//! asked for. [`spelled`] answers it in the surface's spelling or answers
//! nothing: a shape with no written form is left unsaid rather than shown in a
//! vocabulary the reader cannot write back.

use musa_syntax::{SyntaxKind, SyntaxNode};

use super::{child, children, is_type_node};
use crate::docs::{ItemDoc, ItemSource, ParameterDoc, TypeNote};
use crate::resolve::NameKind;

/// What a reader is told about the declaration at `node`, bound as `name`.
///
/// `uri` is the document it is written in, `None` for the one being compiled;
/// `inferred` is asked only where the declaration wrote no type, and only for a
/// declaration that names a value.
///
/// [`None`] for a node that is not a declaration this records — a `data`, a
/// `trait`, an `impl`, and an `import` are not values and are documented by
/// whoever declares them, and a `signature` and a `structure` are
/// `crate::module`'s.
pub(crate) fn documented(
    node: &SyntaxNode,
    name: &str,
    uri: Option<&str>,
    inferred: impl FnOnce() -> Option<String>,
) -> Option<ItemDoc> {
    let (word, kind) = declaring(node.kind())?;
    let parameters = parameters(node);
    // The written type where one was written, and otherwise the one elaboration
    // settled on — asked for only here, because a declaration that names no
    // value has no type to state and a written one is already the answer.
    let result = states_a_result(kind)
        .then(|| written_result(node).or_else(|| inferred().map(TypeNote::new)))
        .flatten();
    let mut signature = format!("{word} {name}{}", type_parameters(node));
    // The empty parameter list is written for a `fn` that has one, because a
    // nullary `fn` is *called* with one: a reader shown `let do_re_mi_strong:
    // List<Bool>` would write the name bare and be told it is a function.
    if node.kind() == SyntaxKind::FnDecl || !parameters.is_empty() {
        signature.push('(');
        for (position, parameter) in parameters.iter().enumerate() {
            if position > 0 {
                signature.push_str(", ");
            }
            signature.push_str(&parameter.label);
        }
        signature.push(')');
    }
    // A motif, a fragment, and a bar are music by construction, so saying so
    // would add a word to every line without adding a fact — which is why
    // `result` is already empty for them.
    if let Some(result) = result.as_ref() {
        signature.push_str(if node.kind() == SyntaxKind::FnDecl || !parameters.is_empty() {
            " -> "
        } else {
            ": "
        });
        signature.push_str(&result.name);
    }
    signature.push_str(&constraints(node));
    let summary = crate::docs::summary_above(node);
    Some(ItemDoc {
        name: name.to_owned(),
        kind,
        source: ItemSource {
            uri: uri.map(str::to_owned),
            span: crate::resolve::token_span(node, SyntaxKind::Identifier)
                .unwrap_or_else(|| crate::resolve::trimmed_span(node)),
            read_only: uri.is_some_and(|uri| crate::imports::standard_library_source(uri).is_some()),
        },
        deprecation: summary.as_deref().and_then(crate::docs::deprecation_in),
        summary,
        result,
        parameters,
        signature,
    })
}

/// The word a reader writes to declare this, and what the name names.
///
/// The two travel together because they answer one question asked twice, and
/// the list is the declarations that reach [`super::items::Item::Definition`] —
/// every one of them binds a name a later expression may write.
fn declaring(kind: SyntaxKind) -> Option<(&'static str, NameKind)> {
    match kind {
        SyntaxKind::FnDecl => Some(("fn", NameKind::Function)),
        SyntaxKind::LetDecl => Some(("let", NameKind::Value)),
        SyntaxKind::RecordDecl => Some(("record", NameKind::Value)),
        SyntaxKind::MotifDecl => Some(("motif", NameKind::Motif)),
        SyntaxKind::FragmentDecl => Some(("fragment", NameKind::Fragment)),
        SyntaxKind::BarStmt => Some(("bar", NameKind::Bar)),
        _ => None,
    }
}

/// Whether this kind of declaration names a value whose type is worth stating.
///
/// A `record` declares a *type*, so it has no result for the same reason
/// `crate::module`'s `signature` has none: what it names is not a value. A
/// motif, a fragment, and a bar name music and say so by being one.
fn states_a_result(kind: NameKind) -> bool {
    matches!(kind, NameKind::Value | NameKind::Function)
}

/// `<A, B>`, or nothing where none were written.
fn type_parameters(node: &SyntaxNode) -> String {
    let Some(list) = child(node, |kind| kind == SyntaxKind::TypeParams) else {
        return String::new();
    };
    let written: Vec<String> = children(&list, |kind| kind == SyntaxKind::TypeParam)
        .iter()
        .map(collapsed)
        .collect();
    if written.is_empty() {
        String::new()
    } else {
        format!("<{}>", written.join(", "))
    }
}

/// ` where Eq<A>`, or nothing where no clause was written.
///
/// Part of the signature because it is part of what a caller must supply: a
/// reader who copies the line and leaves the clause off has written a
/// declaration this one is not.
fn constraints(node: &SyntaxNode) -> String {
    child(node, |kind| kind == SyntaxKind::WhereClause)
        .map(|clause| format!(" {}", collapsed(&clause)))
        .unwrap_or_default()
}

/// The parameters this declaration wrote, in order.
///
/// A parameter with no written type is labelled by its name alone, which is what
/// the line says. Elaboration refuses a binder whose type nothing determines
/// (`02-core-calculus.md` §2.1), so the record a reader ends up holding is
/// either complete or attached to a declaration that was refused.
fn parameters(node: &SyntaxNode) -> Vec<ParameterDoc> {
    super::items::written_parameters(node)
        .iter()
        .filter_map(|parameter| {
            let name = crate::resolve::token_text(parameter, SyntaxKind::Identifier)?;
            let ty = written_result(parameter);
            Some(ParameterDoc {
                label: ty
                    .as_ref()
                    .map_or_else(|| name.clone(), |ty| format!("{name}: {}", ty.name)),
                name,
                ty: ty.unwrap_or_else(|| TypeNote::new(String::new())),
            })
        })
        .collect()
}

/// The type child of `node`, as the file spells it.
fn written_result(node: &SyntaxNode) -> Option<TypeNote> {
    Some(TypeNote::new(collapsed(&child(node, is_type_node)?)))
}

/// A node's own text on one line.
///
/// A written type may wrap, and a signature is a line. Runs of whitespace become
/// one space rather than being deleted, so `A -> B` stays two words apart.
fn collapsed(node: &SyntaxNode) -> String {
    node.to_string().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How the surface spells the type `ty`, when it has a spelling.
///
/// [`None`] rather than the core's own spelling wherever the two vocabularies
/// differ in shape, because this fills a *hover*: a reader shown
/// `(x : Nat) → Nat` has been shown a type they cannot write, and a reader shown
/// nothing has been shown that the declaration wrote none. The shapes that do
/// have one are the ones a `let` without an annotation ends up at — a base type,
/// a declared family, a definition standing for a type, each with its arguments
/// in angle brackets, and the arrow that declares no parameter.
pub(crate) fn spelled(ty: &musa_calculus::Term) -> Option<String> {
    let mut head = ty;
    let mut arguments: Vec<&musa_calculus::Term> = Vec::new();
    while let musa_calculus::Shape::App {
        ref function,
        ref argument,
    } = *head.shape()
    {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    let spelled_arguments = || -> Option<String> {
        let written: Vec<String> = arguments
            .iter()
            .map(|argument| spelled(argument))
            .collect::<Option<_>>()?;
        Some(if written.is_empty() {
            String::new()
        } else {
            format!("<{}>", written.join(", "))
        })
    };
    match *head.shape() {
        musa_calculus::Shape::Base(ref base) => Some(format!("{base}{}", spelled_arguments()?)),
        musa_calculus::Shape::Const(ref constant) => Some(format!("{constant}{}", spelled_arguments()?)),
        musa_calculus::Shape::Universe(_) if arguments.is_empty() => Some("Type".to_owned()),
        // `A -> B` and only `A -> B`: a Π whose binder is named is one the
        // surface writes as a parameter list, and a parameter list belongs to a
        // declaration rather than to a type.
        musa_calculus::Shape::Pi {
            filling: musa_calculus::Filling::Written,
            ref name,
            ref domain,
            ref codomain,
        } if arguments.is_empty() && &**name == musa_calculus::ARROW_BINDER => {
            Some(format!("{} -> {}", spelled(domain)?, spelled(codomain)?))
        }
        // Written out rather than left to a wildcard, so a shape added to the
        // core has to be classified here before this crate builds again —
        // `crate::registry::machine`'s own discipline, and for its reason.
        musa_calculus::Shape::Universe(_)
        | musa_calculus::Shape::Pi { .. }
        | musa_calculus::Shape::Var(_)
        | musa_calculus::Shape::Def(_)
        | musa_calculus::Shape::Numeral(_)
        | musa_calculus::Shape::Lit(_)
        | musa_calculus::Shape::Builtin(_)
        | musa_calculus::Shape::Lam { .. }
        | musa_calculus::Shape::App { .. }
        | musa_calculus::Shape::RecordType(_)
        | musa_calculus::Shape::Record(_)
        | musa_calculus::Shape::Project { .. }
        | musa_calculus::Shape::Hole(_)
        | musa_calculus::Shape::Let { .. } => None,
    }
}
