# 69. Hostile review of the adapter freeze

## Verdict

**Repair required.** The conservativity and sealed-step arguments in note 67 survive the attack, as do the five concrete
musical traces in note 68. Two Medium findings prevent a final verdict. There is no fatal or High finding.

## Findings

### Medium: E11 does not run every law it claims to run

The label says “all sealed-step laws,” but its regular expression selects six tests. It misses
`law_3_a_branch_is_read_under_exactly_the_context_it_was_run_with` and
`law_7_two_runs_of_one_transformer_agree_on_value_and_on_charge`; the latter happens to run under E3, but the map does
not say that. It also fails to select the registry law that proves `run_syntax_step` is exactly projection from the
sealed function and the visibility law that makes a private constructor unnameable. F11 therefore has executable
evidence in the repository but not in the script that claims to map it.

### Medium: E1 does not execute its direct phase-separation witness

The selection runs `a_piece_cannot_write_a_quote_at_all`, which proves one surface consequence. It does not run
`the_phase_vocabulary_is_readable_only_in_a_phase_source`, the direct differential law over the same declaration read
once ordinarily and once as phase source. The prose proof is sound, but the frozen rule's advertised executable map is
incomplete.

### Low: “storable ordinary expression” conflates syntax and value

F1 says an expansion crosses only after it is a “storable ordinary expression.” The phase actually returns
`Syntax Expr`; the gate parses and elaborates that syntax under the ordinary environment, and only an ordinary storable
value may survive the enclosing declaration boundary. The later proof states the sequence more accurately. F1 should say
it once without collapsing the two gates.

## Counterexamples attempted and rejected

- A nested recursor can restart on the original larger subject, so runtime subject size does not decrease globally. Note
  67 does not make that false claim; its reducibility proof handles the nested call independently and uses sealed
  association only when the captured outer step resumes.
- Function-valued contexts and answers do not escape the proof. They use the function clause of the same reducibility
  interpretation, and phase results remain subject to the storable boundary.
- Indexed `Syntax` does not change an ordinary family's conversion. Ordinary resolution never supplies its family or
  constructors to NbE, so the induction on the unchanged judgment is sufficient.
- The five cases do not hide a universal final representation. Each names the exact fork and records projection loss.

## Required repair

Make E1 and E11 select their direct laws, correct F1's crossing sentence, rerun the conformance script, and re-review.
