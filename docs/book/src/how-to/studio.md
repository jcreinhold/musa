# Write for the studio

A piece can describe its own sound in a `studio` block. Patches are signal chains, modulation is a typed connection to a
named parameter, and the mix is written the way it is heard.

## A patch

```musa
studio {
    patch glass_pad {
        oscillator(sine)
            |> envelope(adsr(attack: 30 ms, decay: 1.8 s, sustain: 0.65, release: 3.5 s))
            |> lowpass(cutoff: 1400 Hz, resonance: 0.7)
            |> output;
    }
}
```

`|>` pipes one processor into the next. A patch ends at `output`.

## Units are syntax

Audio parameters require units: `1400 Hz`, `250 ms`, `-18 dB`, `0.08 Hz`. A bare `1400` is rejected, because it could
mean hertz, MIDI units, or a normalized control value.

## Modulation

Connect a signal to a named parameter:

```musa
lfo = oscillator(sine, frequency: 0.08 Hz) |> scale(250 Hz) |> bias(1400 Hz);
modulate lfo -> glass_pad.lowpass.cutoff;
```

The destination is a path through the patch — here, the cutoff of the `lowpass` stage of `glass_pad`.

## Assignment and routing

A patch is wiring; wiring no part connects to is a cable that ends in the air. Assign parts to patches, and route their
output:

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
