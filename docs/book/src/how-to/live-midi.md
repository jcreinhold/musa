# Play a piece to Logic Pro or GarageBand, live

A *bundle* hands a workstation files. Live MIDI hands it the performance as it happens: Musa publishes named MIDI
sources, and the workstation records from them the way it records from a keyboard.

Ask first what would be sent. This touches nothing:

```bash
musa midi plan first.musa
```

It prints one line per published port, one per part with the channel it sounds on, how many messages the run holds, and
anything the MIDI projection could not carry.

Then send it:

```bash
musa midi send first.musa
```

## Choosing what is sent

| Option | What it changes |
| --- | --- |
| `--mode performance` | The played reading — profiled gates and dynamic-derived velocity. The default |
| `--mode score` | The written reading — notated durations, one velocity |
| `--single-source` | One port carrying every part on its own channel, instead of one port each |
| `--to <endpoint>` | Send to a destination the host already offers, instead of publishing sources of Musa's own |

`musa midi endpoints` lists the destinations, each with the identifier `--to` takes.

## Receiving it in Logic Pro or GarageBand

Musa publishes one **source** per part, named for the piece and the part — `Musa — first · flute`. A workstation sees
each as an input, so arm one track per part and record them together. With `--single-source` there is one input instead,
and the parts arrive on their own MIDI channels.

Nothing here changes the music to suit a host. The performance sent live and the `performance.mid` in a bundle are the
same performance: the same notes, the same velocities, the same order, at the same moments.

## What live MIDI does not carry

The same things a MIDI file does not carry, and for the same reason — MIDI states note numbers:

- spelling, ties, voices, and beams are not in this projection at all;
- MIDI states no tuning, so a consumer plays these key numbers at whatever its instruments are tuned to;
- a piece whose parts play at their own speeds is placed at the moments it sounds, but the tempo stated is the piece's;
- a piece with more parts than MIDI has melodic channels shares channels, and the run says which part shares.

Every one of these is printed when it applies rather than left to be discovered.

## Timing

Musa's own transport is the clock: it does not follow a workstation's, and a workstation does not follow it. Each
message is handed to the port as it comes due, which places it within one scheduling window — four milliseconds — of its
moment. `musa midi send` prints how many messages went out, how many went out after their moment, how many were dropped,
and how many a port refused.

An attack whose moment is more than twenty milliseconds past is dropped rather than played late, because a late attack
is wrong music. A release always goes out, because a note left holding is worse than a late one. Stopping releases every
note the run left sounding and quiets every channel it used.

Live MIDI is macOS only: CoreMIDI is the only MIDI output macOS has, and no other platform Musa builds for publishes
one. `musa midi plan` works everywhere.
