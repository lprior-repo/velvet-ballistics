# Proof Writer Report — vb-8mdp.7 Invocation Supersession

**State**: 5 | **Attempt**: proof-write-1 | **Date**: 2026-05-30

## Lane Results

| Lane | Tool | Exit | Result | Evidence |
|------|------|------|--------|----------|
| Spec/Model | Verus | 0 | **6 verified, 0 errors** | evidence/verus-collect-lowering.log |
| Property | Proptest (vb_core) | 0 | **16 passed** | evidence/cargo-test-vb-core-budget.log |
| Property | Proptest (vb_runtime) | 0 | **22 passed** | evidence/cargo-test-vb-runtime-admission.log |
| Property | Proptest (vb_compile) | 0 | **15 passed** | evidence/cargo-test-vb-compile-collect-lowering.log |
| Integration | Cargo test (workspace) | 0 | **21 passed** | evidence/cargo-test-workspace-resource-admission.log |
| Bounded MC | Kani | 101 | **BLOCKED_TOOLING** (45 pre-existing errors) | evidence/kani-blocked-compilation.log |
| Refinement | Flux | 0 | **BLOCKED_TOOLING** (package smoke only) | evidence/flux-package-smoke.log |
| Temporal | TLA+ | — | **DROPPED** (controller directive) | evidence/tlc-collect-body-model.log |

**Total behavior tests: 74 passing** across 4 crates.

## Verus Detail

- File: `verification/verus/collect_lowering.rs` (173 lines)
- 6 proof functions, all non-tautological
- L1: Step offset strict monotonicity (body < page < done)
- L2: Emission count bounds without overflow
- L3: Consecutive ID spacing
- L4: Max valid start ID = u16::MAX - 3
- L5: Option matching safety by cases
- L6: Composite chain lemma combining L1+L2+L3
- **GOD RULE 2 binding gap**: Standalone model. Production exec fn `lower_canonical_collect` at `part_03.rs:169-227` lacks `requires`/`ensures`.

## GOD RULE Compliance

| Rule | Status |
|------|--------|
| 1 (No hardcoded Kani shapes) | BLOCKED — Kani cannot compile |
| 2 (No vacuum Verus) | GAPPED — standalone model, non-tautological but not production-bound |
| 3 (Bounded TLA+) | DROPPED per controller directive |
| 4 (No loop oscillations) | PASS |
| 5 (No blind mutations) | PASS |

## Blockers

1. **Kani**: 45 pre-existing `unused Result` errors in crate-wired Kani harnesses
2. **Flux**: No Flux artifacts for vb-8mdp.7 obligations
3. **GOD RULE 2**: Production binding gap
