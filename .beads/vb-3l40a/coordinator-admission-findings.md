# Admission candidate status

Candidate is for independent review, not acceptance or bead closure.

Actual local 5090 evidence: `evidence/admission-binding/full_lib_tests.log`
records 1903 passed, zero failed or ignored. The two public with-contract
runtime APIs and both shard command handlers call workflow-bound validation.
No-contract handlers retain their previous production behavior.

Remaining observed review concerns:

- Negative shard tests check no active run, but do not attach a recording
  journal or assert no journal mutation. Error messages use substring checks.
- Positive tests check real scheduled tickets and terminal membership, but
  terminal membership alone does not distinguish RunFinished from RunFailed
  or verify the output value. Input ticket fields also need review.
- Contract addendum error precedence differs from the combined per-entry
  duplicate/index/name validation order. Review actual multi-error behavior.
- Helper decomposition/formatting and changed fixture API selection require
  review; an earlier report claiming functions <=25 lines was inaccurate.
- Existing test-only production replacements remain tracked separately as
  P0 `vb-8iilq`; library results do not close production integration coverage.

Discarded attempts are retained in the invocation ledger. XML pseudo-tool
output from `vb-admission-finalfix` did not modify files and is not evidence
of implementation. Later `vb-admission-scope` made the actual handler fix.
