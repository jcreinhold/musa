---
name: rust-macros
description: Use for Rust macros — declarative (macro_rules!) and procedural (derive, attribute, function-like); boilerplate reduction, trait impls, DSLs, repetition not solvable with generics or functions.
---

# Writing Rust Macros

Use a macro only when a function or generic cannot express the job. Read `references/macro-patterns.md` before writing one.

## Choose the Tool

| Need | Tool |
| --- | --- |
| Uniform repetition, type lists, variadic syntax, call-site `file!()`/`line!()` | `macro_rules!` |
| Inspect fields, parse attributes, or make per-item choices | Proc macro |
| Identifier joining or case conversion | `macro_rules!` with `paste`, or a proc macro if other logic needs it |

## Workflow

1. Rule out functions and generics.
2. Choose declarative or procedural before writing expansion code.
3. Separate parsing from emission.
4. Inspect the result with `cargo expand`.
5. Test every accepted input shape; use `trybuild` compile-fail tests for proc macros.
6. Use `$crate::` for all paths in `#[macro_export]` macros.

## Debugging

```bash
cargo expand
cargo expand module::name
```

On nightly, use `trace_macros!(true)` for `macro_rules!`. For proc macros, print the generated tokens while debugging and remove the output afterward.

## Common Errors

- Use a proc macro for field inspection.
- Respect follow-set restrictions: an `expr` fragment may only precede `=>`, `,`, or `;`.
- Bind an expression before using it twice.
- Return `syn::Error::new_spanned()` rather than panicking.
- Use token trees when a later macro must parse the input again; typed fragments are opaque.

## API

- Make input look like the intended output.
- Support relevant item attributes and `$vis:vis`.
- Make item macros work at module and function scope when their expansion permits it.
