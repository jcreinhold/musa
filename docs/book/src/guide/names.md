# When you need a name

Musa has four ways to write something once and use it more than once. They are not interchangeable, and picking the
right one is mostly a matter of noticing what varies.

| What varies | Reach for |
| --- | --- |
| nothing — the same phrase, repeated | a `motif` |
| a pitch, an interval, a duration | a `fn` returning `EventTrack<WrittenTime>` |
| a whole declaration: a piece, a voice | a `template` |
| a *bundle* of facts that must travel together | a `signature` and a `structure` |

## 1. Functions over music

A function takes values and returns one. `EventTrack<WrittenTime>` — a track of written events, which is what `music { …
}` builds — is an ordinary value, so a function can take music and return music. From `examples/canon-functions.musa`:

```musa
fn canon(
    subject: EventTrack<WrittenTime>,
    answer: EventTrack<WrittenTime> -> EventTrack<WrittenTime>,
    gap: Duration<WrittenTime>,
) -> EventTrack<WrittenTime> { together(subject, shift(gap, answer(subject))) }
```

`EventTrack<WrittenTime> -> EventTrack<WrittenTime>` is a function type, so `answer` is a transformation the caller
supplies rather than one this function picked:

```musa
let octave_answer: EventTrack<WrittenTime> -> EventTrack<WrittenTime> = fn (
    line: EventTrack<WrittenTime>,
) -> EventTrack<WrittenTime> { transpose(P8, line) };
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

fn harmonize(
    subject: EventTrack<WrittenTime>,
    answer_pitch: Pitch -> Pitch,
) -> EventTrack<WrittenTime> { together(subject, map_note_pitches(answer_pitch, subject)) }
```

The restriction is what keeps a function from becoming a second score model. A harmonizer written this way cannot see
barlines, key signatures, chord symbols, or the temporal structure — it sees the pitches, which is what a harmonizer
needs.

## 2. Templates: a declaration with parameters

A function returns a value. A template parameterizes a *declaration* — a piece, a voice, or a structure — so the whole
thing can be made more than once. From `examples/template-study.musa`:

```musa
// A voice that answers a subject through whatever transformation it is
// handed. Twice below: the same body, two instances, two identities.
template voice answer(
    subject: EventTrack<WrittenTime>,
    transform: EventTrack<WrittenTime> -> EventTrack<WrittenTime>,
) {
    use transform(subject);
}
```

and a whole piece can be one:

```musa
// The piece itself is the template. Its key and scale arrive as arguments,
// so the study exists in whatever key it is made in.
template piece study(k: Key, mode: Scale, subject: EventTrack<WrittenTime>) "Study" {
    meter 4/4;
    key k;
```

A template is instantiated with `make ... as ...`:

```musa
make answer(
    subject,
    fn (line: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> {
        transpose(P8, line)
    },
) as upper;
make answer(
    subject,
    fn (line: EventTrack<WrittenTime>) -> EventTrack<WrittenTime> {
        transpose(P15, line)
    },
) as higher;
```

Two rules make instances predictable.

**A template body reads only its own parameters and the file's lexical root** — never the site that made it. An instance
therefore means the same thing wherever it stands, and moving a `make` cannot change what it makes.

**Identity is generative and comes from the site.** Making the same template twice with equal arguments still produces
two declarations. `upper` and `higher` above are two voices, not one voice mentioned twice, and the editor can tell you
which notes came from which.

## 3. Signatures and structures: facts that travel together

Sometimes what you want to pass is not one value but a small bundle that is useless when separated: a key, the
collection it steps through, how a degree gets spelled in register, how a chord class gets voiced. Passing them one at a
time is how they drift apart.

A `signature` names what such a bundle must provide. `std::context` declares one:

```musa
signature TonalContext {
    // The key the passage is written in. A key is a signature and a
    // tonic, never a scale.
    let tonic: Key;

    // The collection stepwise motion reads. A default, not a claim: a
    // passage may still name another collection where it wants one.
    let collection: Scale;
```

A `structure` provides one. `std::context` ships `CMajor` and `ANaturalMinor`, and either may be handed anywhere a
`TonalContext` is asked for.

A `template structure` is a function from structures to a structure. From `examples/module-functor-study.musa`:

```musa
template structure Canon(C: TonalContext, gap: Duration<WrittenTime>): CanonMaterial {
    // The subject steps through whatever collection the context named.
    let subject: EventTrack<WrittenTime> = music {
        in scale C.collection {
            c5/4
            d5/4
            e5/4
            f5/4
        }
    };
```

```musa
// Two instances, two structures. Identity is generative and comes from the
// *site*: making the same functor twice with equal arguments would still be
// two declarations, and here the arguments differ as well.
make Canon(CMajor, duration_of(1/1)) as MajorCanon;
make Canon(ANaturalMinor, duration_of(2/1)) as MinorCanon;
```

Two properties are worth knowing.

**A signature seals.** The functor above reads `C` through `TonalContext` and nothing else; whatever else `CMajor`
happens to define is private to `CMajor`. The same rule runs the other way:

```musa
// Private. `CanonMaterial` does not list it, so nothing outside this
// structure may name `MajorCanon.stretto` — which is what sealing means.
let stretto: EventTrack<WrittenTime> = together(
    subject,
    shift(duration_of(1/2), answer(subject)),
);
```

This is also why the generated reference lists a structure's signature members and not the rest: a private member is not
a name anyone can write.

**Expansion is binding, not rewriting.** The functor body is checked once per instance with `C` naming the structure the
site passed. No syntax is copied, so every span an editor points at is the one you wrote, and an error inside a functor
is reported where the functor is written rather than at four instantiation sites.

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
        time (4, 4)
        spelling shortest_readable
```

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
