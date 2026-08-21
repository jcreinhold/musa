# Events interchange format

The events interchange syntax is the concrete text form of the event-track's terms. It exists for golden tests, semantic
hashing, and cross-tool exchange. **It is not the syntax musicians write** — the musician-facing language is `.musa`,
and [The language](language.md) describes it.

## Lexical conventions

- `%` begins a line comment. The **first line of a file is the version header**, `% musa-events-3` — lexically a
  comment, semantically required, so a consumer can refuse a format it does not know.
- Names match `[A-Za-z_][A-Za-z0-9_-]*`. Strings are double-quoted with backslash escapes for `"`, `\`, and `\n`.
- Durations and positions are exact rational literals in beats: `0` and `3/2` are legal; `0.75` is not.
- Reserved keywords: `events`, `composition`, `track`, `occurrence`, `from`, `to`, `follow`, `together`, `let`, `in`,
  `scale`, `restrict`, `shift`, `by`, `EventTrack`.
- A coordinate name — `WrittenTime`, `PerformedTime`, `PhysicalTime` — is the first type argument. It is part of what a
  file *is*: nothing converts one coordinate into another, so a reader typed in one refuses a file written in another
  rather than reinterpreting it.

## Grammar

```ebnf
events-file      = version-header, "events", string, "{", composition-decl, "}" ;
composition-decl = "composition", name, ":", "EventTrack", "[", coordinate, ",", payload-type, "]",
                   "=", expr, ";" ;

expr  = track | follow | together | scale | restrict | shift | let | reference | "(", expr, ")" ;

track = "track", duration, "{", { occurrence }, "}" ;
occurrence = "occurrence", string, "from", position, "to", position, ";" ;

follow   = "follow",   "{", expr, ";", { expr, ";" }, "}" ;
together = "together", "{", expr, ";", { expr, ";" }, "}" ;

scale    = "scale", "by", positive-rational, expr ;
restrict = "restrict", "from", position, "to", position, expr ;
shift    = "shift", "by", duration, expr ;
let      = "let", name, "=", expr, "in", expr ;
reference = name, [ "@", string ] ;
```

## Reading notes

- **`shift by d t` is sugar** for `follow { track d { }; t }`. It may be written; it is never printed.
- **A payload is an opaque quoted string.** The event track never looks inside one; it hands the string to the consumer
  that owns the payload type — for musa, `musa-compiler` and its `ScoreFact`. The type annotation exists so a reader can
  *refuse* a file whose payloads it does not own.
- **A reference may carry a mark**, `subject @ "…"`, an opaque string the payload's owner interprets. A consumer that
  does not recognize a mark applies the identity: it has read the music and skipped a provenance detail it does not own.
- **`let` is the only sharing form.** A file declares exactly one composition; shadowing is rejected, and references
  must be acyclic.
- **There is no `map`.** Naming a payload function would require a syntax for functions; payloads arrive already
  transformed.
- **Writing is not normalizing.** The printer prints the term it is given — a canon prints as a `let` and two `shift`s.
  Normalizing is a separate call; `musa events <file> --normalized` makes it.

## An example

```text
% musa-events-3
events "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    let subject = track 1 {
      occurrence "voice 0 0 note c4 1/4 [10:16]" from 0 to 1/4;
    } in together {
      subject;
      shift by 1/2 subject;
    };
}
```

The meaning of every production — the denotation, the well-formedness rules, the laws — is specified in
`docs/rules/events/` in the repository, and where that specification and this grammar disagree, the semantics is right:
a notation cannot promise a meaning the semantics does not define.
