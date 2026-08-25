# Studio vocabulary

This page is generated from Musa's built-in studio catalogue. The compiler, editor help, and Sound/Mix workspaces read the same facts.

## `oscillator`

Produces a sine wave as an instrument partial or control signal. In a patch it follows score pitch and `ratio` scales that pitch; as a top-level control signal, `frequency` sets its rate.

- Signature: `oscillator(sine, frequency: Hz = 1 Hz, ratio: Ratio = 1)`
- Role: audio or control source
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `carrier = oscillator(sine, ratio: 1)`

- Ports: `NoteEvents -> Audio or () -> Control`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `frequency` | Sets a control oscillator's frequency; a patch oscillator follows score pitch. | `Hz` | 1 | 0–200 |
| `ratio` | Scales the pitch supplied by the score. | `Ratio` | 1 | 0.25–16 |

## `gain`

Changes a signal's level. The written level is in decibels and is converted to a linear multiplier only at the render-graph boundary.

- Signature: `gain(gain: dB = 0 dB)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `gain(gain: -12 dB)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `gain` | Sets level in decibels. | `dB` | 0 | -60–12 |

## `mix`

Combines two or more audio signals. Its inputs are signal arguments rather than numeric parameters.

- Signature: `mix(first: Audio, second: Audio, ...): Audio`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `mix(carrier, shimmer)`

- Ports: `Audio -> Audio`

## `envelope`

Shapes a note's level over time. Attack, decay, sustain, and release may be grouped inside `adsr(...)` for readability.

- Signature: `envelope(attack: s = 5 ms, decay: s = 0 s, sustain: Ratio = 1, release: s = 50 ms)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `envelope(adsr(attack: 30 ms, release: 400 ms))`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `attack` | Sets the rise time after a note begins. | `s` | 0.005 | 0–5 |
| `decay` | Sets the time to reach the sustain level. | `s` | 0 | 0–10 |
| `sustain` | Sets the held level while a note continues. | `Ratio` | 1 | 0–1 |
| `release` | Sets the fade time after a note ends. | `s` | 0.05 | 0–10 |

## `lowpass`

Keeps frequencies below a cutoff. `resonance` is conventionally represented by quality factor Q; larger values emphasize the cutoff without changing the public spelling.

- Signature: `lowpass(cutoff: Hz = 20000 Hz, resonance: Ratio = 0.707)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `lowpass(cutoff: 1400 Hz, resonance: 0.7)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `cutoff` | Sets the boundary frequency. | `Hz` | 20000 | 20–20000 |
| `resonance` | Emphasizes the cutoff, conventionally represented by quality factor Q. | `Ratio` | 0.7071067811865476 | 0.1–20 |

## `highpass`

Keeps frequencies above a cutoff. `resonance` is conventionally represented by quality factor Q; larger values emphasize the cutoff without changing the public spelling.

- Signature: `highpass(cutoff: Hz = 20 Hz, resonance: Ratio = 0.707)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `highpass(cutoff: 80 Hz, resonance: 0.7)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `cutoff` | Sets the boundary frequency. | `Hz` | 20 | 20–20000 |
| `resonance` | Emphasizes the cutoff, conventionally represented by quality factor Q. | `Ratio` | 0.7071067811865476 | 0.1–20 |

## `reverb`

Places audio in a simulated room. `room` controls apparent size, `damping` absorbs high frequencies, and `mix` balances dry and reverberant sound.

- Signature: `reverb(room: Ratio = 0.5, damping: Ratio = 0.5, mix: Ratio = 1)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `reverb(room: 0.82, damping: 0.55, mix: 0.3)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `room` | Sets the apparent room size. | `Ratio` | 0.5 | 0–1 |
| `damping` | Controls high-frequency absorption. | `Ratio` | 0.5 | 0–1 |
| `mix` | Balances dry and processed audio. | `Ratio` | 1 | 0–1 |

## `delay`

Repeats audio after a written time. Feedback is bounded below unity and the two-second maximum matches the preallocated delay line.

- Signature: `delay(time: s = 250 ms, feedback: Ratio = 0.3, mix: Ratio = 0.3)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `delay(time: 250 ms, feedback: 0.4, mix: 0.3)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `time` | Sets the interval before each repeat. | `s` | 0.25 | 0–2 |
| `feedback` | Sets how much delayed sound repeats. | `Ratio` | 0.3 | 0–0.95 |
| `mix` | Balances dry and processed audio. | `Ratio` | 0.3 | 0–1 |

## `chorus`

Adds a gently moving doubled voice. A control-rate oscillator varies a short delay; `mix` is the dry/wet balance.

- Signature: `chorus(rate: Hz = 0.6 Hz, depth: s = 4 ms, mix: Ratio = 0.4)`
- Role: audio processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `chorus(rate: 0.6 Hz, depth: 4 ms, mix: 0.4)`

- Ports: `Audio -> Audio`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `rate` | Sets how quickly the doubled voice moves. | `Hz` | 0.6 | 0–20 |
| `depth` | Sets the maximum delay variation. | `s` | 0.004 | 0–0.01 |
| `mix` | Balances dry and processed audio. | `Ratio` | 0.4 | 0–1 |

## `scale`

Multiplies a control signal. The factor currently carries hertz because cutoff modulation is the implemented control target.

- Signature: `scale(factor: Hz = 1 Hz): Control`
- Role: control processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `scale(250 Hz)`

- Ports: `Control -> Control`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `factor` | Multiplies each control value. | `Hz` | 1 | 0–20000 |

## `bias`

Offsets a control signal. The offset currently carries hertz because cutoff modulation is the implemented control target.

- Signature: `bias(offset: Hz = 0 Hz): Control`
- Role: control processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `bias(1400 Hz)`

- Ports: `Control -> Control`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `offset` | Adds to each control value. | `Hz` | 0 | 0–20000 |

## `clamp`

Bounds a control signal. Values below `min` or above `max` are held at the corresponding endpoint.

- Signature: `clamp(min: Hz = 0 Hz, max: Hz = 20000 Hz): Control`
- Role: control processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `clamp(min: 200 Hz, max: 6000 Hz)`

- Ports: `Control -> Control`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `min` | Sets the lowest output value. | `Hz` | 0 | 0–20000 |
| `max` | Sets the highest output value. | `Hz` | 20000 | 0–20000 |

## `smoothing`

Slows abrupt control changes. A one-pole slew runs once per audio frame, avoiding zipper noise without changing the authored control graph.

- Signature: `smoothing(time: s = 20 ms): Control`
- Role: control processor
- Origin: `builtin`
- Schema: version 1
- Native process: available
- Example: `smoothing(time: 20 ms)`

- Ports: `Control -> Control`

| Parameter | Meaning | Unit | Default | Written range |
| --- | --- | --- | ---: | ---: |
| `time` | Sets how quickly the control catches its target. | `s` | 0.02 | 0–1 |

## Studio concepts

### `Audio`

Audio is a stream of sounding samples. It is a catalogue port type, not a source type annotation in today's studio block.

- Shape: `Audio`
- Origin: `builtin`
- Example: `oscillator(sine) |> gain(-6 dB)`

### `Control`

Control is a frame-rate stream that changes a parameter. Control signals are typed separately from audio and connect only through `modulate`.

- Shape: `Control`
- Origin: `builtin`
- Example: `lfo = oscillator(sine, frequency: 0.5 Hz);`

### `NoteEvents`

NoteEvents are scheduled score gestures supplied to an instrument. They are a public port contract prepared from the score, not a sample stream or source keyword.

- Shape: `NoteEvents`
- Origin: `builtin`
- Example: `assign violin -> glass_pad;`

### `Hz`

Hertz measure cycles per second. Frequency parameters require this suffix; Musa never guesses a missing unit.

- Shape: `Number Hz`
- Origin: `builtin`
- Example: `cutoff: 1400 Hz`

### `dB`

Decibels express an audio level logarithmically. Conversion to a linear multiplier happens once at the private render boundary.

- Shape: `Number dB`
- Origin: `builtin`
- Example: `gain: -12 dB`

### `s`

Seconds measure studio time. Time values are normalized to seconds while preserving the written source span.

- Shape: `Number s`
- Origin: `builtin`
- Example: `release: 2 s`

### `ms`

Milliseconds are thousandths of a second. The compiler normalizes them to seconds without asking later stages to parse the suffix again.

- Shape: `Number ms`
- Origin: `builtin`
- Example: `attack: 30 ms`

### `Ratio`

A ratio is a dimensionless control value. Ratio parameters are written as bare numbers and reject a unit suffix.

- Shape: `Ratio`
- Origin: `builtin`
- Example: `resonance: 0.7`

### `adsr`

ADSR groups an envelope's four stages. It is a readable argument group flattened into `envelope`, not a second processor node.

- Shape: `adsr(attack: s, decay: s, sustain: Ratio, release: s)`
- Origin: `builtin`
- Example: `envelope(adsr(attack: 30 ms, release: 2 s))`

### `sine`

Sine selects a smooth periodic waveform. It is the oscillator waveform implemented by the current closed built-in set.

- Shape: `sine: Waveform`
- Origin: `builtin`
- Example: `oscillator(sine)`

### `studio`

A studio block connects a score to authored sound. It owns patches, buses, control signals, assignments, sends, routes, and modulations without changing score facts.

- Shape: `studio { StudioItem* }`
- Origin: `builtin`
- Example: `studio { route violin -> master; }`

### `assign`

An assignment chooses the patch that realizes a part. It is the narrow bridge between score identity and studio sound; a part is not a synthesizer.

- Shape: `assign Part -> Patch;`
- Origin: `builtin`
- Example: `assign violin -> glass_pad;`

### `modulate`

A modulation connects a control signal to one parameter. The target path resolves statically and must name a compatible public parameter.

- Shape: `modulate Signal -> Patch.Stage.Parameter;`
- Origin: `builtin`
- Example: `modulate lfo -> glass_pad.lowpass.cutoff;`

### `at`

At introduces a send's written level. The following value is required in decibels.

- Shape: `send Source -> Bus at dB;`
- Origin: `builtin`
- Example: `send violin -> hall at -18 dB;`

### `instrument`

An instrument turns score gestures into sound. In today's source an instrument is expressed by a `patch` assigned to a part, not by a separate declaration.

- Shape: `Part -> Patch -> Audio`
- Origin: `builtin`
- Example: `assign violin -> glass_pad;`

### `patch`

A patch describes how a part sounds. Its chains produce audio and end at `output`; assigning a part selects the patch without making part and synthesizer the same thing.

- Shape: `patch Name { SignalChain* }`
- Origin: `builtin`
- Example: `patch soft { oscillator(sine) |> output; }`

### `signal`

A signal is a named audio or control chain. Signal is the concept's name, not a source keyword; a binding can feed another stage or a modulation connection.

- Shape: `Name = SignalChain;`
- Origin: `builtin`
- Example: `drift = oscillator(sine) |> scale(250 Hz);`

### `bus`

A bus processes audio sent from parts or other buses. A bus names a shared effect path; it is not a score part or a desktop-only mixer channel.

- Shape: `bus Name { SignalChain* }`
- Origin: `builtin`
- Example: `bus hall { reverb(room: 0.8); }`

### `send`

A send copies some signal to a bus. The level is written in decibels and does not replace the source's main route.

- Shape: `send Source -> Bus at dB;`
- Origin: `builtin`
- Example: `send violin -> hall at -18 dB;`

### `route`

A route chooses where a signal goes next. A destination is a declared bus or the built-in `master` output.

- Shape: `route Source -> Destination;`
- Origin: `builtin`
- Example: `route violin -> master;`

### `master`

The master is the studio's final audio destination. It is built in and cannot be redeclared; prepared audio applies the final safety boundary there.

- Shape: `master: Audio`
- Origin: `builtin`
- Example: `route hall -> master;`

### `room`

Room is the apparent size of a reverb space. It ranges from 0 to 1 and belongs to the `reverb` processor.

- Shape: `room: Ratio = 0.5`
- Origin: `builtin`
- Example: `reverb(room: 0.82)`

### `output`

Output marks the signal leaving a patch. It is a terminal marker rather than a processor node.

- Shape: `Signal |> output`
- Origin: `builtin`
- Example: `oscillator(sine) |> output;`

