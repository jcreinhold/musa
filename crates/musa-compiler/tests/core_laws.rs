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
        "let saved: nat = 7; \
         fn keep(discard: bool) -> nat = saved; \
         fn id(x: nat) -> nat = x; \
         fn apply(f: nat -> nat, x: nat) -> nat = f(x); \
         let captured: nat = keep(false); \
         let applied: nat = apply(id, captured);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
    assert!(compilation.snapshot().is_some());
}

#[test]
fn products_defaults_and_named_arguments_are_checked_and_evaluated() {
    let compilation = compile_core(
        "let pair: (nat, bool) = (3, true); \
         fn keep(value: (nat, bool), ornament: interval = P5) -> (nat, bool) = value; \
         let first: (nat, bool) = keep(pair); \
         let second: (nat, bool) = keep(ornament: M3, value: pair);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn forward_dependencies_are_acyclic_not_source_ordered() {
    let compilation = compile_core(
        "fn first(x: nat) -> nat = second(x); \
         fn second(x: nat) -> nat = x; \
         let answer: nat = first(4);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn definition_and_use_spans_reach_the_existing_reference_index() {
    let compilation = compile_core("let seed: nat = 3; fn keep(x: nat) -> nat = seed; let answer: nat = keep(seed);");
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
        "library { let basis: nat = 5; fn preserve(x: nat) -> nat = x; }",
    );
    let source = SourceDocument::new(
        "piece \"Imported core\" { use \"theory.musa\"; let answer: nat = preserve(basis); score { part p { voice v { c4/1 } } } }",
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
