# Contract Verification Review — vb-qi37.1.5

STATUS: REJECTED

## Files Reviewed
- contract.md
- tla-spec.md
- lean-contract.md
- verification-layers.md
- proof-obligations.jsonl
- traceability-matrix.jsonl

## Command Evidence
```bash
jq -c . proof-obligations.jsonl >/dev/null  # VALID JSONL
jq -c . traceability-matrix.jsonl >/dev/null  # VALID JSONL
```

## Findings

### Severity: LETHAL — Module Declaration Blocker

**Clause**: PO-001, PO-002, PO-003, PO-004, PO-005, PO-007
**Problem**: The Kani harness file `crates/vb_storage/src/kani_recovery_digest.rs` was created but cannot be compiled by `cargo kani` because it is not declared in vb_storage's module tree. The existing `kani_codec.rs` is declared as `pub mod kani_codec;` in lib.rs:35. The new proof file requires `pub mod kani_recovery_digest;` to be added to vb_storage/src/lib.rs.

**Required fix**: Add `pub mod kani_recovery_digest;` to vb_storage/src/lib.rs. This is a 1-line production code change. Route to holzman-rust or require a project maintainer waiver.

**Evidence**: cargo kani -p vb_storage --harness kani_workflow_digest_reflexive_eq → "no harnesses matched the harness filter"

### Severity: LETHAL — Production Bug in PO-005 Target

**Clause**: PO-005 (POST-004)
**Problem**: `crates/vb_storage/src/recovery/replay/summary.rs:190` uses `RecoveryError::CompiledIrDigestMismatch` but POST-004 requires detection of workflow source digest mismatch. This is the wrong error variant for the function's stated purpose.

**Required fix**: Change `RecoveryError::CompiledIrDigestMismatch` to `RecoveryError::WorkflowSourceDigestMismatch` in summary.rs:190. Route to holzman-rust (State 10) for repair.

**Evidence**: summary.rs:182-199 — reject_workflow_digest_mismatch returns CompiledIrDigestMismatch for workflow digest mismatch

### Severity: MAJOR — FjallJournal Dependency Blocks Formal Verification

**Clause**: PO-002 (check_workflow_source_digest), PO-004 (verify_digests)
**Problem**: Both functions require `&FjallJournal` which is a database handle with file I/O and internal state. Kani cannot symbolically execute file I/O. No stub or harness exists for these functions.

**Required fix**: Either (a) create a test-only FjallJournal stub that returns in-memory events for Kani, or (b) accept that these functions can only be verified by unit/integration tests and update proof-obligations.jsonl to change their mode from verify-proof to verify-standard with cargo test coverage.

**Evidence**: proof-writer-report.md — "BLOCKED — FjallJournal required"

### Severity: MAJOR — PO-006 Not Addressed

**Clause**: INV-004 (UnsupportedRecoveryState monotonicity)
**Problem**: No proof artifact or test was created for PO-006 (UnsupportedRecoveryState::union monotonicity).

**Required fix**: Either add a unit test for union() monotonicity or create a Kani harness once module declaration is fixed.

### Severity: MINOR — PO-008 to PO-011 Owned by State 8

**Clause**: POST-005 (4 corruption injection tests)
**Problem**: The 4 integration tests are not yet written — they are correctly owned by State 8 (test-writer).

**Required fix**: None at this stage. Not a blocker for contract approval.

## Coverage Decision

### Contract clauses traced:
- INV-001: VERIFIED — proof-obligations.jsonl includes PO-001 (WorkflowDigest equality)
- INV-002: VERIFIED — proof-obligations.jsonl includes PO-002 (check_workflow_source_digest determinism)
- INV-003: VERIFIED — proof-obligations.jsonl includes PO-007 (RecoveryError exhaustive)
- INV-004: NOT VERIFIED — PO-006 not addressed
- POST-001: VERIFIED — proof-obligations.jsonl includes PO-002 (check_workflow_source_digest postconditions)
- POST-002: VERIFIED — proof-obligations.jsonl includes PO-003 (check_compiled_ir_digest postconditions)
- POST-003: VERIFIED — proof-obligations.jsonl includes PO-004 (verify_digests level priority)
- POST-004: INVALID — PO-005 target has production bug
- POST-005: PENDING — PO-008 to PO-011 owned by State 8

### TLA+-owned clauses covered:
- None — TLA+ non-applicability confirmed in tla-spec.md. Valid.

### Verus-owned clauses:
- Verus is NOT INSTALLED. Waiver in proof-strategy.md maps Verus obligations to Kani. Acceptable given environment constraint.

### Proof obligations traced:
- All 11 obligations have entries in proof-obligations.jsonl with valid schema
- PO-001, PO-003, PO-007: harnesses written but module declaration blocker prevents execution
- PO-002, PO-004: BLOCKED_TOOLING (FjallJournal)
- PO-005: INVALID (production bug)
- PO-006: NOT ADDRESSED
- PO-008 to PO-011: PENDING (State 8 ownership)

### TLA+ scope valid:
- Yes — no temporal behavior in scope; non-applicability rationale is sound

### Verus scope valid:
- N/A — Verus not installed; Kani is acceptable substitute for pure functions

### Lean/Aeneas/Hax scope valid:
- N/A — confirmed non-applicable; Lean would add no value for byte equality proof

### Waivers valid:
- Verus waiver: YES — owner, reason, expiry, compensating evidence all present in proof-strategy.md
- TLA+ waiver: YES — no temporal behavior in scope
- Kani substitution for Verus: YES — compensating evidence is Kani bounded proof for the same pure functions

## Summary

The contract is well-formed with correct error taxonomy, clear pre/post conditions, and sound non-applicability rationale for TLA+ and Lean. However, the proof plan cannot be executed due to:

1. **LETHAL**: Module declaration blocker — kani_recovery_digest.rs needs to be declared in lib.rs
2. **LETHAL**: Production bug — summary.rs uses wrong error variant for reject_workflow_digest_mismatch
3. **MAJOR**: FjallJournal dependency — check_workflow_source_digest and verify_digests cannot be verified by Kani without stubbing
4. **MAJOR**: PO-006 not addressed

**Do not proceed to test planning or implementation until these issues are resolved.**

## Routing

- Module declaration: holzman-rust (State 10) or project maintainer — 1-line lib.rs change
- Production bug fix: holzman-rust (State 10) — summary.rs:190 error variant fix
- FjallJournal stubbing: formal-verifier (State 11) or test-only coverage
- PO-006: proof-writer (State 5 retry) or test-planner (State 7) with unit test
