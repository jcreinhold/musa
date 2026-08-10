use musa_compiler::{CompileOptions, SourceDocument, compile};

fn compile_data(declarations: &str) -> musa_compiler::Compilation {
    compile(
        &SourceDocument::new(
            format!("piece \"Finite data\" {{ {declarations} score {{ part p {{ voice v {{ c4/1 }} }} }} }}"),
            "finite-data.musa",
        ),
        &CompileOptions::default(),
    )
}

#[test]
fn finite_constructors_and_nested_data_are_values() {
    let compilation = compile_data(
        "let empty: list[nat] = []; \
         let singleton: list[nat] = [1]; \
         let nested: list[list[nat]] = [empty, singleton]; \
         let absent: option[nat] = none; \
         let present: option[list[nat]] = some(singleton);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn compiler_owned_schemes_monomorphize_at_each_direct_call() {
    let compilation = compile_data(
        "fn id(x: nat) -> nat = x; \
         fn keep(x: nat) -> bool = true; \
         let counted: list[nat] = range(8); \
         let mapped: list[nat] = map(id, counted); \
         let filtered: list[nat] = filter(keep, mapped); \
         let repeated: list[bool] = repeat(false, 4);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn structural_folds_cover_empty_singleton_and_nonempty_inputs() {
    let compilation = compile_data(
        "fn keep_index(index: nat, accumulator: nat) -> nat = accumulator; \
         fn keep_item(item: nat, accumulator: nat) -> nat = accumulator; \
         fn some_value(value: nat) -> nat = value; \
         let by_nat: nat = nat_fold(7, keep_index, 16); \
         let by_empty: nat = list_fold(7, keep_item, []); \
         let by_list: nat = list_fold(7, keep_item, [1, 2, 3]); \
         let by_none: nat = option_fold(7, some_value, none); \
         let by_some: nat = option_fold(7, some_value, some(9));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn match_is_exhaustive_and_binds_constructor_members() {
    let compilation = compile_data(
        "fn from_bool(value: bool) -> nat = match value { true -> 1, false -> 0 }; \
         fn from_option(value: option[nat]) -> nat = match value { none -> 0, some(found) -> found }; \
         fn from_list(value: list[nat]) -> nat = match value { [] -> 0, [head, ..tail] -> head }; \
         fn swap(value: (nat, bool)) -> (bool, nat) = match value { (number, flag) -> (flag, number) }; \
         fn literal(value: nat) -> bool = match value { 0 -> false, _ -> true }; \
         let answer: nat = from_option(some(from_list([3]))); \
         let pair: (bool, nat) = swap((answer, true)); \
         let bit: nat = from_bool(true); \
         let nonzero: bool = literal(answer);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}
