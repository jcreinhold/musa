# Performance and sound

**Status: candidate.** Performance gestures and the sound side, as event tracks at a gesture payload.

> **`docs/rules/constitution.md` §8 governs where this document differs.** The gesture track named below is the one
> event-track structure at a gesture payload — `EventTrack[PerformedTime, Gesture]` — not a structure with its own
> ordering, equality, or hash.
>
> The machine is **not** outside the core language (`../constitution.md` §4).
> `schedule(format, policy, time map, track)` is the named, checked operation that connects a track to one
> (`../constitution.md` §5), and `prepare_audio` fixes the format under the law `R1`. What stays outside is the audio
> *history*, because a history is coinductive and no source value is. Rewritten at prompt 127a.

Sound belongs in Musa because a canonical source should be capable of naming the intended performance, instrument, and
room. It remains downstream of score semantics because a written mark is not a waveform. The studio describes the
instrument and the room of the work, not an accumulating substitute for recording, editing, mixing, and mastering in a
DAW.

OMT `007-other-aspects-of-notation.md` treats dynamics, articulation, and tempo as notation whose realization depends on
performers and instruments. OMT `074-swing-rhythms.md` distinguishes written subdivision from performed swing and notes
that the ratio varies. OMT `084-drumbeats.md` records both performed and sampled/programmed percussion practice. OMT
`114-core-principles-of-orchestration.md` and `116-transcription-from-piano.md` show orchestration as a choice among
realizations, not a deterministic property of pitch content. These are the reasons profiles and instruments interpret
score facts rather than changing their meaning.

## 0. Source declarations and host boundaries

The sound language is ordinary Musa source. `Gesture`, control kinds and keys, performance profiles, exact quantities
and units, instrument signatures and mappings, studio descriptions, sample maps, standard instruments, and presets are
declared in the bundled standard library or another imported Musa package. A construct does not become a Rust builtin
because it is familiar, editable, discoverable, or performance-sensitive.

The host boundary follows `00-semantics.md`'s ownership test. Rust may preserve provenance while constructing an opaque
event track, validate and instantiate a registered primitive's private state and resource contract, resolve verified
asset bytes, schedule exact events into frames, convert exact quantities at the DSP edge, and hold compact prepared
indices and real-time state. Those are different abstractions from the source declarations.

A caller-oriented Rust value is permitted only as a projection of one checked source value. It is not a public
construction API or an independent semantic schema: every field has a source derivation, its version follows the source
declaration, and differential laws compare its complete exact encoding with the checked value. LSP, documentation, and
structured-editor vocabulary come from declarations and source indexes. They consult the primitive registry only for a
primitive's genuinely private contract.

Dependent relationships are expressed by ordinary source indices. A control key and its value share a value-kind index;
omitted indices are solved by the one Miller-pattern unifier of `02-core-calculus.md` §2.1. Sound elaboration has no
special inference table, default, coercion, or host-side fallback. The ownership repair and migration are recorded in
[`../../notes/research/language-design-closure/79-source-owns-the-sound-language.md`](../../notes/research/language-design-closure/79-source-owns-the-sound-language.md).

## 1. Factorization

For every part `l`, preparation follows exactly:

```text
EventTrack[WrittenTime, ScoreFact]
    --profile-->                    EventTrack[PerformedTime, Gesture]  ⊨ Signature
    --schedule(format, policy, time map, track)-->
                                    Schedule[Gesture] = machine + decisions
    --instrument implementation-->  Machine[AudioFrameStep, _, Frame]
part machines --mix graph-->        Machine[AudioFrameStep, _, StereoFrame]
```

More formally, after projecting the part facts `πₗ(T)`:

```text
R(Pₗ, Sₗ, πₗ(T)) = Gₗ                    profile realization
C(tempoₗ, grooveₗ, tuning, Gₗ) = Lₗ      scheduling; Lₗ is a Schedule[Gesture]
I(implementationₗ, Lₗ) = xₗ              instrument transduction; xₗ is a machine
M(mix, {PartIdₗ ↦ xₗ}) = (left,right)    routing and mixing; the result is a machine
```

`R` and the musical inputs to `C` are exact and deterministic. `I` and `M` build machines, and a machine is a finite
description; the audio history is what stepping one produces, and is never a value here. Score, performance profile,
instrument, and mix declarations remain independently editable source. One deep `prepare_audio` operation validates and
binds them into an immutable plan; neither ordinary callers nor the real-time callback assemble these stages piecemeal.

The former independent `PerformancePlan`/Rust `StudioSpec` handoff had the wrong semantic owner and an insufficient
contract: it turned profiles into a few floats before knowing the instrument, discarded part identity at graph input,
declared but ignored parameter events, and exposed graph-stage addressing as if it were musical control. The production
handoff is now a versioned checked source value decoded into a read-only DSP preparation projection; the compiler has no
DSP dependency. A rejected combined score-audio object would make notation edits mutate DSP state and would destroy
independent export, caching, and UI projections.

## 2. Performance gestures

A gesture track is an `EventTrack[PerformedTime, Gesture]` carrying a checked conformance witness to an instrument
signature `S`, written `EventTrack[PerformedTime, Gesture] ⊨ S`. Its payload vocabulary is:

```text
NoteGesture {
    instance: GestureId,
    written_pitch: WrittenPitch,
    controls: finite ControlKey ↦ exact value,
    techniques: finite typed requests,
    origin: Origin
}
ControlCurve { key: ControlKey, values: Progress indexed by normalized local time }
PhraseGroup { members: nonempty list GestureId, connection: detached | ordinary | legato }
ReleaseGesture { instance: GestureId }
```

These are source-declared data in `std::performance`, not Rust enums. The witness is specification notation for a track
plus a checked conformance judgment `G ⊨ S`; it is not a second type index. The core carries exactly one type index, the
coordinate (`../events/02-static-semantics.md` K2), so conformance is a pass result rather than a dependent type in the
source calculus or a public Rust generic over user declarations.

The occurrence span carries performed onset and extent; a release is a point occurrence. No payload stores an absolute
position in written, performed, physical, or frame coordinates. A separate immutable lineage projection maps `GestureId`
to the source `EventId` and written span for notation/MIDI/provenance consumers; it is not a temporal container, part of
gesture canonical equality, or an input to scheduling. A note's sounding tail remains instrument behavior and may extend
after release.

Continuous curves store exact piecewise-linear `Progress` over normalized local occurrence time. The containing
occurrence span supplies `[s,e]`; moving or stretching it leaves identical payload bytes, as payload-admission law L24
requires. Discontinuities are source-stably ordered point occurrences. Exponential frequency/gain laws belong to an
instrument's physical mapping, not to normalized gesture arithmetic.

A profile is a named interpretation `R(P,S,−)`. It reads symbolic dynamics, hairpins, accents, staccato, tenuto, fermata
policy, grace policy, slurs, phrase marks, and groove and emits only controls/gestures admitted by `S`. A hairpin may
become an `expression` curve because the profile declares that reading. It cannot become a filter sweep by convention. A
slur normally creates phrase/legato grouping; it is neither an ADSR envelope nor proof that sound remains continuous on
every instrument.

Precisely, if a hairpin spans `[s,e]`, its profile maps the prevailing and target dynamics to exact values `d₀,d₁`, and
its written shape is `p : [0,1] → [0,1]`, then for `s ≤ b ≤ e`:

```text
expression(b) = d₀ + (d₁-d₀) · p((b-s)/(e-s)).
```

The constructor requires `s(e`, `p(0)=0`, and `p(1)=1`; hence the endpoints are exactly `d₀,d₁`. The payload encodes
each breakpoint at normalized local coordinate `(b-s)/(e-s)`; explicit point changes are separate point occurrences with
source-stable same-beat ordering. Instrument mapping occurs later and cannot change this gesture-level curve.

Groove maps exact written beat to exact performed beat before tempo. Tempo then maps performed beat to physical time.
Thus swing survives a tempo change, and tempo never stretches the written event track.

## 3. Instrument signatures

An instrument is defined by exposed behavior:

```text
Instrument = (Signature, private Implementation)

Signature = {
    gesture inputs,
    controls with key/domain/rate/default/requirement,
    techniques with parameter types,
    channel output
}
```

The standard `note_instrument` signature accepts note/release/phrase gestures and stereo output. Its standard controls
are musical intentions, not acoustic measurements:

| `ControlKey` | Domain/rate | Meaning |
| --- | --- | --- |
| `expression` | normalized `[0,1]`, continuous | sustained relative intensity; not dB, MIDI velocity, or orchestration density |
| `emphasis` | normalized `[0,1]`, per note | attack salience relative to neighbors; not an attack-time number |
| `separation` | normalized `[0,1]`, per transition | requested perceptual detachment; not a universal gate multiplier |
| `brightness` | normalized `[0,1]`, continuous | relative spectral-brightness intention; not filter cutoff |
| `sustain` | normalized `[0,1]`, continuous | requested continuation after release; not pedal position or release seconds |
| `phrase` | typed group/connection | grouping and `ordinary`/`legato`/`detached` relation; not a scalar graph parameter |

`normalized` means an exact rational in `[0,1]`. It does not promise equal loudness or timbre across instruments.
Libraries may add typed namespaced keys, for example `bow.pressure : normalized` or
`prepared_piano.mallet_position : cm in [0 cm,12 cm]`. Namespacing prevents accidental agreement between unrelated
controls. A physical custom control reduces swappability by design and must match name, unit, domain, and rate.

`ControlKey(K)` is an indexed source record comprising namespace, name, value kind `K`, and update rate; its paired
`ControlValue(K)` is fixed by the same index. Standard keys are ordinary values exported by `std::performance`, not
constructors of a closed host enum. During preparation a key resolves privately to one or more render-plan parameter
indices or sample-engine operations. Those indices are not stable, are not serializable source addresses, and never
enter a profile, gesture, diagnostic identity, or public instrument signature.

## 4. Private implementations and full expressivity

An implementation may be a native DSP graph, sample map, physical-model engine, or future trusted adapter. The simple
surface imports an instrument and binds it. Authors who build instruments use the progressively disclosed implementation
block:

```musa
instrument glass conforms note_instrument {
    implementation graph {
        voice oscillator(sine)
            |) envelope(attack: 12 ms, decay: 180 ms, sustain: 0.7, release: 600 ms)
            |> lowpass(cutoff: 4200 Hz, resonance: 0.8)
            |> output;

        map expression -> voice.gain using perceptual(-42 dB, 0 dB);
        map brightness -> voice.lowpass.cutoff using exponential(300 Hz, 9000 Hz);
        map emphasis -> voice.envelope.attack using inverse(35 ms, 2 ms);
    }
}
```

`oscillator`, `envelope`, `lowpass`, `resonance`, ports, and units are ordinary declarations in the edition-pinned
`std::sound` import, documented from that source. Their implementation wrappers may name registered primitives whose
identity, state, port formats, and resource contracts remain private to the host registry. Advanced graph paths are
legal only inside the private implementation. The `map` clauses prove conformance by implementing semantic controls. A
profile can name `brightness`; it cannot name `voice.lowpass.cutoff`. Replacing the graph with a sample map preserves
the signature while changing every private address.

The graph language retains typed audio/control/gate/note ports, explicit delay for cycles, precompiled topology, and
bounded processors. A new registered primitive requires a musician/audio-engineer definition, units/ranges/defaults, an
RT-safe implementation, partition-law tests, and a prompt-scoped justification; its source wrapper and hover text are
ordinary library declarations. This is how full instrument design remains available without making
electrical-engineering vocabulary the entry surface.

## 5. Identity, selection, and swapping

Three identities must not collapse:

- `PartId` identifies the notated part and survives instrument changes;
- `InstrumentDeclId` identifies the selected source declaration and its signature;
- `PreparedInstrumentId = H(plan, PartId, InstrumentDeclId, instance ordinal)` identifies mutable render state inside
  one prepared plan.

Each part's `Schedule[Gesture]` retains `PartId` and is delivered only to its prepared instances. Sharing one instrument
declaration does not share voices or state unless an explicit ensemble implementation says so. This removes the current
shared-note-stream behavior. The mix graph connects labeled part machines; it does not inspect note events.

Instrument replacement is accepted when the new signature is a behavioral super-signature of every gesture, technique,
and control required by the selected profile and explicit source bindings. Standard normalized controls make common
swaps easy. Namespaced or physical controls require a matching signature or an explicit rewrite; Musa does not guess a
conversion.

## 6. Surface binding and policy

The ordinary binding is one readable sentence inside the part:

```musa
part violin {
    sound solo_strings using lyrical;
    voice melody { c5/4 d5/4 e5/2 }
}
```

`sound i using p;` desugars to three independent declarations: select profile `p` for the containing part, assign its
`PartId` to instrument `i`, and route that part output to `master`. The expert surface spells those as `profile`,
`assign`, and `route`, and can add sends. A part may instead carry defaults, but after resolution there is exactly one
selected instrument and profile. The formatter and hover explain `sound`, `assign`, `send`, `room`, `bus`, `route`, and
every edition-pinned standard-library processor wrapper. A `room` is a named shared ambience path. A `bus` is the
advanced general form: a named summing path with an effect chain. `send part -> room at level` copies a part's output to
it; `route x -> master` selects what reaches stereo output. Voice is not mixer track, and part is not synthesizer.

`studio { ... }` is a source grouping retained for compatibility and readability. Elaboration separates its assignments,
instrument implementation declarations, and mix declarations before preparation; it is not a combined score/audio value
or one mutable compiler object.

An unbound part uses the language-edition declarations `std.sound.basic_sine` and `std.performance.neutral` and is
routed to `master`. This preserves audible zero-setup playback without hiding the effective choice: hover and the Sound
inspector show both declarations, and “make sound explicit” inserts
`sound std.sound.basic_sine using std.performance.neutral;`. Changing that default requires a language edition change.

Existing studio source migrates by this table; format never rewrites a user's expert graph implicitly:

| Existing form | Candidate account | Compatibility |
| --- | --- | --- |
| `patch P { chain }` | `instrument P conforms note_instrument { implementation graph { chain } }` | accepted with a deprecation and exact source fix through the next edition |
| `assign part -> P;` | expert part-to-instrument binding | remains canonical |
| `profile p;` in a part | expert profile selection | remains canonical |
| `route` / `send` | explicit mix routing | remains canonical |
| filter parameter `q:` | filter parameter `resonance:` | `q` is a hard error with an exact fix; there is one accepted spelling |

Prompt 176 amends the last row. The earlier compatibility promise predated a catalogue capable of naming a canonical
parameter and its fix, and no released stored format or public runtime identity encoded `q`; keeping both spellings
would make every catalogue consumer and structured editor carry an alias indefinitely. Existing repository sources are
migrated atomically, while an older source receives a certain replacement rather than an ambiguous rejection. The
argument and migration inventory remain in
[`../../notes/research/language-design-closure/77-one-studio-vocabulary.md`](../../notes/research/language-design-closure/77-one-studio-vocabulary.md).

Unsupported realization is explicit:

1. an explicit source control or a control emitted by a profile but absent from the chosen signature is an error;
2. an explicit technique absent from the signature is an error unless that assignment names it in
   `unsupported technique NAME -> notation_only warning;`;
3. a written expressive mark with no profile rule remains valid notation, uses the signature default, and emits an
   `unrealized-mark` warning when the selected output is audio;
4. optional implementation detail that cannot be honored must emit a stable warning and use its documented signature
   fallback;
5. no event, control, technique, route, or asset failure is silently ignored.

`notation_only warning` is the only fallback spelling and always diagnoses; there is no wildcard “ignore unsupported.”

## 7. Exact-to-physical boundary

Source numbers, profile mappings, gesture positions, normalized controls, units, and curves remain integers/rationals or
dimensioned exact quantities through the gesture track. Tempo integration uses the canonical exact map already specified
by the backend contract: seconds per beat is piecewise linear along exact `Progress`, so each segment has an exact
rational trapezoidal integral. Unsupported shapes are rejected rather than sampled early.

`prepare_audio(sample_rate, block_limit, options)` is the single late approximation boundary. It:

1. integrates groove and tempo and rounds event boundaries to frames by the documented nearest-frame, ties-to-even rule
   while preserving monotonic order;
2. asks tuning for physical frequency;
3. maps semantic controls through the selected instrument implementation to dimensioned DSP values;
4. converts those values to the implementation's `f32`/`f64` representation;
5. allocates voices, state, queues, schedules, and buffers on the control thread.

No eager `f64` is stored in a checked source studio value or its exact projection. Diagnostics show the written exact
value and, when relevant, the prepared approximation.

## 8. Machine and mix laws

A prepared machine is causal: output frame `n` depends only on scheduled inputs through `n` and prior private state
(machine Theorem M2, `../across-stages/03-machine-calculus.md`). It allocates, locks, performs I/O, logs, or destroys no
large object in the audio callback. All capacity failures are handled during preparation or by a documented bounded
real-time policy.

**One audio step is one sample frame** (`../constitution.md` §4, `../obligations.md` rule 8). Registered primitives
define state transitions per frame, and smoothing and automation are functions of absolute frame index. A host block is
therefore an optimization, never the semantics: replacing `n` repeated steps by one `batch(n)` call is legal only where
that primitive or machine has a `batch` implementation satisfying the contract in
`../across-stages/03-machine-calculus.md` §4, and R1-batch is what then makes the caller's partition unobservable. A
machine containing feedback does not inherit a valid batch from its parts. Floating-point tests use a processor-specific
tolerance where algebraic reassociation is unavoidable; deterministic offline export uses one documented summation
order.

Offline and live execution step the same prepared machine over the same `Schedule[Gesture]`. Offline may choose block
sizes and write files; it may not substitute a different synthesis algorithm. The law is:

```text
concat(run(prepared, partition₁, inputs)) ≈samples concat(run(prepared, partition₂, inputs))    where every block is a valid batch
offline(prepared, inputs) = run(prepared, canonical offline partition, inputs)
```

Part isolation is observable: with all parts except `l` silent, only the machine paths reachable from `PartId(l)` and
explicit sends may produce a nonzero frame. A mix route never causes another instrument to receive `l`'s gestures.

## 9. Scope rule

A feature belongs in Musa sound when it specifies reproducible compositional performance, instrument identity/behavior,
or intentional spatial/mix relation needed to hear the work. Non-destructive room, balance, and instrument processing
qualify. Multitrack recording edits, comping, restoration, metering workflows, plugin hosting, and mastering-delivery
chains remain external. The deciding question is semantic ownership, not whether a processor happens to be useful.

The Sound and Mix workspaces project these declarations with progressive disclosure. They may offer knobs and graph
views, but every edit rewrites `.musa` source and every semantic control is shown by its musical name before its private
implementation mapping.
