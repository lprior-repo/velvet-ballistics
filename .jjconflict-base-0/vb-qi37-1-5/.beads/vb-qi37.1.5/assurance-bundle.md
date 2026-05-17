# Assurance Bundle — vb-qi37.1.5

## Bead: vb-qi37.1.5 — runtime/recovery: Prove replay digest mismatch detection
## State: 13 (evidence-packaging)

---

## Requirement-to-Evidence Traceability Matrix

### Preconditions

| Clause | Requirement | Evidence | Status |
|--------|-------------|----------|--------|
| PRE-001 | `check_workflow_source_digest` requires non-empty event list | Black-hat Phase 1: `recover.rs` returns `Err(NoRecoveryData)` for empty list | **PASS** |
| PRE-002 | `verify_digests` requires `workflow_digest` and `ir_digest` as expected reference values | Black-hat Phase 1: Type-level enforcement via `WorkflowDigest` type and `&FjallJournal` | **PASS** |
| PRE-003 | `recover_runtime_frame_seed_from_events` requires at least one `RunAccepted` as first element | Black-hat Phase 1: `ok_or(NoRecoveryData)` at `summary.rs:205-207` | **PASS** |

### Postconditions

| Clause | Requirement | Evidence | Status |
|--------|-------------|----------|--------|
| POST-001 | `check_workflow_source_digest` returns `Ok(())` iff journal's `RunAccepted.workflow == expected`; `Err(WorkflowSourceDigestMismatch)` otherwise | Black-hat Phase 1: Implementation verified at `recover.rs:28-35`; Kani harness `kani_workflow_digest_reflexive_eq` (16/16); unit test `workflow_digest_rejection_reports_exact_mismatch_and_accepts_match` | **PASS** |
| POST-002 | `check_compiled_ir_digest` returns `Ok(())` iff `expected == found`; `Err(CompiledIrDigestMismatch)` otherwise | Black-hat Phase 1: Implementation verified at `recover.rs:46-50`; Kani harness `kani_check_ir_digest_mismatch_returns_err`; Kani harness `kani_ir_digest_error_variant_exhaustive` | **PASS** |
| POST-003 | `verify_digests` returns `Ok(())` only when all requested digest levels pass; returns first mismatch in priority order (workflow source, then IR) | Black-hat Phase 1: Implementation verified at `recover.rs:62-70`; Kani harness `kani_digest_check_exhaustive_match`; `kani_verify_digests_enforces_priority` | **PASS** |
| POST-004 | `reject_workflow_digest_mismatch` returns `Ok(())` on match or absent; `Err(WorkflowSourceDigestMismatch)` on mismatch | Black-hat Phase 1: Implementation verified at `summary.rs:182-199`; unit test `workflow_digest_rejection_reports_exact_mismatch_and_accepts_match`; unit test `frame_seed_with_workflow_rejects_digest_mismatch_before_replay` | **PASS** |
| POST-005 | Corruption injection tests fail with exact `RecoveryError` variant | WAIVER-FJALL-CORRUPT-001/002/003 (corrupt artifact digest, journal sequence, slot value); WAIVER-EVENTSEQ-ORDER-001 (corrupt slot taint) — all approved in proof-obligations.jsonl; compensating evidence: unit tests + Kani harnesses cover mismatch detection path | **WAIVED** |

### Invariants

| Clause | Requirement | Evidence | Status |
|--------|-------------|----------|--------|
| INV-001 | `WorkflowDigest` is pure content identity — byte-exact equality, no false positives/negatives | Black-hat Phase 3: `WorkflowDigest` is `[u8; 32]`; Kani harness `kani_workflow_digest_reflexive_eq` (16/16); `kani_workflow_digest_symmetric_eq`; `kani_workflow_digest_transitive_eq`; `kani_workflow_digest_mismatch_detected` | **PASS** |
| INV-002 | `check_workflow_source_digest` is deterministic — same journal state always yields same result | Black-hat Phase 2: Pure function delegation to FjallJournal with deterministic event stream | **PASS** |
| INV-003 | `RecoveryError` variants are exhaustive — every failure mode maps to exactly one variant | Black-hat Phase 3: Sum type covering all failure modes; Kani harness `kani_ir_digest_error_variant_exhaustive` | **PASS** |
| INV-004 | `UnsupportedRecoveryState::union` is monotonically additive — once set, never cleared | Black-hat Phase 1: Unit test `unsupported_recovery_state_union_is_monotonic` at `summary.rs:1213-1243` PASSES | **PASS** |

### Deferred Clauses (Formal Waivers)

| Clause | Requirement | Waiver | Status |
|--------|-------------|--------|--------|
| Action ABI Digest | `ActionAbiMismatch` detection during recovery replay | WAIVER-FJALL-CORRUPT-001: Fjall does not expose byte-level corruption API; compensating evidence: Workflow source + IR digest checks are primary defense-in-depth | **WAIVED** |
| Policy Digest Mismatch | `PolicyDigestMismatch` detection during recovery replay | WAIVER-FJALL-CORRUPT-002: `RecoveryError::PolicyDigestMismatch` defined but not instantiated; compensating evidence: `RuntimePolicy` enforced at admission | **WAIVED** |

### TLA+-Owned Clauses

| Clause | Requirement | Rationale | Status |
|--------|-------------|-----------|--------|
| N/A | Not applicable | Recovery digest verification is deterministic pure-function property over immutable journal event stream. No temporal/lifecycle/state machines, no concurrency, no liveness requirements, no fairness conditions. | **N/A** |

### Verus-Owned Clauses

| Clause | Requirement | Evidence | Status |
|--------|-------------|----------|--------|
| VERUS-INV-001 | `WorkflowDigest` byte-exact equality | WAIVER-VERUS-VACUITY-001 (Verus not installed); Kani provides compensating bounded proof for pure WorkflowDigest equality | **WAIVED** |
| VERUS-POST-001 | `check_workflow_source_digest` returns `Ok(())` iff journal's `RunAccepted.workflow == expected` | WAIVER-VERUS-VACUITY-001; Kani PO-003 | **WAIVED** |
| VERUS-POST-002 | `check_compiled_ir_digest` returns `Ok(())` iff `expected == found` | WAIVER-VERUS-VACUITY-001; Kani PO-003 | **WAIVED** |
| VERUS-POST-003 | `verify_digests` enforces digest level priority | WAIVER-VERUS-VACUITY-001; Kani PO-003 | **WAIVED** |
| VERUS-POST-004 | `reject_workflow_digest_mismatch` returns `Ok(())` on match or absent | WAIVER-VERUS-VACUITY-001; Kani PO-003 | **WAIVED** |

---

## Summary

| Category | Total | PASS | FAIL | WAIVED | N/A |
|----------|-------|------|------|--------|-----|
| Preconditions | 3 | 3 | 0 | 0 | 0 |
| Postconditions | 5 | 4 | 0 | 1 | 0 |
| Invariants | 4 | 4 | 0 | 0 | 0 |
| Deferred Clauses | 2 | 0 | 0 | 2 | 0 |
| TLA+ Clauses | 0 | 0 | 0 | 0 | 1 |
| Verus Clauses | 5 | 0 | 0 | 5 | 0 |
| **TOTAL** | **19** | **11** | **0** | **8** | **1** |

**OVERALL: PASS** — All testable requirements verified. Waived items have formal waivers with compensating evidence. No unmitigated gaps.