# The test suite is not slow on macOS; it is blocked

## The symptom

`cargo nextest run --workspace` appears to hang. No output, no failures, no progress for tens of minutes. Killing and
rerunning it seems to help for a while, and then it happens again after the next edit.

## How to confirm it in ten seconds

Look at the test binaries while it is happening:

```sh
ps -Ao pid,stat,etime,%cpu,rss,comm | grep target/debug/deps
```

The tell is a row like this, repeated:

```text
61123 S    05:06   0.0     32  /path/to/musa/target/debug/deps/compiler-27436e4ade67b77d
```

State `S`, **0.0% CPU**, **~32 KB resident**. A binary that is actually running tests uses CPU and tens of megabytes.
These have been execed and have never reached `main`. Watch for a minute and you will see them start one at a time,
seconds to minutes apart, in alphabetical order — the other half of the tell, because a parallel test runner does not
schedule work alphabetically. That ordering is a queue.

## The cause

macOS assesses an executable it has not seen before, through `syspolicyd`. The assessments are serialized and the
process is blocked in the kernel for the whole of one. A newly linked binary has never been seen.

Measured on this repository, on integration test binaries:

|  | Wall clock | CPU consumed |
| --- | --- | --- |
| A binary never executed before, idle machine | **23.9s** | 0.00s user, 0.00s system |
| The same situation on a machine under load | **1:05 – 4:25** | 0.00s user, 0.00s system |
| A binary already assessed, copied to a new path | **0.36s** | negligible |
| The same binary again, 100 consecutive execs | **0.245s total** (2.4ms each) | negligible |

Zero CPU throughout the slow cases. The process is not computing; it is waiting. The last two rows are what make the
cost bearable at all: it is paid **once per binary**, not once per exec.

Two facts turn one 24-second toll into a stalled afternoon:

- **The suite has around a hundred integration binaries** (`ls crates/*/tests/*.rs | wc -l`). At ~24s each that is
  roughly forty minutes, and far worse under load.
- **Everything downstream of an edit is relinked, including for a comment.** Changing a doc comment in `musa-compiler`
  recompiles the crate and relinks every test binary depending on it. A change that is a no-op to semantics is
  full-price to the queue.

The cost is not the test runner's. One binary of ten tests, timed end to end with `cargo test`:

|  |  |
| --- | --- |
| Time actually spent running tests | **0.01s** |
| Total wall clock | **2:20.00** |
| CPU consumed | 2.90s user + 4.03s system (4%) |

`cargo test` and `cargo nextest run` both pay it, because both pay it at link time. Switching runners does not help.

Machine load does not slow the blocked processes — they use no CPU. It starves `syspolicyd`, which is doing the work. On
the run that produced the 4:25 figure, an unrelated Lean build was holding nine of twelve cores.

## What to do about it

**Practical, and supported by the measurements above:**

- **Scope the gate while iterating.** `cargo nextest run -p musa-compiler` links and assesses that crate's binaries
  only. Save `--workspace` for once, at the end.
- **Run the gates one at a time.** The suite and `cargo clippy --workspace --all-targets` contend for the build lock,
  and running them together starves `syspolicyd` on top of that. Both finish later than either alone.
- **Expect the first run after a wide change to be slow, and the second to be fast.** Nothing is wrong. If a rerun is
  quick, the binaries were merely being assessed the first time.
- **Separate build cost from run cost** before concluding anything: `cargo nextest run --workspace --no-run` builds and
  links without running. The difference against the full run is exec plus test time, and almost all of it is exec.

**Unresolved: whether the assessment can be skipped.** The usual advice is to register the terminal as a developer tool,
which should exempt files created by that app:

```sh
sudo spctl developer-mode enable-terminal
```

then enable it under **System Settings → Privacy & Security → Developer Tools**. Tried here, and the result was not
interpretable: probes did get faster afterwards, but so did control probes that should not have been affected, and the
machine had also become much less loaded. No causal claim either way.

Three things to save the next person time:

- **`spctl --status` is not the check.** It reports the *global* assessment setting, which developer mode does not
  touch. It reads `assessments enabled` before and after and says nothing about whether an exemption applies.
- **The `com.apple.provenance` xattr is not the lever.** Every binary under `target/` carries one. Stripping it with
  `xattr -c` on a copy gave 0.374s — but the control that *kept* the xattr gave 0.356s. The xattr is not what is being
  keyed on.
- **The verdict appears to be keyed on content, not path.** A previously assessed binary copied to a new path with a new
  inode ran in 0.36s. One earlier measurement contradicts this — the same content at a new path took 4:25 — so treat it
  as a working model rather than established.

To watch the blocking happen live:

```sh
log stream --predicate 'process == "syspolicyd"' --info
```

If someone establishes what actually skips the assessment, replace this section with the measurement.

## What this is not

It is not a reason to mark tests `#[ignore]`. The tests are fast; the exec is not. Marking one slow because of
Gatekeeper would hide a machine configuration problem inside the test suite, where the next person cannot see it — and
`AGENTS.md` asks an `#[ignore]` to argue for itself on what the test protects, which this could never do.
