# Behavior Test Plan — vb-vbdco (RE-010)

**Bead:** vb-vbdco (duplicate of closed `vb-y71ef`)
**Closure status:** No new behavior tests authored in this worktree.
The behavior tests that close the RE-010 contract were already
authored and shipped by `vb-y71ef` (commit `d8221505b`,
merged `5f101f82b`).

This plan records the **already-shipped** behavior tests on `main`
and the raw test output that demonstrates they pass.

## Behavior Test Inventory (already on `main`)

### `crates/vb_runtime/src/engine/types.rs` (`mod tests`)

| Test                                                  | Assertion                                                                                              |
|-------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `evidence_collector_returns_typed_error_at_capacity`  | Capacity 2: first two pushes succeed; third returns `Err(EvidenceCapacityExceeded { step, slot: ZERO, capacity: 2, len: 2, required: REQUIRED_STEP_STARTED })`. |
| `evidence_collector_slot_written_typed_error_at_capacity` | Capacity 1: `push_slot_written` after `push_step_started` returns typed error with `slot = <provided>`, `step = ZERO`. |
| `evidence_collector_step_succeeded_typed_error_at_capacity` | Capacity 1: `push_step_succeeded` after `push_step_started` returns typed error with `step = <provided>`, `slot = ZERO`. |
| `evidence_collector_zero_capacity_returns_typed_error_for_every_push` | Capacity 0: every push returns typed error; buffer stays at length 0.                                  |

### `crates/vb_runtime/src/engine/property_tests.rs`

| Test                                                      | Assertion                                                                                              |
|-----------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `evidence_collector_zero_capacity_returns_typed_error_for_every_push` | Mirrors the types.rs test but uses proptest-style field-by-field equality.                              |
| `evidence_collector_capacity_one_first_succeeds_second_is_typed_error` | Capacity 1: first push is `Ok`; second is `Err` and the buffer remains at length 1.                     |

### `crates/vb_runtime/src/engine/tests.rs` (`mod blackhat_engine`)

| Test                                                | Assertion                                                                                              |
|-----------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `bh_eng_01_evidence_collector_enforces_capacity_bound` | 10,000 pushes against capacity-limited collector: first `capacity` succeed, remaining return typed errors; collector length never exceeds capacity. |
| `bh_eng_15_evidence_collector_drain_after_overflow` | After a capacity overflow, `drain()` returns exactly the events that fit; subsequent `is_empty()` is true. |
| `bh_eng_15_evidence_collector_with_capacity_drops_excess` | (Earlier name retained for compatibility; asserts typed-error overflow instead of silent drop.)       |
| `bh_evidence_events_always_alternate_started_succeeded` | Happy-path alternation invariant preserved after the typed-error migration.                            |

### `crates/vb_runtime/src/engine/drive.rs` (`mod tests`)

| Test                                                       | Assertion                                                                                              |
|------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `re_011_evidence_capacity_overflow_does_not_mark_step_succeeded` | When `begin_drive_step` fails because the collector is at capacity, the `RunFrame` is **not** marked succeeded. |

## Raw Evidence

- `07-cargo-test-vb_runtime-evidence.log` — 31 evidence tests pass
- `08-cargo-test-vb_runtime-blackhat.log` — 23 blackhat engine tests pass
- `09-cargo-test-vb_runtime-property.log` — 19 property tests pass
- `10-cargo-test-vb_runtime-all.log` — 1719 vb_runtime unit tests pass
- `11-cargo-test-vb_runtime-re_011.log` — RE-011 transactional ordering test passes
- `12-cargo-test-vb_core-all.log` — 2125 vb_core unit tests pass

## Why No New Behavior Tests Are Authored

1. `vb-y71ef` already shipped the behavior tests above; they pass on
   `main` (see raw logs).
2. Re-running the test fleet for a duplicate-bead closure would
   produce redundant evidence without strengthening the contract.
3. Per AGENTS.md "No Blind Verification Mutations" rule, we trim the
   verification scope to the call-graph blast radius of the bead.
   Because the call-graph blast radius of `vb-vbdco` is the empty
   set (no new commits), no new tests are needed.

If a future regression is found in this surface, a new bead (not
`vb-vbdco`) should be opened with the failure case.
