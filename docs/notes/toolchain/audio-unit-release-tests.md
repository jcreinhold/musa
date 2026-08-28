# The Audio Unit's Release build tests one architecture, testably

**Status: operational. Governs nothing.**

## Symptom

```sh
xcodebuild -project apps/musa-audio-unit/MusaAudioUnit.xcodeproj -scheme MusaAudioUnit \
    -configuration Release CODE_SIGNING_ALLOWED=NO test
```

fails with

```text
error: unable to resolve Swift module dependency to a compatible module: 'MusaAudioUnitKit'
@testable import MusaAudioUnitKit
```

and, once that is unblocked, with `ld: symbol(s) not found for architecture x86_64` naming every `musa_au_*` entry
point. The `Debug` build of the same scheme — which is what `scripts/check-audio-unit.sh` runs — is unaffected, so the
harness is green while the test bundle is not.

## Two settings, both about Release only

**`ENABLE_TESTABILITY`.** `@testable import` needs the framework compiled with testability, which Xcode's Release
default turns off. The Kit's tests reach `musaDecodeMIDI`, `MusaControl`, and the publication slot deliberately — those
are framework internals with no business being public, and widening the facade to test them would be the wrong repair.
The framework target therefore sets `ENABLE_TESTABILITY = YES` in both of its configurations. The project's own Release
default stays `NO`, so nothing else in the project is built testable.

**`ONLY_ACTIVE_ARCH`.** The framework links `libmusa_au.a`, which a build phase produces with `cargo build -p musa-au` —
for the host triple, and only that one. Release's Xcode default of `ONLY_ACTIVE_ARCH = NO` asks for `arm64` *and*
`x86_64`, so the second slice has no library to link against, and the missing slice is what the module-resolution error
above is really reporting. Release is set to `YES` to match what the build phase actually produces.

Making the Rust side universal instead — two `cargo build` invocations and a `lipo` — would be the fix if this project
shipped a distributable binary. It does not: `apps/musa-audio-unit` exists to be validated and measured on the machine
it is built on, and prompt 216 forbids it a signing identity, a notarization credential, and a release. One
architecture, honestly declared, is the smaller true statement.

## How this went unnoticed

The `Debug` configuration inherits Xcode's own defaults for both settings — testability on, active architecture only —
and every measurement in `scripts/check-audio-unit.sh` is a Debug build. Prompts 216–218 each named the Release `test`
invocation in their Check and each had a green harness beside it. Prompt 219's audit ran the line on its own and found
it had never passed.

The lesson is narrower than "run your checks": a scheme with a passing script beside it is not evidence the scheme's
other configuration builds, and `xcodebuild … test` failing at the *module* level usually means a link failed earlier in
an architecture nobody asked about.
