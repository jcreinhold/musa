//! The default instrument (roadmap §14.8: a new piece is immediately
//! audible with zero setup): a polyphonic sine synth built from the voice
//! allocator, consuming scheduled `NoteOn`/`NoteOff` events.

use crate::spec::{ProcessorSpec, StudioGraphSpec};

/// A spec whose single node is a sine polysynth with `voices` voices
/// (§13.5), designated as output. Until the studio language exists
/// (prompt 24), every part renders through this default instrument.
pub fn poly_sine_spec(voices: u8) -> StudioGraphSpec {
    let mut spec = StudioGraphSpec::new();
    let synth = spec.add_node(ProcessorSpec::PolySine { voices });
    spec.set_output(synth);
    spec
}
