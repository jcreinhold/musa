# First pieces

Everything in this chapter is ordinary notation written down. No functions, no types, no library.

## 1. A piece is a file

`examples/twinkle.musa` is complete:

```musa
piece "twinkle" {
    composer "traditional";
    arranger "musa";

    tempo 1/4 = 104;
    meter 4/4;
    key c major;

    score {
        part piano {
            voice melody {
                | c4/4 c4/4 g4/4 g4/4
                | a4/4 a4/4 g4/2
                | f4/4 f4/4 e4/4 e4/4
                | d4/4 d4/4 c4/2
                | rest/1
            }
        }
    }
}
```

Read it outward. The `piece` names the work and the title is the string. Above the score sit the facts that hold for the
whole thing: who wrote it, how fast, what meter, what key signature. Inside `score` are `part`s — one performer or one
instrument each — and inside a part are `voice`s, which are lines of music, not channels of sound.

A note is a pitch and a duration: `c4/4` is middle C for a quarter note, `g4/2` for a half, `d5/8` for an eighth. The
number after the letter is the octave in scientific pitch notation, so `c4` is middle C and `c5` is the octave above.
`rest/1` is a whole-note rest. The `|` is a barline you write for your own eyes; the compiler already knows where the
bars fall from the meter, and it will tell you when a bar does not add up.

Check it and render it:

```sh
musa check examples/twinkle.musa
musa render examples/twinkle.musa --to lilypond
```

## 2. Accidentals, marks, and dynamics

Sharps and flats are written after the letter: `g#4`, `bb4`, `f##3`. A spelling is a decision Musa keeps — `g#4` and
`ab4` are two different notes that happen to sound alike, and nothing in the compiler will quietly turn one into the
other ([06 §1](06-distinctions.md#1-written-pitch-is-not-sounding-pitch)).

From `examples/chant.musa`:

```musa
voice line {
    dynamic mp;

    // *Salve Regina*, the opening phrase, in modern note values.
    d4/4
    f4/4
    g4/4
    a4/2
```

`meter none;` in that piece is worth knowing about early. It is not the absence of a meter statement — a piece that
writes no `meter` is in 4/4. It is a meter whose answer to "where do the barlines fall" is "nowhere", which is what
unmeasured chant needs. The durations are still exact and the performance still schedules them exactly; what stops is
the barline, not the clock.

A mark spans the music inside it. In `examples/glass-mountain.musa`:

```musa
voice bass {
    // Under the pedal for the whole descent: the harmony is a
    // wash rather than four separate chords, and the pedal is
    // where a page says so.
    mark pedal {
        a2/1
        f2/1
        d2/1
        e2/1
    }
}
```

## 3. Writing a phrase once

A `motif` is a phrase you name so you can use it more than once. From `examples/glass-mountain.musa`:

```musa
motif sigh(root: Pitch = e5) {
    root/2
    rest/4
    c5/2
    b4/4
    a4/2
}
```

and then, in a voice:

```musa
voice lead {
    use sigh();

    transpose down P5 {
        use sigh();
    }
}
```

Three things are happening, and they are worth separating.

`use sigh();` places the motif. The motif is a *declaration*; each `use` is an *occurrence*. They are not the same
object, which is why the editor can show you both the phrase you wrote and every place it landed, and why editing the
declaration changes every occurrence at once.

`root: Pitch = e5` is a parameter with a default. `use sigh();` takes the default; `use sigh(g5);` would not. The motif
is written once and is not fixed to one pitch.

`transpose down P5 { ... }` moves what is inside it by a written perfect fifth. `P5` is an interval, spelled: a perfect
fifth, not seven semitones. Transposing `c5` down a `P5` gives `f4`, and transposing it down an augmented fourth would
give `f#4` — different notes, and the difference is the whole reason intervals are spelled.

## 4. More than one line at once

Parts and voices are both ways of having several things sound together, and they mean different things.

A **part** is a performer: it gets its own staff, its own clef, and its own name on the page. A **voice** is a line
within a part. Two voices in one part share a staff; two parts do not.

From `examples/counterpoint.musa`:

```musa
score {
    part violin {
        clef treble;

        voice lead {
            | d4/2 f4/2
            | e4/2 [d4 f#4 a4]/2 fermata
        }
    }

    part viola {
        clef alto;

        voice line {
            | d3/2 a2/2
            | bb2/2 a2/2 fermata
        }
    }
}
```

`[d4 f#4 a4]/2` is a chord written out: three pitches, one duration, sounding together in one voice. `fermata` is an
articulation attached to the event before it.

## 5. Where to go next

- Degrees, collections, and harmony you construct rather than type out: [02](02-keys-degrees-and-chords.md).
- The same phrase in several keys, or the same piece made twice: [03](03-when-you-need-a-name.md).
- Writing down a claim about a passage and having the compiler check it: [04](04-claims-and-readings.md).
- A specific task, quickly: [05](05-cookbook.md).
