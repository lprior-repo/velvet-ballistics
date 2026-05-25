# Proof Coverage Matrix - vb-fq6u

## Coverage Analysis

### vb-fq6u-ps-001: Non-overflow sum correctness

| Verifier | Coverage | Gap |
|----------|-----------|-----|
| Kani | Full bounded model checking; proves no overflow witness | None |
| Verus | Formal proof of add algebraic properties | None |
| proptest | Property-based testing across randomized inputs | None |

**Overall**: FULL coverage. Primary defense-in-depth via Kani.

### vb-fq6u-ps-002: Overflow saturation semantics (behavior-affecting)

| Verifier | Coverage | Gap |
|----------|-----------|-----|
| Kani | Bounded model checking proves saturate semantics | None |
| Verus | Formal spec fn proves saturating_add semantics | None |
| proptest | Property tests verify saturation algebra | None |

**Overall**: FULL coverage. Behavior-affecting claim verified by three independent lanes.

### vb-fq6u-ps-003: Lint gate verification

| Verifier | Coverage | Gap |
|----------|-----------|-----|
| moon-ci (lint-src) | Canonical lint gate; :lint-src task must exit 0 | None |
| moon-ci (fmt) | Format check; :fmt task must exit 0 | None |

**Overall**: FULL coverage. Required for moon ci green.

### vb-fq6u-ps-004: Determinism of pure functions

| Verifier | Coverage | Gap |
|----------|-----------|-----|
| Verus | Formal proof: same inputs → same outputs | None |
| proptest | Property tests: multiple calls produce identical results | None |

**Overall**: FULL coverage. Kani is not needed as proptest+Verus cover this.

### vb-fq6u-ps-005: Overflow cannot wrap to zero (behavior-affecting)

| Verifier | Coverage | Gap |
|----------|-----------|-----|
| Kani | Bounded model checking proves wrap-to-zero cannot occur | None |
| Verus | Formal safety proof: add(a,b) != 0 for a,b > 0 | None |
| proptest | Property tests verify no-wrap-to-zero for positive inputs | None |

**Overall**: FULL coverage. Critical safety property verified by three independent lanes.

## Coverage Summary

| Requirement | Verifiers | Status |
|-------------|-----------|--------|
| req-001 sum-correctness | Kani, Verus, proptest | **COVERED** |
| req-001 overflow-saturation (BA) | Kani, Verus, proptest | **COVERED** |
| req-002 lint-gate | moon-ci | **COVERED** |
| req-003 determinism | Verus, proptest | **COVERED** |
| req-004 no-wrap-to-zero (BA) | Kani, Verus, proptest | **COVERED** |
| global-readiness | moon-ci | **COVERED** |

**BA** = behavior-affecting. All BA claims have formal proof (Verus) + bounded model checking (Kani) + property testing (proptest).

## Gaps Identified

None. All requirements have appropriate verifier coverage.

## Residual Risk

- **Rust toolchain soundness**: Kani/Verus rely on rustc correctness. Acceptable as these are industry-standard formal verification tools.
- **Formal proof scope**: Verus spec fns are auxiliary until proof-reviewer accepts scope. Kani is the primary lane.
- **Behavioral change acceptance**: The wrap→saturate semantic change is intentional and verified. Budget computation will clamp at MAX instead of wrapping to zero, which is the desired behavior.
- **Lint gate**: clippy::arithmetic_side_effects is the canonical gate; no formal proof needed for mechanical lint fix.