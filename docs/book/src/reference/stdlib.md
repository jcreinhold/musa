# Standard library

The bundled standard library is version-matched Musa source, imported explicitly with `import std::<module>;`. `std` is
reserved: it is never searched in the working directory or the environment, and there is no implicit prelude. Standard
definitions are ordinary Musa definitions — their source is available read-only at stable `musa-stdlib:/std/…` URIs for
hover and go-to-definition, and customizing one means writing a local wrapper.

The modules:

| Module | Contents |
| --- | --- |
| `std::algebra` | `Group`, `Action`, and `Torsor`: the three structures the musical domains share |
| `std::collections` | Scale collections: the modes, harmonic and melodic minor, pentatonic, whole-tone, octatonic |
| `std::context` | The `TonalContext` record and the `c_major` / `a_natural_minor` values |
| `std::core` | Identity and composition combinators |
| `std::cyclic` | `Cycle(n)` and `Cyclic(n)`: a division of the octave or of a pulse cycle, and a position in one |
| `std::harmony` | Chord classes: roots, bass, members, inversions, slash basses, the triad refinement |
| `std::indexed` | Families whose constructors choose their index: `Equal`, the length-carrying `Row`, and `Measure` |
| `std::list` | Finite lists: `range`, `repeated`, `map`, `filter`, and the two folds |
| `std::nat` | `nat_fold`: counting upward, told which repetition it is in |
| `std::notation::staff` | Staff documents as data: written values, the items on a staff, and realizing them into exact time |
| `std::option` | Reading an `Option` by naming both cases |
| `std::post_tonal::pcset` | Pitch-class sets at any division: normal order, prime form, set classes |
| `std::pitch` | Pitch and interval operations |
| `std::scale` | Scales, degrees, stepwise spelling |
| `std::sound::graph` | Finite studio descriptions and validation of descriptors, ports, parameters, bindings, and cycles |
| `std::post_tonal::serial` | Tone rows at any division, and their forms |
| `std::transformational` | Neo-Riemannian transformations on triads |
| `std::voicing` | Voicing policies: close and drop positions |

## Phase modules

One directory in the package is not written in the language the rest of it is written in. `std::adapters::doubled` is an
*adapter*: a module the compiler checks and evaluates during expansion, where the syntax types are in scope, and reaches
only through a syntax import.

```musa
import syntax std::adapters::doubled as doubled;

let pair = syntax doubled { c4 };
```

The two words are two statements. An ordinary `import` brings in values and cannot change how anything is read; `import
syntax` names the package that reads a region, and it stands in the header before the first definition that uses one. A
region is named — by the name the import gave — and delimited, so the lexer and the grouper stay fixed and a package
never extends them. What is inside is the adapter's language, and until an adapter promises to *print* one, the
formatter reflows a region by the compiler's own rules rather than by the notation's.

`doubled` expands `syntax doubled { … }` to `(repeated(…, 2), 0)`: whatever the region holds, twice, paired with an
*anchor* — a number the adapter emits and the compiler keeps a table for, so that a value produced by an expansion can
still say which part of the region it came from. It exists to be run rather than to be used — it is the phase's fixture,
and the adapters worth writing music with are their own modules.

`std::adapters::staff` is the first adapter written to be composed in rather than to be run. A region of it is a page of
staff notation: a head stating the instrument, how far its written pitch sits from its sounding pitch, the clef, the
key, the time and how written values are spelled, and then the notation — bars that each say what they measure, holding
notes, rests, chords, dots, exact durations, ties, slurs, tuplets, grace notes, pickups, repeats, alternate endings and
meter changes.

```musa
        instrument "bb_clarinet"
        transposing M2
        clef treble
        key d major
        time (4, 4)
        spelling shortest_readable
```

It expands to one call to `std::notation::staff`'s `Document` constructor, so the adapter reads and the package decides:
no sounding pitch, no performed duration, no resolved tuplet span and no grace-note timing is settled at read time.
Nothing on the page is inherited either — every note states its own register and its own written value, and every bar
states its own length — which is what lets a bar that does not hold what it says it measures be reported against that
bar. `examples/staff-page.musa` is a page that exercises every item it reads.

Every adapter declares what it promises, and the compiler checks the promise where the module is imported. A *readable*
adapter expands, and its regions are read-only; an *editable* one also answers structured commands with edits into its
own region; a *generative* one also writes a new region for a value it is handed. `doubled` is editable and says so.
`staff` is generative: it serves one command, `replace`, which puts new text where the node an anchor names stands, and
it writes a whole page back out of a `StaffDocument`.

`std::adapters::graph` is the generative reader for `std::sound::graph`. Its semicolon-terminated region states graph
inputs and outputs, instrument and processor nodes, parameters, directed connections, and instrument bindings. Reading
checks the local spelling and produces a finite `StudioDescription`; the ordinary package's `validate` then checks
descriptor contracts, exact port kinds and directions, parameter units and ranges, binding targets, and instantaneous
cycles. The adapter allocates no processor and advances no signal. It serves `set_parameter` as an anchored minimal edit
and prints a canonical graph; [`examples/live-studio.musa`](../../../../examples/live-studio.musa) is the complete
trial.

Writing back is not the same claim as reading. A printed page says what the value said — realize the page a printer
wrote and you get the spans the value held — but it is new text, so it preserves no comment, no blank line, and no
origin from any page that came before it. Editing is what preserves those, and it stays a separate operation for that
reason. And a page the staff spelling cannot write is a stated loss rather than a smaller page: a part transposed by an
interval with no written name is refused with a sentence saying so, not printed with the transposition left out.

The reference below is generated from the source comments in the bundled modules.

{{#include ../../../../stdlib/reference.md:3:}}
