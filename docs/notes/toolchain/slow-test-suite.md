# A slow test suite on macOS is a full `target/debug/deps`

## The symptom

`cargo nextest run --workspace` appears to hang. No output, no failures, no progress for tens of minutes. Test binaries
sit at **0.0% CPU** and a few kilobytes resident, starting one at a time, seconds to minutes apart, in alphabetical
order. Killing and rerunning seems to help for a while, then it comes back.

## The cause

`target/debug/deps` fills up with object files, and macOS's per-exec provenance check degrades badly with the number of
entries in the directory an executable lives in.

Both halves are needed. The directory grows because an object file's name carries two components that change under you,
and cargo reclaims neither:

```
musa_compiler-369feb4f79724dca.d4nupuuyz6fiwa4t7payhyne5.0zv1c4l.rcgu.o
              ^metadata hash    ^codegen-unit hash        ^build session
```

The **metadata hash** fingerprints the unit's inputs — feature set, profile, `--cfg`, dependency versions, lib-versus-
test. Change any of them and rustc emits a whole new stem's worth of objects beside the old stem, which stays. One
measurement found 66 crate names holding two retained variants, eight holding four to six; `musa_compiler` had two live
stems at 512 objects each. The **build session** changes on every recompile of the same stem, so half the stems carried
two sessions and seven carried three.

On that measurement this repository had **776,817 entries and 23 GB in `deps` alone**, of which **769,236 were `.o`
files**, 25,344 of them for a single crate hash — and all of them less than seven days old. It is not old cruft; it
accumulates that fast.

The exec cost then scales, superlinearly, with that count. Same binary, same bytes, only the directory changed:

| Entries in the containing directory | First exec |
| --- | --- |
| 100,000 (synthetic) | 0.45s |
| 400,000 (synthetic) | 1.29s |
| 776,817 (a real `deps`) | **41.4s** |

And by location, with one binary copied around a repository in that state:

| Location | First exec |
| --- | --- |
| `/private/tmp/…` | 0.33s |
| `~/` | 0.34s |
| the repository root | 0.30s |
| `target/` | 0.29s |
| **`target/debug/deps/`** | **41.4s** |

Zero CPU throughout. The process is not computing; `syspolicyd` is doing a provenance lookup and the executable is
blocked in the kernel until it answers. The verdict is cached per path, so a second run of the same binary is
instantaneous — which is what makes the problem look intermittent.

## The fix

```sh
cargo clean
```

On the measurement above this removed 963,138 files and 64 GB, and returned first-exec to ~0.3s. Do it whenever the
suite starts feeling slow; the check below says whether it is due.

```sh
ls -1 target/debug/deps | wc -l          # tens of thousands is fine; hundreds of thousands is not
du -sh target/debug/deps
```

The `.cargo/config.toml` profile settings —

```toml
[profile.dev]
debug = "line-tables-only"
[profile.dev.package."*"]
debug = false
```

— are working and worth keeping, but they act on the wrong axis for this symptom. They control how *large* the debug
info is, not how *many* files exist. They keep the directory from being far bigger than 23 GB; they cannot keep it from
holding 769,236 entries.

## Slowing the refill: one test binary per crate

`cargo clean` treats the symptom. The growth *rate* is set by how many compilation units a build has, because each one
emits its own set of objects and each is re-emitted whenever anything it depends on changes.

Cargo makes every file directly under `crates/<crate>/tests/` its own integration-test target — its own binary, its own
link of the whole workspace, its own object set. This repository had **103** of them, against 10 library crates. So
touching `musa-compiler/src/lib.rs` did not rebuild one thing; it rebuilt the library and re-emitted every test binary
downstream of it.

They are now consolidated: the files live in `crates/<crate>/tests/suite/`, and `tests/suite/main.rs` declares one `mod`
per file, which cargo builds as a single target named `suite`. Ten integration-test binaries instead of 103, and the
same 1,243 tests. Measured on the same machine, `cargo clean` first in both cases:

|  | Before (103 targets) | After (10) |
| --- | --- | --- |
| Cold `cargo nextest run --workspace` | 1:34 | **46s** |
| `deps` entries after it | 11,107 | **5,448** |
| Rebuild after touching `musa-compiler/src/lib.rs` | 1:02 | **15s** |
| `deps` entries that rebuild added | ~17,000 | **3,598** |

Three things the layout depends on, so that moving a test file does not quietly break them:

- **A path in `include_str!` is relative to the file that writes it**, so it gained one `../`. A path built from
  `env!("CARGO_MANIFEST_DIR")` is relative to the crate and did not change.
- **insta names a snapshot file after the test target**, so every snapshot gained a `suite__` prefix. Anything that
  reads those files by name — `elaboration_compatibility.rs` digests the render snapshots — records the fixture's own
  name rather than the file's, so the oracle does not move when the harness is rearranged.
- **proptest keeps its regression seeds beside the source file**, so `*.proptest-regressions` moved with it.

The cost is that touching one test file now recompiles its whole crate's suite rather than one small binary. That is the
trade: a slower edit-test loop on a single test file, against a build that does not leave 17,000 files behind.

## Confirming it, and a trap

To watch the check happen:

```sh
/usr/bin/log show --last 10m --style compact --predicate 'process == "syspolicyd"'
```

**Use the absolute path.** `log` is shadowed by a shell function in at least one shell configuration here, and the
shadowed version fails with `too many arguments` on stderr while printing nothing to stdout — so the query looks like it
ran and found nothing. Every attempt to diagnose this before that was noticed came back empty and was written up as
"unresolved".

In the log you will see, once per newly executed binary, a certificate trust evaluation, an attempted notarization
lookup (`Error checking with notarization daemon: 3`), and then
`Putting executable into provenance with metadata: TA(…)`.

## What was wrong before

This page previously said the cost was Gatekeeper assessing each *newly linked, unsigned* binary, priced per binary at
around 24 seconds, and that the only remedies were scoping `-p`, running gates one at a time, and expecting the first
run after a wide change to be slow. That diagnosis was wrong, and its advice merely worked around the symptom.

Three specific claims did not survive:

- **"Never-executed content is what is expensive."** A freshly compiled 33 KB binary ran in 0.144s, and a
  content-modified 16 MB copy of a test binary ran in 0.472s — both never seen before. Novel content is not the trigger.
- **"The verdict is keyed on content, not path."** Backwards. Byte-identical copies differ by 100× depending only on
  which directory they sit in, and the cached verdict follows the path.
- **"`sudo spctl developer-mode enable-terminal` is worth trying."** It is not the lever, and neither is
  `com.apple.provenance` — every file under `target/` carries that xattr, including the fast ones.

## What this is not

It is not a reason to mark tests `#[ignore]`. The tests are fast; the exec was not, and the cause was a directory that
needed emptying. Marking a test slow for this would hide a housekeeping problem inside the test suite, where the next
person cannot see it.
