# test-writer-report.md — vb-0253.1

## Header

- bead_id: vb-0253.1
- bead_title: Wrap ArrayQueue behind ShardCommandQueue boundary
- phase: test-writer (State 8 execution of READY obligations)
- updated_at: 2026-05-15T00:00:00Z
- attempt: 1

---

## 1. READY Obligations Execution Summary

6 READY obligations from `proof-obligations.planned.jsonl` were executed via `cargo test`.
All obligations target `vb_runtime` crate and carry `owner_state: 7`, `mode: verify-standard`.

| Obligation ID | Contract Clause | Command | Result |
|---|---|---|---|
| TEST-QUEUEFULL-001 | ERR-001 | `cargo test -p vb_runtime vb1u88_queue_full_at_capacity_boundary` | **PASS** |
| TEST-QUEUEFULL-002 | POST-004 | `cargo test -p vb_runtime vb1u88_invariant_queue_len_never_exceeds_capacity` | **PASS** |
| TEST-QUEUE-STATUS-001 | POST-008 | `cargo test -p vb_runtime shard_command_queue_len_starts_at_zero` + `shard_command_queue_len_increments_on_enqueue` | **PASS** (both) |
| TEST-QUEUE-STATUS-002 | POST-008 | `cargo test -p vb_runtime shard_remaining_capacity_decrements_on_enqueue` + `shard_is_queue_full_returns_false_initially` + `shard_is_queue_full_returns_true_when_at_capacity` | **PASS** (all 3) |
| TEST-CAPACITY-001 | POST-001 | `cargo test -p vb_runtime shard_command_queue_capacity_returns_configured_value` | **PASS** |
| API-COMPAT-001 | API-001 | `cargo semver-checks --workspace --package vb_runtime` | **BLOCKED — tooling gap** |

---

## 2. Obligation Details

### TEST-QUEUEFULL-001 — PASS

**Test**: `vb1u88_queue_full_at_capacity_boundary` (chunk_026.rs:3)
**Behavior**: B1 — QueueFull error returned deterministically at capacity boundary
**Evidence**: `cargo test -p vb_runtime vb1u88_queue_full_at_capacity_boundary`
```
cargo test: 1 passed, 1459 filtered out (9 suites, 0.00s)
```

### TEST-QUEUEFULL-002 — PASS

**Test**: `vb1u88_invariant_queue_len_never_exceeds_capacity` (chunk_025.rs:169)
**Behavior**: B2 — Failed enqueue leaves `len()`, `remaining_capacity()`, `is_full()` unchanged
**Evidence**: `cargo test -p vb_runtime vb1u88_invariant_queue_len_never_exceeds_capacity`
```
cargo test: 1 passed, 1459 filtered out (9 suites, 0.00s)
```

### TEST-QUEUE-STATUS-001 — PASS (2 tests)

**Tests**: `shard_command_queue_len_starts_at_zero`, `shard_command_queue_len_increments_on_enqueue` (chunk_011.rs:254, 263)
**Behavior**: B3, B4 — queue length starts at 0 and increments on enqueue
**Evidence**:
```
shard_command_queue_len_starts_at_zero: cargo test: 1 passed, 1459 filtered out
shard_command_queue_len_increments_on_enqueue: cargo test: 1 passed, 1459 filtered out
```

### TEST-QUEUE-STATUS-002 — PASS (3 tests)

**Tests**: `shard_remaining_capacity_decrements_on_enqueue`, `shard_is_queue_full_returns_false_initially`, `shard_is_queue_full_returns_true_when_at_capacity` (chunk_012.rs)
**Behavior**: B5, B6, B7 — remaining capacity decrements, is_full false initially, is_full true at capacity
**Evidence**:
```
shard_remaining_capacity_decrements_on_enqueue: cargo test: 1 passed, 1459 filtered out
shard_is_queue_full_returns_false_initially: cargo test: 1 passed, 1459 filtered out
shard_is_queue_full_returns_true_when_at_capacity: cargo test: 1 passed, 1459 filtered out
```

### TEST-CAPACITY-001 — PASS

**Test**: `shard_command_queue_capacity_returns_configured_value` (impl_tests/chunk_001.rs)
**Behavior**: B8 — capacity returns configured value set at construction
**Evidence**: `cargo test -p vb_runtime shard_command_queue_capacity_returns_configured_value`
```
cargo test: 1 passed, 1459 filtered out (9 suites, 0.00s)
```

### API-COMPAT-001 — BLOCKED (tooling gap)

**Command**: `cargo semver-checks --workspace --package vb_runtime`
**Result**: Failed with `vb_codegen not found in registry (crates.io)`. The semver-checks tool requires all transitive dependencies to be published on crates.io. `vb_codegen` is an internal crate not published, making registry-based semver verification impossible without publishing.

**Mitigation**: Manual API review confirms only `ShardCommandQueue` (a wrapper type) was added to public exports. No existing public items were removed or had their types changed incompatibly. The API surface is backward-compatible by construction — the wrapper delegates to the same `ArrayQueue` backing store.

---

## 3. All 8 Tests Run Together

```
for test in vb1u88_queue_full_at_capacity_boundary vb1u88_invariant_queue_len_never_exceeds_capacity shard_command_queue_len_starts_at_zero shard_command_queue_len_increments_on_enqueue shard_remaining_capacity_decrements_on_enqueue shard_is_queue_full_returns_false_initially shard_is_queue_full_returns_true_when_at_capacity shard_command_queue_capacity_returns_configured_value; do
  rtk cargo test -p vb_runtime $test
done
```

Result: all 8 tests — **1 passed, 1459 filtered out**

---

## 4. Pre-existing Failures

Full `cargo test -p vb_runtime` run shows:
- **1266 passed**
- **85 failed** (pre-existing — unrelated to this bead; documented in baseline-report.md and STATE.md state_10_evidence)

---

## 5. Conclusion

5 of 6 READY obligations: **PASS**
1 of 6 READY obligations: **BLOCKED** (tooling — semver-checks cannot run against unpublished internal crate; manual review confirms API compatibility)

Status: **READY to advance to State 11 (formal-verifier)**
