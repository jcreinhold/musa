//! Taps as reads of routing that already exists (roadmap §13.3, §13.8).
//!
//! The contract under test is that a tap changes nothing. Asking for one must
//! not move a sample of the master, must not reorder a step, and must not
//! invent signal a route did not carry. What a tap adds is a second place to
//! read the frame the master was already mixed from.

#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]
// Exact zero is the claim here: a silent stem is silent, not nearly so.
#![allow(clippy::float_cmp)]

use musa_dsp::{AudioTapRole, render_offline, render_offline_multitrack};

use super::audio_support::prepare;

const TAIL: u64 = 4_096;

fn piece(studio: &str) -> String {
    format!(
        "piece \"stems\" {{ tempo 1/4 = 60; meter 4/4; score {{ \
         part a {{ voice v {{ c4/1 }} }} part b {{ voice v {{ g4/1 }} }} }} \
         studio {{ patch tone {{ oscillator(sine) |> gain(-24 dB) |> output; }} \
         bus hall {{ reverb(room: 0.4, mix: 0.3); }} \
         assign a -> tone; assign b -> tone; {studio} }} }}"
    )
}

const ROUTED: &str = "send a -> hall at -12 dB; route a -> master; route b -> master; route hall -> master;";

fn energy(samples: &[f32]) -> f32 {
    samples.iter().map(|sample| sample * sample).sum()
}

#[test]
fn taps_name_the_parts_and_buses_the_source_declared_in_that_order() {
    let audio = prepare(&piece(ROUTED), TAIL);
    let named = audio
        .taps()
        .iter()
        .map(|tap| (tap.role(), tap.name()))
        .collect::<Vec<_>>();
    assert_eq!(
        named,
        vec![
            (AudioTapRole::Part, "a"),
            (AudioTapRole::Part, "b"),
            (AudioTapRole::Bus, "hall"),
        ]
    );
}

#[test]
fn a_studio_that_declares_nothing_still_taps_its_one_part() {
    let audio = prepare(
        "piece \"solo\" { tempo 1/4 = 60; meter 4/4; score { part a { voice v { c4/1 } } } }",
        TAIL,
    );
    let named = audio
        .taps()
        .iter()
        .map(|tap| (tap.role(), tap.name()))
        .collect::<Vec<_>>();
    assert_eq!(named, vec![(AudioTapRole::Part, "a")]);
}

#[test]
fn every_declared_edge_is_reported_beside_the_taps() {
    let audio = prepare(&piece(ROUTED), TAIL);
    let edges = audio
        .routes()
        .iter()
        .map(|route| (route.kind(), route.source(), route.destination()))
        .collect::<Vec<_>>();
    assert!(
        edges.contains(&(musa_dsp::AudioRouteKind::Send, "a", "hall")),
        "the send a tap sits downstream of must be readable: {edges:?}"
    );
    assert!(
        edges.contains(&(musa_dsp::AudioRouteKind::Route, "hall", "master")),
        "{edges:?}"
    );
}

#[test]
fn asking_for_taps_does_not_move_the_master() {
    let mut untapped = prepare(&piece(ROUTED), TAIL);
    let mut tapped = prepare(&piece(ROUTED), TAIL);
    let plain = render_offline(&mut untapped);
    let multitrack = render_offline_multitrack(&mut tapped);
    assert_eq!(
        plain.samples(),
        multitrack.master().samples(),
        "the mix must be the same bytes whether or not anything was read beside it"
    );
}

#[test]
fn every_stem_runs_the_whole_extent_the_master_does() {
    let mut audio = prepare(&piece(ROUTED), TAIL);
    let rendered = render_offline_multitrack(&mut audio);
    let frames = rendered.frames();
    assert!(frames > 0);
    assert_eq!(rendered.master().samples().len() as u64, frames * 2);
    for stem in rendered.stems() {
        assert_eq!(
            stem.audio().samples().len() as u64,
            frames * 2,
            "`{}` must start and end where the master does",
            stem.tap().name()
        );
        assert_eq!(stem.audio().sample_rate(), rendered.master().sample_rate());
    }
}

#[test]
fn a_part_tap_carries_that_part_and_not_the_one_beside_it() {
    // `b` changes; `a`'s tap must not. This is what makes a tap index mean one
    // source route rather than a share of the mix.
    let first = piece("route a -> master; route b -> master;");
    let second = piece("route a -> master; route b -> master;").replace("g4/1", "rest/1");
    let mut left = prepare(&first, TAIL);
    let mut right = prepare(&second, TAIL);
    let left = render_offline_multitrack(&mut left);
    let right = render_offline_multitrack(&mut right);
    let tap_of = |set: &musa_dsp::RenderedMultitrack, name: &str| {
        set.stems()
            .iter()
            .find(|stem| stem.tap().name() == name)
            .expect("a declared tap")
            .audio()
            .samples()
            .to_vec()
    };
    assert_eq!(
        tap_of(&left, "a"),
        tap_of(&right, "a"),
        "a's own output must be a's alone"
    );
    assert_ne!(
        left.master().samples(),
        right.master().samples(),
        "the fixture must actually differ, or the law above proves nothing"
    );
    assert!(energy(&tap_of(&left, "b")) > 0.0, "b plays in the first piece");
    assert_eq!(
        energy(&tap_of(&right, "b")),
        0.0,
        "b rests in the second, and a rest is silence rather than a held buffer"
    );
}

#[test]
fn a_route_that_reaches_nothing_delivers_nothing_to_read() {
    // `b` is assigned a patch and then routed nowhere. The stem is still
    // written, still aligned, and still silent: it reports what the route
    // carried towards the master, which was nothing.
    let mut audio = prepare(&piece("route a -> master;"), TAIL);
    let rendered = render_offline_multitrack(&mut audio);
    let b = rendered
        .stems()
        .iter()
        .find(|stem| stem.tap().name() == "b")
        .expect("an unrouted part is still a declared part");
    assert_eq!(b.audio().samples().len(), rendered.master().samples().len());
    assert_eq!(energy(b.audio().samples()), 0.0);
}

#[test]
fn a_bus_tap_is_the_return_and_not_what_was_sent_to_it() {
    let mut audio = prepare(&piece(ROUTED), TAIL);
    let rendered = render_offline_multitrack(&mut audio);
    let hall = rendered
        .stems()
        .iter()
        .find(|stem| stem.tap().name() == "hall")
        .expect("the declared bus");
    let part = rendered
        .stems()
        .iter()
        .find(|stem| stem.tap().name() == "a")
        .expect("the sending part");
    assert!(energy(hall.audio().samples()) > 0.0, "the send reaches the room");
    assert_ne!(
        hall.audio().samples(),
        part.audio().samples(),
        "a return has its own processing; if it did not, the tap would be reading the wrong node"
    );
}

#[test]
fn reading_taps_frame_by_frame_agrees_with_reading_them_in_one_traversal() {
    let mut whole = prepare(&piece(ROUTED), TAIL);
    let mut stepped = prepare(&piece(ROUTED), TAIL);
    let rendered = render_offline_multitrack(&mut whole);
    let count = stepped.taps().len();
    let mut frame = vec![[0.0f32; 2]; count];
    let mut by_hand = vec![Vec::new(); count];
    let mut master = Vec::new();
    while stepped.position() < stepped.total_frames() {
        let [left, right] = stepped.step_with_taps(&mut frame);
        master.push(left);
        master.push(right);
        for (samples, [left, right]) in by_hand.iter_mut().zip(frame.iter().copied()) {
            samples.push(left);
            samples.push(right);
        }
    }
    assert_eq!(master, rendered.master().samples());
    for (stem, expected) in rendered.stems().iter().zip(by_hand) {
        assert_eq!(stem.audio().samples(), expected, "`{}` drifted", stem.tap().name());
    }
}

#[test]
fn nothing_is_reported_past_the_finite_extent() {
    let mut audio = prepare(&piece(ROUTED), TAIL);
    let count = audio.taps().len();
    let mut frame = vec![[0.0f32; 2]; count];
    audio.seek(audio.total_frames());
    assert_eq!(audio.step_with_taps(&mut frame), [0.0; 2]);
    assert!(frame.iter().all(|[left, right]| *left == 0.0 && *right == 0.0));
}

#[test]
fn a_shorter_slice_reports_fewer_taps_rather_than_wrong_ones() {
    let mut whole = prepare(&piece(ROUTED), TAIL);
    let mut partial = prepare(&piece(ROUTED), TAIL);
    let mut all = vec![[0.0f32; 2]; whole.taps().len()];
    let mut some = [[0.0f32; 2]; 1];
    for _ in 0..64 {
        whole.step_with_taps(&mut all);
        partial.step_with_taps(&mut some);
    }
    assert_eq!(all.first().copied(), Some(some[0]));
}

#[test]
fn the_stems_do_not_sum_to_the_mix_and_nothing_here_pretends_they_do() {
    // `violin` reaches master directly *and* through the room, so the send
    // duplicates it; the master's own limiter is nonlinear on top of that.
    // A consumer who adds the files up gets a different piece, which is
    // exactly why the routing edges are reported beside them.
    let mut audio = prepare(&piece(ROUTED), TAIL);
    let rendered = render_offline_multitrack(&mut audio);
    let mut summed = vec![0.0f32; rendered.master().samples().len()];
    for stem in rendered.stems() {
        for (total, sample) in summed.iter_mut().zip(stem.audio().samples()) {
            *total += *sample;
        }
    }
    assert_ne!(summed.as_slice(), rendered.master().samples());
}

#[test]
fn a_media_tail_lengthens_every_stem_and_not_only_the_mix() {
    // The part rests for the whole bar; the recording outlives it. Every file
    // still ends together, because the extent is the piece's and not the
    // route's.
    let samples = (0..64).map(|index| (index as f32 / 64.0) - 0.5).collect::<Vec<_>>();
    let mut audio = crate::suite::media_laws::fixed_audio(&samples);
    let rendered = render_offline_multitrack(&mut audio);
    assert!(rendered.frames() >= samples.len() as u64, "the recording must fit");
    assert!(
        energy(rendered.master().samples()) > 0.0,
        "the recording reaches the mix"
    );
    let part = rendered
        .stems()
        .iter()
        .find(|stem| stem.tap().name() == "proof")
        .expect("the score's one part");
    assert_eq!(part.audio().samples().len() as u64, rendered.frames() * 2);
    assert_eq!(
        energy(part.audio().samples()),
        0.0,
        "a resting part contributes silence for the whole of it"
    );
}
