# Standard library

The bundled standard library is version-matched Musa source, imported explicitly with `use std::<module>;`. `std` is
reserved: it is never searched in the working directory or the environment, and there is no implicit prelude. Standard
definitions are ordinary Musa definitions — their source is available read-only at stable `musa-stdlib:/std/…` URIs for
hover and go-to-definition, and customizing one means writing a local wrapper.

The modules:

| Module | Contents |
| --- | --- |
| `std::collections` | Scale collections: the modes, harmonic and melodic minor, pentatonic, whole-tone, octatonic |
| `std::context` | The `TonalContext` signature and its `CMajor` / `ANaturalMinor` modules |
| `std::core` | Identity and composition combinators |
| `std::harmony` | Chord classes: roots, bass, members, inversions, slash basses, the triad refinement |
| `std::list` | Finite lists and folds |
| `std::option` | The `option` type and its fold |
| `std::post_tonal::pcset` | Pitch-class sets |
| `std::pitch` | Pitch and interval operations |
| `std::scale` | Scales, degrees, stepwise spelling |
| `std::post_tonal::serial` | Twelve-tone rows and their forms |
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
never extends them. What is inside is the adapter's language, which is why the formatter writes it back as it stands.

`doubled` expands `syntax doubled { … }` to `repeat(…, 2)`: whatever the region holds, twice. It exists to be run rather
than to be used — it is the phase's fixture, and the adapters worth writing music with are their own modules.

The reference below is generated from the source comments in the bundled modules.

{{#include ../../../../stdlib/reference.md:3:}}
