---
id: 174c
slug: dependency-law
status: completed
depends_on: [174a, 174b]
phase: 3
---

# State the Layering Law and Check It

## Task

Two dependency edges ran backwards for a long time — the values crate on the parser, the audio crate on the compiler —
and nothing caught either. `AGENTS.md` describes the ladder in a sentence of prose, and prose is not a check: it was
itself slightly wrong about which crates sit where, and a reader comparing it to `Cargo.toml` had no tool to do so. 174a
and 174b fixed the two edges. This prompt makes the third one impossible.

Write the layering down as a rule with a table of assignments, and add a check that reads every workspace member's
manifest and fails on a violation. A rule the build enforces is worth more than a paragraph in a file nobody diffs.

## Read

- 174a and 174b, whose Design sections state the rule in prose. This prompt turns those two sentences into the
  machine-checked form.
- The dependency edges as they will then stand:

  ```sh
  for f in crates/*/Cargo.toml; do
      printf '%-16s <- %s\n' "$(basename "$(dirname "$f")")" "$(grep -E '^musa[a-z-]* *=' "$f" | cut -d= -f1 | tr -d ' ' | tr '\n' ' ')"
  done
  ```

- [`scripts/check-docs.py`](../../../scripts/check-docs.py) — the model for this check. It validates every relative link
  and anchor across `docs/`, is wired into `make docs-check`, and reports every problem rather than the first.
- `crates/musa-syntax/tests/suite/tree_sitter_fixtures.rs` and the drift law — the repo's existing pattern for holding
  two artifacts to each other by test rather than by memory.
- Roadmap §15's crate list and §15.12's facade boundary, and `AGENTS.md`'s navigation table. Both are directive
  documents this prompt edits; **neither is under `docs/rules/`**, and the crate graph is not a governed commitment.
- *Philosophy of Software Design* ch. 14 on naming, for the layer names: each should say what a crate in it *is*, not
  where it sits.

## Design

**Six roles, and each crate names exactly one.** These are dependency roles, named for what a crate is rather than a
vague vertical position. A role may compose another crate in the same role — `musa-dsp` names score values in its audio
values — while Cargo itself still forbids dependency cycles.

| Layer | Depends on | Members |
| --- | --- | --- |
| theory | nothing in the workspace | `musa-syntax`, `musa-calculus`, `musa-events` |
| vocabulary | theories and vocabularies | `musa-score`, `musa-dsp` |
| pipeline | theories, vocabularies | `musa-compiler` |
| consumer | theories and vocabularies (never the pipeline) | `musa-notation`, `musa-playback` |
| session | anything below | `musa-project` |
| shell | anything below | `musa`, `musa-lsp`, `musa-wasm`, `musa-desktop` |

The rules that generate it: **a theory crate depends on no other Musa crate; a vocabulary may compose only theories and
other vocabularies; the pipeline and consumers are sibling roles, and a consumer never names the pipeline; sessions and
shells may orchestrate anything below them, but nothing depends on a shell.** This admits the deliberate direct shell
edges: the LSP reads syntax and the DSP catalogue while source is half-typed, and the wasm shell calls the compiler and
notation facade without manufacturing a project session.

**Dev-dependencies are exempt, and the check says so out loud.** `musa-notation` and `musa-playback` both take
`musa-compiler` as a dev-dependency to build a fixture from real source. That is a test reaching upward, which is
ordinary and carries no layering claim; silently allowing it would leave a reader unsure whether the check had looked.

**The check reports every violation and names the rule it broke.** Not "musa-dsp: bad dependency" but "musa-dsp is a
vocabulary crate and names musa-compiler, which is the pipeline" — the sentence a reader needs in order to decide
whether the crate or the table is wrong.

**One table, in one file, and the prose cites it.** The layer assignment lives with the script. `AGENTS.md` and roadmap
§15 point at it rather than restating it, because a second copy is a second thing obliged to agree.

**A new crate must be assigned.** A workspace member missing from the table is a failure, not a default — otherwise the
check goes quiet exactly when a crate is added, which is when it is needed.

## Target

- `scripts/check-layers.py`: the table, the role rules, per-violation messages, and the dev-dependency exemption stated
  in its module docstring.
- `Makefile`: wired into a target that CI already runs, beside `docs-check`.
- A negative control: a fixture manifest set the script rejects, so a green run proves the check can fail.
- `AGENTS.md`: the dependency paragraph replaced by the layer names and a pointer to the table.
- `docs/plan/roadmap.md` §15: the same, as the plan's own statement of the crate graph.

## Check

```sh
python3 scripts/check-layers.py
make docs-check && make fmt-check
cargo nextest run --workspace
```

And the control that makes the check meaningful: adding `musa-compiler` to `crates/musa-score/Cargo.toml`'s
`[dependencies]` makes `scripts/check-layers.py` fail, naming `musa-score` as a vocabulary crate and `musa-compiler` as
the pipeline. Revert it; the check passes.

## Stop

- **No crate moves, no code moves, no renames.** 174a and 174b did the moving; this prompt only writes down and enforces
  what they achieved.
- **Do not touch `docs/rules/`.** The crate graph is directive, not governing. If the law seems to need a governing
  statement, that is a finding to report, not an amendment to make here.
- **No compiler split.** Note for whoever reads this next: `musa-compiler` is 42,648 lines and the obvious subtraction
  does not exist — `lint`, `docs`, `factext`, and `bench` are each called from *inside* the pipeline
  (`elaborate/mod.rs`, `lower/documented.rs`, `resolve/resolver.rs`), and only `reference` is reached from `lib.rs`
  alone. A split needs its own investigation and its own prompt; guessing at one here would be the third backwards edge.
- No dependency version changes, no new external crates, no workspace-manifest restructuring.
- No enforcement of *module* layering. `musa_calculus::{kernel, elaboration}` is checked by its own law suite from 148,
  and this script reads manifests only.

Commit as `State the layering law and check it`.
