# External Lessons

- Measure before choosing a data structure, hasher, allocator, cache, or parallel design.
- Use allocation profiles for allocation problems, not CPU profiles alone.
- Small-vector and faster-hash choices need measured size distributions and trusted inputs.
- A benchmark should match the input mix and pipeline stage of the claim.
- A representation may trade space for time; expose or document that tradeoff when callers must choose.
