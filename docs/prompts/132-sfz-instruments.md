---
id: 132
slug: sfz-instruments
status: pending
depends_on: [129, 131]
phase: 4
---

# SFZ Instruments Enter Through a Checked Adapter

## Task

Import a documented SFZ v1-compatible core into Musa's native sample-map representation. SFZ is an interchange adapter,
not Musa's instrument semantics: every accepted opcode has one explicit translation, every unsupported or dialect
opcode is diagnosed by name, and the runtime remains the deterministic sampler built by prompt 131.

## Read

- `docs/language/09-assets-and-packages.md`; prompts 129 and 131.
- SFZ format overview, headers, regions, samples, and opcode/version table:
  `https://sfzformat.com/`, `https://sfzformat.com/headers/`,
  `https://sfzformat.com/headers/region/`, `https://sfzformat.com/opcodes/sample/`, and
  `https://sfzformat.com/opcodes/`. The catalogue mixes SFZ v1/v2, ARIA, and LinuxSampler extensions; never describe
  that union as one implemented standard.
- At least two maintained open-source SFZ players' conformance/status documentation for compatibility evidence, while
  treating the format documentation above as the semantic source. Record versions and disagreements.

## Design

Implement the exact support matrix approved in `docs/language/09-assets-and-packages.md`. The minimum useful core covers
global/group/region inheritance; sample path; key/range/root pitch; velocity ranges; tune/transpose; gain/pan; sample
offset/end; loop points/modes; amplitude envelope; trigger/release behavior; exclusive groups; sequence position/length;
and the standard sustain-pedal conditions needed by the native map. Map MIDI-shaped SFZ selectors into Musa gesture and
control semantics only at this adapter; MIDI controller numbers do not become the instrument contract.

Resolve samples and any supported includes within the SFZ asset/package root with prompt 129's traversal/digest rules.
If includes/macros cannot be implemented without weakening the resolver, reject them in this version and say so. Parse
off-thread with bounded file/region/opcode/string counts. Duplicate/contradictory regions and unknown values get spans
and useful diagnostics. Unsupported sound-changing opcodes are errors by default; explicitly harmless metadata may be
warnings. Never silently approximate a claimed supported opcode.

The surface selects an SFZ asset through prompt 92's settled instrument syntax and may override only controls the
imported signature exposes. Generate a readable imported signature/technique/support summary for hover and UI.

## Target

- Bounded SFZ parser/adapter and support matrix with spec-version labels per opcode.
- Self-authored redistributable fixtures for layers, round-robin, loops, release, pedal, inheritance, and failures;
  optional external corpus paths may supplement but not replace checked-in tests.
- Differential observations against maintained reference players where lawful, plus native-map equality tests.
- End-to-end local and package-contained SFZ instruments with asset provenance and deterministic audio.

## Check

```sh
cargo nextest run -p musa-audio -p musa-project -p musa-cli -p musa-lsp
cargo clippy --all-targets -p musa-audio -p musa-project -p musa-cli -p musa-lsp -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Import SFZ instruments through the sampler`.

## Stop

- No claim of complete SFZ v2/ARIA/LinuxSampler compatibility and no execution of SFZ scripts.
- No SFZ opcode or MIDI CC leaks into the kernel, standard gesture signature, or general Musa type system.
- No implicit fetching of samples referenced by an SFZ file.
