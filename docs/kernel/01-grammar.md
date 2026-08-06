# 01 — Kernel Interchange Grammar

> **Status: candidate** — provisional until prompts 10–11 pass; see `00-purpose.md`.

This document defines the **kernel interchange syntax**: a textual form for finite temporal-kernel compositions. It
is a semantic/interchange language for golden tests, semantic hashing, and cross-tool exchange. **It is not the syntax
musicians write** (course correction §24); the musician-facing surface language is the `.musa` grammar handled by
`musa-language`, and its elaboration is specified in `06-surface-elaboration.md`.

Implementation note: prompt 09 implements **canonical serialization** (kernel value → this text form, in the normal
form of `05-normalization.md`) only. A parser for the full grammar — including un-normalized `sequence`/`overlay`
expressions and named composition references — is deliberately deferred and tracked in `08-open-questions.md`. Nothing
in the current pipeline needs to read kernel text back; everything needs to write it deterministically.

## Lexical conventions

- Whitespace-separated tokens; `%` begins a line comment.
- `string-literal` — double-quoted, backslash escapes for `"` and `\`.
- `name` — `[A-Za-z_][A-Za-z0-9_-]*` (composition names, payload type names, payload field names).
- `rational-literal` — `integer-literal | integer-literal "/" positive-integer-literal`; always reduced on reading.
- `duration-literal`, `position-literal` — `rational-literal`, interpreted as beats (exact rationals; never floats).
- Keywords are reserved: `kernel`, `payload`, `composition`, `timeline`, `occurrence`, `from`, `to`, `sequence`,
  `overlay`.

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
```

## Payload values

Payload schemas are first-order and deliberately boring (course correction §24): named records over `text`,
`rational`, `integer`, `bool`, and previously-declared payload types. No functions, no sums with payloads of different
shapes per constructor, no recursion.

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
- A `composition` reference denotes the value of its declaration; references must be acyclic
  (`02-static-semantics.md`).
- The normal form (every composition reduced to a single flat `timeline` with canonically ordered occurrences) and its
  serialization rules are in `05-normalization.md`.
