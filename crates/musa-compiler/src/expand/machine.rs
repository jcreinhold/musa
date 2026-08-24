use std::collections::{BTreeMap, BTreeSet};

use musa_syntax::ast::{AstNode as _, ImportStmt};
use musa_syntax::{SyntaxKind, SyntaxNode};

use crate::compile::{CompileOptions, SourceDocument};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

use super::Charges;
use super::ExpansionRecord;
use super::SyntaxImport;
use super::refusal;

/// One region's answer, kept so a second identical region costs the same.
#[derive(Clone)]
pub(crate) struct Cached {
    printed: String,
    output: crate::quote::Syntax,
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
pub(crate) struct Refused {
    pub(crate) diagnostic: Box<Diagnostic>,
    pub(crate) charges: Charges,
}

impl From<Box<Diagnostic>> for Refused {
    fn from(diagnostic: Box<Diagnostic>) -> Self {
        Self {
            diagnostic,
            charges: Charges::default(),
        }
    }
}

pub(crate) fn expand_one(
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
    let subject = crate::quote::read_region(
        &body,
        crate::quote::ExpansionPath::at(vec![u32::try_from(ordinal).unwrap_or(u32::MAX)]),
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

    let (answer, work) = crate::phase::expand_syntax(
        adapter_source,
        crate::phase::PhaseImports::at(&uri, &options.imports),
        &subject,
    );
    // The run happened, so the run is charged, and everything below reports
    // against the same charge whether the adapter answered or refused.
    let charged = |generated_syntax_nodes| Charges {
        expansion_steps: 1,
        generated_syntax_nodes,
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
    let printed = crate::quote::print(&output);
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
pub(crate) fn stopped_or_refused(failure: &crate::phase::ExpansionFailure, path: &str, site: SourceSpan) -> Diagnostic {
    if let crate::phase::ExpansionFailure::Refused { message, at } = failure {
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
    if let crate::phase::ExpansionFailure::Stopped(ref limit) = *failure {
        return Diagnostic::error(
            Code::ResourceLimit,
            format!("expanding this region with `{path}` crossed a compilation limit: {limit}"),
        )
        .at(site, "this region")
        .help("the adapter is total, so this is a limit rather than a loop: give the region less to read");
    }
    // The gate's own sentence is carried through rather than summarized away:
    // each of its cases is a different mistake, and a transformer author
    // reading this has to be told which one they made — which of two nodes sat
    // at one path, or what a group that says it is one lexeme actually spells.
    let why = match *failure {
        crate::phase::ExpansionFailure::Stopped(_) | crate::phase::ExpansionFailure::Refused { .. } => {
            "handled above".to_owned()
        }
        crate::phase::ExpansionFailure::NotATransformer(_) => {
            "the adapter's `expand` is not a transformer — it must take one region and answer with one".to_owned()
        }
        crate::phase::ExpansionFailure::NoAnswer => "the adapter did not answer".to_owned(),
        crate::phase::ExpansionFailure::NotAnExpression(ref refused) => {
            format!("the adapter answered with something that is not a well-formed expression: {refused}")
        }
    };
    refusal(site, format!("`{path}` did not expand this region"), &why)
}

/// Step 5: the answer is one ordinary expression, or it is refused.
///
/// Checked by parsing it the way the composer's own source would be parsed,
/// which is what makes the four forbidden emissions one rule rather than four:
/// an `import`, a `mod`, a `data`, or a `let` is not an expression, so a
/// transformer that emits one does not get past the ordinary parser. The one
/// thing that *is* an expression and still may not be emitted is another
/// region, so that is asked separately.
pub(crate) fn ordinary_expression(text: &str, site: SourceSpan) -> Result<(), Box<Diagnostic>> {
    let parsed = crate::quote::read_expression(text);
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
pub(crate) fn region_name(region: &SyntaxNode) -> Option<String> {
    region
        .children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

/// The delimited group a region holds.
pub(crate) fn region_body(region: &SyntaxNode) -> Option<SyntaxNode> {
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
pub(crate) fn syntax_imports(root: &SyntaxNode) -> Vec<SyntaxImport> {
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
pub(crate) fn names_written(source: &str) -> BTreeSet<String> {
    musa_syntax::lex(source)
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
pub(crate) fn digest(source: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
