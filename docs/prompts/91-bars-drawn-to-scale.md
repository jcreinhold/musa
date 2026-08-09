---
id: 91
slug: bars-drawn-to-scale
status: pending
depends_on: [90, 84]
phase: 2
---

# Bars Drawn To Scale

## Task

Let a project ask for its bars to be drawn to scale: each event at a column proportional to when it sounds, so beat
*n* lands in the same column on every line and a reader can look down the page instead of across it. One setting, in
`musa.toml`, with a computed default.

## Read

- Philosophy of Software Design ch. 8: *"you should avoid configuration parameters as much as possible. Before
  exporting a configuration parameter, ask yourself: 'will users be able to determine a better value than we can
  determine here?'"* This prompt exports exactly one, and the Design section is the answer to that question.
- `crates/musa-project/src/project.rs` — `read`, and the double `.ok()?` that makes a malformed manifest cost the
  project its name rather than error.
- `crates/musa-project/src/session.rs` — `ProjectCommand::Format`, `formatted_source`, and the composer fallback,
  which is the precedent for a manifest fact reaching a lower crate.
- `crates/musa-project/src/realization.rs` — a `tracing::warn!` and a default, which is the precedent for a manifest
  value nobody recognises.

## Design

### Gate this before building it

Implement the algorithm as a throwaway and render all of `examples/` both ways side by side **before** touching the
API, the manifest or the tests. The risk is real and it is not the algorithm: every bar with a `use`, a tuplet, a
slur, a grace group, a repeat, a hairpin or a parameter duration falls back to compact, and a page that is 80%
aligned may read *worse* than one uniformly compact, because the eye starts trusting columns that are not there.
**If the corpus looks ragged, stop and report. Do not ship the setting.**

### The curve is linear, and that is a proof rather than a preference

Engravers space logarithmically, so the obvious design is a concave width function. It cannot work here. If the width
allotted to an event is a function `W` of its own duration and columns accumulate, then "equal elapsed time, equal
column" applied to `1/4 + 1/4` against `1/2` forces `W(1/2) = 2·W(1/4)` — and applied at every dyadic split, forces
`W` linear. Any concave curve provably puts beat 3 in a different column on a line of quarters than on a line of
halves, which is the one thing this feature exists to prevent.

Real engraving recovers alignment by solving one spacing problem across a whole *system*. Here that would make bar 5's
edit relayout bars 1–8, and diff locality is the thing a text formatter must not spend.

```
grid(t) = floor(t × 64)                                  // 64 columns per whole note
col(0)  = 0
col(k)  = max( grid(t_k), col(k-1) + len(text of item k-1) + 1 )
```

Sixteen columns per quarter. 64 divides by 2, 4, 8, 16 and 32 exactly and by 3 closely enough that a triplet returns
to the grid at its end. Columns are measured from the `|`, indent excluded.

**No `f64` anywhere.** `log2` is not bit-identical across libm implementations, and a formatter whose output depended
on the platform's libm would make `musa format --check` fail in CI on a machine other than the one that wrote the
file. Time accumulates as an exact `(u64, u64)` reduced by `gcd`; `musa-language` does not grow `num-rational` for
this.

Three rungs, in order: proportional if the bar is measurable and it fits `MEASURE`; else compact if it fits; else
wrap. Because `col(k) ≥ col(k-1) + len + 1`, the proportional line is **never narrower** than the compact one — which
gives the property that makes the setting safe to turn on: *the set of bars that wrap is identical in both modes.*
Turning it on never breaks a bar across lines; the worst it does to any bar is leave it looking as it did.

### The API is a required parameter

```rust
pub fn format(document: &ParsedDocument, bars: BarSpacing) -> FormattedSource

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSpacing { #[default] Compact, Proportional }
```

A `format_with` overload is rejected: it would leave `format(&doc)` in the API as a trap that silently means
"compact", so the next call site added — a build script, a preview, a doc test — would quietly ignore the project's
setting. Making the parameter mandatory is the type system enforcing what the crate graph cannot: every caller answers
which layout this is. A one-field options struct and a one-implementor trait both add interface without functionality;
a bare `bool` is unreadable at the call site.

`Default = Compact` is **computed here, not asked for**, and the doc comment says why: every existing file is compact,
so `Proportional` would rewrite the corpus on upgrade; and compact is the answer that is never actively wrong.
Consequently `Option<BarSpacing>` appears nowhere — there is no unset state to branch on.

Could the formatter infer the mode instead? Only from the file's current shape — "if bars already contain double
spaces, keep them" — which is inferring the setting from the artifact the setting produced. Delete a bar and the file
falls off the grid; add a long chord and it climbs back on. This is the case ch. 8 allows, where the higher layer
holds information the lower one cannot recover.

**Nothing else joins it.** `MEASURE` and the four-space indent stay compile-time constants, because `formatter.rs`
and `01-visual-language.md` are already nineteen lines and a paragraph of finished reasoning about them. Musa
determines those better than a user can, and exposing them would be exactly the ch. 8 failure — punting a solved
problem upward.

### The manifest, and a typo that must not cost the project

```toml
[format]
bars = "proportional"
```

Typed as `Option<String>` **on purpose**. Today the second `.ok()?` in `read` means a malformed `musa.toml` costs the
project its name, its composer *and* its running order — a policy that is stated and defended for TOML that is not
TOML. Extending it unchanged would mean a typo in the newest and least important key destroys the oldest and most
important ones, and then `musa format` rewrites every file back to compact. A string always parses, so `[project]`
survives and the interpretation happens in code:

> **A typo in `[format]` costs you the setting. Only a manifest that is not TOML costs you the project.**

An unrecognised value warns through `tracing::warn!` and falls back, which is what `realization.rs` already does three
lines long for a sidecar it cannot read. No `ManifestComplaint` type, no widened `read` signature.

`ProjectMeta` gains `bar_spacing: BarSpacing`, not an `Option`. `ProjectSession` gains one private `bar_spacing()` so
the `Format` command and the `formatted_source` preview cannot disagree.

The composer fallback looks like a precedent and is not, and the commit message should say so: that sets a scalar on a
*finished* artifact — the score exists and the field is empty. Spacing is a decision *inside* the layout algorithm,
and the only post-hoc version of it is a second formatter living in the crate whose doc says formatting belongs to
`musa-language`.

### The editor and the command line must agree

This setting is only honest if the language server reads the same manifest. That fix — the LSP naming its compiler
document with a `file:///` URI, so `ProjectSession` never finds a `musa.toml` and import resolution does path
arithmetic on a URI — is being made separately. **This prompt depends on it and must not duplicate it.**

## Target

- `crates/musa-language/src/formatter.rs`: `BarSpacing`, the grid, the three rungs.
- `crates/musa-language/src/lib.rs` and `crates/musa-project/src/lib.rs`: `BarSpacing` exported and re-exported.
- `crates/musa-project/src/project.rs`: the `[format]` section, `ProjectMeta::bar_spacing`, the warn-and-default.
- `crates/musa-project/src/session.rs`: `bar_spacing()`, and the two call sites.
- `examples/album/musa.toml`: the section, so the fixture exercises it.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- format --check examples/album/pieces/01-first.musa
```

New laws:

- **the two spacings write the same program** — `significant_tokens` of both modes are equal, over generated sources.
  This is the formatter-level statement of the manifest invariant, and it subsumes `format_preserves_semantics`.
- both existing proptests run under **both** modes, which is the whole idempotence risk for the price of one line;
- an event at bar-relative time *t* begins at column `grid(t)` or later, never earlier;
- the set of bars that wrap is identical in both modes, and a proportional line is never narrower;
- an unmeasurable bar is **byte-identical** under both modes;
- `bars = "nonsense"` yields `Compact` **and** the project is still named — the error design, encoded so it cannot
  silently regress.

## Stop

- **No second `[format]` key**, now or later. If someone asks for line width or indent, the answer is no and the
  reason is already written down.
- **No log curve, no adaptive scale, no per-file constant.** Each makes one bar's layout depend on another's.
- **No tuplet measurement.** Bars containing tuplets fall back to compact; revisit only if a real piece complains.
- **No `f64`, anywhere on this path.**
- **No changes to `musa-lsp`.**
- **No proportional spacing in the desktop editor's own view.** The setting is about the text on disk; how the score
  is engraved is `musa-render`'s and is already proportional by construction.
