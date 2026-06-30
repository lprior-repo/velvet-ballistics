# Black-Hat Review: vb-core-proof-15-gate

## Gap Summary
`crates/vb_storage/src/admission.rs` line 86-99: `VerificationProof::new()` sets all proof flags (`bounded`, `taint_safe`, `retry_safe`, `replayable`) to `true` unconditionally. No per-gate validation occurs.

## Adversarial Review

### Finding 1: Unconditional Proof Flag Assignment (CRITICAL)
**Severity**: Critical
**Location**: `crates/vb_storage/src/admission.rs:86-99`

**Problem**: `VerificationProof::new()` sets all proof flags to `true` without any validation:
```rust
pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
    Self {
        bounded: true,       // ALWAYS true!
        taint_safe: true,   // ALWAYS true!
        retry_safe: true,    // ALWAYS true!
        replayable: true,    // ALWAYS true!
        ...
    }
}
```

**Impact**: Any CompiledWorkflow (valid or malicious) produces proof with all flags=true. An artifact with secret taint propagation, non-idempotent actions, or unbounded resource usage will pass runtime admission because the proof flags claim it's safe.

**Attack Vector**: Submit a workflow with:
1. Actions that propagate secret taint → claim `taint_safe=true`
2. Non-idempotent actions → claim `retry_safe=true`
3. Unbounded resource usage → claim `bounded=true`
4. Non-replayable semantics → claim `replayable=true`

Runtime `load_accepted_artifact()` validates only that flags are true, not that the workflow actually satisfies the safety properties.

**Kani Proof**: `kani_proof_flags_gap.rs` VB-STORAGE-GAP-001 through VB-STORAGE-GAP-006 prove flags are always true.

### Finding 2: Gate Count Mismatch with Runtime (HIGH)
**Severity**: High
**Location**: `crates/vb_storage/src/admission.rs:119` vs `crates/vb_runtime/src/admission.rs:16`

**Problem**: Storage emits `ADMISSION_GATE_COUNT = 15` but no actual 15 gates run. Runtime validates `gate_count == 15` but doesn't verify the gates actually performed validation.

### Finding 3: No Per-Gate Validation Chain (HIGH)
**Severity**: High
**Location**: `crates/vb_storage/src/admission.rs:172-223`

**Problem**: The `submit_artifact_with_contracts` function for Journaled/Strict policies performs:
1. Structure validation (try_from_parts)
2. Checksum validation (BLAKE3 hash)

But then claims 15 gates passed without running gates 3-15.

**Evidence**: `kani_idempotency_gates.rs` shows verification functions exist (`verify_idempotency`, `validate_idempotency_key_ingredients`) but they're not called from `submit_artifact`.

### Finding 4: Relaxed Policy Skips All Gates (INFO)
**Severity**: Info
**Location**: `crates/vb_storage/src/admission.rs:150-171`

Relaxed policy explicitly sets `gate_count=0` and `durable=false`, which is correct behavior for testing-only mode.

## Risk Assessment

| Risk | Severity | Likelihood | Impact |
|------|----------|------------|--------|
| Malicious artifact with unsafe actions passes admission | Critical | Low | Runtime executes unsafe actions |
| Secret taint propagates due to missing validation | High | Medium | Data exfiltration |
| Resource exhaustion due to missing bounded check | High | Medium | Denial of service |
| Non-idempotent action retried causing corruption | High | Low | Data corruption |

## Recommendations

1. **Immediate**: Document the gap in code comments and require explicit opt-in for production use
2. **Short-term**: Implement per-gate validation for at least `taint_safe` and `retry_safe`
3. **Long-term**: Implement all 15 gates with actual verification before setting proof flags

## Conclusion

The gap is real and provable via Kani. The proof flags claim safety properties that are not validated. Production use of Journaled/Strict policy with this implementation is NOT safe without implementing per-gate validation.
