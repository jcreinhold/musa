# Repairs after the first review

**Status: repair record. This document does not set Musa's rules.**

The first review gave the selected draft an **Incorrect** verdict. The repaired files keep that review intact and make
four changes.

## 1. Source primitives are first order

A foreign source primitive may no longer accept or return a source function, even when the function is hidden inside a
constructor, list, event track, primitive configuration, or machine description.

This closes the boxed-loop counterexample. Higher-order finite work such as `map_events` stays in the source calculus,
where the termination proof sees each callback application.

## 2. Batch correctness belongs to the whole machine

One audio frame remains the reference step. A batch method is valid only when it matches repeated frame steps for the
whole machine on which the method is called.

Valid child batches compose through feedback-free `connect` and `beside`. They do not automatically compose through
feedback. A feedback machine runs frame by frame unless its whole batch method has its own proof or test against the
reference steps.

## 3. The scheduling law names its real premise

Scheduling preserves `together` only for an occurrence-local policy. Such a policy assigns each boundary without looking
at neighboring events and resolves same-frame collisions only by ordering.

A policy that shifts or drops one event because of another is allowed, but the preservation law does not apply. Its
combined schedule and decision record are the result.

## 4. Resource costs are fixed integers

The source evaluator carries an integer budget. A versioned table gives each step and constructed value a fixed integer
cost. Wall time, allocator behavior, and current machine load never affect the result.

This makes the resource-error boundary deterministic.

## 5. Small correction

The feedback state equation now uses `initial`, the name that appears in the constructor and start rule.

