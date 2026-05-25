# Proof Strategy - vb-fq6u

## Bead
**ID**: vb-fq6u
**Title**: P0: restore global moon ci green after idempotency package
**Scope**: clippy arithmetic_side_effects fix in vb_core/budget.rs, fmt drift

## Issue Summary
`SmallLinearMetrics::add` (crates/vb_core/src/budget.rs:315-321) uses bare `+` on `u64` and `u32`, triggering `clippy::arithmetic_side_effects`. The fix changes overflow semantics from **wrap** (u64::MAX + 1 = 0) to **saturate** (u64::MAX + 1 = u64::MAX). This is a **behavior-affecting** change that must be verified.

**Contract**: See `contract.md` for exact pre/post state and required fix.

## Risk Classification
| Risk | Assessment |
|------|------------|
| Arithmetic/overflow | **HIGH** - behavior-affecting semantic change: wrap → saturate (ps-002, ps-005) |
| Bounded state | **HIGH** - metrics used in budget admission checks |
| Lint gate | **MEDIUM** - clippy must pass for moon ci |
| Determinism | **MEDIUM** - pure fn used in budget computation |
| Concurrency | **LOW** - no concurrency surface in linear metrics |
| Unsafe/UB | **LOW** - no unsafe code; pure arithmetic |
| Supply chain | **LOW** - local vb_core change only |

## Proof Seeds (from rust-contract)

| ID | Requirement | Domain Claim | Behavior Affecting |
|----|-------------|--------------|-------------------|
| vb-fq6u-ps-001 | vb-fq6u-req-001 | Correct sums under non-overflow conditions | No |
| vb-fq6u-ps-002 | vb-fq6u-req-001 | Saturating under overflow conditions | **Yes** |
| vb-fq6u-ps-003 | vb-fq6u-req-002 | lint-src gate passes | No |
| vb-fq6u-ps-004 | vb-fq6u-req-003 | Budget accumulation is deterministic | No |
| vb-fq6u-ps-005 | vb-fq6u-req-004 | Overflow cannot wrap to zero | **Yes** |

## Verifier Selection

| Risk | Verifier | Rationale |
|------|----------|-----------|
| Overflow saturation semantics | Kani | Bounded model checking proves saturating_add correctness |
| Rust-local pure invariants | Verus | Functional correctness of SmallLinearMetrics |
| Lint gate | moon ci | Required for moon ci; clippy + fmt |
| Determinism | proptest | Property-based testing for pure function determinism |

## Proof Obligations

### PO-001: Overflow Saturation Correctness (Kani)
- **Requirement**: vb-fq6u-req-001 (ps-002, ps-005)
- **Contract Clause**: arithmetic_overflow_semantics
- **Verifier**: Kani
- **Artifact**: crates/vb_core/src/budget.rs
- **Command**: `cargo kani --harness small_linear_metrics_overflow --no-unwind`
- **Expected Evidence**: Kani reports no witness; all paths with MAX inputs saturate at MAX instead of wrapping
- **Assumptions**: Metrics bounded by workflow node count; node count <= 1_000_000
- **Mode**: verify-deep
- **Required**: true
- **Behavior Affecting**: true

### PO-002: Pure Function Determinism (proptest)
- **Requirement**: vb-fq6u-req-003 (ps-004)
- **Contract Clause**: pure_determinism
- **Verifier**: proptest
- **Artifact**: crates/vb_core/src/budget.rs
- **Command**: `cargo test --package vb_core --lib budget::small_linear --no-fail-fast`
- **Expected Evidence**: All property tests pass; same inputs produce same outputs
- **Assumptions**: Deterministic input node list
- **Mode**: verify-property
- **Required**: true
- **Behavior Affecting**: false

### PO-003: Lint Gate Verification (clippy)
- **Requirement**: vb-fq6u-req-002 (ps-003)
- **Contract Clause**: clippy_arithmetic_side_effects
- **Verifier**: clippy / moon ci
- **Artifact**: crates/vb_core/src/budget.rs
- **Command**: `moon run :lint-src`
- **Expected Evidence**: moon lint-src exits 0
- **Assumptions**: Nightly toolchain
- **Mode**: verify-lint
- **Required**: true
- **Behavior Affecting**: false

### PO-004: Format Gate Verification (fmt)
- **Requirement**: vb-fq6u-req-002 (ps-003)
- **Contract Clause**: fmt_drift
- **Verifier**: cargo-fmt / moon ci
- **Artifact**: crates/vb_core/src/budget.rs
- **Command**: `moon run :fmt`
- **Expected Evidence**: moon fmt exits 0 (no formatting drift)
- **Assumptions**: None
- **Mode**: verify-fmt
- **Required**: true
- **Behavior Affecting**: false

### PO-005: Formal Proof of Saturating Semantics (Verus)
- **Requirement**: vb-fq6u-req-001 (ps-001, ps-002, ps-005)
- **Contract Clause**: arithmetic_correctness, overflow_semantics, overflow_safety
- **Verifier**: Verus
- **Artifact**: crates/vb_core/src/budget.rs
- **Command**: `verus crates/vb_core/src/budget.rs`
- **Expected Evidence**: Verus verified with 0 errors
- **Assumptions**: Verus spec fns correctly bound to exec fns
- **Mode**: verify-proof
- **Required**: true
- **Behavior Affecting**: true

### PO-006: moon ci Global Gate
- **Requirement**: vb-fq6u-req-002
- **Contract Clause**: global-readiness
- **Verifier**: moon-ci
- **Artifact**: workspace
- **Command**: `moon ci`
- **Expected Evidence**: All moon CI tasks pass
- **Assumptions**: All above obligations must pass first
- **Mode**: verify-ci
- **Required**: true
- **Behavior Affecting**: false

## Waiver Requests
None - all obligations have appropriate verifiers and behavior-affecting overflow is proven via Kani + Verus.

## Strategy Summary
- **Primary**: Kani for bounded overflow saturation model checking (behavior-affecting)
- **Secondary**: Verus auxiliary invariants for pure functional correctness
- **Tertiary**: proptest for deterministic pure function properties
- **Gates**: moon ci (lint-src + fmt) required for green
- **No TLA+ needed**: Local arithmetic/data structure, no temporal behavior
- **No Loom needed**: No concurrency surface
- **No Miri needed**: No unsafe code; pure arithmetic
- **No Flux needed**: No refinement type annotations present
- **No cargo-fuzz needed**: Not a byte-parser or external input surface