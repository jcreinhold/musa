# A slow test suite on macOS is a full `target/debug/deps`

## The symptom

`cargo nextest run --workspace` appears to hang. No output, no failures, no progress for tens of minutes. Test binaries
sit at **0.0% CPU** and a few kilobytes resident, starting one at a time, seconds to minutes apart, in alphabetical
order. Killing and rerunning seems to help for a while, then it comes back.

## The cause

`target/debug/deps` fills up with object files, and macOS's per-exec provenance check degrades badly with the number of
entries in the directory an executable lives in.

Both halves are needed. The directory grows because a `.rcgu.o` name contains a per-rebuild component, so every
incremental rebuild writes a fresh set and cargo never removes the superseded ones. On one measurement this repository
had **776,817 entries and 23 GB in `deps` alone**, of which **769,236 were `.o` files**, 25,344 of them for a single
crate hash — and all of them less than seven days old. It is not old cruft; it accumulates that fast.

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
