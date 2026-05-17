# Proof Plan Review Input: vb-0253.7

## Bead Context

- **Bead ID**: vb-0253.7
- **Title**: cli: Make lifecycle tracker event-applied
- **Phase**: 4 (Proof Planning)
- **Primary Crate**: `vb_cli` (`crates/vb_cli/src/lifecycle.rs`)
- **Source**: `/home/lewis/src/velvet-ballistics` (read-only checkout)

## Contract Summary

Refactoring `RunStateTracker` (in-memory `LazyLock<Mutex<>>`) to derive ALL state from journal events. Public API unchanged. Invariants: INV-001 (state-journal consistency), INV-002 (no divergence), INV-003 (valid transitions only), INV-004 (event immutability), INV-005 (terminal states final).

## Risk-Tag Breakdown

| Risk Tag | Count | Obligations |
|----------|-------|-------------|
| `proof` | 5 | TLA-LIFECYCLE-001, TLA-LIFECYCLE-002, TLA-LIFECYCLE-003, VERUS-DERIVE-001, VERUS-TRANSITION-001 |
| `high` | 4 | KANI-001, KANI-002, MIRI-001, STATIC-LINT-001 |
| `medium` | 7 | POST-CANCEL-001, POST-RESUME-001, POST-RETRY-001, POST-ANSWER-001, POST-REPLAY-001, SEMVER-001, TEST-COMPILE-001 |

**Total**: 16 proof obligations across 7 verification lanes.

## Strategy Summary

1. **Phase 1 (Formal Proof)**: verus + tla-plus lanes, ordered by dependency
2. **Phase 2 (Deep Verification)**: kani + miri after verus confirms panic-freedom
3. **Phase 3 (Standard)**: static-scan, cargo-test, api-compat after refactoring lands

## Verification Layer Assignments (from verification-layers.md)

| Clause | Primary Layer | Secondary Layer |
|--------|--------------|-----------------|
| INV-001 | tla-plus | verus |
| INV-002 | tla-plus | verus |
| INV-003 | verus | kani |
| INV-004 | tla-plus | verus |
| INV-005 | tla-plus | verus |
| PRE-001 | verus | kani |
| PRE-002 | verus | kani |
| PRE-003 | verus | kani |
| PRE-004 | tla-plus | miri |
| POST-001 to 004 | tla-plus | verus |
| POST-005 | verus | kani |
| POST-006 | tla-plus | verus |

## Open Questions for Review

1. **Q1**: Does the TLA+ model in `specs/Lifecycle.tla` already exist in the velvet-ballistics checkout, or does it need to be authored?
2. **Q2**: Are the Verus spec functions (`spec_derive_lifecycle_state_from_events`, `spec_check_lifecycle_transition`) already defined in `vb_core`?
3. **Q3**: Should KANI-001 and KANI-002 be merged into a single harness for efficiency?
4. **Q4**: Should POST-* obligations be reclassified from `verify-standard` to `verify-proof` given their TLA+ nature?

## Requested Review Decisions

1. Approve/reject lane execution order
2. Approve/reject critical path assessment
3. Approve/reject waiver applications
4. Confirm dependency graph correctness
5. Flag any obligations requiring proof-reviewer gate before execution

---

**Status**: Awaiting proof-reviewer input before executing formal verification lanes.
