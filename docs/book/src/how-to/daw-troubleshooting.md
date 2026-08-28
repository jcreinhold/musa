# When a workstation and Musa disagree

Symptoms in the order they usually appear, each with the thing to check first. If none of these is your problem, the
[DAW boundary](../concepts/daw-boundary.md) explains what Musa is and is not promising, which usually names it.

## The export refuses to write

**"the folder already exists".** A bundle is never half-replaced. Either name a folder that does not exist, or pass
`--replace` when you mean to write over the last one.

**A missing or stale asset lock.** `musa assets verify <project>` says which file the lock and the bytes disagree about.
No export chooses assets by filename or modification time, so a lock that does not match is an error rather than a
guess.

## The workstation opened the files, but

**Every part is on one track.** You imported `score.mid` or `performance.mid` and the host merged them. That is the
host's import behaviour, not a property of the file — both files are format 1 with one track per part.

**There are two of everything.** You imported both MIDI documents, or a MIDI document *and* ran the MIDI Processor. They
are projections of one piece. Choose one.

**The stems do not add up to the mix.** They are not meant to. A send duplicates signal, a return may share nonlinear
processing, and the master's limiting cannot be reconstructed from what reached it. The manifest's `routes` say where
the signal actually went, and it says in words that they do not sum.

**Nothing lines up.** It should: every audio file in a bundle starts at project frame zero and is exactly as long as the
mix. If it does not, check that the host did not apply an import offset of its own.

**The notation is missing.** The GarageBand profile contains no notation file, and its manifest says so rather than
quietly omitting one. Export with `--profile logic`, or run `musa render --to musicxml`.

**It sounds out of tune.** Neither a Standard MIDI File nor a WAV states a tuning; a workstation plays the MIDI at
whatever its own instruments are tuned to. The rendered audio in the bundle is what carries Musa's tuning.

**The dynamics are flat.** The MIDI files carry notes, tempo, meter, and key and no continuous controllers. The
dynamics, articulation, and profile shaping are in the rendered audio. This is reported as a `controller` loss on every
bundle.

## Live MIDI

**No ports appear in the workstation.** `musa midi plan` needs no host and tells you what *would* be published; if it
lists the ports and the host does not see them, the host is filtering inputs. Arm the tracks. With `--single-source`
there is one port and the parts arrive on their own channels, which some hosts show as one input rather than several.

**Messages went out late, or were dropped.** `musa midi send` prints both counts. An attack more than 20 ms past its
moment is dropped rather than played late; releases always go out. A large count usually means the machine is loaded,
not that the piece is wrong.

**Two parts are on one channel.** MIDI has fifteen melodic channels. A piece with more parts than that shares, and the
run names which parts share.

**Notes hang after stopping.** They should not — stopping releases every note the run left sounding and quiets every
channel it used. A note left holding after that came from something else in the chain.

## Sharing a transport

**"a session has one clock authority".** You asked Musa both to lead and to follow. Pick one: `--lead`, or
`--from <source>`.

**"this piece is polytempo…".** One MIDI clock states one tempo and the piece has several. Name the scope with
`--reference <part>`; the others are reported unsynchronized rather than bent onto that grid.

**The lock says `acquiring` and stays there.** Fewer than 24 steady pulses have arrived. Check that the leader is
actually rolling and that `--protocol` matches what it sends — reading MTC as MIDI clock produces a stream that never
locks.

**The lock says `lost`.** The leader went quiet for half a second while it was rolling. Musa stops rather than
free-running, which is the safe answer and not a failure to reconnect.

**It re-seeks audibly.** Musa re-seeks once it is more than 30 ms from the leader, because at that distance a listener
hears two attacks rather than one. A large `jitter` figure in the report is the reason to chase.

**No tempo is reported.** You are following MTC, which carries frames of the leader's wall clock and no tempo at all.

## The Audio Units

**`auval -a` lists neither component.** The containing app has not been launched, or the extension is not registered.
Launching the app registers it.

**`auval -a` lists only one.** Almost always a *stale* registration of the same four-character triple from another build
shadowing the new one. Unregister the leftover with `pluginkit -r <path-to-the-old>.appex` and relaunch the app. Two
separate extensions each declaring one component also produce this, but Musa ships one extension declaring both.

**The plug-in is silent and says why.** Read what it says. The usual reasons are that no piece is selected yet, that the
saved bookmark no longer reaches the project, or that a saved document was written by a newer version. A future document
is refused rather than partially read.

**Automation moved to the wrong control.** It should not: parameter addresses come from a table saved in the host's
document. If the plug-in refuses a preparation because it cannot read that table, that refusal is the protection working
— re-deriving the addresses would move your lanes silently.

**A control has no knob.** It could not become a host parameter, and the plug-in window lists it as a named loss with
the reason. It is not missing; it is declared not to be a float.

**The processor is not in the MIDI FX menu.** MIDI FX slots are a Logic Pro feature. GarageBand is not supported for the
processor.

**The processor refuses the host timeline.** The piece is polytempo, and the host timeline is one quarter-note grid. Put
it on the *piece* timeline, where every part plays at its own speed.

**A host with no tempo.** The processor makes no guess. Give the host a tempo, or use the piece timeline.
