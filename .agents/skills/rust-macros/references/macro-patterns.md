# Rust Macro Patterns

## Declarative Macros

Use fragment kinds that match the needed syntax: `ident`, `path`, `ty`, `expr`, `pat`, `item`, `stmt`, `block`, `vis`, `literal`, and `tt`. Use `$(...)*`, `+`, or `?` for repetition. An `expr` fragment may only be followed by `=>`, `,`, or `;`.

Use separate parse and emit rules. Start with one stamp-out rule; add named internal rules for phases. Use token-tree munching only when the macro must consume custom syntax recursively. Typed fragments are opaque to a later macro, so capture `tt` when later parsing is required.

```rust
macro_rules! impl_from {
    ($( $variant:ident : $source:ty ),* $(,)?) => {
        $(impl From<$source> for AppError {
            fn from(value: $source) -> Self { Self::$variant(value) }
        })*
    };
}
```

## Procedural Macros

Parse input with `syn`, transform a typed representation, and emit with `quote`. Support generics and where clauses through the parsed AST. Parse attributes explicitly and report bad input with `syn::Error::new_spanned()`.

Test accepted forms and diagnostic failures with `trybuild`. Inspect output with `cargo expand`.

## Hygiene

Use `$crate::` for crate paths and absolute paths for standard-library items in exported declarative macros. Accept `$vis:vis` and relevant attributes for generated items. Avoid evaluating a captured expression more than once.
