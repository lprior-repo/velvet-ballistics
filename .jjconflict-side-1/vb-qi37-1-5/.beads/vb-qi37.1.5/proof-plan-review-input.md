# Proof Plan Review Input — vb-qi37.1.5

## Bead Overview

- **Bead**: vb-qi37.1.5 — runtime/recovery: Prove replay digest mismatch detection
- **Goal**: Prove that the recovery system deterministically detects digest mismatches during replay
- **Scope**: vb_storage recovery module — `check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests`, `reject_workflow_digest_mismatch`, and 4 corruption injection tests

## Contract Summary

The contract establishes:
- **INV-001**: `WorkflowDigest` is byte-exact `[u8; 32]` equality
- **POST-001**: `check_workflow_source_digest` returns `Ok(())` iff journal `RunAccepted.workflow == expected`
- **POST-002**: `check_compiled_ir_digest` returns `Ok(())` iff `expected == found`
- **POST-003**: `verify_digests` enforces priority order (workflow source before IR)
- **POST-004**: `reject_workflow_digest_mismatch` detects digest mismatch in events
- **POST-005**: Corruption injection tests produce exact error variants
- **INV-004**: `UnsupportedRecoveryState` flags are monotonic

## Verifier Availability (from discovery)

| Verifier | Available | Notes |
|---|---|---|
| Kani | YES (0.67.0) | Used by `moon run :verify-proof` |
| Verus | NO | Placeholder crate, not installed |
| Miri | YES (0.1.0) | Not applicable — no unsafe code |
| TLA+ | N/A | Non-applicability confirmed — pure function, no temporal model |
| cargo test | YES | For integration tests |

## Proof Obligations

| ID | Clause | Target | Verifier | Mode |
|---|---|---|---|---|
| PO-001 | INV-001 | WorkflowDigest byte equality | kani | verify-proof |
| PO-002 | POST-001 | check_workflow_source_digest | kani | verify-proof |
| PO-003 | POST-002 | check_compiled_ir_digest | kani | verify-proof |
| PO-004 | POST-003 | verify_digests level priority | kani | verify-proof |
| PO-005 | POST-004 | reject_workflow_digest_mismatch | kani | verify-proof |
| PO-006 | INV-004 | UnsupportedRecoveryState monotonicity | unit test | verify-standard |
| PO-007 | ERR-MAP-001 | RecoveryError exhaustive | kani | verify-deep |
| PO-008 | POST-005 | corrupt_artifact_digest test | cargo test | verify-standard |
| PO-009 | POST-005 | corrupt_journal_sequence test | cargo test | verify-standard |
| PO-010 | POST-005 | corrupt_slot_value test | cargo test | verify-standard |
| PO-011 | POST-005 | corrupt_slot_taint test | cargo test | verify-standard |

## Key Review Questions for proof-reviewer

1. Is Kani an acceptable substitute for Verus given the environment constraint? The pure functions are fully addressable by Kani harnesses.
2. Is the TLA+ non-applicability rationale sound? The digest comparison is a pure function with no state machine.
3. Are the 4 integration test obligations sufficient to cover POST-005?
4. Is PO-007 (error exhaustive) correctly classified as `verify-deep` or should it be `verify-proof`?
5. Are there any missing proof obligations for the slot corruption detection paths?
