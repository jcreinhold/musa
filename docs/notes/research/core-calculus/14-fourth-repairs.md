# Repairs after the fourth design audit

**Status: repair record. This document does not set Musa's rules.**

## 1. Time maps preserve order

Scheduling checks the finite set of boundaries it actually uses. If source boundary `x` is no later than `y`, the mapped
physical time and assigned frame for `x` may not be later than those for `y`. A failure returns `ScheduleError`.

This still permits rubato, swing, fermatas, and gradual tempo change. It forbids calling a time-reversing relation a
time map.

## 2. Event spans are half open

A positive occurrence from `s` to `e` occupies `[s,e)`. Its beginning affects frame `s`; its ending takes effect before
frame `e` is produced. A point occurrence is one message at one frame.

The machine rule already reads an input before returning the output for that step. The new statement applies that rule
to instruments and removes the ambiguity.

## 3. The scheduling law compares successful results

The overlay law now says that the combined schedule and both separate schedules succeeded. It makes no equation between
a `ScheduleError` and a running machine.
