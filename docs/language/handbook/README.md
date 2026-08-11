# The Musa handbook

Two ways in, one language.

The **musician's path** starts from something you want to write down and stops as soon as the piece will play. It
assumes you read music and nothing else. The **implementor's path** starts from the grammar and follows one note all the
way to a rendered page, and it assumes you will be changing the compiler.

Neither is a summary of the other. They describe the same language from the two ends a reader can be standing at, and
where one needs a fact the other owns, it links to it rather than restating it.

| Read this | If you want |
| --- | --- |
| [01 — First pieces](01-first-pieces.md) | notes, bars, voices, parts, and reusable phrases |
| [02 — Keys, degrees, and chords](02-keys-degrees-and-chords.md) | scale degrees, collections, chord classes, voicings |
| [03 — When you need a name](03-when-you-need-a-name.md) | functions, templates, structures, and what each is for |
| [04 — Claims and readings](04-claims-and-readings.md) | assertions, analyses, and the kernel escape hatch |
| [05 — Cookbook](05-cookbook.md) | a musical task, and the shortest way to it |
| [06 — What Musa refuses to blur](06-distinctions.md) | the distinctions the type system keeps, and why |
| [07 — Implementor's reference](07-implementor.md) | grammar to kernel, laws, ownership, extension points |
| [08 — Where the theory comes from](08-citations.md) | every theoretical claim, and the chapter or proof behind it |

The standard library is not written out here. Every operation it publishes is generated into
[`stdlib/reference.md`](../../../stdlib/reference.md) from the compiler's own record of the declaration — the same
record the editor shows on hover — so a signature exists in exactly one place and cannot drift from the source. What
this handbook adds is which operation to reach for and why.

## What is checked

`./scripts/check-language-docs.sh` proves four things, and it runs in CI with the rest:

- the generated standard-library reference is what the compiler would write today, and every name it publishes carries a
  sentence saying what it is for;
- every fenced `musa` block in this directory appears, line for line, in a file under `examples/` or `stdlib/src/`, so
  nothing here is syntax that the shipped parser would refuse;
- every link inside `docs/language/` lands, including its `#anchor`;
- every Open Music Theory chapter cited by filename exists, and every bundled module is named somewhere here.

The examples are therefore not illustrations. They are the fixtures, quoted, and the file each one comes from is named
beside it so you can open the whole piece.

## What this handbook is not

It is not the specification. The specification is the rest of [`docs/language/`](../README.md): the surface grammar, the
total core calculus and its metatheory, the musical domains and their proofs, the module judgments, the verification
contract, and the analysis boundary. The handbook teaches; the specification decides. Where they disagree, the
specification is right and the handbook has a bug.

It is not a music-theory textbook either. Musa implements a particular, bounded set of theoretical constructions and
says where each came from ([08](08-citations.md)). It has nothing to say about the enormous amount of music those
constructions do not describe, and the fact that a convention is built in is a fact about this software, not about
music.

Sound, instruments, the studio, packages, and recorded media are specified in
[`../08-performance-and-sound.md`](../08-performance-and-sound.md) and
[`../09-assets-and-packages.md`](../09-assets-and-packages.md), and are not taught here: prompts 130–142 build them, and
prompt 143 writes their half of the handbook against what was actually built.
