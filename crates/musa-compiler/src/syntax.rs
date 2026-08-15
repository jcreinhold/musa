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

/// How a syntax value parses — `../rules/language/11-quotation.md` §1's index.
///
/// **Two cases, not four.** Prompt 131 wrote `Item` and `Pattern` beside them
/// and prompt 132's trial found that no program constructs either. `Item` comes
/// back when an adapter expands a region into declarations rather than into an
/// expression, which no planned adapter does; re-adding it is a case here and
/// its round-trip test.
///
/// The index is a claim about how the tree parses, and the representation is
/// the same either way. A refined claim is introduced only by an operation that
/// establishes it — today, [`as_expression`]'s checked parse — and it is
/// forgotten wherever it is not needed, which is one acceptance rule in the
/// checker rather than a `forget` an author writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Cat {
    /// The real parser read this tree as an expression.
    Expr,
    /// Nothing is claimed about how this tree parses.
    TokenTree,
}

impl Cat {
    /// The name this category is written by, inside `Syntax<…>`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Expr => "Expr",
            Self::TokenTree => "TokenTree",
        }
    }

    /// The category a written name denotes.
    pub(crate) fn named(text: &str) -> Option<Self> {
        [Self::Expr, Self::TokenTree]
            .into_iter()
            .find(|candidate| candidate.name() == text)
    }

    /// Whether a position of this category accepts a value of `theirs`.
    ///
    /// §1's forgetting rule, and the whole of it: a token-tree position accepts
    /// anything, because a token-tree position is precisely one that has not
    /// been parsed as anything more specific, and every other position requires
    /// its own category exactly. Directional on purpose — the reverse is the
    /// uncertified splice the index exists to refuse.
    pub(crate) fn accepts(self, theirs: Self) -> bool {
        self == Self::TokenTree || self == theirs
    }
}

/// The lexer's own token kinds, under the names an adapter writes them by.
///
/// The phase's `TokenKind` *is* [`musa_language::SyntaxKind`] rather than a
/// parallel enum, so there is no second set of cases to fall out of step with
/// the lexer's. What this adds is the spelling, and the spelling cannot
/// disagree with the kind because `stringify!` writes it from the same
/// identifier. The one thing the macro cannot say is that the list is
/// *complete*, so a drift test says it: every kind the lexer can produce
/// appears here, and no parser node kind does.
macro_rules! token_kinds {
    ($($case:ident),* $(,)?) => {
        pub(crate) const TOKEN_KINDS: &[(&str, musa_language::SyntaxKind)] =
            &[$((stringify!($case), musa_language::SyntaxKind::$case)),*];
    };
}

token_kinds!(
    Whitespace,
    LineComment,
    BlockComment,
    Identifier,
    Integer,
    Float,
    Rational,
    String,
    PitchLiteral,
    IntervalLiteral,
    UnitHz,
    UnitMs,
    UnitS,
    UnitDb,
    UnitBpm,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Semicolon,
    Comma,
    Colon,
    Arrow,
    PipeForward,
    Equals,
    EqualsEquals,
    Minus,
    Plus,
    Star,
    Tilde,
    Dot,
    Slash,
    Pipe,
    Greater,
    Less,
    Caret,
    Hash,
    Dollar,
    Question,
    PieceKw,
    TempoKw,
    MeterKw,
    KeyKw,
    SubtitleKw,
    ComposerKw,
    ArrangerKw,
    CopyrightKw,
    MotifKw,
    ScoreKw,
    PartKw,
    VoiceKw,
    ClefKw,
    UseKw,
    ImportKw,
    SyntaxKw,
    ModKw,
    TransposeKw,
    DownKw,
    UpKw,
    RestKw,
    RepeatKw,
    SlurKw,
    DynamicKw,
    TupletKw,
    PerformanceKw,
    ProfileKw,
    MarkKw,
    GrooveKw,
    GraceKw,
    StudioKw,
    PatchKw,
    ModulateKw,
    BusKw,
    AssignKw,
    RouteKw,
    SendKw,
    MasterKw,
    AtKw,
    OutputKw,
    PitchKw,
    StretchKw,
    RetrogradeKw,
    InvertKw,
    AroundKw,
    WithKw,
    NoteKw,
    PhraseKw,
    SectionKw,
    HarmonyKw,
    LibraryKw,
    CrescendoKw,
    DiminuendoKw,
    ToKw,
    BarKw,
    AssertKw,
    SenzaKw,
    EndingKw,
    FragmentKw,
    MobileKw,
    ImproviseKw,
    OverKw,
    LetKw,
    FnKw,
    MusicKw,
    KernelKw,
    OptionKw,
    ListKw,
    ResultKw,
    MatchKw,
    IfKw,
    ElseKw,
    SomeKw,
    NoneKw,
    OkKw,
    ErrKw,
    TrueKw,
    FalseKw,
    ScaleKw,
    DegreeKw,
    FrameKw,
    InKw,
    StepKw,
    ChordKw,
    StackKw,
    TemplateKw,
    MakeKw,
    AsKw,
    SignatureKw,
    StructureKw,
    DataKw,
    RecordKw,
    EnumKw,
    ModuleKw,
    PrivateKw,
    TraitKw,
    ImplKw,
    WhereKw,
    QuoteKw,
    Error,
);

/// The token kind `TokenKind.<case>` names.
///
/// A linear scan, because it runs once per name an adapter writes and the
/// alternative is a second ordering to keep in step with the first.
pub(crate) fn token_kind_named(case: &str) -> Option<musa_language::SyntaxKind> {
    TOKEN_KINDS
        .iter()
        .find(|(name, _)| *name == case)
        .map(|(_, kind)| *kind)
}

/// The four delimiters the fixed grouper knows.
///
/// A type rather than the spellings it used to be. A transformer named one as
/// text and the gate checked afterwards that the text named something real;
/// now there is nothing to check, because the only values are these four and
/// the phase offers them by name (`../rules/language/11-quotation.md` §4).
/// `syntax_group`'s ownership entry claimed to hide "the fixed grouper's
/// delimiter set", and this is what hides it instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Delimiter {
    Parentheses,
    Brackets,
    Braces,
    /// No delimiter at all — siblings held together by their layout.
    Layout,
}

impl Delimiter {
    /// Every delimiter, in the order the grouper tries them.
    ///
    /// [`Self::Layout`] is last and is the fallback: its pair is empty, so it
    /// matches every node and would swallow the other three if it were tried
    /// first.
    pub(crate) const ALL: [Self; 4] = [Self::Parentheses, Self::Brackets, Self::Braces, Self::Layout];

    /// The name the phase spells this delimiter by, after `Delimiter.`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Parentheses => "Parentheses",
            Self::Brackets => "Brackets",
            Self::Braces => "Braces",
            Self::Layout => "Layout",
        }
    }

    /// The text that opens and closes a group of this delimiter, both empty
    /// for [`Self::Layout`].
    pub(crate) const fn pair(self) -> (&'static str, &'static str) {
        match self {
            Self::Parentheses => ("(", ")"),
            Self::Brackets => ("[", "]"),
            Self::Braces => ("{", "}"),
            Self::Layout => ("", ""),
        }
    }

    /// The delimiter `Delimiter.<case>` names.
    pub(crate) fn named(case: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| candidate.name() == case)
    }

    /// The byte this delimiter encodes as, for a syntax value's exact bytes.
    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::Parentheses => 0,
            Self::Brackets => 1,
            Self::Braces => 2,
            Self::Layout => 3,
        }
    }
}

/// How a node was derived, when it was derived rather than read.
///
/// `../rules/language/11-quotation.md` §3's triple, and the name for what
/// [`NodePath::built`] has computed all along: the `origin` is the path a
/// builder was pointed at, the `quotation` separates two construction sites
/// that read one input node, and the `path` is the position within what that
/// site built. It is not a third case of [`SourceInfo`] and not a second
/// derivation graph — a derived node is [`SourceInfo::Generated`], and this is
/// *how* its path is computed.
///
/// **The identity law**: two derived nodes are one node exactly when their
/// origin, quotation, and path are all equal, and nothing else makes two
/// equal. Uniqueness is then structural rather than a convention an author
/// keeps, which is what `check_expression`'s duplicate-path gate turns from a
/// check on an author into evidence about the compiler.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Derived {
    /// The node this was built from.
    pub(crate) origin: NodePath,
    /// Which construction site built it.
    pub(crate) quotation: u32,
    /// Where inside that site's own tree it sits.
    ///
    /// Never empty. A derivation names a place *inside* what a site built, and
    /// a site that built nothing named no place — an empty path would restate
    /// the origin, which is neither derived nor distinct from the input node
    /// and would put every site's outermost node at one address. So the
    /// outermost node a site builds is `[0]` rather than `[]`, and every
    /// builder here maintains that: `syntax_built(here, role, child)` names
    /// `[child]`, and [`instantiate`] starts a template's walk one step in.
    pub(crate) path: Vec<u32>,
}

impl Derived {
    /// The path this derivation names.
    ///
    /// Total, and injective in all three components *given a non-empty path*:
    /// the steps a [`NodePath`] holds are [`PathStep::Built`] only, which is
    /// disjoint from the [`PathStep::Child`] steps reading produces, so a
    /// derived path can never collide with the structural path of an input
    /// node either. See [`Self::path`](Derived::path) — the field — for why the
    /// empty path is not a place.
    pub(crate) fn path(&self) -> NodePath {
        let mut built = self.origin.clone();
        for step in &self.path {
            built = built.built(self.quotation, *step);
        }
        built
    }
}

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
    ///
    /// The kind *is* the lexer's own, rather than a name printed from it and
    /// compared back as text. That is what makes the phase's `TokenKind` a
    /// generated type rather than a second table to keep in step: there is no
    /// second table.
    Token {
        info: SourceInfo,
        kind: musa_language::SyntaxKind,
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
        delimiter: Delimiter,
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

    /// The anchor of the sub-node at `path`: its position in this value's own
    /// reading order, counting this node as zero.
    ///
    /// A function of this value alone — not a counter, not the region's ordinal
    /// in the file, and not anything the compiler allocates. That is what keeps
    /// prompt 127dc's law intact: two identical regions anywhere in a file mint
    /// the same anchors, so one region still has one answer and the cache still
    /// replays a miss's charge. `None` where `at` is `None`, and for the same
    /// reason — an adapter anchors a node it was given, and a node it built has
    /// no place in the composer's text to be anchored to.
    pub(crate) fn anchor(&self, path: &NodePath) -> Option<u64> {
        let root = self.info().path();
        if root.expansion != path.expansion || !root.steps.is_empty() {
            return None;
        }
        self.count_to(&path.steps, 0)
    }

    fn count_to(&self, steps: &[PathStep], here: u64) -> Option<u64> {
        let Some((first, rest)) = steps.split_first() else {
            return Some(here);
        };
        let PathStep::Child(index) = *first else {
            return None;
        };
        let Self::Group { children, .. } = self else {
            return None;
        };
        let index = usize::try_from(index).ok()?;
        // Pre-order: this node, then each earlier sibling's whole subtree,
        // then the wanted child. `shape` already counts a subtree's nodes.
        let mut reached = here.saturating_add(1);
        for earlier in children.get(..index)? {
            reached = reached.saturating_add(earlier.shape().0);
        }
        children.get(index)?.count_to(rest, reached)
    }

    /// Every node's source range, in the order [`Syntax::anchor`] numbers.
    ///
    /// The compiler-owned half of the anchor: the number alone means nothing
    /// without this table, which is what keeps a forged anchor a mislocated
    /// sentence rather than a way to read a range the adapter was never given.
    /// A region read from source is `Original` throughout, so `fallback` is
    /// reached only by a value that was built rather than read.
    pub(crate) fn spans(&self, fallback: SourceSpan) -> Vec<SourceSpan> {
        let mut out = Vec::new();
        self.push_spans(fallback, &mut out);
        out
    }

    fn push_spans(&self, fallback: SourceSpan, out: &mut Vec<SourceSpan>) {
        out.push(match self.info() {
            SourceInfo::Original { span, .. } => *span,
            SourceInfo::Generated(_) => fallback,
        });
        if let Self::Group { children, .. } = self {
            for child in children {
                child.push_spans(fallback, out);
            }
        }
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
                out.extend_from_slice(&u16::from(*kind).to_be_bytes());
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
                out.push(delimiter.tag());
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
            // A kind and a delimiter are two bytes and one, now that neither
            // is a string. Charging them their stored size rather than their
            // spelling's is the same rule prompt 127dcec set — a value is
            // charged what it occupies — applied to a value that shrank.
            Self::Token { text, .. } => (1, size(text).saturating_add(2)),
            Self::Identifier { name, .. } => (1, size(name)),
            Self::Group { children, .. } => children.iter().fold((1, 1), |(nodes, bytes), child| {
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
    let (delimiter, pieces) = delimited(node.children_with_tokens().collect());
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
        delimiter,
        children,
    }
}

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

fn read_token(token: &musa_language::SyntaxToken, path: NodePath) -> Syntax {
    let info = SourceInfo::Original {
        span: SourceSpan::new(
            u32::from(token.text_range().start()),
            u32::from(token.text_range().end()),
        ),
        path,
    };
    if token.kind() == musa_language::SyntaxKind::Identifier {
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

/// Read `text` the way the composer's own source is read.
///
/// Wrapping it in a piece and a binding is what makes "is this one expression"
/// a question the ordinary parser answers rather than a second grammar this
/// module would have to keep in step with the first. One wrapping, so the two
/// callers cannot disagree about what they asked.
pub(crate) fn read_expression(text: &str) -> musa_language::ParsedDocument {
    musa_language::parse(&format!("piece \"expansion\" {{\n    let it = {text};\n}}\n"))
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

/// A quote's body, with a hole where each splice stands.
///
/// The shape [`read_region`] would have read out of the same text, minus the
/// positions the author left open. Keeping it is what lets one `quote at …` be
/// read once, at its definition, and instantiated at every call: the tree does
/// not depend on the environment, and what does — the anchor and each splice's
/// value — is a hole here and an expression in the checked quote.
///
/// It carries no source information at all. Every node a quote writes is
/// generated, and its path is a function of the anchor, the quotation, and the
/// position in *this* tree, which is exactly what [`instantiate`] computes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Template {
    /// A node the parser expected inside the body and did not find.
    Missing,
    Token {
        kind: musa_language::SyntaxKind,
        text: String,
    },
    Identifier {
        name: String,
        hygiene: Hygiene,
    },
    Group {
        delimiter: Delimiter,
        /// Whether this position separates its elements with commas.
        ///
        /// §2: "the separator a sequence splice needs — the commas of an
        /// argument list — is supplied by the grammar of the position". So the
        /// commas the body writes are *not* children of a separated group: one
        /// is minted between every pair of elements at instantiation, which is
        /// what makes `f($a, $..rest)` right for a `rest` of any length, the
        /// empty one included. A group that kept the written commas and spread
        /// beside them would leave a trailing one there, and musa rejects a
        /// trailing comma.
        separated: bool,
        children: Vec<Self>,
    },
    /// `$x` or `${ e }` — one node, from the splice at this index.
    Splice(usize),
    /// `$..xs` — a run of nodes, from the splice at this index. Legal only as a
    /// separated group's child, which is the only place a run of siblings has
    /// both room and a separator.
    Sequence(usize),
}

/// What a quoted name refers to.
///
/// `11-quotation.md` §4's two halves, decided while the body is read rather
/// than while it is instantiated: a name the quote itself binds gets a scope
/// derived from where the binder stands, and every other name is written plain
/// and means what it meant where the quote was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Hygiene {
    /// A name the quote does not bind.
    Free,
    /// A binder the quote writes.
    Binder,
    /// A use of a binder the quote writes, named by that binder's position in
    /// this template.
    Bound(Vec<u32>),
}

/// One splice's value, at instantiation — and what matching hands back.
///
/// One type for both directions, because they are one relation read two ways:
/// [`instantiate`] takes these and a template and answers a value, and
/// [`matched`] takes that value and the same template and answers these. A
/// separate "what a pattern bound" type would be the same three words with a
/// second name, and the law that the two are inverse would have to be stated
/// across a conversion instead of as an equation.
#[derive(Clone, Debug)]
pub(crate) enum Spliced {
    One(Syntax),
    Many(Vec<Syntax>),
}

/// Match `value` against `template`, binding each of its `holes`.
///
/// `11-quotation.md` §4's pattern form, and the inverse of [`instantiate`]:
/// `matched(t, &instantiate(t, …, s)?, s.len())` is `s` again, node for node.
/// `None` where the value has a different shape, which is a pattern doing its
/// job rather than a failure.
///
/// **What "the same shape" means**, since matching is where it has to be said
/// exactly:
///
/// - **Trivia is not shape.** A read region keeps every comment and space it
///   was written with, and a template kept none, so the significant children
///   are what is compared. A comment between two spliced elements therefore
///   does not defeat a match, which is what keeps an adapter from breaking
///   when a composer runs the formatter.
/// - **The parser's bracketing is not shape.** A quote's body goes through the
///   expression grammar, so `a` inside an argument list arrives wrapped in the
///   layout groups the CST puts around a name, an argument, and an operand; a
///   region's own reader wraps nothing. Peeling a layout group that holds one
///   node is what makes those two the same tree — and it is the whole of the
///   difference, because a layout group holding *two* nodes is a shape the
///   grammar meant.
/// - **A separated position's commas are not shape either.** They are supplied
///   by the position ([`Template::Group::separated`]) and a template has none,
///   so a value's are read as what separates its elements: exactly one between
///   each pair, none at either end.
/// - **Provenance is not shape at all.** Nothing here reads a [`SourceInfo`],
///   a [`Scope`], or a [`Derived`], so a node a quote built and a node a
///   composer wrote match alike — §4's second rule, and the reason an adapter
///   cannot ask where a node came from.
pub(crate) fn matched(template: &Template, value: &Syntax, holes: usize) -> Option<Vec<Spliced>> {
    let mut bound: Vec<Option<Spliced>> = vec![None; holes];
    if !match_one(peel_template(template), peel(value), &mut bound) {
        return None;
    }
    bound.into_iter().collect()
}

/// One peeled template against one peeled value.
fn match_one(template: &Template, value: &Syntax, bound: &mut [Option<Spliced>]) -> bool {
    match template {
        Template::Splice(hole) => bind(bound, *hole, Spliced::One(value.clone())),
        // A run of nodes is not one node. The checker refuses a spread that
        // stands where one node stands, and this is the evaluator refusing to
        // guess what it would have meant.
        Template::Sequence(_) => false,
        Template::Missing => matches!(value, Syntax::Missing(_)),
        Template::Token { kind, text } => {
            matches!(value, Syntax::Token { kind: found, text: written, .. } if found == kind && written == text)
        }
        // Names only: a scope is a coordinate the compiler owns, and §4's
        // second rule is that a pattern reads nothing of the sort.
        Template::Identifier { name, .. } => matches!(value, Syntax::Identifier { name: found, .. } if found == name),
        Template::Group {
            delimiter,
            separated,
            children,
        } => {
            let Syntax::Group {
                delimiter: found,
                children: held,
                ..
            } = value
            else {
                return false;
            };
            if found != delimiter {
                return false;
            }
            let items: Vec<&Syntax> = held.iter().filter(|child| !is_trivia(child)).collect();
            if !*separated {
                return match_run(children, &items, bound);
            }
            let Some(elements) = elements(&items) else {
                return false;
            };
            match_run(children, &elements, bound)
        }
    }
}

/// A run of templates against a run of values, with at most one spread.
///
/// The spread's length is decided by the two ends rather than searched for:
/// what stands before it matches from the front, what stands after it matches
/// from the back, and what is left over is the run it binds. Two spreads would
/// leave the split between them undetermined, so the checker refuses them and
/// this refuses to guess.
fn match_run(templates: &[Template], items: &[&Syntax], bound: &mut [Option<Spliced>]) -> bool {
    let peeled: Vec<&Template> = templates.iter().map(peel_template).collect();
    let mut spreads = peeled
        .iter()
        .enumerate()
        .filter_map(|(index, template)| match template {
            Template::Sequence(hole) => Some((index, *hole)),
            Template::Missing
            | Template::Token { .. }
            | Template::Identifier { .. }
            | Template::Group { .. }
            | Template::Splice(_) => None,
        });
    let Some((at, hole)) = spreads.next() else {
        return peeled.len() == items.len()
            && peeled
                .iter()
                .zip(items)
                .all(|(template, value)| match_one(template, peel(value), bound));
    };
    if spreads.next().is_some() {
        return false;
    }
    let after = peeled.len().saturating_sub(at).saturating_sub(1);
    let Some(tail) = items.len().checked_sub(after) else {
        return false;
    };
    if tail < at {
        return false;
    }
    let before = peeled
        .get(..at)
        .into_iter()
        .flatten()
        .zip(items.get(..at).into_iter().flatten());
    let behind = peeled
        .get(at.saturating_add(1)..)
        .into_iter()
        .flatten()
        .zip(items.get(tail..).into_iter().flatten());
    for (template, value) in before.chain(behind) {
        if !match_one(template, peel(value), bound) {
            return false;
        }
    }
    let run = items.get(at..tail).unwrap_or_default();
    bind(
        bound,
        hole,
        Spliced::Many(run.iter().map(|node| (*node).clone()).collect()),
    )
}

/// Fill one hole. Each hole is written by exactly one splice in the template
/// it came from, so there is nothing to reconcile here; a hole out of range is
/// a template the checker never made.
fn bind(bound: &mut [Option<Spliced>], hole: usize, value: Spliced) -> bool {
    let Some(slot) = bound.get_mut(hole) else {
        return false;
    };
    *slot = Some(value);
    true
}

/// A value with the parser's bracketing taken off.
///
/// See [`matched`] for why a layout group holding one node is not a shape.
fn peel(value: &Syntax) -> &Syntax {
    let mut node = value;
    loop {
        let Syntax::Group {
            delimiter: Delimiter::Layout,
            children,
            ..
        } = node
        else {
            return node;
        };
        let mut significant = children.iter().filter(|child| !is_trivia(child));
        let (Some(only), None) = (significant.next(), significant.next()) else {
            return node;
        };
        node = only;
    }
}

/// The same, on the pattern's side.
///
/// A separated group is never peeled, whatever its delimiter turned out to be:
/// its commas are part of how its children are read, and a group that lost
/// them would be read as a run of siblings instead of a list of elements.
fn peel_template(template: &Template) -> &Template {
    let mut node = template;
    loop {
        let Template::Group {
            delimiter: Delimiter::Layout,
            separated: false,
            children,
        } = node
        else {
            return node;
        };
        let [only] = children.as_slice() else {
            return node;
        };
        node = only;
    }
}

fn is_trivia(node: &Syntax) -> bool {
    matches!(node, Syntax::Token { kind, .. } if kind.is_trivia())
}

/// The elements a separated group holds, or `None` if its children are not
/// one node between each pair of commas.
///
/// `(a, b)` is two elements; `()` is none; `(a,)`, `(, a)` and `(a b, c)` are
/// not a separated position's shape at all, so nothing matches them.
fn elements<'a>(items: &[&'a Syntax]) -> Option<Vec<&'a Syntax>> {
    let mut out: Vec<&Syntax> = Vec::with_capacity(items.len());
    let mut element: Option<&Syntax> = None;
    for item in items {
        if matches!(item, Syntax::Token { kind, .. } if *kind == musa_language::SyntaxKind::Comma) {
            out.push(element.take()?);
        } else if element.replace(item).is_some() {
            return None;
        }
    }
    match element {
        Some(last) => out.push(last),
        None if out.is_empty() => {}
        None => return None,
    }
    Some(out)
}

/// Build one quote's answer: `template`, with `spliced` in its holes, derived
/// from `origin` by construction site `quotation`.
///
/// **This is where provenance is minted** (`11-quotation.md` §3). Every node the
/// template writes is [`SourceInfo::Generated`] at a [`Derived`] path, and every
/// node that arrived through a splice is copied in with the identity it already
/// had. So the author supplies no number: the origin comes from the anchor, the
/// quotation from the elaborator's counter, and the path from the position in
/// the template — which is fixed when the quote is read, so a sequence splice
/// expanding to three nodes does not shift what its siblings are called.
///
/// `None` where a hole's value has the wrong arity for the position it stands
/// in, which the checker has already refused; it is here as well because a
/// total evaluator cannot assume its own checker ran.
pub(crate) fn instantiate(
    template: &Template,
    origin: &NodePath,
    quotation: u32,
    spliced: &[Spliced],
) -> Option<Syntax> {
    build(template, origin, quotation, spliced, &mut template_root())
}

/// Where a walk of a quote's template starts.
///
/// One step in, never at the origin itself: what a quote builds outermost is a
/// node of its own, and two quotes reading one input node have to disagree
/// about where that node sits — see [`Derived`]'s `path` field.
///
/// Both walks start here. The checker records a quoted binder's position
/// against its walk of the body and [`build`] rebuilds that position when it
/// instantiates, so a binder and the uses that name it are one binding only
/// while the two walks agree about the first step.
pub(crate) fn template_root() -> Vec<u32> {
    vec![0]
}

fn build(
    template: &Template,
    origin: &NodePath,
    quotation: u32,
    spliced: &[Spliced],
    path: &mut Vec<u32>,
) -> Option<Syntax> {
    let at = |path: &Vec<u32>| {
        Derived {
            origin: origin.clone(),
            quotation,
            path: path.clone(),
        }
        .path()
    };
    match template {
        Template::Missing => Some(Syntax::Missing(SourceInfo::Generated(at(path)))),
        Template::Token { kind, text } => Some(token(at(path), *kind, text.clone())),
        Template::Identifier { name, hygiene } => Some(match hygiene {
            Hygiene::Free => identifier(at(path), name.clone()),
            // The binder's own path *is* its binding's, so a quote that writes
            // two binders writes two names and one written twice is one name,
            // with nothing allocated either way.
            Hygiene::Binder => binder(&at(path).binding(quotation), name.clone()),
            Hygiene::Bound(declared) => reference(at(path), &at(&declared.clone()).binding(quotation), name.clone()),
        }),
        Template::Group {
            delimiter,
            separated,
            children,
        } => {
            let mut elements: Vec<Syntax> = Vec::with_capacity(children.len());
            for (index, child) in children.iter().enumerate() {
                if let Template::Sequence(hole) = child {
                    let Spliced::Many(values) = spliced.get(*hole)? else {
                        return None;
                    };
                    elements.extend(values.iter().cloned());
                    continue;
                }
                path.push(u32::try_from(index).unwrap_or(u32::MAX));
                let node = build(child, origin, quotation, spliced, path);
                path.pop();
                elements.push(node?);
            }
            if *separated {
                // Every child of a separated position is one element, and so is
                // every node a spread put there, so one comma goes between each
                // consecutive pair and none goes at either end. The commas are
                // numbered past the template's own children, so they cannot
                // collide with a literal node's path however long the spread
                // turns out to be.
                let mut minted = u32::try_from(children.len()).unwrap_or(u32::MAX);
                let mut separated_elements = Vec::with_capacity(elements.len().saturating_mul(2));
                for element in elements {
                    if !separated_elements.is_empty() {
                        path.push(minted);
                        separated_elements.push(token(at(path), musa_language::SyntaxKind::Comma, ",".to_owned()));
                        path.pop();
                        minted = minted.saturating_add(1);
                    }
                    separated_elements.push(element);
                }
                elements = separated_elements;
            }
            Some(group(at(path), *delimiter, elements))
        }
        Template::Splice(hole) => match spliced.get(*hole)? {
            Spliced::One(node) => Some(node.clone()),
            Spliced::Many(_) => None,
        },
        // Reached only if a sequence splice stood where no group could spread
        // it, which the checker refuses with a diagnostic that can say where.
        Template::Sequence(_) => None,
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
            let (open, close) = delimiter.pair();
            out.text.push_str(open);
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    out.text.push(' ');
                }
                write_syntax(child, marks, out);
            }
            out.text.push_str(close);
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
    ///
    /// The fold's answer is wrapped in `Ok`, because a transformer answers
    /// `Result<Syntax, (Syntax, Text)>` and every law here is about the half
    /// that accepts. The refusing half is `crate::expand`'s to exercise, where
    /// there is a diagnostic to read it out of.
    fn transformer(missing: &str, token: &str, identifier: &str, group: &str) -> String {
        format!(
            "fn (region) {{ Ok(syntax_fold_from_leaves(fn (here) {{ {missing} }}, fn (here, kind, text) {{ {token} }}, \
             fn (here, name) {{ {identifier} }}, fn (here, delimiter, children) {{ {group} }}, region)) }}"
        )
    }

    /// A transformer that rebuilds the region node for node, marking each case
    /// with its own builder role.
    fn rebuild() -> String {
        transformer(
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
            [syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1"),
             syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1")])"#;
        assert_eq!(
            expand_region(
                &transformer(
                    r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
            [option_fold(syntax_token(syntax_built(here, 7, 0), TokenKind.Error, ""), fn (node) { node },
                         syntax_at(region, here))])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
             syntax_group(syntax_built(here, 8, 0), Delimiter.Parentheses,
                 [syntax_binder(syntax_binding(here, 5), "x")])])"#;
        assert_eq!(
            expand_region(
                &transformer(
                    r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
    fn a_group_that_names_no_real_delimiter_does_not_check() {
        // The question moved. It used to be asked of a finished expansion, by
        // the gate, because a transformer named a delimiter as text; now
        // `Delimiter` is a type whose only values are the four, so a name that
        // is not one of them is refused where it is *written* — and the whole
        // expansion never runs.
        let written = expand_region(
            &transformer(
                r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
                r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                r"syntax_identifier(syntax_built(here, 2, 0), name)",
                r"syntax_group(syntax_built(here, 3, 0), Delimiter.Curly, children)",
            ),
            REGION,
            expansion(),
        );
        assert!(
            matches!(written, Err(ExpansionFailure::NotATransformer(_))),
            "a delimiter the fixed grouper does not have was accepted: {written:?}"
        );
    }

    #[test]
    fn a_token_kind_the_lexer_does_not_have_does_not_check() {
        let written = expand_region(
            &transformer(
                r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Zither, "")"#,
                r"syntax_token(syntax_built(here, 1, 0), kind, text)",
                r"syntax_identifier(syntax_built(here, 2, 0), name)",
                r"syntax_group(syntax_built(here, 3, 0), delimiter, children)",
            ),
            REGION,
            expansion(),
        );
        assert!(
            matches!(written, Err(ExpansionFailure::NotATransformer(_))),
            "a token kind the lexer never produces was accepted: {written:?}"
        );
    }

    #[test]
    fn every_token_kind_the_lexer_produces_is_nameable() {
        // The drift law. `TokenKind` *is* the lexer's own kind, so the case set
        // cannot disagree about what a kind means; what a hand-written list can
        // still do is fall behind, and a kind the lexer produces that the phase
        // cannot name would be a silent gap in an adapter's dispatch.
        for kind in musa_language::SyntaxKind::all() {
            let named = TOKEN_KINDS.iter().any(|(_, candidate)| *candidate == kind);
            assert_eq!(
                named,
                musa_language::TokenClass::of(kind).is_some(),
                "`{kind:?}` is a token kind the phase cannot name, or a node kind it can"
            );
        }
    }

    #[test]
    fn a_named_token_kind_is_the_kind_it_names() {
        for (name, kind) in TOKEN_KINDS {
            assert_eq!(token_kind_named(name), Some(*kind), "`{name}` named another kind");
        }
        assert_eq!(token_kind_named("Zither"), None, "an invented name found a kind");
        for delimiter in Delimiter::ALL {
            assert_eq!(
                Delimiter::named(delimiter.name()),
                Some(delimiter),
                "`{}` named another delimiter",
                delimiter.name()
            );
        }
        assert_eq!(Delimiter::named("Curly"), None, "an invented name found a delimiter");
    }

    #[test]
    fn a_derivation_is_its_three_components_and_nothing_else() {
        // The identity law, minted directly rather than through an expansion,
        // so that what is under test is the representation and not one
        // transformer's use of it.
        let origin = NodePath::root(expansion());
        let derived = Derived {
            origin: origin.clone(),
            quotation: 3,
            path: vec![0, 1],
        };
        assert_eq!(derived.path(), derived.path(), "one derivation gave two paths");
        for other in [
            Derived {
                origin: origin.child(0),
                ..derived.clone()
            },
            Derived {
                quotation: 4,
                ..derived.clone()
            },
            Derived {
                path: vec![0, 2],
                ..derived.clone()
            },
        ] {
            assert_ne!(
                other.path(),
                derived.path(),
                "two derivations that differ in one component gave one path"
            );
        }
        // And a derived path is never an input node's path, whatever the
        // components are: the steps are disjoint by construction.
        assert!(
            read().at(&derived.path()).is_none(),
            "a derived path addressed an input node"
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
            [match checked_expression(syntax_token(syntax_built(here, 4, 0), TokenKind.Integer, "1")) {
                Ok(node) -> node,
                Err(message) -> syntax_token(syntax_built(here, 5, 0), TokenKind.Error, message),
             }])"#;
        let produced = run(&transformer(
            r#"syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "")"#,
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
        assert_eq!(
            *kind,
            musa_language::SyntaxKind::Integer,
            "the gate refused a fragment it should have accepted"
        );
    }
}
