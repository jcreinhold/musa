# Shape a performance and its sound

The shortest route to useful playback is to keep three choices separate: the score states what is written, a performance
profile interprets its marks, and an instrument turns the resulting gestures into audio. This is why changing a sound
never rewrites pitches or voices.

## Give written marks a reading

A profile can read the same staccato differently for winds and strings. The settings below are from
`examples/profile-fixture.musa`:

```musa
profile strings {
    mark staccato {
        gate = 0.5;
        attack = 8 ms;
    }
    mark tenuto {
        gate = 1;
        attack = 30 ms;
    }
    mark accent {
        gate = 0.9;
    }
    dynamic p {
        amplitude = 0.35;
    }
    dynamic mf {
        amplitude = 0.6;
    }
    dynamic f {
        amplitude = 0.85;
    }
}
```

This profile is performance policy, not rewritten notation. Open Music Theory distinguishes a written articulation from
the instrument-specific act that realizes it (`007-other-aspects-of-notation.md`), and treats swing as straight notation
performed unevenly (`074-swing-rhythms.md`). Musa preserves that distinction through exact gesture controls.

## Choose or build an instrument

An instrument declaration states a public signature and hides its machine body. The editor shows the signature, exposed
musical controls, supported techniques, output channels, and origin first. The graph is a source declaration, not an
editor-owned preset:

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

Hover and completion for `oscillator`, `envelope`, and `lowpass` come from checked declarations in
`std::sound::catalogue`. Registered primitives report only private implementation support.

## Make a room and route part outputs

A room is a shared ambience path. A send copies a part output into it; routing the room to `master` returns that sound
to the final output. The part remains a score part, not a mixer track.

```musa
room hall {
    reverb(room: 0.82, damping: 0.55);
}
```

```musa
send violin -> hall at -18 dB;
send strings -> hall at -14 dB;
route hall -> master;
```

## Use sample banks and field recordings

An SFZ or SoundFont declaration is still an instrument. The project manifest supplies immutable asset policy and the
lock supplies its digest; `musa assets verify` checks both without fetching. Hover reports `sfz@1` or `sf2@1`, the exact
translation boundary, the named refusal classes, and the asset's current status. These adapters deliberately do not
promise compatibility with every player extension.

A beat-fitted clip and a fixed-media cue are different musical declarations even when they use the same WAV bytes. A
clip has performed-beat support and an explicit crop, loop, or rate fit. A fixed cue starts at an exact physical instant
and is unaffected by later tempo changes. The Mix workspace labels the distinction and never invents a waveform editor.

Online libraries enter only through an exact-pinned package and an explicit `musa fetch`. Checking, playback, and
rendering stay offline over the verified lock closure.

The generated [Studio vocabulary](../reference/studio-vocabulary.md) gives the source-owned processor and instrument
contracts and the registered foreign-format matrices. [Write for the studio](../how-to/studio.md) is the concise wiring
recipe.
