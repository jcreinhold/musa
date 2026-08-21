//! The gate a transformer's output passes through: one path per generated
//! node, one declaration per binding.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::path::NodePath;
use super::tree::{SourceInfo, Syntax};

/// Why a transformer's output was refused.
///
/// One type rather than a string, because each of these is a different mistake
/// and a transformer author reading a diagnostic should be told which one they
/// made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NotAnExpression {
    /// Two nodes were built at one path, so a later stage could not say which
    /// one a generated anchor names.
    DuplicatePath,
    /// Two binders were declared at one binding path, so one name would have
    /// two declarations and every reference would be ambiguous.
    ConflictingBinder,
}

impl std::fmt::Display for NotAnExpression {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePath => out.write_str("two nodes were built at one path"),
            Self::ConflictingBinder => out.write_str("two binders were declared at one binding path"),
        }
    }
}

/// The gate a transformer's output passes through.
///
/// Two well-formedness questions, both about the *output* rather than about
/// what it will later mean: every generated node sits at its own path, and
/// every binding is declared once. Whether the result resolves, type-checks, or
/// is musically sensible is asked afterwards by the ordinary passes, in the
/// ordinary way.
///
/// It asked a third until [`Delimiter`] became a type. "This group names a real
/// delimiter" was a question because a transformer wrote the name as text; now
/// there is no text and no unreal delimiter to name, so the question is
/// answered where the value is made rather than checked after the fact.
///
/// Only generated nodes are checked for path collisions. An input node keeps
/// its original source information wherever it is preserved, and preserving one
/// twice is a transformer duplicating text, not a transformer breaking anchors.
pub(crate) fn check_expression(root: &Syntax) -> Result<(), NotAnExpression> {
    let mut built: Vec<&NodePath> = Vec::new();
    let mut binders: Vec<&NodePath> = Vec::new();
    walk(root, &mut |node| {
        let SourceInfo::Generated(path) = node.info() else {
            // An input node keeps its original source information wherever it
            // is preserved, and preserving one twice is a transformer
            // duplicating text rather than breaking an anchor.
            return Ok(());
        };
        // A binder is an identifier whose own path *is* its binding's, which is
        // what `NodePath::binding` arranges and what separates the declaration
        // from the references that share its scope. It is asked first because a
        // repeated binder path is a *specific* mistake — one name declared
        // twice — and saying "duplicate path" about it would be true and
        // useless.
        if let Syntax::Identifier { scopes, .. } = node
            && scopes.first().is_some_and(|scope| scope.0 == *path)
        {
            if binders.contains(&path) {
                return Err(NotAnExpression::ConflictingBinder);
            }
            binders.push(path);
            built.push(path);
            return Ok(());
        }
        if built.contains(&path) {
            return Err(NotAnExpression::DuplicatePath);
        }
        built.push(path);
        Ok(())
    })
}

fn walk<'a>(
    node: &'a Syntax,
    visit: &mut impl FnMut(&'a Syntax) -> Result<(), NotAnExpression>,
) -> Result<(), NotAnExpression> {
    visit(node)?;
    if let Syntax::Group { children, .. } = node {
        for child in children {
            walk(child, visit)?;
        }
    }
    Ok(())
}
