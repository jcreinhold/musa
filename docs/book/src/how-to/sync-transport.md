# Share one transport with Logic Pro

Live MIDI sends the notes. This is the other question: whose *transport* is in charge — who owns play, stop, and where
in the piece "here" is.

A session has exactly one clock authority. Either Musa leads and a device follows its clock, or one external source
leads and Musa follows. There is no both: two transports correcting each other is drift with extra steps, so a
configuration that names two is refused before anything opens.

## What Logic actually does

Apple's guide, [Sync multiple MIDI devices to Logic Pro for Mac](https://support.apple.com/en-us/102005), documents the
two directions separately, and they are not symmetric:

| Direction | What Apple documents |
| --- | --- |
| Logic **transmits** | MIDI clock, MIDI Timecode (MTC), and MIDI Machine Control (MMC), to each device independently |
| Logic **receives** | MTC — "when you need to sync Logic to video or to another digital audio workstation" |

Apple documents no way to make Logic follow an incoming MIDI clock. So:

- **Logic leads, Musa follows** is the supported pairing, over **MTC**.
- **Musa leads** sends MIDI clock with a song position and start/stop. Hardware sequencers, drum machines, and other
  programs that slave to MIDI clock follow it. Logic is not documented as one of them, and this page does not claim it
  is.
- Musa does not send MTC, and does not send or answer MMC.

Nothing here is claimed about GarageBand: Apple's current GarageBand guide documents no synchronization settings, so
there is no mode for it.

## Following Logic

List the inputs and pick the one Logic is sending on:

```bash
musa midi sources
```

Then follow it:

```bash
musa midi follow first.musa --from <source> --protocol mtc
```

In Logic, open the synchronization settings and enable transmitting MTC to that destination. Press play there; Musa's
transport follows. It never corrects Logic — the leader owns the position, and Musa moves to it.

When it stops, Musa prints what it measured: the lock state, how many observations arrived, how many dropouts there
were, the leader's estimated tempo where the protocol carries one, and the drift and jitter.

To follow a device that sends MIDI clock instead, use `--protocol midi-clock`. Start, continue, stop, and the song
position pointer are all followed.

## Leading

```bash
musa midi send first.musa --lead
```

This publishes one further port — `Musa — first · clock` — carrying only the transport stream: a song position, a start,
twenty-four pulses to the quarter, and a stop. The notes stay on exactly the ports they were on without it. Point the
receiving device at the clock port and at whichever note ports it should hear.

`musa midi plan first.musa --lead` shows what would be published, with no host involved.

## What the protocols cannot say

Stated here rather than discovered by drift:

- **MIDI clock** carries no meter, no key, and no exact rational position. It counts twenty-four pulses to a quarter,
  and a song position to the sixteenth.
- **MTC** carries physical frames of the leader's wall clock. It carries no tempo, no meter, no key, and no musical
  position at all.

The two are converted by two different laws, and neither becomes the other. A clock position is exact quarters, and the
piece's own tempo map says which second that is. An MTC position is already seconds, and stays physical: it is a fact
about the leader's clock and never enters a score value.

### Polytempo

One clock lane states one tempo. A piece whose parts run at their own speeds therefore has to say which one is being
synchronized:

```bash
musa midi send canon-x.musa --lead --reference rising
```

Without `--reference`, a polytempo piece is refused rather than flattened. With it, the clock states that scope's tempo
and every other scope is reported as unsynchronized. Polymeter needs no such choice: the shared beat clock and each
part's own meter stay distinct.

## Resynchronizing

While following, Musa compares where the leader says it is to where Musa is. Below thirty milliseconds the difference
stands and the measured drift reports it; past that, Musa re-seeks. Under MTC, a reading that lands where the previous
one predicted is a confirmation and not an instruction, so a stream arriving four times a second does not stutter.

A seek or a loop boundary is a discontinuity: the live run is stopped — which releases every note it left sounding and
quiets every channel it used — and then restarted from the new boundary. A leader that stops sending for half a second
is *lost*, and Musa stops rather than free-running.

## Measured timing

Musa's clock leader and follower, in a `CoreMIDI` loopback on one machine — Musa publishing its own clock port and
following it back:

| Host | Measurement |
| --- | --- |
| Apple M4 Pro, macOS 26.6.2 (build 25G83) | Over 97 pulses at a quarter of 120: estimated tempo 119.8–120.3 bpm, jitter 0.9–2.7 ms, accumulated drift 4–10 ms, no queue overflow |

The drift is Musa's own send window rather than the follower's arithmetic: each message is handed to its port as it
comes due, within one four-millisecond scheduling window of its moment, and the follower measures where they actually
landed. There is no claim of sample accuracy across `CoreMIDI` clients — for sample-accurate rendering inside a host,
the Audio Unit path is the one to wait for.

Following and leading are both macOS-only for the same reason live MIDI is: `CoreMIDI` is the only MIDI the platform
has. `musa midi plan --lead` works everywhere.
