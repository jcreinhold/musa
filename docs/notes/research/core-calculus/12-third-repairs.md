# Repairs after the third design audit

**Status: repair record. This document does not set Musa's rules.**

## 1. Scheduling has no hidden policy

`schedule` now takes a `SchedulePolicy` value. The value fixes rounding, same-frame order, collapse handling, and the
largest frame number it can represent. A decision that cannot be represented returns `ScheduleError`.

## 2. A scheduled source reaches a fixed finished state

The generated source stores a cursor into its finite table and a bounded countdown to the next batch. It never stores an
unbounded mathematical counter. Once it emits the final batch, it enters `Finished`. Every later step returns the same
state and an empty batch.

This makes the generated source compatible with the fixed-memory audio contract.

## 3. Merging scheduled sources keeps event identity apart

`merge_event_batches(policy)` injects left and right handles into disjoint sets before sorting the combined messages. An
instrument can therefore pair each beginning with its own ending even when both source tables used the same local handle
spelling.

## 4. Equal copies receive equal time decisions

An occurrence-local policy cannot use a sorting copy number to move one exact duplicate. The number exists only to
assign distinct private handles. This closes the last gap in the overlay-preservation theorem.

## 5. The algebraic claim is now exact

Machine connection and side-by-side wiring form a symmetric monoidal category under behavioral equality. The calculus
does not claim that side-by-side wiring is a categorical product.

