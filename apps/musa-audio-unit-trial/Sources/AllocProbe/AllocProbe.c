//  An allocation probe for the trial harness.
//
//  `docs/rules/across-stages/06-daw-boundary.md` §7 says a render block does
//  not allocate. Apple publishes no way to assert that, so the trial inserts
//  this library into the harness with `DYLD_INSERT_LIBRARIES` and counts the
//  allocations that happen while the probe is armed. It reaches in-process
//  rendering only: an out-of-process extension runs in a process the system
//  spawned, which this library is not inserted into, and the report says so
//  rather than implying otherwise.
//
//  The replacements call `malloc_zone_*` rather than `malloc`, because a call
//  to `malloc` from inside the replacement would be interposed too.

#include <malloc/malloc.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

static atomic_int probe_armed = 0;
static atomic_ullong probe_allocations = 0;

/// Start or stop counting. Called from the control side around a render.
void musa_probe_arm(int armed) {
    atomic_store_explicit(&probe_armed, armed, memory_order_release);
}

/// How many allocations happened while armed.
unsigned long long musa_probe_allocations(void) {
    return atomic_load_explicit(&probe_allocations, memory_order_acquire);
}

/// Forget the count, so one experiment cannot read another's.
void musa_probe_reset(void) {
    atomic_store_explicit(&probe_allocations, 0, memory_order_release);
}

static void musa_probe_note(void) {
    if (atomic_load_explicit(&probe_armed, memory_order_acquire)) {
        atomic_fetch_add_explicit(&probe_allocations, 1, memory_order_relaxed);
    }
}

static void *musa_probe_malloc(size_t size) {
    musa_probe_note();
    return malloc_zone_malloc(malloc_default_zone(), size);
}

static void *musa_probe_calloc(size_t count, size_t size) {
    musa_probe_note();
    return malloc_zone_calloc(malloc_default_zone(), count, size);
}

static int musa_probe_posix_memalign(void **pointer, size_t alignment, size_t size) {
    musa_probe_note();
    void *block = malloc_zone_memalign(malloc_default_zone(), alignment, size);
    if (!block) {
        return 12; /* ENOMEM */
    }
    *pointer = block;
    return 0;
}

static void *musa_probe_realloc(void *pointer, size_t size) {
    musa_probe_note();
    malloc_zone_t *zone = pointer ? malloc_zone_from_ptr(pointer) : malloc_default_zone();
    if (!zone) {
        zone = malloc_default_zone();
    }
    return malloc_zone_realloc(zone, pointer, size);
}

#define MUSA_INTERPOSE(replacement, replacee)                                                    \
    __attribute__((used)) static struct {                                                        \
        const void *replacement;                                                                 \
        const void *replacee;                                                                    \
    } musa_interpose_##replacee __attribute__((section("__DATA,__interpose"))) = {               \
        (const void *)(unsigned long)&replacement, (const void *)(unsigned long)&replacee}

MUSA_INTERPOSE(musa_probe_malloc, malloc);
MUSA_INTERPOSE(musa_probe_calloc, calloc);
MUSA_INTERPOSE(musa_probe_realloc, realloc);
MUSA_INTERPOSE(musa_probe_posix_memalign, posix_memalign);
