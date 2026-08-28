/* The one place a prepared instrument crosses between threads.
 *
 * `06-daw-boundary.md` §7 says the render block never allocates, locks, or
 * destroys. Publishing a newly prepared instrument therefore cannot be a
 * Swift property assignment: that is a reference-counted store the render
 * thread would have to synchronize with. It is this instead — a pointer and
 * a generation counter, written with release ordering on the control worker
 * and read with acquire ordering in the block.
 *
 * The old instrument is not freed here. `musa_au_slot_publish` hands it back
 * to the caller, which is on the control side, because releasing a prepared
 * graph is exactly the destruction the render thread may not do.
 */

#ifndef MUSA_AU_SLOT_H
#define MUSA_AU_SLOT_H

#include <stdint.h>

#include "musa_au.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    /* Written by the control worker, read by the render block. Accessed only
     * through the four functions below, which carry the ordering. */
    _Atomic(MusaAuInstrument *) current;
    _Atomic(uint32_t) generation;
} MusaAuSlot;

/* An empty slot. Call once, before anything can read it. */
void musa_au_slot_init(MusaAuSlot *slot);

/* Install `next`, returning whatever was there so the caller can retire it.
 * Control side only. */
MusaAuInstrument *musa_au_slot_publish(MusaAuSlot *slot, MusaAuInstrument *next);

/* The installed instrument, or NULL. Real-time safe. */
MusaAuInstrument *musa_au_slot_current(const MusaAuSlot *slot);

/* How many times this slot has been published to. Real-time safe; a host
 * reads it to tell "not ready yet" from "ready and silent". */
uint32_t musa_au_slot_generation(const MusaAuSlot *slot);

/* The same publication, for the piece a MIDI Processor reads.
 *
 * A separate type rather than a void slot: the two components publish two
 * unrelated things, and one slot holding either would be a place where a
 * wrong cast compiles. They share no mutable state — a processor and an
 * instrument in the same session are two components, not two views of one.
 */
typedef struct {
    _Atomic(MusaAuSchedule *) current;
    _Atomic(uint32_t) generation;
} MusaAuScheduleSlot;

void musa_au_schedule_slot_init(MusaAuScheduleSlot *slot);
MusaAuSchedule *musa_au_schedule_slot_publish(MusaAuScheduleSlot *slot, MusaAuSchedule *next);
MusaAuSchedule *musa_au_schedule_slot_current(const MusaAuScheduleSlot *slot);
uint32_t musa_au_schedule_slot_generation(const MusaAuScheduleSlot *slot);

#ifdef __cplusplus
}
#endif

#endif /* MUSA_AU_SLOT_H */
