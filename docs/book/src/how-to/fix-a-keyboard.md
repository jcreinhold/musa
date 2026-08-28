# Fix a keyboard or latency problem

What the top margin says is the diagnosis. Each line below is a state the app reports rather than a symptom you have to
infer.

## The app cannot see the keyboard

**No MIDI keyboard — choose an input device.** A picker sits beside the message; choose the port. Source editing,
playback, and export all keep working without a keyboard.

If the port is not listed, the operating system is not offering it: reconnect the device, then reopen the picker. Which
input is selected is application state, not part of the piece — it does not travel with the file and is not a source
edit.

## The keyboard is listed but silent

The audition instrument is the *selected part's* declared instrument. Click a note in the part you mean, so the caret is
in it; the margin then reads, for example, *Concert grand — KeyLab 61*.

If the source does not currently compile, audition falls back to the last valid prepared instrument and the app marks
the score as stale. Fix the diagnostics and it recovers on the next valid revision.

Velocity, pressure, bend, and pedals shape audition only through controls the source declares. Input the piece does not
declare is reported in the device details rather than silently remapped into something else.

## The keyboard disappeared mid-take

**Keyboard disconnected — capture preserved.** Held notes are ended safely, and the take survives the disconnection:

- if the note and pedal state is explainable, the take can still be reviewed;
- if it is not, the take is kept for inspection and transcription is refused rather than guessed.

Reconnecting does not steal focus, move your selection, or start a capture.

## It feels late

Musa measures the paths it owns and does not estimate the ones it does not. On an Apple M4 Pro, decoding one MIDI event
and handing it to the audio thread takes about 3 ns, and one audition event plus a 128-frame render block about 6.4 µs —
roughly a quarter of one percent of the 2.67 ms that block represents at 48 kHz.

What that leaves is everything outside the app: key scan, USB or Bluetooth transport, the operating system's MIDI
delivery, the audio device's buffer, and the converter. If playing feels late, that is where to look first — lower the
audio buffer in the system settings, prefer USB to Bluetooth for the keyboard, and check whether another application
holds the audio device.

Latency does not damage a take. Timestamps are calibrated when the take is recorded, so a phrase captured through a
laggy path is transcribed at the times you played, not the times the app happened to hear them.

## Something else refused

Every refusal in this workflow names its reason and what would fix it — a take past its bounds, a phrase that cannot be
written exactly, a piece that has changed since the take. [Capture and
transcription](../reference/capture-and-transcription.md) lists them.
