# The language

A `.musa` file is one piece or one library. This page surveys the surface syntax; `examples/` in the repository holds
a runnable fixture for every construct named here.

## A piece

```musa
piece "Glass Mountain" {
    subtitle "for violin and strings";
    composer "musa";

    tempo 1/4 = 72;
    meter 4/4;
    key a minor;

    score { /* parts and voices */ }
    performance { /* interpretation profiles */ }
    studio { /* patches, routing */ }
}
```

The header statements mean "from the beginning". `score` holds the music; `performance` and `studio` are optional and
say how it sounds.

## Notes, rests, chords

```musa
c4/4        // middle C, a quarter note
g#4/2       // G-sharp above, a half note
rest/1      // a rest
[a3 c4 e4]/2   // a chord, explicit
```

Durations are exact fractions of a whole note: `1` whole, `1/2` half, `3/8` dotted quarter, `1/12` triplet eighth.
Events are self-delimiting and take no semicolon.

A grace note has no written duration — it is a point in time, and the language says so by giving it none:

```musa
grace { c#6 }
grace { g5 staccato }
```

## Voices, parts, bars

A `part` holds one player's staff; a `voice` holds one line inside it. Parallelism is always by named voices, never
inferred from cursor position.

```musa
part violin {
    clef treble;
    voice lead {
        | c5/4 e5/4 g5/4 e5/4      // a bar: asserts its contents fill one measure
        bar head { a4/2 c5/2 }     // a named bar: sounds here, and binds `head`
        use head;                  // plays it again, anywhere later
    }
}
```

A bar means nothing — its contents elaborate exactly as they would without it. What it buys is the assertion: a
dropped duration would otherwise move every later barline, silently.

## Context changes

`meter`, `key`, and `clef` are voice items that take effect where they are written:

- A `meter` or `key` change is the piece's, and must land on a barline.
- A `clef` change is the part's own, and need not land on a barline.
- None may be written inside a motif: a motif stands at several places, and a context change is an absolute position.

## Repeats and endings

```musa
repeat 2 {
    | d5/4 g5/4 f#5/4 g5/4
    ending 1 { | a5/2 d5/2 }
    ending 2 { | g5/1 }
}
```

The page prints the body once between repeat barlines; the performance plays every pass. Endings come last, numbered
from 1. A repeat folds on the page only when every voice sounding under it writes the same one.

## Material and transforms

```musa
motif sigh(root: pitch = e5) {
    root/2
    rest/4
    c5/2
}

fragment turn { c5/8 d5/8 c5/8 }

voice lead {
    use sigh();
    transpose down P5 { use sigh(); }
}
```

`motif` takes parameters and is reused by `use`; `fragment` is named material without parameters. The transforms
include `transpose`, `stretch`, `retrograde`, `invert`, and `in scale c dorian { ... }` for scale-local stepwise
motion. Recursion is rejected: every piece compiles to a finite score.

## Annotation

Nothing in the annotation layer is interpreted. Chord symbols are recorded and printed; sections and phrases name the
structure a reader already hears.

```musa
score {
    section "Exposition" at 1:1;
    harmony {
        at 1:1 am;
        at 3:1 e7;
    }
    part piano {
        voice lead {
            phrase "antecedent" { a4/4 c5/4 e5/2 }
            dynamic mf;
            crescendo to f { c5/4 d5/4 e5/2 }
            mark pedal { a2/1 f2/1 }
        }
    }
}
```

## Values and functions

Bindings and functions are typed, and live at the file's root or inside blocks:

```musa
let fifth: interval = P5;

fn third(root: pitch) -> pitch = root up M3;

fn transpose_answer(subject: music, by: interval) -> music =
    transpose(by, subject);
```

The primitive types include `bool`, `nat`, `ratio`, `duration`, `pitch`, `interval`, `scale`, `key`, `chord_class`,
`voicing`, `row12`, and `music`, with `option[...]`, `list[...]`, products, and arrows as constructors. `match` is the
case-analysis spelling. A multi-statement musical body is explicitly `music { ... }`; `use e;` instantiates a `music`
value at the current cursor.

## Chords and voicings

A chord class is content; a voicing is a realization of it. The two are separate types on purpose.

```musa
let sonority: chord_class = chord c major7;   // spells pitch classes; does not sound
let close: option[voicing] = close_position(sonority, c4);

stack c4 major7/2    // sugar: close position, sounded, register fixed by the written root
```

`chord` derives no notes. Only `play` (or the `stack` sugar) creates sounded music, and a voicing policy returns
`option[voicing]` — absent when its preconditions do not hold.

## Performance profiles

The score says what is written; a profile says what an instrument makes of it.

```musa
performance {
    profile strings {
        mark staccato { gate = 0.5; attack = 8 ms; }
        dynamic p { amplitude = 0.35; }
    }
}
```

Two parts can play the same marks and read them differently: a staccato is not a number until an instrument says so.

## Studio

Patches, modulation, assignment, and routing are covered in [Write for the studio](../how-to/studio.md).

## Imports

```musa
import std::pitch;
import "lib/my-matters.musa";
```

A quoted path resolves relative to the importing file. `std::` names the bundled
[standard library](stdlib.md); it is reserved, never searched on disk, and there is no implicit prelude.
