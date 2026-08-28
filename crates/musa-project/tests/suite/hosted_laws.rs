//! Source controls and source outputs, as a host is allowed to see them.
//!
//! `06-daw-boundary.md` Rule D2 says a plug-in parameter is exactly a
//! source-declared exposed control. That single sentence is most of this
//! file: what may be projected, what must be refused with its reason, and
//! what has to stay true about an address once a host has automated it.
//!
//! The other half is §4's rule about who owns time. A host chooses block
//! sizes and a host chooses when a parameter moves, and neither may change
//! the music — so the outputs a host negotiates and the partition it renders
//! in are both tested for having no effect on what bus zero says.

// A fixture that does not contain what a test looks for is the test failing.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::float_cmp)]

use std::path::PathBuf;

use musa_project::{
    HostedControlTable, HostedInstrument, HostedOutputRole, HostedRequest, open_hosted_instrument,
    open_hosted_instrument_at,
};

const RATE: u32 = 48_000;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// The multi-output fixture: `violin` sends to the `hall` room and routes to
/// the master, so its part output, that room, and the main output are three
/// different declared points.
fn glass_mountain() -> HostedInstrument {
    open_hosted_instrument_at(example("glass-mountain.musa"), "violin", RATE).expect("the example prepares")
}

/// The zero-setup fixture: no studio at all, so the edition instrument is
/// the whole of the sound and nothing else drives its parameters.
fn invention() -> HostedInstrument {
    open_hosted_instrument_at(example("invention.musa"), "piano", RATE).expect("the example prepares")
}

fn address_of(instrument: &HostedInstrument, identity: &str) -> u64 {
    instrument
        .controls()
        .iter()
        .find(|control| control.identity == identity)
        .unwrap_or_else(|| panic!("`{identity}` is exposed"))
        .address
}

// === What may be projected =================================================

#[test]
fn every_exposed_parameter_is_a_source_declared_control() {
    let instrument = glass_mountain();
    let identities: Vec<&str> = instrument
        .controls()
        .iter()
        .map(|control| control.identity.as_str())
        .collect();
    // The five normalized controls `note_instrument` declares, in the order
    // the signature declares them. Nothing else: no gain, no cutoff, no
    // graph-local address, and nothing the interface found convenient.
    assert_eq!(
        identities,
        [
            "std.performance::expression",
            "std.performance::emphasis",
            "std.performance::separation",
            "std.performance::brightness",
            "std.performance::sustain",
        ]
    );
}

#[test]
fn a_parameters_range_and_default_come_from_the_declaration() {
    let instrument = glass_mountain();
    for control in instrument.controls() {
        assert_eq!(control.kind, "Normalized", "{} is not normalized", control.identity);
        assert_eq!((control.minimum, control.maximum), (0.0, 1.0));
        assert!(
            control.default >= control.minimum && control.default <= control.maximum,
            "{} defaults outside its own domain",
            control.identity
        );
    }
    // `expression` defaults to one and `brightness` to zero, which is what
    // `std::sound::instrument` declares. A host reading a different number
    // would be reading something this crate invented.
    let expression = instrument
        .controls()
        .iter()
        .find(|control| control.identity == "std.performance::expression")
        .expect("expression is exposed");
    assert_eq!(expression.default, 1.0);
    assert!(expression.continuous);
}

#[test]
fn a_per_note_control_is_projected_without_promising_a_ramp() {
    let instrument = glass_mountain();
    let emphasis = instrument
        .controls()
        .iter()
        .find(|control| control.identity == "std.performance::emphasis")
        .expect("emphasis is exposed");
    // `emphasis` is declared `PerNote`. It is still a parameter — a host may
    // set it — but the declaration does not say the quantity moves between
    // its statements, so nothing here claims a ramp means anything for it.
    assert_eq!(emphasis.update_rate, "PerNote");
    assert!(!emphasis.continuous);
}

#[test]
fn an_incompatible_kind_becomes_a_named_loss_rather_than_a_float() {
    let instrument = glass_mountain();
    let losses: Vec<&str> = instrument
        .control_losses()
        .iter()
        .map(|loss| loss.identity.as_str())
        .collect();
    assert_eq!(
        losses,
        ["std.performance::phrase", "std.sound.basic_sine::partial_ratio"]
    );
    // Each says which fact it could not carry, not that some information may
    // have been lost (`06-daw-boundary.md` §6).
    let phrase = &instrument.control_losses()[0];
    assert_eq!(phrase.kind, "PhraseConnection");
    assert!(phrase.reason.contains("typed relation"), "{}", phrase.reason);
    let ratio = &instrument.control_losses()[1];
    assert_eq!(ratio.kind, "ExactRatio");
    assert!(ratio.reason.contains("no value domain"), "{}", ratio.reason);
    // And neither of them is also a parameter.
    for loss in instrument.control_losses() {
        assert!(
            !instrument
                .controls()
                .iter()
                .any(|control| control.identity == loss.identity),
            "`{}` was refused and published",
            loss.identity
        );
    }
}

#[test]
fn every_parameter_carries_the_sentence_its_declaration_wrote() {
    let instrument = glass_mountain();
    for control in instrument.controls() {
        assert!(
            !control.summary.is_empty(),
            "`{}` reaches a host with nothing to say about itself",
            control.identity
        );
    }
}

// === Addresses =============================================================

#[test]
fn no_published_address_is_zero_or_shared() {
    let instrument = glass_mountain();
    let mut seen = std::collections::BTreeSet::new();
    for control in instrument.controls() {
        assert_ne!(control.address, 0, "`{}` is at address zero", control.identity);
        assert!(seen.insert(control.address), "two controls share one address");
    }
}

#[test]
fn a_table_survives_the_round_trip_it_is_saved_through() {
    let instrument = glass_mountain();
    let encoded = instrument.control_table().encode();
    let decoded = HostedControlTable::decode(&encoded).expect("the table this component wrote is readable");
    assert_eq!(&decoded, instrument.control_table());
}

#[test]
fn an_unchanged_source_keeps_every_address_across_a_restart() {
    let first = glass_mountain();
    let table = first.control_table().encode();
    let again = open_hosted_instrument(&HostedRequest {
        project: example("glass-mountain.musa"),
        piece: None,
        part: "violin".to_owned(),
        sample_rate: RATE,
        table: Some(table),
    })
    .expect("the example prepares again");
    for control in first.controls() {
        assert_eq!(
            address_of(&again, &control.identity),
            control.address,
            "`{}` moved across a restart",
            control.identity
        );
    }
}

#[test]
fn a_collision_is_detected_rather_than_hashed_over() {
    // Whatever `expression` derives, hand that address to somebody else
    // first. A design that trusted the hash would now publish two controls at
    // one address; this one has to notice and place the second elsewhere.
    let taken = address_of(&glass_mountain(), "std.performance::expression");
    let table = HostedControlTable::decode(&format!("musa-control-table 1\n{taken:016x}\tsomeone::else\n"))
        .expect("a table with one entry");
    let extended = table.extended(&["std.performance::expression".to_owned()]);
    let assigned = extended
        .address_of("std.performance::expression")
        .expect("expression is assigned");
    assert_ne!(assigned, taken);
    assert_eq!(extended.address_of("someone::else"), Some(taken));
    let addresses: std::collections::BTreeSet<u64> = extended.entries().iter().map(|entry| entry.address).collect();
    assert_eq!(addresses.len(), extended.entries().len());
}

#[test]
fn a_table_that_records_one_address_twice_is_refused() {
    let error =
        HostedControlTable::decode("musa-control-table 1\n0000000000000007\ta::one\n0000000000000007\ta::two\n")
            .expect_err("a table that collides with itself is not a table");
    assert!(error.contains("twice"), "{error}");
    let error = HostedControlTable::decode("musa-control-table 1\n0000000000000000\ta::one\n")
        .expect_err("address zero names nothing");
    assert!(error.contains("zero"), "{error}");
    let error = HostedControlTable::decode("musa-control-table 2\n").expect_err("a future table is not this one");
    assert!(error.contains("version 2"), "{error}");
}

#[test]
fn a_removed_control_keeps_its_address_out_of_circulation() {
    let table = HostedControlTable::default().extended(&["a::gone".to_owned(), "a::kept".to_owned()]);
    let retired = table.address_of("a::gone").expect("assigned once");
    // The source no longer declares `a::gone`. Every later assignment still
    // has to avoid its address, or a host's old automation lane would start
    // driving a control the composer never connected it to.
    let later = table.extended(&["a::kept".to_owned(), "a::new".to_owned()]);
    assert_eq!(later.address_of("a::gone"), Some(retired));
    assert_ne!(later.address_of("a::new"), Some(retired));
    // And if it comes back, it comes back to the same address.
    let returned = later.extended(&["a::gone".to_owned()]);
    assert_eq!(returned.address_of("a::gone"), Some(retired));
}

#[test]
fn reordering_and_renaming_move_only_what_actually_changed() {
    let before = HostedControlTable::default().extended(&["a::one".to_owned(), "a::two".to_owned()]);
    // Reordered: the same identities in the other order assign nothing new.
    let reordered = before.extended(&["a::two".to_owned(), "a::one".to_owned()]);
    assert_eq!(reordered, before);
    // Renamed: `a::two` became `a::three`. The new name is a new identity and
    // gets a new address; the old one is retired rather than recycled.
    let renamed = before.extended(&["a::one".to_owned(), "a::three".to_owned()]);
    assert_eq!(renamed.address_of("a::one"), before.address_of("a::one"));
    assert_ne!(renamed.address_of("a::three"), before.address_of("a::two"));
    assert_eq!(renamed.address_of("a::two"), before.address_of("a::two"));
}

#[test]
fn a_table_a_component_cannot_read_refuses_the_preparation() {
    let error = open_hosted_instrument(&HostedRequest {
        project: example("glass-mountain.musa"),
        piece: None,
        part: "violin".to_owned(),
        sample_rate: RATE,
        table: Some("not a control table at all\n".to_owned()),
    })
    .expect_err("an unreadable table is not a fresh start");
    let error = error.to_string();
    assert!(error.contains("control table"), "{error}");
}

// === Outputs ===============================================================

#[test]
fn the_outputs_are_the_declared_points_this_part_reaches() {
    let instrument = glass_mountain();
    let outputs: Vec<(&str, HostedOutputRole)> = instrument
        .outputs()
        .iter()
        .map(|output| (output.name.as_str(), output.role))
        .collect();
    assert_eq!(
        outputs,
        [
            ("main", HostedOutputRole::Main),
            ("violin", HostedOutputRole::Part),
            ("hall", HostedOutputRole::Bus),
        ]
    );
    // `strings` is a part of the same piece and is routed to the same master.
    // It is not an output of *this* instrument, and a Music Device that
    // published it would be publishing somebody else's signal.
    assert!(!instrument.outputs().iter().any(|output| output.name == "strings"));
}

#[test]
fn a_piece_with_no_studio_still_has_a_main_output() {
    let instrument = open_hosted_instrument_at(example("invention.musa"), "piano", RATE).expect("the example prepares");
    let outputs: Vec<(&str, HostedOutputRole)> = instrument
        .outputs()
        .iter()
        .map(|output| (output.name.as_str(), output.role))
        .collect();
    // The edition default routes the part straight to the master, so there
    // are two declared points and not one: the part's own output and the
    // main output it reaches.
    assert_eq!(
        outputs,
        [("main", HostedOutputRole::Main), ("piano", HostedOutputRole::Part)]
    );
}

#[test]
fn bus_zero_says_the_same_thing_whatever_a_host_negotiates() {
    // The GarageBand fallback is not a different mix. A host that takes one
    // output and a host that takes all three hear the same main output,
    // frame for frame, because the extra outputs are reads of buffers the
    // frame already wrote.
    let mut all = glass_mountain();
    let mut alone = glass_mountain();
    all.note_on(1, 64, 100);
    alone.note_on(1, 64, 100);
    let mut extra = [[0.0f32; 2]; 2];
    for frame in 0..2_048 {
        let together = all.step_outputs(&mut extra);
        let separate = alone.step_outputs(&mut []);
        assert_eq!(together, separate, "the main output differs at frame {frame}");
    }
}

#[test]
fn an_extra_output_is_not_silence_and_is_not_the_main_output() {
    let mut instrument = glass_mountain();
    instrument.note_on(1, 64, 110);
    let mut extra = [[0.0f32; 2]; 2];
    let mut part_energy = 0.0f32;
    let mut hall_energy = 0.0f32;
    let mut main_energy = 0.0f32;
    for _ in 0..24_000 {
        let [left, right] = instrument.step_outputs(&mut extra);
        main_energy += left.abs() + right.abs();
        part_energy += extra[0][0].abs() + extra[0][1].abs();
        hall_energy += extra[1][0].abs() + extra[1][1].abs();
    }
    assert!(part_energy > 0.0, "the part output delivered nothing");
    assert!(hall_energy > 0.0, "the room the part sends to delivered nothing");
    assert!(main_energy > 0.0, "the main output delivered nothing");
    assert!(
        (main_energy - part_energy).abs() > f32::EPSILON,
        "the main output is not the part output; a send duplicates signal"
    );
}

// === Driving a parameter ===================================================

#[test]
fn an_address_this_instrument_does_not_publish_is_a_loss_and_not_an_error() {
    let mut instrument = glass_mountain();
    assert_eq!(instrument.set_control(1, 0.5, 0), musa_project::HostedOutcome::Unbound);
}

#[test]
fn setting_a_control_to_its_declared_default_changes_nothing() {
    // The differential the projection has to pass: the host path lands on
    // exactly the value preparation already applied, rather than near it.
    // If the mapping were re-derived here instead of reused, this drifts.
    //
    // On the zero-setup instrument, because that is where the claim is
    // exactly true. Where an implementation explicitly writes a private
    // target, preparation leaves that value alone rather than covering it
    // with a signature default — and a host parameter then moves it, exactly
    // as a scheduled control statement does. The test below says so.
    let mut driven = invention();
    let mut untouched = invention();
    for control in untouched.controls().to_vec() {
        driven.set_control(control.address, control.default, 0);
    }
    driven.note_on(1, 64, 100);
    untouched.note_on(1, 64, 100);
    for frame in 0..4_096 {
        assert_eq!(driven.step(), untouched.step(), "frame {frame} differs");
    }
}

#[test]
fn a_ramp_is_gradual_and_arrives_where_a_point_change_would_have_put_it() {
    let address = address_of(&invention(), "std.performance::expression");
    // Gradual: with a note sounding, a ramped parameter and a stepped one do
    // not produce the same frames while the ramp is running.
    let mut ramped = invention();
    let mut stepped = invention();
    ramped.note_on(1, 64, 100);
    stepped.note_on(1, 64, 100);
    ramped.set_control(address, 0.25, 512);
    stepped.set_control(address, 0.25, 0);
    let differed = (0..512).any(|_| ramped.step() != stepped.step());
    assert!(differed, "a ramp that arrived instantly is not a ramp");

    // Arrived: run the ramp out on a silent instrument, then sound a note in
    // each. Identical frames mean the ramp landed on the point change's own
    // destination rather than near it. Comparing the sounding instruments
    // above would compare paths, not destinations — a voice carries the
    // history of how its parameters moved.
    let mut ramped = invention();
    let mut stepped = invention();
    ramped.set_control(address, 0.25, 512);
    stepped.set_control(address, 0.25, 0);
    for _ in 0..1_024 {
        ramped.step();
        stepped.step();
    }
    ramped.note_on(1, 64, 100);
    stepped.note_on(1, 64, 100);
    for frame in 0..2_048 {
        assert_eq!(ramped.step(), stepped.step(), "frame {frame} after the ramp differs");
    }
}

#[test]
fn a_host_parameter_and_the_midi_path_agree_through_one_declared_mapping() {
    // `basic_sine` binds the damper pedal to the `sustain` control, above a
    // declared threshold, with a declared output range. Driving that control
    // as a host parameter and driving it as CC64 are two independent paths
    // into one source mapping, and they have to produce one sound.
    let address = address_of(&invention(), "std.performance::sustain");
    let mut parameter = invention();
    let mut pedal = invention();
    parameter.set_control(address, 1.0, 0);
    pedal.input(musa_project::HostedInput::SustainPedal, 127, None);
    parameter.note_on(1, 64, 100);
    pedal.note_on(1, 64, 100);
    for frame in 0..4_096 {
        assert_eq!(parameter.step(), pedal.step(), "frame {frame} differs");
    }
}

#[test]
fn a_host_parameter_moves_a_value_the_implementation_wrote() {
    // Where an instrument's own graph states a private value, preparation
    // does not cover it with a signature default. A control statement still
    // reaches it, though — that is what a performance control is — so the
    // host parameter has to move it too. A parameter that a host can see and
    // cannot hear would be the worse failure.
    let base = glass_mountain();
    let address = address_of(&base, "std.performance::brightness");
    let mut driven = glass_mountain();
    let mut untouched = glass_mountain();
    driven.set_control(address, 1.0, 0);
    driven.note_on(1, 64, 100);
    untouched.note_on(1, 64, 100);
    let moved = (0..4_096).any(|_| driven.step() != untouched.step());
    assert!(moved, "a published parameter changed nothing that can be heard");
}

#[test]
fn a_control_the_source_does_not_call_continuous_takes_no_ramp() {
    let address = address_of(&glass_mountain(), "std.performance::separation");
    let mut ramped = glass_mountain();
    let mut stepped = glass_mountain();
    ramped.set_control(address, 0.9, 4_096);
    stepped.set_control(address, 0.9, 0);
    ramped.note_on(1, 64, 100);
    stepped.note_on(1, 64, 100);
    for frame in 0..2_048 {
        assert_eq!(ramped.step(), stepped.step(), "frame {frame} differs");
    }
}

#[test]
fn a_value_outside_the_declared_domain_is_held_at_its_end() {
    let address = address_of(&glass_mountain(), "std.performance::brightness");
    let mut beyond = glass_mountain();
    let mut edge = glass_mountain();
    beyond.set_control(address, 4.0, 0);
    edge.set_control(address, 1.0, 0);
    beyond.note_on(1, 64, 100);
    edge.note_on(1, 64, 100);
    for frame in 0..2_048 {
        assert_eq!(beyond.step(), edge.step(), "frame {frame} differs");
    }
}
