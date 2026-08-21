//! Reading a parsed region into syntax values, and reading text back the
//! way the composer's own source is read.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::build::delimited;
use super::path::{ExpansionPath, NodePath};
use super::print::print;
use super::tree::{SourceInfo, Syntax};
use musa_score::origin::SourceSpan;

/// Read a parsed region into a syntax value.
///
/// Lossless in the sense that matters: every token the region holds becomes a
/// token here, in source order, including trivia, so the region's text is
/// recovered by writing the tokens back with each group's delimiters around
/// its children. A node whose first and last tokens are a matched delimiter
/// pair becomes a group of that delimiter; every other node is a layout group,
/// which is what the fixed grouper does with a line that opens a block.
pub(crate) fn read_region(node: &musa_syntax::SyntaxNode, expansion: ExpansionPath) -> Syntax {
    read_node(node, &NodePath::root(expansion))
}

fn read_node(node: &musa_syntax::SyntaxNode, path: &NodePath) -> Syntax {
    let span = crate::resolve::trimmed_span(node);
    // Where the parser gave up, the reader reports a hole rather than a shape
    // it did not find. A transformer folding a half-written region still gets
    // a node here, and still gets its path.
    if node.kind() == musa_syntax::SyntaxKind::Error {
        return Syntax::Missing(SourceInfo::Original {
            span,
            path: path.clone(),
        });
    }
    let (delimiter, pieces) = delimited(node.children_with_tokens().collect());
    let children = pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let child = path.child(u32::try_from(index).unwrap_or(u32::MAX));
            match piece {
                musa_syntax::SyntaxElement::Node(inner) => read_node(inner, &child),
                musa_syntax::SyntaxElement::Token(token) => read_token(token, child),
            }
        })
        .collect();
    Syntax::Group {
        info: SourceInfo::Original {
            span,
            path: path.clone(),
        },
        delimiter,
        children,
    }
}

fn read_token(token: &musa_syntax::SyntaxToken, path: NodePath) -> Syntax {
    let info = SourceInfo::Original {
        span: SourceSpan::new(
            u32::from(token.text_range().start()),
            u32::from(token.text_range().end()),
        ),
        path,
    };
    if token.kind() == musa_syntax::SyntaxKind::Identifier {
        return Syntax::Identifier {
            info,
            name: token.text().to_owned(),
            scopes: Vec::new(),
        };
    }
    Syntax::Token {
        info,
        kind: token.kind(),
        text: token.text().to_owned(),
    }
}

pub(super) fn token_text(piece: &musa_syntax::SyntaxElement) -> Option<&str> {
    match piece {
        musa_syntax::SyntaxElement::Token(token) => Some(token.text()),
        musa_syntax::SyntaxElement::Node(_) => None,
    }
}

/// Read `text` the way the composer's own source is read.
///
/// Wrapping it in a piece and a binding is what makes "is this one expression"
/// a question the ordinary parser answers rather than a second grammar this
/// module would have to keep in step with the first. One wrapping, so the two
/// callers cannot disagree about what they asked.
pub(crate) fn read_expression(text: &str) -> musa_syntax::ParsedDocument {
    musa_syntax::parse(&format!("piece \"expansion\" {{\n    let it = {text};\n}}\n"))
}

/// Whether `node` stands where an expression stands.
///
/// This is `as_expression`'s whole content, and it is deliberately not a
/// structural test: the index's claim is that the tree *parses* as an
/// expression, so the only thing that can establish it is the parser. Printing
/// and reading back is what discharges the round-trip law rather than asserting
/// it — a structural approximation would be a second answer to a question the
/// parser already answers, and the two would drift.
pub(crate) fn parses_as_expression(node: &Syntax) -> bool {
    read_expression(&print(node).text).errors().is_empty()
}
