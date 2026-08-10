---
id: 104
slug: library-modules-and-functors
status: done
depends_on: [99, 103]
phase: 3
---

# Library Signatures and Static Module Functors

## Task

Extend structural templates with the one case direct typed parameters cannot express: a parameter that is a bundle of
related declarations from a library. Add small explicit module signatures and static module-to-module functors, with a
real tonal-context bundle as the caller, while keeping modules out of the value calculus and runtime.

## Read

- `docs/language/04-templates-and-modules.md`; prompt 99's bundled library and prompt 103's structural staging.
- Current import/reference/index behavior across files and projects.
- The design comparison in `docs/elaboration-language.md`: Reader `local`, `music -> music`, declaration template, and
  OCaml-style module functor are four different mechanisms.

## Design

A signature lists exported values, functions, and music bindings with their ordinary types; each member is a `let`
without its definition, because a function is a value of arrow type and one member form covers all three. Signature
members that name an identity-bearing declaration, and template members, are deferred until something calls for them.
No abstract types, type members, subtyping, implicit module search, or first-class modules are needed for the concrete
caller. Matching is structural by name and exact type; a missing member and a mistyped member are reported with both
the signature's and the module's labels. A member the signature does not list is *private* rather than an error —
that is what sealing means — and naming one from outside is reported against the signature that hides it.

A functor is a statically named map from modules matching signatures to a library module. Instantiation is generative
at the structural level with stable identity derived from functor, argument-module identities, and instance site. It is
finite, acyclic, deterministic, and evaluated before piece/voice templates and context tracks. Applicative identity or
runtime module sharing must not be inferred accidentally; document the chosen generative law and test it.

The concrete caller is a `TonalContext` signature bundling a `key`, default `scale`, and spelling/voicing functions. A
canon/sequence library functor consumes it and exports ordinary music functions used by two piece templates. This
demonstrates why a bundle helps without making the module system a language inside the language.

## Target

- Surface syntax/formatter/tree-sitter for signatures, modules, and static functor application from the governing spec.
- Private module checker/matcher/instantiator in the structural stage; import/reference/LSP definition support.
- `stdlib/context.musa` with the concrete signature and two modules; `examples/module-functor-study.musa` with a real
  consumer.
- `crates/musa-compiler/tests/module_laws.rs`: matching, generativity/stability, substitution, acyclicity, diagnostic
  labels, and equivalence to direct declarations.
- Developer documentation stating why no Rust `Functor` trait or public module object exists.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/module-functor-study.musa
cd editors/tree-sitter-musa && tree-sitter test
cargo bench -p musa-compiler
```

Commit as `Add static library module functors`.

## Stop

- No abstract/associated types, type-level computation, first-class modules, recursive modules, dynamic linking, or
  runtime functor application.
- No public Rust trait mirroring the source feature.
- No `piece -> piece` or `voice -> voice`; structural templates remain the only place those declarations are built.
- Do not add module machinery with no caller beyond the committed tonal-context fixture.
