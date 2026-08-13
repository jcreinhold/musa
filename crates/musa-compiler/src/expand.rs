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
    /// The expansion this one was produced inside, if any.
    ///
    /// Always `None` today, and a field rather than an omission because the
    /// record's shape is fixed by §3.4 and a reader should not have to know
    /// which parts of it this build can reach. An adapter cannot emit a
    /// region, so an expansion has no children; if that rule is ever relaxed,
    /// this is where the parent goes.
    pub(crate) parent: Option<usize>,
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
}

/// Run step 4 of the fixed order over `source`.
pub(crate) fn expand(source: &SourceDocument, options: &CompileOptions) -> Expansion {
    let parsed = musa_language::parse(source.text());
    let root = parsed.syntax();
    let regions: Vec<SyntaxNode> = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::SyntaxRegion)
        .collect();
    if regions.is_empty() {
        return Expansion::unchanged(source);
    }
    let imports = syntax_imports(&root);
    let written = names_written(source.text());
    let mut cache: BTreeMap<(String, String), Cached> = BTreeMap::new();
    let mut expansion = Expansion::unchanged(source);
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
    cache: &mut BTreeMap<(String, String), Cached>,
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

    let key = (version.clone(), interior);
    if let Some(hit) = cache.get(&key) {
        // The charge is replayed from the record, so the second of two
        // identical regions costs exactly what the first did.
        let record = ExpansionRecord {
            adapter: import.path.clone(),
            version,
            use_site: site,
            input: subject,
            output: hit.output.clone(),
            parent: None,
        };
        return Ok((hit.printed.clone(), record, hit.charges));
    }

    let transformer = transformer_of(adapter_source, &import.path, site)?;
    let (answer, work) = crate::core::expand_syntax(&transformer, subject.clone());
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
    let wrapped = format!("piece \"expansion\" {{\n    let it = {text};\n}}\n");
    let parsed = musa_language::parse(&wrapped);
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

/// The `expand` an adapter module declares, as the text of one expression.
fn transformer_of(adapter_source: &str, path: &str, site: SourceSpan) -> Result<String, Box<Diagnostic>> {
    let parsed = musa_language::parse(adapter_source);
    let root = parsed.syntax();
    // The adapter-free bootstrap, checked rather than assumed. An adapter
    // whose own definition needed an adapter would put the expansion order
    // back into a cycle, and this is the whole of what prevents it.
    if root.descendants().any(|node| node.kind() == SyntaxKind::SyntaxRegion)
        || root.descendants().any(|node| is_syntax_import(&node))
    {
        return Err(Box::new(refusal(
            site,
            format!("`{path}` is written with an adapter of its own"),
            "an adapter is written in the adapter-free bootstrap: no region, no syntax import",
        )));
    }
    root.descendants()
        .filter(|node| node.kind() == SyntaxKind::LetDecl)
        .find(|declaration| declared_name(declaration).as_deref() == Some("expand"))
        .and_then(|declaration| declared_body(&declaration, adapter_source))
        .ok_or_else(|| {
            Box::new(refusal(
                site,
                format!("`{path}` declares no `expand`"),
                "an adapter module declares `let expand = fn (region) { … };`",
            ))
        })
}

/// The text of `let <name> = <this>;`.
fn declared_body(declaration: &SyntaxNode, source: &str) -> Option<String> {
    let mut equals = None;
    let mut semicolon = None;
    // Everything else in a `let` — the name, the type, the body's own tokens —
    // lies between the two marks rather than being one of them.
    for token in declaration.children_with_tokens().filter_map(|it| it.into_token()) {
        if token.kind() == SyntaxKind::Equals && equals.is_none() {
            equals = Some(u32::from(token.text_range().end()));
        } else if token.kind() == SyntaxKind::Semicolon {
            semicolon = Some(u32::from(token.text_range().start()));
        }
    }
    let from = usize::try_from(equals?).ok()?;
    let to = usize::try_from(semicolon.unwrap_or_else(|| u32::from(declaration.text_range().end()))).ok()?;
    Some(source.get(from..to)?.trim().to_owned())
}

/// The name a `let` declares.
fn declared_name(declaration: &SyntaxNode) -> Option<String> {
    declaration
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
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

/// Whether a statement is `import syntax …`.
fn is_syntax_import(node: &SyntaxNode) -> bool {
    node.kind() == SyntaxKind::ImportStmt
        && node
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .any(|token| token.kind() == SyntaxKind::SyntaxKw)
}

/// Step 3: the header's syntax imports, in the order they are written.
fn syntax_imports(root: &SyntaxNode) -> Vec<SyntaxImport> {
    root.descendants()
        .filter(is_syntax_import)
        .filter_map(|node| {
            let mut path = String::new();
            let mut alias = None;
            let mut after_as = false;
            // The words of the statement are read off in order: the two that
            // open it and the `;` that closes it say nothing, `as` switches
            // from the path to the name, and everything left is one or the
            // other. A module may be named after a domain keyword, so what
            // spells a path is not only identifiers.
            for token in node.children_with_tokens().filter_map(|it| it.into_token()) {
                let kind = token.kind();
                if kind == SyntaxKind::AsKw {
                    after_as = true;
                } else if kind == SyntaxKind::Colon {
                    path.push(':');
                } else if kind.is_trivia()
                    || matches!(
                        kind,
                        SyntaxKind::ImportKw | SyntaxKind::SyntaxKw | SyntaxKind::Semicolon
                    )
                {
                } else if after_as {
                    alias = Some(token.text().to_owned());
                } else {
                    path.push_str(token.text());
                }
            }
            Some(SyntaxImport {
                alias: alias?,
                path,
                ends: u32::from(node.text_range().end()),
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
            "fn (region) {{ Ok(syntax_fold(fn (here) {{ syntax_token(syntax_built(here, 0, 0), \"Missing\", \"\") }}, \
             fn (here, kind, text) {{ syntax_token(syntax_built(here, 1, 0), kind, text) }}, \
             fn (here, name) {{ syntax_identifier(syntax_built(here, 2, 0), name) }}, \
             fn (here, delimiter, children) {{ {emitted} }}, region)) }}"
        )
    }

    /// Expand one region's worth of text through `transformer`, and print it.
    fn answer(transformer: &str, region: &str) -> Result<crate::syntax::Printed, crate::core::ExpansionFailure> {
        let read = musa_language::parse(region);
        let subject = crate::syntax::read_region(&read.syntax(), crate::syntax::ExpansionPath::at(vec![0]));
        crate::core::expand_syntax(transformer, subject)
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
            r#"syntax_token(syntax_built(here, 4, 0), "ImportKw", "import \"x.musa\";")"#,
            r#"syntax_token(syntax_built(here, 4, 0), "LetKw", "let generated = 3;")"#,
            r#"syntax_token(syntax_built(here, 4, 0), "ModKw", "mod generated;")"#,
            r#"syntax_token(syntax_built(here, 4, 0), "DataKw", "data Generated { One }")"#,
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
            &answering(r#"syntax_token(syntax_built(here, 4, 0), "SyntaxKw", "syntax doubled { c4 }")"#),
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
        let refusal = transformer_of(
            "library {\n    let expand = syntax other { c4 };\n}\n",
            "std::adapters::circular",
            SourceSpan::new(0, 1),
        )
        .expect_err("the bootstrap is adapter-free");
        assert!(refusal.message.contains("adapter of its own"), "{}", refusal.message);
    }

    #[test]
    fn a_region_that_binds_and_uses_one_name_keeps_them_together_and_apart() {
        // One binding, written twice: the binder and the reference ask for the
        // same binding path, so they must print as one name.
        let bound = r#"syntax_group(syntax_built(here, 3, 0), "parentheses",
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
            !read
                .syntax()
                .descendants()
                .any(|node| node.kind() == SyntaxKind::SyntaxRegion || is_syntax_import(&node)),
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
        let refused = r#"Err((syntax_token(syntax_built(here, 9, 0), "Missing", ""), "nothing here is mine"))"#;
        let refusing = format!(
            "fn (region) {{ syntax_fold(fn (here) {{ {refused} }}, fn (here, kind, text) {{ {refused} }}, \
             fn (here, name) {{ {refused} }}, fn (here, delimiter, children) {{ {refused} }}, region) }}"
        );
        let read = musa_language::parse("c4");
        let subject = crate::syntax::read_region(&read.syntax(), crate::syntax::ExpansionPath::at(vec![0]));
        let Err(generated) = crate::core::expand_syntax(&refusing, subject).0 else {
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
        // And end to end, under a budget small enough to stop the run: the
        // same region that refuses at the language budget must not report the
        // stop as the adapter's own sentence.
        let source = piece(REFUSED);
        let narrow =
            crate::core_budget::under_budget(crate::core_budget::Budget::LANGUAGE.narrowed(512), || run(&source));
        for diagnostic in &narrow.diagnostics {
            assert_ne!(
                diagnostic.code,
                Code::Expansion,
                "a narrowed budget reported a stop as the adapter refusing: {}",
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
}
