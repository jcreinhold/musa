use musa_compiler::{CompileOptions, ImportSources, NameKind, SourceDocument, compile};

fn compile_core(declarations: &str) -> musa_compiler::Compilation {
    let source = SourceDocument::new(
        format!("piece \"Core laws\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
        "core-laws.musa",
    );
    compile(&source, &CompileOptions::default())
}

#[test]
fn scalar_functions_capture_lexical_values_and_accept_higher_order_arguments() {
    let compilation = compile_core(
        "let saved: Nat = 7; \
         fn keep(discard: Bool) -> Nat { saved } \
         fn id(x: Nat) -> Nat { x } \
         fn apply(f: Nat -> Nat, x: Nat) -> Nat { f(x) } \
         let captured: Nat = keep(false); \
         let applied: Nat = apply(id, captured);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    assert!(compilation.snapshot().is_some());
}

#[test]
fn products_are_checked_and_evaluated_at_every_width() {
    let compilation = compile_core(
        "let pair: (Nat, Bool) = (3, true); \
         let wide: (Nat, Bool, Nat) = (3, true, 4); \
         fn keep(value: (Nat, Bool), ornament: Interval) -> (Nat, Bool) { value } \
         let first: (Nat, Bool) = keep(pair, P5); \
         let second: (Nat, Bool) = keep(pair, M3);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn a_wide_product_nests_the_same_way_written_out_by_hand() {
    // What makes `(A, B, C)` a spelling rather than an encoding: the composer
    // who nests by hand gets the same type and the same value, so the two ways
    // of writing it are interchangeable in both directions. If the fold leaned
    // the other way this would be a conversion mismatch at every one of the
    // four bindings.
    let compilation = compile_core(
        "let wide: (Nat, Bool, Nat) = (3, true, 4); \
         let nested: (Nat, (Bool, Nat)) = wide; \
         let byHand: (Nat, (Bool, Nat)) = (3, (true, 4)); \
         let back: (Nat, Bool, Nat) = byHand;",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn an_argument_label_at_a_use_is_refused() {
    // A field is named at its declaration, and a label at a *use* is either
    // agreeing with the position it already sits in or contradicting it. The
    // law is that neither is a second way to pass an argument, so the refusal
    // names the position rule rather than deferring to a later prompt.
    let compilation = compile_core(
        "fn keep(value: Nat, ornament: Interval) -> Nat { value } \
         let second: Nat = keep(ornament: M3, value: 3);",
    );
    assert!(compilation.has_errors(), "a labelled argument was accepted");
}

#[test]
fn forward_dependencies_are_acyclic_not_source_ordered() {
    let compilation = compile_core(
        "fn first(x: Nat) -> Nat { second(x) } \
         fn second(x: Nat) -> Nat { x } \
         let answer: Nat = first(4);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn definition_and_use_spans_reach_the_existing_reference_index() {
    let compilation = compile_core("let seed: Nat = 3; fn keep(x: Nat) -> Nat { seed } let answer: Nat = keep(seed);");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    let seed = compilation
        .references()
        .iter()
        .find(|reference| reference.kind == NameKind::Value && reference.name == "seed");
    assert!(matches!(seed, Some(reference) if reference.declaration.is_some() && reference.uses.len() == 2));
    let keep = compilation
        .references()
        .iter()
        .find(|reference| reference.kind == NameKind::Function && reference.name == "keep");
    assert!(matches!(keep, Some(reference) if reference.declaration.is_some() && reference.uses.len() == 1));
}

#[test]
fn imported_values_are_lexical_dependencies_not_a_second_evaluator() {
    let mut imports = ImportSources::default();
    imports.insert(
        "theory.musa",
        "library { let basis: Nat = 5; fn preserve(x: Nat) -> Nat { x } }",
    );
    let source = SourceDocument::new(
        "piece \"Imported core\" { import \"theory.musa\"; let answer: Nat = preserve(basis); score { part p { voice v { c4/1 } } } }",
        "piece.musa",
    );
    let compilation = compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}
