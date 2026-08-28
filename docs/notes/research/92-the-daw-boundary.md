# The DAW boundary

**Status: governs nothing.** The argument behind
[`docs/rules/across-stages/06-daw-boundary.md`](../../rules/across-stages/06-daw-boundary.md), written when prompt 210
added that page. The decision is there; this is why it reads the way it does, and what it chose against.

## 1. The reason

Musa's product boundary already said what it would not become: no audio recording or waveform editing, no third-party
plug-in hosting, no complete engraving (roadmap §4). What it did not say was how a finished piece reaches the program a
composer actually delivers from.

Phase 4 of the implementation sequence listed "optional CLAP hosting; optional Audio Unit bridge on macOS" in the same
breath as sample playback and MusicXML import. Read as a work item, "Audio Unit bridge" has two opposite meanings — Musa
hosting somebody's plug-in, or a host loading Musa — and the first of them contradicts §4's own non-goal three sections
earlier. A nine-prompt execution cone (211–219) was about to be built on a phrase that pointed both ways.

The engineering reason is narrower than the wording problem. Three crossings were being treated as one feature:

| Crossing | Owns time | Fails as |
| --- | --- | --- |
| A directory a person imports | Musa, at the moment of export | wrong alignment, unstated losses |
| A live MIDI stream | one declared authority, per session | two transports fighting, silent drift |
| A component a host renders | the host | a render block that allocates, or a parameter that is not a source control |

A single "DAW integration" abstraction would have let a fact proved on one crossing be cited on another, which is
exactly the class of mistake `04-identity-and-realization.md` exists to prevent for caches. They needed separate names
before they had separate code.

## 2. What fails under the rule as it stood

**Judged.** Each of these is a thing a composer can want today, against the repository as it is at prompt 210.

1. *Finish a piece in Logic.* Musa exports MEI, LilyPond, MusicXML, a Standard MIDI File, and one stereo WAV
   (`ExportRequest` in `musa-project`). Nothing states which project revision, source, and lock closure produced them;
   nothing states frame zero or a common length across artifacts; nothing lists what MIDI dropped. A person who imports
   them and later asks "which version of the piece is this?" has no answer inside the bundle, and obligations §9's
   origin path stops at the file boundary.
2. *Play Musa's instrument in a host.* The prepared machine and its `prepare_execution` arguments are exactly what a
   Music Device would need, and §7's exact-to-physical boundary already fixes the approximation point. But nothing said
   whether the host's block size may mean anything, whether the host's transport is authoritative, or which source
   values may become plug-in parameters. Every one of those has an obvious wrong answer that would have looked
   reasonable in a pull request.
3. *Drive Logic live.* `musa-playback` owns a transport and a MIDI input callback. Two transports with no declared
   authority is the default outcome of connecting them, and MIDI clock carries no meter, key, spelling, or exact
   rational position — so "sync" without a stated loss list would have promised something the protocol cannot do.
4. *Keep a mix.* The routing graph is checked source, but a workstation wants stems. Without a rule, "stems that sum to
   the master" is the natural thing to claim, and it is false the moment a send duplicates signal or the master chain is
   nonlinear. Prompt 211 was already written to forbid that claim; it had nothing governing to cite.

None of these is a defect in the current code. They are all questions the current documents did not answer.

## 3. The replacement, in plain language

The `.musa` project and its locked closure remain the only master record. A bundle, a MIDI stream, an Audio Unit
parameter tree, a host automation lane, and whatever document a person builds in their workstation are presentations
made by named conversions, and none of them may write back into source or stand in for it.

There are three crossings, not one. Each states its own derivation record — project, source, and lock identity, seed,
profile, format, policy, part and control identities, artifact digests, compatibility target, origins, and losses — and
each states who owns time: Musa for an offline file, one declared authority for a live session, the host for a
component. Every loss is refused or written down by name; nothing is flattened quietly.

Musa is hosted and does not host. Logic and GarageBand may load a Musa Music Device that renders one checked instrument;
Logic may additionally load a MIDI Processor that projects a checked piece onto its timeline, if a host probe measures
that surface. Musa loads no third-party Audio Unit, CLAP, or VST, writes no `.logicx` or `.band`, and reads neither.

## 4. What changed, document by document

- **New.** `docs/rules/across-stages/06-daw-boundary.md`, governing, with the compatibility and loss table in its §8.
- **Amended.** `docs/rules/language/08-performance-and-sound.md` §10 (rendering inside a host, and the parameter-tree
  projection); `docs/rules/language/09-assets-and-packages.md` §9 (the same locked closure inside a host process);
  `docs/rules/desktop/09-sound-and-mix.md` §5 (export and live-projection status language only — export itself is a File
  action and no workspace gained a surface).
- **Indexes.** `docs/rules/across-stages/README.md` reading order; this directory's index; the amendment record in
  `docs/rules/README.md`.
- **Plan.** Roadmap §4's plug-in non-goal now says explicitly which direction it forbids; §12.6 is new and states the
  three crossings; §13.2 says a host render block inherits the callback rules; Phase 4's ambiguous "Audio Unit bridge"
  line is replaced by the producer boundary. The code map gains a DAW-boundary section, every row **absent**, naming
  prompts 211–219.
- **Book.** [Export scores and audio](../../book/src/how-to/export.md) states what a workstation gets and what it does
  not, without claiming a transport that does not exist yet.

**Unchanged, deliberately:** the constitution, the obligations, and Musa's musical ontology. This decision is about
presentations and host boundaries. It adds no source keyword, no data family, no control catalogue, and no host-side
semantic default, and it needed no music-theory amendment.

## 5. Migration

Nothing stored migrates, because nothing exists yet to migrate. No `.musa` file, no lock file, no autosave, no exported
artifact, and no public API changes at prompt 210 — the boundary is defined before the first transport is built, which
is the whole point of writing it now.

The obligations this creates are forward ones. Prompts 211–219 must produce a derivation record from the start rather
than adding one to a shipped format later; the first bundle version is version 1 of a record with a version field. The
existing `ExportRequest` targets keep their meaning and become constituents of a bundle rather than being replaced.

## 6. What was rejected, and why

**Judged**, all six.

**Hosting third-party plug-ins.** Roadmap §4 rejected it on discovery, compatibility, state serialization, GUI
embedding, crash isolation, and threading. Nothing has changed except that Apple now makes the opposite direction cheap:
producing an AUv3 is a bounded piece of work with one documented render contract, and hosting is still six unbounded
ones. The rejection stands and this page states it as a direction rather than a feature.

**Writing `.logicx` or `.band`.** A proprietary session document is undocumented, versioned by someone else, and would
make Musa responsible for a project it cannot check. The open files a host documents importing are the whole of the
supported path.

**A round trip.** Reading a workstation's document back into `.musa` was the most requested-sounding option and is the
one the constitution most clearly forbids: §6 allows a reversible conversion only where reversibility is *proved* for
that conversion, and no MIDI or workstation-document conversion can be, because spelling, voices, ties, and meter are
gone. Offering an approximate import would create exactly the second editable model §1 exists to prevent.

**One "DAW integration" abstraction.** Rejected on the argument in §1: three crossings with three owners of time and
three failure modes, unified only by the name of the consumer. Ousterhout ch. 8 is the relevant reading — the shared
abstraction would have moved complexity out of the three implementations and into every reader trying to work out which
guarantees applied.

**Two authoritative clocks.** "Follow whichever is running" is what most integrations do and it is why they drift. One
declared authority per session, and losing the clock is a state with a sentence rather than a silent promotion.

**Exposing DSP nodes as plug-in parameters.** It would have been the easy way to make a component feel powerful, and it
would have handed the host a mutable surface with no source identity — a second place a piece's sound is decided, which
is `08-performance-and-sound.md` §0's ownership test failing in public. Public source controls, and nothing else.

## 7. What this note cannot tell you

The compatibility table in the governing page is currently *documented* and *declined* only. Its "measured" column is
empty on every row: prompt 210 builds no transport and this note ran no host probe. The Logic Pro guide pages cited
there did not render their article bodies when the table was written, so the Logic rows are cited at title strength; the
GarageBand import and Audio Units pages and the `AUAudioUnit` reference did render and are quoted at what they say.

Prompt 215 exists to replace that column with measurements against named Logic, GarageBand, macOS, and Xcode versions,
and prompt 219 exists to audit the whole boundary against real hosts. Until they run, no implementation may cite the
table as evidence that a host behaves in a particular way — only as evidence of what Musa has decided.
