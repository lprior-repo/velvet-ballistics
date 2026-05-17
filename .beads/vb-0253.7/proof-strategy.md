# Proof Strategy: CLI Lifecycle Event-Applied Tracker (vb-0253.7)

## Strategy Overview

**Bead**: vb-0253.7 — cli: Make lifecycle tracker event-applied
**Phase**: Proof Planning (p4)
**Scope**: Turn contract/risk tags into verifier strategy

## Verification Lane Map

| Lane | Obligations | Risk Profile | Mode |
|------|-------------|-------------|------|
| `verus` | VERUS-DERIVE-001, VERUS-TRANSITION-001 | proof | verify-proof |
| `tla-plus` | TLA-LIFECYCLE-001, TLA-LIFECYCLE-002, TLA-LIFECYCLE-003, POST-*-001 (5×) | proof/medium | verify-proof/verify-standard |
| `kani` | KANI-001, KANI-002 | high | verify-deep |
| `miri` | MIRI-001 | high | verify-deep |
| `static-scan` | STATIC-LINT-001 | high | verify-standard |
| `cargo-test` | TEST-COMPILE-001 | medium | verify-standard |
| `api-compat` | SEMVER-001 | medium | verify-standard |

## Lane Execution Order

```
Phase 1: Formal Proof (verus, tla-plus)
  └─ VERUS-DERIVE-001 → VERUS-TRANSITION-001
  └─ TLA-LIFECYCLE-001 → TLA-LIFECYCLE-002 → TLA-LIFECYCLE-003
  └─ POST-*-001 (can run in parallel with TLA+ once model exists)

Phase 2: Deep Verification (kani, miri)
  └─ KANI-001, KANI-002 (after verus lane passes)
  └─ MIRI-001 (after refactoring complete)

Phase 3: Standard Verification (static-scan, cargo-test, api-compat)
  └─ STATIC-LINT-001 (parallel with Phase 2)
  └─ TEST-COMPILE-001, SEMVER-001 (after refactoring lands)
```

## Risk Stratification

### Proof-Risk Obligations (Primary Gate)
| ID | Layer | Clause | Claim | Mode |
|----|-------|--------|-------|------|
| TLA-LIFECYCLE-001 | tla-plus | INV-001 | Journal-derived state consistency | verify-proof |
| TLA-LIFECYCLE-002 | tla-plus | INV-002 | No divergence between in-memory and journal | verify-proof |
| TLA-LIFECYCLE-003 | tla-plus | INV-005 | Terminal states have no outgoing transitions | verify-proof |
| VERUS-DERIVE-001 | verus | INV-001 | derive_lifecycle_state_from_events is total | verify-proof |
| VERUS-TRANSITION-001 | verus | INV-003 | check_lifecycle_transition correctness | verify-proof |

**Gate**: All proof-risk obligations must pass before Phase 2.

### High-Risk Obligations (Bounded Model Checking)
| ID | Layer | Clause | Claim | Mode |
|----|-------|--------|-------|------|
| KANI-001 | kani | INV-003 | Bounded transition sequences never panic | verify-deep |
| KANI-002 | kani | PRE-001 | Valid RunId passes preconditions | verify-deep |
| MIRI-001 | miri | INV-004 | No undefined behavior after refactoring | verify-deep |
| STATIC-LINT-001 | static-scan | INV-002 | No unsafe/unwrap/panic/todo/dbg | verify-standard |

**Gate**: kani/miri lanes run after verus confirms no panic paths.

### Medium-Risk Obligations (Standard Verification)
| ID | Layer | Clause | Claim | Mode |
|----|-------|--------|-------|------|
| POST-CANCEL-001 | tla-plus | POST-001 | cancel produces Cancelled state | verify-standard |
| POST-RESUME-001 | tla-plus | POST-002 | resume produces Active state | verify-standard |
| POST-RETRY-001 | tla-plus | POST-003 | retry produces Active state | verify-standard |
| POST-ANSWER-001 | tla-plus | POST-004 | answer produces Completed state | verify-standard |
| POST-REPLAY-001 | tla-plus | POST-006 | replay derives state purely from journal | verify-standard |
| SEMVER-001 | api-compat | API-001 | Public API unchanged | verify-standard |
| TEST-COMPILE-001 | cargo-test | TEST-001 | Tests compile and pass | verify-standard |

**Gate**: These run after refactoring lands; TLA+ POST assertions can run in parallel with proof lane once model is complete.

## Dependency Graph

```
VERUS-DERIVE-001 ──────────────────────────────┐
VERUS-TRANSITION-001 ──→ KANI-001, KANI-002 ──→ MIRI-001
        │                                           │
        ▼                                           ▼
TLA-LIFECYCLE-001 ──→ TLA-LIFECYCLE-002 ──→ TLA-LIFECYCLE-003
        │
        ▼
POST-*-001 (5 parallel branches)
        │
        ▼
STATIC-LINT-001 ──→ TEST-COMPILE-001, SEMVER-001
```

## Critical Path

```
VERUS-DERIVE-001 → KANI-001 → MIRI-001 → TEST-COMPILE-001
```

**Estimated critical path length**: 3 formal verification lanes + 2 standard lanes.

## Waiver Status

| Waiver ID | Applies To | Status | Notes |
|-----------|-----------|--------|-------|
| WAIVER-LOOM-001 | Loom | Active | Journal thread-safe; no shared mutable state post-refactoring |
| WAIVER-PERF-001 | Performance | Active | Not a correctness requirement |
| WAIVER-LEAN-001 | Theorem | Active | Finite-state, deterministic |

## Exit Criteria

All obligations in `proof-obligations.planned.jsonl` must reach terminal state `pass` or `waived` before bead can close.

---

**Generated**: 2026-05-17
**Author**: femdation p4-proof-plan for vb-0253.7
