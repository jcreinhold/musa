# Taking a piece into a workstation

This lesson takes one piece across all three ways into Logic Pro or GarageBand — as files, as a live performance, and as
a plug-in — and shows what each one keeps and what each one drops. It follows on from
[Getting started](getting-started.md), and it assumes macOS, because that is where CoreMIDI and Audio Units are.

We want two parts, so there is something to route. `examples/counterpoint.musa` is the smallest piece in the repository
that has them:

```musa
piece "Counterpoint Study" {
    tempo quarter = 90;
    meter 4/4;
    key d minor;

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
}
```

Check it first, as always:

```bash
musa check examples/counterpoint.musa
```

## 1. Hand the workstation files

```bash
musa render examples/counterpoint.musa --to daw --profile logic -o "counterpoint for logic"
```

That folder is a **bundle**. Look inside it:

```text
counterpoint for logic/
    score.mid            the piece as written
    performance.mid      the piece as played
    score.musicxml       editable notation
    audio/mix.wav        the rendered master
    audio/parts/         one stem per part
    musa-manifest.json   what this is, and what it lost
    musa-origins.json    which note came from which source event
```

Open the manifest and find `losses`. Every bundle reports at least two, and they are worth reading now rather than
discovering later:

- **tuning** — a MIDI file states no tuning, so the workstation plays these key numbers at whatever its instruments are
  tuned to;
- **controller** — the MIDI carries notes, tempo, meter, and key, and no continuous controllers. Our dynamics live in
  `audio/mix.wav`, not in `performance.mid`.

Now import it. In Logic, drag in `score.musicxml` *or* `performance.mid` — one of them. Both would give you two copies
of one piece. Drag `audio/mix.wav` in beside it; it starts at frame zero and needs no offset.

Run the same export again into a second folder and compare the two with `diff -r`. They are identical, byte for byte.
Nothing in a bundle records when or where it was made, which is what makes a bundle a fact about the piece rather than a
fact about the afternoon.

## 2. Play it to the workstation, live

Ask first what would be sent. This touches nothing and needs no host:

```bash
musa midi plan examples/counterpoint.musa
```

It names one published port per part — `Musa — Counterpoint Study · violin` and `Musa — Counterpoint Study · viola` —
the channel each sounds on, how many messages the run holds, and what the projection could not carry. Then send it:

```bash
musa midi send examples/counterpoint.musa
```

In the workstation, arm one track per input and record. The performance that arrives is the same performance that is in
`performance.mid`: the same notes, the same velocities, the same order, at the same moments. That is the point of having
both — they are two deliveries of one reading, not two readings.

When it finishes, read the counts it prints: messages sent, sent late, dropped, refused. An attack more than 20 ms past
its moment is dropped rather than played late, because a late attack is wrong music.

## 3. Put Musa inside the workstation

Install the containing app, and check that the host can see both components:

```bash
auval -a | grep Musa
```

**The instrument.** Make a software-instrument track, choose **AU Instruments ▸ Musa ▸ Musa: Instrument**, point it at
`examples/counterpoint.musa`, and select the piece and the `violin` part. Play the host's keyboard: the notes reach that
part's instrument through the mapping the source declares.

Look at the plug-in's parameters. They are not a fixed catalogue — they are exactly the controls this source declares,
with the ranges the source gave them, and every control that could not become a host float is listed as a named loss
instead of being coerced into one. Write a little automation on one of them, save the session, close it, reopen it. The
automation still points at the same control, because the address it points at was saved in your document rather than
re-derived.

Nothing you did there changed `counterpoint.musa`. A parameter move is a performance, not an edit.

**The processor (Logic Pro).** On a software-instrument track's channel strip, open **MIDI FX ▸ Audio Units ▸ Musa ▸
Musa: Processor**, point it at the project, and choose the piece. Then make its two choices explicitly:

- reading: *score* or *performance*;
- timeline: *piece* — seconds on the piece's own exact schedule — or *host* — quarter notes on the host's timeline, so
  its tempo track moves the piece.

Press play in the host. The whole piece is scheduled onto its timeline; transport, tempo, cycle, and position stay the
host's throughout. Seek somewhere in the middle: it starts where you seeked to rather than replaying what you skipped,
and notes that were sounding across that point are re-entered.

And now do not import `performance.mid` into the same project. Two projections of one piece would duplicate every
message.

## What you have seen

Three crossings, one source. The bundle is permanent and one-way; live MIDI is the performance as it happens; the plug-
ins are the source itself, read inside the host. None of them writes `.musa` back, and each says out loud what its
format could not carry.

Next: [Import a bundle](../how-to/daw-bundle.md), [play live](../how-to/live-midi.md),
[share a transport](../how-to/sync-transport.md), and [use the Audio Units](../how-to/audio-unit.md) as tasks rather
than as a tour. [The DAW boundary](../concepts/daw-boundary.md) explains why it is shaped this way.
