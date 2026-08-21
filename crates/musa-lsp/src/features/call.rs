//! The call the caret is inside, read from the lossless tree.
//!
//! Signature help and completion ask the same question — *what is being called
//! here, and which argument am I on?* — and asked it separately until the two
//! answers disagreed about what a callee is. One reading, in one place: a
//! shell with two readers of the same syntax has two chances to be wrong about
//! it.
//!
//! Read from the tree rather than from the session's facts, and it must be:
//! both features are asked for while the call is half-written, when there is
//! no valid compile to describe it. Reading is the whole extent of it — no
//! expression is evaluated, no type inferred, and a callee that is not a plain
//! name is no call at all here, because the shell has no way to say what an
//! arbitrary expression evaluates to and guessing is the failure this module
//! exists to prevent.

use musa_syntax::{SyntaxKind, SyntaxNode};

/// A call site the caret sits in the arguments of.
pub(crate) struct CallSite {
    /// The callee as it is written: `lifted`, or `Harmony.triad` when the
    /// import was qualified — the same spelling a declaration is recorded
    /// under, so a lookup needs no second normalization.
    pub(crate) name: String,
    /// Which argument the caret is on, counting the separators before it.
    pub(crate) argument: usize,
}

/// The innermost call whose argument list contains `byte`.
///
/// "Inside" is strict about the parentheses and lax about everything else: the
/// caret must be within the argument list, because a caret on the callee is
/// asking what the name *is* and hover answers that. An assertion is a call
/// for this purpose — `assert realizes(chord c major, may_add)` writes a name
/// and its arguments, and the parser gives it the same argument-list node.
pub(crate) fn at(tree: &SyntaxNode, byte: u32) -> Option<CallSite> {
    let mut best: Option<CallSite> = None;
    for node in tree.descendants() {
        if !matches!(node.kind(), SyntaxKind::ApplyExpr | SyntaxKind::AssertStmt) {
            continue;
        }
        let Some(arguments) = node.children().find(|child| child.kind() == SyntaxKind::ExprArgList) else {
            continue;
        };
        let range = arguments.text_range();
        if byte <= u32::from(range.start()) || byte > u32::from(range.end()) {
            continue;
        }
        let Some(name) = callee(&node, &arguments) else {
            continue;
        };
        // Descendants come outermost first, so the last match encloses most
        // tightly — which is the call the caret is actually writing.
        best = Some(CallSite {
            name,
            argument: separators_before(&arguments, byte),
        });
    }
    best
}

/// The name being called, when the callee is one.
///
/// Two shapes, because the parser gives two. An assertion writes its claim as
/// a token of the statement itself; an application writes its callee as an
/// expression beside the argument list, and that expression is a name only
/// when it is identifiers and the dots between them. Anything else — a call
/// returning a function, a parenthesized expression — is a callee this module
/// declines to name, which is the honest answer for a shell that does not
/// evaluate.
fn callee(node: &SyntaxNode, arguments: &SyntaxNode) -> Option<String> {
    if let Some(token) = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
    {
        return Some(token.text().to_owned());
    }
    let callee = node.children().find(|child| child != arguments)?;
    let mut parts: Vec<String> = Vec::new();
    for element in callee.descendants_with_tokens() {
        let Some(token) = element.into_token() else {
            continue;
        };
        // An `if` chain rather than a match: the alternative is naming every
        // syntax kind the language has in order to reject them.
        if token.kind() == SyntaxKind::Identifier {
            parts.push(token.text().to_owned());
        } else if !matches!(token.kind(), SyntaxKind::Dot | SyntaxKind::Whitespace) {
            return None;
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("."))
}

/// How many argument separators precede the caret in this argument list.
///
/// Commas at the list's own depth, counted from the tree: a comma inside a
/// nested call belongs to that call and is skipped by asking only the argument
/// list's immediate children for their tokens.
fn separators_before(arguments: &SyntaxNode, byte: u32) -> usize {
    arguments
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Comma && u32::from(token.text_range().end()) <= byte)
        .count()
}
