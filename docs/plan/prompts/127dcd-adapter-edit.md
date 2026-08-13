---
id: 127dcd
slug: adapter-edit
status: pending
depends_on: [127dcc]
phase: 3
---

# Give an Adapter the Edit Operation and Prove Its Law

## Task

`expand` is one of three declared adapter operations. Add the second: an adapter answers a structured command with
focused text edits into its own region, under a law that says what the edit may touch and what re-expanding it must
agree with. Route it so that an edit reaches a musician rather than only a test.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §4 — `edit_A`, `apply_A`, the edit law in
  full, the sentence that bytes outside the returned ranges remain unchanged, and the three conformance levels. Note
  that "reprinting an expanded value is not a safe edit" is stated there as a rule, not as advice.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` §2.6 and §3.5 — the two commands the trials must
  answer (`SetWrittenPitch`, `SetParameter`) and what each is required to leave alone.
- `crates/musa-project/src/edit.rs`: `EditCommand`, `CandidateEdit`, and how a structured score edit already becomes
  text a composer can see before it is committed. An adapter edit arrives through this door or it has no user.
- `crates/musa-lsp/src/`: how a command reaches an editor.
- `crates/musa-compiler/src/expand.rs` and `core.rs`: `expand_syntax`, the phase environment, and `Refused` — the
  machinery a second entry point reuses rather than duplicates.
- Prompt 127dcc's anchors: a command names the item it is about by anchor, which is the only name an adapter and an
  editor both have.

## Design

The operation, in the phase environment `expand` already runs in:

```text
edit : Syntax × Text × Text -> Result<List<(Nat, Nat, Text)>, Text>
```

One command, spelled as the adapter's own two texts — the command's name and its argument — because the phase is
type-blind and may not learn a package's command type. The answer is a list of replacements in *region* coordinates,
which the phase translates into the composer's file, and the error half is the adapter's own sentence exactly as a
refusal is.

**The edit law**, stated as three parts and tested as three:

1. **Locality.** Every returned range lies inside the region. The phase checks this rather than trusting it, because a
   patch that reached outside would let an adapter rewrite text no musician asked it to touch.
2. **Agreement.** Expanding the patched region gives the value applying the command to the original value gives. The
   fixture states its own semantic equality; the phase tests structural equality of the expanded syntax, which is the
   strongest thing the compiler can check without knowing the package's type.
3. **Preservation.** Bytes outside the returned ranges are unchanged — comments, layout, names, and neighbouring
   definitions are not regenerated. Tested by diffing the whole file, not the region.

An adapter that declares no `edit` is *readable* and its regions are read-only. That is the level, not a failure.

## Target

- `edit` as a second phase entry point in `crates/musa-compiler`, reusing the transformer machinery, the work meter, and
  the charges, with its refusals reported the way 127dcb reports an expansion's.
- The translation from region coordinates to file coordinates, and the locality check on the way through.
- An `EditCommand` variant in `crates/musa-project` that carries a region and an adapter command, producing
  `CandidateEdit`s the way every other structured edit does, and reachable through `musa-lsp`.
- `stdlib/src/adapters/doubled.musa` answers one command it can actually serve, so the fixture exercises the law.
- Tests: locality; agreement; preservation; a command the adapter does not know is refused with the adapter's sentence
  and changes nothing; an adapter with no `edit` reports read-only rather than broken.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Give an adapter the edit operation and prove its law`.

## Stop

- No printer, and no edit that regenerates a region from a value — §4 forbids exactly that, and prompt 127dce owns
  `print`.
- No second editable representation of a region: source stays the only editable record.
- No type-directed editing, no command type crossing the phase boundary, and no adapter reading the value its region
  produced.
- No staff or studio adapter; the fixture carries this prompt.
- No change to `docs/rules/`.
