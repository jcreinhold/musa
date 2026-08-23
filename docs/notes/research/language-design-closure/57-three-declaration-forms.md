# 57. Three declaration forms, and which capability each one is missing

**Status: governs nothing.** `../../../rules/language/01-surface.md` §1.2 and §1.3 hold the decisions; this page holds
the measurement behind prompt 161's repair. Written at prompt 161, before a line of it was implemented.

## 1. The matrix

`data`, `record`, and `enum` are three parsers in
`crates/musa-syntax/src/parser/declarations.rs` and three readers in `crates/musa-compiler/src/lower/items.rs`. Read off
the grammar rather than off the prose, they differ in five capabilities:

| | index telescope | chosen indices | per-variant `private` | positional fields | named fields |
| --- | --- | --- | --- | --- | --- |
| `data` | ✓ `data_indices` | ✓ `data_chosen` | ✗ | ✗ | ✓ |
| `enum` | ✗ | ✗ | ✓ `enum_case` calls `visibility` | ✓ | ✓ |
| `record` | ✗ | ✗ | ✗ | ✗ | ✓, exactly one case |

No two rows differ in the same way, which is the whole of what "the differences are not principled" means. It is not
that somebody chose three overlapping forms; it is that each grew the capability its first caller needed. `data` got
indices at prompt 156. `enum` got per-case `private` from §1.3's abstract-type rule. `record` never needed either.

## 2. `record` is a shape of `data`; `enum` is not

**Derived.** `record`'s capability set is a subset of `data`'s, so every `record` declaration has a `data` spelling
with the same meaning: one variant, named for the type, with the declaration's fields. `Lowering::structural` already
builds exactly that `RawData`.

**`enum`'s is not a subset.** Two capabilities — a `private` marker on a case, and the positional field form — have no
`data` spelling at all. So "`enum` desugars to `data`" is false today in the strong sense: there is no `data`
declaration that means what `enum Chord { private NamedChord(ChordSymbol, List<Spelling>), … }` means. Collapsing the
lowerer would leave the two capabilities exactly where they are, in an `enum`-only branch, which is the thing prompt 161
exists to remove.

That settles the direction of the fix. Either `enum` loses the two, or `data` gains them. §1.3 governs and gives `enum`
both, at length and with an argument for each: the positional form "names types and not fields", and a private case is
what makes a package's type abstract without hiding the type. So `data` gains them, which is also the direction root
`AGENTS.md` argues for on its own: a form that is the general one minus two capabilities is a second abstraction, and
every author who reaches for the general form pays for the subtraction.

## 3. The corpus, which says something else

**No `.musa` file in the repository declares an `enum`.** Not `stdlib/src/`, not `examples/`, not
`tests/fixtures/`. Twenty-nine `data` declarations and eight `record`s, and the one occurrence of the word is a comment
in `tests/fixtures/staff-dispatch.musa` explaining that the trial wrote `enum StaffWord` and the fixture spells it
`data` instead.

That is worth writing down and it is not yet an argument. Three things it could mean, and the repository cannot
currently tell them apart:

- The spelling is redundant. `data Tying { Untied, TiedOn }` is the same eleven characters as the `enum` form — the two
  keywords are the same length — so the readability claim in prompt 161's first draft ("a scale degree list written as
  `data` reads worse") is not visibly true.
- The spelling is *new*. Namespaced constructors and per-case `private` arrived with §1.3, and the two programs that
  would exercise them are the staff and studio adapters, which prompts 166 and 167 rewrite.
- The corpus is writing the workaround. That is what prompt 143 found for indices, and the same shape of evidence would
  be `data` declarations with a hand-written namespace prefix or a comment saying which cases are meant to be private.

Prompt 161's Stop keeps the keyword for the second reason, and the measurement to take after 166 and 167 is the one
that decides between the three: count the `enum` declarations in the two rewritten adapters. Zero there, after the
programs §1.3 was written for have been written, is an argument. Zero here is a schedule.

## 4. What "one declaration path" then means

One description and one builder, with three shape-normalizers over the CST. Concretely: `nominal`, `enumeration`, and
`structural` stop constructing `RawData`, `RawFamily`, and `RawConstructor` independently — three places that each
decide visibility, field naming, and index handling for themselves — and instead answer with the same description,
which one function turns into the `RawData`.

The check that it worked is not a line count. It is that the three readers can no longer *disagree*: today `nominal`
takes a constructor's visibility from the declaration because `data` admits no marker, `enumeration` takes it from the
case, and `structural` takes it from the declaration for a third reason written out separately. After the collapse
there is one rule, and each reader supplies a marker or does not.

## 5. Where a spelling can still lie

Once `data` is the union, the two conveniences are restrictions, and a restriction the *grammar* enforces produces a
parse error rather than a sentence. `enum Vect<A>(n: Nat) { … }` is refused today at the `(` with `expected \`{\``,
which tells the author that the parser wanted a brace and not that indexed families are written with `data`.

So the restriction belongs one level later: `record_decl` and `enum_decl` admit the telescope, and the lowerer refuses
it by name. One refusal per spelling, and it is the only place either spelling can be written to mean something it
cannot deliver — everything else `enum` and `record` can say, `data` can say too.
