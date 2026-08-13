# 01 — Kernel Interchange Grammar

This document defines the **kernel interchange syntax**: the concrete notation for the terms of `10-term-calculus.md`.
It is a semantic/interchange language for golden tests, semantic comparison, and cross-tool exchange. Parsing and
evaluation produce the event track whose separately framed N6 bytes are hashed. **It is not the syntax musicians
write**; the musician-facing surface language is the `.musa` grammar handled by `musa-language`, and its elaboration is
specified in `06-surface-elaboration.md`.

**Grammar here, calculus there.** This document says how a term is written; `10-term-calculus.md` says what it means,
which terms are well-formed (with `02-static-semantics.md` K7), and which theorems hold. Neither is complete without the
other, and where they disagree the calculus is right — a notation cannot promise a meaning the semantics does not
define.

Implemented at prompt 48 in `musa-kernel/src/text.rs` (printer and parser) with `musa-compiler`'s `ScoreFact` payload
form; `musa kernel` is the CLI, and `examples/kernel/*.musa.kernel` is the committed corpus. That corpus and its second
producer/consumer are what `08-open-questions.md` Q6 was waiting for.

**Prompt 127a renames the vocabulary and raises the format version to `musa-kernel-2`.** The change is a clean break:
`musa-kernel-1` files are refused, not migrated, and the committed corpus is rewritten rather than aliased
(`../../plan/clean-break-ledger.md`). A version header exists precisely so this can be a refusal instead of a
misreading.

## Lexical conventions

- Whitespace-separated tokens; `%` begins a line comment.
- The **first line of a file is the version header**, `% ` followed by the format version (`musa-kernel-2`). It is
  lexically a comment and semantically required: a consumer must be able to refuse a format it does not know, and a file
  that merely omitted the line would otherwise read as a valid file of an unknown vintage.
- `string-literal` — double-quoted, backslash escapes for `"`, `\`, and `\n`.
- `name` — `[A-Za-z_][A-Za-z0-9_-]*` (composition names, coordinate names, payload type names).
- `rational-literal` — `integer-literal | integer-literal "/" positive-integer-literal`; always reduced on reading.
- `length-literal`, `position-literal` — `rational-literal`, interpreted in the file's coordinate (exact rationals;
  never floats).
- `positive-rational-literal` — a `rational-literal` denoting a value in `ℚ>0`; scaling by zero or a negative factor is
  a static error (`02-static-semantics.md` K2).
- Keywords are reserved: `kernel`, `composition`, `track`, `occurrence`, `from`, `to`, `follow`, `together`, `let`,
  `in`, `scale`, `restrict`, `shift`, `by`, `EventTrack`.

## Grammar

```ebnf
kernel-file
    = version-header,
      "kernel", string-literal, "{",
          composition-declaration,
      "}"
    ;

composition-declaration
    = "composition", composition-name,
      ":", "EventTrack", "[", coordinate-name, ",", payload-type, "]",
      "=",
      composition-expression,
      ";"
    ;

coordinate-name
    = "WrittenTime" | "PerformedTime" | "SecondTime"
    ;

composition-expression
    = track-expression
    | follow-expression
    | together-expression
    | scale-expression
    | restrict-expression
    | shift-expression
    | let-expression
    | composition-reference
    | "(", composition-expression, ")"
    ;

composition-reference
    = composition-name, [ "@", string-literal ]
    ;

track-expression
    = "track", length-literal, "{",
          { occurrence-statement },
      "}"
    ;

occurrence-statement
    = "occurrence", payload-value,
      "from", position-literal,
      "to", position-literal,
      ";"
    ;

payload-value
    = string-literal
    ;

follow-expression
    = "follow", "{",
          composition-expression,
          ";",
          { composition-expression, ";" },
      "}"
    ;

together-expression
    = "together", "{",
          composition-expression,
          ";",
          { composition-expression, ";" },
      "}"
    ;

scale-expression
    = "scale", "by", positive-rational-literal, composition-expression
    ;

restrict-expression
    = "restrict", "from", position-literal, "to", position-literal, composition-expression
    ;

shift-expression
    = "shift", "by", length-literal, composition-expression
    ;

let-expression
    = "let", composition-name, "=", composition-expression, "in", composition-expression
    ;
```

Five notes a reader needs:

- **The coordinate is written once, in the declaration's type**, and every track inside the composition is in that
  coordinate. There is no operation in this grammar that changes coordinate, because changing coordinate is a named
  conversion above the core (`07-backend-contract.md`), not a term.
- **`shift by d t` is sugar** for `follow { track d { }; t }` (`10-term-calculus.md`). It may be written; it is never
  printed, because the printer prints the term it is given and `Term::shift` records the sugar rather than the expansion
  only when a producer wrote it.
- **A reference may carry a mark**, `subject @ "depth 0 origin 251:263 scope voice 0 0 via motif 251:263"`. The mark is
  an opaque string; the kernel hands it to the consumer that owns the payload, which chooses a payload map from it
  (`10-term-calculus.md` T6). It is how a file both shares a body and says how each use of it differs — for musa, which
  repetition or which call site an occurrence came from, and what to substitute for the placeholders a shared body
  carries where its call site would be. Its internal shape belongs to the payload, not here: `06-surface-elaboration.md`
  specifies `ScoreFact`'s. A consumer that does not recognise a mark applies the identity, which is the correct default:
  it has read the music, and it has not read a provenance detail it does not own.
- **`let` scopes over the expression after `in`**, and shadowing is rejected (K7). A file declares exactly one
  composition, so `let` is the only sharing form a file has — which is deliberate: two file-level declarations would be
  two ways to say the same thing, and one of them would have to be canonical anyway.
- **There is no `map`**, deliberately: naming a payload function would require a syntax for functions
  (`10-term-calculus.md`). Payloads arrive already transformed.

## Payload values

**A payload is an opaque quoted string.** The kernel is generic in its payload type and never looks inside one: it reads
the string and hands it to the consumer that owns the payload — `musa-compiler` for `ScoreFact`. The `payload-type` in
the composition's type annotation exists so a reader can *refuse* a file whose payloads it does not own, not so it can
validate one it does.

This is a repair to an earlier draft of this document, which specified a `payload` declaration and record-shaped payload
values. That design would have made a kernel file self-describing at the cost of the invariant the crate exists to hold:
a kernel that reads `Note { letter = "c"; octave = 4; }` knows what a note is. The layering is worth more than the
self-description, and the payload's own text form is specified where the payload is — `06-surface-elaboration.md` for
`ScoreFact`.

```text
% musa-kernel-2
kernel "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    let subject = track 1 {
      occurrence "voice 0 0 note c4 1/4 [10:16]" from 0 to 1/4;
    } in together {
      subject;
      shift by 1/2 subject;
    };
}
```

## Design rules

- **Clear names, no unexplained shorthand**: `follow`/`together`/`occurrence`, never `par`/`seq`/`atom`.
- Lengths and positions are exact rationals in the declared coordinate. `0` and `3/2` are legal; `0.75` is not. This
  holds inside payload text too — a hairpin shape crosses as rational breakpoints, and a consumer that rounds it
  produces different sound from the same file (`07-backend-contract.md`).
- A `composition` reference denotes the value of its declaration; references must be acyclic (`02-static-semantics.md`
  K4), as `let`-bound names are by construction (K7).
- **Writing is not normalizing.** An earlier draft said it was, when the only writer was N5's serializer. Prompt 48 made
  the printer print the term it is given: a canon prints as a `let` and two `shift`s, which is the capability that makes
  this a format worth exchanging rather than a dump. Normalizing first is a separate call the caller may make, and
  `05-normalization.md` says what it produces.
- Nothing here means anything on its own. Every production above denotes through `10-term-calculus.md`, and a production
  that denoted nothing would be a syntax for a meaning the kernel does not have — which is the failure this split exists
  to prevent.
- **This grammar has no machine forms.** A machine is not written in a kernel file, because a kernel file is an exchange
  of finite exact musical facts and a machine is a description of a running source.
  `../across-stages/03-machine-calculus.md` owns machines.
