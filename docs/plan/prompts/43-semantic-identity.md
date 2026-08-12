---
id: 43
slug: semantic-identity
status: done
depends_on: [42]
phase: 3
---

# Semantic Identity Instead of Revision Counters

## Task

The kernel computes canonical forms and semantic equality (`05-normalization.md` N4–N6) and nothing in the application
asks. `ProjectSession` decides whether to reinstall the playback plan by comparing a revision counter (`session.rs`:
`if self.installed_revision != current`), so editing a comment, reformatting, or touching an unrelated part reinstalls
the plan mid-playback. Give the kernel a semantic hash, make the session key on it, and — because prompts 39–40 made the
timeline total — make that hash mean *the whole piece*, not just its notes.

Then make canonicalization fast enough to be asked on every keystroke, using prompt 38's measurements and nothing else.

## Read

- `docs/rules/kernel/05-normalization.md` N1–N6 (normal form, payload keys, semantic equality, canonical serialization,
  and the semantic hash N6 describes but nothing computes).
- `crates/musa-project/src/session.rs` — `installed_revision`, `recompile`, `set_source`, `install_current_plan`, and
  the `ProjectUpdate::unchanged` paths.
- `crates/musa-kernel/src/occurrence.rs` — `Canonical::canonical_key(&self) -> String`, allocating one `String` per
  occurrence, called by `sort_by_key` (which may call it more than once per element) and twice per comparison in
  `semantic_eq`.
- `docs/rules/kernel/09-performance.md` — P4 is exactly this cost, measured at prompt 38 and re-measured since.
- `docs/rules/desktop/06-performance.md` B1 (keystroke → diagnostics ≤ 120 ms) and §4 (no speculative optimization).
- PoSD ch. 20: prefer the design change that removes work.

## Design

### The hash, and what it must cover

```rust
impl<A: Canonical> Timeline<A> {
    /// A stable digest of the canonical form (N6). Equal hashes mean equal
    /// canonical forms with cryptographic-collision probability; unequal
    /// hashes mean the semantics differ. Never a proxy for source text.
    pub fn semantic_hash(&self) -> SemanticHash;
}
```

Requirements, stated as invariants in the doc comment and tested:

- **Stable across runs and processes** — no `DefaultHasher`, no address-derived or hash-order-derived input. Pick a
  fixed algorithm and name it in `05-normalization.md`.
- **Agrees with `semantic_eq`**: `a.semantic_eq(b) ⟹ a.semantic_hash() == b.semantic_hash()`. A proptest, alongside the
  existing law suite.
- **Covers provenance**, because `Origin` is part of the canonical payload key today (N3) and the editor's
  source-mapping depends on it. A pure re-indentation therefore changes source spans and *will* change the hash. That is
  correct and must be documented, or the first person to try it will file it as a bug: the hash answers "is this the
  same compiled piece", not "does it sound the same". If a sounds-the-same digest is ever wanted, it is a second
  function over a provenance-free projection — not this one, and not in this prompt.

### The session keys on semantics

`installed_revision: Option<Revision>` becomes the semantic hash of the installed plan's source of truth. The playback
plan is reinstalled when the hash differs, not when the counter moved. Revisions keep doing their real job — undo/redo
history and telling the UI that the *document* changed.

This is un-complecting in Hickey's sense: "the text changed", "the meaning changed", and "the playback plan is stale"
are three facts that the counter conflated into one. Check the other places the counter is standing in for meaning —
autosave, export invalidation, `ProjectUpdate::unchanged`, and the desktop's stale-revision behaviour from prompt 21 —
and move each to whichever fact it actually wants. Some genuinely want the document counter; say which and why. Do not
convert them all reflexively.

The user-visible payoff, and the test: **playback does not restart when the edit did not change the music.** Add a
session test that plays, edits a comment, and asserts no reinstall; and one that edits a note and asserts a reinstall.

### Then, and only then, make it fast

With the session asking for a hash on every recompile, P4 moves onto B1's budget. Re-run prompt 38's benchmark and act
on what it says, in this order:

1. If P4 on the large fixture is already comfortably inside the budget, **stop and record that**. A recorded
   non-intervention is a result.
2. If it is not, the measurement will point at the `String` per occurrence. The candidate interventions, cheapest and
   least invasive first — do not apply more than the measurement justifies:
   - `sort_by_cached_key` instead of `sort_by_key`, so the key is built once per element;
   - `Canonical::write_canonical(&self, out: &mut String)`, letting callers reuse one buffer across occurrences, with
     `canonical_key` kept as the convenience built on it (this changes a public trait — state the migration in
     `05-normalization.md` N3);
   - hashing incrementally through the ordered occurrences without materializing the serialization at all.
3. Re-measure P1 and P4, small and large, and append the row. Report the numbers in "Repairs made while implementing",
   including the ones for interventions you tried and reverted.

Do not reach for parallelism, arena allocation, interning, or a faster hash function without a measurement naming it.

## Target

- `crates/musa-kernel/src/timeline.rs`, `occurrence.rs`: `SemanticHash`, `semantic_hash`, and whatever the measurement
  justifies in `Canonical`.
- `crates/musa-kernel/tests/laws.rs`: hash/equality agreement property.
- `crates/musa-compiler`: the compilation exposes its semantic hash (one accessor, one caller).
- `crates/musa-project/src/session.rs`: plan installation and any other consumer moved off the counter; new tests.
- `docs/rules/kernel/05-normalization.md`: N6 made concrete — algorithm named, invariants stated, the provenance caveat.
- `docs/rules/kernel/09-performance.md`: P1/P4 rows before and after any intervention.

## Repairs made while implementing

**The algorithm is FNV-1a 128, and the doc comment's "cryptographic-collision probability" is wrong.** The dependency
lists in roadmap §15 are closed and contain no hash crate, so adding `blake3` or `sha2` for this would have been a
dependency the roadmap did not sanction, for a property nothing needs: the caller is an editor deciding whether to
reinstall a playback plan, not a signature scheme. FNV-1a 128 is written out in `musa-kernel/src/hash.rs` (~30 lines),
is stable by construction, and is named in `05-normalization.md`. The claim recorded there is the honest one —
accident-resistant, not adversary-resistant — and the test asserts a fixed digest so a change of algorithm, basis, or
byte order fails loudly instead of silently invalidating every stored identity.

**The digest and the canonical text are the same writer.** `Timeline::write_canonical` serves both `Display` and
`semantic_hash`, with `Digest` implementing `std::fmt::Write`. Two serializations that could drift apart is the one bug
a semantic hash cannot survive, and this makes drift unrepresentable rather than tested for.

**The installed-plan key is two documents, not one.** The prompt says `installed_revision` becomes "the semantic hash of
the installed plan's source of truth". A playback plan is built from `(score, studio)` — `playback::prepare` takes both
— so keying on the piece's semantic hash alone would have made a changed instrument inaudible until the next note edit,
which the old revision counter did catch. The field is therefore `installed: Option<InstalledPlan>` holding the music's
hash *and* the `StudioSpec`, compared by value. The studio is a small declaration set with no timeline in it; digesting
it would cost more than comparing it.

**The staleness check moved into `install_current_plan`.** Two callers were asking "is the installed plan stale" two
different ways: `recompile` compared engraved MEI, `ensure_playable` compared revisions. Now one function answers it
once and returns early when the answer is no, which is what makes it *correct* for both rather than accidentally right
for each. `score_changed` keeps its real job — telling the interface to redraw — and no longer decides playback.

**`ValidArtifacts::revision` stays.** It is what `ProjectSnapshot::score_revision` reports: "the score you are looking
at came from revision N, and you are editing N+3". That is a fact about the *document*, and the prompt's instruction not
to convert consumers reflexively applies to it exactly.

**"Editing a comment does not reinstall" had to be tested with a trailing comment.** The identity covers provenance, as
the prompt requires; inserting a comment *above* the notes moves every source span and therefore does change the hash.
The test appends the comment after the piece, which is the honest form of "the text changed and the music did not". This
is the caveat the prompt predicted someone would file as a bug, met on the first test written against it.

**The tests live in `session.rs`, not `tests/`.** Installing a plan needs a sound card, so the observable is the key
`install_current_plan` compares, not the engine — a private `plan_identity()` that both the installer and the tests
call. Testing it from outside would have meant a new public accessor whose only caller is a test, which the Stop section
forbids. Three tests: a comment does not change it, a note does, and a studio-only edit does.

**The measurement, and the one intervention it justified.** Full numbers and reasoning are in
`docs/rules/kernel/09-performance.md`; a fifth benchmark, **P5**, was added because P4 no longer measures what the
session asks for.

- P1 large **1.41 ms → 1.95 ms (+38%)** and P2 large **+33%** — over the block's 10% gate, declared here. Hashing the
  piece is what `compile` now does that it did not before, and no design removes that work while still answering the
  question. Against B1's 120 ms keystroke budget it is 1.6%.
- P4 large **1.67 ms → 389 µs (−77%)**, allocations **30 934 → 9 457 (−69%)**, from `sort_by_cached_key` in
  `canonical_occurrences`: `sort_by_key` was rebuilding a `String` key on every comparison. This is the first candidate
  on the prompt's list and the only one applied.
- Not applied, and recorded so they are not re-derived: `Canonical::write_canonical` with a shared buffer, and hashing
  without materializing keys. Both change a public trait to buy time nothing is short of.

## Check

```sh
cargo nextest run -p musa-kernel -p musa-compiler -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo bench -p musa-compiler
grep -n "installed_revision" crates/musa-project/src/session.rs | wc -l   # 0
```

Commit as `Key playback installation on semantic identity`.

## Stop

- No incremental or query-based compiler (Salsa). `06-performance.md` §4 makes that its own prompt with its own
  measurement; a hash is not a cache.
- No content-addressed store, no on-disk cache keyed by the hash, no export memoization. One caller, this prompt.
- No second "musical" hash that ignores provenance until something needs it.
- No optimization the benchmark did not ask for, however obvious it looks.
