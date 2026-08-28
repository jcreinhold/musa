# The DAW boundary, closed

**Status: governs nothing.**
[`docs/rules/across-stages/06-daw-boundary.md`](../../rules/across-stages/06-daw-boundary.md) states the boundary and
[`92-the-daw-boundary.md`](92-the-daw-boundary.md) says why it reads the way it does. This page is prompt 219's audit of
what was actually built against it: one matrix with a row for every crossing the boundary names, and a column saying,
for each, what is *documented* about a host, what a *person* verified, what the automated harness *measured*, and what
Musa *declined*.

The rule this page is written under: **a cell is only as strong as its column, and an empty cell is not a pass.**
[`scripts/check-daw-integration.sh`](../../../scripts/check-daw-integration.sh) parses §3's table and refuses a run in
which any row's evidence does not resolve — a test that no longer exists, a harness finding that did not pass, or a
bundle claim it could not reproduce. So this page cannot quietly go stale while the code moves.

## 1. What was measured, and on what

|  |  |
| --- | --- |
| Host | arm64 macOS 26.6.2, build 25G83 |
| Toolchain | Xcode 26.6 (17F113), SDK `macosx26.5`, Rust from `rust-toolchain.toml` |
| Logic Pro | **not installed on this machine.** Every Logic row is documentation or an unexecuted protocol step, and says so |
| GarageBand | 10.4.14, installed. Not launched: prompt 219 forbids launching a workstation automatically, so its rows are documentation or unexecuted protocol steps too |
| Commands | `scripts/check-daw-integration.sh`, which runs `scripts/check-audio-unit.sh midi-processor` inside it |

Two things follow from the version rows, and both are deliberate.

A workstation is a commercial application that a machine may not have, so **the automated harness is the contract**. If
the matrix's Logic and GarageBand columns were the gate, this repository's suite would pass or fail on whether someone
bought a licence, which is not a fact about Musa. Every row therefore carries automated evidence *as well*, and §4's
protocol is what a person adds on top when they do have the application.

And a claim about a host that nobody ran is not made. Prompt 210 §8's compatibility table cites Apple's own
documentation for what Logic and GarageBand import and host; this page does not re-derive it, does not extend it to a
version nobody checked, and marks every unexecuted protocol step `not run` rather than assuming it would have passed.

## 2. The corpus

Eleven checked-in pieces, each chosen because it is the smallest thing that exercises one row. They are ordinary
fixtures in [`examples/`](../../../examples/), compiled by the same suite as everything else.

| Fixture | Why it is in the corpus |
| --- | --- |
| `invention.musa` | one part, no studio: the bundle a piece with nothing to route still produces |
| `counterpoint.musa` | several voices in one part, so a MIDI track is not a voice |
| `glass-mountain.musa` | parts, sends, a return, and fixed media: routing that does not sum, and a tail |
| `live-studio.musa` | a studio whose graph is more than a chain |
| `tempo-changes.musa` | a tempo map a host has to be told about rather than shown |
| `changing-meter.musa` | meter changes across the piece |
| `bulgarian.musa` | additive meter — the barring a host cannot hold |
| `canon-x.musa` | polytempo: refused on a host grid, played on the piece's own |
| `mobile.musa` | deterministic choice: the realization seed is part of identity |
| `riser.musa` | sampled media, and the tail past the last written note |
| `twinkle.musa` | the trivial case, so a failure in it is a failure in the machinery |

What is deliberately *not* in the corpus: any `.logicx` or `.band` document. A proprietary host document may not be
committed and may not be golden truth (`06-daw-boundary.md` §9), so nothing here compares against one.

## 3. The conformance matrix

**How to read a row.** *Logic* and *GarageBand* say what is true of the application, at the strength of the word:
`documented` cites Apple's current documentation through `06-daw-boundary.md` §8, `protocol §N` names a step of §4 and
carries `(not run)` until someone records an observation, and `n/a` means the row is not about the host. *Harness* says
what this repository measured. *Reference* says what it was compared against — the offline artifact or the native path
that has no host in it. *Musa* is this project's decision, which no host version changes. *Evidence* resolves: `test:`
names a test the suite must still contain and pass, `finding:` names an id in the Audio Unit harness report, `bundle:`
names a claim `check-daw-integration.py` reproduces from an exported bundle, and `declined` is a decision with nothing
to measure.

| Claim | Logic | GarageBand | Harness | Reference | Musa | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Source and lock identity travel with the bundle | n/a | n/a | measured | manifest | carried | bundle:identity-is-the-source |
| The same source exports the same bytes twice | n/a | n/a | measured | byte compare | deterministic | bundle:deterministic, test:daw_bundle_laws::exporting_the_same_piece_twice_writes_the_same_bytes |
| A changed source is a changed identity | n/a | n/a | measured | manifest | carried | bundle:identity-moves-with-the-source |
| Nothing records when or where it was made | n/a | n/a | measured | byte scan | required | bundle:no-clock-no-path, test:daw_bundle_laws::nothing_in_the_bundle_records_when_or_where_it_was_made |
| Realization seed and profile are in the record | n/a | n/a | measured | manifest | carried | bundle:seed-and-profile, test:daw_bundle_laws::the_manifest_states_the_render_it_was_taken_from |
| Score MIDI imports as written | documented | documented | measured | native MIDI export | produced | bundle:midi-parses, test:daw_bundle_laws::a_logic_bundle_holds_both_midi_documents_the_notation_and_the_audio |
| Performance MIDI is a second, different document | documented | documented | measured | native MIDI export | produced | bundle:two-readings-differ, test:daw_bundle_laws::a_shaped_performance_is_written_twice_and_the_two_readings_differ |
| MusicXML imports as notation | documented | not documented | measured | native MusicXML export | produced for Logic; named as absent for GarageBand | bundle:musicxml-parses, test:daw_bundle_laws::a_garageband_bundle_says_it_has_no_notation_rather_than_quietly_omitting_one |
| The profile changes packaging and never the music | protocol §4.1 (not run) | protocol §4.1 (not run) | measured | byte compare across profiles | required | bundle:profile-changes-only-packaging, test:daw_bundle_laws::the_profile_changes_the_packaging_and_never_the_music |
| The mix and every stem cover the same stretch of time | documented | documented | measured | native WAV export | required | bundle:one-common-length, test:stem_laws::every_file_covers_the_same_stretch_of_time |
| The mix beside the stems is the WAV export byte for byte | n/a | n/a | measured | native WAV export | required | test:stem_laws::the_mix_beside_the_stems_is_the_wav_export_byte_for_byte |
| Fixed media reaches the mix and every stem shares its length | documented | documented | measured | native WAV export | required | test:daw_bundle_laws::fixed_media_reaches_the_mix_and_every_stem_shares_its_length |
| The stems do not sum to the mix, and the bundle says so | n/a | n/a | measured | routing record | recorded | bundle:routes-are-recorded, test:daw_bundle_laws::the_routes_are_recorded_because_the_stems_do_not_sum_to_the_mix |
| Every file the manifest names is there with the digest it claims | n/a | n/a | measured | digest recompute | required | bundle:digests, test:daw_bundle_laws::every_file_the_manifest_names_is_there_with_the_digest_it_claims |
| Every part maps to its MIDI track and its stem | protocol §4.1 (not run) | protocol §4.1 (not run) | measured | manifest | stable | bundle:part-mapping, test:daw_bundle_laws::every_part_is_mapped_to_its_midi_track_and_its_stem |
| Every written note names the source event that made it | n/a | n/a | measured | origins sidecar | carried | test:daw_bundle_laws::every_written_note_can_be_named_back_to_a_source_event |
| Every loss the export reports is written into the bundle | n/a | n/a | measured | sidecar compare | required | bundle:losses-are-written, test:daw_bundle_laws::every_loss_the_bundle_reports_is_also_written_into_it |
| Two clocks and two barrings are reported, not flattened | n/a | n/a | measured | loss record | recorded | bundle:polytempo-is-a-loss, test:daw_bundle_laws::a_piece_with_two_clocks_and_two_barrings_reports_both_losses_and_still_bundles |
| An export that cannot finish leaves nothing behind | n/a | n/a | measured | filesystem | required | test:daw_bundle_laws::an_export_that_cannot_finish_leaves_nothing_behind |
| Live MIDI carries the same performance as the file | protocol §4.2 (not run) | protocol §4.2 (not run) | measured | written MIDI file | required | test:live_midi_laws::the_live_run_and_the_written_file_carry_the_same_performance |
| One published port per part, each saying where it plays | protocol §4.2 (not run) | protocol §4.2 (not run) | measured | CoreMIDI endpoints | produced | test:live_midi_laws::a_run_publishes_one_named_port_for_each_part_and_says_where_each_plays |
| A live run loses exactly what the projection loses | n/a | n/a | measured | projection compare | required | test:live_midi_laws::the_live_run_loses_exactly_what_the_projection_loses_and_says_which |
| How exactly this platform places a message is stated | n/a | n/a | measured | platform report | recorded | test:live_midi_laws::the_report_states_how_exactly_this_platform_places_a_message |
| One declared clock authority, never both | documented | not documented | measured | sync state | required | test:sync_laws::a_session_leads_or_follows_and_never_both, test:sync_laws::the_authority_cannot_change_while_a_run_is_in_progress |
| A leading session publishes one further port for the clock | documented | not documented | measured | CoreMIDI endpoints | produced | test:sync_laws::a_leading_session_publishes_one_further_port_for_the_clock |
| One clock lane refuses a polytempo piece until a scope is named | n/a | n/a | measured | sync state | refused | test:sync_laws::one_clock_lane_refuses_a_polytempo_piece_until_a_scope_is_named |
| The Music Device is a registered, validated component | documented | documented, after Audio Units are enabled | measured | `auval` | produced | finding:discovery.instrument |
| It renders one checked source-declared instrument | protocol §4.3 (not run) | protocol §4.3 (not run) | measured | offline native render | produced | finding:render.sounds, finding:prepare.selectsAPart |
| A host's block partition is unobservable | n/a | n/a | measured | offline native render | required | finding:render.blockPartition, finding:param.blockPartition |
| Every parameter is a source-declared control | n/a | n/a | measured | declaration | required | finding:param.tree.declared, test:hosted_laws::every_exposed_parameter_is_a_source_declared_control |
| A control with no value domain is a named loss | n/a | n/a | measured | declaration | recorded | finding:param.losses, test:hosted_laws::an_incompatible_kind_becomes_a_named_loss_rather_than_a_float |
| Automation still points at its control after a rename | protocol §4.3 (not run) | protocol §4.3 (not run) | measured | address table | required | finding:param.addressesStable, test:hosted_laws::reordering_and_renaming_move_only_what_actually_changed |
| Output buses are the declared points the part reaches | documented | documented | measured | offline native render | produced | finding:bus.identity, test:hosted_laws::the_outputs_are_the_declared_points_this_part_reaches |
| Bus zero is the same whatever a host negotiates | protocol §4.3 (not run) | protocol §4.3 (not run) | measured | offline native render | required | finding:bus.mainUnchanged, test:hosted_laws::bus_zero_says_the_same_thing_whatever_a_host_negotiates |
| The MIDI Processor is a registered, validated component | not read here | not mentioned in the cited page | measured | `auval` | produced for Logic | finding:discovery.processor |
| A seek starts where the host is, without replaying | protocol §4.4 (not run) | n/a | measured | index compare | required | finding:processor.seek.doesNotReplay, test:hosted_schedule_laws::a_seek_starts_where_the_host_is_and_not_at_the_beginning |
| A seek re-enters held notes without scanning the piece | protocol §4.4 (not run) | n/a | measured | index compare | required | finding:processor.seek.noWholePieceScan, finding:processor.seek.reentersHeldNotes |
| Every message belongs to exactly one half-open block | n/a | n/a | measured | brute force | required | finding:processor.block.exactlyOnce, test:hosted_schedule_laws::every_message_belongs_to_exactly_one_half_open_block |
| A stopped transport is silence, and releases what sounded | protocol §4.4 (not run) | n/a | measured | offline compare | required | finding:processor.transport.stopped |
| A host that offers no tempo gets no guess | n/a | n/a | measured | offline compare | refused | finding:processor.context.missing |
| Polytempo is refused on the host grid, not flattened | protocol §4.4 (not run) | n/a | measured | refusal text | refused | finding:processor.polytempo.refused, test:hosted_schedule_laws::the_hosts_timeline_refuses_a_piece_that_has_no_single_grid |
| A saved document restores the same music, or refuses | protocol §4.3 (not run) | protocol §4.3 (not run) | measured | round trip | required | finding:state.roundTrip, finding:processor.state.roundTrip, finding:state.refusesAFutureVersion |
| A state from a later version is refused, never half-read | n/a | n/a | measured | round trip | refused | finding:processor.state.fromTheFuture, finding:processor.state.unknownTimeline |
| The component survives an extension restart | n/a | n/a | measured | out-of-process host | required | finding:instantiate.afterTermination, finding:instantiate.relaunch |
| No callback allocates, locks, does I/O, or logs | n/a | n/a | measured | interposing counter | required | finding:rt.allocation.render, finding:processor.rt.noAllocation |
| No presentation writes `.musa` source | n/a | n/a | n/a | n/a | declined | declined |
| Musa hosts no third-party plug-in | n/a | n/a | n/a | n/a | declined | declined |
| Musa neither writes nor reads `.logicx` or `.band` | n/a | n/a | n/a | n/a | declined | declined |
| A round trip back into source | n/a | n/a | n/a | n/a | declined | declined |
| An Audio Unit effect for other people's audio | documented | documented | n/a | n/a | declined | declined |

## 4. The manual protocol, version 1

What a person does when they have the application, and what to write down. Each step names the matrix rows it settles.
Record an observation by adding a dated line under the step with the application version, or leave it as it stands —
**an unexecuted step stays `not run`, and no row is coloured by assuming what would have happened.**

Run `scripts/check-daw-integration.sh` first, so the bundle referred to below exists and its digests are known.

**§4.1 — Import a bundle.** Export `glass-mountain.musa` at both profiles. In the application, import `score.mid`, then
`performance.mid` into a second project, then (Logic only) `score.musicxml`, then the WAV files. Check: the part count
matches `parts` in `musa-manifest.json`; the tempo and meter match the piece; the two MIDI readings differ audibly in
the way `musa-origins.json` says; the audio files line up at bar one with no offset; and the GarageBand import of a
Logic-profile bundle finds no notation, which is what that profile's manifest already says. *Settles: profile changes
packaging only; every part maps to its MIDI track and its stem.*

**§4.2 — Live MIDI.** Start `musa midi <piece> --mode performance`, and in the application arm one software-instrument
track per published source. Check: each port appears under the name the report gave; notes arrive on the channel the
report names; stopping the run stops the notes rather than leaving one hanging. *Settles: live MIDI carries the same
performance; one port per part.*

**§4.3 — The Music Device.** Register the component with `scripts/check-audio-unit.sh instrument`, then in the
application insert *Musa: Instrument* on a software-instrument track, point it at a project, and play the track's MIDI
into it. Check: it sounds; the parameter list is exactly the source's declared controls; an automation lane written
against one control still points at it after renaming an unrelated one and reopening the project; and saving, closing,
and reopening the project restores the same piece rather than silence. *Settles: it renders one checked instrument;
automation survives a rename; bus zero; state round trip.*

**§4.4 — The MIDI Processor (Logic only).** With `scripts/check-audio-unit.sh midi-processor` run, insert *Musa:
Processor* in a track's MIDI FX slot above any instrument. Check: the piece plays; moving the playhead into the middle
starts there rather than from the beginning; cycling repeats without a stuck note; stopping silences everything; and
`canon-x.musa` on the host timeline refuses with the message the harness records rather than playing something wrong.
*Settles: seek; stopped transport; polytempo refusal.*

**§4.5 — What must not happen, in any step.** The `.musa` file's modification time does not change; no application
writes into the project directory; and nothing appears in the bundle that was not there when the export finished. If one
of these fails, it is a boundary violation and not a compatibility note.

## 5. What the numbers were

Measured by the runs in §1, on the machine in §1. These are records, not budgets: the governing budgets are in
[`../../rules/across-stages/06-daw-boundary.md`](../../rules/across-stages/06-daw-boundary.md) §7 and roadmap §13, and
nothing here changes one.

| What | Number |
| --- | --- |
| Audio Unit findings asserted | 67, `auval` validating both `aumu musa Musa` and `aumi musp Musa` |
| Allocations in the instrument's render block | 0 across 2000 blocks of 512 frames, against a baseline of 0 |
| Allocations in the processor's render block | 0 across 64 blocks, against a baseline of 0 |
| Thread Sanitizer over the component's own threading | clean; the Rust library is not instrumented, and the report says so |
| Seek cost | two binary searches into an immutable array; the active-note scan starts at the bracketing index, not at zero |

The one thing these numbers do *not* cover is the workstation itself: no jitter, drift, or deadline number here was
taken with Logic or GarageBand in the loop, because §1 says why neither was run. The harness is an automated host, and
an automated host is not a person's machine under load.

## 6. What this audit changed

Nothing in the boundary. Every law `06-daw-boundary.md` states was already implemented by prompts 211–218, and the audit
found no cell that had to be turned green by weakening one. What it added is the machinery that keeps the claim honest:
the matrix above, and a gate that refuses to pass while a row's evidence has gone missing.
