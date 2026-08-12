---
id: 116
slug: explicit-theory-assertions
status: done
depends_on: [101, 102, 107]
phase: 3
---

# Explicit Musical Assertions

## Task

Let a composer ask the compiler to prove an objective musical claim about a particular contextual passage: bar extent,
scale membership, chord-symbol/chord-class realization, or voice-count/range. An assertion is a checked identity on
`music`: success preserves facts and extent and adds provenance; failure returns a teaching diagnostic. Nothing becomes
a global style rule merely because it can be checked locally.

## Read

- `docs/rules/language/05-verification.md` and the three guarantee strengths.
- OMT `013`–`016` for scale-degree membership, `017`–`019` and `075` for chord spelling/content/inversion, and
  `022-chords-in-satb-style.md` for the explicit range/spacing claims offered here. Cite the exact definition each
  assertion checks.
- Prompt 57's named-bar assertion and prompt 83's lint boundary: errors prove declared requirements; lints advise about
  compiling source.

## Design

Use the assertion syntax fixed by `docs/rules/language/01-surface.md`. Its predicate is one of a typed, documented
family over a coherent private view—not an arbitrary `ScoreFact` callback or user reflection. Evaluate after contextual
instantiation so ambient scale, absolute placement, and exact spans are real. A successful assertion returns the same
kernel facts/extent under `≈facts`, adding an `Assertion` Origin step only; a failure names the claim, smallest witness,
expected domain, and source spans for assertion and offending material.

Initial assertions:

- `fills_meter`/existing named-bar law at every use site;
- `pitches_in(scale)` over sounded written pitches, with chromatic alterations reported rather than respelled;
- `realizes(chord_class, policy)` with explicit policy for omissions/doublings/non-chord tones;
- `voices(count)` and `within_ranges(ranges)` for a selected voicing/passage.

`realizes` takes a **chord class**, and not also a chord symbol, though this prompt was drafted asking for both.
`crates/musa-compiler/src/harmony.rs` opens by saying a chord symbol is "recorded, never interpreted": nothing in musa
derives notes from one or checks that the notes under it agree with it, and later theory libraries are algorithms over
that model rather than parts of it. Deriving members from a `ChordSymbol` here would be the compiler doing the deriving,
which is the sentence that module exists to keep true. There is also no `ChordSymbol` value in the elaboration language
and no expression that produces one — `harmony "fmaj7";` is a statement — so the second spelling would have to be a
string argument in a language whose whole argument for itself is that its musical values are typed. `chord c major7` is
how a chord class is written, it is what the assertion needs, and one spelling is enough. What OMT `075` contributes is
its *content* rule, and that rule reaches the checker through the class.

The chord realization policy is a typed parameter so strict pitch-set equality, subset-with-doubling, and allowed
non-chord-tone modes are different claims. Its type is owned by the predicate registry rather than added to the
elaboration language: three inhabitants no function can take or return would be language surface with no caller, which
`AGENTS.md` forbids in the same breath as deep modules. Assertions may wrap generated or handwritten music. A raw local
kernel quote in prompt 121 intentionally does not inherit surface assertions unless the assertion is outside the quote.

## Target

- Assertion syntax/formatting/tree-sitter, private checker registry, stable diagnostics/explanations, Origin step.
- Reuse/refactor the named-bar check through the same internal obligation mechanism without changing its messages.
- `examples/theory-assertions.musa` and focused broken fixtures for every claim/policy.
- `crates/musa-compiler/tests/assertion_laws.rs`: identity-on-success, exact failure witnesses, per-context rechecking,
  provenance, handwritten/generated parity, and absence of global out-of-key/chord warnings.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/theory-assertions.musa
cd editors/tree-sitter-musa && tree-sitter test
```

Commit as `Add explicit music-theory assertions`.

## Stop

- No global key-membership, chord-fit, SATB, counterpoint, or cadence enforcement.
- No automatic repair, respelling, reharmonization, or source rewrite.
- No arbitrary user predicate over hidden music/facts.
- No interpretive key/Roman/modulation result—prompts 119–118.
