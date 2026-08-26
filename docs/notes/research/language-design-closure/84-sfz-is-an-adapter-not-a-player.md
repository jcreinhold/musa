# SFZ is an adapter, not an installed player

Prompt 185 originally directed implementation of “the exact support matrix approved” by the asset specification, but
that specification contained no matrix. Implementing from that sentence would make compatibility whatever the parser
happened to accept. This note records the evidence used to add §3.1 and governs nothing by itself.

## Sources read on 2026-08-25

- The SFZ Format source tree at commit `2f933f593895f92dcc2fc78a1ca4110158250272` (2026-06-08), including its header,
  opcode, sample, trigger, envelope, sequence, and cymbal-muting pages. Its catalogue explicitly mixes SFZ v1, SFZ v2,
  ARIA, LinuxSampler, and other extensions, and labels each entry. The rendered source is
  <https://sfzformat.com/opcodes/>.
- liquidsfz `0.4.1` and repository commit `da84faf56ece82cb0f6177968f6dbd9934f93d99` (2026-04-24). Its `OPCODES.md`
  accepts the matrix's core but deliberately has a permissive parser, player-specific defaults, includes/macros, many CC
  modulations, filters, and extensions that `sfz@1` must not silently inherit.
- OpenMPT `1.32.11.00` (2026-08-15) and its maintained SFZ implementation manual. It supports the core plus several
  extensions, imports includes/macros, accepts more codecs, reads sample loop metadata, and acknowledges that its
  sampler implements only a fraction of a full SFZ player. That is compatibility evidence, not a second specification.
- sfizz's published opcode-status table and `1.0.0` release record were also inspected because its conformance history
  is unusually detailed. The repository is now archived, so it is corroborating evidence rather than one of the two
  maintained-player observations.

## Disagreements that affect Musa

`<global>` is labelled SFZ v2 although common “v1-compatible” libraries use it. The adapter therefore labels it v2
rather than pretending it was v1. Players disagree on release-region correspondence, envelope defaults and endpoints,
zero-valued choke groups, permissive number parsing, path case-folding, sequence counters, and whether malformed or
unknown opcodes merely warn. They also add distinct macro, include, generator, codec, controller, and scripting
facilities.

Musa resolves those disagreements by versioning one strict translation. Unknown sound-changing syntax is an error, paths
stay inside the already verified closure, all applicable regions layer, dB remains exact until the DSP edge, release and
release-key remain distinct, choke direction is represented rather than collapsed into a symmetric group, and the
SFZ-format envelope equation is fixed in `sfz@1`. None of those choices adds an SFZ concept to the event track or
instrument signature; the adapter's answer is the same checked `SampleMapArtifact` an ordinary Musa package can write.
