# The vocabulary amendment: position, duration, primitive, builtin

**Status: record.** Governs nothing. This page keeps visible the argument for the rename that followed prompt 127a, so
that a later reader who finds `Length`, `SecondTime`, or `primitive` in this directory's earlier pages knows why they no
longer match `docs/rules/`.

Documents 00–17 in this directory were written before the rename and are **not** edited to match it. They are the record
of how the calculus was chosen, and rewriting them would falsify that record. The mapping in §5 is how to read them.

## 1. What prompted it

Nothing about the calculus. The names were tested against two pieces of outside writing, in this order:

- Ousterhout, *A Philosophy of Software Design*, ch. 14 — names must be **precise** (a name broad enough to mean several
  things conveys nothing) and **consistent** (one name for one purpose, and never that name for a second purpose). Its
  worked example is a file system that used `block` for both a physical disk block and a logical block within a file,
  which cost six months of debugging.
- Hickey, *Simple Made Easy* — **complecting** is braiding together things that could be independent. Its worked example
  of an overloaded *construct* is Lisp parentheses, which "wrap calls, they wrap grouping, they wrap data structures",
  and its verdict is that such overloading is the language designer's fault and is fixable without changing the design.

Both tests found the same two defects, which is the reason to act on them rather than on either alone.

## 2. Position is not length, and the kernel already said so

`Length[C]` was one type for two quantities: *when* something happens and *how much time* it takes.

The evidence that these were already distinct everywhere except in the type:

- `docs/rules/constitution.md` §3 is titled "An event track records exact positions and lengths" — two words — and its
  body says "a nonnegative exact rational length" and, separately, occurrences "each with an exact start, end".
- `docs/rules/events/00-purpose.md`: "positions form the abelian group `(ℚ, +, 0)` and lengths the ordered commutative
  monoid `(ℚ≥0, +, 0)`". Two different algebras.
- `docs/rules/events/01-grammar.md` lexes `position-literal` and `length-literal` as separate categories.

So the braid was only in the source type grammar. The musical falsifier is that one type for both makes beat 3 and three
beats addable, which is the single arithmetic error a coordinate-tagged rational exists to catch. `Position[C]` was
added beside it, with the affine rules stated: a position plus a duration is a position, durations add, positions do
not, and the difference of two positions is a duration only when nonnegative, so it returns `Result`.

## 3. Length became duration, and this is the part that needed the amendment procedure

Splitting the type was an ordinary repair; renaming the surviving half was not, because `constitution.md` §3's title and
twelve of its sentences used the word.

Two facts settled it against `length`:

- **`length` already meant something else in the same document.** `02-core-calculus.md` §1 lists "list lengths" among
  the things counted with `nat`, and §5.6 says `map` and `filter` "decrease list length by one". That is a `nat` count
  of elements; `Length[C]` was an exact rational amount of time. One word, two behaviours, two types, one file — the
  `block` bug of §1, in the document that was being used to remove it elsewhere.
- **The surface already called this concept duration.** `01-surface.md` and `04-templates-and-modules.md` write
  `gap: Duration` for the gap between canon entries, which is an amount of written time — exactly a
  `Length[WrittenTime]`. `01-surface.md` puts both words in one sentence: "recorded duration remains seconds and is
  never manufactured into a written-time length."

`duration` is also unambiguously temporal where `length` is a spatial metaphor, it is the standard word in the wider
ecosystem (`std::time::Duration`), and it is roadmap §2's own word in "notated duration ≠ performed duration" — so
`Duration[WrittenTime]` versus `Duration[PerformedTime]` states that separation in the type system using the
separation's vocabulary.

**What `length` still means, and must keep meaning:** the number of elements in a list, and the byte count of a
variable-width field in an encoding. Those were deliberately not renamed. A spatial use in the desktop specification —
the width of a title that nobody controls — was also left alone.

## 4. Primitive is not builtin

`primitive` named a registered stepping unit *and* a compiler-owned source operation. `02-core-calculus.md` §5.8
contradicted itself about which one it meant: its family list defined "machine primitives" as the machine *constructors*
of `across-stages/03-machine-calculus.md` §2, while its privacy paragraph used the same phrase for the registered
*units* whose `State`, `start`, and `step` are private.

The boundary that resolves it was already in the code — `crates/musa-compiler/src/phase/mod.rs` declares
`const BUILTIN_OWNERSHIP: [PrimitiveOwnership<Builtin>; 8]`, which is the braid in one line — and it is an information
boundary, not a hierarchy:

- a **primitive** is a registered unit whose implementation this language does not own, so the specification states a
  contract for it (`Primitive[K,δ,δ]`, and foreign operations supplied by a host);
- a **builtin** is an operation the compiler owns and can see inside, so the specification proves things about it (the
  four families of §5.8).

Base types are called base types. `docs/rules/events/` keeps "primitive" in its ordinary English sense of *irreducible*,
where no registered unit is in scope.

## 5. Reading documents 00–17 after the rename

| In this directory | In `docs/rules/` after the amendment |
| --- | --- |
| `Length<C>` / `Length[C]` as a track's extent | `Duration<C>` / `Duration[C]` |
| `Length<C>` used for an occurrence's start or end | `Position<C>` |
| `length(M)` | `duration(M)` |
| `SecondTime` | `PhysicalTime` |
| `map_events` | `map_payloads` |
| `Scheduled<A>` | `Schedule<A>` |
| "primitive" naming a compiler-owned source operation | "builtin" |
| "machine primitive" | "registered primitive" (the unit) or "machine builtin" (the constructor), per §4 |

## 6. What was considered and refused

- **`Machine` → `Circuit`.** Refused. `State × A → State × B` with a start state is the textbook Mealy machine, so the
  current name is a term of art rather than a reach. `Machine` is also the de-complected replacement for the process
  graph, whose whole-node schedule was the mechanism by which host callback size could change what a piece sounded like;
  renaming it would obscure that it is the fix rather than the defect.
- **`Primitive` → `Device`.** Refused on evidence. `constitution.md` §4 already says "device that asks for a block of
  frames at once", `obligations.md` says "across devices", and CPAL device negotiation runs through roadmap §15. It
  would have replaced one overload with a worse one, colliding with a word the constitution already owns.
- **`beside` → `parallel`.** Refused. `03-machine-calculus.md` §2 must already warn that a mixer is a primitive rather
  than an implied meaning of parallel placement; `parallel` invites that misreading harder than `beside` does.
- **`empty(d)` → `region(d)`.** Refused. Its disclaimer — that empty tracks have duration, and duration is real — is
  about time being ambient, which is a semantic decision no name can carry.
- **Renaming `length` where it means a list count or a byte count.** Refused, and that refusal is the point of the
  rename: the reason `Duration` is the better name is precisely that `length` is needed for those.

## 7. What the amendment procedure asked, and the answers

`docs/rules/README.md` requires six things of a change to `constitution.md` or `obligations.md`.

1. **A concrete musical or engineering reason.** §2 and §3 above. The musical one is that beat 3 and three beats must
   not be addable; the engineering one is that `length` was already carrying a `nat` count in the same document.
2. **Which current examples no longer work.** None. No `.musa` example, fixture, or stored file changes meaning; the
   decisions of §3, §4, and §8 are the same decisions in different words. This step has no honest content for a rename
   and is recorded as empty rather than filled in.
3. **The replacement rule in plain language.** A track has a **duration**; an occurrence has **positions**. A registered
   unit whose insides this language does not own is a **primitive**; an operation the compiler owns is a **builtin**.
4. **The formal specification and code map updated.** Done in the same commit across `docs/rules/` and
   `docs/plan/code-map/`.
5. **How stored files and public APIs migrate.** They do not migrate; they break, on the existing ledger terms. No Rust
   identifier had moved yet when this landed, so the rename is carried by new rows in `docs/plan/clean-break-ledger.md`
   §2 and discharged by prompt 127c along with the rest of the event-track cutover. `% musa-kernel-1` and encoding
   versions 1–2 were already refusals; nothing new becomes unreadable.
6. **This record.**
