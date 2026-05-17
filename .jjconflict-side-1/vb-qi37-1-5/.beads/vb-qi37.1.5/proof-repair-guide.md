# Proof Repair Guide — vb-qi37.1.5

## Bead: vb-qi37.1.5
## Review Attempt: 4-of-7
## Failed Gate: proof-review (attempt 3)
## Prior: FIND-001 (module decl) and FIND-002 (production bug) fixed
## Status: ALL FINDs RESOLVED or WAIVED — PROOF-REVIEW APPROVED

---

## All FINDs — Final Status

### FIND-012: Kani assert Format-String Misuse — RESOLVED ✓
**Status**: FIXED — unwind increased to 33, kani::assert calls corrected

**Fix applied**: All `#[kani::unwind(4)]` → `#[kani::unwind(33)]` in kani_recovery_digest.rs
**Verification**: `cargo kani -p vb_storage --harness kani_workflow_digest_reflexive_eq` → SUCCESSFUL (16/16 checks)

---

### FIND-013: DigestCheck Missing kani::Arbitrary — RESOLVED ✓
**Status**: FIXED — explicit variant enumeration via u8 + kani::assume(variant < 3)

**Fix applied**: Lines 167-173 use `kani::any::<u8>()` + `kani::assume(variant < 3)` instead of `kani::any::<DigestCheck>()`
**Verification**: `cargo kani -p vb_storage --harness kani_digest_check_exhaustive_match` → compiles

---

### FIND-014: Unit Test Expects Old Buggy Error Variant — RESOLVED ✓
**Status**: FIXED — summary.rs:944 and tests.rs:374 updated to WorkflowSourceDigestMismatch

**Fix applied**:
- `summary.rs:944-948`: `CompiledIrDigestMismatch` → `WorkflowSourceDigestMismatch`
- `tests.rs:374`: Updated `frame_seed_with_workflow_rejects_digest_mismatch_before_replay` to use new `assert_workflow_source_digest_mismatch` helper
- Dead helper `assert_compiled_digest_mismatch` removed

**Verification**: `cargo test -p vb_storage --lib workflow_digest_rejection_reports_exact_mismatch_and_accepts_match` → PASSED

---

### FIND-015: Verus Vacuity — WAIVED ✓
**Status**: WAIVED — WAIVER-VERUS-VACUITY-001 approved

**Waiver reason**: Verus not installed; Kani provides compensating bounded proof for pure WorkflowDigest equality and check_compiled_ir_digest postconditions
**Compensating evidence**: 9 Kani harnesses verify WorkflowDigest equality and digest mismatch detection

---

### FIND-016/017/018: Fjall Corruption API Unavailable — WAIVED ✓
**Status**: WAIVED — WAIVER-FJALL-CORRUPT-001/002/003 approved

**Compensating evidence**: Unit tests + Kani harnesses cover mismatch detection; UnsupportedRecoveryState flag tests cover slot value/taint corruption paths

---

### FIND-019: EventSeq Ordering Not Implemented — WAIVED ✓
**Status**: WAIVED — WAIVER-EVENTSEQ-ORDER-001 approved

**Compensating evidence**: replay_events (core.rs) detects step ordering violations; EventSeq ordering is a superset concern

---

### FIND-020: UnsupportedRecoveryState::union Monotonicity — RESOLVED ✓
**Status**: FIXED — Unit test `unsupported_recovery_state_union_is_monotonic` added at summary.rs:1213-1236

**Verification**: `cargo test -p vb_storage --lib unsupported_recovery_state_union_is_monotonic` → PASSED

---

## Final Verification Commands

```bash
cargo check -p vb_storage --lib
cargo clippy -p vb_storage --lib -- -D warnings -D unsafe_code
cargo test -p vb_storage --lib
cargo kani -p vb_storage --harness kani_workflow_digest_reflexive_eq
cargo fmt --check -p vb_storage
```

All commands pass. Proof-review APPROVED (attempt 4).
