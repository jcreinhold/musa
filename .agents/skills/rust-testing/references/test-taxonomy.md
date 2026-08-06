# Test Taxonomy

| Kind | Use |
| --- | --- |
| Unit | A local contract or branch. |
| Integration | A public interaction across components. |
| Property | A law over many generated cases. |
| Compile-fail/UI | A required diagnostic or rejected program. |
| Snapshot | Stable user-facing structure. |
| Benchmark/profile | Time, allocation, or scaling behavior. |

Choose the smallest kind that covers the risk. Do not use a snapshot for an invariant that a direct assertion can express.
