#include "MusaAuSlot.h"

void musa_au_slot_init(MusaAuSlot *slot) {
    __c11_atomic_store(&slot->current, (MusaAuInstrument *)0, __ATOMIC_RELAXED);
    __c11_atomic_store(&slot->generation, 0u, __ATOMIC_RELAXED);
}

MusaAuInstrument *musa_au_slot_publish(MusaAuSlot *slot, MusaAuInstrument *next) {
    /* Release, so that everything the worker wrote while preparing `next` is
     * visible to the block that acquires this pointer. */
    MusaAuInstrument *previous = __c11_atomic_exchange(&slot->current, next, __ATOMIC_ACQ_REL);
    __c11_atomic_fetch_add(&slot->generation, 1u, __ATOMIC_RELEASE);
    return previous;
}

MusaAuInstrument *musa_au_slot_current(const MusaAuSlot *slot) {
    return __c11_atomic_load((_Atomic(MusaAuInstrument *) *)&slot->current, __ATOMIC_ACQUIRE);
}

uint32_t musa_au_slot_generation(const MusaAuSlot *slot) {
    return __c11_atomic_load((_Atomic(uint32_t) *)&slot->generation, __ATOMIC_ACQUIRE);
}
