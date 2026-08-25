# The module tree

**Status: candidate.** Packages, the module tree, and what the declaration templates became.

A **package** is a directory containing `musa.toml` and a source root. Its root file `lib.musa` declares its children
with `mod`, a directory module declares its own in `mod.musa`, and module paths nest to any depth. Resolution follows
those declarations and never scans the directory: a `.musa` file under the source root that no `mod` reaches is rejected
as declared nowhere, and a `mod` naming no file is rejected as missing. The bundled standard library is one such
package, embedded at build time so that its `musa-stdlib:/std/…` URIs are readable without filesystem access, with the
embedding generated from the `mod` traversal rather than transcribed beside it.

`import "path.musa";` imports a local `library`; `import std::list;` and `import std::tonal::harmony;` import bundled
modules. Both run through the same parser, checker, evaluator, cycle detection, and name-collision rules, and both bind
into the flat value namespace — which, since prompt 162 deleted the static structure layer, is the only namespace there
is. See `docs/notes/research/60-language-decision-record.md` for the correction that introduced packages. There is no
prelude, environment search, registry, or dependency solver.

## 1. What the declaration templates became

This file once specified a second declaration layer above the core: `signature`, `structure`, `template structure`,
`make`, and `template` before a piece or a voice, with their own typing judgment, their own expansion stage, and their
own generative identity. Prompt 162 deleted all five, on the finding recorded in
`docs/notes/research/language-design-closure/58-the-module-layer-and-its-make.md`: every one of them had an ordinary
term-level spelling once the core had records, functions, and event-track values, so the layer was a second way to say
what the language already said.

| The old declaration form | What it is now |
| --- | --- |
| `signature S { let m: τ; … }` | `record S { m: τ; … }` |
| `structure X: S { let m: τ = e; … }` | `let X: S = S { m = e, … };` |
| `template structure F(A: S): T { … }` | `fn F(a: S) -> T { … }` |
| `make F(X) as Y;` | `let Y: T = F(X);` |
| `template voice V(…) { … }` with `make V(a) as n;` | `fn V(…) -> EventTrack[WrittenTime, ScoreFact] { … }` with `voice n { use V(a); }` |

The fifth word, `template piece`, reduces to no expression at all, because a piece is not a value. A file is one piece
however the piece got there, so what was reusable about a parameterized piece is its *material* — functions and bindings
any number of files may import — and what was parameterized about it is the file's own lexical root. A
`template piece study(k: Key, subject: EventTrack[WrittenTime, ScoreFact]) "Study" { … }` with one `make` site is the
piece written out, with `k` and `subject` root bindings above it. `examples/template-study.musa` is that translation
performed on the example this section used to carry.

**Identity is what the author wrote, not what a site generated.** With no expansion stage there are no generated
declarations, so there is no `GeneratedKey`, no site digest, and no rule about two instances of one template staying
distinct. A voice is identified by the voice declaration in the source and by its position among its part's voices; a
value is identified by its binding. The identity obligations that mattered — recompiling an unchanged closure yields the
same identities, and no two declarations in one accepted project collide — are now discharged by the ordinary source
addresses, because every declaration has one.

**Sealing is marking.** A structure hid a member by leaving it off a signature; nothing does that any more, and nothing
needs to. `private` (`01-surface.md` §1.3, §6.1) marks a `let`, `fn`, `record`, `enum`, or `data`, and its boundary is
the module that declares it: a marked declaration is nameable from a sibling definition in that module and from nowhere
else, including through an `import` alias and through a re-export. `stdlib/src/context.musa` is the worked example — a
`record TonalContext`, public values of it, and a private register the contexts spell from.

Visibility is a filter on the one lookup that already exists, not a second resolution path, and a private declaration is
declined *by name*: the checker knows the name exists and is hidden here, so the refusal says which module maintains it.
Building each module's scope without its private declarations would be simpler and would report "no such thing as
`NamedChord`", which sends a reader looking for a typo instead of to the interface. Two things follow and both are
checkable: a private name is still in its own module's scope, so a sibling reads it bare with no ceremony; and a private
name never read inside its own module is dead code, which the unused-declaration diagnostic already says.

The kernel implements that boundary (`crates/musa-calculus/src/kernel/visibility.rs`), and prompt 162a wired each source
file to its own `ModuleId`. A source-level `private` is therefore refused across an import while remaining visible to
sibling definitions in its declaring module.

## 2. Resolution and compilation order

For a project build closure:

1. resolve immutable local and package modules and detect static cycles;
2. type-check value definitions;
3. build score context tracks;
4. build the fragment, close the core term, and project the score;
5. independently resolve performance, instrument, and mix declarations, then prepare sound.

Earlier stages cannot query results from later stages. In particular, a function that builds material cannot branch on
an analysis result computed from the piece that material ends up in; authors instead call a finite value-level analysis
on explicit input data or write an assertion after construction.

## 3. Rejections

The following are static errors: an import cycle; first-class `piece` or source-syntax use; a structural declaration
embedded in a track value; two declarations with the same public address; naming a private declaration from outside its
module; an `enum` that marks some of its cases and not the others; and a `match` outside the module on a type whose
cases are private.

Prompt 110 implements the package and module tree. Prompt 162 deleted the declaration-template layer this file used to
specify. Prompt 191 verifies that identity and Origin remain stable through the migration.
