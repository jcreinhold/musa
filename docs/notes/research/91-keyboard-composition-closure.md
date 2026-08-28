# Keyboard composition closure

**Status:** conformance, performance, and accessibility record for prompt 209. This page governs nothing. The workflow
is governed by [`docs/rules/desktop/10-keyboard-composition.md`](../../rules/desktop/10-keyboard-composition.md) and the
numbers it reports are the ones [`docs/rules/desktop/06-frame-budgets.md`](../../rules/desktop/06-frame-budgets.md) §7
asked for.

## Decision

Step entry is gone, and the workflow that replaced it is closed rather than merely built. A keyboard auditions whenever
it is safely connected; Capture and Keep that are the only armed states; a take becomes notation only through Review;
and Accept settles the notation while Keep writes it, once, into canonical source. Every claim below names the
executable evidence that holds it, and every cell the host could not answer says so instead of estimating.

Two things this record does not claim. The transcription figures are the repository corpus's, not a claim about
repertoire in general — that limit is note 90's and it has not moved. And the interface's timings and the core's are
reported beside each other, never added: they were taken in different processes on different workloads, and their sum is
a number nobody measured.

## Host, devices, and toolchain

|  |  |
| --- | --- |
| Host | Apple M4 Pro, 24 GiB RAM, macOS 26.6.2 |
| Rust | 1.98.0 (`88d9e12ae`, 2026-08-18); benches in the `bench` profile, laws in `dev` |
| Node / Playwright | Node 24.16.0, Playwright 1.62.1, Chromium headless, `deviceScaleFactor: 2` |
| MIDI devices | **None connected.** No physical keyboard, no loopback, no virtual port |
| Working commit | `204901c3`, 2026-08-28 |
| Corpus | ten `.musa` fixtures and their canonical traces in `crates/musa-project/tests/fixtures/transcription/`; five committed readings in `apps/musa-desktop/ui/fixtures/reviews.json` |

## Conformance matrix

Every row names a law or a screen test. A row is green because that test passes in the suite this commit ships, not
because the behaviour was observed once.

| Claim | Evidence | State |
| --- | --- | --- |
| The source is unchanged by listening, capturing, and Keep that | `midi.rs` capture unit laws; `capture.spec.ts` "a phrase is played, read, accepted, and only then written" asserts the revision is unmoved | green |
| The source is unchanged by reading and by accepting | `review_placement_laws::accepting_a_reading_still_does_not_touch_the_source`, `::planning_writes_nothing_and_asking_twice_is_free` | green |
| Timestamp and pedal evidence survive complete | `midi.rs::key_release_and_pedal_transition_stay_separate`, `::repeated_notes_pair_fifo_and_missing_releases_remain_facts`, `::a_take_records_its_start_fit_and_connection_discontinuity` | green |
| Known clock | `rhythm_transcription_laws::a_known_clock_keeps_its_origin_and_reports_the_exact_onset` | green |
| Free clock, and taps that answer it | `review_laws::a_pulse_needs_more_than_one_tap_and_settles_the_mark_when_it_has_them`; `review.spec.ts` "a free take asks for the pulse, and taps answer it" | green |
| Unmeasured scope is refused, not gridded | `rhythm_transcription_laws::unmeasured_scope_is_a_write_source_refusal`; `review_laws::an_ametric_take_is_refused_rather_than_written_onto_a_grid` | green |
| Straight, syncopated, tuplet, compound, asymmetric, and pickup rhythm | `notation_proposal_laws::the_corpus_syncopation_fixture_previews_with_dots_and_a_barline_tie`, `::the_corpus_triplet_fixture_previews_as_bracketed_tuplets`, and the corpus admission law over all ten fixtures | green |
| Chords and rolled attacks | `notation_proposal_laws::a_block_chord_previews_as_separate_voices`; `review_laws::a_cluster_reads_as_one_chord_or_as_the_notes_that_were_played` | green |
| One, several, and crossing voices, bounded at four | `review_laws::assigning_a_line_is_refused_past_the_four_a_keyboard_proposal_writes`; `review.spec.ts` "crossing lines are assigned by line" | green |
| Pitch spelling follows the key, and every spelling sounds where it came from | `midi.rs::a_note_is_spelled_the_way_its_key_spells_it`, `::every_spelling_sounds_at_the_note_it_came_from`; `notation_proposal_laws::chromatic_spelling_is_explicit_and_round_trips` | green |
| Group duration and pitch edits, previewed then applied once | `group_edit_laws` (23 laws, including `::one_transformation_is_one_undo` and `::a_plan_is_consumed_once_and_is_stale_afterwards`) | green |
| A group edit against generated music edits the motif, and says so | `group_edit_laws::transforming_generated_music_transforms_the_motif_and_says_so`, `::a_specialization_leaves_the_motif_alone`; `editing.spec.ts` "editing a generated note asks first" | green |
| Candidate and constraint locality | `rhythm_transcription_laws::a_matched_pin_is_local_and_idempotent`; `review_laws::choosing_a_reading_settles_that_mark_and_leaves_the_others_standing` | green |
| Acceptance formatting, provenance, and identical bytes for identical readings | `review_placement_laws::equal_readings_at_equal_anchors_write_the_same_bytes`, `::naming_the_lines_adds_them_and_replaces_nothing`; `review_laws::a_decision_never_changes_where_a_note_came_from` | green |
| Autosave and crash recovery are untouched by the removal | `editing.spec.ts` "work a crash left behind is offered", "declining the offer keeps the file's own text", "unsaved work says whether it is kept" | green |
| One kept phrase is exactly one undo | `review_placement_laws::one_undo_puts_the_piece_back_exactly`; `capture.spec.ts` "one undo puts the piece back where it was" | green |
| Device loss during a take | `midi.rs::disconnect_releases_latched_sustain_before_held_voices`; `capture.spec.ts` "a keyboard that goes away says the take survived it" | green |
| A piece that moved on: stale review, and a refusal that keeps the take | `review_laws::an_edit_to_the_piece_makes_the_review_stale_rather_than_gone`; `review_placement_laws::a_part_the_piece_no_longer_has_is_a_refusal_that_keeps_the_take`, `::edits_that_leave_the_anchor_alone_leave_the_phrase_placeable` | green |
| A phrase that would not compile changes nothing and can still be kept after | `capture.spec.ts` "a phrase that would not compile changes nothing, and can be kept after" | green |
| Audition parity: played or written, one at a time, deciding nothing | `review_laws::auditioning_the_other_performance_is_not_a_decision`; `review.spec.ts` "audition is A or B, never both, and decides nothing" | green |
| Privacy and bounds: the recent buffer is memory-only, bounded, and can be turned off | `midi.rs::recent_memory_is_bounded_by_event_count_and_can_be_disabled`, `::the_published_recent_ring_stays_below_one_mebibyte`, `::explicit_capture_refuses_growth_past_its_event_bound` | green |
| Keep that takes a phrase with an observed start, and refuses a suffix without one | `midi.rs::keep_that_freezes_the_latest_complete_phrase_start`, `::keep_that_refuses_a_suffix_with_no_observed_start_boundary` | green |
| Every declared placement refusal | `review_placement_laws::a_reading_that_cannot_be_written_exactly_has_no_phrase_to_keep`, `::a_reading_still_taking_decisions_cannot_be_kept`, `::a_phrase_in_two_lines_will_not_be_kept_until_they_are_named`, `::two_lines_pointed_at_one_voice_are_refused`, `::a_name_the_language_cannot_write_is_refused_before_the_parser_sees_it` | green |
| Every declared review refusal | `review_laws::a_refused_gesture_leaves_the_reading_exactly_as_it_was`, `::an_accepted_reading_takes_no_further_decisions`, `::a_reading_with_a_length_it_cannot_write_cannot_be_kept_yet` | green |
| The interface computes no notation | `review.spec.ts` and `tests/unit/review.test.ts` (26 arrangement tests) assert placement from the project's own ticks only | green |
| Nothing writes a note that was not played or typed | `pointer.spec.ts` "a click on empty staff clears the selection and writes nothing"; `capture.spec.ts` "the keyboard says it never writes notes, and it never does" | green |

## Measured workflows

Twenty trials each, p50/p95/max, printed by a passing run of `playwright test --project=budgets`. These are the
interface's share: the harness's shell answers with a committed reading rather than transcribing one.

| Workflow | p50 | p95 | max | Budget |
| --- | ---: | ---: | ---: | --- |
| Capture → a readable proposal | 42 ms | 45 ms | 93 ms | B13, ≤ 250 ms |
| Keep that → a readable proposal | 43 ms | 44 ms | 108 ms | B13, ≤ 250 ms |
| A group command → its preview | 6 ms | 7 ms | 8 ms | B14, ≤ 250 ms |
| A review decision → the replacement reading | — | 10 ms | — | B14, ≤ 250 ms |
| Asking to hear it the other way → the audition in force | — | 9 ms | — | ≤ 100 ms |
| Focus back on the note being read | — | 0 ms | — | ≤ 50 ms |
| Keep → the phrase on the leaf | 176 ms | 180 ms | 190 ms | B15, ≤ 400 ms |
| Undo → the previous document in hand | 0 ms | 0 ms | 0 ms | B15, ≤ 400 ms |

The undo row is a real zero, not a missing measurement: the stub answers in a microtask, so the round trip lands inside
the clock's own resolution. What an undo costs in *ink* is B2's number on a workload where the page changes.

The core's share, `bench` profile, 100 samples:

|  | Median | Slowest |
| --- | ---: | ---: |
| 128-note take through the production facade | **2.658 ms** | 10.35 ms |
| The whole generated corpus through the trial path | 8.645 ms | 9.293 ms |
| 128 regular notes through the trial path | 43.17 ms | 44.75 ms |

The facade is sixteen times faster than the trial path it replaced on the same take, which is what prompt 204a's shared
back-pointer representation bought over the trial's cloned candidate vectors. Its bounds are asserted, not assumed:
`rhythm_transcription_laws::a_128_note_take_completes_within_the_published_bounds` reports 96 peak states and 2,090
bytes of retained search against the published ceilings of 96 states and 128 KiB.

Real-time paths, 2,000 samples each (`musa-playback::suite rt::latency_probe_reports_bounded_software_paths`):

| Path | p50 | p95 | max |
| --- | ---: | ---: | ---: |
| MIDI callback decode and hand-off to the ring, per event | 3 ns | 4 ns | 22 ns |
| One audition event plus a 128-frame render block | 6,417 ns | 6,708 ns | 17,542 ns |

A 128-frame block is 2.67 ms of audio at 48 kHz, so the render costs about a quarter of one percent of real time.
Callback violations: zero — `rt::the_callback_path_allocates_nothing` and
`rt::the_midi_input_callback_allocates_nothing` count allocations on the callback thread and admit none, and
`rt::the_callback_module_contains_no_logging` holds the module to it by source.

## Corrections and recall

Measured through the production facade over the ten-fixture corpus, printed by
`rhythm_transcription_laws::corpus_admission_thresholds_hold_through_the_facade`:

| Quantity | Through the facade | Prompt 203's trial | Admission floor |
| --- | ---: | ---: | ---: |
| Exact onsets | 54/58 | 54/58 | ≥ 54 |
| Intended reading in the top five | 8/9 | 8/9 | ≥ 8 |
| Exact key durations | 47/58 | 47/58 | ≥ 47 |
| Source-token edits to reach the intended score | 23 | 23 | ≤ 23 |

The production path reproduces the trial's numbers exactly, which is the claim the facade was built to make: nine
fixtures, 58 onsets, and 23 token edits — a mean of 2.6 corrections per fixture. Note 90's review-operation count of 16
for the same corpus is the interface-side figure and has not moved either, because the same candidates reach Review.

A transcription that met an onset score but demanded heavy correction would be red. It does not: two thirds of the
fixtures need no correction at all, and the four that do are the syncopation, tuplet, and rolled-chord readings where
Review asks a question rather than guessing.

## Accessibility

| Check | Evidence |
| --- | --- |
| No axe violations on the Review screen | `review.spec.ts`, `AxeBuilder` over the opened review with a live ambiguity |
| No axe violations elsewhere in the workspace, source column, palette, and sheet | `accessibility.spec.ts` |
| Every review gesture reachable from the keyboard alone | `review.spec.ts` "every review gesture is reachable from the keyboard alone" |
| Focus survives a proposal being replaced | `review.spec.ts` "focus survives the proposal being replaced", and R3 measures the restoration at p95 0 ms |
| Every proposal note carries a spoken musical name | `review_laws::every_note_carries_a_musical_name_a_reader_could_speak`; `review.spec.ts` "every proposal note has a musical name" |
| Nothing is clipped at 200 % text | `accessibility.spec.ts` "nothing is clipped at 200 % zoom"; `review.spec.ts` "review at 200 % text" |
| Reduced motion is honoured | `review.spec.ts` "review with reduced motion" |
| The armed state is not colour alone | Capture is a pressed button with a title that says which clock the take will get; `capture.spec.ts` "the capture line says which clock the take will get" |

## What was not available

**No MIDI hardware.** No keyboard, loopback, or virtual port was connected to this host. Key scan, USB or Bluetooth
transport, CoreMIDI delivery, device buffering, and DAC therefore have no p50, p95, or maximum here, and the "plug in to
first sound" workflow has no end-to-end number. The host-side figures above are host-side figures; calling their sum
"input-to-sound latency" would be false. Note 89 states the experiment a hardware report would have to run, and that
requirement is unchanged.

**No ASAP checkout.** Prompt 209 asks for the representative human takes to be re-run. `MUSA_ASAP_ROOT` is unset and the
dataset is not on this machine; the adapter refuses any checkout that is not ASAP v1.1 commit
`fad8d1e8078d0ae47ad2f280b5d022bd2de24784`, and fetching one is a network action outside this repository. The
exact-pinned figures in note 90 — 5,530 of 5,714 performed notes matched, 3,431 on the exact 24-tick grid — therefore
stand as the most recent measurement on real performances, and this closure adds no new evidence about them. What the
production facade demonstrably preserves is the generated-corpus behaviour, byte for byte, at a sixteenth of the cost.

Neither gap is closed by the numbers that are here, and neither is presented as if it were.

## What was removed

`musa-project`'s `MidiEntry`, the 40 ms `EntryBuffer` and its chord window, `ProjectSession::midi_entry`, and the
key-at-caret spelling path. The desktop's `NoteEntry` class, the `state/compose.ts` keystroke map, the `N` mode and its
palette command, the duration glyph in the top margin, the click that wrote a note onto blank staff, and the
`musa://midi` event with the `listen_to_midi` compatibility flag behind it — the latter replaced by `audition_at`, which
says where the composer is and nothing about arming. Their tests went with them; the seven tests in `entry.spec.ts` that
were never about entry are now `editing.spec.ts`.

What stayed is what other callers use: the semantic `insertNote`/`changePitch`/`changeDuration` edits the source column,
the inspector, the pointer, and placement all issue; the duration keys, which are prompt 206's group commands with a
selection and nothing without one; and the pitch spelling, which transcription needs.
