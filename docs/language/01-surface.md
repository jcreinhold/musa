# Surface language candidate

This file settles the punctuation and spellings introduced by the candidate. Existing syntax remains unless a rule
below explicitly desugars it. Braces delimit blocks. Added bindings, calls-as-statements, and declarations end in `;`;
commas separate arguments; `=` introduces an expression body or binding. Existing note, rest, and chord events remain
self-delimiting and do not take `;`. No added production is newline-sensitive.

## 1. Added grammar

The normative schematic grammar is:

```ebnf
type         := primitive | "option" "[" type "]" | "list" "[" type "]"
              | "(" type ")" | "(" type "," type ("," type)* ")" | type "->" type
binding      := "let" IDENT ":" type "=" expr ";"
function     := "fn" IDENT "(" params? ")" "->" type "=" expr ";"
param        := IDENT ":" type ("=" expr)?
call         := expr "(" args? ")"
expr         := literal | IDENT | path | "(" expr ")" | product | list | option
              | call | match | music-expr
product      := "(" expr "," expr ("," expr)* ")"
list         := "[" (expr ("," expr)*)? "]"
option       := "none" | "some" "(" expr ")"
match        := "match" expr "{" match-arm ("," match-arm)* ","? "}"
match-arm    := pattern "->" expr
pattern      := "_" | literal | IDENT | "none" | "some" "(" IDENT ")"
              | "[" "]" | "[" IDENT "," ".." IDENT "]"
              | "(" IDENT "," IDENT ("," IDENT)* ")"
music-expr   := "music" "{" music-statement* "}"
music-use    := "use" expr ";"
scale-local  := "in" "scale" expr "{" music-statement* "}"
assertion    := "assert" IDENT "(" args? ")" "{" music-statement* "}"
analysis     := "analysis" IDENT "=" expr ";"
kernel-quote := "kernel" "Timeline" "[" "ScoreFact" "]" "{" kernel-item* "}"
antiquote    := "${" expr "}"
document     := (import | binding | function | signature | module | template | instance)*
                (piece | library | instance)
signature    := "signature" IDENT "{" member* "}"
member       := "let" IDENT ":" type ";"
module       := "module" IDENT params? ":" IDENT "{" (binding | function)* "}"
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
import       := "use" (STRING | "std" "::" IDENT) ";"
```

Function arrows associate right; call binds tighter than pitch operators; pitch operators bind as follows, tightest
first: parentheses, `step`, `up`/`down`. `root up M2 down m2` is rejected as ambiguous; write parentheses. Every `fn`
has an expression body. A multi-statement musical body is explicitly `music { ... }`.

Named intervals use conventional `P`, `M`, `m`, and repeated `A`/`d` qualities. Because lowercase `d4` already means
the written pitch D4, a singly diminished fourth is written `dim4`; `dd4` and `ddd4` remain the compact multiply
diminished spellings.

`match` is the sole added case-analysis spelling. Arms are comma-separated and a final comma is accepted; braces and
arrows keep the alternatives legible when an arm's expression spans lines. The initial patterns cover booleans,
naturals and other literal domains, options, empty/cons lists, and products. A bare identifier binds the whole value;
`_` discards it. Prompt 96 defines exhaustiveness and rejects duplicate or unreachable arms. It also owns the
constructor meaning of `[head, ..tail]`; `..` is two adjacent `.` tokens, not a new general range operator.

Structural folds do not add syntax. `nat_fold(zero, step, count)`, `list_fold(zero, step, values)`, and
`option_fold(zero, some_case, value)` are ordinary calls to compiler-owned total primitives. Their step arguments are
named functions because this candidate deliberately has no anonymous-lambda surface. This gives musicians one call
notation to learn and leaves `repeat n { body }` as the notation-facing fold over `music`.

The primitive value types added here are `bool`, `nat`, `ratio`, `duration`, `pitch`, `interval`, `spelled_pc`, `pc12`,
`scale`, `key`, `degree`, `chord_class`, `triad`, `voicing`, `row12`, `analysis[A]`, and `music`. Products, options,
lists, and arrows are the constructors described in `02-core-calculus.md`. Declaration kinds are not types.

The core literals introduced here are `true`, `false`, nonnegative decimal naturals, exact rational literals,
products, finite lists, and `some`/`none`. Existing pitch and interval literals are also expression atoms. Strings and
floating-point values remain syntax of their owning declaration domains rather than core values.

A pitch-name literal is checked in its expected domain: `chord c# minor` supplies `spelled_pc`, while an argument to
`row12` supplies `pc12`. Outside such an expected constructor position, write a type annotation. Converting an existing
`spelled_pc` value to `pc12` requires `forget_spelling`; there is no implicit value coercion in the opposite direction.

`control-domain`, `quantity`, and `range` use the exact unit grammar shared with studio values. `path` is a qualified
identifier such as `std.sound.basic_sine` or `bow.pressure`. `notation-selector` is one documented dynamic,
articulation, span/grouping mark, pedal, or technique pattern; it is not an arbitrary graph path. The sound forms are
staged and desugared by `08-performance-and-sound.md`, not values in the core calculus.

Imports are explicit. A quoted path is resolved lexically relative to the importing file. `std::core`, `std::list`,
`std::option`, `std::pitch`, `std::scale`, `std::collections`, `std::harmony`, and `std::voicing` name
version-matched source libraries bundled with Musa; `std` is reserved, never searched in the
working directory or environment, and has no implicit prelude. Imported definitions use the current flat value
namespace. Their source remains available at stable `musa-stdlib:/std/…` URIs for hover and go-to-definition, but is
read-only; a musician customizes one by writing a local wrapper. The authoritative signatures and prose are generated
from source comments in `stdlib/reference.md`.

## 2. Functions and music

```musa
let fifth: interval = P5;

fn third(root: pitch) -> pitch = root up M3;

fn transpose_answer(subject: music, by: interval) -> music =
    transpose(by, subject);

motif turn(root: pitch = c5) {
    root/8
    (root up M2)/8
    ((root up M2) down m2)/8
    root/8
}
```

`motif turn(...) { body }` desugars to a named `fn turn(...) -> music = music { body };` with a `Motif` role retained
for lints, extraction, editing, and Origin. `fragment name { body }` desugars to `let name: music = music { body };`
with a `Fragment` role. `use e;` checks `e : music`, instantiates it at the current cursor, and sequences it. Existing
`use name(args);` is the same rule, not a second invocation mechanism.

`music` values are contextual rather than captured timelines:

```musa
fn figure() -> music = music {
    c5/8
    (c5 step 1)/8
    (c5 step 2)/4
};

let subject: music = figure();
in scale c major { use subject; }
in scale c dorian { use subject; }
```

The two uses differ under `≈facts`; saving `subject` does not freeze the scale. `in scale` is lexical and emits no key
fact. An absent scale makes `step` a type-context diagnostic, not an implicit C-major choice.

## 3. Higher-order construction with controlled traversal

```musa
fn canon(subject: music, answer: music -> music, gap: duration) -> music = music {
    use overlay(subject, shift(gap, answer(subject)));
};

fn harmonize(subject: music, answer_pitch: pitch -> pitch) -> music = music {
    use overlay(subject, map_note_pitches(answer_pitch, subject));
};

use canon(theme(), transpose(P5), 1/2);
```

`map_note_pitches` is the sole initial user-facing traversal of `music`. It changes pitches in note and sounded-chord
events; it preserves time, annotations, marks, scope, and Origin; it does not traverse key signatures or chord-symbol
analysis. No iterator exposes a `ScoreFact` or kernel occurrence.

## 4. Assertions and analyses

```musa
assert fits_scale(scale c major) {
    c5/4 e5/4 g5/2
}

assert fits_scale(scale c major) {
    c5/4 fs5/4 g5/2
}
```

The first succeeds and returns the body as music. The second is a compile error at `fs5`, with the predicate's witness
and the enclosing assertion in the diagnostic. `assert p(args) { body }` desugars to
`checked(p(args), music { body })`; `p` must be a constructor invariant or decidable assertion returning a structured
witness, not an interpretive analysis.

Interpretation is named and non-blocking:

```musa
analysis harmony = roman_numerals(chorale(), in: key c major);
```

This produces `analysis[roman_numeral]`; it neither changes nor validates the score unless an explicit assertion reads
a decidable property of the result.

## 5. Chords, rows, and explicit register

```musa
let sonority: chord_class = chord c major7;
let close: option[voicing] = close_position(sonority, c4);
let open: option[voicing] = drop_position(sonority, c3, 2);

fn sound(chosen: voicing) -> music = play(chosen, 1/2);
fn sounded(chosen: option[voicing]) -> music = option_fold(music { rest/2 }, sound, chosen);
let close_bar: music = sounded(close);
let open_bar: music = sounded(open);

use close_bar;
use open_bar;

stack c4 major7/2

let row: row12 = row12(c, cs, e, d, fs, f, as, g, gs, b, a, ds);
let symmetric: row12 = row12(c, fs, d, gs, e, as, f, b, g, cs, a, ds);
let matrix: list[list[pc12]] = row_matrix(symmetric, convention: zero_based);
```

`chord` does not sound: a chord class is rooted spelled content with no register, spacing, doubling, or bass. A
voicing policy is an ordinary named function that selects those and returns `option[voicing]`, absent when its
preconditions do not hold — a bass the class does not contain, or a register the written range cannot reach. `play`
alone creates sounded music. `stack <pitch> <quality>/<duration>` is sugar for the close-position policy with the
absolute root fixing register; `stack c major7/2` is rejected, because a pitch class chooses no register. `row12` statically requires each `pc12` exactly once; symmetry may make fewer than
48 distinct `P`/`I`/`R`/`RI` forms, which is a result, not an error. Row-form naming always states a convention.

## 6. Declaration templates

```musa
fn theme() -> music = music {
    c4/4
    d4/4
};

template voice answer(subject: music, transform: music -> music) {
    use transform(subject);
}

template piece study(k: key, mode: scale, subject: music) "Study" {
    key k;
    score {
        part piano {
            voice right { in scale mode { use subject; } }
            make answer(subject, transpose(P8)) as follower;
        }
    }
}

make study(key g major, scale g mixolydian, theme()) as study_in_g;
```

`template` and `make` are structural syntax, not expressions. The `as` name is mandatory and participates in stable
generative identity. `piece`, `voice`, or `module` with parameters but without `template` is rejected.

A file is still one piece or one library, so an instance is placed by the kind it makes: a `make` of a piece template
stands at the file root and *is* that file's piece, and a `make` of a voice template stands among a part's voices.
Whatever precedes the file's piece or library — imports, bindings, functions, templates — is the file's lexical root. A
template body reads that root and its own parameters and nothing from the site that instantiates it; the arguments at a
site are evaluated in the site's own scope, which is why `subject` above can be passed on from `study` to `answer`.

## 6.1 Signatures and static modules

A signature names what a bundle of values must provide; a module provides them; a `template module` is a functor from
modules to a module.

```musa
signature TonalContext {
    let tonic: key;
    let collection: scale;
    let spell: degree -> option[pitch];
}

module CMajor: TonalContext {
    let tonic: key = key c major;
    let collection: scale = scale c ionian;
    let spell: degree -> option[pitch] = degree_in_c;
}

template module Sequences(C: TonalContext, gap: duration): TonalContext {
    let tonic: key = C.tonic;
    let collection: scale = C.collection;
    let spell: degree -> option[pitch] = C.spell;
    let delay: duration = gap;
}

make Sequences(CMajor, 1/2) as CSequences;
```

A signature member is a `let` without its definition: a name and the type the module must give it. Matching is by name
and exact type — a missing member and a member of the wrong type are both errors, each labelled at the signature and at
the module. A member the signature does not mention stays private to the module: it is what the module's own definitions
may use and what nothing outside may name.

Members are read as `Module.member`. Inside a module, a sibling member is read by its bare name. There is no module
value, no module argument to a function, no unpacking, and no recursion: `module`, `signature`, and `template module`
are structural syntax that has finished before any value exists.

## 7. Kernel documents and quotation

A standalone `.musa.kernel` file contains exactly one closed term in the grammar of `docs/kernel/10-term-calculus.md`:

```text
% musa-kernel-1
kernel "example" {
  composition main : Timeline[ScoreFact] =
    timeline 1/2 {
      occurrence "voice 0 0 note c4 1/2 [0:4]" from 0 to 1/2;
    };
}
```

It has no imports, functions, surface pitch operations, or free variables. Its payload text must decode as
`ScoreFact`.

A local quote is host syntax containing kernel syntax and typed antiquotation:

```musa
fn delayed_double(subject: music) -> music =
    kernel Timeline[ScoreFact] {
        let s = ${subject} in
        overlay { s; shift by 1/2 s; }
    };
```

`${subject}` is one `music` antiquotation. It is instantiated in the quote's host environment and inserted as a typed
kernel-term hole. Kernel identifiers never capture host identifiers; alpha-renaming prevents capture among inserted
terms. The completed quote must close and type-check before it becomes `music`. No raw payload escape exists.

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

Within a part, `sound instrument using profile;` is the ordinary one-action form. It desugars to the independent
profile selection, part-to-instrument assignment, and part-output-to-master route below. The expert surface may spell
those facts separately and add sends:

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
recorded duration remains seconds and is never manufactured into a musical extent.

## 9. Corpus correctness relation

| Corpus case | Desugaring | Required equality |
| --- | --- | --- |
| root-dependent turn | motif-role function plus interval action | `≈music` |
| major/dorian rebinding | Reader-style `in_scale` | `≈music` per environment; uses differ under `≈facts` |
| canon | `overlay(subject, shift(gap, answer(subject)))` | `≈music` |
| harmonizer | controlled pitch traversal | `≈music` |
| key-parameterized piece / parameterized voice | declaration-template expansion | full facts retain distinct instance Origin; `≈facts` after erasure |
| one chord class, two voicings | `play(voice(...))` | intentionally unequal under `≈facts` |
| generic/symmetric row | finite row constructor and transforms | value equality; distinct-form count is observed |
| assertion | `checked(predicate, body)` | successful body `≈music`; failure has no value |
| standalone kernel | closed term parsing | `≡kernel` |
| quote with antiquotation | typed substitution then closure | `≡kernel` after instantiation |
| swappable instruments/profiles | signature checking and profile realization | equal gesture type; sound equality not promised |
| expression hairpin | profile-generated `ControlKey::expression` curve | exact gesture equality |
| shared room | explicit mix-graph sends | signal equality modulo documented deterministic summation order |
| sampled instrument | sample-map implementation of signature | behavioral conformance, not waveform equality |
| beat-fitted loop | tempo-scheduled clip gesture | scheduled-lane equality |
| fixed-duration cue | onset conversion plus immutable seconds duration | scheduled-media equality |
