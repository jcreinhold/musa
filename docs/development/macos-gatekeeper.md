# The test suite is not slow on macOS; it is blocked

## The symptom

`cargo nextest run --workspace` appears to hang. No output, no failures, no progress bar movement for tens of minutes.
Killing and rerunning it seems to help for a while and then it happens again after the next edit.

## How to confirm it in ten seconds

Look at the test binaries while it is happening:

```sh
ps -Ao pid,stat,etime,%cpu,rss,comm | grep target/debug/deps
```

The tell is a row like this, repeated:

```text
61123 S    05:06   0.0     32  /path/to/musa/target/debug/deps/compiler-27436e4ade67b77d
```

State `S`, **0.0% CPU**, **~32 KB resident**. A test binary that is actually running tests uses CPU and tens of
megabytes. These have been execed but have never reached `main`. Watch for a minute and you will see them start one at a
time, minutes apart, in alphabetical order — which is the other half of the tell, because a parallel test runner does
not schedule work alphabetically. That ordering is a queue.

## The cause

macOS validates every executable on first exec through `syspolicyd` (Gatekeeper). A freshly linked binary has no cached
verdict, so it is assessed, and the assessments are serialized. The binary is blocked in the kernel the entire time.

Measured on this repository, on one integration test binary, by copying it to a new path so it had a new inode and
therefore no cached verdict:

|  | Wall clock | CPU consumed |
| --- | --- | --- |
| First exec, no cached verdict | **4:25.28** | 0.00s user, 0.00s system |
| The same binary, already validated | **0.003s** | negligible |

Four and a half minutes of wall clock for zero CPU. The process is not computing; it is waiting.

Two facts turn that single cost into a stalled afternoon:

- **The suite has around a hundred integration binaries** (`ls crates/*/tests/*.rs | wc -l`), and each pays the toll
  once per link.
- **Everything downstream of an edit is relinked, including for a comment.** Changing a doc comment in `musa-compiler`
  recompiles the crate and relinks every test binary that depends on it. A no-op change to semantics is a full-price
  change to the queue.

A third fact explains why it is sometimes worse: since the blocked processes consume no CPU, machine load does not slow
*them* — it starves `syspolicyd`, which is what is doing the work. Running this suite alongside a saturating build
elsewhere stretched the per-binary wait from roughly twenty seconds to the four and a half minutes above. On the run
that produced these numbers, an unrelated Lean linter was holding nine of twelve cores.

## What to do about it — open

**No fix is confirmed working on this repository yet.** The obvious candidate did not help when tested, and the note
records that rather than repeating advice that sounds right.

The usual recommendation is to register the terminal as a developer tool, which is supposed to exempt files created by
that app from assessment:

```sh
sudo spctl developer-mode enable-terminal
```

followed by enabling it under **System Settings → Privacy & Security → Developer Tools**. Measured immediately after
doing both, on the same probe:

|  | Wall clock |
| --- | --- |
| Before the change | 4:25.28 |
| After the change | 1:05.62 |

Faster, but the run in between was on a far less loaded machine, so this is not evidence of a fix — it is consistent
with `syspolicyd` simply being less starved. The process was still blocked at 0% CPU throughout. Treat the exemption as
unproven here.

Two things are worth knowing before spending more time on it:

- **`spctl --status` is not the check.** It reports the *global* assessment setting, which developer mode does not
  touch. It reads `assessments enabled` before and after, and that says nothing about whether the exemption applies.
- **The exemption is keyed to which app created the file**, via the `com.apple.provenance` extended attribute that every
  binary under `target/` carries. Two consequences follow, and both are untested: binaries built *before* the grant may
  keep their old provenance and never benefit, so a `cargo clean` and rebuild may be required; and the grant must go to
  whichever app actually spawns `cargo`, which for an IDE or agent host is not Terminal.

To watch the blocking happen live:

```sh
log stream --predicate 'process == "syspolicyd"' --info
```

If someone establishes what actually works, replace this section with the measurement.

## Two habits that help regardless

**Separate build cost from run cost** before concluding anything is slow. `cargo nextest run --workspace --no-run`
builds and links without running; the difference between that and the full run is exec and test time, and on an
unconfigured machine almost all of it is exec.

**Do not run the suite and `cargo clippy --workspace --all-targets` at the same time.** They contend for the same build
lock, so both finish later than either would alone, and the interleaved output makes it harder to see which one is
stuck.

## What this is not

It is not a reason to mark tests `#[ignore]`. The tests are fast; the exec is not. Marking a test slow because of
Gatekeeper would hide a machine configuration problem inside the test suite, where the next person cannot see it — and
`AGENTS.md` asks a `#[ignore]` to argue for itself on what the test protects, which this could never do.
