# Rust Hotspot Classes

Check these common sources after profiling:

- Phase-local data repeatedly allocated or cloned.
- Normalization and evaluation that rebuild or re-walk terms.
- Traversals that revisit structure or allocate per node.
- Type checking and unification that repeat substitution or lookup.
- Registries, metadata tables, caches, and side maps with poor key or access patterns.
- Closures that capture more state than the hot path needs.
- Pass boundaries that serialize, copy, or invalidate work.
- Persistent structures used in deep write-heavy paths.

For large enums, measure footprint, allocation rate, and cache misses before changing layout. Consider field order, niche use, or boxing rare variants, but recheck hot-path indirection. Interpreter dispatch often depends on branch prediction and instruction-cache behavior; do not choose SIMD before measuring those limits.

Use the profile and input shape to select one; do not assume compiler-like code shares one bottleneck.
