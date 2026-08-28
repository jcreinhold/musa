//  The tiny versioned C ABI the production boundary expects, trialled at the
//  smallest size that can still be wrong.
//
//  `docs/rules/across-stages/06-daw-boundary.md` §7 says every render callback
//  inherits the real-time rules it already had: no allocation, no lock, no
//  I/O, no logging, no compilation, no decoding, no large destruction. The way
//  Musa intends to keep that promise is to prepare a plan on a control worker
//  and publish it as an immutable C structure the render block only reads.
//  This header is that structure at trial size. It carries no Musa type and
//  links no Musa crate: the question it answers is whether the *shape* works,
//  not whether the synthesizer sounds like anything.
//
//  Nothing here allocates. Every buffer is owned by the caller and every plan
//  is fixed-capacity, so a render block can be handed one and be finished with
//  it without touching the heap.

#ifndef MUSA_TRIAL_ABI_H
#define MUSA_TRIAL_ABI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// The version of this ABI. A host-visible change to any structure below
/// changes this number; a consumer that reads a different one refuses rather
/// than reinterprets.
#define MUSA_TRIAL_ABI_VERSION 1u

/// The largest number of notes one trial instrument sounds at once. Fixed so
/// that publishing a plan never sizes an allocation.
#define MUSA_TRIAL_MAX_VOICES 16u

/// The largest number of occurrences one trial schedule holds.
#define MUSA_TRIAL_MAX_EVENTS 64u

/// A prepared trial instrument: what a control worker would publish.
///
/// Immutable once published. The render block reads it and never writes it;
/// a parameter change replaces the whole plan rather than mutating one, which
/// is the retirement discipline the production runtime already uses.
///
/// `increments` is the point of the exercise. Turning a note number into a
/// phase increment needs `exp2`, and calling libm from a render block risks
/// the first call resolving a lazy symbol under dyld's lock. So the control
/// side does that arithmetic once, for every note, and the render block only
/// indexes a table it was handed.
typedef struct {
    uint32_t abi_version;
    double sample_rate;
    float gain;
    float detune;
    uint32_t increments[128];
} MusaTrialPlan;

/// One sounding voice. Phase is a 32-bit fixed-point turn, so the oscillator
/// is exactly periodic and two runs of the same input are bit-identical
/// without depending on the platform's libm.
typedef struct {
    uint32_t phase;
    uint32_t increment;
    float velocity;
    uint8_t note;
    uint8_t sounding;
    uint8_t reserved[2];
} MusaTrialVoice;

/// The whole render-side state of the trial Music Device. Preallocated by the
/// Swift subclass at `allocateRenderResources` time and never resized.
typedef struct {
    MusaTrialPlan plan;
    MusaTrialPlan staged;
    uint32_t staged_generation;
    uint32_t applied_generation;
    MusaTrialVoice voices[MUSA_TRIAL_MAX_VOICES];
    uint64_t rendered_frames;
} MusaTrialInstrument;

/// One scheduled occurrence, positioned in the host's musical time.
///
/// `beat` is a host beat, not a Musa position: the boundary document (§4)
/// keeps written time, performed time, physical seconds, and host sample time
/// apart, and this is the host's coordinate throughout.
typedef struct {
    double beat;
    double length_beats;
    uint8_t note;
    uint8_t velocity;
    uint8_t reserved[6];
} MusaTrialEvent;

/// A finite, random-access schedule. Sorted by `beat` on publication so a
/// seek is a search rather than a replay from the beginning.
typedef struct {
    uint32_t abi_version;
    uint32_t count;
    MusaTrialEvent events[MUSA_TRIAL_MAX_EVENTS];
} MusaTrialSchedule;

/// Prepare an instrument for a sample rate. Control side only.
void musa_trial_instrument_init(MusaTrialInstrument *instrument, double sample_rate);

/// Prepare a plan for a sample rate, gain, and detuning in cents.
///
/// Control side only: this is where the note table is computed, and the only
/// place in the trial that calls libm.
void musa_trial_plan_prepare(MusaTrialPlan *plan, double sample_rate, float gain, float detune);

/// Publish a prepared plan directly. Control side only, and only while the
/// render block is not running — the trial uses it before render resources
/// exist and in tests.
void musa_trial_instrument_publish(MusaTrialInstrument *instrument, const MusaTrialPlan *plan);

/// Stage a prepared plan for a running render block to pick up.
///
/// Control side only. Wait-free in both directions: the writer fills the
/// staging slot and then releases a generation counter, and the reader
/// acquires that counter and copies. No lock crosses the boundary, which is
/// what obligations §12 requires of anything a callback participates in.
void musa_trial_instrument_stage(MusaTrialInstrument *instrument, const MusaTrialPlan *plan);

/// Take a staged plan, if one is waiting. Real-time safe; returns 1 when a
/// plan was taken. Call once at the top of a render block.
int musa_trial_instrument_apply_staged(MusaTrialInstrument *instrument);

/// Silence every voice and rewind the frame counter. Control side only.
void musa_trial_instrument_reset(MusaTrialInstrument *instrument);

/// Start a note. Real-time safe: called from the render block as host MIDI
/// events are consumed.
void musa_trial_instrument_note_on(MusaTrialInstrument *instrument, uint8_t note, uint8_t velocity);

/// Stop a note. Real-time safe.
void musa_trial_instrument_note_off(MusaTrialInstrument *instrument, uint8_t note);

/// Render `frames` frames into two caller-owned channel buffers.
///
/// Real-time safe. The result depends only on the plan, the sounding voices,
/// and `frames` — never on how the host partitioned the frames, which is what
/// Theorem R1-batch requires of a Musa realization.
void musa_trial_instrument_render(MusaTrialInstrument *instrument,
                                  float *left,
                                  float *right,
                                  uint32_t frames);

/// Prepare an empty schedule. Control side only.
void musa_trial_schedule_init(MusaTrialSchedule *schedule);

/// Append one occurrence, keeping the schedule sorted. Control side only;
/// returns 0 when the fixed capacity is full rather than growing.
int musa_trial_schedule_add(MusaTrialSchedule *schedule, MusaTrialEvent event);

/// The index of the first occurrence at or after `beat`, by binary search.
///
/// Real-time safe, and the reason a seek costs nothing: the processor selects
/// occurrences by host position instead of stepping from the beginning.
uint32_t musa_trial_schedule_lower_bound(const MusaTrialSchedule *schedule, double beat);

#ifdef __cplusplus
}
#endif

#endif /* MUSA_TRIAL_ABI_H */
