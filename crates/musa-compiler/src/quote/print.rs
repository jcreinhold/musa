//! Printing a syntax value back to text, with the marks that let a later
//! stage find a generated node in it.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::tree::{SourceInfo, Syntax};

/// One expansion, written back as source text for the ordinary parser.
pub(crate) struct Printed {
    /// The text step 5 of the fixed order parses as one ordinary expression.
    pub(crate) text: String,
    /// Every name the printer wrote for a generated binding, in the spelling
    /// it wrote them.
    pub(crate) generated_names: std::collections::BTreeSet<String>,
    /// How many of the output's nodes the transformer built rather than
    /// received — the phase's `generated_syntax_nodes` charge.
    pub(crate) generated_nodes: u64,
}

/// Write `node` back as source text, for the ordinary parser to read.
///
/// This is step 5 of the fixed order — "parse each expansion as one ordinary
/// expression" — and the reason it is a *printer* rather than a splice is that
/// the ordinary parser is the only thing in the compiler that decides what an
/// expression is. An adapter's answer becomes text and then goes through the
/// same reader the composer's own source does; nothing skips a check by having
/// been generated.
///
/// The spacing is uniform, one space between siblings, because nothing reads
/// this text for its shape: a diagnostic that would have landed in it is moved
/// to the region's use site by the source map instead. The one exception is the
/// delimiter that says so — a [`super::category::Delimiter::Fused`] group's children are the
/// parts of one lexeme, and `c # 5` is three things to the reader that reads
/// this text back where `c#5` is one. Which groups may claim that is the gate's
/// question ([`super::check_expression`]) and not the printer's.
///
/// **Hygiene crosses here by renaming.** A generated binder and its references
/// carry a [`Scope`], which the ordinary flat namespace has no notion of, so
/// each distinct scope is interned to an ordinal in first-appearance order and
/// written as a suffix on the name. Interning rather than hashing is what makes
/// it exact: one binding is one name and two bindings are two names, with no
/// collision to argue about. What the printer cannot decide alone is whether a
/// name it wrote is also a name the *composer* wrote, so it reports the names
/// in [`Printed::generated_names`] and the phase refuses an expansion that
/// would shadow one.
pub(crate) fn print(node: &Syntax) -> Printed {
    let mut printed = Printed {
        text: String::new(),
        generated_names: std::collections::BTreeSet::new(),
        generated_nodes: 0,
    };
    let mut marks: Vec<Vec<u8>> = Vec::new();
    write_syntax(node, &mut marks, &mut printed);
    printed
}

fn write_syntax(node: &Syntax, marks: &mut Vec<Vec<u8>>, out: &mut Printed) {
    if matches!(node.info(), SourceInfo::Generated(_)) {
        out.generated_nodes = out.generated_nodes.saturating_add(1);
    }
    match node {
        // A hole has no text: where it was missing is the composer's own
        // source, and the diagnostic for it is anchored there.
        Syntax::Missing(_) => {}
        Syntax::Token { text, .. } => out.text.push_str(text),
        Syntax::Identifier { name, scopes, .. } => {
            let mut written = name.clone();
            for scope in scopes {
                let key = scope.key();
                let mark = marks.iter().position(|seen| *seen == key).unwrap_or_else(|| {
                    marks.push(key);
                    marks.len().saturating_sub(1)
                });
                {
                    use std::fmt::Write as _;
                    let _ = write!(written, "_g{mark}");
                }
            }
            if !scopes.is_empty() {
                out.generated_names.insert(written.clone());
            }
            out.text.push_str(&written);
        }
        Syntax::Group {
            delimiter, children, ..
        } => {
            let (open, close) = delimiter.pair();
            out.text.push_str(open);
            let separator = delimiter.separator();
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    out.text.push_str(separator);
                }
                write_syntax(child, marks, out);
            }
            out.text.push_str(close);
        }
    }
}
