//! The constructors a transformer calls: tokens, identifiers, groups, and
//! the binder/reference pair that shares one hygienic scope.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::category::Delimiter;
use super::path::{BindingPath, NodePath};
use super::read::token_text;
use super::tree::{SourceInfo, Syntax};

/// The delimiter a node's own outermost tokens put around it, and what is left
/// inside once they are taken off.
///
/// One place says what a group's delimiter is, because a quote's body has to be
/// grouped exactly the way the same text would be if it had been read: a
/// template that decided delimiters its own way would print back as text that
/// parses differently from what the author wrote.
pub(crate) fn delimited(
    mut pieces: Vec<musa_language::SyntaxElement>,
) -> (Delimiter, Vec<musa_language::SyntaxElement>) {
    for candidate in Delimiter::ALL {
        let (open, close) = candidate.pair();
        if open.is_empty() {
            continue;
        }
        let opens = pieces.first().and_then(token_text).is_some_and(|text| text == open);
        let closes = pieces.last().and_then(token_text).is_some_and(|text| text == close);
        if opens && closes && pieces.len() >= 2 {
            pieces.pop();
            pieces.remove(0);
            return (candidate, pieces);
        }
    }
    (Delimiter::Layout, pieces)
}

/// Build a token at `at`.
///
/// Total, like every builder here, and now total for a better reason than
/// "the gate asks later": `kind` is the lexer's own kind, so there is no
/// spelling that names no token.
pub(crate) fn token(at: NodePath, kind: musa_language::SyntaxKind, text: String) -> Syntax {
    Syntax::Token {
        info: SourceInfo::Generated(at),
        kind,
        text,
    }
}

/// Build a plain identifier at `at`, carrying no scope of its own.
///
/// This is how a transformer writes a name the composer's own source binds —
/// `pitch`, a library function — rather than one the expansion introduced. A
/// name that the expansion binds is [`binder`] and [`reference`], which carry a
/// scope and therefore cannot be captured.
pub(crate) fn identifier(at: NodePath, name: String) -> Syntax {
    Syntax::Identifier {
        info: SourceInfo::Generated(at),
        name,
        scopes: Vec::new(),
    }
}

/// Build a group at `at`.
pub(crate) fn group(at: NodePath, delimiter: Delimiter, children: Vec<Syntax>) -> Syntax {
    Syntax::Group {
        info: SourceInfo::Generated(at),
        delimiter,
        children,
    }
}

/// Build an identifier that declares `binding`.
///
/// The binder's own path *is* the binding's, so that [`check_expression`] can
/// tell a declaration from a use without a flag in the value, and so that a
/// transformer cannot declare the same name at two places by accident.
pub(crate) fn binder(binding: &BindingPath, name: String) -> Syntax {
    Syntax::Identifier {
        info: SourceInfo::Generated(binding.0.clone()),
        name,
        scopes: vec![binding.scope()],
    }
}

/// Build a reference to `binding`, at `at`.
pub(crate) fn reference(at: NodePath, binding: &BindingPath, name: String) -> Syntax {
    Syntax::Identifier {
        info: SourceInfo::Generated(at),
        name,
        scopes: vec![binding.scope()],
    }
}
