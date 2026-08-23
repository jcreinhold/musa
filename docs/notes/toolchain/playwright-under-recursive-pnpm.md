# Why `pnpm -r test` fails a screen test that passes on its own

**Status: operational. Governs nothing.**

`pnpm -r test` runs each workspace package's test script, and by default it runs several packages at once. The UI
package's script is `pnpm run test:unit && playwright test`, and Playwright then starts its own worker pool on top of
whatever else pnpm has running. On a machine where that oversubscribes the cores, one hover-timing test in
`apps/musa-desktop/ui/tests/screens/` loses its race and the run stops there — `maxFailures` is 1, so the report ends
with "1 failed, 13 did not run" and 175 passed.

**It is not one test.** Measured on 2026-08-23, an M-series laptop, four consecutive `pnpm -r test` runs failed at
`focus.spec.ts:143` ("the focus does not survive leaving the leaf"), then at `origin.spec.ts:128`, then at
`focus.spec.ts:143` again. A failure that moves between runs on an unchanged tree is contention, not a regression.

**What passes.** Both of these are green on the same tree:

```sh
pnpm --filter musa-ui test
```

```sh
pnpm -r --workspace-concurrency=1 test
```

The second is the one to reach for when `pnpm -r test` is a gate: it runs exactly the same scripts and only stops them
from competing for the machine. If a screen test still fails there, it is a real failure — read it as one.

**Why the fix is not in the config.** Playwright's own `workers` setting cannot see how many other pnpm packages are
running beside it, and pinning it low would slow the common case (the UI package alone) to protect the rare one. The
knob that knows is pnpm's, so it belongs on the command line of whoever is running the whole workspace.
