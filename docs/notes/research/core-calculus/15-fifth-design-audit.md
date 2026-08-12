# Fifth design audit

**Status: findings found before the final proof review. This document does not set Musa's rules.**

## Findings

### Medium

1. **A primitive id and version did not explicitly select one implementation.**
   - **Location**: selected calculus §5.1, structural equality, and proof assumption A2.
   - **Problem**: structural equality compares a primitive's id, version, and configuration. If one build could register
     two different step functions under the same id and version, equal machine values could run differently.
   - **Repair**: preparation uses one finite build-local registry. It rejects a repeated id and version unless the exact
     primitive definition, state layout, configuration codec, resource contract, and batch contract are the same.

### Low

1. **Testing was described as if it could prove the batch law.**
   - A differential test can find a bad batch implementation but cannot prove equality for every state and input. The
     theorem remains conditional. A primitive owner must state the law as a trusted contract; proof or exhaustive
     checking can discharge it, while tests provide evidence and regression protection.

## Verdict

- **Decision**: Incomplete before repair.
- **Basis**: a functional build-local registry is required for structural machine meaning.
- **Clean passes**: no persistent binary identity, package cache, or behavioral-equivalence decision is needed.

