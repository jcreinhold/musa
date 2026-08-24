//! The gate a transformer's output passes through: one path per generated
//! node, one declaration per binding.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::category::Delimiter;
use super::path::NodePath;
use super::print::print;
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
    /// A fused group's children do not spell one lexeme of one of the three
    /// composite kinds, so the text it prints to would be read back as
    /// something else — or as several things.
    NotOneLexeme {
        /// What the group's children spell, together.
        text: String,
        /// What the reader made of that text.
        lexed: String,
    },
}

impl std::fmt::Display for NotAnExpression {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePath => out.write_str("two nodes were built at one path"),
            Self::ConflictingBinder => out.write_str("two binders were declared at one binding path"),
            Self::NotOneLexeme { text, lexed } => {
                write!(out, "`{text}` is written as one lexeme and reads as {lexed}")
            }
        }
    }
}

/// The gate a transformer's output passes through.
///
/// Three well-formedness questions, all about the *output* rather than about
/// what it will later mean: every generated node sits at its own path, every
/// binding is declared once, and every fused group really is one lexeme.
/// Whether the result resolves, type-checks, or is musically sensible is asked
/// afterwards by the ordinary passes, in the ordinary way.
///
/// It asked a fourth until [`Delimiter`] became a type. "This group names a real
/// delimiter" was a question because a transformer wrote the name as text; now
/// there is no text and no unreal delimiter to name, so the question is
/// answered where the value is made rather than checked after the fact.
///
/// The lexeme question cannot move there for the opposite reason: what a fused
/// group's children spell is not known until they are all in place, and it is
/// the *reader* that decides whether they spell one thing. So a transformer
/// builds freely and the gate says whether the reader would have read it, which
/// is the discipline this gate already runs on paths and binders. It is also
/// where `11-quotation.md` §2's splice rule is kept rather than weakened: a
/// fused group assembling `abc` lexes as one token whose kind is `Identifier`,
/// which is not one of the three, so there is still no way to build a name out
/// of pieces.
///
/// Only generated nodes are checked for path collisions. An input node keeps
/// its original source information wherever it is preserved, and preserving one
/// twice is a transformer duplicating text, not a transformer breaking anchors.
pub(crate) fn check_expression(root: &Syntax) -> Result<(), NotAnExpression> {
    // Sets rather than vectors: both were scanned linearly for every node, and
    // each comparison walks a path, so the gate cost the square of the region's
    // node count times its depth. On a deep region that is the difference
    // between a refusal and a host that stops responding.
    let mut built: std::collections::HashSet<&NodePath> = std::collections::HashSet::new();
    let mut binders: std::collections::HashSet<&NodePath> = std::collections::HashSet::new();
    walk(root, &mut |node| {
        if let Syntax::Group {
            delimiter: Delimiter::Fused,
            ..
        } = node
        {
            one_lexeme(node)?;
        }
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
            if !binders.insert(path) {
                return Err(NotAnExpression::ConflictingBinder);
            }
            built.insert(path);
            return Ok(());
        }
        if !built.insert(path) {
            return Err(NotAnExpression::DuplicatePath);
        }
        Ok(())
    })
}

/// Whether a fused group's children spell one lexeme of one of the three
/// composite kinds.
///
/// The reader answers, over the text the printer would write, because those are
/// the two halves of the claim: the group says it is one lexeme, and the only
/// thing that can establish that is the lexer that would read the text back.
/// A structural test — "the children are a letter, an accidental and an octave"
/// — would be a second grammar beside the one in `musa-syntax`, and the two
/// would drift.
fn one_lexeme(node: &Syntax) -> Result<(), NotAnExpression> {
    let text = print(node).text;
    let lexed = musa_syntax::lex(&text);
    let refuse = |lexed: String| {
        Err(NotAnExpression::NotOneLexeme {
            text: text.clone(),
            lexed,
        })
    };
    let [only] = lexed.tokens() else {
        return refuse(format!("{} tokens", lexed.tokens().len()));
    };
    if !only.kind.is_composite_literal() {
        return refuse(format!("one `{}`", super::token_kind_spelling(only.kind)));
    }
    Ok(())
}

/// # Why this is a loop
///
/// The depth this descends is the depth of the region a transformer was handed,
/// which is the size of an input rather than how deeply anybody wrote a term —
/// the same argument §4.1 accepts for the two data walks, and the same remedy:
/// the pending work is an explicit stack in this function's own frame, so a
/// region deep enough to matter costs heap and not host frames. Recursing here
/// cost about 500 bytes a level, so this walk alone aborted the process on a
/// region some four thousand groups deep — a refusal turned into a crash, which
/// is the one outcome `02-core-calculus.md` §4 does not have.
fn walk<'a>(
    node: &'a Syntax,
    visit: &mut impl FnMut(&'a Syntax) -> Result<(), NotAnExpression>,
) -> Result<(), NotAnExpression> {
    let mut pending: Vec<&'a Syntax> = vec![node];
    while let Some(node) = pending.pop() {
        visit(node)?;
        if let Syntax::Group { children, .. } = node {
            // Reversed, so that popping takes them in source order: the gate
            // reports the *first* duplicate, and which node that is is part of
            // the diagnostic rather than an accident of traversal order.
            pending.extend(children.iter().rev());
        }
    }
    Ok(())
}
