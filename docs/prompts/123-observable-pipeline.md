---
id: 123
slug: observable-pipeline
status: in-progress
depends_on: [56, 77, 99, 122]
phase: 3
---

# The Pipeline Says What It Is Doing

## Task

Give musa a diagnostic voice. Today nine `tracing::warn!` calls sit in `musa-engine` and `musa-project` reporting
swallowed errors — a MIDI port that would not open, a recovery copy that could not be written, an unknown `musa.toml`
value — and not one of them is ever seen, because no shell installs a subscriber. That is worse than no logging: it is
logging that lies about being there. This prompt installs the subscriber in the three shells, gives the semantic
pipeline one span per public facade entry and events at the phase boundaries that are otherwise invisible, and fixes the
rules that keep instrumentation from becoming either noise or a performance tax.

The question a person asks a compiler when something is wrong is *which phase*, *on what document*, and *how much*.
Spans on the facades and counts at the boundaries answer exactly that and nothing more.

## Read

- Roadmap §15.6 and §15.7 — `tracing` is already a listed dependency of `musa-engine` and `musa-project`; §15.3–§15.5,
  §15.8 and §15.11 are silent and this prompt repairs them.
- Roadmap §13 (real-time rules) and `crates/musa-engine/tests/rt.rs` — the callback allocates nothing, locks nothing,
  and does no I/O. Logging is I/O.
- `crates/musa-compiler/src/compile.rs` and `src/elaborate.rs` — `compile` dispatches on the document alternative and
  `elaborate_parsed` is already split from parsing so the two can be measured apart (`crate::bench`).
- `crates/musa-project/src/session.rs` — `open`, `apply`, `export`, `analyze`, `realize` are the whole application's
  entry points.
- `crates/musa-lsp/src/lib.rs` `main_loop` — stdout is the JSON-RPC transport.
- `crates/musa/src/main.rs` — argument parsing, `install_renderer`, and the usage text that must gain the new flags.

## Design

### One policy, three installations

`musa-project` decides *what* is logged; each shell decides *that* it is logged. Every shell already depends on
`musa-project`, so one filter policy there is one policy everywhere; putting the `install` call in each `main` keeps the
global-state decision where it belongs, in a binary. No new crate: the roadmap's crate list is closed, and this is fifty
lines with three callers.

```rust
/// How much the pipeline should say, and where.
///
/// Logs are a *shell* concern: a library that installed a global subscriber
/// would decide for an embedder that never asked. This type carries the
/// policy — filter, format, destination — and `install` is called by a `main`.
pub struct Logging { /* hidden */ }

impl Logging {
    /// The default policy: warnings from musa's own crates, nothing else.
    pub fn new() -> Self;

    /// Raise the level: 1 = info, 2 = debug, 3 or more = trace.
    pub fn verbosity(self, count: u8) -> Self;

    /// Lower it to errors only. Wins over `verbosity`.
    pub fn quiet(self, quiet: bool) -> Self;

    /// Install this policy as the process-wide subscriber.
    ///
    /// Returns `false` when one is already installed, which is not an error:
    /// a host application may have installed its own, and musa does not fight
    /// it. Installing is idempotent, so a test may call it freely.
    pub fn install(self) -> bool;
}
```

### `MUSA_LOG`, not `RUST_LOG`

`RUST_LOG` is a shared namespace. A person debugging some other Rust tool in the same shell should not drown in musa's
elaboration trace, and a person debugging musa should not have to read `hyper`'s. `MUSA_LOG` takes the same `EnvFilter`
syntax — nothing new to learn — and names only this program.

`MUSA_LOG` **replaces** the filter; `-v`/`-q` choose the default when `MUSA_LOG` is unset. A dial and a filter are
different instruments, and the filter is the more specific of the two.

The default with neither is `warn` for musa's crates and off for everything else. That default is the point of the
prompt: it is what makes the nine existing warnings audible.

### Never stdout

Logs go to stderr in all three shells, with no option to change it. `musa render -o -` and `musa kernel` write payloads
to stdout, and the language server writes JSON-RPC there. A log line on stdout is a corrupted file or a protocol
violation, so the destination is not a knob a caller can turn the wrong way.

### What may be recorded

Two rules, both checkable, both there so instrumentation does not rot into noise or cost:

1. **One span per public facade entry.** `compile`, `render_notation` and each exporter, `compile_graph`,
   `AudioEngine::open`, each `ProjectSession` method, each LSP request. Not per voice, per note, or per graph node.
2. **A span or event records only what the code already computed.** No traversal, no formatting, and no allocation
   exists solely to fill a field. Where a useful count is not already to hand, it is guarded by `tracing::enabled!` or
   it is not recorded. This is what keeps a disabled callsite an atomic load and a branch.

Events sit at phase boundaries and at decisions that leave no other trace: which alternative a document was read as,
which module a name resolved to and from which path, how many diagnostics each phase added, the semantic identity hash,
the device and stream configuration CPAL negotiated, which seed a realization used and what it decided, and the revision
each session command produced.

### The callback stays silent

`musa-engine`'s control side is instrumented; its callback is not, and cannot become so by accident. Device negotiation
in `open` — the chosen device's name, the sample rate, the buffer size — is precisely what a bug report needs and
precisely what no user can otherwise see. Everything downstream of the `rtrb` boundary logs nothing.

## Target

- Roadmap repair: add `tracing` to the dependency lists of §15.3 `musa-compiler`, §15.4 `musa-render`, §15.5
  `musa-audio`, and §15.11 `musa-lsp`; add `tracing-subscriber` to §15.7 `musa-project`; state in §15.8 that the CLI
  installs the subscriber and reads `MUSA_LOG`. The lists are closed, so this is the prompt that opens them.
- `musa-project`: a new `logging` module exposing `Logging` on the facade, and spans on `ProjectSession::open`, `apply`,
  `export`, `analyze`, `realize`, plus the autosave and realization paths that already warn.
- `musa-compiler`: spans on `compile` and `format_document`; events at the parse / check / elaborate / adapt boundaries
  and in `imports::load`.
- `musa-render`, `musa-audio`: a span per public entry, recording the plan or graph size the caller already knows.
- `musa-engine`: spans on `open`, `install`, and `command`, recording the negotiated device and stream configuration.
  Nothing below the queue boundary.
- `musa-lsp`: a span per request carrying the method and document, and `Logging::new().install()` in `serve`.
- `musa` CLI: `-v`/`-vv`/`-vvv` and `-q` accepted before or after the subcommand, a top-level span naming the
  subcommand, `Logging` installed in `main`, and both flags plus `MUSA_LOG` documented in the usage text.
- `apps/musa-desktop/src-tauri`: `Logging` installed in `run`.
- Laws in `crates/musa-project/tests/`: the default filter admits `warn` from a musa crate and rejects `info`; `-v`
  admits `info`; `-q` rejects `warn`; `MUSA_LOG` replaces the dial and a per-target filter admits one crate and not
  another; a second `install` returns `false` rather than panicking.
- A law that no logging macro appears below `musa-engine`'s queue boundary, held the way the tree-sitter drift law is
  held: over the source of the callback path, with the real-time rule named in its failure message.
- A law that a compilation emits exactly one `compile` span naming its document, recorded through a collecting layer
  rather than by reading text off a terminal.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render -p musa-audio -p musa-engine -p musa-project -p musa-lsp -p musa
cargo nextest run -p musa-engine --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
taplo fmt --check
mdwright fmt-check
MUSA_LOG=musa_compiler=debug cargo run -q -p musa -- check examples/canon.musa
cargo run -q -p musa -- render examples/canon.musa --to lilypond -o - | head -1
```

The last two are the prompt in one line each: the first must print compiler phase events to stderr, and the second must
print LilyPond and nothing else to stdout.

Commit as `Give the pipeline a diagnostic voice`.

## Stop

- No `log` crate, no second logging facade, and no bridge between them.
- No metrics, no OpenTelemetry, no JSON log format, no file appender, no rotation. Stderr and a filter.
- No logging inside the audio callback, the render loop, or any per-note or per-sample path, at any level.
- No span or event that computes something to have something to say.
- No `tracing` in `musa-language` or `musa-kernel`: a lexer and a finite term language have nothing to report that their
  return values do not already say, and both are leaves that other crates' spans already cover.
- No `tracing` in `musa-wasm`: the browser has no stderr, and the shell adds nothing to the pipeline.
- No change to what any diagnostic says. Diagnostics are for the person writing the piece; logs are for the person
  debugging musa, and this prompt does not move a sentence from one to the other.
