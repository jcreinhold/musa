# Write for the studio

A piece can describe its own sound in ordinary Musa source. An instrument has a public signature and a private machine
implementation; modulation is a typed connection inside that implementation, and the mix is written the way it is heard.

## An instrument

```musa
instrument glass_pad conforms note_instrument {
    implementation graph {
        carrier = oscillator(sine);
        shimmer = oscillator(sine, ratio: 2) |> gain(-15 dB);

        mix(carrier, shimmer)
            |> envelope(adsr(attack: 30 ms, decay: 1.8 s, sustain: 0.65, release: 3.5 s))
            |> lowpass(cutoff: 1400 Hz, resonance: 0.7)
            |> output;
    }
}
```

`|>` pipes one processor into the next. An instrument implementation ends at `output`. The previous-edition `patch`
spelling is accepted only through its documented migration window and receives an exact source fix.

## Units are syntax

Audio parameters require units: `1400 Hz`, `250 ms`, `-18 dB`, `0.08 Hz`. A bare `1400` is rejected, because it could
mean hertz, MIDI units, or a normalized control value.

## Modulation

Connect a signal to a named parameter:

```musa
lfo = oscillator(sine, frequency: 0.08 Hz) |> scale(250 Hz) |> bias(1400 Hz);
modulate lfo -> glass_pad.lowpass.cutoff;
```

The destination is a private path through the implementation — here, the cutoff of the `lowpass` stage of `glass_pad`.
It is not an exposed musical control. Exposed controls and techniques are declared by the instrument signature and
mapped to private paths in source.

## Assignment and routing

An instrument no part selects is a cable that ends in the air. The concise form inside a part chooses an instrument and
performance profile together. The expert form assigns part outputs and routes them:

```musa
assign violin -> glass_pad;
route violin -> master;
```

A `bus` holds shared processing, fed by sends:

```musa
bus hall {
    reverb(room: 0.82, damping: 0.55);
}

send violin -> hall at -18 dB;
route hall -> master;
```

Every stage runs real DSP. `musa play` performs through this graph live; `musa render --to wav` renders it offline.

## Sampled instruments and media

`instrument name from "…" conforms note_instrument;` adapts a verified SFZ or SoundFont asset into the same checked
sample-map contract as a native Musa declaration. The project manifest states kind, versioned adapter, bounds, license,
and attribution; `musa.lock` states exact bytes and digest. Use `musa assets list`, `musa assets verify`, and the
explicit `musa fetch` operation rather than relying on ambient files or network access.

A `clip` occupies performed musical time and names `crop`, `loop`, or `rate` fitting. A `fixed_media` cue begins at
exact physical time. Tempo changes affect the former and never move the latter. Neither declaration is a
waveform-editing surface.

The generated [Studio vocabulary](../reference/studio-vocabulary.md) lists processor contracts, exposed controls,
standard instruments, and the exact `sfz@1`/`sf2@1` support and refusal summaries.
