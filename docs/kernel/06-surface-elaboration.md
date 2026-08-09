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
| `c5/4` | One occurrence of `ScoreFact::Note` over the current position's span, carrying pitch, written duration, and any articulations; the voice cursor advances by `1/4`. |
| `rest/2` | One occurrence of `ScoreFact::Rest` over the current position's span; the voice cursor advances by `1/2`. A *written* rest is notation an author asked for, and export and provenance both need it; what stays absent is unwritten silence (§2). |
| `[c5 e5 g5]/2` | `overlay` of one `Note` occurrence per pitch over the same span (§30 Step 4). The projection regroups same-span, same-scope, same-origin note occurrences into `ScoreEventKind::Chord`. |
| voice body | `sequence` of its items in source order (cursor semantics = left-fold of successive extents). |
| part | `overlay` of its voice timelines. Voice identity is the fact's `Scope`, not a timeline of its own. |
| piece score | `overlay` of its part timelines: **one** `Timeline<ScoreFact>` per compilation. |
| `use motif(args);` | Binding + reference at the HIR level: the motif body elaborates with bound arguments; each resulting occurrence's `Origin` gains the `MotifApplication` step. Not a kernel concept. |
| `repeat n { … }` | HIR-level `sequence` of `n` evaluations (§19); each iteration's occurrences gain the `RepeatIteration(i)` provenance step. |
| `transpose up P5 { … }` | `map_payload` with the transposition function on `pitch` (§13); occurrences gain the `Transposition` provenance step. |
| `c4/4 ~` (tie) | **No kernel construct, and no fact.** A tie says two written noteheads spell *one* occurrence, so elaboration merges the tied statement with its continuation on the spot: one occurrence, span the sum, `NotatedDuration` the compound spelling. Merging happens at every nesting level, so a tie inside a `retrograde` is gone before the block is reversed and needs no repair. A tie onto a different pitch, or with nothing after it, is a diagnostic. |
| `c4/4 accent staccato` | Articulations are a **field of the note fact**, not facts of their own: a staccato dot has no extent and no identity apart from its note. The projection emits one `ArticulationMarking` per name, in written order, against the event's id. |
| `dynamic mf;` | A **point** occurrence of `ScoreFact::Dynamic` at the cursor, with no cursor advance. The projection resolves it to the first event at or after it in the same voice; nothing after it is a diagnostic. |
| `slur { … }` | The body elaborates unchanged, and one `ScoreFact::Slur` occurrence is **overlaid** over `[0, extent)` of it. Nothing is copied onto the notes. The projection emits a `SlurSpan` naming the first event at or after the region's start and the last ending at or before its end. `phrase` and `crescendo`/`diminuendo` work identically. |
| `tuplet n/d { … }` | The body elaborates with the voice's duration scale multiplied by `d/n`, so written values become exact rationals (`3/2` of eighths gives `1/12`). A `ScoreFact::Tuplet` occurrence is overlaid over the body, and the projection emits a `TupletSpan` carrying the unreduced `n/d`, which is what the backends need to print the bracket. A tuplet that would cross a barline is a diagnostic. |
| `performance { profile v { … } }` | **Nothing elaborates.** A profile is a reading of marks, not material: it produces no occurrence, occupies no time, and is carried on the snapshot beside the motif table for the performance layer to consult. Written marks stay written (§6.4). |
| `profile v;` inside a part | **A binding, not an occurrence.** It names which profile realizes this part; naming an undeclared one is a diagnostic. Both semantic paths read it through the same `part_metadata`, so it cannot drift between them. |
| `key c major;`, `meter 4/4;` | **Region occurrences** of `ScoreFact::Key`/`Meter`, scoped to the piece and spanning `[0, d]`. An unwritten meter still produces a fact — 4/4 governs a piece that never says so — while an unwritten key produces none, which is why the projection's `key` is an `Option` and its `meter` is not. |
| `section "A" at 9:1;`, `harmony { at 1:1 c; }` | **Point occurrences** of `ScoreFact::Section`/`Harmony` at the time the coordinate names. The coordinate is resolved against the *meter occurrence* and the timeline's own extent; naming a place the piece never reaches is a diagnostic. |
| `tempo 1/4 = 60;`, `tempo 1/4 = 90 at 9:1;` | **Not a fact, ever** (§22). Tempo is the performance layer's `Beat → Second` map; it stays on the snapshot's `TempoMap`. See "Tempo stays out" below. |
| piece | The one timeline **projected** into `ScoreSnapshot` (`musa-compiler/src/project.rs`): voices, context maps, and annotations alike. |

## The payload, and the adapter contract

A `ScoreFact` has exactly three axes, and they vary independently:

- **`scope`** — where in the score's *structure* the fact sits: one voice of one part, or the piece as a whole. Not
  where it is in time.
- **`kind`** — what is stated: a note, a rest, a slur, a phrase, a tuplet, a dynamic, a hairpin, a key, a meter, a
  section, a chord symbol.
- **`origin`** — why it exists (§20). Provenance stays above the kernel.

*Where it is in time is the occurrence's span*, and is never a field. Keeping the three apart is the point: a slur moves
in time without changing voice, a voice is renamed without moving anything, a dynamic changes from `mf` to `f` in place.

The projection (`project.rs`) visits the canonically ordered occurrences **once**, buckets them by scope, assigns
`EventId`s to note and rest facts in visit order, and resolves region facts to the event ids at their ends. It relies on
one invariant, stated on the function and asserted in debug builds: **a region fact's boundaries coincide with event
boundaries in its own scope**, because a region is built from the extent of the items it encloses. If that is ever
violated, the elaboration that violated it is the bug.

## Sharing and provenance: how `repeat` and `use` elaborate (prompt 49)

Elaboration emits a **term** (`10-term-calculus.md`), evaluated at the compiler's boundary. `repeat n { body }`
elaborates the body **once** into a `let` and references it `n` times; a `use motif(args)` elaborates the motif's body
once per distinct argument tuple and references it at each call site. Course correction §19 is the sentence being
implemented: normalization is a semantic boundary, not the internal representation of every compiler pass — "nothing
requires duplicating thousands of nodes merely to obey the normalized model".

**The provenance question, and its answer.** Every occurrence of the third repetition must carry `RepeatIteration(2)`,
and the Origin view depends on it. If the body is elaborated once, the occurrences inside the `let` cannot each carry a
different iteration — that is the saving. Two options were on the table; the resolution is **provenance at the
reference**:

> A reference carries a **mark** naming what distinguishes this use. Evaluation applies a payload map chosen from that
> mark, rewriting each instantiated occurrence's `Origin` and nothing else.

**The mark's text**, which is `ScoreFact`'s and not the kernel's — the kernel treats it as an opaque string
(`10-term-calculus.md` T6):

```text
mark  = <depth> "|" <origin-span> "|" <scope> "|" <steps>
depth = <integer>                      (* where in the expansion path the steps belong *)
steps = <step> { "," <step> }          (* the expansion-path grammar below *)
```

`origin-span` and `scope` are `-` when the reference does not rewrite them. The steps come **last** so they are escaped
once rather than twice: nothing before them contains a `|`, so a reader splits three times and takes the rest verbatim.

Three things this had to get right, none of them obvious from the option alone:

- **Depth, not append.** A repeat's iteration index belongs *before* the steps of everything nested inside the body,
  which is where direct expansion puts it. Appending would put it after. So the mark says where to splice, and a repeat
  splices at its body's own depth while a motif call splices at zero.
- **A motif body is elaborated with no path at all**, because two call sites in different places must reach the same
  body. The path leading to the call — every enclosing transposition and motif application — travels on the mark
  instead, and is spliced back at depth zero.
- **Placeholders for what the call supplies.** A motif body's occurrences take their `source_span` from the *call* and
  their `scope` from the *voice*, and neither can be baked into a shared body. The body carries `u32::MAX` in both, and
  the mark says what to put there. This is visible in `examples/kernel/*.musa.kernel` as `4294967295` inside a shared
  binding's payloads, and it is not corrupt data: it is the hole the reference fills.

Provenance is byte-identical to what direct expansion produced: the same steps, in the same order, on the same
occurrences. The saving is in elaboration — the CST is walked once, pitches resolved once, diagnostics emitted once —
not in evaluation, which still materializes every occurrence.

**Where sharing is spent.** A level that needs a *value* rather than a term cannot stay shared, and there are exactly
four: a tie crossing an item boundary (merging joins two occurrences into one, which no payload map can do),
`retrograde` and `invert` and `stretch` (payload maps and mirroring, applied during elaboration), and a `use` with
`with { … }` overrides (which respell notes of *this* call). Each of these evaluates its reference, which instantiates
the body exactly as direct expansion would have built it — so the sharing is spent, not lost, and the binding it made is
pruned when the piece's term is closed. This is why `examples/kernel/variation.musa.kernel` has one `let` for five `use`s:
four of its five are inside a transformation.

The rejected option was to share only where the expansion path would be identical, which for `repeat` is never, and
which would therefore have bought nothing. What was *not* an option was dropping the iteration index: the Origin view is
a promise the project already made.

**What does not share.** `transpose`, `invert` and `stretch` bodies are payload maps and time scaling applied during
elaboration; `scale` has a term and the payload maps do not, and inventing one would breach the calculus's absent list.
Voices become `over` and voice items `seq` — structural, and what makes a printed file legible.

## `ScoreFact`'s interchange text form (prompts 48, 86)

A kernel file carries payloads as opaque quoted strings (`01-grammar.md`); this is what `ScoreFact` puts inside one. It
is specified here, with the payload, rather than in the grammar, because the kernel neither writes it nor reads it.

A label is a **flat, whitespace-separated stream of words**. Flat is load-bearing: the first form nested five
separators five deep and escaped each level again at the next, so one colon inside a motif call reached the file as
eight backslashes. Nothing here nests, so nothing is escaped twice.

A word is either **bare** — no whitespace, no `'`, no `\`, no bracket — or **quoted**, `'…'` escaping `\` and `'`.
Every free-text field is quoted *always*, even where quoting would not be needed: that is what keeps `mark text '8'`
and `mark ottava 8` apart without case analysis, and it means a payload never contains `"`, so the kernel's own string
escape has nothing to double.

```text
label       = <scope> <kind> <origin>

scope       = "piece" | "part" <n> | "voice" <part> <voice>
origin      = "[" <span> [ "def" <span> ] [ "#" <n> ] [ "via" <step> { <step> } ] "]"
span        = <start> ":" <end>
step        = "motif" <span> | "repeat" <n> | "transpose" <steps> <semitones>
            | "stretch" <ratio> | "retrograde" | "invert" <quoted> | "special" <span>

kind        = "note" <pitch> <duration> { <articulation> } [ <free> ]
            | "rest" <duration> { <articulation> } [ <free> ]
            | "mark" <name> [ <quoted> | <n> ]
            | "grace" <pitch> <index> { <articulation> }
            | "slur" | "phrase" <quoted> | "tuplet" <n> "/" <d>
            | "dynamic" <mark>
            | "hairpin" ("cres" | "dim") <mark> <progress>
            | "key" <tonic> ("major" | "minor")
            | "meter" <n> "/" <d> | "clef" <name>
            | "tempo" [ <ratio> "=" <bpm> ] [ <quoted> ] [ [ "to" <bpm> ] "over" <ratio> <progress> ]
            | "section" <quoted> | "harmony" <quoted>
            | "repeat" <times> [ "from" <least> "to" <most> ]
            | "ending" <bracket> "pass" <pass>
            | "mobile" { <quoted> } "order" { <n> }
            | "improvise" [ "over" <quoted> ]

duration    = <ratio> [ "spelled" <quoted> ] [ "tied" <ratio> { <ratio> } ]
free        = "free" <ratio> <ratio>
ratio       = <p> "/" <q> | <p>                      (* `1`, not `1/1` *)
progress    = <u> ":" <v> { "," <u> ":" <v> }        (* N3's canonical form *)
```

```text
occurrence "voice 0 0 note c4 1/4 [191:198 #4]"                              from 0 to 1/4;
occurrence "piece tempo 1/4=96 [26:50]"                                      from 0 to 2;
occurrence "voice 0 0 note g4 1/4 [299:311 def 97:104 #4 via motif 299:311]" from 0 to 1/4;
```

Five rules make it read back:

- **Names, not positions.** Absence is absence rather than a counted run of empty fields, so a tempo marking that is
  only a metronome mark is `tempo 1/4=96` and not seven fields of which four are empty.
- **Elision is a biconditional, never a guess.** `def` is written iff the definition span differs from the source
  span; `#n` iff the declaration is not zero; `via` iff the expansion path is non-empty; `spelled` iff the spelling
  differs from how a ratio is written; `tied` iff the pieces are not exactly the one value. The span itself is never
  elided, so `[0:0]` is written as it stands.
- **Every rational is `p/q`, or `p` when the denominator is one.** Durations still carry the written spelling *and*
  the exact value *and* the tied pieces where those differ, because a tuplet keeps the symbol while changing what it
  sounds for (§2) and no one of the three derives the others.
- **A hairpin's shape is its `Progress` in canonical form** — the one place where N3's key and the interchange text
  coincide, because a `Progress` has no provenance to quotient away.
- **`tied` is absent.** It is elaboration-only and false on every fact that leaves elaboration: a tie says two noteheads
  spell one occurrence, which is resolved before a timeline exists. A file carrying it would describe a state no
  timeline is ever in.

A reference's mark is the same word stream — `depth <n> [ "origin" <span> ] [ "scope" <scope> ] [ "via" <step>… ]` —
so a file has one tokenization and one escape rule throughout.

Everything the value holds is present, including the definition span and the declaration id that `canonical_key` (N3)
deliberately drops. That is why these are two functions and not one: N3 is the *equality* serialization and may
quotient; interchange must reproduce. `05-normalization.md` N3 states the same repair from the other side.

## Key, meter, harmony: the present shape

Course correction §21 places key/meter/harmony **regions** in the kernel as typed interval payloads whenever their
temporal extent matters — e.g. `modulate to C major { … }`. Prompt 40 put them there **before** the surface grew such a
construct, and that order was deliberate: a region that happens to cover the whole piece is not a special case, but a
piece-wide scalar called `MeterMap` is. Modelling meter as one region over `[0, d]` now means the later change adds
*more occurrences* rather than a second way to ask the same question (PoSD ch. 10).

So today:

- `Key { tonic, mode }` and `Meter { numerator, denominator }` are occurrences over `[0, d]`, scoped `Scope::Piece`.
- `Section { name }` and `Harmony { symbol }` are point occurrences at the time their `measure:beat` coordinate names.
- `KeyMap`, `MeterMap`, and the section and harmony lanes of `AnnotationStore` are **projections** of those occurrences,
  reproducing byte for byte what the direct lowerer emits — which is what `fixtures_have_full_parity` checks.

The promise of §21 — that these arrive "without any kernel change" — is therefore demonstrated rather than asserted:
prompt 40 touched no file in `musa-kernel`. When `modulate` or a mid-piece `meter` arrives, the elaboration emits a
region with a narrower span and nothing else changes; this document is extended, not repaired, at that prompt.

Two consequences worth stating, because a later reader will otherwise re-derive them:

- **Positions resolve against the meter *occurrence*, and against the timeline's own extent.** Neither is recomputed
  from the snapshot. The extent is exact rather than a maximum over event ends, and the two agree only because a written
  rest is an occurrence (prompt 39) — a piece that ends in silence ends where the silence ends, which
  `a_piece_that_ends_in_a_rest_ends_where_the_rest_ends` fixes as a fixture.
- **Piece-scoped facts sort first at a shared instant.** Their canonical key begins `*|*`, and `*` sorts before any part
  number, so the normal form prints the context a reader meets first.

## Tempo stays out (§22)

Tempo never elaborates into kernel occurrences and never rescales kernel time. It is the performance layer's monotone
map `Beat → Second` applied to symbolic positions at realization time (`07-backend-contract.md`). "Stretch the material"
(payload/time action, D5) and "perform the same material more slowly" (tempo map) remain different operations.

## Adapter contract (ScoreSnapshot)

The projection reads the piece's one `Timeline[ScoreFact]` back out as a `ScoreSnapshot`:

- note and rest occurrences → `ScoreEvent`s (id assignment deterministic: visit order, part- and voice-major);
- same-span/same-scope/same-origin groups → `Chord`; singletons → `Note`;
- payload `scope` → part/voice lanes; payload `origin` → event `Origin` verbatim;
- region and point occurrences → the annotation lanes, resolved to the ids at their ends;
- piece-scoped occurrences → `KeyMap` and `MeterMap`.

**The regression net (was: the parity requirement, §30 Step 5).** Through prompt 40 this section required the adapter's
snapshot to equal the frozen direct lowerer's on positions, durations, spelling, part/voice identity, multiplicity,
ordering, and provenance. Prompt 41 deleted that lowerer — a frozen second implementation of a shrinking subset is a
second answer to "what does this piece mean", not a safety net — and the net is now what it should always have been:

- the `examples/*.musa` corpus with `insta` goldens at every backend (MEI, `LilyPond`, `MusicXML`, MIDI,
  `NotationPlan`);
- the law suites (`transform_laws`, `notation_details_laws`, `annotation_laws`, `profile_laws`, `import_laws`);
- the kernel's own property tests, and the kernel normal form of every fixture;
- a generated corpus checked against **its own text** — one event per written statement, a voice as long as the
  durations written in it — which is strictly more direct than a comparison, since a bug both paths shared was invisible
  to the comparison.

The phase-2 constructs above (tie, articulation, dynamic, slur, tuplet) needed no such boundary once there was one path;
the rejection diagnostics the frozen lowerer carried (`` `<construct>` needs the kernel elaboration path ``) went with
it.

## What elaboration must never do

- Introduce rest/silence occurrences to "fill" regions the author left empty (§2) — a `rest` the author wrote is
  material and elaborates to an occurrence; a gap is not.
- Push production history into kernel semantics (e.g. making equality motif-aware) (§20).
- Add kernel constructs because one surface feature is awkward — awkwardness is elaboration's problem (§34).
- Change the surface grammar to make elaboration easier (§35.9).
