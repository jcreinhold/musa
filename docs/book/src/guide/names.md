# When you need a name

Musa has three ways to write something once and use it more than once. They are not interchangeable, and picking the
right one is mostly a matter of noticing what varies.

| What varies | Reach for |
| --- | --- |
| nothing — the same phrase, repeated | a `motif` |
| a pitch, an interval, a duration | a `fn` returning `EventTrack(WrittenTime)` |
| a *bundle* of facts that must travel together | a `record`, and a `fn` over it |

## 1. Functions over music

A function takes values and returns one. `EventTrack(WrittenTime)` — a track of written events, which is what `music { …
}` builds — is an ordinary value, so a function can take music and return music. From `examples/canon-functions.musa`:

```musa
fn canon(
    subject: EventTrack(WrittenTime),
    answer: EventTrack(WrittenTime) -> EventTrack(WrittenTime),
    gap: Duration(WrittenTime),
) -> EventTrack(WrittenTime) { together(subject, shift(gap, answer(subject))) }
```

`EventTrack(WrittenTime) -> EventTrack(WrittenTime)` is a function type, so `answer` is a transformation the caller
supplies rather than one this function picked:

```musa
let octave_answer: EventTrack(WrittenTime) -> EventTrack(WrittenTime) = fn (
    line: EventTrack(WrittenTime),
) -> EventTrack(WrittenTime) { transpose(P8, line) };
```

That right-hand side is an **anonymous function**: a declaration's own words without its name. It is here because
`transpose` takes an interval *and* a passage, and a call supplies every parameter — so the interval is written down in
a function of the passage alone, rather than left out of the call. Its parameter and result types may be omitted
wherever a declaration may omit them.

and the site reads:

```musa
use canon(subject, octave_answer, duration_of(1/2));
```

`together` sounds two pieces of music at once; `shift` starts one later; `transpose` moves one by a written interval.
Those three, plus `stretch` for renotating durations and `retrograde` for reversal, are the whole vocabulary — there is
no fifth combinator hiding somewhere.

A track carries no key and no scale of its own. What a degree or a numeral inside it means is settled where the phrase
is *written*, by the context in force there, so a phrase that reads two collections is written twice or written as a
function of the collection it reads — `examples/scale-context.musa` shows both. A value that meant two things depending
on where it was later used would be a value whose meaning its own text does not fix, and this language does not have
one.

Traversal is controlled rather than open. You cannot walk the events of a track and look at them; what you can do is
name what each note's pitch becomes:

```musa
fn pedal(_: Pitch) -> Pitch { c3 }

fn harmonize(subject: EventTrack(WrittenTime), answer_pitch: Pitch -> Pitch) -> EventTrack(
    WrittenTime,
) { together(subject, map_note_pitches(answer_pitch, subject)) }
```

The restriction is what keeps a function from becoming a second score model. A harmonizer written this way cannot see
barlines, key signatures, chord symbols, or the temporal structure — it sees the pitches, which is what a harmonizer
needs.

## 2. Reusable material: a function that answers music

A function returns a value, and an event track is a value, so material a piece wants more than once is an ordinary
function. From `examples/template-study.musa`:

```musa
// A voice that answers a subject through whatever transformation it is
// handed. Twice below: the same function, two voices, two identities.
fn answer(
    subject: EventTrack(WrittenTime),
    transform: EventTrack(WrittenTime) -> EventTrack(WrittenTime),
) -> EventTrack(WrittenTime) { transform(subject) }
```

A voice folds one in with `use`, and calling the same function twice makes two voices:

```musa
            voice upper {
                use answer(
                    subject,
                    fn (line: EventTrack(WrittenTime)) -> EventTrack(WrittenTime) {
                        transpose(P8, line)
                    },
                );
            }
            voice higher {
                use answer(
                    subject,
                    fn (line: EventTrack(WrittenTime)) -> EventTrack(WrittenTime) {
                        transpose(P15, line)
                    },
                );
            }
```

Two rules make this predictable.

**A function reads only its own parameters and the file's lexical root** — never the place it is called from. A call
therefore means the same thing wherever it stands, and moving one cannot change what it answers.

**Identity is the declaration you wrote.** `upper` and `higher` above are two voices, not one voice mentioned twice, and
the editor can tell you which notes came from which — the origin of every note names the `use` that folded it in.

A whole piece is not reusable, because a file is one piece however the piece got there. What was reusable about a piece
written in several keys is its material and its parameters, and both are ordinary declarations at the file's root:

```musa
// What was a template's parameter list is the file's lexical root. The study
// exists in whatever key these say.
let mode: Scale = scale g mixolydian;
```

## 3. Records: facts that travel together

Sometimes what you want to pass is not one value but a small bundle that is useless when separated: a key, the
collection it steps through, how a degree gets spelled in register, how a chord class gets voiced. Passing them one at a
time is how they drift apart.

A `record` names such a bundle. `std::context` declares one:

```musa
    record TonalContext {
        // The key the passage is written in. A key is a signature and a
        // tonic, never a scale.
        tonic: Key;

        // The collection stepwise motion reads. A default, not a claim: a
        // passage may still name another collection where it wants one.
        collection: Scale;
```

A value of it is a context. `std::context` ships `c_major` and `a_natural_minor`, and either may be handed anywhere a
`TonalContext` is asked for.

A function from one record to another is what a functor was. From `examples/module-functor-study.musa`:

```musa
// The subject steps through whatever collection the context named. It reads
// the context through `TonalContext` and nothing else: a record's fields are
// its whole interface, so there is nothing else here to read by accident.
fn canon_subject(context: TonalContext) -> EventTrack(WrittenTime) {
    music {
        in scale context.collection {
            c5/4
            d5/4
            e5/4
            f5/4
        }
    }
}
```

```musa
// Two values, two identities. Identity comes from the site the way it always
// did: two calls are two `let`s at two origins, and here the arguments differ
// as well.
let major_canon: CanonMaterial = canon(c_major, duration_of(1/1));

let minor_canon: CanonMaterial = canon(a_natural_minor, duration_of(2/1));
```

Two properties are worth knowing.

**A record's fields are its whole interface.** `canon_subject` reads its argument through `TonalContext`, so whatever
else the value's *maker* knows is not reachable through it. What a module wants to keep is marked instead:

```musa
    // The register C major's degrees are spelled in. Private: a context
    // promises spelled pitches, not the frame it spells them from, so moving
    // this one changes nothing anyone outside can name. That is what sealing
    // was, written with the visibility the language already had.
    private let c_major_home: Option(Frame) = frame_on(scale c ionian, c4);
```

**A call is a call.** The function's body is checked once, where it is written, so every span an editor points at is the
one you wrote, and an error inside it is reported there rather than at each of its callers.

## 4. Importing

Bundled modules are ordinary Musa source, compiled into the binary, and reaching them is explicit:

```musa
import std::harmony;
import std::voicing;
```

An import never searches the filesystem and never guesses a version. `std::tonal::harmony` is a nested module path and
lives in a nested file; nothing else about it is different. Binding is flat — after the import, `close_position` is
simply in scope.

Follow a name to its declaration in the editor and you get the bundled module's own source, read-only, because there is
no file on disk to edit. Every published name is listed in [`stdlib/reference.md`](../../../../stdlib/reference.md).

## 5. The other import

There is a second word, and it is a second statement rather than a modifier on the first:

```musa
    import syntax std::adapters::doubled as doubled;

    let pair = syntax doubled { c4 };
```

An ordinary `import` brings values into scope and cannot change how anything is read. `import syntax` names a package
that reads a *region* — a piece of the file written in that package's language rather than in Musa's — and it stands in
the header, before the first definition that writes one. Reading the top of a file therefore tells you whether anything
in it can be read unusually, which is the whole reason the word is there.

A region is named and delimited: named by whatever the import called the adapter, delimited by braces the grouper
already knows. Packages do not add tokens and do not move the boundary. Inside the braces the words belong to the
adapter, and Musa complains about none of them — though until an adapter promises to print a region back, the formatter
still reflows one by its own rules rather than by the notation's.

`std::adapters::doubled` is a fixture rather than a tool — it expands a region to its contents twice, paired with the
anchor of the region it read, which is enough to watch the machinery run and no use at all in a piece.

`std::adapters::staff` is one written to be composed in. Its regions are pages of staff notation, and they need both
imports — one for the package's names, one for the adapter that reads the region:

```musa
    import std::notation::staff;
    import syntax std::adapters::staff as staff;
```

Inside the region a page opens by saying what it is written for and how it is to be read:

```musa
        instrument "bb_clarinet"
        transposing M2
        clef treble
        key d major
        time 4/4
        spelling shortest_readable
```

A signature is a count of a unit, and `time (4, 4)` says the same thing: the adapter reads a rational in the parts the
lexer found, so `4/4` is four quarters here rather than the whole note it reduces to. Write it whichever way reads
better on the page.

and then holds the notation itself:

```musa
        bar (4, 4) { c5/4 d5/4. e5/8 f5/4 ~ }
        bar (4, 4) { f5/8 rest/8 [g4 b4 d5]/4 c5(3/8) rest/8 }
```

Every note states its own register and its own written value, and every bar states its own length, so any one of them
can be read without reading the one before it — and a bar that does not hold what it says it measures is reported
against that bar. What the region expands to is one call to `std::notation::staff`'s `Document` constructor: the adapter
reads, and the package decides what the reading means. `examples/staff-page.musa` is a page that uses every item it
reads.

The same boundary serves a structurally different language. `std::adapters::graph` reads the finite studio region in
`examples/live-studio.musa` into `std::sound::graph`'s `StudioDescription`. The adapter checks the local grammar and
retains anchors; the ordinary package validates descriptor names, parameters, directed ports, bindings, and cycles.
Neither operation allocates a processor or runs a signal.

The parameter values it produces use `std::sound::quantity`, where the unit and quantity share a source type index.
Milliseconds normalize exactly to seconds there; the original token remains in the lossless source for an editor, and
only later DSP preparation converts the checked rational to a floating physical value.

The processor and studio vocabulary itself lives in `std::sound::catalogue`. Completion, hover, signatures, and the
generated reference read that checked source value, so names, documentation, and exact parameter contracts can change
with a standard-library edition without acquiring a second authoritative Rust table. Native primitives still own only
the runtime facts source cannot declare, joined by stable id and version.

The compatibility studio spelling reaches `std::sound::production` next. That module applies source-owned defaults and
builds the versioned checked production artifact; the DSP only decodes an exact read-only preparation projection from
that artifact. The compiler therefore neither publishes a studio object model nor depends on the DSP crate.

Instrument behavior follows the same ownership rule. `std::sound::instrument` declares typed signatures, indexed control
requirements, technique fallbacks, and the standard instruments as ordinary Musa values. Its registered-machine
components and graph-local parameter targets stay private to the declaring module; preparation receives their checked
projections, not a second Rust `InstrumentSpec` language.
