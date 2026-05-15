# Implementation Report — vb-core-proof-gate-inputs

**Bead**: vb-core-proof-gate-inputs
**State**: 10 (Holzman-Rust Implementation)
**Workspace**: /tmp/vb-ws/vb-core-proof-gate-inputs
**Date**: 2026-05-15

---

## Contract Compliance Summary

| Contract Clause | Implementation Status | Evidence |
|----------------|----------------------|----------|
| POST-001 (VerificationProof::new defaults) | ✓ IMPLEMENTED | `crates/vb_storage/src/admission.rs:86-96` |
| POST-002 (Relaxed gate_count=0) | ✓ IMPLEMENTED | `crates/vb_storage/src/admission.rs:149-169` |
| POST-003 (Journaled gate_count=2) | ✓ IMPLEMENTED | `crates/vb_storage/src/admission.rs:171-218` |
| POST-004 (Strict gate_count=2, durable=true) | ✓ IMPLEMENTED | `crates/vb_storage/src/admission.rs:171-218` |
| INV-001 (VerificationProof well-formed) | ✓ IMPLEMENTED | Verified by 2445 cargo tests |
| INV-002 (VerificationWarning gate range) | ✓ IMPLEMENTED | `admission.rs:34-36` + boundary tests |
| ERR-001 (ArtifactChecksumMismatch) | ✓ IMPLEMENTED | `admission.rs:182-184` |
| ERR-002 (ArtifactMalformed) | ✓ IMPLEMENTED | `admission.rs:175,180,191,217` |
| Gate 1 (structure validation) | ✓ IMPLEMENTED | `vb_core::CompiledWorkflow::try_from_parts` |
| Gate 2 (checksum validation) | ✓ IMPLEMENTED | `admission.rs:177-184` |

### Proof Flag Defaults (POST-001)

All proof flags default to `true` in `VerificationProof::new`:

```rust
// crates/vb_storage/src/admission.rs:86-96
pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
    Self {
        digest,
        gate_count,
        durable,
        bounded: true,
        taint_safe: true,
        retry_safe: true,
        replayable: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: Vec::new(),
    }
}
```

Action-contract-based flag derivation is out of scope (waived per WAIVER-FLAG-DERIV).

---

## Formal Verification Lane Results

### cargo test (vb_core, vb_storage lib tests)

```
cargo test -p vb_core -p vb_storage --lib
Result: 2445 passed (2 suites, 1.87s)
```

All POST-002/003/004 policy dispatch tests pass:
- Relaxed: `gate_count == 0`, `durable == false` ✓
- Journaled: `gate_count == 2`, `durable == false` ✓
- Strict: `gate_count == 2`, `durable == true` ✓

### cargo check (vb_core, vb_storage)

```
cargo check -p vb_core -p vb_storage
Result: compiled successfully (61 crates)
```

---

## Known Issue: gate_count Mismatch Between Storage and Runtime

### Storage Layer (vb_storage)

```rust
// crates/vb_storage/src/admission.rs:118
const ADMISSION_GATE_COUNT: u8 = 2;
```

- Relaxed → `gate_count = 0` (skips gates)
- Journaled → `gate_count = 2`
- Strict → `gate_count = 2`

### Runtime Layer (vb_runtime)

```rust
// crates/vb_runtime/src/admission.rs:16
pub const REQUIRED_GATE_COUNT: u8 = 15;
```

```rust
// crates/vb_runtime/src/admission.rs:324-329
if artifact.verification.gate_count != REQUIRED_GATE_COUNT {
    return Err(ArtifactEnvelopeError::InvalidGateCount {
        found: artifact.verification.gate_count,
        required: REQUIRED_GATE_COUNT,
    });
}
```

### Impact

Any artifact stored via `vb_storage::submit_artifact` will be rejected by `vb_runtime::load_accepted_artifact` because:
- Storage produces `gate_count ∈ {0, 2}`
- Runtime expects `gate_count == 15`

### Status

**DEFERRED_GLOBAL** — This is an architectural mismatch that requires cross-layer coordination to resolve. It is not resolvable within the scope of vb-core-proof-gate-inputs which only covers the storage admission layer. A future bead should reconcile ADMISSION_GATE_COUNT (2) with REQUIRED_GATE_COUNT (15).

---

## Artifacts Produced

| Artifact | Location | Status |
|----------|----------|--------|
| implementation.md | `.beads/vb-core-proof-gate-inputs/implementation.md` | Written |
| VerificationProof spec | `verification/verus/budget_bounded.rs` | Reviewed |
| Policy dispatch tests | `crates/vb_storage/src/admission.rs:504-573` | PASS |
| BDD tests | `crates/vb_storage/src/admission.rs:754-790` | PASS |
| Proptest | `verification/proptest/vb_core_admission_proptests.rs` | PASS |

---

## Pre-Existing Issues (Not in Scope)

1. **velvet_ballastics CLI missing blake3 dependency** — `crates/velvet_ballastics/src/cli_postcard.rs:153` references `blake3::hash` but blake3 is not in that crate's Cargo.toml. Triggers on `cargo check --workspace`. Not blocking vb_core/vb_storage.

2. **Source checkout lint failure** — `crates/vb_core/src/budget/tests.rs:3574` unused import `ResourceContract`. Pre-existing lint debt in tests.

3. **gate_count mismatch** — described above as DEFERRED_GLOBAL.

---

*Implementation complete for vb-core-proof-gate-inputs — State 10*
