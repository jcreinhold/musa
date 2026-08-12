# The plan

**Status: directive.** These pages say what to build and where the code has got to. They are bound by
[`../rules/`](../rules/README.md): a plan that contradicts a rule is a defect in the plan, repaired before it is worked.

| Page | What it is |
| --- | --- |
| [`roadmap.md`](roadmap.md) | The broad crate and product plan: layers, crate ownership, DSP rules, what is deliberately rejected. Binding where the rules are silent |
| [`language-design-closure.md`](language-design-closure.md) | Bounded research, proof, and documentation work that will settle the source language before studio and audio implementation continues |
| [`code-map/`](code-map/README.md) | Which crate implements which stage, and what is implemented, partial, or absent. Reports on code; decides nothing |
| [`prompts/`](prompts/README.md) | The numbered work plan, executed in dependency order, one prompt per commit |

The pages answer different questions. `roadmap.md` says what musa is for and how it is carved into crates.
`language-design-closure.md` settles the source language before the pending studio and audio work. `code-map/` says what
exists today. `prompts/` says what happens next, and in what order.

When the roadmap and a specification under `../rules/` disagree, the specification wins and the roadmap is stale. When
`code-map/` and the code disagree, `code-map/` is stale. When a prompt and a rule disagree, the prompt is repaired
first, in its own commit, before it is implemented.
