# vb-kyyf Proof Plan Review Input (Attempt 3 — COMMAND REPAIR)

**Bead**: vb-kyyf
**State**: 4 (proof-planning)
**Reviewer**: proof-reviewer (before State 5 implementation)
**Manifest**: contract.md, verification-layers.md, proof-obligations.jsonl, traceability-matrix.jsonl, delivery-scope.jsonl
**Rerun**: Attempt 3 — State 11 rejected for invalid `-p workspace_tests` package commands; controller patched proof-obligations.planned.jsonl; proof-strategy.md commands now validated against patched JSONL.

---

## Plan Summary

4 lanes proposed: BDD (MANDATORY), TLA+ (MANDATORY), Verus (MANDATORY), GATE (MANDATORY).
3 lanes pruned/waived: Kani, proptest, fuzz.
2 lanes status `blocked_tooling`: TLA+ (spec file missing), Verus (normalization kernel target missing).
3 BDD sub-targets status `blocked_file_missing`: vb_kyyf_cross_run_determinism.rs (BDD-KYYF-001/003/006).

---

## Discovery Evidence (Mandatory Verification Gate)

```
verification/tla/VbKyyfReplayDeterminism.tla          MISSING — TLA+ blocked_tooling
crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs  MISSING — BDD-001/003/006 blocked
crates/workspace_tests/src/vb_kyyf_normalization.rs    MISSING — Verus blocked_tooling
crates/vb_storage/tests/replay_resume.rs                EXISTS
crates/vb_storage/tests/recovery_bdd_tests.rs           EXISTS
crates/vb_codegen/src/tests.rs                         EXISTS
crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs  EXISTS
```

---

## Questions for proof-reviewer

### Q1: Is `blocked_tooling` status correct for TLA+ and Verus lanes?

TLA+ target `verification/tla/VbKyyfReplayDeterminism.tla` does not exist in workspace. Verus target `crates/workspace_tests/src/vb_kyyf_normalization.rs` does not exist and the kernel location is TBD per implementation. Are these correctly `blocked_tooling` rather than `planned`? If spec author in State 5 is expected to create the target as part of the lane itself, should status be `planned` with `rerun_from: 5`?

### Q2: Should BDD-KYYF-001/003/006 (targeting vb_kyyf_cross_run_determinism.rs) be `blocked_tooling` or `planned`?

The file does not exist. Implementation authors it in State 5. Is `blocked_file_missing` appropriate, or should this be `planned` with `rerun_from: 5` like TLA+ and Verus? The distinction matters for how formal-verifier schedules the ledger.

### Q3: Is the BDD lane sufficient for POST-001..POST-006 and INV-007 given 3 of 7 scenarios share a missing target file?

The 4 existing targets (replay_resume.rs, recovery_bdd_tests.rs, vb_codegen/src/tests.rs, vb_hxm0_acceptance_catalog.rs) cover BDD-KYYF-002/004/005/007. BDD-KYYF-001/003/006 all target the same missing file. Should these be split into separate obligation rows once the file exists, or tracked as a single blocked group?

### Q4: Does the GATE lane correctly classify bead-local failures vs unrelated global failures?

Evidence: moon ci as workspace-wide gate after all scoped evidence passes. Contract says: POST-006 requires scenario-level traceable output; ERR-009 requires evidence artifact path. If moon ci fails but all scoped commands pass, formal-verifier must classify. Is this sufficient or does GATE-KYYF-001 need a narrower scoped command?

---

## JSONL Parse Status of proof-obligations.planned.jsonl

**ATTEMPT 1 (CONTAMINATED)**: 12 rows, valid JSON, but ALL rows missing required fields `verifier`, `artifact`, `waiver`, `requirement_id`. Used non-standard fields `lane`, `pruned`, `pruned_reason`. proof-plan-review-input.md falsely claimed "No parse errors detected" — only checked JSON syntax, not schema compliance.

**ATTEMPT 2 (REJECTED)**: Same schema defects as Attempt 1. State 11 formal-verifier rejected: PO-001/003/006/007 used invalid `-p workspace_tests` package name (package does not exist).

**ATTEMPT 3 (THIS RUN) — COMMAND REPAIR**:
```
python3 validation:
  Rows: 10
  Parse errors: 0
  Schema errors: 0
  Package command validation: 0 invalid -p workspace_tests commands
  Commands verified:
    PO-001: cargo test -p velvet-ballistics-workspace-tests --test vb_kyyf_cross_run_determinism -- --test-threads=1 ✓
    PO-003: cargo test -p velvet-ballistics-workspace-tests --test vb_kyyf_cross_run_determinism ✓
    PO-006: cargo test -p velvet-ballistics-workspace-tests --test vb_kyyf_cross_run_determinism ✓
    PO-007: cargo test -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog ✓
```

All 10 obligation rows are valid JSON Lines with all required obligation-schema fields. Controller-patched package names confirmed correct.

---

## Risk Flags

1. **TLA+ spec file MISSING** — plan assumes State 5 authoring; if spec cannot be written as described, TLA+ lane may need to be waved or downgraded.
2. **Verus normalization kernel MISSING and location TBD** — plan assumes implementation selects module path in State 5.
3. **BDD test file MISSING** — 3 of 7 scenarios blocked on vb_kyyf_cross_run_determinism.rs authoring in State 5.
4. **CLI binary harness shape unconfirmed** — contract Open Question 1; BDD scenarios must discover CLI conventions before writing tests.
5. **Generated parity API incomplete** — contract Open Question 2; compare_generated_to_ir source-pattern checks alone may not constitute full parity evidence.

---

## Evidence Produced by This Plan

| Lane | Evidence |
|------|----------|
| BDD | .evidence/vb-kyyf/bdd-cross-run-determinism.md, .evidence/vb-kyyf/storage-replay-resume.md, .evidence/vb-kyyf/non-replay-safe-actions.md, .evidence/vb-kyyf/recovery-bdd-errors.md, .evidence/vb-kyyf/generated-ir-parity.md, .evidence/vb-kyyf/generated-subset-fail-closed.md, .evidence/vb-kyyf/acceptance-catalog-traceability.md |
| TLA+ | .evidence/vb-kyyf/tla-replay-determinism.md (TLC invariant violation count, deadlock report, temporal property status) — BLOCKED until spec authored |
| Verus | .evidence/vb-kyyf/verus-normalization.md (Verus proof report PASS/FAIL/waiver) — BLOCKED until kernel location determined |
| GATE | .evidence/vb-kyyf/moon-ci.md (exit code, classification of failures) |
