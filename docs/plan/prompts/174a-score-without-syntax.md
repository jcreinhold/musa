---
id: 174a
slug: score-without-syntax
status: done
depends_on: [174]
phase: 3
---

# Make the Values Crate a Leaf

## Task

`musa-score` is the crate of musical *values* — "everything here is a result or a word for one" — and it depends on the
lexer and parser:

```sh
$ grep '^musa' crates/musa-score/Cargo.toml
musa-events = { workspace = true }
musa-syntax = { workspace = true }
$ grep -rn 'musa_syntax' crates/musa-score/src/ | grep -v '^.*://'
crates/musa-score/src/lib.rs:98:pub use musa_syntax::beat_groups;
crates/musa-score/src/assert.rs:571:        let rest = format!("rest{}", musa_syntax::spell_duration(&fraction(difference)));
```

Two items, and neither justifies the edge. `beat_groups` has **no caller inside `musa-score` at all** — the line is a
bare re-export, and its only consumer in the workspace is
[`crates/musa-notation/src/plan/staff.rs:145`](../../../crates/musa-notation/src/plan/staff.rs), which spells it
`musa_score::beat_groups`. The re-export exists so that a backend can reach the syntax crate without admitting that it
does. `spell_duration` is called once, to spell a filling rest inside a bar-length fix hint — a piece of *source text*,
assembled in the crate that is supposed to hold no source text.

Delete the edge. `musa-score` ends the prompt depending on `musa-events` and nothing else in the workspace.

## Read

- [`crates/musa-score/src/lib.rs`](../../../crates/musa-score/src/lib.rs)'s crate doc, which states the rule this prompt
  restores: everything in the crate is a result or a word for one.
- [`crates/musa-syntax/src/edits.rs`](../../../crates/musa-syntax/src/edits.rs)'s `spell_duration` doc comment, which
  already gives the constraint that decides where it lives: *"two spellings of one duration must not disagree."* One
  home, and the formatter and the fix hint both read it.
- [`crates/musa-score/src/assert.rs`](../../../crates/musa-score/src/assert.rs)'s `fills_meter` — the single call site,
  and the one piece of surface text this crate builds.
- [`crates/musa-notation/src/plan/staff.rs`](../../../crates/musa-notation/src/plan/staff.rs)'s beaming, the real
  consumer of `beat_groups`.
- `AGENTS.md`'s dependency sentence and roadmap §15's crate list, both of which describe the ladder this prompt makes
  true.
- *Philosophy of Software Design* ch. 7 — "wrapper only delegates" is the smell, and a wrapper whose only work is to
  launder a dependency is its worst case.

## Design

**Both functions stay in `musa-syntax`.** They are about written form: `beat_groups` says how a meter's beats are
grouped on a page, and `spell_duration` maps written text to written text. Moving them into `musa-score` would only
reverse the arrow and make the *formatter* depend on the values crate, which is worse. The bug was never their home.

**`musa-notation` takes its own `musa-syntax` dependency.** A notation backend depending on the crate that owns written
form is honest and says what it is; laundering it through `musa-score` is what this prompt deletes. The re-export at
`lib.rs:98` goes with it.

**The fix hint is assembled above `musa-score`.** `fills_meter` keeps deciding that a bar is short and by how much, and
its `Diagnostic` carries the exact missing `MusicalTime`. Spelling that as `rest/8` is a surface act, so it happens
where source text is already the subject — `musa-compiler`, which depends on both crates and is already the only place a
refusal becomes a diagnostic the author reads. The composer sees exactly the sentence and the fix they see today.

**The rule this restores, stated for 153c to enforce.** A vocabulary crate names results and depends only on theories.
This prompt is the first of two that make the workspace obey it; 153b is the second.

## Target

- `crates/musa-score/Cargo.toml`: no `musa-syntax`.
- `crates/musa-score/src/lib.rs`: the `beat_groups` re-export deleted.
- `crates/musa-score/src/assert.rs`: `fills_meter` yields the missing duration rather than the spelled rest, with its
  doc comment saying which stage spells it.
- `crates/musa-compiler`: the bar-length fix hint assembled there, from the same `spell_duration`.
- `crates/musa-notation/Cargo.toml` and `src/plan/staff.rs`: `musa_syntax::beat_groups`, named directly.
- `AGENTS.md`'s navigation and dependency sentences updated to the graph that now exists.

## Check

```sh
cargo nextest run --workspace
cargo nextest run -p musa-score -p musa-notation -p musa-compiler
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the two facts the prompt exists for:

- `grep musa-syntax crates/musa-score/Cargo.toml` is empty.
- **The diagnostic is byte-identical.** A short bar in `examples/` produces the same sentence, the same help line, the
  same fix text, and the same span as before — every `insta` snapshot and
  `crates/musa-compiler/tests/fixtures/elaboration-compatibility.txt` unchanged.

## Stop

- **Nothing moves out of `musa-syntax`.** Not `beat_groups`, not `spell_duration`, not the formatter.
- **No new crate.** Two call sites do not earn one.
- **No rewording of any diagnostic**, and no change to which spans a fix covers.
- **Nothing about the studio spec.** That edge is 153b.
- No change to `musa-score`'s public API beyond the deleted re-export, and no reshuffling of `analysis/`.
- No `musa-events` change.

Commit as `Make the values crate a leaf`.
