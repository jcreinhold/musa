# Repairs after the second review

**Status: repair record. This document does not set Musa's rules.**

## 1. Values and storable data are now distinct

Every source function is a value. A function is not storable data. A product, list, nominal value, or other container is
storable data only when everything inside it is.

Event tracks, machine ports, feedback values, primitive configurations, and foreign source calls require storable data.
Type variables remember whether they may stand for any value or only storable data. Unification preserves that mark.

This closes the generic-payload loophole without dependent types, subtyping, or a public type-class system.

## 2. Evaluation errors now have a typed home

Source evaluation uses a configuration indexed by the expected result type:

```text
run(budget, expression)
done(value)
failed(ResourceError)
```

All three configurations retain the same expected type. A primitive that can fail returns an ordinary `Result`; only the
evaluator's fixed resource limit uses `failed`.

Preservation and progress are now statements about these exact configurations.

