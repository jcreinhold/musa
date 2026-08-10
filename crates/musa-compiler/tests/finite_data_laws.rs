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
        "let empty: List[Nat] = []; \
         let singleton: List[Nat] = [1]; \
         let nested: List[List[Nat]] = [empty, singleton]; \
         let absent: Option[Nat] = None; \
         let present: Option[List[Nat]] = Some(singleton);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn compiler_owned_schemes_monomorphize_at_each_direct_call() {
    let compilation = compile_data(
        "fn id(x: Nat) -> Nat { x } \
         fn keep(x: Nat) -> Bool { true } \
         let counted: List[Nat] = range(8); \
         let mapped: List[Nat] = map(id, counted); \
         let filtered: List[Nat] = filter(keep, mapped); \
         let repeated: List[Bool] = repeat(false, 4);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn structural_folds_cover_empty_singleton_and_nonempty_inputs() {
    let compilation = compile_data(
        "fn keep_index(index: Nat, accumulator: Nat) -> Nat { accumulator } \
         fn keep_item(item: Nat, accumulator: Nat) -> Nat { accumulator } \
         fn some_value(value: Nat) -> Nat { value } \
         let by_nat: Nat = nat_fold(7, keep_index, 16); \
         let by_empty: Nat = list_fold(7, keep_item, []); \
         let by_list: Nat = list_fold(7, keep_item, [1, 2, 3]); \
         let by_none: Nat = option_fold(7, some_value, None); \
         let by_some: Nat = option_fold(7, some_value, Some(9));",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}

#[test]
fn match_is_exhaustive_and_binds_constructor_members() {
    let compilation = compile_data(
        "fn from_bool(value: Bool) -> Nat { match value { true -> 1, false -> 0 } } \
         fn from_option(value: Option[Nat]) -> Nat { match value { None -> 0, Some(found) -> found } } \
         fn from_list(value: List[Nat]) -> Nat { match value { [] -> 0, [head, ..tail] -> head } } \
         fn swap(value: (Nat, Bool)) -> (Bool, Nat) { match value { (number, flag) -> (flag, number) } } \
         fn literal(value: Nat) -> Bool { match value { 0 -> false, _ -> true } } \
         let answer: Nat = from_option(Some(from_list([3]))); \
         let pair: (Bool, Nat) = swap((answer, true)); \
         let bit: Nat = from_bool(true); \
         let nonzero: Bool = literal(answer);",
    );
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics());
}
