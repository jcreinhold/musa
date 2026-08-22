# Surface language candidate

**Status: candidate.** The punctuation and spellings of the source language.

This file settles the punctuation and spellings introduced by the candidate. Existing syntax remains unless a rule below
explicitly desugars it. Braces delimit blocks. Added bindings, calls-as-statements, and declarations end in `;`; commas
separate arguments; `=` introduces an expression body or binding. Existing note, rest, and chord events remain
self-delimiting and do not take `;`. No added production is newline-sensitive.

## 1. Added grammar

The normative schematic grammar is:

```ebnf
type         := type-name type-args? | "(" type ")" | "(" type "," type ("," type)* ")"
              | fn-type
fn-type      := type "->" type | "(" (type ("," type)*)? ")" "->" type
type-name    := (module-path "::")? IDENT
type-args    := "<" type ("," type)* ">" index-args?
type-params  := "<" type-param ("," type-param)* ">"
type-param   := IDENT | "{" IDENT (":" type)? "}"          % braces mark an inferred parameter
index-params := "(" IDENT ":" type ("," IDENT ":" type)* ")"
index-args   := "(" expr ("," expr)* ")"                    % a type applied to index arguments
where-clause := "where" constraint ("," constraint)*
constraint   := type-name type-args
visibility   := "private"
binding      := visibility? "let" IDENT (":" type)? "=" expr ";"
function     := visibility? "fn" IDENT type-params? "(" params? ")" "->" type where-clause? block
param        := IDENT (":" type)? ("=" expr)?
data         := visibility? "data" IDENT type-params? index-params? "{" data-case ("," data-case)* ","? "}"
data-case    := visibility? IDENT ("(" params? ")")? (":" type)?     % the result type names the indices it chooses
record       := visibility? "record" IDENT type-params? where-clause? "{" field-decl* "}"
field-decl   := IDENT ":" type ";"
enum         := visibility? "enum" IDENT type-params? where-clause? "{" (enum-case ("," enum-case)* ","?)? "}"
enum-case    := visibility? IDENT ("(" type ("," type)* ")" | "{" field-decl* "}")?
trait        := visibility? "trait" IDENT type-params where-clause? "{" trait-item* "}"
trait-item   := "fn" IDENT type-params? "(" params? ")" "->" type where-clause? (";" | block)
impl         := visibility? "impl" type-params? constraint where-clause? "{" function* "}"
inherent     := "impl" type-params? type-name type-args? "{" function* "}"
call         := expr "(" args? ")"
args         := arg ("," arg)*
arg          := expr | "_" | "{" IDENT "=" expr "}"          % a named inferred argument
expr         := literal | IDENT | qualified | "(" expr ")" | block | product | list
              | call | projection | method-call | index | operation | question
              | match | conditional | record-literal | record-update | music-expr
block        := "{" expr "}"
product      := "(" expr "," expr ("," expr)* ")"
list         := "[" (expr ("," expr)*)? "]"
qualified    := type-name "::" IDENT
projection   := expr "." IDENT
method-call  := expr "." IDENT "(" args? ")"
index        := expr "[" expr "]"
operation    := expr binary-op expr
binary-op    := "==" | "<" | "+" | "-" | "*" | "/"
record-literal := (type-name | qualified) "{" field-init ("," field-init)* ","? "}"
field-init   := IDENT "=" expr
match        := "match" expr "{" match-arm ("," match-arm)* ","? "}"
match-arm    := pattern "->" expr
conditional  := "if" expr block "else" (block | conditional)
record-update := expr "with" "{" field-update ("," field-update)* ","? "}"
field-update := field-path "=" expr
field-path   := IDENT ("." IDENT)*
question     := expr "?"
pattern      := "_" | literal | IDENT | constructor-pattern | record-pattern
              | "[" "]" | "[" pattern "," ".." IDENT "]"
              | "(" pattern "," pattern ("," pattern)* ")"
constructor-pattern := (type-name "::")? IDENT ("(" pattern ("," pattern)* ")")?
record-pattern := (type-name | qualified) "{" field-pattern ("," field-pattern)* ","? "}"
field-pattern := IDENT ("=" pattern)?
music-expr   := "music" "{" music-statement* "}"
music-use    := "use" expr ";"
scale-local  := "in" "scale" expr "{" music-statement* "}"
assertion    := "assert" IDENT "(" args? ")" "{" music-statement* "}"
analysis     := "analysis" IDENT "=" expr ";"
events-quote := "events" "EventTrack" "[" "WrittenTime" "," "ScoreFact" "]" "{" events-item* "}"
antiquote    := "${" expr "}"
document     := (import | binding | function | data | record | enum | trait | impl | inherent
                | signature | structure | template | instance)*
                (piece | library | instance)
signature    := "signature" IDENT "{" member* "}"
member       := "let" IDENT ":" type ";"
structure    := visibility? "structure" IDENT params? ":" IDENT "{" (binding | function)* "}"
template     := "template" decl-kind IDENT "(" params? ")" decl-body
instance     := "make" IDENT "(" args? ")" "as" IDENT ";"
path         := IDENT "." IDENT
sound-bind   := "sound" expr "using" expr ";"
instrument   := "instrument" IDENT ("from" STRING)? "conforms" path
                (";" | "{" instrument-item* "}")
instrument-item := control-decl | implementation
control-decl := "control" path ":" control-domain ("in" range)? "=" quantity ";"
profile      := "profile" IDENT "for" path "{" profile-rule* "}"
profile-rule := notation-selector "->" control-assignment ";"
clip         := "clip" IDENT "from" STRING "fit" duration "by" ("rate" | "loop" | "crop") ";"
fixed-media  := "fixed_media" IDENT "from" STRING ";"
cue          := "cue" IDENT "at" position ("repeat" NAT)? ";"
room         := "room" IDENT "{" room-setting* "}"
fallback     := "unsupported" "technique" IDENT "->" "notation_only" "warning" ";"
import       := "import" (STRING | module-path) ("as" IDENT)? ";"
module-path  := IDENT ("::" IDENT)*
mod-decl     := "mod" IDENT ";"
module-file  := mod-decl*
```

Function arrows associate right, and a function type may name more than one parameter. `(B, A) -> B` is a function of
two arguments, not a function of one pair: a parenthesized type list immediately followed by `->` is a parameter list,
and that reading wins over the product reading, which is only reachable where no arrow follows. `A -> B` remains the
one-parameter shorthand, and a function *over a pair* is written `((A, B)) -> C`. This is not a convenience. A call is
complete (`02-core-calculus.md` §1.3), so a two-argument step function has a two-parameter type and there has to be a
way to write it down; §1.6's own `fold_from_start` declaration needs one, and note 40 §2 recorded the same gap for the
recursor's four-argument branches.

Expression forms bind as follows, tightest first:

| Level | Forms | Associativity |
| --- | --- | --- |
| 1 | `e(…)`, `e.f`, `e.m(…)`, `e[i]`, `e?`, `e with { … }`, `T::x`, and every brace-delimited form | left |
| 2 | `*`, `/` | left |
| 3 | `+`, `-` | left |
| 4 | `step` | left |
| 5 | `up`, `down` | none |
| 6 | `==`, `<` | none |

A brace-delimited form — a block, `match`, `if`, `music`, a record literal — is a primary expression at level 1, so it
never needs parentheses to be an operand. Arithmetic binds tighter than the pitch operators, which is the reading a
musician wants: `c4 up M3 + P5` transposes by the sum of two intervals rather than adding a pitch to an interval, and
the second reading is not well typed anyway. The two non-associative levels reject a chain rather than silently choosing
one: `root up M2 down m2` is rejected as ambiguous; write parentheses. So is `a == b == c`, which in a language whose
`==` answers `Bool` would otherwise compare a boolean with `c`. `up` and `down` take a `Pitch` or a `NoteName` and
return whichever they were given, so `c4 up M3` is a pitch and `chord_root(triad) up M3` is a pitch class: the operand's
own type decides, and no register is invented for a value that never had one. Every `fn` has an expression body, written
in braces: `{ e }` is a block, it holds exactly one expression, and it means that expression — `⟦{ e }⟧ = ⟦e⟧`
(`02-core-calculus.md` §2). A block is an expression form wherever an expression is admitted, not a special case of
`fn`. There is no statement language inside it: no `let`, no `return`, no `;`-separated sequence, and a second
expression in a block is a static error naming the rule. A multi-statement musical body is explicitly `music { ... }`,
which is a different construct that happens to abut the body's brace.

Named intervals use conventional `P`, `M`, `m`, and repeated `A`/`d` qualities. Because lowercase `d4` already means the
written pitch D4, a singly diminished fourth is written `dim4`; `dd4` and `ddd4` remain the compact multiply diminished
spellings.

`match` is the sole added case-analysis spelling. Arms are comma-separated and a final comma is accepted; braces and
arrows keep the alternatives legible when an arm's expression spans lines. Patterns cover booleans, naturals and other
literal domains, empty/cons lists, products, enum constructors, and record fields. A bare identifier binds the whole
value; `_` discards it. Prompt 96 defines exhaustiveness and rejects duplicate or unreachable arms. It also owns the
constructor meaning of `[head, ..tail]`; `..` is two adjacent `.` tokens, not a new general range operator.

**Patterns nest.** A sub-position holds another pattern rather than only a binder, and `02-core-calculus.md` §6.2 is
where that is decided: a `match` compiles through a case tree to the generated eliminators, and coverage is decided
there. This replaces the earlier depth-one rule, whose whole argument was that the case-tree compiler had not earned its
place; §6.2 states what changed and why. There are still no guards, no conditional equations, and no pattern on the left
of a definition.

`if condition { consequent } else { alternative }` is one expression and not a statement. The condition has type `Bool`,
the two branches have one type between them, and that type is the conditional's. The `else` is mandatory: a one-armed
conditional would need a value for the case it does not cover, and this language has neither a unit value in expression
position nor an implicit failure. A ladder is written by putting another conditional after `else`, which nests in the
alternative rather than adding a third keyword.

It adds no term to the calculus. `if c { a } else { b }` elaborates to `match c { true -> a, false -> b }`, the
exhaustive two-arm boolean match the surface already had, so it costs what that match costs and every rule about
matching applies to it unchanged — `02-core-calculus.md` §1. What it buys is the reading: a value decided by a yes-or-no
question is written as a yes-or-no question rather than as case analysis on a two-valued type, and a run of them is a
ladder rather than a staircase of nested braces. Guards on match arms would flatten the same staircase and are refused
separately; a guard reintroduces the fall-through between equations that a case tree exists to eliminate
(`02-core-calculus.md` §6.2), so a guard proposal has to earn its own change.

`subject with { path = expr, ... }` rebuilds a record: the result is the subject's value with the named fields replaced
and every other field carried over unchanged. Six things are fixed about it.

- The subject is evaluated exactly once, however many fields are carried over.
- Each written right-hand side is evaluated exactly once, and against the scope *around* the update rather than against
  the subject's fields. `p with { n = plus(n, 1) }` reads the `n` in scope where it is written, not `p`'s field of that
  name; an author who means the field writes the projection or the match that reads it. That reading is decidable by
  looking at one line, which is the whole reason for the rule.
- **A left-hand side is a path**, so `p with { region.anchor = a }` reaches through a field into the record it holds.
  Every segment must name a field of the record its prefix denotes, and every proper prefix must therefore denote a
  record. A path costs one construction per segment and nothing else: no segment's siblings are re-evaluated, and no
  intermediate value is built twice.
- The subject's type — and every proper prefix's — must be a `record` declaration (§1.2). A value that could be one of
  several cases is taken apart with `match`, which names the case, and rebuilt inside the arm.
- Two paths where one is a prefix of the other are refused, and so is the same path twice; the diagnostic points at both
  mentions. `p with { r = x, r.a = y }` has two readings that differ, and nothing about the spelling says which one the
  author meant.
- The result has the subject's own type. An update never widens, narrows, or changes what a value is.

It adds no term to the calculus. `p with { f = e }` elaborates to a `let` binding the subject and one record literal
whose other fields are projections of that binding — `02-core-calculus.md` §2. A path nests the same rule: `p with { a.b
= e }` is `p with { a = (p.a with { b = e }) }` with the subject bound once and read twice, which is why the cost is one
construction per segment. The subject is bound rather than repeated, which is what evaluates it once; exactly one record
is built at each level, which is what makes an update cost what it looks like; and the binder the elaboration introduces
cannot be written in source, which is what makes the second rule above true by construction rather than by renaming.
Right-hand sides are *checked* where they are written, so a diagnostic points at the line the author wrote, and
*evaluated* in the declaration's field order, which is already the order a record literal written with named fields
evaluates in. Nothing observes the difference — the language is total and its expressions have no effects — and fixing
evaluation to one order across both spellings is what keeps `p with { f = e }` and the full literal the same program.

`with` is the same word the `use` statement spells its occurrence overrides with, and the statement keeps it: in
`use theme() with { note 3 = a5; }` the `with` belongs to the statement, because that is where a reader's eye already
puts it. Only the top level of a `use` value is affected; an update written inside an argument is an ordinary update.

`e?` carries a failure outward. Where `e : Result<A, E>`, the whole expression has type `A` and denotes what `e`
succeeded with; where `e` fails, the answer around the `?` is that same `Err` value. Four things are fixed about it.

- `e` is evaluated exactly once, however much of the answer is written around the `?`.
- The error types are identical. There is no conversion, widening, or coercion: a `Result<A, E₁>` asked inside an answer
  of `Result<B, E₂>` is refused, naming both. A function's written type is supposed to say which failures come out of
  it, and a silent conversion would make that sentence untrue. A caller with a different failure matches on it and says
  what it means here.
- The answer the `?` leaves is the enclosing *function*'s. Written directly in a function's body, or in a branch whose
  value is that body's value, `?` is what it says. Written where the value is not the function's answer — inside an
  argument, say, so that a match wrapped around it would answer the call rather than the caller — it is refused, because
  there is no statement in this language to return from and inventing one would be a second way for a value to leave.
  That branch writes the `match` it means.
- The answer's type has to be a `Result`, and it is the type the enclosing function was *checked* against. A public
  signature is written, so in ordinary code that type is already known when the `?` is reached, and `?` is refused only
  when it turns out not to be a `Result` — the diagnostic naming the type it turned out to be. Where the answer's type
  is not yet decided when the `?` is reached, the `?` is refused there rather than waiting: what `02-core-calculus.md`
  §2.1 postpones is a *comparison*, and "is this a `Result`" is a question about the type's head rather than a
  comparison, so there is nothing to wait for. The refusal names the signature to write.

`?` is for `Result` and nothing else. An `Option` says only that a value is missing, not why, so there is no failure for
`?` to carry; a caller that wants propagation matches and says what the absence means. There is no `Try`, no `Monad`, no
`do`, and no `bind`: one constructor propagating is evidence for one operation, not for abstracting over which
constructor it is.

It adds no term to the calculus. `C[e?]`, where `C` is the answer written around it, elaborates to
`match e { Ok(x) -> C[x], Err(y) -> Err(y) }` — the exhaustive two-arm `Result` match the surface already had —
`02-core-calculus.md` §2. `e` is the scrutinee, which is what evaluates it once; `x` is unspellable, so nothing an
author writes can capture it; and several questions in one answer nest in the order they are written, so a program with
more than one thing wrong with it reports the leftmost.

Structural folds do not add syntax. `nat_fold(zero, step, count)`, `list_fold_from_start(zero, step, values)`, and
`list_fold_from_end(zero, step, values)` are ordinary calls to compiler-owned total builtins. A step argument is a named
function or an anonymous one, whichever reads better at the call site. This gives musicians one call notation to learn
and leaves `repeat n { body }` as the notation-facing fold over musical material. `option_fold` is gone: `Option` is an
ordinary enum (§1.3), so the way to eliminate one is `match`.

A list has two folds because Musa has demonstrated uses for both directional readings, and the direction is in the name
rather than in the type: both have the identical signature, so a reader comparing two calls compares only the word that
differs. `list_fold_from_start` accumulates left to right; `list_fold_from_end` is the catamorphism, and it is what
reads a region into right-nested data without a closure chain. This does not claim that no other finite structure admits
an order-sensitive traversal. `Nat` and generated data folds keep one canonical eliminator because no second primitive
for them has earned admission. The two list folds become the `Iterable` methods `fold_from_start` and `fold_from_end` in
§1.6, which changes the notation and not the count: two directions, two names, one signature.

The value types added here are `Bool`, `Nat`, `Ratio`, `Duration`, `Pitch`, `Interval`, `NoteName`, `Pc12`, `Scale`,
`Key`, `Degree`, `ChordClass`, `Triad`, `Roman`, `Voicing`, `Row12`, `Analysis<A>`, and `EventTrack[C, A]`. Products,
lists, and arrows are the constructors described in `02-core-calculus.md`; `Option<A>` and `Result<A, E>` are enums
declared in `std` rather than grammar (§1.3). A `record`, `enum`, or `trait` declaration adds a type of its own, so this
list is no longer closed by the compiler. Declaration kinds are not types. Every type is spelled with a capital and
every music statement keyword is not, which is what lets `key c major;` set a key and `Key` name the type of what it set
without either word looking the other up (prompt 113). Six of these words — `pitch`, `music`, `scale`, `key`, `degree`,
`frame` — are *also* music statement keywords, and one word doing two jobs in two grammars is a collision a parser can
only paper over; a capital settles it in the lexer. `NoteName` is the letter and accidental as written, with no octave:
a pitch class is octave *and* enharmonic equivalence (Open Music Theory 99), so a type in which C♯ and D♭ differ is a
name rather than a class, and `Pc12` is the class it names.

**A type parameter is angle-bracketed, and now users write them.** `record`, `enum`, `trait`, `impl`, and `fn` all take
`<A, B>`, and a type is applied as `List<Pitch>` or `Vec<A, n>`. That is a real change: the previous rule said `Option`,
`List`, and `Analysis` were the only parameterized types and there was no user-written type application at all. The
argument for `<>` survives the change intact, and the ambiguity that makes it expensive elsewhere still cannot arise:

- **`<>` belongs to the type grammar and `[]` to the term grammar**, and the parser always knows which one it is in. So
  `[` is free to serve the list literal `[c4, d4]`, the list pattern `[x, ..xs]`, and indexing `xs[i]` — three spellings
  of one idea — without ever appearing in a type.
- **There is no term-level type application.** A type parameter is solved by elaboration (`02-core-calculus.md` §2.1),
  never written, so a `<` in term position is always the comparison operator and `f<a>(b)` has exactly one reading. This
  is the rule that keeps `<>` cheap, and it is why the language admits type parameters without admitting the ambiguity
  they usually bring.

Four former spellings are **hard errors carrying an applicable fix**, on the same precedent as `use` in import position
and for the same reason — a language that accepts both spellings has a mixed corpus forever, and the fix machinery makes
one spelling affordable:

| Former spelling | Current spelling | Rule |
| --- | --- | --- |
| `fn f(x: τ) -> υ = e;` | `fn f(x: τ) -> υ { e }` | a function body is a block expression |
| a lowercase type name, and `pitchclass` | `UpperCamelCase`, and `NoteName` | a type is spelled with a capital |
| `option[τ]`, `list[τ]` | `Option<τ>`, `List<τ>` | a type parameter is angle-bracketed |
| `data D { … }` | `record D { … }`, `enum D { … }` | a product and a sum are read differently and get different words |

The last row is the only one whose fix is not mechanical: the tool offers `record` when the declaration has one case and
`enum` when it has several, and a one-case declaration an author wants to stay nominal keeps `enum` (§1.2 says when that
matters). `data` said neither thing, which is how `Pending` came to be an eight-field product spelled as a sum.

The core literals introduced here are `true`, `false`, nonnegative decimal naturals, exact rational literals, products,
and finite lists. Existing pitch and interval literals are also expression atoms. `Some` and `None` are now enum
constructors rather than literal syntax, and read identically. Strings and floating-point values remain syntax of their
owning declaration domains rather than core values.

A pitch-name literal is checked in its expected domain: `chord c# minor` supplies `NoteName`, while an argument to
`Row12` supplies `Pc12`. Outside such an expected constructor position, write a type annotation. Converting an existing
`NoteName` value to `Pc12` requires `forget_spelling`; there is no implicit value coercion in the opposite direction.

`control-domain`, `quantity`, and `range` use the exact unit grammar shared with studio values. `path` is a qualified
identifier such as `std.sound.basic_sine` or `bow.pressure`. `notation-selector` is one documented dynamic,
articulation, span/grouping mark, pedal, or technique pattern; it is not an arbitrary graph path. The sound forms are
staged and desugared by `08-performance-and-sound.md`, not values in the core calculus.

Imports are explicit and are spelled `import`. A quoted path is resolved lexically relative to the importing file; a
`module-path` names a module of a package. `std` is reserved, is never searched in the working directory or environment,
and has no implicit prelude. `use` is not an import: it is the score's splice statement. The two were one keyword, and
the former spelling `use std::…;` is a hard error carrying an applicable fix.

Paths nest to any depth, so a bundled module is named by its position in the package's module tree —
`std::tonal::harmony`, not the flat `std::tonal_harmony`. A package is a directory with `musa.toml` and a source root
whose `lib.musa` declares its children with `mod`, a directory module declares its own in `mod.musa`, and a source file
no `mod` reaches is rejected as *declared nowhere* rather than silently unreachable — as is a `mod` naming no file,
which is *missing*. Resolution follows declarations, never a directory scan.

Imported definitions enter the current **flat** value namespace, so a score writes `numeral_chord(home, five)` rather
than qualifying every call. This deliberately differs from the `Module.member` rule that §6.1's static modules use, and
the reason is the reader: qualification is information to someone building an abstraction and noise to a musician
reading a score. Importing two modules that export the same name is an error naming both; `import p::q as alias;`
resolves it by qualifying that one, so an alias is required exactly at a real conflict and absent otherwise.

Bundled source remains available at stable `musa-stdlib:/std/…` URIs for hover and go-to-definition, but is read-only; a
musician customizes one by writing a local wrapper. The authoritative signatures and prose are generated from source
comments in `stdlib/reference.md`.

### 1.1 Documentation comments

A declaration's documentation is the run of `//` lines written directly above it, with no blank line between. That is
the whole convention: no doc-comment sigil, no attribute, no second comment syntax. A blank line ends a thought, so a
comment separated by one is about the section rather than about the name, and a comment inside a brace or at the end of
a line is not documentation at all.

Deprecation is written the same way, because a deprecation is documentation. A first line reading `deprecated: <what to
write instead>` marks the declaration; the replacement is the rest of that line, and the line stays in the prose so a
reader meets it in place as well as in the editor's strike-through.

```musa
// deprecated: write `subject` instead.
let theme = music { c4/1 };
```

Nothing about this changes what compiles. A deprecated name resolves, elaborates, and sounds exactly as it did; what
changes is what an editor says about it. The alternative — a keyword or an attribute — would make a note to a reader
into a fact about the language, and the compiler has nothing to do with it.

### 1.2 Records

A `record` declares named fields. It is constructed by naming them, read by projecting them, matched by naming the ones
an arm cares about, and rebuilt by `with`:

```musa
record Pending {
    read: Reading;
    length: Length;
    dots: Dots;
    tying: Tying;
    numbers: Numbers;
    taken: Taken;
    voiced: Voiced;
    words: Words;
}

fn clear_body(state: Pending) -> Pending { state with { taken = Taken::NoBody } }

fn refuse(state: Pending, why: Text) -> Pending { state with { read.refusal = Refusal::First(why) } }
```

Five rules fix it.

- **Construction names every field**, in any order, and the declaration's order is the evaluation order. There is no
  positional form. Eight fields written positionally is a line that says nothing about what is in it, and swapping two
  of them is a type error only when their types happen to differ.
- **Projection is the only way to read a field.** `state.dots` reads one field and mentions one field. A record pattern
  is available where an arm wants several — `Pending { read = r, taken = t }` binds those two and says nothing about the
  rest, and `Pending { read }` is the shorthand that binds a field to its own name. There is nothing to be exhaustive
  about, so there is no `..`: a record has one shape.
- **A record is a declaration, and two of them are two types.** This is a **repair**, and the rule it replaces was the
  opposite one: a record used to be *its fields*, so two declarations with the same field names at the same types
  denoted the same type. That followed from a core in which a record type was a structural product, and it existed to
  make trait dictionaries work — two projections of the same dictionary had to be convertible. Prompt 146 deletes the
  trait system and prompt 157 makes a record a **one-constructor inductive family** (`02-core-calculus.md` §1.1), which
  is nominal like every other family, so both the mechanism and its client are gone. What replaces the structural rule
  is nothing, because nothing else used it: no two records in `stdlib/` or `examples/` declare the same field set, so
  the change is a rule about programs nobody has written. An author who wants two quantities kept apart now gets that by
  default.
- **Parameters are allowed and are ordinary**: `record Cell<A> { at: Nat; value: A; }`. A record that must carry an
  operation carries it as a field, which after prompt 146 is how a structure is written at all: `Group` is a record
  whose fields are its unit, its composition, and its inverse, so there is no constraint left for a `where` to discharge
  at construction.
- **It adds no term to the calculus, and after prompt 157 it adds no *shape* either.** A `record` declaration elaborates
  to a one-constructor family, a literal to that constructor applied to its fields, a projection to a generated function
  whose body is a one-branch case tree, a pattern to the case tree of `02-core-calculus.md` §6.2, and `with` to the
  `let`-and-literal rule of §1. The core has no record former, no record introduction, and no projection form; `record`
  is a spelling, and prompt 161 is where that is stated as one declaration form with two conveniences.

The measurement this is answering is in the file above. `stdlib/src/adapters/staff.musa` declares `Pending` as an
eight-field product with the only spelling the language had — a single-constructor `data` — and then destructures all
eight fields at fourteen separate sites to read one or two. Where it wants to change a field of a field it cannot say
so, so it rebuilds the inner value positionally instead: `later with { read = Reading(items, span, opens, hangs, stated,
refusal), … }` is six positional arguments written to replace one of them. Path update is what that line is asking for,
and `refuse` above is what it becomes.

### 1.3 Enums, and constructors that live in a namespace

An `enum` declares a nominal sum. Its cases may be empty, positional, or named:

```musa
enum Tying { Untied, TiedOn }

enum TokenKind { PitchLiteral, Rational, Whitespace, LineComment, BlockComment }

enum Reading<A> {
    Done(A),
    Refused { at: NodePath, why: Text },
}
```

**Constructors live in the type's namespace**: `Tying::Untied`, `TokenKind::PitchLiteral`,
`Reading::Refused { at = p, why = w }`. A bare constructor name is accepted exactly where the expected type is already
known, which is the check direction and not a heuristic: in a checking position (`02-core-calculus.md` §2) the
elaborator has the type, looks the name up in that type's namespace, and either finds it or reports that the type has no
such case. In an inferring position the qualified form is required. Patterns are checked against the scrutinee's type,
so arms write bare constructors and read as they always did.

A named-field case is written with a *qualified* head in both literal and pattern position, which is why
`record-literal` and `record-pattern` take `(type-name | qualified)` rather than `type-name`. `Reading::Refused` is not
a type name and §1.5's path rule forbids reading it as one — a capitalized segment ends a module path — so the head
production has to admit it explicitly. The bare form `Refused { at = p, why = w }` is accepted in checking position
under the same rule as every other constructor.

That rule is not cosmetic, and the evidence is a bug the compiler is still carrying. The staff adapter declares
`Untied`, the staff *package* declares `Untied`, and because constructor names were flat within a module the two
collided; `names_a_phase_type` in `crates/musa-compiler/src/phase/mod.rs` exists to work around what that collision did
to the printer splice. Namespaced constructors delete the collision at its source, so the workaround goes when the last
flat-constructor program does. It also changes what an import can do: a module brings the *type* into scope and the
constructors arrive with it, so two imported enums with a case of the same name cannot conflict at all.

**Both are nominal, and both are the same declaration underneath.** Each `enum` declaration generates its own inductive
family with its own constructors (`02-core-calculus.md` §1.1), so `enum Beats { Beats(Nat) }` and
`enum Bars { Bars(Nat) }` are two types — and after prompt 157 so is every `record`, for the same reason (§1.2). The
choice between the two spellings is therefore about **arity and field names** rather than about identity: `enum` where
there are several cases, `record` where there is one and its fields want names. Prompt 161 makes that exact statement
the rule, with `data` as the form both desugar to.

**An enum may have no cases at all.** `enum Empty {}` declares the type with no closed inhabitant, which is the type
`02-core-calculus.md` §5's consistency obligation is about and the one `P -> Empty` uses to say *not P*. A `match` on a
value of it has no arms, and every arm it does not have is covered.

**A declaration may carry indices, and `data` is where they are written.** A constructor's result type names the indices
it chooses — `Nil : Vec<A>(0)`, `Cons(head: A, tail: Vec<A>(n)) : Vec<A>(n + 1)` — and matching on such a value refines
the index in each branch (`02-core-calculus.md` §1.1). `enum` and `record` are the two spellings that do not write one:
`enum` for several nullary or positional cases, `record` for one case with named fields. This reverses what this
paragraph said before, which was that parameters-and-no-indices was final on the evidence that no committed program
narrows a type by matching; prompt 143's amendment answers that evidence — the corpus was writing the workaround,
seventeen compiler builtins spent on one modulus, rather than exhibiting no demand. `Syntax` is the immediate
beneficiary and is now a family over `Cat` (`11-quotation.md` §1). `Option<A>` and `Result<A, E>` become ordinary enums
declared in `std` rather than grammar; `Some`, `None`, `Ok`, and `Err` read exactly as before under the bare-constructor
rule, and `option_fold` is replaced by the `match` that was always underneath it.

The dispatch table is the other measurement. `text_equal(kind, "PitchLiteral")` appears in the staff adapter at
twenty-one sites over thirteen distinct string literals, and a misspelling in any of them is a comparison that is
quietly false forever. `kind == TokenKind::PitchLiteral` is the same test with the misspelling turned into a resolution
error, and `match kind { … }` over the declared cases is the same table with coverage checked.

**A case may be private, and then the type is abstract outside its module.** `private` before a case hides the
constructor and leaves the type public, so a package can maintain an invariant that its clients cannot break:

```musa
enum Chord {
    private NamedChord(ChordSymbol, List<Spelling>),
    private AnonymousChord(List<Spelling>),
}

fn build(symbol: ChordSymbol) -> Chord { Chord::NamedChord(symbol, tones_of(symbol)) }
```

Inside `Chord`'s own module the constructor is an ordinary name with no ceremony, which is what makes `build` writable.
Outside it, three things are refused and each names the module rather than falling through to "no such name": the
constructor (`private-name`), the generated eliminator, and a `match` that takes the value apart (`abstract-match`). The
eliminator goes with the cases because eliminating an enumeration *is* the case analysis the marker exists to prevent,
and the `match` is refused where it is written rather than silently becoming inexhaustive — a client eliminates through
whatever the package exports. What stays reachable is the type itself: a client writes `Chord` in a signature and
receives one from `build`. The bare-constructor rule above is unaffected inside the module and refuses outside it for
the same reason and with the same diagnostic.

**All the cases or none of them.** One private case beside a public one is refused (`mixed-visibility`), naming both.
The reason is coverage: outside the module a `match` on such a type could still be written, and the arms it is allowed
to write would never exhaust it, so every one of them would need a catch-all for cases the author cannot see. That is a
worse thing to explain than a refusal. Re-opening this needs a program with a genuinely public case beside a private
one, and a stated answer for what its `match` coverage means.

**`private` marks a declaration, and public is the default.** The same word stands before a `let`, `fn`, `record`,
`enum`, `data`, or `structure` and hides the whole declaration; §4 of `04-templates-and-modules.md` states the boundary
it hides behind and why it does not overlap with sealing. A marked declaration is nameable from a sibling definition in
its own module and from nowhere else, including through an `import` alias and through a re-export, and marking one
changes no program that did not name it. `trait` and `impl` take the marker in the grammar above for as long as those
forms survive; §1.4 says who removes them. After prompt 146 the question the marker used to raise there does not arise,
because a structure is an ordinary value with an ordinary name: marking it private hides that name and nothing else,
exactly as it does for a `let`.

Public by default is the opposite of Rust's choice and the opposite of what *A Philosophy of Software Design* ch. 5
would argue for a fresh language, and the argument it loses to is specific rather than general: Musa's packages are
*vocabularies* — `std::notation::staff` exists to be named — and flipping the default would mean marking almost every
one of `stdlib/`'s roughly 150 definitions inside the single migration that is already the largest prompt in the
language pass. The re-opening condition is a measured count, taken after the two adapter rewrites: if the declarations
that *should* have been private and were not are a large fraction of `stdlib/`, the default was wrong and this paragraph
is what changes.

There is one visibility boundary and it is the module. No `pub(crate)`, no `pub(super)`, no package-visible tier, and no
export list: a second tier is a new decision that needs a program that wants it.

### 1.4 Traits and impls — deprecated, owned by 146

**These forms are being removed and this section states what replaces them.** A `trait` declared methods over one or
more type parameters and an `impl` supplied them at a type; the specification that owned coherence, the orphan rule,
instance lookup, and dictionary elaboration is retired by prompt 145 with no successor, and prompt 146 deletes the
mechanism. The grammar keeps `trait`, `impl`, and `where` until that prompt runs, so the corpus still validates against
this document in the meantime.

The measurement behind the removal is the whole argument: six traits, 82 call sites, and **zero** trait-constrained
signatures — not one function in `stdlib/` or `examples/` is polymorphic over a trait — with `Eq`'s five instance bodies
literally the five compiler builtins. A dispatch mechanism with nothing to dispatch on is a name-resolution mechanism
wearing a costume.

Three things replace it, and each is smaller than what it replaces:

- **A structure becomes a record.** `trait Group<G>` becomes `record Group(G : Type) { unit: G; compose: G -> G -> G;
  inverse: G -> G; }`, and an instance becomes an ordinary value. This is strictly more than the trait had, because a
  record is first-class: a function may take two groups, return one, or hold a list of them. `stdlib/src/algebra.musa`'s
  own comments name all three of those as things the trait could not do.
- **Overloading becomes disambiguation.** `==` at five types is five names in scope and an elaborator that already knows
  the expected type (§1.5). The failure mode improves rather than degrades: a diagnostic listing the candidates and the
  type that ruled each out says more than "no instance found".
- **Open dispatch, where anything genuinely wants it, is a macro's job** — `11-quotation.md`, and prompt 160.

An `impl` block *without* a trait head declares inherent items in the type's namespace: `impl Duration { fn of(r: Ratio)
-> Result<Duration, RangeError> { … } }`. That form is **not** deprecated: it is a namespace, not a dispatch mechanism,
and prompt 146 keeps it.

### 1.5 Methods, paths, and operators

`x.m(y)` resolves **by exact receiver and in one step**. The elaborator takes the head of `x`'s already-known concrete
type, looks for `m` among that type's inherent items and among the methods of the traits whose dictionaries are in scope
for that head, and finds exactly one candidate or reports the failure. Three things follow, and each is refused rather
than left to a search:

- A value whose type is a generic parameter `A` never acquires `.m` from anywhere. The caller writes the constraint or
  the qualified path; otherwise adding a trait to a package would change what existing code means.
- There is no auto-deref, no receiver coercion, and no fallback to a free function whose first parameter happens to fit.
- Where the receiver's type is still undetermined after the spine walk, the method call is refused at the call, and the
  refusal names the qualified path to write instead. Nothing is postponed *here*: the constraint queue
  `02-core-calculus.md` §2.1 installs holds *comparisons* — a conversion that is not yet decidable, retried when an
  unknown it mentions is solved — and a receiver at an unknown is not a comparison but a name that did not resolve,
  which that queue has no way to hold. §2.1's two-pass spine is what makes a receiver's type known in the cases that
  used to need it — an argument the walk defers is checked after the arguments that decide it, so
  `applied(fn (p) { p.act(P8) }, c4)` resolves `.act` at `Pitch`.

`T::x` names an item in `T`'s namespace: a constructor, an inherent function, or a trait method under
`Trait::method(x)`. Explicit qualification is always available and always resolves, which is the escape hatch that makes
the strictness above affordable. A `::` path is read left to right, and the capitalization rule §1 already fixed decides
where the module prefix ends: lowercase segments are modules, the first capitalized segment names a type or a trait, and
exactly one segment follows it. `std::tonal::TokenKind::PitchLiteral` has one reading.

The `.` in an expression is projection or a method call. The `path` production's `.` — `bow.pressure`,
`std.sound.basic_sine` — is a control address inside the sound declaration forms, which are staged rather than
evaluated, and `Structure.member` (§6.1) is read at declaration time. The three never meet in one grammar. Nothing else
is ever added to that list: `f . g` and `f.g` are the same three tokens — whitespace is trivia — so a composition
operator spelled `.` would make one of the three unreadable. Composition is `std::core`'s `compose`, an ordinary
declaration, which is what a section makes writable.

**A `_` in an argument list is a section**: `f(a, _)` denotes the function that still needs the slot the `_` stands in,
and several read left to right, so `g(_, b, _)` is `fn (x, y) { g(x, b, y) }`. It is surface only — the elaborated term
is the complete call inside a lambda, so `02-core-calculus.md` §1.3's completeness rule is untouched and an
under-applied call *without* a `_` is the type error it always was. The word is the one `pattern` already uses and means
the same thing there: a slot with no name. A `_` written anywhere but an argument list is refused, naming itself.

**Operators are surface syntax for named functions.** `x == y` is `equal(x, y)`, `x < y` is `less(x, y)`, `x + y`, `x -
y`, `x * y`, `x / y` are `add`, `sub`, `mul`, `div`, and `xs[i]` is `at(xs, i)`. Which `equal` is meant is decided by
the **type-directed disambiguation** prompt 146 introduces: the candidates are the definitions of that name in scope,
the expected type rules out all but one, and a call that leaves more than one standing is refused with the candidates
and the type that failed to separate them listed. Two rules keep this from becoming overloading under another name.

- **An operator resolves only when the expected type or the head argument's type is known.** There is no search and no
  defaulting; an unresolved operator names the type it could not separate the candidates by, and lists them.
- **An operation that can fail keeps its failing shape.** `ratio_div` answers `Result` today and `x / y` answers
  `Result` tomorrow; `xs[i]` answers `Option<A>` for a list, because a list index can be out of range. A partial
  operator is how a total language quietly grows a hole, and the shape is the thing that stops it. A container whose
  index type cannot be out of range may have a total instance; the language does not promise one here.

Heterogeneous operations stay named functions on purpose. `position_shift(p, d)` adds a duration to a position and
`duration_scale(d, r)` scales a duration by a rational; neither is `+` or `*`, because the operators are homogeneous and
because these are exactly the two operations `02-core-calculus.md` §1.1 separates `Position` from `Duration` to keep
distinguishable. An operator that quietly accepted a beat where a number of beats was meant would give back the one
arithmetic error the two types exist to catch.

### 1.6 Collections at the surface

A list literal has a type: `[c4, d4, e4] : List<Pitch>`. The elements are checked against one type, and an empty `[]`
takes its element type from the position it is written in — in an inferring position with nothing to take it from, it is
refused, and the diagnostic names the annotation to write.

The rest is ordinary definitions, and that is a change from what this section used to say. It described two traits,
`Iterable` and `Buildable`, with `fold_from_start` and `fold_from_end` required and `map`, `filter`, and `collect`
derived. Prompt 146 deletes the trait system, and prompt 156 generates an eliminator for every declared family, so the
fold over a container **is** that container's recursor and nobody writes it: `iterable_list()`'s hand-written
catamorphism was a stand-in for the one `List` already implies. `map`, `filter`, and `collect` become ordinary library
functions — one set per container, reached by exact receiver like any other method: `xs.map(f)`, `xs.filter(keep)`,
`xs.fold_from_end(zero, step)`.

`collect` is where "no return-type-directed overloading" needs saying precisely. `let out: List<Nat> = xs.collect();`
works because the answer type is fixed by *checking* against the annotation, and a type fixed by checking is not a
search. `xs.collect()` in an inferring position is refused, naming the answer type as the thing it could not determine.
The refused design is the other one: choosing which definition to use *because* of a return type nobody has written down
yet, which makes elaboration depend on the order constraints are reached.

This is the grammar and not the library. The library owns `List`, its folds, and its builders; nothing here promises
what those look like. There is no comprehension in v1: a comprehension is sugar over `map` and `filter` (Peyton Jones
1987 ch. 7), and adding the sugar before the thing it sugars has a user is the wrong order.

## 2. Functions and music

```musa
let fifth: Interval = P5;

fn third(root: Pitch) -> Pitch { root up M3 }

fn transpose_answer(subject: EventTrack[WrittenTime, ScoreFact], by: Interval) -> EventTrack[WrittenTime, ScoreFact] { transpose(by, subject) }

motif turn(root: Pitch) {
    root/8
    (root up M2)/8
    ((root up M2) down m2)/8
    root/8
}
```

`motif turn(...) { body }` desugars to a named `fn turn(...) -> EventTrack[WrittenTime, ScoreFact] { music { body } }`
with a `Motif` role retained for lints, extraction, editing, and Origin. `fragment name { body }` desugars to
`let name = music { body };` with a `Fragment` role. `use e;` checks that `e` is a written-time score track, `follow`s
it onto the voice at the current cursor, and advances by `duration(e)`. Existing `use name(args);` is the same rule, not
a second invocation mechanism.

The type is written out rather than abbreviated. Prompt 127a deleted the type name `Music`, because a name that short
for a type that specific is how the deleted contextual-`Music` design read as ordinary in the first place; a shorter
spelling needs its own prompt and its own evidence.

A track value is an ordinary value, and `in scale` is lexical rather than captured:

```musa
fn opening() -> EventTrack[WrittenTime, ScoreFact] { music { c5/8 } }

let subject = opening();
in scale c major  { use subject; (c5 step 1)/8 (c5 step 2)/4 }
in scale c dorian { use subject; (c5 step 1)/8 (c5 step 2)/4 }
```

*Lexical* is the whole of it: `in scale` supplies the collection to the pitch positions written inside its braces, and
to nothing else. A `step` is finished where it stands, so the two blocks above differ under `≈facts` at the steps they
each wrote, and `subject` — already a track by the time either `use` names it — plays the same `c5` in both. A phrase
that steps and is written outside any scale is refused at its own definition, because no use site can supply what it is
missing; a phrase parameterized by its collection is a `fn` over the pitches, not a track waiting for a context. A
`music { … }` value is its own lexical region for the same reason, so an `in scale` outside its braces does not reach
inside them.

This paragraph said the opposite through prompt 141n, and the design it described is the contextual `Music` prompt 127a
deleted: a saved phrase whose notes its own definition did not fix. Prompt 142 is where the sentence caught up with the
decision.

`in scale` emits no key fact and adds one `ScaleContext` step to the origin of every fact made under it, which is how a
reader tells a spelling the source wrote from one a step arrived at. An absent scale makes `step` a type-context
diagnostic, not an implicit C-major choice — but a written `key` is not absence: it suggests a collection (`key_scale`),
and that suggestion is the default a `step` counts in until an `in scale` overrides it.

## 3. Higher-order construction with controlled traversal

```musa
fn canon(
    subject: EventTrack[WrittenTime, ScoreFact],
    answer: EventTrack[WrittenTime, ScoreFact] -> EventTrack[WrittenTime, ScoreFact],
    gap: Duration,
) -> EventTrack[WrittenTime, ScoreFact] { music {
    use together(subject, shift(gap, answer(subject)));
} }

fn harmonize(
    subject: EventTrack[WrittenTime, ScoreFact],
    answer_pitch: Pitch -> Pitch,
) -> EventTrack[WrittenTime, ScoreFact] { music {
    use together(subject, map_note_pitches(answer_pitch, subject));
} }

use canon(
    theme(),
    fn (line: EventTrack[WrittenTime, ScoreFact]) -> EventTrack[WrittenTime, ScoreFact] { transpose(P5, line) },
    1/2,
);
```

`fn (…) -> τ { e }` is the **anonymous function**: a declaration's own words without its name, with the parameter and
result types omissible exactly where a declaration may omit them. It is how a higher-order call is specialized by a
value the caller supplied, because a call supplies every parameter (`docs/rules/constitution.md` §9) and a named `fn` is
declared where the declarations are, so it cannot close over an argument its caller just wrote. It captures lexically,
by value; it has no name and so cannot apply itself; and by §1.1 of `02-core-calculus.md` it may be applied and passed
and may not be stored.

`map_note_pitches` is the sole initial user-facing traversal of a score track. It changes pitches in note and
sounded-chord events; it preserves time, annotations, marks, scope, and Origin; it does not traverse key signatures or
chord-symbol analysis. No iterator exposes a `ScoreFact` or a core occurrence.

## 4. Assertions and analyses

```musa
assert pitches_in(scale c major) {
    c5/4 e5/4 g5/2
}

assert pitches_in(scale c major) {
    c5/4 f#5/4 g5/2
}
```

The first succeeds and returns the body as music. The second is a compile error at `f#5`, with the predicate's witness
and the enclosing assertion in the diagnostic. `assert p(args) { body }` desugars to `checked(p(args), music { body })`;
`p` must be a constructor invariant or decidable assertion returning a structured witness, not an interpretive analysis.
It is drawn from a fixed registry rather than from the composer's own definitions, and the name says what is read: this
one reads the passage's sounded written pitches, so `pitches_in` is what it is called. §2 of `05-verification.md` is the
rule the naming follows, and `fits_scale` — which this example said before prompt 116 — is what it rules out, since
nothing in the word "fits" says which of a passage's properties was looked at.

Interpretation is named and non-blocking:

```musa
analysis harmony = roman_numerals(chorale(), in: key c major);
```

This produces `Analysis<roman_numeral>`; it neither changes nor validates the score unless an explicit assertion reads a
decidable property of the result.

## 5. Chords, rows, and explicit register

```musa
let sonority: ChordClass = chord c major7;
let close: Option<Voicing> = close_position(sonority, c4);
let open: Option<Voicing> = drop_position(sonority, c3, 2);

fn sound(chosen: Voicing) -> EventTrack[WrittenTime, ScoreFact] { play(chosen, duration_of(1/2)) }
fn sounded(chosen: Option<Voicing>) -> EventTrack[WrittenTime, ScoreFact] {
    match chosen {
        Some(voicing) -> sound(voicing),
        None -> music { rest/2 },
    }
}
let close_bar = sounded(close);
let open_bar = sounded(open);

use close_bar;
use open_bar;

stack c4 major7/2

let row: Row12 = row12(c, cs, e, d, fs, f, as, g, gs, b, a, ds);
let symmetric: Row12 = row12(c, fs, d, gs, e, as, f, b, g, cs, a, ds);
let matrix: List<List<Pc12>> = row_matrix(symmetric, convention: zero_based);
```

`chord` does not sound: a chord class is rooted spelled content with no register, spacing, doubling, or bass. A voicing
policy is an ordinary named function that selects those and returns `Option<Voicing>`, absent when its preconditions do
not hold — a bass the class does not contain, or a register the written range cannot reach. `play` alone creates sounded
music. `stack <pitch> <quality>/<duration>` is sugar for the close-position policy with the absolute root fixing
register; `stack c major7/2` is rejected, because a pitch class chooses no register. `Row12` statically requires each
`Pc12` exactly once; symmetry may make fewer than 48 distinct `P`/`I`/`R`/`RI` forms, which is a result, not an error.
Row-form naming always states a convention.

## 6. Declaration templates — deprecated, owned by 162

**`template`, `signature`, `structure`, `template structure`, and `make` are being removed, and prompt 162 removes
them.** They are the module layer, and after prompt 146 a structure is an ordinary record and a functor an ordinary
function, so the layer describes a second way to say what the term language already says. The forms stay in the grammar
and in this specification until that prompt runs, so `stdlib/` and `examples/` still validate against this document in
the meantime; nothing new should be written in them, and §6 and §6.1 below record what they mean rather than what they
are for.

```musa
fn theme() -> EventTrack[WrittenTime, ScoreFact] { music {
    c4/4
    d4/4
} }

template voice answer(subject: EventTrack[WrittenTime, ScoreFact], transform: EventTrack[WrittenTime, ScoreFact] -> EventTrack[WrittenTime, ScoreFact]) {
    use transform(subject);
}

template piece study(k: Key, mode: Scale, subject: EventTrack[WrittenTime, ScoreFact]) "Study" {
    key k;
    score {
        part piano {
            voice right { in scale mode { use subject; } }
            make answer(subject, fn (line: EventTrack[WrittenTime, ScoreFact]) -> EventTrack[WrittenTime, ScoreFact] { transpose(P8, line) }) as follower;
        }
    }
}

make study(key g major, scale g mixolydian, theme()) as study_in_g;
```

`template` and `make` are structural syntax, not expressions. The `as` name is mandatory and participates in stable
generative identity. `piece`, `voice`, or `structure` with parameters but without `template` is rejected.

A file is still one piece or one library, so an instance is placed by the kind it makes: a `make` of a piece template
stands at the file root and *is* that file's piece, and a `make` of a voice template stands among a part's voices.
Whatever precedes the file's piece or library — imports, bindings, functions, templates — is the file's lexical root. A
template body reads that root and its own parameters and nothing from the site that instantiates it; the arguments at a
site are evaluated in the site's own scope, which is why `subject` above can be passed on from `study` to `answer`.

## 6.1 Signatures and structures — deprecated, owned by 162

A signature names what a bundle of values must provide; a structure provides them; a `template structure` is a functor
from structures to a structure. All three are deprecated on §6's terms: a record type is the signature, a record value
is the structure, and a function from one to another is the functor.

```musa
signature TonalContext {
    let tonic: Key;
    let collection: Scale;
    let spell: Degree -> Option<Pitch>;
}

structure CMajor: TonalContext {
    let tonic: Key = key c major;
    let collection: Scale = scale c ionian;
    let spell: Degree -> Option<Pitch> = degree_in_c;
}

template structure Sequences(C: TonalContext, gap: Duration): TonalContext {
    let tonic: Key = C.tonic;
    let collection: Scale = C.collection;
    let spell: Degree -> Option<Pitch> = C.spell;
    let delay: Duration = gap;
}

make Sequences(CMajor, 1/2) as CSequences;
```

A signature member is a `let` without its definition: a name and the type the structure must give it. Matching is by
name and exact type — a missing member and a member of the wrong type are both errors, each labelled at the signature
and at the structure. A member the signature does not mention stays private to the structure: it is what the structure's
own definitions may use and what nothing outside may name.

Members are read as `Structure.member`. Inside a structure, a sibling member is read by its bare name. There is no
structure value, no structure argument to a function, no unpacking, and no recursion: `structure`, `signature`, and
`template structure` are structural syntax that has finished before any value exists.

## 7. Events documents and quotation

A standalone `.musa.events` file contains exactly one closed term in the grammar of
`docs/rules/events/10-term-calculus.md`:

```text
% musa-events-3
events "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    track 1/2 {
      occurrence "voice 0 0 note c4 1/2 [0:4]" from 0 to 1/2;
    };
}
```

It has no imports, functions, surface pitch operations, or free variables. Its payload text must decode as `ScoreFact`.

A local quote is host syntax containing event-track syntax and typed antiquotation:

```musa
fn delayed_double(
    subject: EventTrack[WrittenTime, ScoreFact],
) -> EventTrack[WrittenTime, ScoreFact] { events EventTrack[WrittenTime, ScoreFact] {
        let s = ${subject} in
        together { s; shift by 1/2 s; }
    } }
```

`${subject}` is one track antiquotation. It is instantiated in the quote's host environment and inserted as a typed
events-term hole. Events identifiers never capture host identifiers; alpha-renaming prevents capture among inserted
terms. The completed quote must close and type-check before it becomes a track. No raw payload escape exists.

Musa has a second quotation form — `quote at p { … }`, which builds a `Syntax(c)` in the expansion phase — and the two
are deliberately not merged. The discipline they share is stated once in [`11-quotation.md`](11-quotation.md) §6: holes
are typed, nothing quoted captures a host identifier or the reverse, a quote must close and check before it becomes
anything, and the locus is where a hole is instantiated. What follows is this quote's own, and it stays here.

Four rules a writer of quotes needs, and each one is the same rule the rest of the language already keeps:

- **A quote is commented the way the file around it is.** `//` and `/* */` are trivia inside a quote, and `%` is not:
  the quote is Musa source, so the host's comment syntax is the one that applies. The event track's `%` lines belong to
  `.musa.events` documents, which are not written inside a piece.
- **A raw payload says what the material is, and nothing about where it goes.** It states no scope and no origin; both
  are supplied by the use, exactly as they are for any shared body, and a quote that spells either is refused. A key,
  meter, clef or tempo payload is refused for the same reason at one remove — those are structural declarations with
  scope authority, and "from here onward" has no unique meaning in a value used at several places (`00-semantics.md`
  §3).
- **The quotation locus is where a hole is *instantiated*, not where its facts land.** The two differ under `let`: a
  hole in a `let` value is instantiated once, at the `let`'s own locus, and each reference then places the finished
  facts wherever the term writes it. Every fact leaving a quote records that locus.
- **Raw `shift`, `scale` and `restrict` are operations on time.** They move occurrences; they do not rewrite payloads,
  because payloads are opaque to the event track and arrive already transformed (`docs/rules/events/01-grammar.md`).
  Augmentation therefore belongs in the host — `${stretch(1/2, subject)}` — and raw `scale` belongs to material whose
  written values already say what was meant.

## 8. Sound corpus

The following is the settled musician-facing shape; `08-performance-and-sound.md` defines it.

```musa
performance {
    profile lyrical for note_instrument {
        dynamic p  -> expression 0.28;
        dynamic f  -> expression 0.82;
        articulation accent -> emphasis 0.75;
        articulation staccato -> separation 0.65;
        slur -> phrase legato;
    }
    profile dry for note_instrument {
        dynamic p -> expression 0.35;
        dynamic f -> expression 0.90;
    }
}

instrument solo_strings from "pkg:orchestra/solo-violin.sfz" conforms note_instrument {
    control bow.noise: normalized = 0.12;
    control bow.bridge_distance: mm in [0 mm, 50 mm] = 20 mm;
}

instrument mallets from "assets/marimba.sfz" conforms note_instrument;

score {
    part violin {
        sound solo_strings using lyrical;
        voice melody {
            dynamic p;
            crescendo to f { c5/4 d5/4 e5/2 }
        }
    }
    part marimba {
        sound mallets using dry;
        voice pulse { c4/4 c4/4 g3/2 }
    }
}
```

Within a part, `sound instrument using profile;` is the ordinary one-action form. It desugars to the independent profile
selection, part-to-instrument assignment, and part-output-to-master route below. The expert surface may spell those
facts separately and add sends:

```musa
part violin {
    profile lyrical;
    voice melody {
        dynamic p;
        crescendo to f { c5/4 d5/4 e5/2 }
    }
}

part marimba {
    profile dry;
    voice pulse { c4/4 c4/4 g3/2 }
}

studio {
    assign violin -> solo_strings;
    assign marimba -> mallets;
    send violin -> concert_hall at -12 dB;
    send marimba -> concert_hall at -16 dB;
    room concert_hall { decay: 1.8 s; }
    route violin -> master;
    route marimba -> master;
    route concert_hall -> master;
}
```

A hairpin becomes an exact `expression` curve because the chosen profile says so; it never names a filter address.
Swapping `solo_strings` for any instrument conforming to `note_instrument` preserves well-typed gestures, not identical
sound.

```musa
clip pulse from "assets/pulse.wav" fit 4/1 by rate;
cue pulse at 9:1;

fixed_media harbor from "assets/harbor.wav";
cue harbor at 17:1;
```

The clip is beat-fitted and follows tempo. The fixed-media cue is only an event track point at the score position; its
recorded duration remains seconds and is never manufactured into a written-time duration.

## 9. Corpus correctness relation

| Corpus case | Desugaring | Required equality |
| --- | --- | --- |
| root-dependent turn | motif-role function plus interval action | `≈music` |
| major/dorian rebinding | Reader-style `in_scale` | `≈music` per environment; uses differ under `≈facts` |
| canon | `together(subject, shift(gap, answer(subject)))` | `≈material` |
| harmonizer | controlled pitch traversal | `≈music` |
| key-parameterized piece / parameterized voice | declaration-template expansion | full facts retain distinct instance Origin; `≈facts` after erasure |
| one chord class, two voicings | `play(voice(...))` | intentionally unequal under `≈facts` |
| generic/symmetric row | finite row constructor and transforms | value equality; distinct-form count is observed |
| assertion | `checked(predicate, body)` | successful body `≈music`; failure has no value |
| standalone events | closed term parsing | `≡events` |
| quote with antiquotation | typed substitution then closure | `≡events` after instantiation |
| swappable instruments/profiles | signature checking and profile realization | equal gesture type; sound equality not promised |
| expression hairpin | profile-generated `ControlKey::expression` curve | exact gesture equality |
| shared room | explicit mix-graph sends | frame equality modulo documented deterministic summation order |
| sampled instrument | sample-map implementation of signature | behavioral conformance, not waveform equality |
| beat-fitted loop | tempo-scheduled clip gesture | scheduled-lane equality |
| fixed-duration cue | onset conversion plus immutable seconds duration | scheduled-media equality |

### 9.1 The added forms, accepted and rejected

The table above relates a corpus case to the equality it must satisfy. The forms §§1.2–1.6 add are not about equality
between two programs; they are about which programs exist. Each therefore carries a pair — the accepted spelling with
the core form it elaborates to, and the rejected one with what the diagnostic says. A rejection with no named diagnostic
is a rule nobody can implement, which is why the third column is not optional.

| Form | Accepted, and what it becomes | Rejected, and what the diagnostic names |
| --- | --- | --- |
| record declaration | `record Pending { read: Reading; … }` ⇝ a core record type | a field named twice — *duplicate field*, pointing at both |
| record literal | `Pending { read = r, … }` ⇝ core record introduction | a literal missing a field, or naming one the record does not have — *missing field* / *no such field*, listing the declared set |
| positional record construction | — | `Pending(r, l, d, …)` — *a record is constructed by naming its fields*, with the field list as the fix |
| projection | `state.dots` ⇝ core projection | `state.dot` — *no such field*, naming the record and its fields |
| record pattern | `Pending { read = r, taken = t }` ⇝ a case-tree binding | `Pending { .. }` — *a record pattern names the fields it binds*, since there is nothing to be exhaustive about |
| path update | `state with { read.refusal = e }` ⇝ one `let` and one literal per segment | `state with { read = x, read.refusal = e }` — *one path is a prefix of the other*, pointing at both |
| two records, same fields | one is accepted where the other is expected, by §1.2 | — (this is the priced consequence, and the fix a diagnostic would offer is `enum`) |
| enum declaration | `enum Tying { Untied, TiedOn }` ⇝ an inductive family | a case named twice — *duplicate case*, pointing at both |
| qualified constructor | `Tying::Untied` ⇝ the family's constructor | `Tying::Tied` — *no such case*, listing the declared cases |
| bare constructor, checking | `let t: Tying = Untied;` ⇝ the same constructor | `let t = Untied;` — *bare constructor needs an expected type*, with the qualified form as the fix |
| enum pattern | `match t { Untied -> …, TiedOn -> … }` ⇝ a case tree | a missing case — *non-exhaustive match*, naming the cases left out; an arm no constraint reaches — *unreachable arm* |
| private declaration | `private fn dotted_factor(dots: Dots) -> Ratio { … }` ⇝ the same declaration, answered only inside its module | naming it from anywhere else, including through an `import` alias — *private name* (`private-name`), naming the module that maintains it |
| private cases | `enum Chord { private NamedChord(…), private AnonymousChord(…) }` ⇝ the family with module-local constructors and a public type | one private case beside a public one — *mixed visibility* (`mixed-visibility`), naming both cases |
| abstract elimination | inside the module, `match c { NamedChord(s, t) -> … }` ⇝ a case tree; outside it, whatever the package exports | the same `match` outside the module — *abstract match* (`abstract-match`), naming the type and its module rather than reporting an inexhaustive one |
| misplaced marker | — | `private use x;` — *`private` does not mark this*, since only a declaration can be private |
| redundant marker | — | `private` on a structure member — *this is already private*, naming the signature that hides everything it does not list |
| trait declaration | `trait Eq<A> { fn equal(x: A, y: A) -> Bool; }` ⇝ `Eq : (A : Type 0) → Type 0` over a record type | a required method with no parameter mentioning a trait parameter — *method does not use the trait's parameter* |
| impl | `impl Eq<Tying> { … }` ⇝ a definition of `Eq Tying` | an impl omitting a required method — *missing method*; an impl supplying a derived one — *derived methods are not replaceable* |
| coherence | one impl per trait and head type | a second `impl Eq<Tying>` anywhere in the program — *duplicate instance*, naming both declarations |
| orphan rule | an impl in the trait's package or the head type's | either package's impl for two foreign names — *orphan instance*, naming the two packages that could hold it |
| lookup termination | an instance whose `where` constraints are smaller than its head | one that is not — *instance context does not decrease*, at the declaration and never at a use |
| `where` on a generic | `fn same<A>(x: A, y: A) -> Bool where Eq<A>` ⇝ a dictionary parameter | the same signature without the clause — *missing constraint*, naming `==` as what needed it |
| operator at a known head | `x == y` ⇝ `d.equal(x, y)` | `x == y` at an unconstrained parameter — *no instance*, naming the type and the trait |
| failing operator shape | `a / b : Result<Ratio, ArithmeticError>` | an author treating it as a `Ratio` — the ordinary type error, and the `?` or `match` as the fix |
| indexing | `xs[i] : Option<A>` ⇝ `Index::at(xs, i)` | indexing a type with no instance — *no instance*, naming `Index` |
| method call | `xs.map(f)` ⇝ the derived method applied to the dictionary | `x.m(y)` where `x : A` is a parameter — *method lookup needs a concrete type*, with the `where` or `Trait::m(x, y)` as the fix |
| qualified path | `std::tonal::TokenKind::PitchLiteral` ⇝ that constructor | a lowercase segment after a capitalized one — *a type namespace holds one item*, pointing at the extra segment |
| inherent constructor | `Duration::of(r) : Result<Duration, RangeError>` | an unqualified `of(r)` chosen by its result type — *unresolved name*, since return-type-directed overloading does not exist to find it |
| list literal | `[c4, d4] : List<Pitch>` | `[]` in an inferring position — *element type unknown*, with the annotation as the fix |
| `collect` | `let out: List<Nat> = xs.collect();` ⇝ `D` fixed by checking | `xs.collect()` in an inferring position — *type parameter not determined*, naming `D` |
| comprehension | — | `[f(x) for x in xs]` — *no comprehension*, with `xs.map(f)` as the fix |
| former `data` | — | `data D { … }` — *a product and a sum get different words*, with `record` or `enum` as the applicable fix |
