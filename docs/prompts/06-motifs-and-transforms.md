---
id: 06
slug: motifs-and-transforms
status: done
depends_on: [05]
phase: 1
---

# Motifs and Transformations

## Task

Implement the language's compositional heart: motif declarations with pitch-defaulted parameters, `use` calls, finite
`repeat`, and `transpose` — expanded at compile time into the `ScoreSnapshot` with full provenance, and guaranteed to
terminate.

## Read

- Roadmap §5.2–§5.4 (composition laws and transformation laws), §6.2 (transformations must survive in the high-level
  model), §7.2 (finite constructs only; recursion rejected), §8.3 (abstractions must lower transparently), §9
  (`ExpansionStep`).
- Prompt 03's motif/use/transpose CST (if parsed there), prompt 05's compiler.

## Design

- Grammar surface (already parsed, or added now if prompt 03 deferred it):

  ```text
  motif sigh(root: pitch = e5) { root 1/2; rest 1/4; c5 1/2; }
  voice lead {
      use sigh();
      transpose down P5 { use sigh(); }
      repeat 4 { use sigh(); }
  }
  ```

- Interval spelling: `P5`, `M3`, `m3`, etc., with `up`/`down` direction. Define a small `Interval` type (semitones +
  diatonic steps); transpose preserves spelling (transposing `c5` down P5 yields `f4`, not `e#4` — diatonic steps drive
  the letter, semitones the accidental).
- Expansion is a private compiler pass producing expanded events whose `Origin` records each `ExpansionStep`:
  `MotifApplication { call_site }`, `RepeatIteration(u32)`, `Transposition(Interval)` (§9). This is what makes "click a
  note, see why it exists" possible later — test the expansion path contents, not just pitches.
- **Finiteness**: reject recursive or mutually recursive motifs during name resolution (build the call graph, diagnose
  cycles with the offending spans). `repeat` takes a literal positive integer. No user-controlled recursion of any kind
  (§7.2, §19).
- Motif parameters: typed (`pitch` only for now), with defaults; argument count/type mismatches are diagnostics with
  both decl and call spans.
- The high-level model (§6.2) retains `transpose down P5 { use sigh(); }` **as a transformation**; only the expanded
  snapshot flattens it. `Compilation` exposes the high-level model read-only for later provenance display.
- Property tests (§17.2) at the operation level, on generated fragments:
  - `transpose(0, x) = x`; `transpose(a, transpose(b, x)) = transpose(a + b, x)`;
  - `span(seq(x, y)) = span(x) + span(y)`;
  - `transpose(i, a then b) = transpose(i, a) then transpose(i, b)` (homomorphism, §5.4);
  - lowering law: `lower(a then b) = lower(a) then lower(b)` (§5.5).

## Target

- `musa-compiler`: motif resolution + expansion pass, `Interval`, transposition with spelling, `ExpansionStep`
  populated, recursion diagnostics.
- Restore the full `examples/glass-mountain.musa` §7.1 motif usage (without `performance`/`studio` blocks) and verify
  its expansion by snapshot.
- insta snapshot: expanded score debug rendering for a motif + transpose + repeat case, showing `Origin` expansion
  paths.

## Check

```sh
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/glass-mountain.musa
# recursive motif is rejected:
printf 'piece "x" { motif f() { use f(); } score { part p { voice v { use f(); } } } }' > /tmp/rec.musa
! cargo run -p musa -- check /tmp/rec.musa
```

Commit as `Add motifs, repeat, and transpose with provenance`.

## Stop

- No `stretch`, `retrograde`, `invert`, variation, or occurrence specialization (`use sigh() with {...}`) — prompt 34.
- No motif extraction command (prompt 25).
- No performance or notation consequences yet; this prompt ends at the snapshot.
