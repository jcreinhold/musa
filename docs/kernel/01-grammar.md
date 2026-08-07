# 01 — Kernel Interchange Grammar

This document defines the **kernel interchange syntax**: the concrete notation for the terms of
`10-term-calculus.md`. It is a semantic/interchange language for golden tests, semantic hashing, and cross-tool
exchange. **It is not the syntax musicians write** (course correction §24); the musician-facing surface language is the
`.musa` grammar handled by `musa-language`, and its elaboration is specified in `06-surface-elaboration.md`.

**Grammar here, calculus there.** This document says how a term is written; `10-term-calculus.md` says what it means,
which terms are well-formed (with `02-static-semantics.md` K7), and which theorems hold. Neither is complete without
the other, and where they disagree the calculus is right — a notation cannot promise a meaning the semantics does not
define. Both are candidate until prompt 48 graduates them together.

Implementation note: prompt 09 implemented **canonical serialization** (kernel value → this text form, in the normal
form of `05-normalization.md`) only. A parser for the full grammar arrives at prompt 48, which is the second
producer/consumer `08-open-questions.md` Q6 was waiting for; N5's output is a strict subset of this grammar, so today's
golden files are already readable by it.

## Lexical conventions

- Whitespace-separated tokens; `%` begins a line comment.
- `string-literal` — double-quoted, backslash escapes for `"` and `\`.
- `name` — `[A-Za-z_][A-Za-z0-9_-]*` (composition names, payload type names, payload field names).
- `rational-literal` — `integer-literal | integer-literal "/" positive-integer-literal`; always reduced on reading.
- `duration-literal`, `position-literal` — `rational-literal`, interpreted as beats (exact rationals; never floats).
- `positive-rational-literal` — a `rational-literal` denoting a value in `ℚ>0`; scaling by zero or a negative factor is
  a static error (`02-static-semantics.md` K2).
- Keywords are reserved: `kernel`, `payload`, `composition`, `timeline`, `occurrence`, `from`, `to`, `sequence`,
  `overlay`, `let`, `in`, `scale`, `restrict`, `shift`, `by`.

## Grammar

```ebnf
kernel-file
    = "kernel", string-literal, "{",
          { declaration },
      "}"
    ;

declaration
    = payload-type-declaration
    | composition-declaration
    ;

payload-type-declaration
    = "payload", payload-type, "{",
          { payload-field, ";"},
      "}"
    ;

payload-field
    = field-name, ":", field-type
    ;

field-type
    = "text" | "rational" | "integer" | "bool" | payload-type
    ;

composition-declaration
    = "composition", composition-name,
      ":", "Timeline", "[", payload-type, "]",
      "=",
      composition-expression,
      ";"
    ;

composition-expression
    = timeline-expression
    | sequence-expression
    | overlay-expression
    | scale-expression
    | restrict-expression
    | shift-expression
    | let-expression
    | composition-name
    | "(", composition-expression, ")"
    ;

timeline-expression
    = "timeline", duration-literal, "{",
          { occurrence-statement },
      "}"
    ;

occurrence-statement
    = "occurrence", payload-value,
      "from", position-literal,
      "to", position-literal,
      ";"
    ;

sequence-expression
    = "sequence", "{",
          composition-expression,
          ";",
          composition-expression,
          { ";", composition-expression },
      "}"
    ;

overlay-expression
    = "overlay", "{",
          composition-expression,
          ";",
          composition-expression,
          { ";", composition-expression },
      "}"
    ;

scale-expression
    = "scale", "by", positive-rational-literal, composition-expression
    ;

restrict-expression
    = "restrict", "from", position-literal, "to", position-literal, composition-expression
    ;

shift-expression
    = "shift", "by", duration-literal, composition-expression
    ;

let-expression
    = "let", composition-name, "=", composition-expression, "in", composition-expression
    ;
```

`scale`, `restrict`, `shift`, and `let` are the forms this document gained when the calculus was specified. Three notes
a reader needs:

- **`shift by d t` is sugar** for `sequence { timeline d { }; t }` (`10-term-calculus.md`). It may be written and it is
  never printed: canonical serialization emits the expansion, so N5 output stays unique.
- **`let` scopes over the expression after `in`**, and shadowing is rejected (K7). A `composition` declaration is the
  file-level form of the same idea; `let` is the expression-level one, and a file may use either.
- **There is no `map`**, deliberately: naming a payload function would require a syntax for functions
  (`10-term-calculus.md`). Payloads arrive already transformed.

## Payload values

Payload schemas are first-order and deliberately boring (course correction §24): named records over `text`, `rational`,
`integer`, `bool`, and previously-declared payload types. No functions, no sums with payloads of different shapes per
constructor, no recursion.

```ebnf
payload-value
    = payload-type, "{", { field-name, "=", field-value, ";" }, "}"
    ;

field-value
    = string-literal
    | rational-literal
    | integer-literal
    | "true" | "false"
    | payload-value
    ;
```

Example (for a toy payload; real score payloads belong to the score adapter, `07-backend-contract.md`):

```text
kernel "example" {
    payload Note {
        letter: text;
        accidental: integer;
        octave: integer;
    }

    composition melody : Timeline[Note] = sequence {
        timeline 1 {
            occurrence Note { letter = "c"; accidental = 0; octave = 4; } from 0 to 1;
        };
        timeline 2 {
            occurrence Note { letter = "e"; accidental = 0; octave = 4; } from 0 to 2;
        };
    };

    composition piece : Timeline[Note] = overlay {
        melody;
        melody;
    };
}
```

## Design rules

- **Clear names, no unexplained shorthand** (§24): `sequence`/`overlay`/`occurrence`, never `par`/`seq`/`atom`.
- Durations and positions are beats as exact rationals. `0` and `3/2` are legal; `0.75` is not.
- A `composition` reference denotes the value of its declaration; references must be acyclic (`02-static-semantics.md`
  K4), as `let`-bound names are by construction (K7).
- The normal form (every composition reduced to a single flat `timeline` with canonically ordered occurrences) and its
  serialization rules are in `05-normalization.md`. Writing is normalizing: a file *read* may share and abbreviate, a
  file *written* by this implementation never does.
- Nothing here means anything on its own. Every production above denotes through `10-term-calculus.md`, and a
  production that denoted nothing would be a syntax for a meaning the kernel does not have — which is the failure this
  split exists to prevent.
