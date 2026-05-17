# Proof Strategy — vb-qi37.1.5

## Discovery Evidence

From isolated workspace `/home/lewis/src/vb-qi37-1-5`:
- `moon run :verify-proof` → `cargo kani` (Kani 0.67.0 confirmed)
- Verus: **NOT INSTALLED** — the `verus` crate in the environment is a placeholder ("try Verus in your browser or install locally")
- Miri: 0.1.0 installed — NOT APPLICABLE (all recovery files use `#![forbid(unsafe_code)]`)
- No unsafe code in any recovery source file (confirmed by grep)
- No TLA+/temporal behavior in scope (confirmed in contract.md)

## Risk Classification

| Risk | Trigger | Verifier | Mode |
|---|---|---|---|
| Digest byte-equality correctness | Rust-local pure invariant, critical | Kani | verify-proof |
| Workflow digest mismatch detection | Rust-local postcondition, critical | Kani | verify-proof |
| IR digest mismatch detection | Rust-local postcondition, critical | Kani | verify-proof |
| verify_digests level priority | Rust-local postcondition, critical | Kani | verify-proof |
| UnsupportedRecoveryState monotonicity | Rust-local invariant, high | unit tests | verify-standard |
| Error exhaustive mapping | Bounded model, high | Kani | verify-deep |
| Corruption injection tests | Integration test, critical | cargo test | verify-standard |

## Verifier Selection Rationale

**Verus NOT AVAILABLE**: The environment's `verus` crate is a placeholder. The proof obligation cannot be discharged by Verus in this environment.

**Kani available and used by `moon run :verify-proof`**: Kani is the only available formal verifier for Rust-local pure logic in this environment. It will cover:
- `check_workflow_source_digest` — bounded over all `WorkflowDigest` values and journal event sequences
- `check_compiled_ir_digest` — bounded over all `WorkflowDigest` pairs
- `verify_digests` — bounded over all `DigestCheck` levels and digest combinations
- `reject_workflow_digest_mismatch` — bounded over event sequences

**Integration tests for corruption**: The 4 missing corruption injection tests require actual journal state manipulation which is outside Kani's scope. They will be covered by `cargo test --test recovery_integration`.

**Waiver**: Verus obligations from contract.md are waived in favor of Kani because Verus is not installed. Compensating evidence: Kani provides bounded proof for the same pure functions. This is an environment constraint, not a proof adequacy issue.

## Proof Lane Mapping

| Proof Obligation ID | Contract Clause | Verifier | Artifact | Mode |
|---|---|---|---|---|
| PO-001 | INV-001 | kani | `crates/vb_storage/src/recovery/recover.rs` | verify-proof |
| PO-002 | POST-001 | kani | `crates/vb_storage/src/recovery/recover.rs` | verify-proof |
| PO-003 | POST-002 | kani | `crates/vb_storage/src/recovery/recover.rs` | verify-proof |
| PO-004 | POST-003 | kani | `crates/vb_storage/src/recovery/recover.rs` | verify-proof |
| PO-005 | POST-004 | kani | `crates/vb_storage/src/recovery/replay/summary.rs` | verify-proof |
| PO-006 | INV-004 | unit test | `crates/vb_storage/src/recovery/types.rs` | verify-standard |
| PO-007 | ERR-MAP-001 | kani | `crates/vb_storage/src/recovery/types.rs` | verify-deep |
| PO-008 | POST-005 (corrupt_artifact) | cargo test | `crates/vb_storage/tests/recovery_integration.rs` | verify-standard |
| PO-009 | POST-005 (corrupt_journal_seq) | cargo test | `crates/vb_storage/tests/recovery_integration.rs` | verify-standard |
| PO-010 | POST-005 (corrupt_slot_value) | cargo test | `crates/vb_storage/tests/recovery_integration.rs` | verify-standard |
| PO-011 | POST-005 (corrupt_slot_taint) | cargo test | `crates/vb_storage/tests/recovery_integration.rs` | verify-standard |

## Execution Plan

1. Write Kani harnesses for each pure recovery function
2. Run `moon run :verify-proof` for proof lane
3. Write the 4 corruption injection integration tests
4. Run `cargo test --test recovery_integration` for integration test lane
5. Formal verifier (State 11) collects all results into `verification-ledger.jsonl`

## Assumptions

- Kani can stub `FjallJournal` I/O by providing a harness that directly calls the pure functions with in-memory event slices
- The corruption injection tests can be written against the existing `FjallJournal` test infrastructure in `recovery_integration.rs`
- No TLA+ model is needed (temporal non-applicability confirmed in tla-spec.md)
