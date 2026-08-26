# Play a piece

`musa play` performs a piece through the audio engine:

```bash
musa play first.musa
musa play first.musa --loop
```

Playback runs to completion; Ctrl-C ends it early.

What you hear depends on the piece. Without a `studio` block, parts sound with a default instrument. With one, the
instruments, assignments, and routes the piece declares are the sound — see [Write for the studio](studio.md).

For live work with immediate feedback, the [desktop app](desktop.md) plays from the score with a transport and playhead.
The CLI player is for the terminal and for scripts.
