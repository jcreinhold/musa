# Intervention Patterns

Apply the lowest layer that matches the profile.

1. Remove repeated work or choose a better algorithm.
2. Borrow, intern, index, or cache only when lifetime and reuse justify it.
3. Allocate by phase or arena when data dies together; do not extend lifetimes by accident.
4. Improve layout and locality for measured traversal patterns.
5. Match hashers and keys to trusted inputs and measured lookup cost.
6. Move rare work off the hot path.
7. Check build profile and backend effects before instruction-level changes.

Do not add `Rc`, `Arc`, caching, precomputed hashes, or custom allocation merely to quiet a local cost.
