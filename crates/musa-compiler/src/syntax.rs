//! The phase-local syntax calculus: finite syntax values, derived paths, and
//! the gate a transformer's output passes through.
//!
//! `docs/rules/language/02-core-calculus.md` §5 closes the source type grammar
//! and says the source language has no syntax value. That stays true. Nothing
//! here is nameable from ordinary source: the types have no written spelling at
//! all, and the operations over them are offered only where a transformer is
//! checked ([`crate::core`]'s expansion phase). What §5's sentence forbids is a
//! program that can inspect itself, and a program still cannot.
//!
//! The defect this module exists to repair is
//! `docs/notes/research/language-design-closure/37-final-blocker.md` §1: the
//! previous design's `NodePath` and `BindingPath` were abstract with no
//! constructors, so a transformer could not obtain the path of an input node
//! nor derive a child path, and the displayed staff and studio expansions were
//! desired output rather than programs. Here **a path is derived, never
//! invented**. There are exactly two sources of one:
//!
//! - the path-aware fold hands each input node its own structural path; and
//! - [`NodePath::built`] and [`NodePath::binding`] derive an output path from a
//!   path already held, a builder role, and a child number.
//!
//! There is no constructor from a number, a name, or a counter, and nothing
//! mints a fresh id — which is the other half of the repair
//! `34-proof-review.md` asked for, where a `fresh_name` operation could not be
//! both deterministic and fresh. Hygiene here is a *coordinate*: a binder and
//! a reference written at one [`BindingPath`] carry one [`Scope`], and two
//! binding paths are two names. Nothing is allocated, so two runs agree.
//!
//! A transformer also never reads a source range. [`SourceInfo`] lives inside
//! the value and has no eliminator: a transformer carries a node in order to
//! point at it, and can neither read nor forge where it came from.

use crate::origin::SourceSpan;

/// Which adapter call a generated node belongs to.
///
/// A structural coordinate rather than an allocated id — the build node, the
/// file, the region's ordinal within it, and so on down through parent
/// expansions — so that two runs of one compiler over one input agree on it
/// exactly. The expansion phase derives it by a fixed traversal; here the
/// driver supplies a root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExpansionPath(Vec<u32>);

impl ExpansionPath {
    /// The expansion at `ordinals`, counted from the outermost coordinate in.
    pub(crate) fn at(ordinals: Vec<u32>) -> Self {
        Self(ordinals)
    }

    fn write_into(&self, out: &mut Vec<u8>) {
        push_len(out, self.0.len());
        for ordinal in &self.0 {
            out.extend_from_slice(&ordinal.to_be_bytes());
        }
    }
}

/// One step along a path.
///
/// The two constructors are disjoint by construction, and that disjointness is
/// what keeps a derived output path from ever colliding with the structural
/// path of an input node: reading descends by [`Self::Child`] only, and
/// building extends by [`Self::Built`] only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum PathStep {
    /// The nth child of a group, as the region was read.
    Child(u32),
    /// What a builder made at `role`, `child` steps along.
    Built { role: u32, child: u32 },
}

/// Where one node sits, as a path from an expansion's root.
///
/// Two nodes have the same path exactly when they were derived the same way,
/// which is what makes "every generated node has a unique path" a property a
/// transformer can violate and [`check_expression`] can report.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodePath {
    expansion: ExpansionPath,
    steps: Vec<PathStep>,
}

impl NodePath {
    /// The root of one expansion — the only path not derived from another, and
    /// the compiler's to make.
    pub(crate) const fn root(expansion: ExpansionPath) -> Self {
        Self {
            expansion,
            steps: Vec::new(),
        }
    }

    /// The path of this node's `index`th child, as the region was read.
    ///
    /// Private, because descending is the fold's job: a transformer that could
    /// walk to a child itself could walk to one that is not there.
    fn child(&self, index: u32) -> Self {
        let mut steps = self.steps.clone();
        steps.push(PathStep::Child(index));
        Self {
            expansion: self.expansion.clone(),
            steps,
        }
    }

    /// A path for something built here: at builder `role`, `child` steps along.
    ///
    /// This is the derivation blocker 1 asked for. A transformer holding one
    /// input path can address as much output as it needs — the role separates
    /// two builders that read the same input node, and the child number
    /// separates a run of siblings — and it can address nothing else.
    pub(crate) fn built(&self, role: u32, child: u32) -> Self {
        let mut steps = self.steps.clone();
        steps.push(PathStep::Built { role, child });
        Self {
            expansion: self.expansion.clone(),
            steps,
        }
    }

    /// The binding this path declares at builder `role`.
    ///
    /// Deriving a name's identity instead of allocating it is what lets an
    /// operation be both hygienic and a function of its arguments. Asking
    /// twice gives the same binding, which is exactly what a reference to a
    /// binder needs.
    pub(crate) fn binding(&self, role: u32) -> BindingPath {
        BindingPath(self.built(role, 0))
    }

    /// How much of the evaluator's budget a path occupies: one node, and one
    /// step's worth of bytes for each step it took to derive it.
    pub(crate) fn shape(&self) -> (u64, u64) {
        let steps = u64::try_from(self.steps.len()).unwrap_or(u64::MAX);
        (1, steps.saturating_mul(9).saturating_add(4))
    }

    pub(crate) fn write_into(&self, out: &mut Vec<u8>) {
        self.expansion.write_into(out);
        push_len(out, self.steps.len());
        for step in &self.steps {
            match *step {
                PathStep::Child(index) => {
                    out.push(0);
                    out.extend_from_slice(&index.to_be_bytes());
                }
                PathStep::Built { role, child } => {
                    out.push(1);
                    out.extend_from_slice(&role.to_be_bytes());
                    out.extend_from_slice(&child.to_be_bytes());
                }
            }
        }
    }
}

/// Which name a binder declares and a reference means.
///
/// One binding path is one name: a binder and every reference derived from the
/// same path resolve together, and two paths are two names however they are
/// spelled. Nothing else about a [`BindingPath`] is observable, which is what
/// makes hygiene a property of the API rather than a convention a transformer
/// is asked to keep.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BindingPath(NodePath);

impl BindingPath {
    /// The scope this binding carries, which a binder and its references share.
    fn scope(&self) -> Scope {
        Scope(self.0.clone())
    }

    pub(crate) fn shape(&self) -> (u64, u64) {
        self.0.shape()
    }

    pub(crate) fn write_into(&self, out: &mut Vec<u8>) {
        self.0.write_into(out);
    }
}

/// An opaque hygiene mark on an identifier.
///
/// A transformer may preserve the scopes it received and may compare two
/// identifiers, and has no operation that constructs one: a [`Scope`] enters a
/// value only by being copied from an input node or derived from a
/// [`BindingPath`]. That is the whole of what "package code cannot invent a
/// scope id" means here.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Scope(NodePath);

impl Scope {
    /// This scope's exact bytes, which the printer interns to a mark.
    fn key(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.0.write_into(&mut out);
        out
    }
}

/// Where a node came from.
///
/// It has no eliminator on purpose. A transformer holds a node in order to
/// point at it — a diagnostic, a preserved use — and cannot read the range,
/// so it cannot claim a position it did not receive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceInfo {
    /// Text the composer wrote, at the structural path the reader gave it.
    Original { span: SourceSpan, path: NodePath },
    /// Something an expansion made, at the path it made it.
    Generated(NodePath),
}

impl SourceInfo {
    /// Where this node sits.
    ///
    /// A node carries its own path rather than having one reconstructed for it,
    /// which is what lets the fold hand a step function the path of the node it
    /// is looking at without a second traversal — and what makes the path of a
    /// preserved input node the same path however deep in the output it ends up.
    pub(crate) const fn path(&self) -> &NodePath {
        match self {
            Self::Original { path, .. } | Self::Generated(path) => path,
        }
    }

    fn write_into(&self, out: &mut Vec<u8>) {
        match self {
            Self::Original { span, path } => {
                out.push(0);
                out.extend_from_slice(&span.start.to_be_bytes());
                out.extend_from_slice(&span.end.to_be_bytes());
                path.write_into(out);
            }
            Self::Generated(path) => {
                out.push(1);
                path.write_into(out);
            }
        }
    }
}

/// The four delimiters the fixed grouper knows.
///
/// Spellings rather than an enum inside [`Syntax`], because a transformer
/// writes one as text and the check that it names a real delimiter belongs at
/// the gate with the other well-formedness questions — which keeps every
/// builder total.
pub(crate) const DELIMITERS: [(&str, &str, &str); 4] = [
    ("parentheses", "(", ")"),
    ("brackets", "[", "]"),
    ("braces", "{", "}"),
    ("layout", "", ""),
];

/// A finite syntax value.
///
/// The four forms of `26-language-design-decision.md` §3.3, with the source
/// information and the hygiene scopes the compiler owns held inside rather than
/// handed out. It contains no arrow at any depth, so it is storable data and
/// the kind system refuses a syntax value that hides a closure without being
/// asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// A node the reader expected and did not find. Kept rather than dropped,
    /// so that a transformer over a half-written region still has a shape to
    /// fold and a node to point a diagnostic at.
    Missing(SourceInfo),
    /// A token, by the kind the reader gave it and its exact text.
    Token {
        info: SourceInfo,
        kind: String,
        text: String,
    },
    /// A name, with the hygiene scopes it carries.
    Identifier {
        info: SourceInfo,
        name: String,
        scopes: Vec<Scope>,
    },
    /// A delimited or layout group and its children, in source order.
    Group {
        info: SourceInfo,
        delimiter: String,
        children: Vec<Self>,
    },
}

impl Syntax {
    /// Where this node came from.
    pub(crate) const fn info(&self) -> &SourceInfo {
        match self {
            Self::Missing(info)
            | Self::Token { info, .. }
            | Self::Identifier { info, .. }
            | Self::Group { info, .. } => info,
        }
    }

    /// The sub-node at `path`, if this value has one there.
    ///
    /// How a transformer preserves input: the fold reveals a node's path, and
    /// this turns that path back into the node, with its own source
    /// information intact. It reveals nothing the fold did not already, and it
    /// answers `None` for a path from another expansion or another shape,
    /// which is why it is total.
    pub(crate) fn at(&self, path: &NodePath) -> Option<&Self> {
        let root = self.info().path();
        if root.expansion != path.expansion || !root.steps.is_empty() {
            return None;
        }
        self.descend(&path.steps)
    }

    fn descend(&self, steps: &[PathStep]) -> Option<&Self> {
        let Some((first, rest)) = steps.split_first() else {
            return Some(self);
        };
        let PathStep::Child(index) = *first else {
            return None;
        };
        let Self::Group { children, .. } = self else {
            return None;
        };
        children.get(usize::try_from(index).ok()?)?.descend(rest)
    }

    /// This node's exact bytes, for identity.
    pub(crate) fn write_into(&self, out: &mut Vec<u8>) {
        match self {
            Self::Missing(info) => {
                out.push(0);
                info.write_into(out);
            }
            Self::Token { info, kind, text } => {
                out.push(1);
                info.write_into(out);
                push_text(out, kind);
                push_text(out, text);
            }
            Self::Identifier { info, name, scopes } => {
                out.push(2);
                info.write_into(out);
                push_text(out, name);
                push_len(out, scopes.len());
                for scope in scopes {
                    scope.0.write_into(out);
                }
            }
            Self::Group {
                info,
                delimiter,
                children,
            } => {
                out.push(3);
                info.write_into(out);
                push_text(out, delimiter);
                push_len(out, children.len());
                for child in children {
                    child.write_into(out);
                }
            }
        }
    }

    /// How much of the evaluator's budget this value occupies: one node per
    /// node, and the text it holds as its size.
    pub(crate) fn shape(&self) -> (u64, u64) {
        let size = |text: &str| u64::try_from(text.len()).unwrap_or(u64::MAX);
        match self {
            Self::Missing(_) => (1, 0),
            Self::Token { kind, text, .. } => (1, size(kind).saturating_add(size(text))),
            Self::Identifier { name, .. } => (1, size(name)),
            Self::Group {
                delimiter, children, ..
            } => children.iter().fold((1, size(delimiter)), |(nodes, bytes), child| {
                let (theirs, their_bytes) = child.shape();
                (nodes.saturating_add(theirs), bytes.saturating_add(their_bytes))
            }),
        }
    }
}

/// Read a parsed region into a syntax value.
///
/// Lossless in the sense that matters: every token the region holds becomes a
/// token here, in source order, including trivia, so the region's text is
/// recovered by writing the tokens back with each group's delimiters around
/// its children. A node whose first and last tokens are a matched delimiter
/// pair becomes a group of that delimiter; every other node is a layout group,
/// which is what the fixed grouper does with a line that opens a block.
pub(crate) fn read_region(node: &musa_language::SyntaxNode, expansion: ExpansionPath) -> Syntax {
    read_node(node, &NodePath::root(expansion))
}

fn read_node(node: &musa_language::SyntaxNode, path: &NodePath) -> Syntax {
    let span = crate::resolve::trimmed_span(node);
    // Where the parser gave up, the reader reports a hole rather than a shape
    // it did not find. A transformer folding a half-written region still gets
    // a node here, and still gets its path.
    if node.kind() == musa_language::SyntaxKind::Error {
        return Syntax::Missing(SourceInfo::Original {
            span,
            path: path.clone(),
        });
    }
    let mut pieces: Vec<musa_language::SyntaxElement> = node.children_with_tokens().collect();
    let mut delimiter = "layout";
    for (name, open, close) in DELIMITERS {
        if open.is_empty() {
            continue;
        }
        let opens = pieces.first().and_then(token_text).is_some_and(|text| text == open);
        let closes = pieces.last().and_then(token_text).is_some_and(|text| text == close);
        if opens && closes && pieces.len() >= 2 {
            delimiter = name;
            pieces.pop();
            pieces.remove(0);
            break;
        }
    }
    let children = pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let child = path.child(u32::try_from(index).unwrap_or(u32::MAX));
            match piece {
                musa_language::SyntaxElement::Node(inner) => read_node(inner, &child),
                musa_language::SyntaxElement::Token(token) => read_token(token, child),
            }
        })
        .collect();
    Syntax::Group {
        info: SourceInfo::Original {
            span,
            path: path.clone(),
        },
        delimiter: delimiter.to_owned(),
        children,
    }
}

fn read_token(token: &musa_language::SyntaxToken, path: NodePath) -> Syntax {
    let info = SourceInfo::Original {
        span: SourceSpan::new(
            u32::from(token.text_range().start()),
            u32::from(token.text_range().end()),
        ),
        path,
    };
    let kind = format!("{:?}", token.kind());
    if token.kind() == musa_language::SyntaxKind::Identifier {
        return Syntax::Identifier {
            info,
            name: token.text().to_owned(),
            scopes: Vec::new(),
        };
    }
    Syntax::Token {
        info,
        kind,
        text: token.text().to_owned(),
    }
}

fn token_text(piece: &musa_language::SyntaxElement) -> Option<&str> {
    match piece {
        musa_language::SyntaxElement::Token(token) => Some(token.text()),
        musa_language::SyntaxElement::Node(_) => None,
    }
}

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
    /// A group named a delimiter the fixed grouper does not have.
    UnknownDelimiter(String),
}

impl std::fmt::Display for NotAnExpression {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePath => out.write_str("two nodes were built at one path"),
            Self::ConflictingBinder => out.write_str("two binders were declared at one binding path"),
            Self::UnknownDelimiter(delimiter) => write!(out, "`{delimiter}` is not a delimiter"),
        }
    }
}

/// The gate a transformer's output passes through.
///
/// Three well-formedness questions, all of them about the *output* rather than
/// about what it will later mean: every generated node sits at its own path,
/// every binding is declared once, and every group names a real delimiter.
/// Whether the result resolves, type-checks, or is musically sensible is asked
/// afterwards by the ordinary passes, in the ordinary way.
///
/// Only generated nodes are checked for path collisions. An input node keeps
/// its original source information wherever it is preserved, and preserving one
/// twice is a transformer duplicating text, not a transformer breaking anchors.
pub(crate) fn check_expression(root: &Syntax) -> Result<(), NotAnExpression> {
    let mut built: Vec<&NodePath> = Vec::new();
    let mut binders: Vec<&NodePath> = Vec::new();
    walk(root, &mut |node| {
        if let Syntax::Group { delimiter, .. } = node
            && !DELIMITERS.iter().any(|(name, _, _)| name == delimiter)
        {
            return Err(NotAnExpression::UnknownDelimiter(delimiter.clone()));
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

/// Build a token at `at`.
///
/// Total, like every builder here: whether `kind` is a token kind the reader
/// uses is a well-formedness question, and well-formedness is asked once, at
/// [`check_expression`], rather than at each of five construction sites.
pub(crate) fn token(at: NodePath, kind: String, text: String) -> Syntax {
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
pub(crate) fn group(at: NodePath, delimiter: String, children: Vec<Syntax>) -> Syntax {
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
/// to the region's use site by the source map instead.
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
            let pair = DELIMITERS
                .iter()
                .find(|(name, _, _)| name == delimiter)
                .map_or(("layout", "", ""), |entry| *entry);
            out.text.push_str(pair.1);
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    out.text.push(' ');
                }
                write_syntax(child, marks, out);
            }
            out.text.push_str(pair.2);
        }
    }
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&u64::try_from(len).unwrap_or(u64::MAX).to_be_bytes());
}

fn push_text(out: &mut Vec<u8>, text: &str) {
    push_len(out, text.len());
    out.extend_from_slice(text.as_bytes());
}

#[cfg(test)]
// A law suite reports a violated law by failing, and the helpers below take
// apart a value whose existence the law has already asserted: a panic is the
// report, not an accident.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::core::{ExpansionFailure, expand_region};

    const REGION: &str = "let melody = together(a, b)";

    /// The expansion these laws run under. Any coordinate does; what matters is
    /// that it is the compiler's to supply and never a transformer's to invent.
    fn expansion() -> ExpansionPath {
        ExpansionPath::at(vec![7])
    }

    fn read() -> Syntax {
        read_region(&musa_language::parse(REGION).syntax(), expansion())
    }

    fn run(transformer: &str) -> Syntax {
        match expand_region(transformer, REGION, expansion()) {
            Ok(produced) => produced,
            Err(failure) => panic!("the transformer did not run: {failure:?}"),
        }
    }

    /// Every generated node in a value, in the order they were written.
    fn generated(node: &Syntax, into: &mut Vec<NodePath>) {
        if let SourceInfo::Generated(path) = node.info() {
            into.push(path.clone());
        }
        if let Syntax::Group { children, .. } = node {
            for child in children {
                generated(child, into);
            }
        }
    }

    fn identifiers<'a>(node: &'a Syntax, into: &mut Vec<&'a Syntax>) {
        if matches!(node, Syntax::Identifier { .. }) {
            into.push(node);
        }
        if let Syntax::Group { children, .. } = node {
            for child in children {
                identifiers(child, into);
            }
        }
    }

    fn count(node: &Syntax) -> usize {
        match node {
            Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Identifier { .. } => 1,
            Syntax::Group { children, .. } => children.iter().map(count).sum::<usize>().saturating_add(1),
        }
    }

    /// Write a transformer whose four cases are `missing`, `token`,
    /// `identifier`, and `group`, in that order.
    fn transformer(missing: &str, token: &str, identifier: &str, group: &str) -> String {
        format!(
            "fn (region) {{ syntax_fold(fn (here) {{ {missing} }}, fn (here, kind, text) {{ {token} }}, \
             fn (here, name) {{ {identifier} }}, fn (here, delimiter, children) {{ {group} }}, region) }}"
        )
    }

    /// A transformer that rebuilds the region node for node, marking each case
    /// with its own builder role.
    fn rebuild() -> String {
        transformer(
            r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            r"syntax_group(syntax_built(here, 3, 0), delimiter, children)",
        )
    }

    #[test]
    fn a_transformer_is_a_program_rather_than_a_picture_of_one() {
        assert_eq!(
            count(&run(&rebuild())),
            count(&read()),
            "rebuilding the region node for node gives a value of the same shape"
        );
    }

    #[test]
    fn every_node_the_fold_visits_has_its_own_path() {
        let produced = run(&rebuild());
        let mut paths = Vec::new();
        generated(&produced, &mut paths);
        let unique: std::collections::BTreeSet<_> = paths.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            paths.len(),
            "the fold handed two nodes one path, so an output anchor would be ambiguous"
        );
        assert_eq!(paths.len(), count(&read()), "the fold visited each input node once");
    }

    #[test]
    fn a_derived_output_path_never_addresses_an_input_node() {
        // Reading descends by `Child` and building extends by `Built`, so a
        // derived path and a structural one cannot be the same path.
        let mut paths = Vec::new();
        generated(&run(&rebuild()), &mut paths);
        let read = read();
        for path in &paths {
            assert!(read.at(path).is_none(), "a derived output path addressed an input node");
        }
    }

    #[test]
    fn folding_is_total_and_deterministic() {
        assert_eq!(
            run(&rebuild()),
            run(&rebuild()),
            "two runs of one transformer disagreed"
        );
        for region in ["", "let", "let x = (", "piece \"p\" { }"] {
            assert!(
                expand_region(&rebuild(), region, expansion()).is_ok(),
                "the fold declined to answer on `{region}`"
            );
        }
    }

    #[test]
    fn a_builder_is_a_function_of_its_displayed_arguments() {
        // Two calls written with the same arguments give one value, so they
        // give one *path*, and the gate refuses the pair. A builder that minted
        // a fresh id would hand back two paths and the pair would be accepted,
        // which is exactly the operation `34-proof-review.md` found could not
        // be both fresh and deterministic.
        let twice = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [syntax_token(syntax_built(here, 4, 0), "Nat", "1"),
             syntax_token(syntax_built(here, 4, 0), "Nat", "1")])"#;
        assert_eq!(
            expand_region(
                &transformer(
                    r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
                    r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                    r"syntax_identifier(syntax_built(here, 2, 0), name)",
                    twice,
                ),
                REGION,
                expansion(),
            ),
            Err(ExpansionFailure::NotAnExpression(NotAnExpression::DuplicatePath)),
            "one builder written twice with one argument list gave two different paths"
        );
    }

    #[test]
    fn one_binding_path_denotes_one_name() {
        // The binder and the reference each ask for `syntax_binding(here, 5)`,
        // and share a scope because asking twice gives the same binding.
        let bound = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [syntax_binder(syntax_binding(here, 5), "voice"),
             syntax_reference(syntax_built(here, 6, 0), syntax_binding(here, 5), "voice")])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            bound,
        ));
        let mut names = Vec::new();
        identifiers(&produced, &mut names);
        let scoped: Vec<&Vec<Scope>> = names
            .iter()
            .filter_map(|node| match node {
                Syntax::Identifier { scopes, .. } if !scopes.is_empty() => Some(scopes),
                Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Identifier { .. } | Syntax::Group { .. } => None,
            })
            .collect();
        assert!(scoped.len() >= 2, "the transformer bound nothing");
        assert_eq!(
            scoped.first(),
            scoped.get(1),
            "a binder and a reference to it carry one scope"
        );
    }

    #[test]
    fn two_binding_paths_are_two_names() {
        let two = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [syntax_binder(syntax_binding(here, 5), "voice"),
             syntax_binder(syntax_binding(here, 6), "voice")])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            two,
        ));
        let mut names = Vec::new();
        identifiers(&produced, &mut names);
        let scopes: Vec<Scope> = names
            .iter()
            .filter_map(|node| match node {
                Syntax::Identifier { scopes, .. } => scopes.first().cloned(),
                Syntax::Missing(_) | Syntax::Token { .. } | Syntax::Group { .. } => None,
            })
            .collect();
        assert!(scopes.len() >= 2, "the transformer bound nothing");
        let unique: std::collections::BTreeSet<_> = scopes.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            scopes.len(),
            "two binding paths gave one name, so two binders would capture each other"
        );
    }

    #[test]
    fn a_preserved_input_node_keeps_its_own_source_information() {
        // `syntax_at` is how a transformer carries input through: it turns a
        // path the fold revealed back into the node, unchanged.
        let carry = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [option_fold(syntax_token(syntax_built(here, 7, 0), "Missing", ""), fn (node) { node },
                         syntax_at(region, here))])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            carry,
        ));
        let Syntax::Group { children, .. } = &produced else {
            panic!("expected a group");
        };
        assert_eq!(
            children.first(),
            Some(&read()),
            "carrying an input node through changed it"
        );
    }

    #[test]
    fn a_conflicting_binder_is_refused_by_name() {
        let conflict = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [syntax_binder(syntax_binding(here, 5), "x"),
             syntax_group(syntax_built(here, 8, 0), "parentheses",
                 [syntax_binder(syntax_binding(here, 5), "x")])])"#;
        assert_eq!(
            expand_region(
                &transformer(
                    r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
                    r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                    r"syntax_identifier(syntax_built(here, 2, 0), name)",
                    conflict,
                ),
                REGION,
                expansion(),
            ),
            Err(ExpansionFailure::NotAnExpression(NotAnExpression::ConflictingBinder)),
            "one binding path declared two binders"
        );
    }

    #[test]
    fn a_group_that_names_no_real_delimiter_is_refused() {
        assert_eq!(
            expand_region(
                &transformer(
                    r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
                    r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                    r"syntax_identifier(syntax_built(here, 2, 0), name)",
                    r#"syntax_group(syntax_built(here, 3, 0), "angle", children)"#,
                ),
                REGION,
                expansion(),
            ),
            Err(ExpansionFailure::NotAnExpression(NotAnExpression::UnknownDelimiter(
                "angle".to_owned()
            ))),
            "a delimiter the fixed grouper does not have was accepted"
        );
    }

    #[test]
    fn the_gate_is_reachable_from_inside_a_transformer() {
        // `checked_expression` answers with a value either way, so a
        // transformer can ask the question of a fragment it is still building
        // rather than only having it asked of its finished result. The answer
        // is a `Result`, so it is taken apart by the one match evaluator every
        // other sum in the language is taken apart by.
        let gated = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
            [match checked_expression(syntax_token(syntax_built(here, 4, 0), "Nat", "1")) {
                Ok(node) -> node,
                Err(message) -> syntax_token(syntax_built(here, 5, 0), "Refused", message),
             }])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), "Missing", "")"#,
            r"syntax_token(syntax_built(here, 1, 0), kind, text)",
            r"syntax_identifier(syntax_built(here, 2, 0), name)",
            gated,
        ));
        let Syntax::Group { children, .. } = &produced else {
            panic!("expected a group");
        };
        let Some(Syntax::Token { kind, .. }) = children.first() else {
            panic!("the gate answered with nothing");
        };
        assert_eq!(kind, "Nat", "the gate refused a fragment it should have accepted");
    }
}
