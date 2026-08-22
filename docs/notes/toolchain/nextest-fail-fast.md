# `--run-ignored all` stops at the first failure

## The symptom

A prompt whose oracle is a recorded failure list — 147, 147a, 166, 165 all say some version of "the same failure list,
byte for byte" — runs its Check line

```sh
cargo nextest run --workspace --run-ignored all
```

and gets a summary that names far fewer tests than the workspace has, with a red line above it:

```text
Canceling due to test failure
------------
     Summary [  71.402s] 629/1890 tests run: 627 passed, 2 failed, 43 skipped
```

The recorded baseline it is meant to be compared against looks like `1890 tests run: 1860 passed, 30 failed`. Nothing
regressed and nothing got faster: 1,261 tests were never started.

The trap is that the truncated report is *plausible*. It says "2 failed", which reads as two regressions to go and
debug, when the class the prompt already knows about is thirty tests wide and twenty-eight of them never ran. Worse, the
two are not a stable prefix — see below — so a second run of the same tree can report a different pair, and the
byte-for-byte comparison the prompt asks for fails against itself.

## Why

nextest's embedded default profile sets `fail-fast = true`:

```sh
strings "$(command -v cargo-nextest)" | grep -n '^fail-fast'
```

```text
# Failure handling for the test run.
# For CI runs, consider setting this to false.
fail-fast = true
```

That is the documented default, not a regression in 0.9.143, and `--run-ignored all` does not disturb it. Until this
note was written the repo had no `.config/nextest.toml`, so nothing overrode it; [the fix](#the-fix) is now checked in,
and this section explains what that file is defending against.

`fail-fast = true` is shorthand for `{ max-fail = 1, terminate = "wait" }`. `max-fail = 1` cancels the run at the first
failure; `terminate = "wait"` lets the tests already running finish rather than killing them. On a twelve-core machine
that means up to a dozen tests are in flight when the cancel lands, and however many of *those* also fail are reported
alongside the first. So the failure count under fail-fast is a function of scheduling, which is exactly the property a
byte-for-byte oracle cannot tolerate.

The whole list only appears with `--no-fail-fast` (alias `--nff`), or with a profile that sets `fail-fast = false`.

## The fix

**`.config/nextest.toml` sets `fail-fast = false` on the default profile, and every `cargo nextest run` in this
workspace now runs to completion.** Nothing needs to be passed on the command line; the Check blocks already written
into the prompt stack are correct as they stand.

```toml
[profile.default]
fail-fast = false
```

Two repairs were available and only one was taken, so if this file ever looks redundant, read the rest of this section
before deleting it.

Three reasons the config file is the right one:

- **Forty commands, one setting.** `--run-ignored all` appears in about forty prompt Check blocks. The property they all
  depend on is a property of how this repo tests, not of any one prompt, and repo-wide test policy is what
  `.config/nextest.toml` is for.
- **It reaches the prompts that may no longer be edited.** Prompt 147 is `status: done` and 166 records the baseline
  every later prompt cites. Their Check lines are the record of how a finished commit was judged; editing them rewrites
  that record, and leaving them alone means the next person to re-run a done prompt's Check hits this again.
- **It matches what the tree is for right now.** The core-calculus cutover deliberately runs with a known red class
  (166's thirty staff tests). A runner configured to stop at the first failure is configured for a green tree, which
  this one is not, and will not be until 166 lands.

The cost is that an interactive `cargo nextest run` also runs to completion instead of stopping early. That is
recoverable per invocation — `--fail-fast` (alias `--ff`), or `--max-fail 1` — whereas the truncated baseline is not
recoverable at all, because you cannot tell from the report that it was truncated except by knowing the workspace's test
count by heart.

**The repair not taken: amending each affected Check command to pass `--no-fail-fast`.** It would edit `status: done`
prompts to say something their commits did not say, it would have to be remembered by hand for every prompt written
after it, and it would leave the same trap armed for anyone who runs the command from `AGENTS.md` rather than from a
prompt.

Do not now add the flags as well. Doing both puts the same decision in two places, and with the config file in place
every `--no-fail-fast` on a command line is a no-op — so a later change to the config would look harmless and would not
be. `--no-fail-fast` remains the right thing to reach for on a checkout that predates this file, or in another repo.

## What was measured

cargo-nextest 0.9.143 (60fa45f, 2026-08-04), nextest-runner 0.122.1, macOS 15 (Darwin 25.6.0), arm64, 12 cores. The
`fail-fast = true` default was read out of that binary's embedded default config with the `strings` command above, on a
checkout that had no `.config/nextest.toml`. The truncated summary above was observed on this workspace while working
prompt 147a, against a baseline of 1,890 tests and 30 known failures recorded by prompt 166 at `7cf258e0`.

nextest validates its config before it builds anything, so a typo here fails in a second rather than after a compile:
feeding it `fail-fast = "nonsense"` reports `invalid type: string "nonsense", expected a boolean or { max-fail = ... }`
and stops. That is also how the checked-in file was confirmed to be read — an accepted config falls through to the
build.
