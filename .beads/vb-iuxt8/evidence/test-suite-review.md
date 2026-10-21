# Independent artifact test-suite re-review

**Status: REJECTED**

**Finding P2 — Relaxed-policy success is not tied to the requested artifact.** In `crates/workspace_tests/tests/runtime_version_barrier_tests.rs:142-155`, `relaxed_policy_skips_evidence_validation` corrupts the artifact evidence and then only asserts `.is_ok()`. A mutant that returns a fabricated successful admission for any Relaxed request would pass; the test never checks that the returned admission preserves the requested digest/run identity. Assert the exact admitted artifact digest (and run identity if exposed) so this scenario proves the specific artifact was admitted while evidence validation was skipped.

## Review target and provenance

- Candidate: `729778e06599b1d1bc1f3f6655a47e375c928903`
- Candidate parent: `6311991293c539d3016419acea3bb5c2e1f4f843` (the stated code-approved source)
- Candidate subject: `vb-9agmk: repair real-evidence integration fixtures and Journaled binding tests`
- Fresh reviewer workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-9agmk-test-rereview-729778-gitjj`
- `git rev-parse --show-toplevel` and `jj root` both resolved to that workspace.
- Candidate delta from the pinned parent contains evidence files and tests/fixtures only; no production source changes. Candidate tree was not modified during review.
- Review issue: `vb-iuxt8`, discovered from `vb-9agmk`.

## Findings checked

- Exact ledger obligations, order, policy digest, verifier digest, and per-obligation closures are asserted. Rehashed forged closures and duplicate ledger entries are rejected.
- Artifact-derived capability requirements are non-empty in the action workflow; exact grants succeed, missing/extra grants fail with exact errors, and rehashed capability removal fails evidence validation.
- Journaled artifact policy escalation is rejected through both admission entry points.
- Altered valid IR and replacement/omitted contracts are tested through public runtime enqueue and shard processing under both Strict and Journaled policies, with valid matching controls and exact errors/state checks.
- The wave16 capability fixture now prepares a real Strict artifact with dense action/contract registration and uses its digest; all three scenarios pass.
- The version-barrier fixture uses actual compiled evidence and covers claimed proof mutation, unsupported evidence version, altered IR, digest mismatch, missing artifact, compiler rejection, and arbitrary gate counts. The Relaxed-policy case has the P2 assertion gap above.

## Scoped execution

All commands ran from the fresh isolated workspace. Raw stdout, stderr, and exit status are alongside this report.

- `cargo test -p velvet-ballistics-workspace-tests --test vb_9agmk_artifact_evidence -j 2 -- --test-threads=2` — **22 passed, 0 failed, 0 ignored**.
- `cargo test -p vb_runtime --test vb_wave16_mqa -j 2 -- --test-threads=2` — **3 passed, 0 failed, 0 ignored**.
- `cargo test -p velvet-ballistics-workspace-tests --test runtime_version_barrier_tests -j 2 -- --test-threads=2` — **10 passed, 0 failed, 0 ignored**.

These scoped passes do not imply full CI or production-code approval. The candidate is rejected solely on the remaining behavioral assertion gap above.
