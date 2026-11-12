bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 13
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Assurance Bundle

### Requirement-to-Evidence Map

| Requirement | Contract | Proof Evidence | Test Evidence | Review Evidence | Command Evidence | Status |
|---|---|---|---|---|---|---|
| EARS-1 | ActionAbiMismatch on mismatch | vb-2bzz-obl-1 PASS | action_abi_mismatch_returns_typed_error | black-hat-review.md APPROVED | cargo test: passed | PASS |
| EARS-2 | PolicyDigestMismatch on mismatch | vb-2bzz-obl-2 PASS | policy_digest_mismatch_returns_typed_error | black-hat-review.md APPROVED | cargo test: passed | PASS |
| EARS-3 | Explicit verifier input | vb-2bzz-obl-3 PASS | empty_input tests | black-hat-review.md APPROVED | cargo test: passed | PASS |
| INV-1 | No false positive ABI | vb-2bzz-obl-3 PASS | action_abi_match_returns_ok | test-suite-review.md APPROVED | cargo test: passed | PASS |
| INV-2 | No false positive policy | vb-2bzz-obl-3 PASS | policy_digest_match_returns_ok | test-suite-review.md APPROVED | cargo test: passed | PASS |
| INV-3 | Matching digests succeed | vb-2bzz-obl-1 PASS | match tests | test-suite-review.md APPROVED | cargo test: passed | PASS |
| INV-4 | Exact identifiers in errors | vb-2bzz-obl-1 PASS | exact field assertions | test-suite-review.md APPROVED | cargo test: passed | PASS |

### Artifact Inventory

| Artifact | Exists | Non-empty |
|---|---|---|
| STATE.md | Yes | Yes |
| baseline-report.md | Yes | Yes |
| research-notes.md | Yes | Yes |
| delivery-scope.jsonl | Yes | Yes |
| contract-spec.md | Yes | Yes |
| test-plan.md | Yes | Yes |
| test-plan-review.md | Yes | Yes |
| test-suite-review.md | Yes | Yes |
| implementation.md | Yes | Yes |
| machine-gate-report.md | Yes | Yes |
| black-hat-review.md | Yes | Yes |
| proof-obligations.jsonl | Yes | Yes |
| traceability-matrix.jsonl | Yes | Yes |
| verification-ledger.jsonl | Yes | Yes |
| formal-verification-report.md | Yes | Yes |
