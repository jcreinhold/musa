# When you need a name

Musa has four ways to write something once and use it more than once. They are not interchangeable, and picking the
right one is mostly a matter of noticing what varies.

| What varies | Reach for |
| --- | --- |
| nothing — the same phrase, repeated | a `motif` |
| a pitch, an interval, a duration | a `fn` returning `Music` |
| a whole declaration: a piece, a voice | a `template` |
| a *bundle* of facts that must travel together | a `signature` and a `structure` |

## 1. Functions over music

A function takes values and returns one. `Music` is an ordinary value, so a function can take music and return music.
From `examples/canon-functions.musa`:

```musa
fn canon(subject: Music, answer: Music -> Music, gap: Duration) -> Music {
    overlay(subject, shift(gap, answer(subject)))
}
```

`Music -> Music` is a function type, so `answer` is a transformation the caller supplies rather than one this function
picked:

```musa
let octave_answer: Music -> Music = transpose(P8);
```

and the site reads:

```musa
use canon(subject, octave_answer, 1/2);
```

`overlay` sounds two pieces of music at once; `shift` starts one later; `transpose` moves one by a written interval.
Those three, plus `stretch` for renotating durations and `retrograde` for reversal, are the whole vocabulary — there is
no fifth combinator hiding somewhere.

Music values are *contextual*: they carry no key and no scale of their own, and take on whichever is in force where they
are used. That is why `subject` in `examples/scale-context.musa` can be written once and mean two things at two sites,
and it is a deliberate property rather than a convenience (`../00-semantics.md` §3).

Traversal is controlled rather than open. You cannot walk the events of a `Music` value and look at them; what you can
do is name what each note's pitch becomes:

```musa
fn pedal(_: Pitch) -> Pitch { c3 }

fn harmonize(subject: Music, answer_pitch: Pitch -> Pitch) -> Music {
    overlay(subject, map_note_pitches(answer_pitch, subject))
}
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
template voice answer(subject: Music, transform: Music -> Music) {
    use transform(subject);
}
```

and a whole piece can be one:

```musa
// The piece itself is the template. Its key and scale arrive as arguments,
// so the study exists in whatever key it is made in.
template piece study(k: Key, mode: Scale, subject: Music) "Study" {
    meter 4/4;
    key k;
```

A template is instantiated with `make ... as ...`:

```musa
make answer(subject, transpose(P8)) as upper;
make answer(subject, transpose(P15)) as higher;
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
template structure Canon(C: TonalContext, gap: Duration): CanonMaterial {
    // The subject steps through whatever collection the context named.
    let subject: Music = music {
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
make Canon(CMajor, 1/1) as MajorCanon;
make Canon(ANaturalMinor, 2/1) as MinorCanon;
```

Two properties are worth knowing.

**A signature seals.** The functor above reads `C` through `TonalContext` and nothing else; whatever else `CMajor`
happens to define is private to `CMajor`. The same rule runs the other way:

```musa
// Private. `CanonMaterial` does not list it, so nothing outside this
// structure may name `MajorCanon.stretto` — which is what sealing means.
let stretto: Music = overlay(subject, shift(1/2, answer(subject)));
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
no file on disk to edit. Every published name is listed in [`stdlib/reference.md`](../../../stdlib/reference.md).
