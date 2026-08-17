//! Adapter expansion, at its one fixed place in the compiler order
//! (`docs/notes/research/language-design-closure/26-language-design-decision.md`
//! §3.1).
//!
//! The order is fixed and no pass reaches backwards:
//!
//! 1. lex and group the file — `musa-language`, once, for everything;
//! 2. read the fixed module header;
//! 3. resolve the header's syntax imports against the resolved package graph;
//! 4. expand each named, delimited region — **this module**;
//! 5. parse each result as one ordinary expression;
//! 6. resolve ordinary names;
//! 7. infer and check types;
//! 8. lower to the private evaluation core; and
//! 9. evaluate to finite values.
//!
//! Steps 6 onwards are [`crate::elaborate`] and have no idea any of this
//! happened, which is the point: an adapter emits an expression and the
//! ordinary compiler checks it, so nothing an adapter writes is exempt from
//! anything the composer's own source must satisfy.
//!
//! **What the phase produces** is a second text — the composer's, with each
//! region replaced by the expression its adapter answered with — plus a
//! [`SourceMap`] back to the first. Everything the compiler goes on to say is
//! said about the second text and reported against the first: a composer never
//! reads a position in text they did not write, so a diagnostic that lands
//! inside an expansion is moved to the region that produced it.
//!
//! **What it cannot do** is the reason it is a phase and not a macro system. An
//! adapter is type-blind: it is handed a version, a region, and nothing else,
//! so it cannot read an expected type, a value from the importing module, a
//! file, a clock, or compiler state. It may answer with one expression and may
//! not answer with an import, a declaration, a module, or another region —
//! which is what makes the set of module declarations known *before* expansion
//! and breaks the expansion-resolution cycle. Adapter definitions contain no
//! regions, so termination is structural: there is no rank arithmetic here
//! because there is nothing to count.

use std::collections::{BTreeMap, BTreeSet};

use musa_language::ast::{AstNode as _, ImportStmt};
use musa_language::{SyntaxKind, SyntaxNode};

use crate::compile::{CompileOptions, SourceDocument};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;

/// The phase-tagged logical charges of `26-language-design-decision.md` §3.5.
///
/// Four counters, each charged by the phase that incurs it, and every one of
/// them a function of the *result* rather than of the work done to reach it.
/// That is what makes a cache hit cost what the miss it replaces cost: the
/// second of two identical regions is charged from the record, not from the
/// evaluation it skipped, so cache warmth cannot change whether a file is
/// accepted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Charges {
    /// One per region expanded, hit or miss.
    pub(crate) expansion_steps: u64,
    /// Nodes a transformer built rather than preserved.
    pub(crate) generated_syntax_nodes: u64,
    /// Equations the transformer's own checking solved.
    pub(crate) type_constraints: u64,
    /// Reductions the transformer's own evaluation took.
    pub(crate) evaluation_steps: u64,
}

impl Charges {
    fn add(&mut self, other: Self) {
        self.expansion_steps = self.expansion_steps.saturating_add(other.expansion_steps);
        self.generated_syntax_nodes = self.generated_syntax_nodes.saturating_add(other.generated_syntax_nodes);
        self.type_constraints = self.type_constraints.saturating_add(other.type_constraints);
        self.evaluation_steps = self.evaluation_steps.saturating_add(other.evaluation_steps);
    }
}

/// What one successful expansion leaves behind
/// (`26-language-design-decision.md` §3.4).
///
/// A compiler source map, and — as
/// `docs/rules/across-stages/02-derivation-diagrams.md` §6 puts it — a source of
/// `Generated` steps and nothing more. It says that an expression was produced
/// at a site by a named adapter at an exact version. It does not say that the
/// music the expression later makes was derived from anything: that claim is
/// the derivation graph's, and it starts at the use site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpansionRecord {
    /// The module path the header named: `std::adapters::doubled`.
    pub(crate) adapter: String,
    /// The exact source the adapter package was read at, as hex.
    pub(crate) version: String,
    /// The region, in the composer's own text.
    pub(crate) use_site: SourceSpan,
    /// What the adapter was handed.
    pub(crate) input: crate::syntax::Syntax,
    /// What it answered with.
    pub(crate) output: crate::syntax::Syntax,
    /// Every node of the region, by range, in the order `syntax_anchor`
    /// numbers them.
    ///
    /// The compiler's half of an anchor. The adapter emits a number into the
    /// value it produces; a later ordinary package function complaining about
    /// that value carries the number along, and this is what turns it back into
    /// a place in the composer's text. It lives on the record rather than in a
    /// compilation-wide map because a number means nothing without the region
    /// that minted it, and the record is what a reader already holds.
    pub(crate) anchors: Vec<SourceSpan>,
    /// The expansion this one was produced inside, if any.
    ///
    /// Always `None` today, and a field rather than an omission because the
    /// record's shape is fixed by §3.4 and a reader should not have to know
    /// which parts of it this build can reach. An adapter cannot emit a
    /// region, so an expansion has no children; if that rule is ever relaxed,
    /// this is where the parent goes.
    pub(crate) parent: Option<usize>,
}

impl ExpansionRecord {
    /// The range an anchor names, or `None` for a number this region never
    /// minted.
    ///
    /// Total for the reason the anchor is a number and not a range: a forged
    /// anchor addresses nothing here, so the worst it can do is leave a
    /// package's complaint without a place — which is a mislocated sentence,
    /// not a way to read text the adapter was never handed.
    ///
    /// Exercised by this prompt's tests; the trials of prompts 127dcf and
    /// 127dcg are what call it in earnest, when a package's `validate` starts
    /// carrying anchors into complaints a piece has to place.
    #[cfg(test)]
    pub(crate) fn anchor(&self, number: u64) -> Option<SourceSpan> {
        self.anchors.get(usize::try_from(number).ok()?).copied()
    }
}

/// Where one stretch of the expanded text came from in the composer's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Replacement {
    /// The stretch, in expanded coordinates.
    from: u32,
    to: u32,
    /// The region it replaced, in original coordinates.
    original: SourceSpan,
}

/// The translation from the text the compiler read to the text the composer
/// wrote.
///
/// Two rules and no more. A position outside every expansion moves by the
/// accumulated difference in length of the expansions before it, which is
/// exact. A position *inside* an expansion becomes the region that produced it,
/// because there is no finer answer that is true: the character is not in the
/// composer's file at all, and pointing at the region is pointing at the only
/// text they can edit to change it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceMap {
    replacements: Vec<Replacement>,
}

impl SourceMap {
    /// Whether this map changes anything, which it does not for the enormous
    /// majority of files: no region, no translation, no walk over a snapshot.
    pub(crate) fn is_identity(&self) -> bool {
        self.replacements.is_empty()
    }

    /// Where `offset` in the expanded text stands in the original.
    fn offset(&self, offset: u32) -> u32 {
        let mut shift: i64 = 0;
        for replacement in &self.replacements {
            if offset < replacement.from {
                break;
            }
            if offset < replacement.to {
                return replacement.original.start;
            }
            let generated = i64::from(replacement.to).saturating_sub(i64::from(replacement.from));
            let original = i64::from(replacement.original.end).saturating_sub(i64::from(replacement.original.start));
            shift = shift.saturating_add(original.saturating_sub(generated));
        }
        u32::try_from(i64::from(offset).saturating_add(shift)).unwrap_or(u32::MAX)
    }

    /// Where `span` in the expanded text stands in the original.
    ///
    /// A span that lies inside one expansion becomes that expansion's region,
    /// whole: half of a generated call is not a place, and a caret under it
    /// would be a caret under nothing.
    pub(crate) fn span(&self, span: SourceSpan) -> SourceSpan {
        for replacement in &self.replacements {
            if span.start >= replacement.from && span.end <= replacement.to {
                return replacement.original;
            }
        }
        SourceSpan::new(self.offset(span.start), self.offset(span.end))
    }

    /// The same, for a span a caller may not have.
    pub(crate) fn maybe(&self, span: Option<SourceSpan>) -> Option<SourceSpan> {
        span.map(|span| self.span(span))
    }
}

/// The phase's whole result.
pub(crate) struct Expansion {
    /// The text steps 5 onwards read.
    pub(crate) document: SourceDocument,
    /// How to say where any of it came from.
    pub(crate) map: SourceMap,
    /// Every successful expansion, in source order.
    pub(crate) records: Vec<ExpansionRecord>,
    /// What the phase charged.
    pub(crate) charges: Charges,
    /// Everything that went wrong, already anchored in the composer's text.
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Expansion {
    /// The result for a document with no region in it, which is every document
    /// the language has had until now.
    fn unchanged(source: &SourceDocument) -> Self {
        Self {
            document: SourceDocument::new(source.text(), source.name()),
            map: SourceMap::default(),
            records: Vec::new(),
            charges: Charges::default(),
            diagnostics: Vec::new(),
        }
    }

    /// The regions, as the derivation graph anchors a generated event at them.
    pub(crate) fn anchors(&self) -> Vec<crate::derivation::ExpansionAnchor> {
        self.records
            .iter()
            .map(|record| crate::derivation::ExpansionAnchor {
                site: record.use_site,
                adapter: record.adapter.clone(),
            })
            .collect()
    }
}

/// One syntax import from the fixed module header.
struct SyntaxImport {
    alias: String,
    path: String,
    /// Where the statement ends, which is what "before any definition that
    /// uses it" is checked against.
    ends: u32,
    /// The whole statement, which is where an adapter that promises more than
    /// it offers is refused.
    at: SourceSpan,
}

/// What an adapter promises, in `26-language-design-decision.md` §4's words.
///
/// Three levels and an order on them, because each one is the one below it plus
/// an operation: *readable* expands, *editable* also edits under the edit law,
/// *generative* also prints under the round-trip law. A level is what a
/// musician is told about a region — whether a control is greyed, whether an
/// interface may offer to make one — so it is declared by the adapter and
/// checked against what the module actually holds, rather than inferred from
/// which `let`s happen to be there. Inferring it would make adding a
/// half-finished `edit` silently promise the edit law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Expansion only: structured views of its regions are read-only.
    Readable,
    /// Expansion and `edit`, under the edit law.
    Editable,
    /// Editable, and `print` for new regions, under the round-trip law.
    Generative,
}

impl Level {
    /// The level a module declares, as it spells it.
    fn named(word: &str) -> Option<Self> {
        match word {
            "readable" => Some(Self::Readable),
            "editable" => Some(Self::Editable),
            "generative" => Some(Self::Generative),
            _ => None,
        }
    }

    /// The word for it, for a sentence a reader gets.
    fn word(self) -> &'static str {
        match self {
            Self::Readable => "readable",
            Self::Editable => "editable",
            Self::Generative => "generative",
        }
    }

    /// Everything a module at this level must declare.
    fn operations(self) -> &'static [&'static str] {
        match self {
            Self::Readable => &["expand"],
            Self::Editable => &["expand", "edit"],
            Self::Generative => &["expand", "edit", "print"],
        }
    }
}

/// Why a module could not be read as an adapter at the level it claims.
///
/// A message and a help, and no span: the same fault is reported at the import
/// that names the module and at a region the module reads, and which of those
/// the caret belongs on is the caller's question rather than this one's.
///
/// And causes, for the other half of the same argument. Which of the caller's
/// spans is right is the caller's question; which document the module's own
/// faults are in is not a question at all, so they travel whole rather than
/// being restated anywhere.
struct LevelFault {
    message: String,
    help: &'static str,
    /// Which complaint this is. A module that crossed a compilation limit
    /// while being read is a limit and says so, exactly as a stop during
    /// expansion does; everything else here is the module's own fault.
    code: Code,
    /// What the module's own checker said, when it said anything.
    causes: Vec<crate::diagnose::Cause>,
}

/// The level `adapter_source` declares, checked against what it offers.
///
/// The check `26-language-design-decision.md` §4 asks for: a declared level is
/// a promise, and a module that promises `generative` while declaring no
/// `print` has promised something no musician can rely on. Under-promising is
/// allowed and over-promising is not — a module may hold an `edit` it does not
/// advertise, and what governs is the word it wrote.
///
/// A module that declares no level at all is refused rather than defaulted.
/// "Declared and checked, not inferred" is the whole point: a default would be
/// the compiler deciding what a package promises.
///
/// `path` is the module as the importer wrote it, which is what the messages
/// name it by; `document` is the key the import resolved to, which is what a
/// cause is filed under, what a renderer looks the text up with, and what the
/// module's own imports resolve against.
fn level_of(
    adapter_source: &str,
    path: &str,
    document: &str,
    sources: &crate::imports::ImportSources,
) -> Result<Level, LevelFault> {
    let plain = |message: String, help: &'static str| LevelFault {
        message,
        help,
        code: Code::Expansion,
        causes: Vec::new(),
    };
    let module = crate::core::read_adapter_module(adapter_source, crate::core::PhaseImports::at(document, sources))
        .map_err(|fault| match fault {
            crate::core::ModuleFault::Stopped => LevelFault {
                message: format!("reading `{path}` crossed a compilation limit"),
                help: "an adapter is total, so this is a limit rather than a loop",
                code: Code::ResourceLimit,
                // A read that ran out of budget said nothing about the module, so
                // there is nothing to carry.
                causes: Vec::new(),
            },
            // The wrapper's own sentence, and not a word of the module's spliced
            // into it: the causes below say what is wrong inside the module, each
            // at its own place in it, and a summary here would say the first one
            // twice and the rest not at all.
            crate::core::ModuleFault::Broken(diagnostics) => LevelFault {
                message: format!("`{path}` is not an adapter module"),
                help: "an adapter module is a `library` of ordinary declarations, checked in the expansion phase",
                code: Code::Expansion,
                causes: diagnostics
                    .into_iter()
                    .map(|diagnostic| crate::diagnose::Cause::of(document, diagnostic))
                    .collect(),
            },
        })?;
    let declared = module.text("level").ok_or_else(|| {
        plain(
            format!("`{path}` declares no conformance level"),
            "an adapter module declares `let level = \"readable\";`, `\"editable\"`, or `\"generative\"`",
        )
    })?;
    let level = Level::named(&declared).ok_or_else(|| {
        plain(
            format!("`{path}` declares the level `{declared}`, which is not one of the three"),
            "the levels are `readable`, `editable`, and `generative`, and each is the one before it plus an operation",
        )
    })?;
    for operation in level.operations() {
        if !module.declares(operation) {
            return Err(plain(
                format!("`{path}` declares the {} level and no `{operation}`", level.word()),
                "a level is a promise: declare the level the module reaches, or write the operation it names",
            ));
        }
    }
    Ok(level)
}

/// Run step 4 of the fixed order over `source`.
pub(crate) fn expand(source: &SourceDocument, options: &CompileOptions) -> Expansion {
    let parsed = musa_language::parse(source.text());
    let root = parsed.syntax();
    let regions: Vec<SyntaxNode> = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::SyntaxRegion)
        .collect();
    let imports = syntax_imports(&root);
    if regions.is_empty() && imports.is_empty() {
        return Expansion::unchanged(source);
    }
    let mut expansion = Expansion::unchanged(source);
    // The declared level, checked where the module is imported rather than
    // where one of its regions stands. A promise is made by importing the
    // package, so a file that overstates one is wrong whether or not it went on
    // to write a region.
    for import in &imports {
        let uri = crate::imports::resolve_import(source.name(), &import.path);
        let Some(adapter_source) = options.imports.get(&uri) else {
            // Reported at the region that needed it, where a reader can see
            // what the missing module was for.
            continue;
        };
        if let Err(fault) = level_of(adapter_source, &import.path, &uri, &options.imports) {
            expansion.diagnostics.push(
                Diagnostic::error(fault.code, fault.message)
                    .at(import.at, "this import")
                    .help(fault.help)
                    .caused_by(fault.causes),
            );
        }
    }
    if regions.is_empty() {
        return expansion;
    }
    let written = names_written(source.text());
    let mut cache: BTreeMap<(String, String, String), Cached> = BTreeMap::new();
    let mut text = String::with_capacity(source.text().len());
    let mut copied = 0usize;
    for (ordinal, region) in regions.iter().enumerate() {
        let site = crate::resolve::trimmed_span(region);
        let produced = expand_one(region, ordinal, site, &imports, &written, options, source, &mut cache);
        let (printed, record, charges) = match produced {
            Ok(answer) => answer,
            Err(complaint) => {
                // Charged either way: a region that was read and refused cost
                // what reading it cost, and only the ones refused before any
                // adapter ran are free.
                expansion.charges.add(complaint.charges);
                expansion.diagnostics.push(*complaint.diagnostic);
                continue;
            }
        };
        expansion.charges.add(charges);
        // The composer's text up to the region, then the expansion in its
        // place. Regions never nest — a region's interior is not parsed as
        // expressions, so nothing inside one can be a second region — which is
        // what makes this a single left-to-right splice.
        let start = usize::try_from(site.start)
            .unwrap_or(usize::MAX)
            .min(source.text().len());
        let end = usize::try_from(site.end).unwrap_or(usize::MAX).min(source.text().len());
        text.push_str(source.text().get(copied..start).unwrap_or_default());
        let from = u32::try_from(text.len()).unwrap_or(u32::MAX);
        text.push_str(&printed);
        let to = u32::try_from(text.len()).unwrap_or(u32::MAX);
        copied = end;
        expansion.map.replacements.push(Replacement {
            from,
            to,
            original: site,
        });
        expansion.records.push(record);
    }
    text.push_str(source.text().get(copied..).unwrap_or_default());
    expansion.document = SourceDocument::new(text, source.name());
    expansion
}

/// One replacement an adapter asked for, in the composer's own file.
///
/// Byte offsets into the document the command was asked against, so a caller
/// applies them the way it applies every other structured edit. The adapter
/// never saw these numbers: it named a node by anchor, and the region's own
/// table turned that into a range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterEdit {
    /// Byte offset the replaced text starts at.
    pub start: u32,
    /// Byte offset it ends at.
    pub end: u32,
    /// What goes in its place.
    pub text: String,
}

/// Why an adapter command produced no edit.
///
/// Four different situations, and a musician reading one needs to be told
/// which: a region that is read-only *by declaration* is not a broken adapter,
/// and an adapter that refused a command is not a compiler fault. Only the last
/// is a fault at all.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterEditError {
    /// No adapter region stands at that offset.
    NoRegion,
    /// The adapter declares no `edit`.
    ///
    /// `26-language-design-decision.md` §4's *readable* level: its regions are
    /// read-only, and that is the level rather than a failure.
    ReadOnly {
        /// The adapter package, as the header names it.
        adapter: String,
    },
    /// The adapter read the command and would not serve it — its own sentence.
    Refused {
        /// The adapter package, as the header names it.
        adapter: String,
        /// What it said.
        message: String,
    },
    /// The adapter, or the command, is broken; the diagnostic says how.
    Broken(Box<Diagnostic>),
}

impl std::fmt::Display for AdapterEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NoRegion => formatter.write_str("no adapter region stands here"),
            Self::ReadOnly { ref adapter } => {
                write!(
                    formatter,
                    "`{adapter}` declares no `edit`, so its regions are read-only"
                )
            }
            Self::Refused {
                ref adapter,
                ref message,
            } => write!(formatter, "`{adapter}`: {message}"),
            Self::Broken(ref diagnostic) => formatter.write_str(&diagnostic.message),
        }
    }
}

impl std::error::Error for AdapterEditError {}

/// Ask the adapter that reads the region at `at` to serve one command.
///
/// The second of `26-language-design-decision.md` §4's declared operations,
/// from the outside. The command is its name, the anchor of the item it is
/// about, and one text argument; the answer is replacements in the composer's
/// own file, which a caller applies the way it applies every other structured
/// edit and then compiles before committing.
///
/// **The edit law holds by construction and is checked anyway.** *Locality*:
/// every range comes from the region's own anchor table, so it lies inside the
/// region — an anchor that names no node of this region is refused rather than
/// clamped. *Preservation*: only the named ranges are returned, so every byte
/// outside them is untouched; nothing here regenerates a region from a value,
/// which §4 forbids outright. *Agreement* is the caller's to keep and the
/// tests' to prove: re-expanding the patched region must give what applying the
/// command to the value gives.
///
/// The run is metered by the ordinary meter, so a runaway command is stopped by
/// the same budget an expansion is. It charges nothing, because an edit is a
/// question about a document rather than a step in compiling one.
///
/// # Errors
/// [`AdapterEditError`], which distinguishes a region that is read-only by
/// declaration, an adapter's own refusal, and an adapter that is broken.
pub fn adapter_edits(
    source: &SourceDocument,
    options: &CompileOptions,
    at: u32,
    command: &str,
    anchor: u64,
    argument: &str,
) -> Result<Vec<AdapterEdit>, AdapterEditError> {
    let parsed = musa_language::parse(source.text());
    let root = parsed.syntax();
    let regions: Vec<SyntaxNode> = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::SyntaxRegion)
        .collect();
    let (ordinal, region) = regions
        .iter()
        .enumerate()
        .find(|(_, region)| {
            let span = crate::resolve::trimmed_span(region);
            span.start <= at && at <= span.end
        })
        .ok_or(AdapterEditError::NoRegion)?;
    let site = crate::resolve::trimmed_span(region);
    let imports = syntax_imports(&root);
    let name = region_name(region).ok_or(AdapterEditError::NoRegion)?;
    let import = imports
        .iter()
        .find(|import| import.alias == name)
        .ok_or(AdapterEditError::NoRegion)?;
    let uri = crate::imports::resolve_import(source.name(), &import.path);
    let adapter_source = options.imports.get(&uri).ok_or_else(|| {
        AdapterEditError::Broken(Box::new(refusal(
            site,
            format!("`{}` is not a module this compilation can read", import.path),
            "check the package path, or provide the file the import names",
        )))
    })?;
    // The level decides, not the presence of an `edit`. A module that declares
    // *readable* has said its regions are read-only, and an `edit` it did not
    // advertise does not quietly make them writable.
    let level = level_of(adapter_source, &import.path, &uri, &options.imports).map_err(|fault| {
        AdapterEditError::Broken(Box::new(
            refusal(site, fault.message, fault.help).caused_by(fault.causes),
        ))
    })?;
    if level < Level::Editable {
        return Err(AdapterEditError::ReadOnly {
            adapter: import.path.clone(),
        });
    }
    let body = region_body(region).ok_or_else(|| {
        AdapterEditError::Broken(Box::new(refusal(
            site,
            "this region is not delimited",
            "write the region's contents inside `{ … }`",
        )))
    })?;
    // The same reading the expansion gets, at the same ordinal, so the anchors
    // a command names are the anchors the expansion minted.
    let subject = crate::syntax::read_region(
        &body,
        crate::syntax::ExpansionPath::at(vec![u32::try_from(ordinal).unwrap_or(u32::MAX)]),
    );
    let anchors = subject.spans(site);
    let (answer, _work) = crate::core::edit_syntax(
        adapter_source,
        crate::core::PhaseImports::at(&uri, &options.imports),
        subject,
        command,
        anchor,
        argument,
    );
    let patches = answer.map_err(|failure| match failure {
        crate::core::EditFailure::Refused(message) => AdapterEditError::Refused {
            adapter: import.path.clone(),
            message,
        },
        crate::core::EditFailure::Stopped => AdapterEditError::Broken(Box::new(
            Diagnostic::error(
                Code::ResourceLimit,
                format!("editing this region with `{}` crossed a compilation limit", import.path),
            )
            .at(site, "this region")
            .help("the adapter is total, so this is a limit rather than a loop"),
        )),
        crate::core::EditFailure::NotAnEditor(_) => AdapterEditError::Broken(Box::new(refusal(
            site,
            format!("`{}`'s `edit` is not an editor", import.path),
            "an adapter module declares `let edit = fn (region: Syntax<TokenTree>, command: Text, anchor: Nat, argument: Text) -> Result<List<Pair<Nat, Text>>, Text> { … };`",
        ))),
        crate::core::EditFailure::NoAnswer => AdapterEditError::Broken(Box::new(refusal(
            site,
            format!("`{}` did not answer this command", import.path),
            "the adapter checked and then produced nothing, which is a fault in the adapter",
        ))),
    })?;
    patches
        .into_iter()
        .map(|(anchor, text)| {
            let named = usize::try_from(anchor)
                .ok()
                .and_then(|anchor| anchors.get(anchor))
                .copied()
                .ok_or_else(|| {
                    AdapterEditError::Broken(Box::new(refusal(
                        site,
                        format!("`{}` asked to replace a node this region does not have", import.path),
                        "an edit names a node the adapter was given, by the anchor the region minted for it",
                    )))
                })?;
            // Locality, checked rather than assumed. It cannot fail while the
            // table is the region's own nodes, and a check that cannot fail
            // today is what keeps it true when the table stops being that.
            if named.start < site.start || named.end > site.end {
                return Err(AdapterEditError::Broken(Box::new(refusal(
                    site,
                    format!("`{}` asked to change text outside the region", import.path),
                    "an adapter edits its own region and nothing else",
                ))));
            }
            Ok(AdapterEdit {
                start: named.start,
                end: named.end,
                text,
            })
        })
        .collect()
}

/// Why an adapter wrote no region for a value.
///
/// Three situations and a musician reading one needs to be told which. Only the
/// last is a fault: an adapter below *generative* was never going to write one,
/// and a stated loss is the printer working.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterPrintError {
    /// The adapter does not declare the *generative* level.
    ///
    /// `26-language-design-decision.md` §4: a printer is what creates a region
    /// where none exists, and an adapter that declares no printer offers no way
    /// to start one — which is a level rather than a failure.
    NotGenerative {
        /// The adapter package, as the header names it.
        adapter: String,
        /// What it declares instead.
        level: String,
    },
    /// The adapter read the value and could not write all of it down.
    ///
    /// §4's `PrintLoss`, in the adapter's own words. A printer that silently
    /// dropped what it could not spell would satisfy the round-trip law by
    /// making the value smaller, so a loss is an answer and never a discard.
    Loss {
        /// The adapter package, as the header names it.
        adapter: String,
        /// What it said it could not write.
        message: String,
    },
    /// The adapter, or the value it was handed, is broken.
    Broken(Box<Diagnostic>),
}

impl std::fmt::Display for AdapterPrintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NotGenerative { ref adapter, ref level } => {
                write!(formatter, "`{adapter}` is {level}, so it writes no new region")
            }
            Self::Loss {
                ref adapter,
                ref message,
            } => write!(formatter, "`{adapter}`: {message}"),
            Self::Broken(ref diagnostic) => formatter.write_str(&diagnostic.message),
        }
    }
}

impl std::error::Error for AdapterPrintError {}

/// Ask a *generative* adapter to write a region for one value.
///
/// The third of `26-language-design-decision.md` §4's declared operations. The
/// answer is the region's **contents** — what goes between the braces of
/// `syntax <name> { … }` — because the region's name is the importing file's
/// word for the package and not the package's word for itself.
///
/// `value` is the value as an ordinary expression, and `at` is the document the
/// region will be written into, which is what the module path is resolved
/// against. Unlike expansion and editing this reaches no region: a printer runs
/// where none exists yet, which is the whole of what it is for.
///
/// **The round-trip law.** For a value the printer accepts, expanding the
/// printed region and evaluating it gives an adapter-equal value. Equality is
/// the package's to state. The law claims nothing about source, comments,
/// layout, or origin — a printed region is new text, and §4 is explicit that a
/// printer does not thereby make an existing region safely editable. That is
/// [`adapter_edits`]' job and stays there.
///
/// # Errors
/// [`AdapterPrintError`], which tells a level below *generative* and a stated
/// loss apart from an adapter that is broken.
pub fn adapter_print(
    at: &SourceDocument,
    options: &CompileOptions,
    adapter: &str,
    value: &str,
) -> Result<String, AdapterPrintError> {
    // Spanless, and every fault below with it: there is no region to point at
    // yet, and a caret over the file's first byte would be a place the reader
    // would go and find nothing.
    let broken_by = |message: String, help: &'static str, causes: Vec<crate::diagnose::Cause>| {
        AdapterPrintError::Broken(Box::new(
            Diagnostic::error(Code::Expansion, message).help(help).caused_by(causes),
        ))
    };
    let broken = |message: String, help: &'static str| broken_by(message, help, Vec::new());
    let uri = crate::imports::resolve_import(at.name(), adapter);
    let adapter_source = options.imports.get(&uri).ok_or_else(|| {
        broken(
            format!("`{adapter}` is not a module this compilation can read"),
            "check the package path, or provide the file the import names",
        )
    })?;
    let level = level_of(adapter_source, adapter, &uri, &options.imports)
        .map_err(|fault| broken_by(fault.message, fault.help, fault.causes))?;
    if level < Level::Generative {
        return Err(AdapterPrintError::NotGenerative {
            adapter: adapter.to_owned(),
            level: level.word().to_owned(),
        });
    }
    crate::core::print_value(
        adapter_source,
        crate::core::PhaseImports::at(&uri, &options.imports),
        at,
        value,
    )
    .map_err(|failure| match failure {
        crate::core::PrintFailure::Loss(message) => AdapterPrintError::Loss {
            adapter: adapter.to_owned(),
            message,
        },
        crate::core::PrintFailure::Stopped => AdapterPrintError::Broken(Box::new(
            Diagnostic::error(
                Code::ResourceLimit,
                format!("writing a region with `{adapter}` crossed a compilation limit"),
            )
            .help("the adapter is total, so this is a limit rather than a loop"),
        )),
        // The checker's own sentences, restated against the adapter, for the
        // reason `level_of` restates its own: "does not read this value" says
        // which two things disagreed and not *how*, and the how is the only part
        // an adapter author can act on.
        //
        // Their coordinates are dropped rather than published. A printer and its
        // subject are read together in a piece neither of them is written in
        // (`crate::core::print_value`), so a span here is a position in text no
        // consumer can open — and the adapter is a file a consumer *can* open,
        // which is what would make an offset into the reading land somewhere
        // real and wrong.
        crate::core::PrintFailure::NotAPrinter(why) => broken_by(
            format!("`{adapter}`'s `print` does not read this value"),
            "a printer is `fn (value: T) { … }` answering `Ok(text)` or `Err(loss)`, where `T` is the type its regions \
             produce",
            why.into_iter().map(|diagnostic| said_by(adapter, diagnostic)).collect(),
        ),
        crate::core::PrintFailure::NoAnswer => broken(
            format!("`{adapter}` did not answer for this value"),
            "the adapter checked and then produced nothing, which is a fault in the adapter",
        ),
    })
}

/// One complaint about a reading, filed against `adapter` and stripped of its
/// coordinates.
///
/// [`crate::diagnose::Cause::of`] everywhere else keeps the labels, because
/// everywhere else the offsets are offsets into a file the reader can open. Here
/// they are offsets into the piece [`crate::core::print_value`] wrote to read the
/// printer and the value together, and `adapter` is a real file: kept, they would
/// resolve against text they did not come from and point somewhere plausible and
/// wrong. The sentence is what an adapter author acts on, and it survives.
fn said_by(adapter: &str, diagnostic: Diagnostic) -> crate::diagnose::Cause {
    crate::diagnose::Cause {
        labels: Vec::new(),
        ..crate::diagnose::Cause::of(adapter, diagnostic)
    }
}

/// One region's answer, kept so a second identical region costs the same.
#[derive(Clone)]
struct Cached {
    printed: String,
    output: crate::syntax::Syntax,
    charges: Charges,
}

/// A region that was not expanded, and what deciding that cost.
///
/// The charge travels with the refusal because a refusal is an answer: an
/// adapter that reads a whole region and then will not have it has read a
/// whole region, and a phase that charged nothing for that would let a file
/// buy unbounded reading by arranging to be refused. Everything the phase
/// refuses *before* running an adapter — a missing import, an adapter written
/// with an adapter — costs nothing and says so, which is what the `From`
/// below is for.
struct Refused {
    diagnostic: Box<Diagnostic>,
    charges: Charges,
}

impl From<Box<Diagnostic>> for Refused {
    fn from(diagnostic: Box<Diagnostic>) -> Self {
        Self {
            diagnostic,
            charges: Charges::default(),
        }
    }
}

/// Expand one region, or say why it cannot be.
#[expect(
    clippy::too_many_arguments,
    reason = "an expansion is a function of exactly these things and of nothing else, which is the property §3.4 calls type blindness; bundling them into a context struct would hide which of them the phase may read"
)]
fn expand_one(
    region: &SyntaxNode,
    ordinal: usize,
    site: SourceSpan,
    imports: &[SyntaxImport],
    written: &BTreeSet<String>,
    options: &CompileOptions,
    source: &SourceDocument,
    cache: &mut BTreeMap<(String, String, String), Cached>,
) -> Result<(String, ExpansionRecord, Charges), Refused> {
    let name = region_name(region).ok_or_else(|| {
        Box::new(refusal(
            site,
            "this region names no adapter",
            "write `syntax <name> { … }`, where the name is one a syntax import gave",
        ))
    })?;
    let import = imports.iter().find(|import| import.alias == name).ok_or_else(|| {
        Box::new(refusal(
            site,
            format!("no syntax import names `{name}`"),
            "add a syntax import for it to the header, and give it this name with `as`",
        ))
    })?;
    if import.ends > site.start {
        return Err(Refused::from(Box::new(refusal(
            site,
            format!("`{name}` is imported after the region that uses it"),
            "a syntax import stands in the fixed module header, before any definition that uses it",
        ))));
    }
    let uri = crate::imports::resolve_import(source.name(), &import.path);
    let adapter_source = options.imports.get(&uri).ok_or_else(|| {
        Box::new(refusal(
            site,
            format!("`{}` is not a module this compilation can read", import.path),
            "check the package path, or provide the file the import names",
        ))
    })?;
    let version = digest(adapter_source);
    let body = region_body(region).ok_or_else(|| {
        Box::new(refusal(
            site,
            "this region is not delimited",
            "write the region's contents inside `{ … }`",
        ))
    })?;
    let interior = body.text().to_string();
    let subject = crate::syntax::read_region(
        &body,
        crate::syntax::ExpansionPath::at(vec![u32::try_from(ordinal).unwrap_or(u32::MAX)]),
    );

    // A function of the region alone, so the cache hit and the cache miss build
    // the same table from the same region and neither has to remember it.
    let anchors = subject.spans(site);

    // The document as well as its digest, because a module's own imports
    // resolve against the document it was read from: two copies of one adapter
    // under two paths hash the same and do not read the same modules, and a
    // cache that said they did would be the second path this prompt's audit
    // exists to find.
    let key = (uri.clone(), version.clone(), interior);
    if let Some(hit) = cache.get(&key) {
        // The charge is replayed from the record, so the second of two
        // identical regions costs exactly what the first did.
        let record = ExpansionRecord {
            adapter: import.path.clone(),
            version,
            use_site: site,
            input: subject,
            output: hit.output.clone(),
            anchors,
            parent: None,
        };
        return Ok((hit.printed.clone(), record, hit.charges));
    }

    let (answer, work) = crate::core::expand_syntax(
        adapter_source,
        crate::core::PhaseImports::at(&uri, &options.imports),
        &subject,
    );
    // The run happened, so the run is charged, and everything below reports
    // against the same charge whether the adapter answered or refused.
    let charged = |generated_syntax_nodes| Charges {
        expansion_steps: 1,
        generated_syntax_nodes,
        type_constraints: work.type_constraints,
        evaluation_steps: work.evaluation_steps,
    };
    let output = match answer {
        Ok(output) => output,
        Err(failure) => {
            return Err(Refused {
                diagnostic: Box::new(stopped_or_refused(&failure, &import.path, site)),
                charges: charged(0),
            });
        }
    };
    let printed = crate::syntax::print(&output);
    ordinary_expression(&printed.text, site)?;
    for generated in &printed.generated_names {
        if written.contains(generated) {
            return Err(Refused::from(Box::new(refusal(
                site,
                format!("the expansion's own name `{generated}` is also written in this file"),
                "rename the file's binding: a name an adapter introduced must not be one the composer can reach",
            ))));
        }
    }
    let charges = charged(printed.generated_nodes);
    cache.insert(
        key,
        Cached {
            printed: printed.text.clone(),
            output: output.clone(),
            charges,
        },
    );
    let record = ExpansionRecord {
        adapter: import.path.clone(),
        version,
        use_site: site,
        input: subject,
        output,
        anchors,
        parent: None,
    };
    Ok((printed.text, record, charges))
}

/// Why a transformer produced no expression, told apart from *what* it
/// produced.
///
/// Three different things wear one code and must not read alike. A run the
/// meter stopped is a resource limit and says so, with the code the rest of
/// the compiler uses for it — that distinction is the budget-independence
/// law's (`docs/rules/language/00-semantics.md` §2): a smaller budget may stop
/// a file, and if it reported the stop as an expansion fault instead, a
/// narrowed run would look like a file that is not well-typed. An adapter that
/// answered `Err` is not a fault at all: it read the region and would not have
/// it, and what a musician needs to read is the adapter's own sentence about
/// their own text. What is left is the adapter being broken.
fn stopped_or_refused(failure: &crate::core::ExpansionFailure, path: &str, site: SourceSpan) -> Diagnostic {
    if let crate::core::ExpansionFailure::Refused { message, at } = failure {
        // The adapter pointed. Where it pointed at the composer's own text,
        // that is where the caret goes; where it pointed at a node it built,
        // there is nothing under it, and saying so is better than a caret that
        // silently means less than it looks like it means.
        return match *at {
            Some(span) => Diagnostic::error(Code::Expansion, format!("`{path}`: {message}"))
                .at(span, "this")
                .help("the adapter read the region and refused it: this is the package's rule, not the compiler's"),
            None => Diagnostic::error(Code::Expansion, format!("`{path}`: {message}"))
                .at(site, "this region")
                .help("the adapter pointed at a node it built rather than at one it was given, so the whole region is as close as the report can get"),
        };
    }
    if matches!(*failure, crate::core::ExpansionFailure::Stopped) {
        return Diagnostic::error(
            Code::ResourceLimit,
            format!("expanding this region with `{path}` crossed a compilation limit"),
        )
        .at(site, "this region")
        .help("the adapter is total, so this is a limit rather than a loop: give the region less to read");
    }
    refusal(
        site,
        format!("`{path}` did not expand this region"),
        match *failure {
            crate::core::ExpansionFailure::Stopped | crate::core::ExpansionFailure::Refused { .. } => "handled above",
            crate::core::ExpansionFailure::NotATransformer(_) => {
                "the adapter's `expand` is not a transformer — it must take one region and answer with one"
            }
            crate::core::ExpansionFailure::NoAnswer => "the adapter did not answer",
            crate::core::ExpansionFailure::NotAnExpression(_) => {
                "the adapter answered with something that is not a well-formed expression"
            }
        },
    )
}

/// Step 5: the answer is one ordinary expression, or it is refused.
///
/// Checked by parsing it the way the composer's own source would be parsed,
/// which is what makes the four forbidden emissions one rule rather than four:
/// an `import`, a `mod`, a `data`, or a `let` is not an expression, so a
/// transformer that emits one does not get past the ordinary parser. The one
/// thing that *is* an expression and still may not be emitted is another
/// region, so that is asked separately.
fn ordinary_expression(text: &str, site: SourceSpan) -> Result<(), Box<Diagnostic>> {
    let parsed = crate::syntax::read_expression(text);
    if !parsed.errors().is_empty() {
        return Err(Box::new(refusal(
            site,
            "the adapter's answer is not one ordinary expression",
            "an adapter may emit an expression and the contents of an expression block, and no declaration, import, or module",
        )));
    }
    if parsed
        .syntax()
        .descendants()
        .any(|node| node.kind() == SyntaxKind::SyntaxRegion)
    {
        return Err(Box::new(refusal(
            site,
            "the adapter's answer contains another adapter region",
            "an adapter may not emit a region: expansion terminates because adapter definitions and their answers are region-free, not because a rank was counted",
        )));
    }
    Ok(())
}

/// The adapter name a region is written with.
fn region_name(region: &SyntaxNode) -> Option<String> {
    region
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

/// The delimited group a region holds.
fn region_body(region: &SyntaxNode) -> Option<SyntaxNode> {
    region.children().find(|node| node.kind() == SyntaxKind::SyntaxGroup)
}

/// Step 3: the header's syntax imports, in the order they are written.
///
/// The path is the one [`ImportStmt::path`] reads, not one rebuilt from the
/// statement's tokens: an import writes a package path bare and a relative path
/// in quotes, and whoever owns the filesystem keys the file's text under the
/// unquoted path. A phase that spelled the path a second way would ask for
/// `"adapter.musa"`, quotes and all, and never find the file that is right
/// there.
fn syntax_imports(root: &SyntaxNode) -> Vec<SyntaxImport> {
    root.descendants()
        .filter_map(ImportStmt::cast)
        .filter(ImportStmt::changes_syntax)
        .filter_map(|import| {
            let node = import.syntax();
            Some(SyntaxImport {
                alias: import.alias()?,
                path: import.path()?,
                ends: u32::from(node.text_range().end()),
                at: crate::resolve::trimmed_span(node),
            })
        })
        .collect()
}

/// Every identifier the composer's own file writes.
///
/// The other half of hygiene. The printer gives a generated binding a name no
/// other generated binding has; this is how the phase knows that name is also
/// one the composer cannot already reach, which is the part a renaming alone
/// cannot promise once the expansion has become ordinary text.
fn names_written(source: &str) -> BTreeSet<String> {
    musa_language::lex(source)
        .tokens()
        .iter()
        .filter(|token| token.kind == SyntaxKind::Identifier)
        .filter_map(|token| {
            let from = usize::try_from(u32::from(token.range.start())).ok()?;
            let to = usize::try_from(u32::from(token.range.end())).ok()?;
            source.get(from..to).map(str::to_owned)
        })
        .collect()
}

/// The exact source an adapter package was read at.
///
/// A content digest, because "exact version" has to mean the bytes that were
/// actually expanded: a package that changes and keeps its version number
/// would otherwise replay a stale record as though it were current.
fn digest(source: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// One refusal, anchored in the composer's text.
///
/// A caller boxes it: these are only ever an `Err`, and a [`Diagnostic`]
/// carries its labels and its fix, so a `Result` holding one inline would be
/// as wide as its failure everywhere it is returned.
fn refusal(site: SourceSpan, message: impl Into<String>, help: &str) -> Diagnostic {
    Diagnostic::error(Code::Expansion, message)
        .at(site, "this region")
        .help(help)
}

#[cfg(test)]
// A law suite reports a violated law by failing, which is what `panic!` and
// `expect` are for here.
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A piece that writes one region holding `contents`.
    fn piece(contents: &str) -> String {
        format!(
            "piece \"laws\" {{\n    import syntax std::adapters::doubled as doubled;\n\n    let it = syntax doubled {{ {contents} }};\n\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
        )
    }

    fn run(source: &str) -> Expansion {
        expand(&SourceDocument::new(source, "laws.musa"), &CompileOptions::default())
    }

    fn messages(expansion: &Expansion) -> Vec<String> {
        expansion
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    }

    /// A transformer that answers `Ok` with `emitted`, whatever the region held.
    fn answering(emitted: &str) -> String {
        format!(
            "fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(syntax_fold_from_leaves(fn (here) {{ syntax_token(syntax_built(here, 0, 0), TokenKind.Error, \"\") }}, \
             fn (here, kind, text) {{ syntax_token(syntax_built(here, 1, 0), kind, text) }}, \
             fn (here, name) {{ syntax_identifier(syntax_built(here, 2, 0), name) }}, \
             fn (here, delimiter, children) {{ {emitted} }}, region)) }}"
        )
    }

    /// The smallest adapter module holding `expand`.
    ///
    /// A law about the fold or about a builder is about that one expression,
    /// and making each such test write a whole `library` around it would bury
    /// the law in ceremony. Every test about a *module* — its scope, its own
    /// declarations, its level — writes the module out.
    fn module(expand: &str) -> String {
        format!("library {{\n    let level = \"readable\";\n\n    let expand = {expand};\n}}\n")
    }

    /// Expand one region's worth of text through `transformer`, and print it.
    fn answer(transformer: &str, region: &str) -> Result<crate::syntax::Printed, crate::core::ExpansionFailure> {
        let read = musa_language::parse(region);
        let subject = crate::syntax::read_region(&read.syntax(), crate::syntax::ExpansionPath::at(vec![0]));
        crate::core::expand_syntax(&module(transformer), crate::core::PhaseImports::bundled(), &subject)
            .0
            .map(|output| crate::syntax::print(&output))
    }

    #[test]
    fn one_region_expands_to_one_ordinary_expression() {
        let expansion = run(&piece("c4"));
        assert!(messages(&expansion).is_empty(), "{:?}", messages(&expansion));
        assert!(
            expansion.document.text().contains("repeat"),
            "the adapter's answer stands where the region was:\n{}",
            expansion.document.text()
        );
        assert!(
            !expansion.document.text().contains("syntax doubled"),
            "and the region does not stand there any more"
        );
        assert_eq!(expansion.records.len(), 1);
        assert_eq!(expansion.charges.expansion_steps, 1);
    }

    #[test]
    fn expansion_is_deterministic_and_depends_on_nothing_but_the_region() {
        let source = piece("c4");
        let once = run(&source);
        let twice = run(&source);
        assert_eq!(once.document.text(), twice.document.text());
        assert_eq!(once.records, twice.records);
        assert_eq!(once.charges, twice.charges);
        // Locality: the same region in a different piece answers the same. The
        // adapter cannot read the file around it, so there is nothing else for
        // the answer to depend on.
        let elsewhere = run(
            "piece \"other\" {\n    import syntax std::adapters::doubled as doubled;\n\n    let unrelated = 7;\n\n    let it = syntax doubled { c4 };\n\n    score { part p { voice v { d4/1 } } }\n}\n",
        );
        assert_eq!(
            once.records.first().map(|record| &record.output),
            elsewhere.records.first().map(|record| &record.output),
            "one region, one answer, wherever it is written"
        );
    }

    #[test]
    fn a_cache_hit_charges_what_the_miss_it_replaces_charged() {
        let one = run(&piece("c4"));
        let two = run(
            "piece \"laws\" {\n    import syntax std::adapters::doubled as doubled;\n\n    let a = syntax doubled { c4 };\n    let b = syntax doubled { c4 };\n\n    score { part p { voice v { c4/1 } } }\n}\n",
        );
        assert!(messages(&two).is_empty(), "{:?}", messages(&two));
        assert_eq!(two.records.len(), 2);
        assert_eq!(
            two.charges.expansion_steps,
            one.charges.expansion_steps.saturating_mul(2),
            "the second region was a cache hit and is charged for anyway"
        );
        assert_eq!(
            two.charges.evaluation_steps,
            one.charges.evaluation_steps.saturating_mul(2),
            "cache warmth cannot change what a file costs, so it cannot change whether it is accepted"
        );
        assert_eq!(
            two.charges.generated_syntax_nodes,
            one.charges.generated_syntax_nodes.saturating_mul(2)
        );
        assert_eq!(
            two.charges.type_constraints,
            one.charges.type_constraints.saturating_mul(2)
        );
    }

    #[test]
    fn a_region_whose_adapter_is_not_imported_is_refused_at_the_region() {
        let source =
            "piece \"laws\" {\n    let it = syntax doubled { c4 };\n\n    score { part p { voice v { c4/1 } } }\n}\n";
        let expansion = run(source);
        let complaint = expansion.diagnostics.first().expect("one refusal");
        assert!(complaint.message.contains("no syntax import names"));
        let span = complaint.primary_span().expect("a place");
        let at = source.get(span.start as usize..span.end as usize).unwrap_or_default();
        assert!(
            at.starts_with("syntax doubled"),
            "the refusal points at the region: {at}"
        );
    }

    #[test]
    fn a_syntax_import_written_after_the_region_that_uses_it_is_refused() {
        let expansion = run(
            "piece \"laws\" {\n    let it = syntax doubled { c4 };\n\n    import syntax std::adapters::doubled as doubled;\n\n    score { part p { voice v { c4/1 } } }\n}\n",
        );
        assert!(
            messages(&expansion)
                .iter()
                .any(|message| message.contains("imported after")),
            "{:?}",
            messages(&expansion)
        );
    }

    #[test]
    fn a_declaration_an_import_or_a_module_is_not_an_expression_and_is_refused() {
        for emitted in [
            r#"syntax_token(syntax_built(here, 4, 0), TokenKind.ImportKw, "import \"x.musa\";")"#,
            r#"syntax_token(syntax_built(here, 4, 0), TokenKind.LetKw, "let generated = 3;")"#,
            r#"syntax_token(syntax_built(here, 4, 0), TokenKind.ModKw, "mod generated;")"#,
            r#"syntax_token(syntax_built(here, 4, 0), TokenKind.DataKw, "data Generated { One }")"#,
        ] {
            let printed = answer(&answering(emitted), "c4").expect("the transformer answers");
            let refusal = ordinary_expression(&printed.text, SourceSpan::new(0, 1))
                .expect_err("an adapter may emit an expression and nothing else");
            assert!(
                refusal.message.contains("not one ordinary expression"),
                "{emitted}: {}",
                refusal.message
            );
        }
    }

    #[test]
    fn an_adapter_may_not_emit_another_region() {
        let printed = answer(
            &answering(r#"syntax_token(syntax_built(here, 4, 0), TokenKind.SyntaxKw, "syntax doubled { c4 }")"#),
            "c4",
        )
        .expect("the transformer answers");
        let refusal =
            ordinary_expression(&printed.text, SourceSpan::new(0, 1)).expect_err("a region is not an emission");
        assert!(
            refusal.message.contains("another adapter region"),
            "{}",
            refusal.message
        );
    }

    #[test]
    fn an_adapter_written_with_an_adapter_is_refused() {
        let Err(crate::core::ModuleFault::Broken(diagnostics)) = crate::core::read_adapter_module(
            "library {\n    let expand = syntax other { c4 };\n}\n",
            crate::core::PhaseImports::bundled(),
        ) else {
            panic!("the bootstrap is adapter-free")
        };
        let refusal = diagnostics.into_iter().next().expect("it says why");
        assert!(refusal.message.contains("adapter of its own"), "{}", refusal.message);
    }

    #[test]
    fn a_region_that_binds_and_uses_one_name_keeps_them_together_and_apart() {
        // One binding, written twice: the binder and the reference ask for the
        // same binding path, so they must print as one name.
        let bound = r#"syntax_group(syntax_built(here, 3, 0), Delimiter.Parentheses,
            [syntax_binder(syntax_binding(here, 5), "each"),
             syntax_reference(syntax_built(here, 6, 0), syntax_binding(here, 5), "each"),
             syntax_reference(syntax_built(here, 7, 0), syntax_binding(here, 8), "each")])"#;
        let printed = answer(&answering(bound), "each").expect("the transformer answers");
        assert_eq!(
            printed.generated_names.len(),
            2,
            "one binding is one name and two bindings are two: {:?}",
            printed.generated_names
        );
        assert!(
            !printed.text.contains(" each "),
            "the composer's own `each` is not the expansion's: {}",
            printed.text
        );
        // And the phase refuses the one case renaming alone cannot settle: a
        // generated name the composer can also reach.
        let written = names_written("piece \"x\" { let each_g0 = 1; }");
        assert!(
            printed
                .generated_names
                .iter()
                .any(|generated| written.contains(generated)),
            "the fixture for the collision law has to actually collide"
        );
    }

    #[test]
    fn a_position_inside_an_expansion_is_reported_as_the_region_that_produced_it() {
        let source = piece("c4");
        let expansion = run(&source);
        let region = expansion.records.first().expect("one record").use_site;
        let replaced = expansion.map.replacements.first().copied().expect("one replacement");
        assert_eq!(
            expansion.map.span(SourceSpan::new(replaced.from, replaced.to)),
            region,
            "the whole expansion is the region"
        );
        assert_eq!(
            expansion
                .map
                .span(SourceSpan::new(replaced.from.saturating_add(1), replaced.to)),
            region,
            "and so is any part of it"
        );
        // Text after the expansion moves by the difference in length, exactly.
        let after = source.find("score").expect("the piece has a score");
        let moved = expansion.document.text().find("score").expect("so does the expansion");
        assert_eq!(
            expansion.map.span(SourceSpan::new(
                u32::try_from(moved).unwrap_or(0),
                u32::try_from(moved).unwrap_or(0)
            )),
            SourceSpan::new(u32::try_from(after).unwrap_or(0), u32::try_from(after).unwrap_or(0)),
            "and everything outside every expansion is where the composer left it"
        );
    }

    #[test]
    fn expansion_terminates_because_the_bootstrap_is_adapter_free() {
        // Not a rank count: the adapter's own source holds no region, and its
        // answer may hold none, so there is no second round to bound.
        let adapter = crate::imports::standard_library_source("musa-stdlib:/std/adapters/doubled.musa")
            .expect("the fixture adapter is bundled");
        let read = musa_language::parse(adapter);
        assert!(
            !read.syntax().descendants().any(|node| {
                node.kind() == SyntaxKind::SyntaxRegion
                    || ImportStmt::cast(node).is_some_and(|import| import.changes_syntax())
            }),
            "an adapter definition contains no adapter region and no syntax import"
        );
        let expansion = run(&piece("c4"));
        assert!(
            !expansion.document.text().contains("syntax doubled"),
            "one pass leaves nothing to expand"
        );
    }

    /// A region the bundled fixture will not read, and the one token it
    /// refuses. Written once because four laws below are about the same
    /// refusal seen from four sides.
    const REFUSED: &str = "c4 ; d4";

    #[test]
    fn a_refusal_lands_on_the_node_the_adapter_pointed_at() {
        let source = piece(REFUSED);
        let expansion = run(&source);
        let complaint = expansion.diagnostics.first().expect("the adapter refused");
        assert_eq!(complaint.code, Code::Expansion, "a refusal is not a compiler fault");
        assert!(
            complaint.message.starts_with("`std::adapters::doubled`:")
                && complaint.message.contains("one expression's worth of tokens"),
            "the adapter's own sentence, over the adapter's own name: {}",
            complaint.message
        );
        let span = complaint.primary_span().expect("a place");
        let at = source.get(span.start as usize..span.end as usize).unwrap_or_default();
        assert_eq!(
            at, ";",
            "the caret is on the token the adapter handed back, not on the region"
        );
        // The distinction the law is about: pointing at a node is narrower
        // than the region, and a report that fell back to the region would
        // still have "looked right".
        let region = expansion.records.first().map(|record| record.use_site);
        assert_ne!(Some(span), region, "the refusal was widened to the whole region");
    }

    #[test]
    fn a_refusal_that_points_at_a_generated_node_lands_on_the_region_and_says_so() {
        // An adapter may only point with a node, and a node it *built* has no
        // composer's text under it. That is not an error — it is a refusal
        // whose caret cannot be as narrow as the sentence, and the report says
        // which of the two cases it is rather than quietly degrading.
        // Every step of this fold refuses with a node it just built, so
        // whichever node reaches the top carries `Generated` and nothing else.
        let refused = r#"Err((syntax_token(syntax_built(here, 9, 0), TokenKind.Error, ""), "nothing here is mine"))"#;
        let refusing = format!(
            "fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ syntax_fold_from_leaves(fn (here) {{ {refused} }}, fn (here, kind, text) {{ {refused} }}, \
             fn (here, name) {{ {refused} }}, fn (here, delimiter, children) {{ {refused} }}, region) }}"
        );
        let read = musa_language::parse("c4");
        let subject = crate::syntax::read_region(&read.syntax(), crate::syntax::ExpansionPath::at(vec![0]));
        let Err(generated) =
            crate::core::expand_syntax(&module(&refusing), crate::core::PhaseImports::bundled(), &subject).0
        else {
            panic!("the adapter refuses");
        };
        assert_eq!(
            generated,
            crate::core::ExpansionFailure::Refused {
                message: "nothing here is mine".to_owned(),
                at: None,
            },
            "a node the adapter built has no composer's text under it, so there is no range to read"
        );
        let site = SourceSpan::new(11, 33);
        let reported = stopped_or_refused(&generated, "std::adapters::doubled", site);
        assert_eq!(reported.code, Code::Expansion);
        assert_eq!(
            reported.primary_span(),
            Some(site),
            "with nothing to point at, the region is as close as the report gets"
        );
        assert!(
            reported
                .help
                .as_deref()
                .is_some_and(|help| help.contains("pointed at a node it built")),
            "the report does not say the caret is wider than the adapter meant: {:?}",
            reported.help
        );
    }

    #[test]
    fn a_refusal_is_told_apart_from_a_broken_adapter_and_from_a_stop() {
        // Three failures wear one phase and must not read alike. The refusal
        // is the adapter working; `NotATransformer` is the adapter broken; a
        // stop is neither, and reporting it as either would make a narrowed
        // budget look like a file that is not well-typed.
        let site = SourceSpan::new(0, 1);
        let refused = stopped_or_refused(
            &crate::core::ExpansionFailure::Refused {
                message: "not mine to read".to_owned(),
                at: Some(SourceSpan::new(4, 5)),
            },
            "std::adapters::doubled",
            site,
        );
        let broken = stopped_or_refused(
            &crate::core::ExpansionFailure::NotATransformer(Vec::new()),
            "std::adapters::doubled",
            site,
        );
        let stopped = stopped_or_refused(&crate::core::ExpansionFailure::Stopped, "std::adapters::doubled", site);
        assert_eq!(refused.code, Code::Expansion);
        assert_eq!(broken.code, Code::Expansion);
        assert_eq!(stopped.code, Code::ResourceLimit, "a stop is a limit and says so");
        assert!(
            refused.message.contains("not mine to read") && !broken.message.contains("not mine to read"),
            "a refusal carries the adapter's sentence and a broken adapter carries the compiler's"
        );
        assert_ne!(
            refused.primary_span(),
            broken.primary_span(),
            "only the refusal was pointed, so only the refusal is narrower than the region"
        );
        // And end to end, on a region the run cannot finish: a stop must not
        // reach the reader as the adapter's own sentence.
        //
        // The stop is provoked by nesting rather than by narrowing a budget.
        // `musa_core::Budget::scaled` says why in as many words — the core's
        // limit is `LANGUAGE` and nothing in the pipeline lowers it, "because a
        // budget the caller could lower would make acceptance a property of the
        // invocation rather than of the language" — so the only honest way to
        // reach the counter is to give it work it genuinely cannot do under it.
        // Deep enough that `doubled`'s own traversal cannot finish under
        // `Budget::NESTING`, and no deeper: the point is the *reporting*, so a
        // depth chosen for headroom would be a slower test saying the same
        // thing.
        const DEPTH: usize = 120;
        let deep = format!("{}a{}", "(".repeat(DEPTH), ")".repeat(DEPTH));
        let stopped = run(&piece(&deep));
        assert!(
            !stopped.diagnostics.is_empty(),
            "the region was supposed to be too deep to read"
        );
        for diagnostic in &stopped.diagnostics {
            assert_ne!(
                diagnostic.code,
                Code::Expansion,
                "a stopped run reported the limit as the adapter refusing: {}",
                diagnostic.message
            );
        }
    }

    #[test]
    fn a_refusal_charges_what_the_run_that_refused_charged() {
        // Refusing is an answer, not a stop: the adapter read the whole region
        // to decide, and a phase that charged nothing for it would let a file
        // buy unbounded reading by arranging to be refused.
        let refused = run(&piece(REFUSED));
        assert!(!refused.diagnostics.is_empty(), "the fixture was supposed to refuse");
        assert_eq!(
            refused.charges.expansion_steps, 1,
            "one region was expanded, whatever it answered"
        );
        assert!(
            refused.charges.evaluation_steps > 0 && refused.charges.type_constraints > 0,
            "the run that refused was checked and evaluated, and is charged for both: {:?}",
            refused.charges
        );
        let accepted = run(&piece("c4 d4"));
        assert!(
            refused.charges.evaluation_steps >= accepted.charges.evaluation_steps,
            "reading a region and refusing it is at least as much work as reading it and not: {:?} vs {:?}",
            refused.charges,
            accepted.charges
        );
    }

    #[test]
    fn an_anchor_names_the_range_of_the_node_it_was_taken_from() {
        // The fixture anchors every group at the node it rebuilt, so the
        // parenthesised group inside the region is anchored by a number the
        // expanded text carries. That number, put back through the record,
        // must land on the composer's own `(c4)`.
        let source = piece("(c4)");
        let expansion = run(&source);
        assert!(messages(&expansion).is_empty(), "{:?}", messages(&expansion));
        let record = expansion.records.first().expect("one record");
        let at = |span: SourceSpan| {
            source
                .get(span.start as usize..span.end as usize)
                .expect("a range inside the file")
        };
        let number = record
            .anchors
            .iter()
            .position(|span| at(*span) == "(c4)")
            .expect("the region holds a parenthesised group");
        assert!(
            expansion.document.text().contains(&format!(", {number})")),
            "the adapter emitted the anchor of the group it rebuilt:\n{}",
            expansion.document.text()
        );
        let named = record.anchor(number as u64).expect("the number the adapter emitted");
        assert_eq!(at(named), "(c4)", "an anchor names the node it was taken from");
        assert_eq!(
            record.anchor(0).map(at),
            Some("{ (c4) }"),
            "and zero is the region's own group, which is what the adapter anchors at the top"
        );
    }

    #[test]
    fn two_identical_regions_mint_the_same_anchors() {
        // The number is the node's position in the region's own reading order,
        // so it cannot depend on where in the file the region stands. If it
        // could, prompt 127dc's law would break here first: the cache would
        // replay the first region's printed text at the second region's
        // offsets, and the two would disagree.
        let expansion = run(
            "piece \"laws\" {\n    import syntax std::adapters::doubled as doubled;\n\n    let a = syntax doubled { (c4) };\n    let b = syntax doubled { (c4) };\n\n    score { part p { voice v { c4/1 } } }\n}\n",
        );
        assert!(messages(&expansion).is_empty(), "{:?}", messages(&expansion));
        let [first, second] = expansion.records.as_slice() else {
            panic!("two regions, two records");
        };
        assert_eq!(
            first.anchors.len(),
            second.anchors.len(),
            "two identical regions have the same shape, so they have the same anchors"
        );
        assert_ne!(
            first.anchors.first(),
            second.anchors.first(),
            "the ranges differ, because the two regions stand at different places"
        );
        let offset = i64::from(second.use_site.start) - i64::from(first.use_site.start);
        for (near, far) in first.anchors.iter().zip(&second.anchors) {
            assert_eq!(
                i64::from(far.start) - i64::from(near.start),
                offset,
                "the second region's table is the first's, moved by where it was written"
            );
        }
    }

    #[test]
    fn an_anchor_for_a_node_the_adapter_built_is_none() {
        // A path the adapter derived addresses no input node, and there is
        // nothing in the composer's text to anchor it to. Answering `None` is
        // what keeps §3.4 exact: an adapter cannot invent a place, and cannot
        // learn one it was not given.
        let built = answer(
            &answering(
                r#"syntax_anchor(region, syntax_built(here, 0, 0), syntax_built(here, 10, 0)).fold_from_start(syntax_token(syntax_built(here, 9, 0), TokenKind.Integer, "404"), fn (held, node) { node })"#,
            ),
            "{ c4 }",
        )
        .expect("the transformer answered");
        assert!(
            built.text.contains("404"),
            "the fallback stood, so the anchor of a built node was `None`: {}",
            built.text
        );
        let given = answer(
            &answering(
                r#"syntax_anchor(region, here, syntax_built(here, 10, 0)).fold_from_start(syntax_token(syntax_built(here, 9, 0), TokenKind.Integer, "404"), fn (held, node) { node })"#,
            ),
            "{ c4 }",
        )
        .expect("the transformer answered");
        assert!(
            !given.text.contains("404"),
            "the same call on a node the adapter was given answers with its anchor: {}",
            given.text
        );
    }

    /// The offset of `text` in `source`, for naming a place a test cares about.
    fn offset_of(source: &str, text: &str) -> u32 {
        u32::try_from(source.find(text).expect("the fixture writes it")).unwrap_or(u32::MAX)
    }

    /// Ask the fixture to replace the node whose text is `target`.
    fn replace(source: &str, target: &str, with: &str) -> Result<Vec<AdapterEdit>, AdapterEditError> {
        let document = SourceDocument::new(source, "laws.musa");
        let expansion = expand(&document, &CompileOptions::default());
        let record = expansion.records.first().expect("one record");
        let anchor = record
            .anchors
            .iter()
            .position(|span| {
                source
                    .get(span.start as usize..span.end as usize)
                    .is_some_and(|held| held == target)
            })
            .expect("the region holds that node");
        adapter_edits(
            &document,
            &CompileOptions::default(),
            record.use_site.start,
            "replace",
            anchor as u64,
            with,
        )
    }

    #[test]
    fn an_adapter_edit_lands_inside_the_region_and_nowhere_else() {
        // Locality and preservation, which are one test because they are two
        // halves of one sentence: the patch is inside the region, and the file
        // outside the patch is the file.
        let source = piece("c4");
        let edits = replace(&source, "c4", "d4").expect("the fixture serves `replace`");
        let [edit] = edits.as_slice() else {
            panic!("one command, one edit: {edits:?}");
        };
        let region = expand(&SourceDocument::new(&source, "laws.musa"), &CompileOptions::default())
            .records
            .first()
            .expect("one record")
            .use_site;
        assert!(
            edit.start >= region.start && edit.end <= region.end,
            "the edit lies inside the region: {edit:?} against {region:?}"
        );
        assert_eq!(
            source.get(edit.start as usize..edit.end as usize),
            Some("c4"),
            "and it replaces the node the command named"
        );
        let patched = format!(
            "{}{}{}",
            source.get(..edit.start as usize).unwrap_or_default(),
            edit.text,
            source.get(edit.end as usize..).unwrap_or_default()
        );
        assert_eq!(patched, source.replace("c4 }", "d4 }"), "and nothing else moved");
    }

    #[test]
    fn re_expanding_a_patched_region_agrees_with_the_command() {
        // Agreement. The strongest thing the compiler can check without knowing
        // the package's type is that the patched region expands to what the
        // region the command describes expands to — so it checks that, over
        // the printed expression, which is what the rest of the compiler reads.
        let source = piece("c4");
        let edits = replace(&source, "c4", "d4").expect("the fixture serves `replace`");
        let [edit] = edits.as_slice() else {
            panic!("one command, one edit: {edits:?}");
        };
        let patched = format!(
            "{}{}{}",
            source.get(..edit.start as usize).unwrap_or_default(),
            edit.text,
            source.get(edit.end as usize..).unwrap_or_default()
        );
        let written = piece("d4");
        assert_eq!(
            run(&patched).document.text(),
            run(&written).document.text(),
            "the patched region expands to what the command meant"
        );
    }

    #[test]
    fn a_command_the_adapter_does_not_know_is_refused_with_its_own_sentence() {
        let source = piece("c4");
        let document = SourceDocument::new(&source, "laws.musa");
        let refused = adapter_edits(
            &document,
            &CompileOptions::default(),
            offset_of(&source, "syntax doubled"),
            "transpose",
            1,
            "up",
        );
        let Err(AdapterEditError::Refused { adapter, message }) = refused else {
            panic!("a command the adapter does not know is the adapter's refusal: {refused:?}");
        };
        assert_eq!(adapter, "std::adapters::doubled");
        assert!(message.contains("one command"), "the adapter's own sentence: {message}");
        // And it changed nothing: a refusal is an answer, not a patch.
        assert_eq!(document.text(), source, "asking a question does not edit the document");
    }

    #[test]
    fn an_anchor_the_region_never_minted_is_a_broken_adapter_rather_than_an_edit() {
        let source = piece("c4");
        let refused = adapter_edits(
            &SourceDocument::new(&source, "laws.musa"),
            &CompileOptions::default(),
            offset_of(&source, "syntax doubled"),
            "replace",
            9_999,
            "d4",
        );
        let Err(AdapterEditError::Broken(diagnostic)) = refused else {
            panic!("an anchor this region never minted is a fault: {refused:?}");
        };
        assert!(diagnostic.message.contains("does not have"), "{}", diagnostic.message);
    }

    #[test]
    fn a_readable_adapters_region_is_read_only_rather_than_broken() {
        // The *readable* level of §4. A region whose adapter declares that
        // level is not a failure to report; it is a structured view that cannot
        // be written through, and an interface has to be able to tell the two
        // apart to know whether to grey a control or show a complaint.
        let source = piece("c4");
        let mut options = CompileOptions::default();
        let uri = crate::imports::resolve_import("laws.musa", "std::adapters::doubled");
        let readable = options
            .imports
            .get(&uri)
            .expect("the fixture is bundled")
            .split("    let edit =")
            .next()
            .map(|kept| format!("{}}}\n", kept.replace("\"editable\"", "\"readable\"")))
            .expect("the fixture declares `edit` last");
        options.imports.insert(uri, readable);
        let answer = adapter_edits(
            &SourceDocument::new(&source, "laws.musa"),
            &options,
            offset_of(&source, "syntax doubled"),
            "replace",
            1,
            "d4",
        );
        assert_eq!(
            answer,
            Err(AdapterEditError::ReadOnly {
                adapter: "std::adapters::doubled".to_owned()
            }),
            "an adapter that declares no `edit` reports its level, not a fault"
        );
    }

    /// A whole generative adapter, for the laws a printer carries.
    ///
    /// Here rather than in `stdlib/` for the reason `doubled`'s own comment
    /// gives: a printer writes a value back as *source*, and the source
    /// language has no text operations, so a printer can only spell what it
    /// already holds the words for. This one holds two words, which is enough
    /// for a round trip and honest about everything else being a loss. Its
    /// regions hold one text literal and its value is that text.
    const MOTTO: &str = r#"library {
    let level = "generative";

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {
        Ok(syntax_fold_from_leaves(
            fn (here) { syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "") },
            fn (here, kind, text) { syntax_token(syntax_built(here, 1, 0), kind, text) },
            fn (here, name) { syntax_identifier(syntax_built(here, 2, 0), name) },
            fn (here, delimiter, children) { syntax_group(syntax_built(here, 3, 0), Delimiter.Layout, children) },
            region
        ))
    };

    let edit = fn (region: Syntax<TokenTree>, command: Text, anchor: Nat, argument: Text) -> Result<List<Pair<Nat, Text>>, Text> {
        match command {
            "replace" -> Ok([(anchor, argument)]),
            _ -> Err("`motto` serves one command, `replace`"),
        }
    };

    let print = fn (value: Text) {
        match value {
            "hello" -> Ok("\"hello\""),
            "goodbye" -> Ok("\"goodbye\""),
            _ -> Err("`motto` writes `hello` and `goodbye`, and this is neither"),
        }
    };
}
"#;

    /// The URI the generative fixture is imported by.
    fn motto() -> String {
        crate::imports::resolve_import("laws.musa", "std::adapters::motto")
    }

    /// A compilation that can read `module` as `std::adapters::motto`.
    fn reading(module: &str) -> CompileOptions {
        let mut options = CompileOptions::default();
        options.imports.insert(motto(), module.to_owned());
        options
    }

    /// A piece that imports the generative fixture, with `body` after the
    /// header.
    fn importing(body: &str) -> String {
        format!(
            "piece \"laws\" {{\n    import syntax std::adapters::motto as motto;\n\n{body}\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
        )
    }

    #[test]
    fn expanding_a_printed_region_gives_back_the_value_it_was_printed_from() {
        // The round-trip law of §4, through the real compiler: print the value,
        // splice the answer into a region, expand it, evaluate it, compare.
        // Compared as *values* and not as text, because printing is allowed to
        // normalize — and by the fixture's own equality, which for a fixture
        // whose value is a text is text equality.
        let options = reading(MOTTO);
        let printed = adapter_print(
            &SourceDocument::new("", "laws.musa"),
            &options,
            "std::adapters::motto",
            "\"hello\"",
        )
        .expect("the printer writes the words it knows");
        let source = importing(&format!("    let it = syntax motto {{ {printed} }};\n"));
        let expansion = expand(&SourceDocument::new(&source, "laws.musa"), &options);
        assert!(messages(&expansion).is_empty(), "{:?}", messages(&expansion));
        let record = expansion.records.first().expect("one record");
        let expression = crate::syntax::print(&record.output).text;
        let round_tripped = crate::core::evaluate_text(&expression);
        assert_eq!(
            round_tripped,
            crate::core::evaluate_text("\"hello\""),
            "the printed region evaluates to the value it was printed from: {expression}"
        );
        assert_eq!(round_tripped.as_deref(), Some("hello"), "and the law is not vacuous");
    }

    #[test]
    fn a_value_the_printer_cannot_spell_is_a_stated_loss_and_not_a_smaller_value() {
        // `PrintLoss` is an answer. A printer that wrote down what it could and
        // dropped the rest would satisfy the round-trip law by making the value
        // smaller, which is the one way of satisfying it that is worthless.
        let refused = adapter_print(
            &SourceDocument::new("", "laws.musa"),
            &reading(MOTTO),
            "std::adapters::motto",
            "\"farewell\"",
        );
        let Err(AdapterPrintError::Loss { adapter, message }) = refused else {
            panic!("a value the printer will not write is its own sentence: {refused:?}");
        };
        assert_eq!(adapter, "std::adapters::motto");
        assert!(message.contains("neither"), "the adapter's own sentence: {message}");
    }

    #[test]
    fn a_printer_reads_the_packages_type_and_its_own_modules_declarations() {
        // The two halves of "read where it is run". The printer below names
        // `Clef` — a type of a package it does not import and could not import
        // — because the *document the region is written into* imports it, and it
        // calls `named`, a declaration of its own module, because a musa block
        // holds one expression and a printer with no local definitions is a
        // printer nobody can write.
        //
        // `unreached` is the control: it belongs to the phase, the printer never
        // names it, and it must not be spliced. If reaching were "the whole
        // module" rather than "what the printer names", this fixture would not
        // check at all, because `Syntax<TokenTree>` has no ordinary reading.
        const CLEFS: &str = r#"library {
    let level = "generative";

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(region) };
    let edit = fn (region: Syntax<TokenTree>, command: Text, anchor: Nat, argument: Text) -> Result<List<Pair<Nat, Text>>, Text> { Err("`clefs` serves no command") };

    let unreached = fn (region: Syntax<TokenTree>) -> Syntax<TokenTree> { region };

    let named = fn (written: Clef) -> Text {
        match written {
            Treble -> "treble",
            Bass -> "bass",
            Alto -> "alto",
            Tenor -> "tenor",
        }
    };

    let print = fn (written: Clef) -> Result<Text, Text> {
        Ok(text_join(["clef ", named(written)]))
    };
}
"#;
        let mut options = CompileOptions::default();
        options.imports.insert(
            crate::imports::resolve_import("laws.musa", "std::adapters::clefs"),
            CLEFS.to_owned(),
        );
        // The importing document is where `Clef` comes from. An empty document
        // would leave the printer's own parameter type unnameable, which is the
        // state 127dce left every printer in.
        let at = SourceDocument::new("piece \"laws\" {\n    import std::notation::staff;\n}\n", "laws.musa");
        let printed = adapter_print(&at, &options, "std::adapters::clefs", "Alto");
        assert_eq!(printed.as_deref(), Ok("clef alto"), "{printed:?}");
    }

    #[test]
    fn an_adapter_below_generative_writes_no_region_and_says_which_level_it_is() {
        // The bundled fixture is *editable* and says so. Asking it to write a
        // region is not a fault and not a loss: it is a level, and an interface
        // that offers "new region" needs to be told which.
        let answer = adapter_print(
            &SourceDocument::new("", "laws.musa"),
            &CompileOptions::default(),
            "std::adapters::doubled",
            "(c4, 0)",
        );
        assert_eq!(
            answer,
            Err(AdapterPrintError::NotGenerative {
                adapter: "std::adapters::doubled".to_owned(),
                level: "editable".to_owned(),
            })
        );
    }

    #[test]
    fn a_module_that_declares_more_than_it_offers_is_refused_at_its_import() {
        // Where the level stops being a label. The module below promises the
        // round-trip law and holds nothing that could satisfy it, and the
        // refusal names the operation it is missing rather than saying the
        // adapter is broken.
        let overstated = MOTTO
            .split("    let print =")
            .next()
            .map(|kept| format!("{kept}}}\n"))
            .expect("the fixture declares `print` last");
        // No region at all: importing the package is where the promise is made,
        // so that is where it is checked.
        let source = importing("");
        let expansion = expand(&SourceDocument::new(&source, "laws.musa"), &reading(&overstated));
        let complaint = expansion.diagnostics.first().expect("the promise is checked");
        assert!(
            complaint.message.contains("generative") && complaint.message.contains("`print`"),
            "the refusal names the level and the operation: {}",
            complaint.message
        );
        let span = complaint.primary_span().expect("a place");
        let at = source.get(span.start as usize..span.end as usize).unwrap_or_default();
        assert!(
            at.starts_with("import syntax"),
            "and it is reported where the promise was made: {at}"
        );
        // A module that declares only what it holds is not refused, so the
        // check is about the promise rather than about the missing operation.
        let honest = overstated.replace("\"generative\"", "\"editable\"");
        assert!(
            expand(&SourceDocument::new(&source, "laws.musa"), &reading(&honest))
                .diagnostics
                .is_empty(),
            "under-promising is allowed: a level is a floor, not a description"
        );
    }

    #[test]
    fn the_anchor_table_is_the_regions_own_nodes_and_no_run_enlarges_it() {
        let source = piece("(c4)");
        let expansion = run(&source);
        let record = expansion.records.first().expect("one record");
        assert_eq!(
            u64::try_from(record.anchors.len()).unwrap_or(u64::MAX),
            record.input.shape().0,
            "one entry per node of the region, which is what makes the number an index into reading order"
        );
        for span in &record.anchors {
            assert!(
                span.start >= record.use_site.start && span.end <= record.use_site.end,
                "an anchor names a range inside the region it was minted from: {span:?}"
            );
        }
    }

    // The inherited-context recursor and its sealed steps, one test per law
    // (`../rules/language/02-core-calculus.md` §5.9). The type-side laws —
    // sealed formation, opacity, the `d`-exclusion, phase conservativity —
    // are refusals rather than runs and live beside the other registry laws in
    // `core.rs`; these are the ones that need a traversal to actually happen.

    /// The regions every differential law below runs over.
    ///
    /// One identifier, one call, a call with two arguments, and a call inside
    /// a call: between them they reach every branch, a group whose child is a
    /// group — which is where a traversal that dropped a level would show —
    /// and a hole, because a region parsed on its own is not always a whole
    /// expression and the reader answers with `Missing` where the parser gave
    /// up. They are shallow because a differential law needs no depth to be
    /// stated, not any more because depth was dangerous: depth is the budget's
    /// business now, and `a_region_deeper_than_the_budget_allows_is_refused_
    /// rather_than_fatal` owns it.
    const REGIONS: [&str; 4] = ["a", "together(a)", "together(a, b)", "together(inner(a))"];

    /// A transformer over the recursor whose branches rebuild what they read.
    ///
    /// `C` is `Text` and the identifier branch emits *the context* rather than
    /// the name, so what a node was read under is visible in the printed
    /// answer. `group` is the one thing a law varies; `initial` is the context
    /// the root is read under.
    fn recursing(initial: &str, group: &str) -> String {
        format!(
            "fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(recurse_syntax(\
             fn (c, here) {{ syntax_token(syntax_built(here, 0, 0), TokenKind.Error, \"\") }}, \
             fn (c, here, kind, text) {{ syntax_token(syntax_built(here, 1, 0), kind, text) }}, \
             fn (c, here, name) {{ syntax_identifier(syntax_built(here, 2, 0), c) }}, \
             fn (c, here, delimiter, kids) {{ {group} }}, \
             {initial}, region)) }}"
        )
    }

    /// The printed text one transformer answers with, over one region.
    fn printed(transformer: &str, region: &str) -> String {
        answer(transformer, region)
            .unwrap_or_else(|fault| panic!("the transformer answers over `{region}`: {fault:?}"))
            .text
    }

    /// What one transformer charged, over one region.
    fn charged(transformer: &str, region: &str) -> u64 {
        let read = musa_language::parse(region);
        let subject = crate::syntax::read_region(&read.syntax(), crate::syntax::ExpansionPath::at(vec![0]));
        let (answered, work) =
            crate::core::expand_syntax(&module(transformer), crate::core::PhaseImports::bundled(), &subject);
        if let Err(fault) = answered {
            panic!("the transformer answers over `{region}`: {fault:?}");
        }
        work.evaluation_steps
    }

    /// Read every child once, left to right, under the context handed down.
    const EACH_ONCE: &str =
        r"syntax_group(syntax_built(here, 3, 0), delimiter, kids.map(fn (kid) { run_syntax_step(c, kid) }))";

    /// Read nothing: a group branch that answers without running a step.
    const NONE_AT_ALL: &str = r"syntax_group(syntax_built(here, 3, 0), delimiter, [])";

    /// A region that is `levels` groups deep, with one identifier at the
    /// bottom.
    ///
    /// Built rather than parsed. Depth is the whole point of these two tests
    /// and the parser flattens nesting it does not need, so a region written
    /// out as text would say how deep the *parser* goes and not how deep the
    /// recursor may.
    fn nested_region(levels: usize) -> crate::syntax::Syntax {
        let root = crate::syntax::NodePath::root(crate::syntax::ExpansionPath::at(vec![0]));
        let mut subject = crate::syntax::identifier(root.clone(), "a".to_owned());
        for _ in 0..levels {
            subject = crate::syntax::group(root.clone(), crate::syntax::Delimiter::Parentheses, vec![subject]);
        }
        subject
    }

    /// A region deeper than the budget allows is refused, not fatal.
    ///
    /// The recursor descends through the transformer's own branches, so one
    /// level of source nesting costs a whole chain of `eval`/`apply_closure`
    /// frames and a deep enough region used to end the process with `fatal
    /// runtime error: stack overflow`. That is the one outcome a total language
    /// with a budget may not have: the budget exists in order to refuse.
    ///
    /// The region is far past the limit — deep enough that the old failure
    /// needed some sixty megabytes of stack — so what this test asserts is not
    /// that the number is exactly right but that no region can be deep enough
    /// to get past the counter. A test process that aborts fails this test by
    /// taking the whole binary with it, which is the failure mode being
    /// guarded and reads unmistakably in the output.
    #[test]
    fn a_region_deeper_than_the_budget_allows_is_refused_rather_than_fatal() {
        let (answered, _) = crate::core::expand_syntax(
            &module(&recursing("\"\"", EACH_ONCE)),
            crate::core::PhaseImports::bundled(),
            &nested_region(1_000),
        );
        assert!(
            matches!(answered, Err(crate::core::ExpansionFailure::Stopped)),
            "a region too deep to read is a limit crossed, not a crash and not a malformed adapter"
        );
        // And the phase says so where the region stands, with the code that
        // means a limit rather than the one that means a broken adapter.
        let complaint = stopped_or_refused(
            &crate::core::ExpansionFailure::Stopped,
            "std::adapters::doubled",
            SourceSpan::new(4, 9),
        );
        assert_eq!(complaint.code, Code::ResourceLimit);
        assert_eq!(complaint.primary_span(), Some(SourceSpan::new(4, 9)));
    }

    /// The limit is set where honest work still fits under it.
    ///
    /// A guard that refused the regions adapters actually meet would be a
    /// crash with better manners. This recursor descends through the
    /// transformer's own branches, so one level of source nesting costs a chain
    /// of frames rather than one, and the 256 of `Budget::LANGUAGE` is what
    /// turns that chain into a refusal instead of a crash.
    ///
    /// **Sixteen and not forty-eight.** Under the replaced evaluator a level of
    /// region cost four frames and the first refusal was at 64 groups deep.
    /// Normalization by evaluation adds a second descent — `quote` walks a value
    /// the way `eval` walks a term — and the measured chain is now some fourteen
    /// frames a level, so the first refusal is at 19. Sixteen is still past
    /// anything a person writes and it leaves the exact boundary to the meter's
    /// own law rather than pinning it here, but the headroom an adapter has
    /// shrank fourfold and that is a measurement, not a preference: prompt 144
    /// sets this limit against the checker that now spends it.
    #[test]
    fn a_region_nested_deeper_than_anyone_writes_still_expands() {
        let (answered, _) = crate::core::expand_syntax(
            &module(&recursing("\"\"", EACH_ONCE)),
            crate::core::PhaseImports::bundled(),
            &nested_region(16),
        );
        assert!(
            !matches!(answered, Err(crate::core::ExpansionFailure::Stopped)),
            "a region forty-eight groups deep was refused: the nesting limit is below what adapters meet"
        );
    }

    #[test]
    fn law_9_the_derived_fold_is_the_recursor_at_a_context_nothing_reads() {
        // Running every step in source order, under a context no branch reads,
        // *is* `syntax_fold_from_leaves`. One traversal implements both, so
        // this is the differential test that keeps the derivation honest
        // rather than merely asserted — and law 4 rides on it, because both
        // sides build their output from the path they were handed and the two
        // outputs are compared byte for byte.
        let recursor = recursing(
            "\"\"",
            r#"syntax_group(syntax_built(here, 3, 0), delimiter, kids.map(fn (kid) { run_syntax_step("", kid) }))"#,
        );
        let fold = "fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(syntax_fold_from_leaves(\
             fn (here) { syntax_token(syntax_built(here, 0, 0), TokenKind.Error, \"\") }, \
             fn (here, kind, text) { syntax_token(syntax_built(here, 1, 0), kind, text) }, \
             fn (here, name) { syntax_identifier(syntax_built(here, 2, 0), \"\") }, \
             fn (here, delimiter, children) { syntax_group(syntax_built(here, 3, 0), delimiter, children) }, \
             region)) }";
        for region in REGIONS {
            assert_eq!(
                printed(&recursor, region),
                printed(fold, region),
                "the two descents disagree over `{region}`"
            );
        }
    }

    #[test]
    fn law_3_a_branch_is_read_under_exactly_the_context_it_was_run_with() {
        // The identifier branch emits its own context, so the printed answer
        // says what each name was read under. Nothing ambient decides it: a
        // child is read under whatever its parent's branch passed, and a fold
        // has no place to put either.
        let saying = |passed: &str| {
            recursing(
                "\"top\"",
                &format!(
                    r#"syntax_group(syntax_built(here, 3, 0), delimiter,
                         kids.map(fn (kid) {{ run_syntax_step("{passed}", kid) }}))"#
                ),
            )
        };
        let under = printed(&saying("under"), "together(a)");
        assert!(
            under.contains("under"),
            "a child is read under what its parent passed: {under}"
        );
        assert!(
            !under.contains("top"),
            "and the root's own context is not handed down behind the branch's back: {under}"
        );
        // The same region, one word changed in the branch: what a node is read
        // under is the branch's decision and the recursor's delivery, with
        // nothing between them.
        let deep = printed(&saying("deeper"), "together(a)");
        assert!(
            deep.contains("deeper") && !deep.contains("under"),
            "the recursor supplies exactly the context the branch chose: {deep}"
        );
    }

    #[test]
    fn law_6_a_step_may_be_omitted_or_run_more_than_once() {
        // Omission first: a group branch that answers without running anything
        // reads none of its children, which is the selective descent a fold
        // cannot do — under a fold the children are values before the branch
        // is entered.
        let dropped = printed(&recursing("\"seen\"", NONE_AT_ALL), "together(a, b)");
        assert!(!dropped.contains("seen"), "an omitted step read nothing: {dropped}");
        let read = printed(&recursing("\"seen\"", EACH_ONCE), "together(a, b)");
        assert!(
            read.contains("seen"),
            "and the same algebra that runs its steps does read them: {read}"
        );
        // And repetition, under two different contexts. Each child is run
        // twice and only the second answer is emitted — two answers at one
        // path would be two nodes in one place, which the output gate refuses
        // for reasons that have nothing to do with steps. What the emitted
        // answer proves is that the second run used the context the second run
        // was given, over the same sealed child.
        let repeated = printed(&recursing("\"\"", TWICE_OVER), "together(a)");
        assert!(
            repeated.contains("second") && !repeated.contains("first"),
            "the second run answered under its own context: {repeated}"
        );
    }

    /// Run every child twice, under two contexts, and emit the second answer.
    const TWICE_OVER: &str = r#"syntax_group(syntax_built(here, 3, 0), delimiter,
         kids.map(fn (kid) {
           Some(run_syntax_step("first", kid)).fold_from_start(
             syntax_token(syntax_built(here, 8, 0), TokenKind.Error, ""),
             fn (held, node) { run_syntax_step("second", kid) })
         }))"#;

    #[test]
    fn law_2_a_step_carried_into_a_nested_recursor_still_runs_its_own_algebra() {
        // The hostile case of prompt 127dcfae's program five, cut to what can
        // be printed. The outer group branch starts a *fresh* recursor over
        // the original region and, from inside that recursor's own group
        // branch, runs the steps the outer traversal minted.
        //
        // The two algebras are told apart by what their identifier branch
        // emits, `outer` against `inner`. If a nested recursor could
        // re-associate a step with itself, the captured steps would come back
        // `inner`. They do not, and no ownership check is what stops it —
        // there is no operation that would let the inner traversal try.
        let hostile = r#"fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(recurse_syntax(
            fn (c, here) { syntax_token(syntax_built(here, 0, 0), TokenKind.Error, "") },
            fn (c, here, kind, text) { syntax_token(syntax_built(here, 1, 0), kind, text) },
            fn (c, here, name) { syntax_identifier(syntax_built(here, 2, 0), "outer") },
            fn (c, here, delimiter, kids) {
                recurse_syntax(
                    fn (d, spot) { syntax_token(syntax_built(here, 4, 0), TokenKind.Error, "") },
                    fn (d, spot, kind, text) { syntax_token(syntax_built(here, 5, 0), kind, text) },
                    fn (d, spot, name) { syntax_identifier(syntax_built(here, 6, 0), "inner") },
                    fn (d, spot, delimiter, others) {
                        syntax_group(syntax_built(here, 7, 0), delimiter,
                            kids.map(fn (kid) { run_syntax_step(d, kid) }))
                    },
                    c, region)
            },
            "", region)) }"#;
        let answered = printed(hostile, "together(a)");
        assert!(
            answered.contains("outer"),
            "the captured step ran its own algebra from inside a foreign traversal: {answered}"
        );
        assert!(
            !answered.contains("inner"),
            "and the foreign traversal did not get to reinterpret it: {answered}"
        );
    }

    #[test]
    fn law_5_a_nested_recursor_over_the_original_subject_still_terminates() {
        // Local decrease alone does not give this: the inner recursor restarts
        // on the *whole* region, which is larger than the child whose step is
        // in flight, so no globally decreasing runtime tree size exists to
        // point at. What answers is §5.9's reducibility argument, resting on
        // the checker's definition acyclicity. This test is the executable
        // half — it cannot stand in for the proof, and it does catch an
        // implementation that lost the local decrease.
        let restarting = recursing(
            "\"\"",
            r"syntax_group(syntax_built(here, 3, 0), delimiter,
                 kids.map(fn (kid) {
                   recurse_syntax(
                     fn (d, spot) { run_syntax_step(d, kid) },
                     fn (d, spot, kind, text) { run_syntax_step(d, kid) },
                     fn (d, spot, name) { run_syntax_step(d, kid) },
                     fn (d, spot, delimiter, others) { run_syntax_step(d, kid) },
                     c, region)
                 }))",
        );
        for region in ["a", "together(a)"] {
            drop(printed(&restarting, region));
        }
    }

    #[test]
    fn law_7_two_runs_of_one_transformer_agree_on_value_and_on_charge() {
        let transformer = recursing("\"\"", EACH_ONCE);
        for region in REGIONS {
            assert_eq!(
                printed(&transformer, region),
                printed(&transformer, region),
                "two runs over `{region}` disagreed on the value"
            );
            assert_eq!(
                charged(&transformer, region),
                charged(&transformer, region),
                "two runs over `{region}` disagreed on the charge"
            );
        }
    }

    #[test]
    fn law_10_capture_and_repetition_are_charged_for_what_they_cost() {
        // Three algebras over one region: one that runs nothing, one that runs
        // each child once, and one that runs each child twice. A step that a
        // branch keeps and never runs must still cost its mint, or capture
        // would be a way to buy work off the meter.
        let region = "together(a, b)";
        let none = charged(&recursing("\"\"", NONE_AT_ALL), region);
        let once = charged(&recursing("\"\"", EACH_ONCE), region);
        let twice = charged(&recursing("\"\"", TWICE_OVER), region);
        assert!(
            none < once,
            "running a step costs more than omitting it: {none} vs {once}"
        );
        assert!(
            once < twice,
            "running a step twice costs more than running it once: {once} vs {twice}"
        );
        // And the mint is charged even where the step is never run. The
        // algebra below descends exactly one level — the context says "stop"
        // and the branch reads it — so every step at that level is minted and
        // none of them is run. A wider level is dearer all the same, which is
        // what stops capture from being a way to buy work off the meter.
        let one_level = recursing(
            "\"go\"",
            r#"match c {
                 "stop" -> syntax_group(syntax_built(here, 3, 0), delimiter, []),
                 _ -> syntax_group(syntax_built(here, 4, 0), delimiter,
                        kids.map(fn (kid) { run_syntax_step("stop", kid) })),
               }"#,
        );
        assert!(
            charged(&one_level, "a b c d") > charged(&one_level, "a"),
            "minting a step is charged even where the step is never run"
        );
    }
}
