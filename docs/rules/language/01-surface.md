# Surface language candidate

This file settles the punctuation and spellings introduced by the candidate. Existing syntax remains unless a rule below
explicitly desugars it. Braces delimit blocks. Added bindings, calls-as-statements, and declarations end in `;`; commas
separate arguments; `=` introduces an expression body or binding. Existing note, rest, and chord events remain
self-delimiting and do not take `;`. No added production is newline-sensitive.

## 1. Added grammar

The normative schematic grammar is:

```ebnf
type         := base-type | "Option" "<" type ">" | "List" "<" type ">"
              | "(" type ")" | "(" type "," type ("," type)* ")" | type "->" type
binding      := "let" IDENT ":" type "=" expr ";"
function     := "fn" IDENT "(" params? ")" "->" type block
param        := IDENT ":" type ("=" expr)?
call         := expr "(" args? ")"
expr         := literal | IDENT | path | "(" expr ")" | block | product | list | option
              | call | match | music-expr
block        := "{" expr "}"
product      := "(" expr "," expr ("," expr)* ")"
list         := "[" (expr ("," expr)*)? "]"
option       := "None" | "Some" "(" expr ")"
match        := "match" expr "{" match-arm ("," match-arm)* ","? "}"
match-arm    := pattern "->" expr
pattern      := "_" | literal | IDENT | "None" | "Some" "(" IDENT ")"
              | "[" "]" | "[" IDENT "," ".." IDENT "]"
              | "(" IDENT "," IDENT ("," IDENT)* ")"
music-expr   := "music" "{" music-statement* "}"
music-use    := "use" expr ";"
scale-local  := "in" "scale" expr "{" music-statement* "}"
assertion    := "assert" IDENT "(" args? ")" "{" music-statement* "}"
analysis     := "analysis" IDENT "=" expr ";"
kernel-quote := "kernel" "EventTrack" "[" "WrittenTime" "," "ScoreFact" "]" "{" kernel-item* "}"
antiquote    := "${" expr "}"
document     := (import | binding | function | signature | structure | template | instance)*
                (piece | library | instance)
signature    := "signature" IDENT "{" member* "}"
member       := "let" IDENT ":" type ";"
structure    := "structure" IDENT params? ":" IDENT "{" (binding | function)* "}"
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

Function arrows associate right; call binds tighter than pitch operators; pitch operators bind as follows, tightest
first: parentheses, `step`, `up`/`down`. `root up M2 down m2` is rejected as ambiguous; write parentheses. `up` and
`down` take a `Pitch` or a `NoteName` and return whichever they were given, so `c4 up M3` is a pitch and
`chord_root(triad) up M3` is a pitch class: the operand's own type decides, and no register is invented for a value that
never had one. Every `fn` has an expression body, written in braces: `{ e }` is a block, it holds exactly one
expression, and it means that expression — `⟦{ e }⟧ = ⟦e⟧` (`02-core-calculus.md` §5). A block is an expression form
wherever an expression is admitted, not a special case of `fn`. There is no statement language inside it: no `let`, no
`return`, no `;`-separated sequence, and a second expression in a block is a static error naming the rule. A
multi-statement musical body is explicitly `music { ... }`, which is a different construct that happens to abut the
body's brace.

Named intervals use conventional `P`, `M`, `m`, and repeated `A`/`d` qualities. Because lowercase `d4` already means the
written pitch D4, a singly diminished fourth is written `dim4`; `dd4` and `ddd4` remain the compact multiply diminished
spellings.

`match` is the sole added case-analysis spelling. Arms are comma-separated and a final comma is accepted; braces and
arrows keep the alternatives legible when an arm's expression spans lines. The initial patterns cover booleans, naturals
and other literal domains, options, empty/cons lists, and products. A bare identifier binds the whole value; `_`
discards it. Prompt 96 defines exhaustiveness and rejects duplicate or unreachable arms. It also owns the constructor
meaning of `[head, ..tail]`; `..` is two adjacent `.` tokens, not a new general range operator.

Structural folds do not add syntax. `nat_fold(zero, step, count)`, `list_fold_from_start(zero, step, values)`,
`list_fold_from_end(zero, step, values)`, and `option_fold(zero, some_case, value)` are ordinary calls to compiler-owned
total builtins. A step argument is a named function or an anonymous one, whichever reads better at the call site. This
gives musicians one call notation to learn and leaves `repeat n { body }` as the notation-facing fold over musical
material.

A list has two folds because it is the one type whose direction is observable, and the direction is in the name rather
than in the type: both have the identical signature, so a reader comparing two calls compares only the word that
differs. `list_fold_from_start` accumulates left to right; `list_fold_from_end` is the catamorphism, and it is what
reads a region into right-nested data without a closure chain. `02-core-calculus.md` §5.6 says why no other type needs
the distinction.

The value types added here are `Bool`, `Nat`, `Ratio`, `Duration`, `Pitch`, `Interval`, `NoteName`, `Pc12`, `Scale`,
`Key`, `Degree`, `ChordClass`, `Triad`, `Roman`, `Voicing`, `Row12`, `Analysis<A>`, and `EventTrack[C, A]`. Products,
options, lists, and arrows are the constructors described in `02-core-calculus.md`. Declaration kinds are not types.
Every type is spelled with a capital and every music statement keyword is not, which is what lets `key c major;` set a
key and `Key` name the type of what it set without either word looking the other up (prompt 113). Six of these words —
`pitch`, `music`, `scale`, `key`, `degree`, `frame` — are *also* music statement keywords, and one word doing two jobs
in two grammars is a collision a parser can only paper over; a capital settles it in the lexer. `NoteName` is the letter
and accidental as written, with no octave: a pitch class is octave *and* enharmonic equivalence (Open Music Theory 99),
so a type in which C♯ and D♭ differ is a name rather than a class, and `Pc12` is the class it names.

A type parameter is angle-bracketed, so `[` keeps exactly one job — the list literal `[c4, d4]` and the list pattern
`[x, ..xs]`, which are one idea seen from two sides. The ambiguity that makes `<>` expensive elsewhere cannot arise
here: `Option`, `List`, and `Analysis` are the only parameterized types, all are keyword-headed, and there is no
user-written type application at all, so the parser knows it is reading a type before it reaches the `<`. This is
notation, not polymorphism — the language has no user parametric polymorphism and these constructors stay
compiler-owned.

Three former spellings are **hard errors carrying an applicable fix**, on the same precedent as `use` in import position
and for the same reason — a language that accepts both spellings has a mixed corpus forever, and the fix machinery makes
one spelling affordable:

| Former spelling | Current spelling | Rule |
| --- | --- | --- |
| `fn f(x: τ) -> υ = e;` | `fn f(x: τ) -> υ { e }` | a function body is a block expression |
| a lowercase type name, and `pitchclass` | `UpperCamelCase`, and `NoteName` | a type is spelled with a capital |
| `option[τ]`, `list[τ]` | `Option<τ>`, `List<τ>` | a type parameter is angle-bracketed |

The core literals introduced here are `true`, `false`, nonnegative decimal naturals, exact rational literals, products,
finite lists, and `Some`/`None`. Existing pitch and interval literals are also expression atoms. Strings and
floating-point values remain syntax of their owning declaration domains rather than core values.

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
fn figure() -> EventTrack[WrittenTime, ScoreFact] { music {
    c5/8
    (c5 step 1)/8
    (c5 step 2)/4
} }

let subject = figure();
in scale c major { use subject; }
in scale c dorian { use subject; }
```

The two uses differ under `≈facts`, because `in scale` is resolved where the pitches are resolved; saving `subject` does
not freeze the scale. `in scale` is lexical and emits no key fact. An absent scale makes `step` a type-context
diagnostic, not an implicit C-major choice.

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

fn sound(chosen: Voicing) -> EventTrack[WrittenTime, ScoreFact] { play(chosen, 1/2) }
fn sounded(chosen: Option<Voicing>) -> EventTrack[WrittenTime, ScoreFact] { option_fold(music { rest/2 }, sound, chosen) }
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

## 6. Declaration templates

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

## 6.1 Signatures and structures

A signature names what a bundle of values must provide; a structure provides them; a `template structure` is a functor
from structures to a structure.

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

## 7. Kernel documents and quotation

A standalone `.musa.kernel` file contains exactly one closed term in the grammar of
`docs/rules/kernel/10-term-calculus.md`:

```text
% musa-kernel-2
kernel "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    track 1/2 {
      occurrence "voice 0 0 note c4 1/2 [0:4]" from 0 to 1/2;
    };
}
```

It has no imports, functions, surface pitch operations, or free variables. Its payload text must decode as `ScoreFact`.

A local quote is host syntax containing kernel syntax and typed antiquotation:

```musa
fn delayed_double(
    subject: EventTrack[WrittenTime, ScoreFact],
) -> EventTrack[WrittenTime, ScoreFact] { kernel EventTrack[WrittenTime, ScoreFact] {
        let s = ${subject} in
        together { s; shift by 1/2 s; }
    } }
```

`${subject}` is one track antiquotation. It is instantiated in the quote's host environment and inserted as a typed
kernel-term hole. Kernel identifiers never capture host identifiers; alpha-renaming prevents capture among inserted
terms. The completed quote must close and type-check before it becomes a track. No raw payload escape exists.

Four rules a writer of quotes needs, and each one is the same rule the rest of the language already keeps:

- **A quote is commented the way the file around it is.** `//` and `/* */` are trivia inside a quote, and `%` is not:
  the quote is Musa source, so the host's comment syntax is the one that applies. The kernel's `%` lines belong to
  `.musa.kernel` documents, which are not written inside a piece.
- **A raw payload says what the material is, and nothing about where it goes.** It states no scope and no origin; both
  are supplied by the use, exactly as they are for any shared body, and a quote that spells either is refused. A key,
  meter, clef or tempo payload is refused for the same reason at one remove — those are structural declarations with
  scope authority, and "from here onward" has no unique meaning in a value used at several places (`00-semantics.md`
  §3).
- **The quotation locus is where a hole is *instantiated*, not where its facts land.** The two differ under `let`: a
  hole in a `let` value is instantiated once, at the `let`'s own locus, and each reference then places the finished
  facts wherever the term writes it. Every fact leaving a quote records that locus.
- **Raw `shift`, `scale` and `restrict` are operations on time.** They move occurrences; they do not rewrite payloads,
  because payloads are opaque to the kernel and arrive already transformed (`docs/rules/kernel/01-grammar.md`).
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

The clip is beat-fitted and follows tempo. The fixed-media cue is only a kernel point at the score position; its
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
| standalone kernel | closed term parsing | `≡kernel` |
| quote with antiquotation | typed substitution then closure | `≡kernel` after instantiation |
| swappable instruments/profiles | signature checking and profile realization | equal gesture type; sound equality not promised |
| expression hairpin | profile-generated `ControlKey::expression` curve | exact gesture equality |
| shared room | explicit mix-graph sends | frame equality modulo documented deterministic summation order |
| sampled instrument | sample-map implementation of signature | behavioral conformance, not waveform equality |
| beat-fitted loop | tempo-scheduled clip gesture | scheduled-lane equality |
| fixed-duration cue | onset conversion plus immutable seconds duration | scheduled-media equality |
