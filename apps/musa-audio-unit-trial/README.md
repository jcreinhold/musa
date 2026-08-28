# The Audio Unit trial

The smallest macOS AUv3 that can falsify
[`docs/rules/across-stages/06-daw-boundary.md`](../../docs/rules/across-stages/06-daw-boundary.md) §3–§7. Prompt 215
built it so that the production Audio Unit — prompts 216–218 — is coupled to a measured answer rather than to an
article.

**It links no Musa runtime, and it is not the beginning of one.** It renders a fixed-point sawtooth from a table and
schedules eight fixed occurrences. Anything it can do, it can do because the *system* allows it; that is the only thing
it is evidence about. The measurements and what they decided are in
[`docs/notes/research/93-the-audio-unit-shape.md`](../../docs/notes/research/93-the-audio-unit-shape.md).

## Running it

```sh
scripts/check-audio-unit-trial.sh
```

Needs a full Xcode (`xcode-select -p` must name `Xcode.app`, not the Command Line Tools). The script builds unsigned,
ad-hoc signs each bundle with its entitlements, registers both extensions for this user with `pluginkit -a`, waits for
Core Audio to see them, runs `auval -v` on each, drives them from the harness with the allocation probe interposed, and
asserts the resulting JSON with [`scripts/check-audio-unit-trial.py`](../../scripts/check-audio-unit-trial.py). An
`EXIT` trap deregisters both extensions, so a failed run leaves nothing installed either.

Build output goes to `target/audio-unit-trial/`, which `.gitignore` already covers. The findings land in
`target/audio-unit-trial/Build/Products/Debug/trial-report.json` beside the `auval` logs.

## What is here

| Path | What it is |
| --- | --- |
| `Sources/TrialKit/MusaTrialABI.{h,c}` | the versioned C ABI: an immutable prepared plan, wait-free publication, fixed polyphony, a finite random-access schedule |
| `Sources/TrialKit/MusaTrialDrive.{h,m}` | an Objective-C holder for the render block, so a measurement is not measuring the Swift block bridge |
| `Sources/TrialKit/TrialInstrumentAudioUnit.swift` | the `aumu` Music Device: parameters, two output buses, document state, the render block |
| `Sources/TrialKit/TrialProcessorAudioUnit.swift` | the `aumi` MIDI Processor: host position in, scheduled MIDI out |
| `Sources/{Instrument,Processor}/` | the two `AUAudioUnitFactory` principal classes; each appex is otherwise empty |
| `Sources/AllocProbe/AllocProbe.c` | a `__DATA,__interpose` dylib counting `malloc`/`calloc`/`realloc`/`posix_memalign` |
| `Sources/Harness/` | the automated host: instantiate, render, measure, and write findings as JSON |
| `Sources/App/` | the containing app, which exists because an appex needs one |
| `Config/` | the `Info.plist` and entitlements for the app and both extensions |

## Reading a finding

Each finding has an id, a title, an outcome, a detail, and sometimes numbers. `pass` and `fail` are self-explanatory;
`unsupported` means the trial could not settle it on this machine and says why, and only the ids listed in
`MAY_BE_UNSUPPORTED` in the Python checker are allowed to be `unsupported` at all. Adding a finding means adding its id
to `REQUIRED` — a finding the checker does not know about cannot silently disappear.
