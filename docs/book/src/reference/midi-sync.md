# Live MIDI and transport sync

What Musa publishes over CoreMIDI, and what it does when a clock is shared. The task-shaped pages are
[Play a piece to a workstation, live](../how-to/live-midi.md) and
[Share one transport with Logic Pro](../how-to/sync-transport.md); this is the surface those two use.

Live MIDI is macOS only. CoreMIDI is the only MIDI output macOS has, and no other platform Musa builds for publishes
one. `musa midi plan` runs everywhere, because it asks the piece and not the host.

## Commands

```text
musa midi endpoints                     every MIDI destination this host offers
musa midi sources                       every MIDI source this host offers
musa midi plan <file.musa>              what sending it would publish and play
musa midi send <file.musa>              play it to a workstation, live
musa midi follow <file.musa>            follow another transport until it stops
```

`endpoints` and `sources` print one `id<TAB>name` per line. The `id` is the stable identifier `--to` and `--from` take;
the name is what the host calls it and may change between reboots.

| Flag | Verbs | What it changes |
| --- | --- | --- |
| `--mode score \| performance` | `plan`, `send`, `follow` | which reading is sent. `performance` is the default |
| `--single-source` | `plan`, `send`, `follow` | one port carrying every part on its own channel, instead of one port per part |
| `--to <endpoint>` | `plan`, `send`, `follow` | send to a destination the host already offers instead of publishing sources |
| `--lead` | `send` | Musa owns the transport and sends MIDI clock |
| `--from <source>` | `follow` | the source whose transport Musa follows |
| `--protocol midi-clock \| mtc` | `follow` | how to read that source. `midi-clock` is the default |
| `--reference <part>` | `follow`, `send --lead` | which scope's tempo the one clock states, for a polytempo piece |

## Ports and channels

One **source** per part by default, named for the piece and the part — `Musa — first · flute`. A workstation sees each
as a separate input, so one armed track per part records them apart. With `--single-source` there is one port and the
parts arrive on their own MIDI channels.

Channels are assigned in part order over the fifteen melodic channels; channel 10 is percussion and is not handed out to
a melodic part. A piece with more parts than there are channels shares them, and the run names which parts share.

## Timing

Each message is handed to the port as it comes due, which places it within one scheduling window — four milliseconds —
of its moment.

| Situation | What happens |
| --- | --- |
| an attack more than 20 ms past its moment | dropped, and counted. A late attack is wrong music |
| a release past its moment | always sent. A note left holding is worse than a late one |
| stopping | every note the run left sounding is released, and every channel it used is quieted |
| a port refusing a packet | counted and reported, not retried into the next window |

`musa midi send` prints how many messages went out, how many went out late, how many were dropped, and how many a port
refused.

## Clock authority

A session has exactly one transport owner. `--lead` makes it Musa; `--from` makes it the named source; neither makes it
nobody, which is the default and means Musa simply plays. Asking for both is refused before anything opens:

| Refusal | When |
| --- | --- |
| `a session has one clock authority` | `--lead` and `--from` were both given |
| `Musa cannot send and follow <protocol>` | one protocol was asked to go both ways |
| `Musa does not send <protocol>` | a protocol Musa does not generate was named to send |
| `the clock authority cannot change while a run is in progress` | the authority was changed mid-run |
| `this piece is polytempo…` | a polytempo piece was synchronized without `--reference` |

Musa leads with MIDI clock only. Generating MTC would mean publishing a wall clock of Musa's own for another program to
slave to, which is a different feature from following one.

## What the protocols cannot carry

Stated when the session starts, not discovered by drift.

| Protocol | Carries | Does not carry |
| --- | --- | --- |
| `midi-clock` | 24 pulses to the quarter, start/continue/stop, a song position to the sixteenth | meter, key, and any exact rational position |
| `mtc` | `hh:mm:ss:ff` of the leader's own wall clock | tempo, meter, key, and any musical position at all |

Following MTC therefore places the piece in seconds and never in beats: a leader's tempo change moves the wall clock,
and Musa follows the wall clock. Following MIDI clock places it in quarters, and the song-position pointer resolves to
the sixteenth — a seek to a finer position lands on the sixteenth that contains it.

## What `musa midi follow` reports

```text
following midi-clock — MIDI clock carries no meter, no key, and no exact rational position: …
locked (status v1): 4183 observations, 0 dropouts
  resynchronizing when further than 30000 µs apart
  leader tempo about 120.02 bpm
  drift -412 µs, jitter 260 µs
```

| Field | Meaning |
| --- | --- |
| lock | `idle` — nothing heard; `acquiring` — arriving but not yet trusted; `locked` — steady and followed; `lost` — the leader stopped arriving and Musa stopped rather than free-running |
| observations | messages the follower admitted since the port opened |
| dropouts | times the leader went quiet for half a second while it was rolling |
| leader tempo | estimated over a 24-pulse window. Absent under MTC, which states no tempo |
| drift | signed microseconds from the rate the lock was taken at; positive means the leader is running late |
| jitter | the greatest departure of one pulse interval from that window's mean |

A lock is claimed after 24 steady pulses. Musa re-seeks rather than letting the difference stand once it is more than 30
ms from the leader; a resynchronization flushes and restarts, so it is audible, and it is worth doing only when the
alternative is worse.

## Polytempo

One MIDI clock states one tempo, and a piece whose parts play at their own speeds has no single one. Synchronizing such
a piece without `--reference` is refused by name. With it, the named scope's tempo is what the clock states, and every
other scope is reported as unsynchronized rather than quietly bent onto the reference's grid.

Sending a polytempo piece with no clock at all is not refused: every part is placed at the moment it sounds, and the
stated tempo is the piece's.

## What live MIDI does not carry

The same things a MIDI file does not, and for the same reason — MIDI states note numbers:

- spelling, ties, voices, and beams are not in this projection at all;
- MIDI states no tuning, so a consumer plays these key numbers at whatever its instruments are tuned to;
- a piece's channel sharing, when it has more parts than channels.

Each of these is printed when it applies. The vocabulary the printed losses use is [Losses](losses.md); the kinds that
reach this projection are `notation`, `tuning`, `channel`, `polytempo`, and `controller`.
