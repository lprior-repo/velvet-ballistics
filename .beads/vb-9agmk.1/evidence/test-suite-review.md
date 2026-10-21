# Independent Test Suite Review

## Header

- **Bead:** `vb-9agmk.1`
- **Candidate:** `948f19cb7cd92afec3d95e9fb285216cc6689d41`
- **Candidate base:** `856603b8001c26a199afb91fc9a410bb5715f532`
- **Reviewer:** independent test review
- **Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-9agmk-test-review-luna`
- **Scope:** behavior tests in the candidate; focused test execution; public-API, mutation-resistance, Holzman Rust, and Black Hat review lenses

## Gate Result

**STATUS: REJECTED**

The focused suite is deterministic and passes 15 tests with no ignored cases. Three material contract behaviors still lack tests that would fail under plausible regressions. The candidate should not advance until those cases are covered and the focused command passes again.

## Findings (ordered by severity)

### TEST-GAP-01 — Ledger closure classifications are only asserted for one assumption

**Severity:** BLOCKER

**Location:** `crates/workspace_tests/tests/vb_9agmk_artifact_evidence.rs:89-99`

**Problem:** The ledger test verifies that all ten obligation names exist and that every entry has the policy digest, but it inspects closure type and assumption ID only for `ReplayEffectSeparation`. It never checks `RecoveryClassification` or `TaintHandling`, nor that the obligations intended to be runtime-validated carry the right closure. A mutation that marks either external assumption as `RuntimeValidated` can pass this test because artifact preparation and verification derive the expected ledger from the same builder.

**Contract:** `velvet-ballistics-MASTER.md` §77.4 requires the ledger to distinguish what was proved, runtime-validated, evidenced, attested, and left as an external assumption.

**Required fix:** Assert the exact closure kind and relevant verifier/assumption identity for every ledger entry, including `RuntimeRecoverySemantics` and `RuntimeTaintEnforcement`; assert any expected evidence-digest binding.

**Disposition:** open

### TEST-GAP-02 — Non-empty declared capabilities are not bound through artifact verification

**Severity:** BLOCKER

**Location:** `crates/workspace_tests/tests/artifact_evidence_support/mod.rs:51-64`; `crates/workspace_tests/tests/vb_9agmk_artifact_evidence.rs:170-204,228-253`

**Problem:** The helper's action contract declares `required_capabilities: Box::new([])`. The rehashed-tampering matrix does not mutate `AcceptedArtifact.required_capabilities`; its action-contract tamper case changes only `max_output_bytes`. The persisted runtime-admission case also uses a workflow without a `Do` action and grants an empty set. Thus the new end-to-end tests do not demonstrate that non-empty action requirements flow into the accepted artifact and stay bound when its envelope is rehashed. Deleting the required-capability equality binding in `verify_expected_bindings` would not be caught by this suite.

**Contract:** Master §66 capability contract requires artifacts to declare action requirements and runtime admission to enforce exact membership/cardinality against grants.

**Required fix:** Build an action artifact with at least one required capability; assert the exact artifact requirement set; mutate that set and rebind the outer digest to prove verification rejects; and exercise public artifact admission for exact grant, missing grant, and over-grant.

**Disposition:** open

### TEST-GAP-03 — Policy escalation is tested only at the storage verifier, not runtime admission

**Severity:** BLOCKER

**Location:** `crates/workspace_tests/tests/vb_9agmk_artifact_evidence.rs:104-119,228-253`

**Problem:** The “cannot upgrade policy” assertion calls `verify_accepted_artifact_for_policy` directly. The public runtime admission test stores and admits a Strict artifact under Strict policy. There is no test that stores a Journaled artifact and calls `admit_artifact_run` requesting Strict. A regression removing or weakening the policy-bound verification call in the runtime admission path could therefore leave this suite green.

**Contract:** Master §66 says admission accepts verified artifacts and binds them to runtime policy. The public path is `vb_runtime::admission::admit_artifact_run`.

**Required fix:** Persist or store a Journaled artifact, request Strict through `admit_artifact_run`, and assert the exact typed admission rejection. Keep a matching-policy positive case to show the rejection is caused by policy mismatch.

**Disposition:** open

## Review lenses

- **Test Reviewer:** Public integration coverage exists for compiler, storage verification, malformed/rehashed payloads, and one persisted admission path. No ignored tests or silent error handling were found in the focused suite. The three findings above are mutation-sensitive contract gaps.
- **Holzman Rust:** No candidate production source was changed or linted in this test-only review. The focused integration test module forbids `unsafe`; its exercised tests completed without panic or ignored cases. This is not a crate-wide source-lint verdict.
- **Black Hat:** Contract parity fails on closure classification, capability binding, and policy escalation coverage. No source-code acceptance verdict is issued by this test review.

## Quality Gates

| Gate | Result | Evidence |
|---|---|---|
| `cargo test -p velvet-ballistics-workspace-tests --test vb_9agmk_artifact_evidence -j 2 -- --test-threads=2` | PASS | 15 passed; 0 failed; 0 ignored. Raw stdout/stderr are in this directory. |
| `git diff --check 856603b8001c26a199afb91fc9a410bb5715f532 948f19cb7cd92afec3d95e9fb285216cc6689d41` | PASS | Candidate diff whitespace check. |
| Broader CI / lint / proof lanes | NOT RUN | Outside this bounded behavior-suite review. |

## Verdict

**STATUS: REJECTED**

The candidate tests pass, but their current assertions do not fully protect the artifact trust boundary described by the master contract. Close the three findings with public behavior tests, then repeat this focused command and obtain independent review of the exact updated candidate SHA.
