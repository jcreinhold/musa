---
id: 143
slug: wasm-shell
status: done
depends_on: [13]
phase: 5
---

# A WebAssembly Shell Over the Existing Pipeline

## Task

Create `crates/musa-wasm`, the shell crate that carries the whole semantic pipeline — parse, compile,
notation plan, MEI render — into the browser as one small WebAssembly module. It is a shell like
`musa-cli` and `musa-lsp`: it depends on `musa-compiler` and `musa-render`, never the reverse, and it
adds no semantics of its own. Its only new work is translating `Diagnostic`s into a wasm-crossing data
type and packaging the artifact the way post-wasm-pack tooling prescribes.

## Read

- Roadmap §15 (shell architecture, dependency direction) and the deep-module conventions in
  `docs/prompts/README.md`.
- `crates/musa-compiler/src/compile.rs` — the `compile` / `SourceDocument` / `Compilation` facade this
  shell calls, and `crates/musa-render/src/render.rs` — `render_notation` with `NotationTarget::Mei`.
- `crates/musa-render/src/mei.rs` header: the `xml:id` contract the web package depends on downstream.
- Post-wasm-pack toolchain (wasm-pack and the rustwasm working group were sunset in July 2025):
  `cargo build --target wasm32-unknown-unknown` + pinned `wasm-bindgen-cli` (`--target web`) + pinned
  binaryen `wasm-opt`. `--target web`, not `bundler`: explicit `init()` avoids the top-level-await
  footgun and lets the JS side decide where assets live.

## Design

The crate is `crate-type = ["cdylib", "rlib"]` so its logic is testable natively; the wasm boundary is
three thin wrappers over ordinary Rust functions that native tests exercise directly.

```rust
/// What one snippet compiles to. `mei` is present exactly when the source
/// compiles to a score with no error-severity diagnostics.
#[derive(serde::Serialize)]
pub struct TypesetResult {
    pub mei: Option<String>,
    pub diagnostics: Vec<WebDiagnostic>,
}

/// A diagnostic that can cross the wasm boundary: severity, stable code,
/// message, byte-span labels, help/note. Fixes are not serialized yet —
/// no web caller can apply them (prompt 146 renders, it does not edit).
#[derive(serde::Serialize)]
pub struct WebDiagnostic {
    pub severity: String,        // "error" | "warning"
    pub code: String,
    pub message: String,
    pub labels: Vec<WebLabel>,   // primary first; byte spans, not line/col
    pub help: Option<String>,
    pub note: Option<String>,
}

// Native-testable core, `#[doc(hidden)] pub` so the integration tests reach
// it without a browser; not part of the documented surface:
fn typeset_impl(source: &str) -> TypesetResult;
fn validate_impl(source: &str) -> Vec<WebDiagnostic>;

// The entire wasm surface (returns `Result<JsValue, JsValue>`: an internal
// serialization failure rejects, an invalid score is a *result*):
#[wasm_bindgen] pub fn typeset(source: &str) -> Result<JsValue, JsValue>;   // serde_wasm_bindgen
#[wasm_bindgen] pub fn validate(source: &str) -> Result<JsValue, JsValue>;
```

Semantics: compile with `CompileOptions::default()` (deterministic realization — a snippet with open
form typesets its deterministic default), document name `"snippet.musa"`. `mei` is `Some` exactly when
`Compilation::snapshot()` is `Some` **and** `!has_errors()`; warnings pass through alongside the MEI.
`DocumentKind::Material` yields `mei: None` with no diagnostic — "no score ever" is not an error
(prompt 146 shows an empty-state, not an error box). `render_notation` failures become a single
error-severity `WebDiagnostic`, never a panic across the boundary; install
`console_error_panic_hook` so anything else is at least legible.

Spans stay **byte offsets**. Line/column is a display concern of the JS layer, which holds the source
text and can compute it losslessly; sending both is information duplicated, not information hidden.

Build and size: workspace `[profile.wasm]` (inherits `release`; `opt-level = "z"`, `lto = true`,
`codegen-units = 1`, `panic = "abort"`, `strip = true`), then `wasm-opt -Oz` with the target features
Rust emits by default enabled (`--enable-bulk-memory --enable-mutable-globals --enable-sign-ext`).
`scripts/build-wasm.sh`
runs the three steps and prints the brotli'd size; tool versions are pinned in `mise.toml`
(`ubi:rustwasm/wasm-bindgen`, `ubi:WebAssembly/binaryen`) with the manual `cargo install` fallback
documented in the script header. The wasm-bindgen **CLI** version must equal the crate's
`wasm-bindgen` version; the script checks and refuses on mismatch rather than emitting a broken glue.

## Target

- `crates/musa-wasm/` with the facade above, `serde_wasm_bindgen` for the crossing, and doc comments
  stating the `mei`-present invariant before implementation.
- Workspace `Cargo.toml`: the crate, the `[profile.wasm]` profile, the lint inheritance.
- `scripts/build-wasm.sh` + `mise.toml` pins; artifact written to `packages/musa-web/wasm/` (created by
  prompt 145; the script creates the directory so it can run first).
- Native tests: `examples/canon.musa` (and one snippet with warnings) typeset to MEI containing
  `event-` ids; a broken snippet from `examples/broken/` yields error diagnostics with correct byte
  spans; a material-only document yields `mei: None` and no errors; MEI output is byte-identical to
  `render_notation` called directly (the shell adds nothing).
- No wasm-bindgen-test harness: the boundary carries plain data, and the DOM behavior is Playwright's
  job (prompt 146). Revisit only if a bug is found that native + Playwright tests cannot see.

## Check

```sh
cargo nextest run -p musa-wasm
cargo clippy --all-targets -p musa-wasm -- -D warnings
cargo fmt --check
bash scripts/build-wasm.sh   # produces packages/musa-web/wasm/*.js + *.wasm, prints size
```

Commit as `Add the WebAssembly shell for web typesetting`.

## Stop

- No DOM, no worker, no Verovio, no npm packaging — that is prompts 144–146.
- No audio, no engine, no `musa-project`: the web shell compiles one self-contained snippet; imports
  beyond the bundled stdlib are out of scope until a web project model exists.
- No streaming compilation API, no incremental recompile, no shared `Compilation` cache — one snippet,
  one call, measured before optimized.
- No `--target bundler` dual build and no wasm-pack: one `--target web` artifact.
