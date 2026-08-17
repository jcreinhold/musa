# 06 — Surface Elaboration

How the `.musa` surface language elaborates into event tracks. This document describes elaboration of the grammar **as
it exists today** (prompts 02–06); it is not a surface redesign. The implementation is prompt 11
(`docs/plan/prompts/11-kernel-elaboration.md`), and prompt 127a renames what it produces.

## The elaboration boundary

```text
source / musical HIR  (motifs, repeats, transforms, chords, provenance — preserved)
        ↓  elaborate / observe
finite event tracks  (flat EventTrack<WrittenTime, ScoreFact>; the semantic quotient)
        ↓  adapt
ScoreSnapshot  (the score-specific projection backends already consume)
```

Normalization is a **semantic boundary, not the internal representation of every compiler pass**. The HIR keeps
`repeat`, `loop`, motif references, and transformations for efficiency, editing, provenance, diagnostics, and structural
display; elaboration evaluates finite observations of them into event tracks. Nothing requires duplicating thousands of
nodes merely to obey the normalized model.

Everything on this page is in `WrittenTime`. Performed time is reached by a named conversion downstream
(`07-backend-contract.md`), and nothing here may produce a value in another coordinate.

## Payload design (the central decision)

Score elaboration uses one payload type per fact domain. For the current grammar there is one essential payload:

```text
NotePayload {
    pitch: WrittenPitch        % written spelling, verbatim — never a MIDI number (roadmap §2)
    voice: VoiceIdentity       % (part, voice) source identity
    origin: Origin             % provenance: source span, declaration, expansion path
}
```

Decisions recorded against the open questions of `08-open-questions.md`:

- **Voice identity rides in payload metadata** — it is not a temporal primitive, and the coordinate remains the only
  index the track type carries (`../constitution.md` §8).
- **Provenance rides in the payload**: the core quotient forgets production history, so history travels with each
  occurrence as data the core is opaque to. `Origin` never participates in temporal semantics; it participates in
  canonical payload serialization only as a stable, deterministic key (N3), so semantic equality can still distinguish
  occurrences a consumer must tell apart.
- **No duration field in the payload** — duration is temporal support (D0).

## Elaboration rules, per surface construct

| Surface construct | Elaboration |
| --- | --- |
| `c5/4` | One occurrence of `ScoreFact::Note` over the current position's span, carrying pitch, written duration, and any articulations; the voice cursor advances by `1/4`. |
| `rest/2` | One occurrence of `ScoreFact::Rest` over the current position's span; the voice cursor advances by `1/2`. A *written* rest is notation an author asked for, and export and provenance both need it; what stays absent is unwritten silence. |
| `[c5 e5 g5]/2` | `together` of one `Note` occurrence per pitch over the same span. The projection regroups same-span, same-scope, same-origin note occurrences into `ScoreEventKind::Chord`. |
| voice body | `follow` of its items in source order (cursor semantics = left-fold of successive durations). |
| part | `together` of its voice tracks. Voice identity is the fact's `Scope`, not a track of its own. |
| piece score | `together` of its part tracks: **one** `EventTrack<WrittenTime, ScoreFact>` per compilation. |
| `let x = track_expression;` | An ordinary source binding of an ordinary event-track value. It emits nothing until it is placed. |
| `use e;` | Place the track `e` at the current voice cursor: `follow` it onto the voice under construction and advance by `duration(e)`. A shared binding plus a marked reference preserves one body and distinct call-site `Origin`. |
| `motif name(args) { … }` | A source function returning an event track. Its `use` follows the preceding rule; there is no motif-only evaluator. |
| `repeat n { … }` | HIR-level `follow` of `n` evaluations; each iteration's occurrences gain the `RepeatIteration(i)` provenance step. |
| `transpose up P5 { … }` | `map_payloads` with the transposition function on `pitch`; occurrences gain the `Transposition` provenance step. |
| `c4/4 ~` (tie) | **No core construct, and no fact.** A tie says two written noteheads spell *one* occurrence, so elaboration merges the tied statement with its continuation on the spot: one occurrence, span the sum, `NotatedDuration` the compound spelling. Merging happens at every nesting level, so a tie inside a `retrograde` is gone before the block is reversed and needs no repair. A tie onto a different pitch, or with nothing after it, is a diagnostic. |
| `c4/4 accent staccato` | Articulations are a **field of the note fact**, not facts of their own: a staccato dot has no span and no identity apart from its note. The projection emits one `ArticulationMarking` per name, in written order, against the event's id. |
| `dynamic mf;` | A **point** occurrence of `ScoreFact::Dynamic` at the cursor, with no cursor advance. The projection resolves it to the first event at or after it in the same voice; nothing after it is a diagnostic. |
| `slur { … }` | The body elaborates unchanged, and one `ScoreFact::Slur` occurrence is placed **together** with it over `[0, duration)`. Nothing is copied onto the notes. The projection emits a `SlurSpan` naming the first event at or after the region's start and the last ending at or before its end. `phrase` and `crescendo`/`diminuendo` work identically. |
| `tuplet n/d { … }` | The body elaborates with the voice's duration scale multiplied by `d/n`, so written values become exact rationals (`3/2` of eighths gives `1/12`). A `ScoreFact::Tuplet` occurrence is placed together with the body, and the projection emits a `TupletSpan` carrying the unreduced `n/d`, which is what the backends need to print the bracket. A tuplet that would cross a barline is a diagnostic. |
| `performance { profile v { … } }` | **Nothing elaborates.** A profile is a reading of marks, not material: it produces no occurrence, occupies no time, and is carried on the snapshot beside the motif table for the performance layer to consult. Written marks stay written. |
| `profile v;` inside a part | **A binding, not an occurrence.** It names which profile realizes this part; naming an undeclared one is a diagnostic. Both semantic paths read it through the same `part_metadata`, so it cannot drift between them. |
| `key c major;`, `meter 4/4;` | **Region occurrences** of `ScoreFact::Key`/`Meter`, scoped to the piece and spanning `[0, d]`. An unwritten meter still produces a fact — 4/4 governs a piece that never says so — while an unwritten key produces none, which is why the projection's `key` is an `Option` and its `meter` is not. |
| `section "A" at 9:1;`, `harmony { at 1:1 c; }` | **Point occurrences** of `ScoreFact::Section`/`Harmony` at the time the coordinate names. The coordinate is resolved against the *meter occurrence* and the track's own duration; naming a place the piece never reaches is a diagnostic. |
| `tempo 1/4 = 60;`, `tempo 1/4 = 90 at 9:1;` | **Not a fact, ever**. Tempo is the performance layer's `WrittenTime → PhysicalTime` map; it stays on the snapshot's `TempoMap`. See "Tempo stays out" below. |
| piece | The one track **projected** into `ScoreSnapshot` (`musa-compiler/src/project.rs`): voices, context maps, and annotations alike. |

## The payload, and the adapter contract

A `ScoreFact` has exactly three axes, and they vary independently:

- **`scope`** — where in the score's *structure* the fact sits: one voice of one part, or the piece as a whole. Not
  where it is in time.
- **`kind`** — what is stated: a note, a rest, a slur, a phrase, a tuplet, a dynamic, a hairpin, a key, a meter, a
  section, a chord symbol.
- **`origin`** — why it exists. Provenance stays above the core.

*Where it is in time is the occurrence's span*, and is never a field. Keeping the three apart is the point: a slur moves
in time without changing voice, a voice is renamed without moving anything, a dynamic changes from `mf` to `f` in place.

The projection (`project.rs`) visits the canonically ordered occurrences **once**, buckets them by scope, assigns
`EventId`s to note and rest facts in visit order, and resolves region facts to the event ids at their ends. It relies on
one invariant, stated on the function and asserted in debug builds: **a region fact's boundaries coincide with event
boundaries in its own scope**, because a region is built from the duration of the items it encloses. If that is ever
violated, the elaboration that violated it is the bug.

## Sharing and provenance: how `repeat` and `use` elaborate (prompt 49, repaired at prompt 142)

**Elaboration no longer emits a kernel term.** Prompt 142 moved the surface onto a dependent core
(`../language/02-core-calculus.md`): a `.musa` document elaborates into a **core** program, the core evaluates it by
normalization, and a `musa_kernel::Term` is *projected* from the result — one literal per voice, plus one for the
piece-wide context. The kernel term is an output of compilation now, not the shape compilation is carried in. That
relocates everything this section decided, and the three claims below are what became of it.

**The saving is kept, and kept where it was argued for.** The claim was never that a track is stored once; it was that
"the CST is walked once, pitches resolved once, diagnostics emitted once". A motif is one core definition, elaborated
from the text once and applied at each call site, so all three hold by construction rather than by a sharing table the
elaborator maintains — and they now hold for every reusable thing the language has, not only for `repeat` and `use`.
What the `let` bought *in addition* — a printed term whose body appears once — is gone. Normalization is what a core
evaluator does, and normalization is what spends sharing, so it is spent everywhere rather than at the four levels the
old rule enumerated. That list is deleted rather than corrected: a rule naming the four places a value is needed
describes an evaluator that stops elsewhere, and this one does not stop.

**Provenance is no longer carried at the reference, because nothing collides.** The mark existed because occurrences
stated once inside a `let` could not each carry a different `RepeatIteration`. With no shared body there is no such
occurrence: every one is built by the voice's own left fold and carries its own `Origin`, and each enclosing transform
records its step at the *front* of the expansion path, so the path reads outside-in the way
`../desktop/04-provenance.md` requires. The mark grammar, its `depth`/`origin-span`/`scope`/`steps` fields, and the
`u32::MAX` placeholders a shared body carried where the call would supply a span and the voice a scope, are all gone
from what the compiler writes.

**Marks stay in the kernel language, and `musa-compiler` is now only their reader.** `10-term-calculus.md` T6 is
unchanged: a `.musa.kernel` file may write `x @ m`, and a reader of one must still evaluate it. What changed is that no
compiler pass writes one. `let` survives in the kernel term for its other use, which was never about motifs: a kernel
quote binds its holes with it — `Term::bind`, first hole outermost — and that is the kernel's own call-by-value sharing.

**What it costs, stated plainly.** An interchange file is larger. `examples/kernel/variation.musa.kernel` had one `let`
for five `use`s; it now writes every occurrence of all five out, and no file in `examples/kernel/` contains a shared
binding or a `4294967295` placeholder. The rejected repair was to rebuild the sharing inside the projection, reading
common expansion-path prefixes back out of the evaluated tracks. It was refused because it re-derives in an output stage
what the core knew and normalization deliberately discarded, which is the mistake this repository names in `AGENTS.md` —
hand a consumer what we already computed, rather than making a later stage recover it.

**What does not share, and never did.** `transpose`, `invert` and `stretch` are payload maps and time scaling applied
during elaboration; `scale` has a term and the payload maps do not, and inventing one would breach the calculus's absent
list. Voices become `together` and voice items `follow` — structural, and what makes a printed file legible.

## Reusable material is an ordinary value (amended at prompt 127a)

Earlier drafts of this section specified a **contextual `Music`** type: a value that read an ambient placement, voice
scope, and duration scale supplied at each use, observed by a private `instantiate`/`close` pair. Prompt 127a deletes
it. There is no contextual universal `Music` in Musa, and this section says what replaces it.

Reusable material is an ordinary source value of an ordinary type:

- a **fragment** is a value of type `EventTrack<WrittenTime, ScoreFact>`;
- a **motif** is a source function returning one;
- **placement is not captured, it is applied**: `use e;` is `follow(voice_so_far, e)`, and the cursor is elaboration's
  own left fold, not something the value reads.

Three things follow, and they are why the change is a simplification rather than a rename:

- **Placement is an argument, so it cannot be ambient.** The old account had to say what a `music` value captured and
  what it deliberately did not (no absolute beat, no voice scope, no mutable key/meter/tempo/clef state). A track value
  captures nothing, so the list disappears rather than needing to be enforced.
- **Call-site differences travel on the reference mark**, exactly as above. Scope and origin are rewritten by a payload
  map chosen from the mark; the temporal facts are untouched, which is what makes sharing safe.
- **Structural mutations stay in the piece/voice walk.** `key`, `meter`, `tempo`, and `clef` changes are rejected inside
  reusable material for the same reason as before: "from here onward" has no unique meaning in a value usable at several
  places. That reason never depended on `Music` being contextual.

The source-level roles survive unchanged, because they were always roles rather than types:

| Written form | Meaning | Retained role |
| --- | --- | --- |
| `let x = track_expression;` | ordinary binding | ordinary value |
| `fn f(p: T) -> EventTrack[WrittenTime, ScoreFact] { … }` | ordinary function | ordinary function |
| `motif f(p: T) { body }` | the same function, written musically | `Motif` |
| `fragment x { body }` | the same binding, written musically | `Fragment` |
| named `bar x { body }` | a binding used at its declaration and by `use x` | `Bar` |
| `use e;` | `follow` `e` onto the voice under construction | none |

Roles preserve diagnostics, lints, mobile eligibility, editor identity, and Origin vocabulary; they do not select a
second semantic implementation, and they are not types.

`instantiate`, `close`, `KernelFragment`, and the `Music`/`ContextualMusic` types are on the clean-break ledger
(`../../plan/clean-break-ledger.md`): prompt 127e deletes them rather than aliasing them. Until it does, the compiler
still contains them, and `../across-stages/05-metatheory.md` §3 is where that gap is tracked.

## `ScoreFact`'s interchange text form (prompts 48, 86)

A kernel file carries payloads as opaque quoted strings (`01-grammar.md`); this is what `ScoreFact` puts inside one. It
is specified here, with the payload, rather than in the grammar, because the core neither writes it nor reads it.

A label is a **flat, whitespace-separated stream of words**. Flat is load-bearing: the first form nested five separators
five deep and escaped each level again at the next, so one colon inside a motif call reached the file as eight
backslashes. Nothing here nests, so nothing is escaped twice.

A word is either **bare** — no whitespace, no `'`, no `\`, no bracket — or **quoted**, `'…'` escaping `\` and `'`. Every
free-text field is quoted *always*, even where quoting would not be needed: that is what keeps `mark text '8'` and
`mark ottava 8` apart without case analysis, and it means a payload never contains `"`, so the core's own string escape
has nothing to double.

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
- **Elision is a biconditional, never a guess.** `def` is written iff the definition span differs from the source span;
  `#n` iff the declaration is not zero; `via` iff the expansion path is non-empty; `spelled` iff the spelling differs
  from how a ratio is written; `tied` iff the pieces are not exactly the one value. The span itself is never elided, so
  `[0:0]` is written as it stands.
- **Every rational is `p/q`, or `p` when the denominator is one.** Durations still carry the written spelling *and* the
  exact value *and* the tied pieces where those differ, because a tuplet keeps the symbol while changing what it sounds
  for and no one of the three derives the others.
- **A hairpin's shape is its `Progress` in canonical form** — the one place where N3's key and the interchange text
  coincide, because a `Progress` has no provenance to quotient away.
- **`tied` is absent.** It is elaboration-only and false on every fact that leaves elaboration: a tie says two noteheads
  spell one occurrence, which is resolved before a track exists. A file carrying it would describe a state no track is
  ever in.

A reference's mark is the same word stream — `depth <n> [ "origin" <span> ] [ "scope" <scope> ] [ "via" <step>… ]` — so
a file has one tokenization and one escape rule throughout.

Everything the value holds is present, including the definition span and the declaration id that `canonical_key` (N3)
deliberately drops. That is why these are two functions and not one: N3 is the *equality* serialization and may
quotient; interchange must reproduce. `05-normalization.md` N3 states the same repair from the other side.

## Key, meter, harmony: the present shape

Key, meter, and harmony are **regions**: typed interval payloads in the core whenever their temporal extent matters —
e.g. `modulate to C major { … }`. Prompt 40 put them there **before** the surface grew such a construct, and that order
was deliberate: a region that happens to cover the whole piece is not a special case, but a piece-wide scalar called
`MeterMap` is. Modelling meter as one region over `[0, d]` now means the later change adds *more occurrences* rather
than a second way to ask the same question (PoSD ch. 10).

So today:

- `Key { tonic, mode }` and `Meter { numerator, denominator }` are occurrences over `[0, d]`, scoped `Scope::Piece`.
- `Section { name }` and `Harmony { symbol }` are point occurrences at the time their `measure:beat` coordinate names.
- `KeyMap`, `MeterMap`, and the section and harmony lanes of `AnnotationStore` are **projections** of those occurrences,
  reproducing byte for byte what the direct lowerer emits — which is what `fixtures_have_full_parity` checks.

The promise that these arrive "without any core change" is therefore demonstrated rather than asserted: prompt 40
touched no file in `musa-kernel`. When `modulate` or a mid-piece `meter` arrives, the elaboration emits a region with a
narrower span and nothing else changes; this document is extended, not repaired, at that prompt.

Two consequences worth stating, because a later reader will otherwise re-derive them:

- **Positions resolve against the meter *occurrence*, and against the track's own duration.** Neither is recomputed from
  the snapshot. The duration is exact rather than a maximum over event ends, and the two agree only because a written
  rest is an occurrence (prompt 39) — a piece that ends in silence ends where the silence ends, which
  `a_piece_that_ends_in_a_rest_ends_where_the_rest_ends` fixes as a fixture.
- **Piece-scoped facts sort first at a shared instant.** Their canonical key begins `*|*`, and `*` sorts before any part
  number, so the normal form prints the context a reader meets first.

## Tempo stays out

Tempo never elaborates into occurrences and never rescales written time. It is the performance layer's monotone map
`WrittenTime → PhysicalTime` applied to symbolic positions at realization time (`07-backend-contract.md`). "Stretch the
material" (payload/time action, D5) and "perform the same material more slowly" (tempo map) remain different operations,
and the coordinate tag is what now makes confusing them a type error.

## Adapter contract (ScoreSnapshot)

The projection reads the piece's one `EventTrack<WrittenTime, ScoreFact>` back out as a `ScoreSnapshot`:

- note and rest occurrences → `ScoreEvent`s (id assignment deterministic: visit order, part- and voice-major);
- same-span/same-scope/same-origin groups → `Chord`; singletons → `Note`;
- payload `scope` → part/voice lanes; payload `origin` → event `Origin` verbatim;
- region and point occurrences → the annotation lanes, resolved to the ids at their ends;
- piece-scoped occurrences → `KeyMap` and `MeterMap`.

**The regression net.** Through prompt 40 this section required the adapter's snapshot to equal the frozen direct
lowerer's on positions, durations, spelling, part/voice identity, multiplicity, ordering, and provenance. Prompt 41
deleted that lowerer — a frozen second implementation of a shrinking subset is a second answer to "what does this piece
mean", not a safety net — and the net is now what it should always have been:

- the `examples/*.musa` corpus with `insta` goldens at every backend (MEI, `LilyPond`, `MusicXML`, MIDI,
  `NotationPlan`);
- the law suites (`transform_laws`, `notation_details_laws`, `annotation_laws`, `profile_laws`, `import_laws`);
- the core's own property tests, and the normal form of every fixture;
- a generated corpus checked against **its own text** — one event per written statement, a voice as long as the
  durations written in it — which is strictly more direct than a comparison, since a bug both paths shared was invisible
  to the comparison.

The phase-2 constructs above (tie, articulation, dynamic, slur, tuplet) needed no such boundary once there was one path;
the rejection diagnostics the frozen lowerer carried (`` `<construct>` needs the kernel elaboration path ``) went with
it.

## What elaboration must never do

- Introduce rest/silence occurrences to "fill" regions the author left empty — a `rest` the author wrote is material and
  elaborates to an occurrence; a gap is not.
- Push production history into core semantics (e.g. making equality motif-aware).
- Add core constructs because one surface feature is awkward — awkwardness is elaboration's problem.
- Change the surface grammar to make elaboration easier.
- Produce a value in a coordinate other than `WrittenTime`, or a machine. Elaboration builds one of the two core values
  and never the other.

## The candidate language, restated after prompt 127a

The rules above remain the governing account of the implemented grammar. `../language/` is the candidate contract for
prompts 93–166 and becomes governing only after prompt 172. Its staging, after this amendment, has one fewer stage than
it used to:

```text
typed total inferred expression → EventTrack<WrittenTime, ScoreFact>
```

There is no intermediate contextual `Music`, no `instantiate`, and no `close`. Elaboration builds track values directly,
retains sharing in a private acyclic binding environment as `10-term-calculus.md` describes, and checks the resulting
term before the semantic boundary evaluates it. It does not define or assume
`EventTrack<C, EventTrack<C,A>> → EventTrack<C,A>` (D12).

A local `kernel EventTrack[WrittenTime, ScoreFact] { ... }` quote uses typed antiquotation `${e}`. Each hole must have
type `EventTrack[WrittenTime, ScoreFact]` and is substituted capture-avoidantly into the existing term grammar. The
completed term must be closed and every payload must decode as `ScoreFact`. Standalone `.musa.kernel` documents remain
exactly the closed calculus of `10-term-calculus.md`.

Parameterized pieces and voices use a static declaration template stage before context tracks are built. Templates do
not make pieces, voices, modules, syntax, or core terms first-class value types. The candidate also leaves performance
gestures, instrument signatures, raw or decoded assets, physical seconds, machines, and mix routing outside this
elaboration. A musical clip may elaborate to an interval `ScoreFact` and a fixed-media cue to a point `ScoreFact`; each
contains only an opaque `AssetRef` and score-level settings. The core never receives sample data or a fixed media
duration and remains musically and media opaque to both payloads.
