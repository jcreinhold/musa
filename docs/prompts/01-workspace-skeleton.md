---
id: 01
slug: workspace-skeleton
status: pending
depends_on: []
phase: 0
---

# Workspace Skeleton

## Task

Create the seven-crate Cargo workspace with the dependency direction the roadmap prescribes, so every later prompt has a
home and no prompt has to relitigate crate boundaries. Each crate gets its public facade as doc comments and stub
signatures; no feature code yet.

## Read

- Roadmap §3 (deep modules), §15 (workspace layout and each crate's ownership, dependencies, and public interface).
- `Cargo.toml` (workspace lints already configured), `deny.toml`, `rustfmt.toml`, `taplo.toml`.

## Design

- Convert the root `Cargo.toml` to a virtual-workspace layout: `members = crates/*`, root package removed (the roadmap
  §15 tree has no root package; the CLI is the binary). Keep `[workspace.lints]` as-is and add
  `[lints] workspace = true` to each member crate.
- Create exactly these crates, with only the roadmap §15 dependency edges: `musa-language` (no internal deps),
  `musa-compiler` (→ language), `musa-render` (→ compiler), `musa-audio` (→ compiler), `musa-engine` (→ compiler,
  audio), `musa-project` (→ language, compiler, render, audio, engine), `musa-cli` (→ project).
- Do **not** add third-party dependencies yet except where a facade signature needs the type (e.g. none do at this
  stage). Each prompt adds its own dependencies from the roadmap §15 lists; this keeps `deny.toml` review incremental.
- Each crate's `lib.rs` carries a crate-level doc comment stating: what it owns, what it must never expose, and its
  intended facade (the §15 signatures, as `//!` docs — not yet as code, except for `musa-cli`).
- `musa-cli` is a real binary crate with a clap-free `main` that prints the planned command surface (`check`, `format`,
  `render`, `play`) as a stub and exits successfully. Argument parsing arrives with the first real command in prompt 04.
- `apps/musa-desktop/` is **not** created here; it arrives with prompt 15.

## Target

- `Cargo.toml` converted to workspace; seven crates under `crates/` with `Cargo.toml` + `src/lib.rs` (or `main.rs`).
- `crates/musa-cli/src/main.rs` stub printing the command surface.
- Every crate compiles with zero warnings under the workspace lints.

## Check

```sh
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa-cli            # prints the stub command surface
cargo deny check                 # if cargo-deny is installed
```

Commit as `Add workspace skeleton with seven crates`.

## Stop

- No logos/rowan/quick-xml/cpal or any other third-party dependency.
- No real types, no tests beyond what compiles in `lib.rs` docs.
- No `musa-desktop` app, no `examples/` directory yet (first example arrives with prompt 03).
