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
fn products_defaults_and_named_arguments_are_checked_and_evaluated() {
    let compilation = compile_core(
        "let pair: (Nat, Bool) = (3, true); \
         fn keep(value: (Nat, Bool), ornament: Interval = P5) -> (Nat, Bool) { value } \
         let first: (Nat, Bool) = keep(pair); \
         let second: (Nat, Bool) = keep(ornament: M3, value: pair);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
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
