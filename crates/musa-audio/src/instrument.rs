//! The default instrument (roadmap §14.8: a new piece is immediately
//! audible with zero setup): a polyphonic sine synth built from the voice
//! allocator, consuming scheduled `NoteOn`/`NoteOff` events.

use crate::spec::{ProcessorSpec, StudioGraphSpec};

/// A sine polysynth with `voices` voices (§13.5) into the master limiter,
/// designated as output. A piece with no `studio` block renders through this.
///
/// The limiter is here for the same reason it is on every master (§13.6):
/// twenty voices at once is louder than one, and the difference should be a
/// balance the composer hears rather than a clipped file they discover later.
/// It is transparent — exactly unity gain — until the mix actually reaches
/// full scale.
pub fn poly_sine_spec(voices: u8) -> StudioGraphSpec {
    let mut spec = StudioGraphSpec::new();
    let synth = spec.add_node(ProcessorSpec::PolySine { voices });
    let master = spec.add_node(ProcessorSpec::Limiter);
    spec.connect(synth, 0, master, 0);
    spec.set_output(master);
    spec
}
