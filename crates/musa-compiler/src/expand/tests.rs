use musa_language::SyntaxKind;
use musa_language::ast::{AstNode as _, ImportStmt};

use crate::compile::{CompileOptions, SourceDocument};
use crate::diagnose::Code;
use crate::origin::SourceSpan;

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
    let refusal = ordinary_expression(&printed.text, SourceSpan::new(0, 1)).expect_err("a region is not an emission");
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
        refused.charges.evaluation_steps > 0,
        "the run that refused was checked and evaluated, and is charged for it: {:?}",
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
                r#"syntax_anchor(region, syntax_built(here, 0, 0)).fold_from_start(syntax_token(syntax_built(here, 9, 0), TokenKind.Integer, "404"), fn (held, node) { node })"#,
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
                r#"syntax_anchor(region, here).fold_from_start(syntax_token(syntax_built(here, 9, 0), TokenKind.Integer, "404"), fn (held, node) { node })"#,
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
