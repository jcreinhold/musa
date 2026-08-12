# Repairs after the fifth design audit

**Status: repair record. This document does not set Musa's rules.**

## 1. Primitive lookup is a function

One build has one finite primitive registry. An id and version select one exact primitive contract. Conflicting entries
are errors before audio preparation. This gives structural machine equality a fixed meaning inside that build.

The rule does not promise that a compiled value remains usable in another build. Persistent compiled identity and
package caching remain outside this work.

## 2. Batch correctness remains a contract

The one-frame step is the definition. A batch method is accepted only under the stated equality contract. Proof or
exhaustive checking may establish the contract. Differential tests can catch violations but do not turn a conditional
theorem into an unconditional one.

