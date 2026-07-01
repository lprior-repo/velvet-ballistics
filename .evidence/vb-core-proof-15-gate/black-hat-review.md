# Black-Hat Review: vb-core-proof-15-gate

**STATUS: STALE — RAW KANI EVIDENCE WITHDRAWN (vb-5kow2)**

## Gap Summary
`crates/vb_storage/src/admission.rs` line 86-99: `VerificationProof::new()` historically set all proof flags (`bounded`, `taint_safe`, `retry_safe`, `replayable`) to `true` unconditionally. The previously-cited Kani harness file (`crates/vb_storage/src/kani_proof_flags_gap.rs`) is not present in the current tree, so this review is flagged stale rather than authoritative.

## Adversarial Review

### Finding 1: Unconditional Proof Flag Assignment (CRITICAL, hypothesis)
**Severity**: Critical (unverified)
**Location**: `crates/vb_storage/src/admission.rs:86-99` (re-verify line numbers; subject to drift)

**Problem**: `VerificationProof::new()` may set all proof flags to `true` without any per-gate validation. The original review quoted the following shape:
```rust
pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
    Self {
        bounded: true,
        taint_safe: true,
        retry_safe: true,
        replayable: true,
        ...
    }
}
```

**Impact (hypothesis)**: A workflow with secret taint propagation, non-idempotent actions, or unbounded resource usage would still produce proof with all flags=true and could pass runtime admission.

**Attack Vector (hypothesis)**: Submit a workflow with:
1. Actions that propagate secret taint → claim `taint_safe=true`
2. Non-idempotent actions → claim `retry_safe=true`
3. Unbounded resource usage → claim `bounded=true`
4. Non-replayable semantics → claim `replayable=true`

**Kani Proof**: WITHDRAWN. The companion harness file is missing and no raw log is attached. See `formal-verification-report.md` (downgraded rows) for the current status.

### Finding 2: Gate Count Mismatch with Runtime (HIGH, hypothesis)
**Severity**: High (unverified)
**Location**: `crates/vb_storage/src/admission.rs:119` vs `crates/vb_runtime/src/admission.rs:16` (re-verify; subject to drift)

**Problem**: Storage historically emitted `ADMISSION_GATE_COUNT = 15` but no actual 15 gates run. Runtime validates `gate_count == 15` but doesn't verify the gates actually performed validation. **This remains a hypothesis pending a re-run with a real harness and log.**

### Finding 3: No Per-Gate Validation Chain (HIGH, hypothesis)
**Severity**: High (unverified)
**Location**: `crates/vb_storage/src/admission.rs:172-223` (re-verify; subject to drift)

**Problem (hypothesis)**: `submit_artifact_with_contracts` for Journaled/Strict policies may perform only structure validation (try_from_parts) and checksum validation (BLAKE3 hash), then claim 15 gates passed without running gates 3-15.

**Evidence (withdrawn)**: `kani_idempotency_gates.rs` was cited as containing `verify_idempotency` and `validate_idempotency_key_ingredients` but the cited harness is not present in the current tree.

### Finding 4: Relaxed Policy Skips All Gates (INFO)
**Severity**: Info
**Location**: `crates/vb_storage/src/admission.rs:150-171` (re-verify; subject to drift)

Relaxed policy explicitly sets `gate_count=0` and `durable=false`, which is correct behavior for testing-only mode.

## Risk Assessment

| Risk | Severity | Likelihood | Impact |
|------|----------|------------|--------|
| Malicious artifact with unsafe actions passes admission | Critical | Low | Runtime executes unsafe actions |
| Secret taint propagates due to missing validation | High | Medium | Data exfiltration |
| Resource exhaustion due to missing bounded check | High | Medium | Denial of service |
| Non-idempotent action retried causing corruption | High | Low | Data corruption |

All rows are hypotheses until raw Kani evidence is regenerated.

## Recommendations

1. **Immediate**: Reintroduce a real `kani_proof_flags_gap.rs` with `Arbitrary` inputs (no hardcoded shapes), bind it to production via `#[path = ".../crates/..."]`, and capture a raw log with command + exit status.
2. **Short-term**: Implement per-gate validation for at least `taint_safe` and `retry_safe`.
3. **Long-term**: Implement all 15 gates with actual verification before setting proof flags.

## Conclusion

This review is preserved as historical context but is no longer admissible as proof. Any follow-up must regenerate the missing Kani harness, attach a raw log, and re-stamp the companion `formal-verification-report.md` together with this file.
