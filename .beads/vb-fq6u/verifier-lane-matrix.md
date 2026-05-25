# Verifier Lane Matrix - vb-fq6u

Maps each (requirement_id, proof_seed_id) tuple to applicable verifier lanes.

## Proof Seeds

| Proof Seed | Requirement | Description | Behavior Affecting |
|------------|-------------|-------------|-------------------|
| vb-fq6u-ps-001 | vb-fq6u-req-001 | Correct sums under non-overflow conditions | No |
| vb-fq6u-ps-002 | vb-fq6u-req-001 | Saturating under overflow conditions | **Yes** |
| vb-fq6u-ps-003 | vb-fq6u-req-002 | lint-src gate passes | No |
| vb-fq6u-ps-004 | vb-fq6u-req-003 | Budget accumulation is deterministic | No |
| vb-fq6u-ps-005 | vb-fq6u-req-004 | Overflow cannot wrap to zero | **Yes** |

## Lane Coverage

| Proof Seed | tla-plus | verus | kani | flux-rs | loom | miri | proptest | cargo-fuzz | moon-ci |
|-----------|----------|-------|------|---------|------|------|----------|------------|---------|
| ps-001 (sum-correctness) | N/A | **REQD** | **REQD** | N/A | N/A | N/A | **REQD** | N/A | - |
| ps-002 (overflow-saturation) | N/A | **REQD** | **REQD** | N/A | N/A | N/A | **REQD** | N/A | - |
| ps-003 (lint-gate) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | **REQD** |
| ps-004 (determinism) | N/A | **REQD** | N/A | N/A | N/A | N/A | **REQD** | N/A | - |
| ps-005 (no-wrap-to-zero) | N/A | **REQD** | **REQD** | N/A | N/A | N/A | **REQD** | N/A | - |

**Legend**: REQD = required applicable lane, N/A = not applicable (cited with evidence), - = not applicable (not a verifier for this type)

## Behavior-Affecting Coverage

| Proof Seed | Verifier | Obligation | Coverage |
|-----------|----------|------------|----------|
| ps-002 (overflow-saturation) | Kani | PO-001 | Primary bounded model checking |
| ps-002 (overflow-saturation) | Verus | PO-005 | Formal proof of saturating semantics |
| ps-002 (overflow-saturation) | proptest | PO-002 | Property-based saturation properties |
| ps-005 (no-wrap-to-zero) | Kani | PO-001 | Proves wrap-to-zero cannot occur |
| ps-005 (no-wrap-to-zero) | Verus | PO-005 | Formal safety proof |
| ps-005 (no-wrap-to-zero) | proptest | PO-002 | Safety property testing |

## Non-Applicable Lanes Summary

| Verifier | Total N/A | Rationale |
|----------|-----------|-----------|
| tla-plus | 5 | Local Rust arithmetic, no temporal protocol |
| flux-rs | 5 | No Flux refinement annotations in vb_core budget |
| loom | 5 | No concurrency surface in pure arithmetic function |
| miri | 5 | Safe Rust; wrap vs saturate is both defined behavior; no UB |
| cargo-fuzz | 5 | Not a byte-parser or external input surface |
| verus | 1 | N/A for lint gate (ps-003) |
| kani | 1 | N/A for lint gate (ps-003) and determinism (ps-004, covered by proptest) |
| proptest | 1 | N/A for lint gate (ps-003) |
| moon-ci | 4 | Only applicable for ps-003 (lint gate) |