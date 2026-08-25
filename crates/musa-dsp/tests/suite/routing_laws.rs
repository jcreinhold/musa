//! Part/instrument/output isolation across the production audio boundary.

#![allow(clippy::arithmetic_side_effects)]

use super::audio_support::{prepare, render_source};

const FRAMES: usize = 4_096;

fn piece(a_music: &str, b_music: &str, studio: &str) -> String {
    format!(
        "piece \"routing\" {{ tempo 1/4 = 60; meter 4/4; score {{ \
         part a {{ voice v {{ {a_music} }} }} part b {{ voice v {{ {b_music} }} }} }} \
         studio {{ patch shared {{ oscillator(sine) |> gain(-24 dB) |> output; }} \
         patch low {{ oscillator(sine, ratio: 0.5) |> gain(-24 dB) |> output; }} \
         bus hall {{ reverb(room: 0.4, mix: 0.3); }} {studio} }} }}"
    )
}

fn energy(samples: &[f32]) -> f32 {
    samples.iter().map(|sample| sample * sample).sum()
}

#[test]
fn an_unrouted_part_cannot_sound_or_steal_another_parts_voices() {
    let crowded = "[c2 d2 e2 f2 g2 a2 b2 c3 d3 e3 f3 g3 a3 b3 c4 d4 e4]/1";
    let first = piece(
        crowded,
        "a4/1",
        "assign a -> shared; assign b -> shared; route b -> master;",
    );
    let second = piece(
        "rest/1",
        "a4/1",
        "assign a -> shared; assign b -> shared; route b -> master;",
    );
    assert_eq!(
        render_source(&first, FRAMES),
        render_source(&second, FRAMES),
        "part a's seventeen note-ons must reach neither part b's oscillator nor its voice allocator"
    );
}

#[test]
fn equal_pitches_through_one_declaration_are_two_instrument_instances() {
    let both = piece(
        "a4/1",
        "a4/1",
        "assign a -> shared; assign b -> shared; route a -> master; route b -> master;",
    );
    let one = piece(
        "a4/1",
        "a4/1",
        "assign a -> shared; assign b -> shared; route a -> master;",
    );
    let both_energy = energy(&render_source(&both, FRAMES));
    let one_energy = energy(&render_source(&one, FRAMES));
    assert!(
        both_energy > one_energy * 3.9,
        "two quiet equal signals should sum before squaring: {both_energy} versus {one_energy}"
    );
}

#[test]
fn a_send_begins_at_the_named_parts_instrument_output() {
    let first = piece(
        "c4/1",
        "g5/1",
        "assign a -> low; assign b -> shared; send a -> hall at -12 dB; route hall -> master;",
    );
    let second = piece(
        "c4/1",
        "c2/1",
        "assign a -> low; assign b -> shared; send a -> hall at -12 dB; route hall -> master;",
    );
    assert_eq!(
        render_source(&first, FRAMES),
        render_source(&second, FRAMES),
        "the unsent part must not enter the room through a declaration-wide event stream"
    );
}

#[test]
fn an_unassigned_part_keeps_its_default_instance_and_route() {
    let first = piece("c2/1", "a4/1", "assign a -> low;");
    let second = piece("g5/1", "a4/1", "assign a -> low;");
    assert_eq!(
        render_source(&first, FRAMES),
        render_source(&second, FRAMES),
        "an explicit choice for a must neither silence b nor leak into b's default output"
    );
}

#[test]
fn reinstall_and_in_place_seek_keep_the_same_part_bindings() {
    let source = piece(
        "c4/1",
        "g4/1",
        "assign a -> low; assign b -> shared; route a -> master; route b -> master;",
    );
    let mut first = prepare(&source, FRAMES as u64);
    let mut reinstalled = prepare(&source, FRAMES as u64);
    for _ in 0..512 {
        assert_eq!(first.step().map(f32::to_bits), reinstalled.step().map(f32::to_bits));
    }
    first.seek(512);
    for _ in 0..512 {
        assert_eq!(first.step().map(f32::to_bits), reinstalled.step().map(f32::to_bits));
    }
}
