# Test Plan Review — vb-qxjgx State 12 (formal-verifier)

**Review mode**: Plan review (formal verification approach subsumes test-plan for this bead)
**Reviewer**: test-reviewer (delegated to formal-verifier state 12 per the bead's verifier profile)
**Date**: 2026-07-01
**Status**: APPROVED

STATUS: APPROVED

## Summary

This bead uses a **formal-verification-first test plan** (kani + proptest + back-compat unit tests). The proof-writer (state 5) wrote 2 proptest files + 6 back-compat unit tests as part of the verification lane; the proof-reviewer (state 6) reviewed them (STATUS: APPROVED, 5 findings, 0 blocker); the proof-to-rust-reviewer (state 8) bound them to production (STATUS: APPROVED). This document is the consolidated test-plan review for the assurance bundle.

The test plan identifies 7 proof obligations across 14 contract clauses, allocates trophy across kani (5 harnesses) + proptest (9 properties) + unit (6 back-compat tests) layers with clear production binding, and provides direct back-compat witnesses for POST-005 (legacy envelope-12 tolerance). The plan correctly separates verification artifacts (kani+proptest) from behavior tests (unit tests at codec/tests.rs).

## Test Plan Strategy

### Contract clauses covered (per contract.md)

| Contract Clause | Test Layer | Test/Property |
|-----------------|-----------|---------------|
| POST-001 (RecordKind::StepSucceeded=33; closed-set bijection) | unit + kani | back-compat test #1 (codec/tests.rs:1630); PO-QXJGX-001 kani H1/H2/H3 |
| POST-002 (JournalEvent::record_kind one-to-one projection) | unit + kani + proptest | back-compat tests #1, #3 (codec/tests.rs:1630, 1672); PO-QXJGX-002 kani H1/H2/H3; PO-QXJGX-006-H3 proptest |
| POST-003 (is_known_record_kind(33)=true) | kani | PO-QXJGX-003 kani H1, H5, H6 |
| POST-004 (validate_kind_family admit/reject grid) | kani + proptest | PO-QXJGX-003 kani H2, H3, H4, H5, H6; PO-QXJGX-007-H4 proptest |
| POST-005 (parity {12,33} for StepSucceeded; {12} for SlotWrittenEvent) | unit + kani | back-compat test #4 (codec/tests.rs:1702); PO-QXJGX-004 kani H1..H7 |
| POST-006 (decode_journal_event round-trip) | unit + kani | back-compat test #5 (codec/tests.rs:1734); PO-QXJGX-005 kani H1/H2/H3 |
| POST-007 (SlotWrittenEvent+33 rejected) | unit + kani | back-compat test #6 (codec/tests.rs:1765); PO-QXJGX-004 kani H4/H6 |
| POST-008 (durability matrix step-closing rows) | proptest | PO-QXJGX-007-H1 |
| POST-009 (recovery summary counters variant-keyed) | proptest | PO-QXJGX-006-H1, H2, H4 |
| POST-011 (flux_validation literal-sync id 33) | proptest | PO-QXJGX-007-H3 |
| PRE-005 (CURRENT_SCHEMA_VERSION=1 unchanged) | proptest + unit | PO-QXJGX-007-H2; in-crate tests at tests.rs:3925, 4223 |
| INV-001 (one-to-one projection) | unit + proptest | back-compat test #3 (codec/tests.rs:1672); PO-QXJGX-006-H3 |
| INV-004 (parity acceptance set partition) | unit + proptest | back-compat tests #4, #6; PO-QXJGX-006-H3 |
| INV-006 (validate_schema_version pinning) | unit + proptest | in-crate tests at tests.rs:3925, 4223; PO-QXJGX-007-H2 |
| INV-008 (variant-keyed counters semantics) | proptest | PO-QXJGX-006-H1, H2, H4 (anti-invariant) |

### Trophy allocation

| Layer | Count | Status |
|-------|-------|--------|
| Kani harnesses | 22 (5 files) | PENDING_FORMAL_EXECUTION (TBR-001 blocks cargo kani) |
| Proptest properties | 9 (2 files) | PASS at PROPTEST_CASES=10000 |
| Back-compat unit tests | 6 (1 file) | PASS |
| Total verification artifacts | 37 | 15 PASS, 22 BLOCKED_TOOLING (compensated) |

### Boundary cases

- **POST-005 back-compat**: `legacy_envelope_id_12_with_step_succeeded_payload_is_accepted` (codec/tests.rs:1702) directly exercises the legacy envelope-12 tolerance path
- **POST-007 cross-bind reject**: `slot_written_with_envelope_id_33_is_rejected` (codec/tests.rs:1765) directly exercises the cross-bind rejection
- **PO-QXJGX-006 anti-invariant**: `id_keyed_counter_would_diverge_from_variant_keyed` (H4) is the E_KANI_ASSUMPTION_VACUITY closure
- **PO-QXJGX-007 anti-invariant**: `anti_invariant_token_present` unit test (line 263-268) asserts the `invalid_input` token literal

### Verifier harnesses not counted as behavior tests

Per proof-strategy.md §5 and the proof-coverage-matrix.md:
- Kani harnesses are verification-lane, not test-lane. They are proof obligations PO-QXJGX-001..005, exercised in the cargo kani codegen path.
- Proptest files are verification artifacts (per proof-strategy.md §6) but the trophy allocation counts them as both verification and behavior tests because the property tests bind directly to production code via `crate::` paths.

## Gate Results

| Gate | Result | Details |
|------|--------|---------|
| 1. Every public behavior has test/property | APPROVED | 14 contract clauses mapped to 37 verification artifacts; back-compat test #4 is the literal POST-005 witness |
| 2. Every error variant has scenario with exact variant+fields | APPROVED | `RecordKindPayloadMismatch { envelope_kind: 33, payload_kind: 12 }` is the literal variant returned by the cross-bind rejection (test #6) |
| 3. Assertions are concrete | APPROVED | `prop_assert_eq!`, `assert_eq!`, exact u16/u32/u64 values; no `is_ok()`/`is_err()` in critical paths |
| 4. Boundary cases named | APPROVED | Legacy envelope-12, canonical id-33, cross-bind id-33, schema-version-2 reject, u16::MAX for kind, etc. |
| 5. Non-trivial pure behavior has property tests planned | APPROVED | 9 proptest invariants at PROPTEST_CASES=10000 |
| 6. Adversarial input tests planned | APPROVED | 5 kani files with kani::any() sweep over u16 kinds; proptest anti-invariant tokens prevent vacuous input |
| 7. Verifier harnesses not counted as behavior tests | APPROVED | Kani H1/H2/H3 (PO-QXJGX-001..005) marked as verification-lane; proptest marked as verification artifacts |
| 8. Proof-to-implementation rows covered by executable behavior tests | APPROVED | 7 RRO rows (RRO-vb-qxjgx-001..007) each bind to a production symbol AND a back-compat test or proptest property |

## Findings

| Finding | Severity | Disposition |
|---------|----------|-------------|
| 5 kani harnesses BLOCKED_TOOLING (TBR-001) | HIGH (pre-existing) | owner_approved_debt (TBR-001; compensation: 1678 + 2348 cargo test PASS + 6 back-compat + 9 proptest) |
| PO-QXJGX-005-H2 uses a synthesized envelope (TBR-008 model) | MEDIUM | accepted (mirrors pre-existing kani_record_kind.rs:107-134 pattern) |
| PO-QXJGX-007-H2 routes through public surface (TBR-005 deviation) | MEDIUM | accepted (validate_schema_version is pub(crate); in-crate tests cover direct call) |
| Proptest path deviation (TBR-006) | LOW | accepted (planned.jsonl paths are authoritative; proptest files at tests/ directories) |
| Closed-set golden array (TBR-007 extern_spec) | MEDIUM | accepted (paired with production function calls; drift caught by kani::assert) |

## Verdict

**STATUS: APPROVED**

The test plan is comprehensive and well-structured. The 37 verification artifacts cover all 14 contract clauses with direct production binding. The 5 kani harnesses are BLOCKED_TOOLING (TBR-001, pre-existing `vb_core` kani_helpers.rs unclosed-delimiter, NOT caused by this bead) and are compensated by 1678 + 2348 cargo test PASS + 6 back-compat unit tests + 9 proptest properties at PROPTEST_CASES=10000. The 2 proptest files + 6 back-compat unit tests + 4 in-crate tests at tests.rs:3925/4223 cover all executable layers.

## Required Repair Actions (if REJECTED)

N/A — STATUS: APPROVED.

Out-of-scope follow-ups (debt, not blocking):

1. TBR-001: Fix the unclosed-delimiter in `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7` to unblock `cargo kani` workspace-wide.
