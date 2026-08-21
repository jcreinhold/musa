use std::collections::BTreeMap;

use musa_language::{SyntaxKind, SyntaxNode};

use crate::compile::{CompileOptions, SourceDocument};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;

use super::Cached;
use super::Expansion;
use super::Level;
use super::{expand_one, level_of, names_written, region_body, region_name, syntax_imports};
use crate::origin::Replacement;

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

/// One refusal, anchored in the composer's text.
///
/// A caller boxes it: these are only ever an `Err`, and a [`Diagnostic`]
/// carries its labels and its fix, so a `Result` holding one inline would be
/// as wide as its failure everywhere it is returned.
pub(crate) fn refusal(site: SourceSpan, message: impl Into<String>, help: &str) -> Diagnostic {
    Diagnostic::error(Code::Expansion, message)
        .at(site, "this region")
        .help(help)
}
