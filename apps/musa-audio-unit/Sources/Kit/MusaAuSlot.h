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

#ifdef __cplusplus
}
#endif

#endif /* MUSA_AU_SLOT_H */
