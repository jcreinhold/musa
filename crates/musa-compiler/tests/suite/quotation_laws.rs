//! What `quote at here { … }` means, from outside the compiler (prompt 139).
//!
//! `docs/rules/language/11-quotation.md` §2 and §3 are the specification, and
//! every law here is a law about a program an *adapter author* writes. So each
//! one runs the real thing: a probe adapter is handed to the compiler the way
//! any package adapter is, and what the compiler says about the piece that
//! imports it is the law's answer.
//!
//! **The observations are types and refusals, never printed text.** An
//! expansion becomes source and goes back through the ordinary reader, so what
//! a quote built is observable as what the piece can then do with it: a
//! substitution that dropped its splice does not have the type the piece
//! annotated, a capture changes which binding a name means and therefore which
//! type it has, and a derivation that collided is reported by the expansion
//! gate. Reading the printed text instead would test the printer's spacing,
//! which is not what any of these say.
//!
//! Five laws, and they are different in kind:
//!
//! - **Substitution.** A spliced value arrives where the splice stood, and a
//!   spread arrives with the separators its position supplies — including the
//!   length that made the hand-written `call1`…`call7` family necessary, zero.
//! - **Hygiene.** A binder a quote writes and a name spliced into it are
//!   different names, so a composer's own binding survives being quoted
//!   around; and a quoted binder and a quoted use of it stay one name.
//! - **Identity.** Two literal positions in one quote are two nodes, two
//!   quotes at one anchor are two sites, and one quote used twice at one
//!   anchor is one site — which the gate reports rather than silently merging.
//! - **The locus.** A quote's identity comes from the anchor it is evaluated
//!   with, so one helper called at every node of a region builds one tree per
//!   node and none of them collide.
//! - **Charging.** Everything a quote builds is charged to the expansion's
//!   budget, so a quote is not a way to build syntax for free.
//!
//! §7's refusals are below the laws, one test each.

// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import};

use musa_score::Severity;

/// Where the probe adapter is imported from, as the piece writes it.
const PROBE: &str = "probe::adapter";

/// A readable adapter module holding `body`.
fn probe(body: &str) -> String {
    format!("library {{\n    let level = \"readable\";\n{body}\n}}\n")
}

/// An adapter that hands every node of the region to `emit` and answers with
/// what `emit` built at the region's own root.
///
/// A quote needs a `NodePath` and the only place one comes from is a
/// traversal — an adapter cannot invent a place — so this is the shape every
/// real construction site has. `built`'s return annotation is what fixes the
/// fold's answer at `Syntax<Expr>`, which is why `children` arrives as a list
/// of expressions rather than of trees.
///
/// A leaf has no children, so it is handed `[]` at the call — the element type
/// is the one `emit`'s own parameter names, and nothing here has to say it
/// twice.
fn folding(emit: &str) -> String {
    probe(&format!(
        "{emit}
    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {{ Ok(built(region)) }};

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {{
        syntax_fold_from_leaves(
            fn (here) {{ emit(here, []) }},
            fn (here, kind, text) {{ emit(here, []) }},
            fn (here, name) {{ emit(here, []) }},
            fn (here, delimiter, children) {{ emit(here, children) }},
            region,
        )
    }};
"
    ))
}

/// A piece whose one region is read by the probe adapter, with `declarations`
/// written above it and the region's answer bound as `binding`.
///
/// `binding` carries the annotation, because the annotation is the
/// observation: what the expansion built is whatever the piece can then say
/// about it.
fn piece(declarations: &str, binding: &str, contents: &str) -> String {
    format!(
        "piece \"probe\" {{\n    import syntax {PROBE} as probe;\n\n{declarations}    let {binding} = syntax probe {{ \
         {contents} }};\n\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
    )
}

/// Every error compiling that piece against that adapter reported, as the
/// whole small document each one is — and every **cause** with it.
///
/// All three parts, because a refusal puts the claim in the message, the
/// reason in the note, and the repair in the help — and a test that reads only
/// the first line tests less than it looks like it does. A fault inside the
/// adapter module arrives as a cause of the diagnostic about the import, whole
/// and one per fault, which is what lets these laws read the note at all.
fn errors(declarations: &str, binding: &str, contents: &str, module: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(declarations, binding, contents), "probe.musa");
    let mut imports = ImportSources::default();
    imports.insert(resolve_import("probe.musa", PROBE), module.to_owned());
    compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
    .diagnostics()
    .iter()
    .filter(|diagnostic| diagnostic.severity == Severity::Error)
    .flat_map(|diagnostic| {
        let mut found = vec![whole(
            &diagnostic.message,
            diagnostic.note.as_deref(),
            diagnostic.help.as_deref(),
        )];
        found.extend(
            diagnostic
                .causes
                .iter()
                .map(|cause| whole(&cause.message, cause.note.as_deref(), cause.help.as_deref())),
        );
        found
    })
    .collect()
}

/// One diagnostic, or one cause, as the small document it is.
fn whole(message: &str, note: Option<&str>, help: Option<&str>) -> String {
    let mut lines = vec![message.to_owned()];
    if let Some(note) = note {
        lines.push(format!("note: {note}"));
    }
    if let Some(help) = help {
        lines.push(format!("help: {help}"));
    }
    lines.join("\n")
}

/// The same, for a region whose answer the piece binds without an annotation.
fn plain(contents: &str, module: &str) -> Vec<String> {
    errors("", "held", contents, module)
}

#[test]
fn a_spliced_value_arrives_where_the_splice_stood() {
    // §2's substitution, observed as a type. The quote builds a pair whose
    // right side is another quote's answer, so the piece sees `(Nat, Text)` —
    // and a splice that had been dropped would leave a pair the piece cannot
    // have annotated that way.
    let module = folding(
        r#"
    let inner = fn (here: NodePath) -> Syntax<Expr> { quote at here { "spliced" } };

    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { (1, ${ inner(here) }) }
    };
"#,
    );
    let found = errors("", "pair: (Nat, Text)", "a", &module);
    assert!(
        found.is_empty(),
        "the spliced text did not arrive in the pair's second place: {found:?}"
    );
    // The same program with the wrong annotation fails, so the half above is
    // evidence rather than a program that would have compiled either way.
    let refused = errors("", "pair: (Nat, Nat)", "a", &module);
    assert!(
        !refused.is_empty(),
        "the pair's second place was not the spliced value at all"
    );
}

#[test]
fn a_spread_arrives_with_the_separators_its_position_supplies() {
    // §2: "the separator a sequence splice needs … is supplied by the grammar
    // of the position". A spread into a product is therefore that many
    // components and not one run of adjacent nodes — which is exactly the work
    // `call1`…`call7` were doing by hand, and the reason the form exists. The
    // annotation counts the components, so a missing separator, a doubled one,
    // or a trailing one all fail here rather than in the printer. The literal
    // head is there because it is a position too: the separator between it and
    // the spread's first element is minted by the same rule.
    let module = probe(
        r#"
    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(built(region)) };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { one(here) },
            fn (here, kind, text) { one(here) },
            fn (here, name) { one(here) },
            fn (here, delimiter, children) { quote at here { ("head", $..children) } },
            region,
        )
    };
"#,
    );
    // Seven, because a read region keeps its trivia and there is nothing else
    // for the count to be: the composer's three digits and the four spaces
    // around them, each folded to a `1`. What the annotation is checking is the
    // arity of the product they arrived in.
    let found = errors("", "held: (Text, Nat, Nat, Nat, Nat, Nat, Nat, Nat)", "1 2 3", &module);
    assert!(
        found.is_empty(),
        "a spread of seven did not build seven components: {found:?}"
    );
    // And it is the spread's own length, not a fixed one: a shorter region is a
    // shorter product, which is the length arithmetic `call1`…`call7` did by
    // hand and got wrong at the ends.
    let shorter = errors("", "held: (Text, Nat, Nat, Nat)", "1", &module);
    assert!(
        shorter.is_empty(),
        "a spread of three did not build three components: {shorter:?}"
    );
}

#[test]
fn an_empty_spread_leaves_no_separator_behind() {
    // The length the hand-written assembly always got wrong. A position that
    // supplies its own separators supplies none around nothing, so an empty
    // list spreads to an empty position rather than to a stray comma — and a
    // stray comma is not source the ordinary reader accepts.
    let module = probe(
        r"
    let nothing: List<Syntax<Expr>> = [];

    let empty = fn (here: NodePath) -> Syntax<Expr> { quote at here { [$..nothing] } };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(built(region)) };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { empty(here) },
            fn (here, kind, text) { empty(here) },
            fn (here, name) { empty(here) },
            fn (here, delimiter, children) { quote at here { [$..children] } },
            region,
        )
    };
",
    );
    let found = errors("", "held: List<List<Nat>>", "1", &module);
    assert!(
        found.is_empty(),
        "a leaf's empty spread left a separator behind: {found:?}"
    );
}

#[test]
fn a_quoted_binder_does_not_capture_a_spliced_name() {
    // §2's hygiene rule, in the direction that is observable. The quote binds
    // `held`, and the name spliced into its body is the composer's own `held`.
    // If the binder captured it the spliced name would mean the quote's `Nat`;
    // because it does not, it still means the file's `Text` — and those are
    // two types, so the annotation says which one happened.
    let module = folding(
        r"
    let inner = fn (here: NodePath) -> Syntax<Expr> { quote at here { held } };

    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { (fn (held: Nat) { ${ inner(here) } })(1) }
    };
",
    );
    let declared = "    let held: Text = \"composer\";\n";
    let found = errors(declared, "kept: Text", "a", &module);
    assert!(
        found.is_empty(),
        "a quote's own binder captured the name spliced into it: {found:?}"
    );
    let refused = errors(declared, "kept: Nat", "a", &module);
    assert!(
        !refused.is_empty(),
        "the spliced name took the quote's binding, which is the capture hygiene forbids"
    );
}

#[test]
fn a_quoted_binder_and_a_quoted_use_of_it_are_one_name() {
    // The other half of hygiene: renaming a binder is only sound if its own
    // uses are renamed with it. So a quote that writes `fn (held) { held }`
    // still means the identity function after it has been printed out, even
    // though neither `held` is spelled `held` in the answer.
    let module = folding(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { (fn (held: Nat) { held })(1) }
    };
",
    );
    let found = errors("", "kept: Nat", "a", &module);
    assert!(
        found.is_empty(),
        "a quoted binder and its own use came apart: {found:?}"
    );
}

#[test]
fn two_literal_positions_in_one_quote_are_two_nodes() {
    // §3's identity law, in the direction that has to hold for a quote to be
    // usable at all: the same shape written three times inside one quote is
    // three nodes, because `path` is a position in the quote's own tree.
    // Nobody counts, and the duplicate-place gate is the evidence.
    let module = folding(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { (1, 1, 1) }
    };
",
    );
    let found = errors("", "held: (Nat, Nat, Nat)", "a", &module);
    assert!(
        found.is_empty(),
        "three identical literals in one quote collided: {found:?}"
    );
}

#[test]
fn two_quotes_at_one_anchor_are_two_sites() {
    // Why `quotation` is in the triple at all. Two *different* quotes reading
    // one input node are two construction sites, so what they build never
    // collides however alike they are written — which is what an adapter with
    // twenty construction sites needs, and what twenty-seven hand-allocated
    // role integers were for.
    let module = folding(
        r"
    let left = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let right = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { [${ left(here) }, ${ right(here) }] }
    };
",
    );
    let found = errors("", "held: List<Nat>", "a", &module);
    assert!(
        found.is_empty(),
        "two quotes at one anchor were treated as one site: {found:?}"
    );
}

#[test]
fn one_quote_used_twice_at_one_anchor_is_one_site() {
    // The direction that makes the law a law: `origin`, `quotation`, and
    // `path` are all of it, so one quote evaluated twice with one anchor mints
    // the same identities both times. §3 says the gate now catches a
    // compiler-shaped mistake rather than an author's miscounted integer, so
    // what it must not do is pass.
    let module = folding(
        r"
    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { [${ one(here) }, ${ one(here) }] }
    };
",
    );
    let found = errors("", "held: List<Nat>", "a", &module);
    assert!(
        found.iter().any(|error| error.contains("not a well-formed expression")),
        "one quote at one anchor twice was mistaken for two nodes: {found:?}"
    );
}

#[test]
fn a_quote_takes_its_identity_from_the_anchor_it_is_evaluated_with() {
    // The locus rule: what a quote builds is derived from the anchor *at the
    // point the quote runs*, not from where its answer is eventually placed.
    // One emitting helper called at every node of a region therefore builds
    // one tree per node, and the piece receives all of them at once with
    // nothing collided.
    let module = probe(
        r"
    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { (1, 1) } };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(built(region)) };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { one(here) },
            fn (here, kind, text) { one(here) },
            fn (here, name) { one(here) },
            fn (here, delimiter, children) { quote at here { [$..children] } },
            region,
        )
    };
",
    );
    let found = errors("", "held: List<(Nat, Nat)>", "1 2 3", &module);
    assert!(
        found.is_empty(),
        "one helper at three anchors collided with itself: {found:?}"
    );
}

#[test]
fn what_a_quote_builds_is_charged() {
    // A quote is not a way to make syntax for free, and an expansion that
    // outgrows the budget is a resource limit rather than an expansion fault —
    // the report's kind is what the budget-independence law requires
    // (`00-semantics.md` §2), and this pins it on the quote path.
    //
    // What the meter can no longer be asked to say: how many *nodes* a quote
    // wrote. Instantiation is one δ-firing — the tree is built host-side and
    // the size charge lands at the expansion boundary's generated-node count,
    // whose limit a quote cannot reach before the fold's own wall: the group
    // branch's children arrive as one right-nested `Cons` chain, so a flat
    // region evaluates as deep as it is wide, and a 256-level budget refuses
    // one at 123 siblings (measured; prompt 144 owns the depth, and this law
    // sizes its regions around the wall rather than pretending the wall is
    // about what a quote builds).
    let narrow = probe(
        r"
    let narrow = fn (here: NodePath) -> Syntax<Expr> { quote at here { [1] } };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(built(region)) };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { narrow(here) },
            fn (here, kind, text) { narrow(here) },
            fn (here, name) { narrow(here) },
            fn (here, delimiter, children) { narrow(here) },
            region,
        )
    };
",
    );
    // The same adapter over two regions: one the fold can read end to end,
    // one four times wider than the wall. The first compiling is what makes
    // the second's stop a statement about the region's size, and the stop
    // reading as a limit is the law.
    let spared = plain(&"1 ".repeat(64), &narrow);
    assert!(
        spared.is_empty(),
        "a quote per node over a region the fold can read: {spared:?}"
    );
    let found = plain(&"1 ".repeat(400), &narrow);
    assert!(
        found.iter().any(|error| error.contains("crossed a compilation limit")),
        "a region past the wall was expanded anyway, or the stop read as the adapter's fault: {found:?}"
    );
}

#[test]
fn an_adapter_helper_is_inferred_in_the_phase_it_is_checked_in() {
    // A declaration whose type the file did not write in full is inferred
    // before anything reads it — and an adapter's declarations are checked in
    // the expansion phase, where `Syntax<Cat>` exists and `quote` is a form.
    // Inferring them as ordinary source instead fails every one of them, and a
    // declaration that pass failed to infer means nothing to its uses: each one
    // instantiates a fresh variable, so the name is read as "something of this
    // arity" rather than as the type it has.
    //
    // What that costs is visible in the shape adapter code is written in. A
    // helper takes `List<Syntax<Expr>>` and a leaf case calls it with a
    // one-element list built on the spot. The callee's parameter is what says
    // what the literal holds, so the literal needs no annotation of its own —
    // while `children`, which arrives already typed from the fold, would be
    // accepted either way.
    let module = probe(
        r"
    let one = fn (here: NodePath) -> Syntax<Expr> { quote at here { 1 } };

    let spread = fn (here: NodePath, items: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { [$..items] }
    };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> { Ok(built(region)) };

    let built = fn (region: Syntax<TokenTree>) -> Syntax<Expr> {
        syntax_fold_from_leaves(
            fn (here) { spread(here, [one(here)]) },
            fn (here, kind, text) { spread(here, [one(here)]) },
            fn (here, name) { spread(here, [one(here)]) },
            fn (here, delimiter, children) { spread(here, children) },
            region,
        )
    };
",
    );
    // The annotation is the observation, as everywhere else here: the region is
    // a group of leaves, so each leaf's one-element list arrives inside the
    // group's own spread. A literal the callee's parameter had not decided
    // could not have reached `List<Nat>`.
    let found = errors("", "held: List<List<Nat>>", "1", &module);
    assert!(
        found.is_empty(),
        "a list literal whose element type the callee's parameter decides was refused: {found:?}"
    );
    let refused = errors("", "held: List<Nat>", "1", &module);
    assert!(
        !refused.is_empty(),
        "the leaves' lists were not built at all, so the half above proves nothing"
    );
}

// --- §7's refusals ----------------------------------------------------------

#[test]
fn a_quote_in_an_inferring_position_builds_at_expr() {
    // §2, as the registry fixes it: a quote's category is never searched for,
    // because construction is always at `Expr` — an inferring position infers
    // `Syntax<Expr>`, and a token-tree position receives the same value by
    // §1's forgetting. What can fail is the certificate, not the category.
    let module = probe(
        r"
    let unannotated = fn (here: NodePath) { quote at here { 1 } };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {
        Ok(syntax_fold_from_leaves(
            fn (here) { unannotated(here) },
            fn (here, kind, text) { unannotated(here) },
            fn (here, name) { unannotated(here) },
            fn (here, delimiter, children) { unannotated(here) },
            region,
        ))
    };
",
    );
    let found = plain("a", &module);
    assert!(
        found.is_empty(),
        "an unannotated quote was refused, or inferred as something other than `Syntax<Expr>`: {found:?}"
    );
}

#[test]
fn a_splice_of_the_wrong_category_names_both_categories() {
    // §7: `$x` at a position of a different category names both categories and
    // the repair — which is not an annotation but `as_expression`, the
    // operation that establishes the claim by running the parser.
    let module = probe(
        r"
    let loose = fn (here: NodePath, tree: Syntax<TokenTree>) -> Syntax<Expr> {
        quote at here { [$tree] }
    };

    let expand = fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>> {
        Ok(syntax_fold_from_leaves(
            fn (here) { loose(here, region) },
            fn (here, kind, text) { loose(here, region) },
            fn (here, name) { loose(here, region) },
            fn (here, delimiter, children) { loose(here, region) },
            region,
        ))
    };
",
    );
    let found = plain("a", &module);
    assert!(
        found
            .iter()
            .any(|error| error.contains("`TokenTree`") && error.contains("`Expr`")),
        "an uncertified tree was spliced into an expression position: {found:?}"
    );
}

#[test]
fn a_spread_where_nothing_spreads_names_the_position() {
    // §7: `$..xs` where a sequence is not grammatical names the position and
    // that it holds one node. A quote's own body is one such position: there
    // is no separator there and no room for a second node.
    let module = folding(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { $..kids }
    };
",
    );
    let found = plain("a", &module);
    assert!(
        found.iter().any(|error| error.contains("spreads a sequence")),
        "a spread was admitted where one node stands: {found:?}"
    );
}

#[test]
fn a_quote_may_not_write_a_name_its_own_hygiene_could_produce() {
    // Hygiene crosses into ordinary source by renaming, and a renaming is a
    // spelling. A quote that writes that spelling itself would be writing a
    // name its own binders could capture, so it is refused where it is written
    // — the author reading that line is the one who can rename it.
    let module = folding(
        r"
    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { (fn (held: Nat) { held })(held_g0) }
    };
",
    );
    let found = plain("a", &module);
    assert!(
        found.iter().any(|error| error.contains("could generate")),
        "a quote wrote a name the printer could have written: {found:?}"
    );
}

#[test]
fn a_piece_cannot_write_a_quote_at_all() {
    // Phase conservativity, from the composer's side. `Syntax<Cat>` exists in
    // the expansion phase, so the form that builds one is refused in ordinary
    // source rather than merely having nothing to build.
    let source = SourceDocument::new(
        "piece \"no quotes\" {\n    let held = quote at here { 1 };\n\n    score { part p { voice v { c4/1 } } }\n}\n"
            .to_owned(),
        "probe.musa",
    );
    let found: Vec<String> = compile(&source, &CompileOptions::default())
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert!(
        found.iter().any(|error| error.contains("adapter's form")),
        "a piece built syntax: {found:?}"
    );
}

#[test]
fn two_refusals_in_one_module_arrive_as_two() {
    // §7's refusals are per quote, and a module holding two of them is told
    // about both. This is a law about delivery rather than about quotation:
    // the module's faults reach its author as causes, one per fault, so a
    // second refusal is not summarized away by the first — and each keeps the
    // note that says why, which is the part that says which refusal it is.
    let module = folding(
        r"
    let also = fn (here: NodePath) -> Syntax<Expr> {
        quote at here { (fn (held: Nat) { held })(held_g0) }
    };

    let emit = fn (here: NodePath, kids: List<Syntax<Expr>>) -> Syntax<Expr> {
        quote at here { $..kids }
    };
",
    );
    let found = plain("a", &module);
    assert!(
        found.iter().any(|error| error.contains("spreads a sequence")
            && error.contains("a spread needs the position's own separator")),
        "the spread refusal did not arrive whole: {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|error| error.contains("could generate") && error.contains("note:")),
        "the second refusal did not arrive at all: {found:?}"
    );
}
