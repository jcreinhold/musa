//  The trial's whole sound and schedule. Deliberately small: a fixed-point
//  sawtooth and a sorted array. Its job is to be obviously correct so that a
//  measurement of the *boundary* is not a measurement of this.

#include "MusaTrialABI.h"

#include <math.h>
#include <string.h>

/// A phase turn is the whole 32-bit range, so the oscillator is exactly
/// periodic and wrapping is the integer overflow rather than a comparison.
static const double kTurn = 4294967296.0;

void musa_trial_plan_prepare(MusaTrialPlan *plan, double sample_rate, float gain, float detune) {
    memset(plan, 0, sizeof(*plan));
    plan->abi_version = MUSA_TRIAL_ABI_VERSION;
    plan->sample_rate = sample_rate;
    plan->gain = gain;
    plan->detune = detune;
    if (sample_rate <= 0.0) {
        return;
    }
    const double cents = (double)detune / 1200.0;
    for (uint32_t note = 0; note < 128u; ++note) {
        const double hertz = 440.0 * exp2(((double)note - 69.0) / 12.0 + cents);
        double increment = hertz * kTurn / sample_rate;
        if (increment < 0.0) {
            increment = 0.0;
        }
        if (increment > kTurn - 1.0) {
            increment = kTurn - 1.0;
        }
        plan->increments[note] = (uint32_t)increment;
    }
}

void musa_trial_instrument_init(MusaTrialInstrument *instrument, double sample_rate) {
    memset(instrument, 0, sizeof(*instrument));
    musa_trial_plan_prepare(&instrument->plan, sample_rate, 1.0f, 0.0f);
}

void musa_trial_instrument_publish(MusaTrialInstrument *instrument, const MusaTrialPlan *plan) {
    instrument->plan = *plan;
    /* A published plan retunes what is already sounding: the increment table
       changed, and a voice reads its note, not a cached frequency. */
    for (uint32_t index = 0; index < MUSA_TRIAL_MAX_VOICES; ++index) {
        MusaTrialVoice *voice = &instrument->voices[index];
        if (voice->sounding) {
            voice->increment = plan->increments[voice->note & 127u];
        }
    }
}

void musa_trial_instrument_stage(MusaTrialInstrument *instrument, const MusaTrialPlan *plan) {
    instrument->staged = *plan;
    const uint32_t next = __atomic_load_n(&instrument->staged_generation, __ATOMIC_RELAXED) + 1u;
    __atomic_store_n(&instrument->staged_generation, next, __ATOMIC_RELEASE);
}

int musa_trial_instrument_apply_staged(MusaTrialInstrument *instrument) {
    const uint32_t staged = __atomic_load_n(&instrument->staged_generation, __ATOMIC_ACQUIRE);
    if (staged == instrument->applied_generation) {
        return 0;
    }
    instrument->applied_generation = staged;
    musa_trial_instrument_publish(instrument, &instrument->staged);
    return 1;
}

void musa_trial_instrument_reset(MusaTrialInstrument *instrument) {
    memset(instrument->voices, 0, sizeof(instrument->voices));
    instrument->rendered_frames = 0;
}

void musa_trial_instrument_note_on(MusaTrialInstrument *instrument, uint8_t note, uint8_t velocity) {
    const uint8_t index_note = note & 127u;
    MusaTrialVoice *chosen = 0;
    for (uint32_t index = 0; index < MUSA_TRIAL_MAX_VOICES; ++index) {
        MusaTrialVoice *voice = &instrument->voices[index];
        if (voice->sounding && voice->note == index_note) {
            chosen = voice;
            break;
        }
        if (!chosen && !voice->sounding) {
            chosen = voice;
        }
    }
    if (!chosen) {
        /* Fixed capacity is a refusal, not a growth. Sixteen voices is the
           trial's whole polyphony and the seventeenth note is dropped. */
        return;
    }
    chosen->note = index_note;
    chosen->velocity = (float)(velocity & 127u) / 127.0f;
    chosen->increment = instrument->plan.increments[index_note];
    chosen->phase = 0;
    chosen->sounding = 1;
}

void musa_trial_instrument_note_off(MusaTrialInstrument *instrument, uint8_t note) {
    const uint8_t index_note = note & 127u;
    for (uint32_t index = 0; index < MUSA_TRIAL_MAX_VOICES; ++index) {
        MusaTrialVoice *voice = &instrument->voices[index];
        if (voice->sounding && voice->note == index_note) {
            voice->sounding = 0;
            voice->phase = 0;
            voice->increment = 0;
            voice->velocity = 0.0f;
        }
    }
}

void musa_trial_instrument_render(MusaTrialInstrument *instrument,
                                  float *left,
                                  float *right,
                                  uint32_t frames) {
    const float gain = instrument->plan.gain;
    for (uint32_t frame = 0; frame < frames; ++frame) {
        float sample = 0.0f;
        for (uint32_t index = 0; index < MUSA_TRIAL_MAX_VOICES; ++index) {
            MusaTrialVoice *voice = &instrument->voices[index];
            if (!voice->sounding) {
                continue;
            }
            /* A naive sawtooth, read straight off the phase word. */
            const float turn = (float)voice->phase * (1.0f / 4294967296.0f);
            sample += (turn * 2.0f - 1.0f) * voice->velocity;
            voice->phase += voice->increment;
        }
        sample *= gain;
        left[frame] = sample;
        right[frame] = sample;
    }
    instrument->rendered_frames += frames;
}

void musa_trial_schedule_init(MusaTrialSchedule *schedule) {
    memset(schedule, 0, sizeof(*schedule));
    schedule->abi_version = MUSA_TRIAL_ABI_VERSION;
}

int musa_trial_schedule_add(MusaTrialSchedule *schedule, MusaTrialEvent event) {
    if (schedule->count >= MUSA_TRIAL_MAX_EVENTS) {
        return 0;
    }
    uint32_t at = schedule->count;
    while (at > 0 && schedule->events[at - 1].beat > event.beat) {
        schedule->events[at] = schedule->events[at - 1];
        --at;
    }
    schedule->events[at] = event;
    schedule->count += 1;
    return 1;
}

uint32_t musa_trial_schedule_lower_bound(const MusaTrialSchedule *schedule, double beat) {
    uint32_t low = 0;
    uint32_t high = schedule->count;
    while (low < high) {
        const uint32_t middle = low + (high - low) / 2u;
        if (schedule->events[middle].beat < beat) {
            low = middle + 1u;
        } else {
            high = middle;
        }
    }
    return low;
}
