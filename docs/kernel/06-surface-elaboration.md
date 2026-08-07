# 06 — Surface Elaboration

How the existing `.musa` surface language elaborates into the temporal kernel (course correction §30 Step 4, §19–21).
This document describes elaboration of the grammar **as it exists today** (prompts 02–06); it is not a surface redesign
(§31). The implementation is prompt 11 (`docs/prompts/11-kernel-elaboration.md`).

## The elaboration boundary

```text
source / musical HIR  (motifs, repeats, transforms, chords, provenance — preserved)
        ↓  elaborate / observe
finite temporal kernel  (flat timelines; the semantic quotient)
        ↓  adapt
ScoreSnapshot  (the score-specific projection backends already consume)
```

Normalization is a **semantic boundary, not the internal representation of every compiler pass** (§19). The HIR keeps
`repeat`, `loop`, motif references, and transformations for efficiency, editing, provenance, diagnostics, and structural
display; elaboration evaluates finite observations of them into kernel timelines. Nothing requires duplicating thousands
of nodes merely to obey the normalized model.

## Payload design (the central decision)

Score elaboration uses one payload type per fact domain. For the current grammar there is one essential payload:

```text
NotePayload {
    pitch: WrittenPitch        % written spelling, verbatim — never a MIDI number (roadmap §2)
    voice: VoiceIdentity       % (part, voice) source identity
    origin: Origin             % provenance: source span, declaration, expansion path
}
```

Decisions recorded against course correction §32:

- **Voice identity rides in payload metadata** — it is not a temporal primitive. The candidate answer to the §32 open
  question is "payload metadata + HIR structure"; prompt 11 gathers the evidence.
- **Provenance rides in the payload** (§20): the kernel quotient forgets production history, so history travels with
  each occurrence as data the kernel is opaque to. `Origin` never participates in temporal semantics; it participates in
  canonical payload serialization only as a stable, deterministic key (N3), so semantic equality can still distinguish
  occurrences a consumer must tell apart.
- **No duration field in the payload** — duration is temporal support (D0).

## Elaboration rules, per surface construct

| Surface construct | Elaboration |
| --- | --- |
| `note c5 1/4;` | One occurrence of `NotePayload` over the current position's span; the voice cursor advances by `1/4`. |
| `rest 1/2;` | **No occurrence.** The voice cursor advances by `1/2` — ambient extent grows; absence is silence (§2). |
| `chord (c5 e5 g5) 1/2;` | `overlay` of one `NotePayload` occurrence per pitch over the same span (§30 Step 4). The snapshot adapter regroups same-span, same-voice, same-origin occurrences into `ScoreEventKind::Chord`. |
| voice body | `sequence` of its items in source order (cursor semantics = left-fold of successive extents). |
| part | `overlay` of its voice timelines. Voice identity stays separable via payload metadata. |
| `use motif(args);` | Binding + reference at the HIR level: the motif body elaborates with bound arguments; each resulting occurrence's `Origin` gains the `MotifApplication` step. Not a kernel concept. |
| `repeat n { … }` | HIR-level `sequence` of `n` evaluations (§19); each iteration's occurrences gain the `RepeatIteration(i)` provenance step. |
| `transpose up P5 { … }` | `map_payload` with the transposition function on `pitch` (§13); occurrences gain the `Transposition` provenance step. |
| `key`, `meter`, `tempo` declarations | **Context, not occurrences**, in the current grammar: they populate the snapshot's `KeyMap`/`MeterMap`/`TempoMap` exactly as the old lowerer does. |
| piece | The part timelines, the context maps, and the annotation store — packaged by the adapter into `ScoreSnapshot`. |

## Key, meter, harmony: the future shape

Course correction §21 places key/meter/harmony **regions** in the kernel as typed interval payloads
(`Timeline[KeyRegion]`, `Timeline[MeterRegion]`, `Timeline[HarmonyAnnotation]`) whenever their temporal extent matters —
e.g. `modulate to C major { … }`. The current surface grammar has only piece-wide declarations, so today's adapter keeps
them as snapshot context maps. When a surface construct gives them temporal extent, they elaborate as payload-bearing
timelines **without any kernel change** (that is the point of §21), and this document is extended — not repaired — at
that prompt. Prompt 30 (annotations) is the first consumer of that shape.

## Tempo stays out (§22)

Tempo never elaborates into kernel occurrences and never rescales kernel time. It is the performance layer's monotone
map `Beat → Second` applied to symbolic positions at realization time (`07-backend-contract.md`). "Stretch the material"
(payload/time action, D5) and "perform the same material more slowly" (tempo map) remain different operations.

## Adapter contract (ScoreSnapshot)

The adapter projects normalized `Timeline[NotePayload]` values into the existing `ScoreSnapshot`:

- occurrences → `ScoreEvent`s (id assignment deterministic: canonical order per part, voice-major);
- same-span/same-voice/same-origin groups → `Chord`; singletons → `Note`;
- payload `voice` → part/voice lanes; payload `origin` → event `Origin` verbatim;
- extent per voice → `Voice::span`; part extent = max over voices;
- context maps and annotations → as the old lowerer emits them.

**Parity requirement (§30 Step 5):** for every fixture, the adapter's snapshot must equal the old lowerer's snapshot on
positions, durations, spelling, part/voice identity, multiplicity, ordering, and provenance. The old lowerer is the
regression oracle until prompt 12, and remains runnable permanently.

## What elaboration must never do

- Introduce rest/silence occurrences to "fill" regions (§2).
- Push production history into kernel semantics (e.g. making equality motif-aware) (§20).
- Add kernel constructs because one surface feature is awkward — awkwardness is elaboration's problem (§34).
- Change the surface grammar to make elaboration easier (§35.9).
