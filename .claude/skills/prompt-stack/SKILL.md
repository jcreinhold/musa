---
name: prompt-stack
description: Execute the musa implementation prompt stack in docs/plan/prompts/ — run the next pending prompt (or a named one), verify its Check, flip status, and commit. Use for "run the next prompt", "continue the stack", "execute prompt NN", or batch runs of several prompts.
---

# Run the musa prompt stack

Execute prompts from `docs/plan/prompts/` in dependency order. Each prompt is one feature, one commit. The full
conventions live in `docs/plan/prompts/README.md`; this skill is the operating procedure. Follow it exactly — do not
improvise scope.

## 1. Select the prompt

```sh
grep -l '^status: pending' docs/plan/prompts/[0-9]*.md | LC_ALL=C sort -t/ -k3,3n -k3,3
```

Pick the **lowest-numbered** pending prompt whose `depends_on` are all `done` (frontmatter grep). If the user named a
prompt, use that one but first confirm its dependencies are done; if not, run the missing dependencies first or ask.

If nothing is pending, say so and stop — the stack is complete.

## 2. Prepare

1. Read the whole prompt file, then every roadmap section its **Read** cites, then the prior-prompt files it names
   (typically the crates/APIs it builds on).
2. Flip its frontmatter to `status: in-progress` (leave uncommitted; the final commit flips it to `done`).
3. Restate the **Task**, **Target**, **Check**, and **Stop** list in one short message before touching code. If anything
   in the prompt contradicts the current repo state, stop and follow §5 below (prompt repair) before implementing.

## 3. Implement

- Deliver exactly **Target**, honoring **Design** where it fixes APIs. Internals are yours, under the conventions in
  `docs/plan/prompts/README.md` and root `AGENTS.md`.
- **Stop** is a hard boundary: no "while I'm in here" work. If a stopped item turns out to be genuinely required, that
  is a prompt-repair situation (§5), not a license.
- Doc-comment each new public API and its invariants before implementing it.
- Keep examples in `examples/` compiling and rendering.

## 4. Check and commit

1. Run the prompt's **Check** section verbatim, from the repo root. Fix failures until every command passes. Do not
   weaken a check; if the check itself is wrong, that is prompt repair (§5).
2. Flip `status` to `done`.
3. Commit everything with the message the prompt names ("Commit as …"). One prompt, one commit; never mix two prompts'
   work.
4. Report: what was delivered, the check output in brief, any deviations from **Design** and why.

## 5. Prompt repair (when the prompt is wrong)

When implementation evidence contradicts the prompt — mis-scoped task, missing prerequisite, wrong API decision:

1. Stop implementing.
2. Edit the prompt file (and any later prompts whose `depends_on`/Design assume the old decision) to reflect the correct
   design.
3. Commit that repair on its own: `Repair prompt NN: <what changed and why>`.
4. Resume from §2 with the repaired prompt.

Never silently implement something different from what the prompt says. Code and prompts must not drift (root
`AGENTS.md`).

## Batch mode

When the user asks for several prompts or an unattended run: loop §1–§5, one prompt at a time, committing each before
starting the next. Stop at the first Check failure you cannot fix within the prompt's scope, the first prompt repair
that changes a later prompt's assumptions, or any decision the prompt explicitly leaves to the user — and report where
and why you stopped.
