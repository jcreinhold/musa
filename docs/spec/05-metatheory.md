# What is proved, implemented, and still open

This chapter keeps three claims separate:

- a mathematical rule may be correct;
- the Rust implementation may or may not implement it; and
- a safe mechanism may still be a poor model of a musical practice.

## 1. Reviewed results

| Result | Review status | Implementation status |
| --- | --- | --- |
| Accepted source expressions terminate under the stated foreign-operation and resource limits | proved in `docs/language/02-core-calculus.md` | implemented for the current source core |
| Source music closes to a finite kernel term | proved in `docs/language/02-core-calculus.md` §5.7 | implemented |
| Timeline sequence, unequal-length overlay, scaling, restriction, and normalization obey the kernel laws | proved in `docs/kernel/03`–`05` and `10` | implemented and tested |
| Versioned timeline bytes represent semantic equality exactly | Theorem I1 reviewed as part of K₃.3 | implemented by prompt 129a with delimiter and structured-payload tests |
| A valid whole-node audio graph has deterministic causal steps and registered feedback | Theorems P1–P3 reviewed as part of K₃.3 | not yet implemented by the current audio graph |
| Complete origin paths compose without losing intermediate anchors | theorem reviewed as part of K₃.3 | only partial provenance exists today |
| Equal complete preparation arguments give equal results, and collision-checked cache hits are sound | Theorems R1 and C1 reviewed as part of K₃.3 | full preparation API and cache are not yet implemented |

The proof-review record is in `docs/scratch/25`, `30`, `35`, `39`, `42`, and `45`. Those files include failed drafts.
The failures matter because they exposed missing assumptions now stated in this specification.

## 2. Claims this specification does not make

Nothing here proves:

- that one object captures every musical practice;
- that one pitch, metre, chord, key, or function system fits all music;
- that harmonic function is the same as scale degree;
- that a musical motif is a split idempotent;
- that hashes never collide;
- that equal musical timelines have equal origin histories;
- that equal audio plans produce bit-identical output on arbitrary devices; or
- that today’s caller-buffer-based feedback obeys the fixed-step audio rules.

## 3. The proposed source-language extension is not yet accepted

The research notes propose finite user-defined data, private constructors, abstract type members in structures, `Text`,
and `Result`. These features would let music-theory packages hide their representations while exposing total operations.

The safety proof has improved through several reviews, but review 54 still found missing rules for choosing one active
version of a stable package and for treating existing `Music` types as stable public types. The proposal remains in
`docs/scratch/`; it does not yet govern `docs/language/` or the prompt stack.

Even a complete type-safety proof would not establish musical value. The next test must implement real algorithms in at
least two differently framed theory packages. A proof can show that a ratio list is safe to store. It cannot show that
the list captures rāga, gamaka, phrasing, or any other practice well.

## 4. Evidence required from implementations

Each implementation step must test the premise on which its proof relies:

- byte encoders test empty strings, delimiters, newlines, multiplicity, version changes, and migration;
- registries reject one id paired with two exact descriptors;
- audio graphs test the known case where a port graph is acyclic but no whole-node schedule exists;
- feedback tests every partition of the same requested frames;
- caches inject a deliberate hash collision and compare complete arguments;
- preparation tests vary each option independently;
- origin tests retain generation roots, sites, and intermediate anchors; and
- source-language extensions include compile-fail tests and evaluation/normalization tests.

Passing tests for today’s code says nothing about a representation that has not been implemented. The implementation map
records that distinction.
