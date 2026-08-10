# Rust Boundary Patterns

Use these checks when a Rust type mixes concerns.

| Mixed concern | Prefer |
| --- | --- |
| State and identity | Give stable identity its own type or map. |
| Mechanism and policy | Inject policy only when callers truly vary it; otherwise keep it private. |
| Storage and domain logic | Expose domain operations; keep layout private. |
| Traversal and computation | Let callers choose traversal only if it is their concern. |
| Error and normal path | Make safe missing or empty cases normal where useful. |
| Interface and representation | Replace field-like access with operations that hide layout. |
| Caller vocabulary and module logic | Use neutral operations instead of UI- or caller-named methods. |
| Ordered steps | Encode valid states with types, scoped guards, or builders. |
| Known type family and generic wrapper | Use the concrete bundle until variance appears. |
| Ordering and business logic | Keep comparison policy separate from the operation. |

For each proposed split, name the information each side hides and the callers that need the boundary.
