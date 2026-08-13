---
name: prompt-stack
description: Work the musa implementation prompt stack in docs/plan/prompts/ — select the next pending prompt, implement it, pass its Check, commit, and keep going to the next prompt until the stack is complete or something genuinely needs the user. Repairs or writes prompts when the plan is wrong, grounding those design decisions in the music-theory and functional-language-implementation references on disk. Use for "run the next prompt", "continue the stack", "execute prompt NN", "work the prompts", "keep going", or any unattended run.
---

# Run the musa prompt stack

Execute prompts from `docs/plan/prompts/` in dependency order. Each prompt is one feature, one commit. The conventions
live in `docs/plan/prompts/README.md`; this skill is the operating procedure.

**This runs until the stack is complete.** One prompt finishing is not the end of the job — it is one turn of a loop.
After committing a prompt, go straight back to §1 and start the next one. Only the conditions under *When to stop* end
the run. Do not ask "shall I continue?" between prompts: the user asked for the stack, and stopping to re-confirm each
time spends their attention on nothing. The loop is interruptible at any moment, so if they want it to end they will say
so.

Before the first iteration, note the starting commit (`git rev-parse --short HEAD`). A long run outlives the
conversation's own memory of it; `git log --oneline <start>..HEAD` is the durable ledger of what the run did, and it
stays true after the conversation is summarized.

## 1. Select the prompt

```sh
grep -l '^status: pending' docs/plan/prompts/[0-9]*.md | LC_ALL=C sort -t/ -k3,3n -k3,3
```

Pick the **lowest-numbered** pending prompt whose `depends_on` are all `done` (frontmatter grep). If the user named a
prompt, use that one but first confirm its dependencies are done; if not, run the missing dependencies first or ask.

Select fresh each time. A repair in the previous iteration may have changed what comes next, so never queue up several
prompts in advance or pre-flip their status.

If nothing is pending, the stack is complete — say so and stop.

## 2. Prepare

1. Read the whole prompt file, then every roadmap section its **Read** cites, then the prior-prompt files it names
   (typically the crates/APIs it builds on).
2. Flip its frontmatter to `status: in-progress` (leave uncommitted; the final commit flips it to `done`).
3. Restate the **Task**, **Target**, **Check**, and **Stop** list in one short message before touching code. If anything
   in the prompt contradicts the current repo state, stop and follow §6 (repair) before implementing.

## 3. Implement

- Deliver exactly **Target**, honoring **Design** where it fixes APIs. Internals are yours, under the conventions in
  `docs/plan/prompts/README.md` and root `AGENTS.md`.
- **Stop** is a hard boundary: no "while I'm in here" work. If a stopped item turns out to be genuinely required, that
  is a repair situation (§6), not a license.
- Doc-comment each new public API and its invariants before implementing it.
- Keep examples in `examples/` compiling and rendering.
- Out-of-scope problems you notice in passing are worth recording, not fixing — spawn a background task for them so the
  commit stays prompt-bound.

## 4. Check and commit

1. Run the prompt's **Check** section verbatim, from the repo root. Fix failures until every command passes. Do not
   weaken a check; if the check itself is wrong, that is repair (§6).
2. Flip `status` to `done`.
3. Commit everything with the message the prompt names ("Commit as …"). One prompt, one commit; never mix two prompts'
   work.
4. Report briefly — see *Reporting* below.

A Check command that fails for a reason the change cannot have caused (a pre-existing environment trap, a tool that is
not installed) is not a pass. Prove it is pre-existing, say so plainly in the report, get the underlying question
answered another way, and record the trap for later rather than editing the Check to be silent about it.

## 5. Continue

1. Confirm the tree is clean (`git status --short` is empty). Anything left over means the previous prompt did not
   actually finish; finish it before moving on.
2. Return to §1.

If the run has to end for reasons outside the stack — the user interrupts, or the work no longer fits — end at a
committed boundary with a clean tree and a `status` that matches reality. Never leave a prompt `in-progress` with
uncommitted work as the last state of a run.

## When to stop and ask

Stop the loop, report where you are, and hand the decision back when:

1. **Nothing is pending.** The stack is complete.
2. **The prompt leaves a decision to the user** explicitly.
3. **A Check fails outside the prompt's boundary** — you cannot make it pass without doing work the **Stop** section
   forbids, or the same failure recurs for the same reason after a real fix attempt. Do not thrash; two honest attempts
   is the signal.
4. **A repair would change a governing document** under `docs/rules/`. The constitution and obligations have an
   amendment procedure (`docs/rules/README.md`), and invoking it is the user's call, not a step in a batch run. A prompt
   whose own Task *is* to amend the rules (127a is one) is not this case — that is the plan working as written.
5. **A repair re-opens finished work** — it invalidates a prompt already marked `done`. Repairs that only change pending
   prompts are ordinary; make them and keep going.
6. **Something outward-facing or hard to reverse** comes up: pushing, publishing, releasing, or touching anything beyond
   this repository. Local commits are the plan's own unit of work and need no permission; the rest does.

## 6. Repair a prompt that is wrong

When implementation evidence contradicts the prompt — mis-scoped task, missing prerequisite, wrong API decision:

1. Stop implementing.
2. Edit the prompt file, and any later prompt whose `depends_on` or **Design** assumed the old decision.
3. Run `python3 scripts/renumber-prompts.py audit`.
4. Commit that repair on its own: `Repair prompt NN: <what changed and why>`.
5. Resume from §2 with the repaired prompt.

A repair is a design decision, so make it a *considered* one — see *Consulting theory* below. The prompt's **Read**
section should end up citing whatever the repair was derived from, so the next reader can check the derivation instead
of taking it on trust.

Never silently implement something different from what the prompt says. Code and prompts must not drift (root
`AGENTS.md`).

## 7. Write a prompt the stack is missing

Sometimes the stack does not merely misdescribe the next step — it lacks one. Prefer repairing an existing prompt when
the problem is scope or wording; write a new prompt when a genuinely separate feature has to land first.

1. Number it to insert without disturbing finished work: a lowercase suffix (`127ae`) inserts between two ranks, and
   `python3 scripts/renumber-prompts.py make-room --at N` is right only when nothing that would move is already done.
2. Write the full anatomy — frontmatter (`id`, `slug`, `status`, `depends_on`, `phase`) and **Task**, **Read**,
   **Design**, **Target**, **Check**, **Stop**. A prompt without a real **Stop** will grow in the doing; a prompt whose
   **Check** does not include the four standard commands scoped to the crates it touches (execution rule 4) cannot be
   finished honestly.
3. Add its row to the sequence-overview table in `docs/plan/prompts/README.md`. A prompt file with no row there is drift
   of exactly the kind this repo forbids.
4. Run `python3 scripts/renumber-prompts.py audit`.
5. Commit it alone: `Add prompt NN: <what it delivers and why the stack needed it>`.
6. Resume from §1 — the new prompt is now the lowest-numbered pending one, or it isn't, and either answer is the plan's.

## Consulting theory

Repairing and writing prompts are the moments where design is actually decided, and two reference corpora sit on disk
for exactly this. Reach for them then — not during routine implementation, where the prompt has already decided.

**Music theory** — `~/Code/papers/music-theory/open-music-theory/`, 130 numbered chapters:

| Question | Files |
| --- | --- |
| Notation, clefs, accidentals, spelling | `001`–`007` |
| Rhythm, meter, hypermeter, metrical dissonance | `009`–`012`, `083`, `098`, `117`–`118` |
| Scales, keys, modes, collections | `013`–`015`, `105`–`107` |
| Intervals | `016`, `100` |
| Triads, sevenths, inversion, Roman numerals, figured bass | `017`–`021` |
| SATB voicing and counterpoint | `022`–`031` |
| Cadences, the phrase model, prolongation, sequences, modulation | `036`–`051` |
| Phrase-level and sectional form | `052`–`060`, `086`–`088` |
| Chromatic harmony, mixture, Neo-Riemannian relations | `061`–`073` |
| Jazz and pop: chord symbols, voicings, substitutions, schemas | `074`–`097` |
| Post-tonal: pitch-class sets, prime form, interval vectors | `099`–`104` |
| Twelve-tone rows and serialism | `108`–`113` |
| Orchestration | `114`–`116` |
| Terms | `991-glossary.md` |

**Functional-language implementation** —
`~/Code/papers/logic-and-computation/software-engineering/implementation-of-functional-programming-languages/` (Peyton
Jones), which several prompts already cite by chapter:

| Question | Files |
| --- | --- |
| Lambda calculus; translating a high-level language into it | `02`, `03` |
| Structured types and the semantics of pattern matching | `04`, `05` |
| The enriched lambda calculus and its transformations | `06` |
| Comprehensions | `07` |
| Polymorphic type checking, and a type checker written out | `08`, `09` |
| Program representation and evaluation order | `10`, `11` |
| Graph reduction, supercombinators, lambda lifting, full laziness | `12`–`15` |
| SK combinators; storage management | `16`, `17` |
| The G-machine and its optimizations | `18`–`21` |
| Strictness analysis; pragmatics; parallelism | `22`–`24` |

How to use them well:

- **They govern nothing.** `docs/rules/` governs and `docs/plan/` directs; these are outside the repository. Theory
  tells you what a *good* design would be where the plan is silent or wrong. If theory suggests a governing document is
  wrong, that is stop condition 4, not a repair you make.
- **Take the analysis, not the machinery.** Musa's core is finite, total, and exact: no general recursion, no
  coinductive signals in the core, rational time. Much of the implementation literature solves problems that arise from
  laziness and unbounded computation — graph reduction, GC, the G-machine — which musa deliberately does not have. The
  chapters on typing, pattern matching, and elaboration transfer directly; the runtime chapters mostly describe a
  machine musa has no use for. Importing one because it is in the book would be the opposite of a well-designed step.
- **Take what a presentation must be able to say, not a privileged theory.** The constitution commits musa to plural,
  theory-owned presentations. Open Music Theory is a common-practice-centred pedagogy, so read it for the distinctions a
  presentation has to be able to make — that a spelling is not a pitch class, that a Roman numeral needs a collection to
  mean anything, that meter and hypermeter are different questions — and not as a claim that musa should hard-wire tonal
  common practice.
- **Cite what you used.** A repaired or new prompt should name the chapter in its **Read** section, the way 127aa cites
  chapters 3 and 6. That is what makes the design decision checkable later instead of merely asserted.

## Reporting

During a run, keep per-prompt reports short — what was delivered, that the Check passed (and the exact story where it
did not), and any deviation from **Design** with its reason. The detail belongs in the commit message, which survives; a
long report repeated per prompt buries the run.

When the run ends, for whatever reason, close with: the prompts completed (`git log --oneline <start>..HEAD`), anything
repaired or added along the way, where the stack now stands, and — if you stopped early — which stop condition it was
and what decision is waiting on the user.
