# Formal Verification Report: vb-core-cli-accepted-path (State 11)

bead_id: vb-core-cli-accepted-path
phase: 11
runner: formal-verifier
updated_at: 2026-05-16T21:17:00Z

## Isolation Verification

- `pwd -P` → `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path` ✓
- Not nested under source checkout `/home/lewis/src/velvet-ballistics` ✓
- Case guard confirms isolation ✓

## Executive Summary

**VERDICT: FORMAL VERIFICATION INCOMPLETE**

| Obligation | Verifier | Result | Classification |
|---|---|---|---|
| PO-001 (TLA+) | tla-plus | PASS | PASS |
| PO-002 (Verus digest) | verus | PASS | PASS |
| PO-003 (Verus policy) | verus | PASS | PASS |
| PO-004 (Verus admission) | verus | PASS | PASS |
| PO-007 (Kani gauntlet) | kani | PASS | PASS |
| PO-007 LETHAL-1 (digest mismatch) | kani | PASS | PASS |
| PO-007 LETHAL-2 (admit_run bypass) | kani | **FAIL** | **FAIL_LOCAL** |
| PO-011 (lint-src) | static-scan | PASS | PASS |
| PO-011 (source-length) | FAIL | FAIL | FAIL_LOCAL |
| PO-011 (agent-cli-contract) | static-scan | PASS | PASS |

## CRITICAL DISCREPANCY

**User claimed: "DEFECT-12-01 admit_run bypass is NOW FIXED (now uses AcceptedArtifactStore for strict/journaled policies)"**

**Actual Evidence:** `strict_legacy_presence_only_bypass_rejects_required_blocker` Kani harness FAILS at line 217:9 with description "strict presence-only bypass must reject before admission".

**Code Evidence:**
- `admit_run` at `crates/vb_runtime/src/admission.rs:367-383` still takes `&dyn ArtifactStore` (presence-only interface)
- For `RuntimePolicy::Strict | RuntimePolicy::Journaled`, only `compiled_ir_exists(digest)` is checked
- `AlwaysPresentArtifactStore::compiled_ir_exists()` always returns `true`
- No digest validation, no gate validation, no proof validation

**Root Cause:** The fix described by user has NOT been applied to the current code. `admit_run` still uses `ArtifactStore` trait, not `AcceptedArtifactStore` trait.

## Detailed Results

### PO-001 / TLA-ACCEPT-001 (TLA+)

**Command:** `tlc -config verification/tla/AcceptedCliAdmission.cfg verification/tla/AcceptedCliAdmission.tla`

**Result:** PASS

**Evidence:**
```
306 states generated, 226 distinct states found, 0 states left on queue
The depth of the complete state graph search is 7
Checking 2 branches of temporal properties for the complete state space
Model checking completed. No error has been found.
```

### PO-002 / VERUS-DIGEST-001 (Verus)

**Command:** `verus verification/verus/accepted_cli_digest_binding.rs`

**Result:** PASS

**Evidence:** `verification results:: 3 verified, 0 errors`

### PO-003 / VERUS-POLICY-001 (Verus)

**Command:** `verus verification/verus/strict_admission_witness.rs`

**Result:** PASS

**Evidence:** `verification results:: 6 verified, 0 errors`

### PO-004 / VERUS-ADMISSION-001 (Verus)

**Command:** `verus verification/verus/accepted_artifact_admission_decision.rs`

**Result:** PASS

**Evidence:** `verification results:: 10 verified, 0 errors`

### PO-007 / KANI-ADMISSION-001 (Kani Aggregate)

**Command:** `moon run :verify-proof`

**Result:** PASS

**Evidence:**
```
[PASS] KANI-ADMISSION-001-MALFORMED-GATE-PROOF-REJECT
[PASS] KANI-ADMISSION-001-CAPABILITY-REJECT
[PASS] KANI-ADMISSION-001-VALID-ACCEPT
[PASS] All proof checks passed
Tasks: 1 completed
```

### PO-007 LETHAL-1: Digest Mismatch (Kani Focused)

**Command:** `cargo kani --package vb_runtime --harness strict_admission_digest_mismatch_rejects_required_blocker --default-unwind 1`

**Result:** PASS

**Evidence:** `0 of 611 failed (10 unreachable), VERIFICATION:- SUCCESSFUL`

**Finding:** LETHAL-1 RESOLVED. `admit_artifact_run` correctly checks decoded digest vs requested digest after State 10 fix.

### PO-007 LETHAL-2: admit_run Bypass (Kani Focused) - CRITICAL FAILURE

**Command:** `cargo kani --package vb_runtime --harness strict_legacy_presence_only_bypass_rejects_required_blocker --default-unwind 1`

**Result:** FAIL

**Evidence:**
```
SUMMARY:
 ** 1 of 120 failed (2 unreachable)
Failed Checks: strict presence-only bypass must reject before admission
 File: "crates/vb_runtime/src/kani_capability_harnesses.rs", line 217
VERIFICATION:- FAILED
```

**Finding:** DEFECT-12-01 admit_run BYPASS STILL OPEN

**Location:** `crates/vb_runtime/src/admission.rs:367-383`

**Root Cause:** `admit_run` function:
```rust
pub fn admit_run(
    store: &dyn ArtifactStore,  // <-- Presence-only interface
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            if !store.compiled_ir_exists(digest) {  // <-- Only presence check
                return Err(AdmissionError::ArtifactNotFound { digest });
            }
        }
        RuntimePolicy::Relaxed => {}
    }
    Ok(RunAdmission::new(digest, run_id, caps, policy))  // <-- Incorrectly admits
}
```

**Problem:** `AlwaysPresentArtifactStore::compiled_ir_exists()` always returns `true`. For `RuntimePolicy::Strict`, `admit_run` only checks presence, not full artifact validation (gates, proof, capability). This enables strict policy bypass.

**Required Fix:** `admit_run` must use `AcceptedArtifactStore` (full validation via `load_accepted_artifact()`) for `Strict` and `Journaled` policies, not `ArtifactStore` (presence-only via `compiled_ir_exists()`).

### PO-011 / STATIC-SCAN-001 (Static Analysis)

**Commands:**
- `moon run :lint-src` → PASS (Tasks: 1 completed)
- `moon run :source-length` → FAIL
  - `crates/vb_runtime/src/error/equality.rs:91` has 28 logical lines (limit 25)
  - `cargo-mutants residue check failed` (not a git repository - jj workspace)
- `moon run :agent-cli-contract` → PASS (Tasks: 1 completed)

**Result:** PARTIAL PASS

### PO-012 / API-COMPAT-001 (Semver)

**Command:** Not executed in isolated workspace (requires git baseline comparison)

**Result:** NOT_EXECUTED

## Classification Summary

| Classification | Count | Obligations |
|---|---|---|
| PASS | 9 | PO-001, PO-002, PO-003, PO-004, PO-007 gauntlet, PO-007 LETHAL-1, PO-011 (2 of 3) |
| FAIL_LOCAL | 2 | PO-007 LETHAL-2 (admit_run bypass), PO-011 source-length |
| FAIL_REGRESSION | 0 | |
| WAIVED | 0 | |
| DEFERRED_GLOBAL | 0 | |
| NOT_APPLICABLE | 0 | |
| NOT_EXECUTED | 1 | PO-012 (semver) |

## Required Actions

### DEFECT-12-01 (BLOCKING - BLOCK_LOCAL)

1. **DO NOT CLAIM STATE 11 COMPLETE** - The admit_run bypass is NOT fixed despite user claim
2. Route to **State 10** (Implementation) for `admit_run` fix
3. Fix requires changing `admit_run` signature to use `AcceptedArtifactStore` for strict/journaled policies
4. After fix, re-run `strict_legacy_presence_only_bypass_rejects_required_blocker` Kani harness - must PASS
5. Then re-run State 11 formal verification

### PO-011 source-length (FAIL_LOCAL)

1. Reduce `crates/vb_runtime/src/error/equality.rs:91` from 28 to ≤25 logical lines
2. Re-run `moon run :source-length`

### PO-012 semver (NOT_EXECUTED)

1. Requires git repository context (isolated jj workspace lacks git baseline)
2. Can only be executed when merged to source checkout with git history

## Completion Evidence

```
State 11 Formal Verification for vb-core-cli-accepted-path

VERDICT: INCOMPLETE

Proof Obligations:
- PO-001 (TLA+): PASS
- PO-002 (Verus digest): PASS
- PO-003 (Verus policy): PASS
- PO-004 (Verus admission): PASS
- PO-007 gauntlet: PASS
- PO-007 LETHAL-1 (digest): PASS (State 10 fix confirmed)
- PO-007 LETHAL-2 (admit_run): FAIL (DEFECT-12-01 STILL OPEN)
- PO-011 (static): PARTIAL (lint-src PASS, source-length FAIL, agent-cli-contract PASS)
- PO-012 (semver): NOT_EXECUTED

CRITICAL: User claimed DEFECT-12-01 is FIXED, but Kani harness proves
admit_run bypass still exists. admit_run uses &dyn ArtifactStore
(presence-only) not &dyn AcceptedArtifactStore (full validation).
State 10 implementation must fix this before State 11 can complete.
```

---

STATUS: STATE_11_INCOMPLETE

**Next gate:** State 10 must fix `admit_run` to use `AcceptedArtifactStore` for strict/journaled policies, then State 11 formal verification must rerun to confirm PO-007 LETHAL-2 PASS.
