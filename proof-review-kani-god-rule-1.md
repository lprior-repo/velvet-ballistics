# KANI GOD RULE 1 PROOF REVIEW

**Reviewer**: proof-reviewer agent  
**Date**: 2026-05-23  
**Scope**: velvet-ballistics Kani harnesses — hardcoded shape audit  
**Rule**: GOD RULE 1 — "No Hardcoded Kani Shapes. Kani verification harnesses MUST NOT hardcode structural inputs with fixed dummy data. You MUST implement and use kani::Arbitrary for core structures, or write safe, exhaustive generator harnesses using kani::any(). Proving that a function doesn't panic on one hardcoded data structure proves nothing."

---

## SUMMARY

| Severity | Count | Files |
|----------|-------|-------|
| CRITICAL (BLOCKER) | 7 | kani_expr_bound.rs, kani_capability_harnesses.rs, kani_ipc_header.rs, kani_ipc_header_rejects_oversize.rs, kani_step_budget.rs, kani_step_budget_one.rs, kani_step_budget_zero.rs |
| MODERATE (DEBT) | 2 | kani_idempotency_gates.rs, kani_codec.rs |
| GOOD (APPROVED) | 5 | kani_step_harnesses.rs, kani_admission.rs, kani_recovery_hydrate.rs, gate_08_accessor.rs (verification module), capability_schema_kani.rs |

**Overall Status**: REJECTED -> FIXES APPLIED -> PENDING KANI EXECUTION

---

## CRITICAL FINDINGS (ALL FIXED)

### 1. `crates/vb_core/src/kani_expr_bound.rs` — COMPLETELY HARDCODED

Every harness used fixed literal arrays. No `kani::any()` anywhere.

| Line | Harness | Hardcoded Values |
|------|---------|-----------------|
| 10 | `harness_empty_ops_returns_zero` | `[]` |
| 18 | `harness_single_loadslot_returns_one` | `[ExprOp::LoadSlot(SlotIdx::new(0))]` |
| 29 | `harness_single_loadconst_returns_one` | `[ExprOp::LoadConst(ConstIdx::new(0))]` |
| 41 | `harness_single_loadaccessor_returns_one` | `[ExprOp::LoadAccessor(AccessorIdx::new(0))]` |
| 53 | `harness_binary_op_tracks_depth_correctly` | `[LoadSlot(0), LoadSlot(1), Add]` |
| 68 | `harness_unary_op_tracks_depth_correctly` | `[LoadSlot(0), Not]` |
| 79 | `harness_appendif_tracks_depth_correctly` | `[LoadSlot(0), LoadSlot(1), LoadSlot(2), AppendIf]` |
| 95 | `harness_nested_binary_ops_tracks_max_depth` | 6-element fixed array |
| 110 | `harness_all_unary_ops_valid` | 7-element fixed array of unary ops |
| 128 | `harness_all_binary_ops_valid` | 18-element fixed array of binary ops |
| 163 | `harness_no_overflow_within_capacity` | 3-element fixed array of load ops |
| 175 | `harness_checked_sub_underflow_detection` | `[Not, LoadSlot(0)]` |
| 182 | `harness_complex_expression_correct` | 6-element fixed array |
| 200 | `harness_multiple_loads_max_correct` | 5-element fixed array |

**FIX**: Rewritten with `arbitrary_expr_op()` helper generating symbolic ExprOp variants, and `kani::any()` for array length and indices. Added `kani::cover!()` for non-vacuity.

---

### 2. `crates/vb_core/src/kani_capability_harnesses.rs` — COMPLETELY HARDCODED

All capability names and action IDs were literal strings/numbers.

| Line | Harness | Hardcoded Values |
|------|---------|-----------------|
| 20 | `capability_name_grants_harness` | `"action"`, `"storage"`, `"act"`, `ActionId::new(7)`, `ActionId::new(8)` |
| 58 | `capability_name_grants_exact_match_case` | `"network"`, `ActionId::new(1)` |
| 68 | `capability_name_rejects_prefix_dot_case` | `"network"`, `"network.github"`, `ActionId::new(1)` |
| 80 | `capability_name_grants_partial_segment_rejected` | `"net"`, `"network"`, `ActionId::new(1)` |
| 91 | `capability_name_grants_non_prefix_rejected` | `"storage"`, `"network"`, `ActionId::new(1)` |
| 101 | `capability_name_empty_grant_rejected` | `""`, `"network"`, `ActionId::new(1)` |
| 112 | `capability_name_action_mismatch_rejected` | `"network"`, `ActionId::new(2)`, `ActionId::new(1)` |

**FIX**: Rewritten with `kani::any::<Capability>()` and `kani::any::<ActionId>()`. Added `kani::cover!()` for non-vacuity.

---

### 3. `crates/vb_ipc/src/kani_ipc_header.rs` — COMPLETELY HARDCODED

No `kani::any()` for command, flags, correlation, or payload_len.

| Line | Harness | Hardcoded Values |
|------|---------|-----------------|
| 13 | `kani_ipc_header_decode_valid` | `IpcCommand::Health`, `flags=0`, `correlation=12345`, `payload_len=0` |
| 42 | `kani_ipc_header_rejects_bad_magic` | `0x12345678` magic |
| 57 | `kani_ipc_header_rejects_bad_version` | `IPC_VERSION+1` |
| 69 | `kani_ipc_header_rejects_reserved_nonzero` | `reserved=1` |
| 84 | `kani_ipc_header_decode_various_commands` | Fixed array of 5 commands |
| 105 | `kani_ipc_header_preserves_all_fields` | `SubmitRun`, `0x00FF`, `0xDEAD_BEEF_CAFE`, `256` |

**FIX**: Rewritten with `arbitrary_command()` helper and `kani::any()` for flags, correlation, payload_len. Added `kani::cover!()` for non-vacuity.

---

### 4. `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` — COMPLETELY HARDCODED

All payload_len and limit values were literals.

| Line | Harness | Hardcoded Values |
|------|---------|-----------------|
| 13 | `kani_ipc_header_rejects_oversize_payload` | `payload_len=1024`, `limit=16` |
| 44 | `kani_ipc_header_accepts_within_bound` | `payload_len=256`, `limit=1024` |
| 64 | `kani_ipc_header_rejects_exactly_over_limit` | `payload_len=101`, `limit=100` |
| 85 | `kani_ipc_header_accepts_exactly_at_limit` | `payload_len=100`, `limit=100` |
| 105 | `kani_ipc_header_rejects_any_payload_when_max_zero` | `payload_len=1` |
| 122 | `kani_ipc_header_accepts_large_with_large_max` | `payload_len=1_000_000` |

**FIX**: Rewritten with symbolic `payload_len` and `limit` using `kani::any()`. Added `kani::cover!()` for non-vacuity.

---

### 5. `crates/vb_core/src/kani_step_budget.rs` — H8/H9 HARDCODED ARRAYS

H8-H9 iterated cartesian product over fixed literal arrays.

| Line | Harness | Issue |
|------|---------|-------|
| 83 | `kani_checked_mul_boundaries` | Fixed array `[0, 1, 2, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX]` — only 49 pairs tested |
| 106 | `kani_checked_add_boundaries` | Fixed array `[0, 1, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX]` — only 36 pairs tested |

**FIX**: Replaced fixed arrays with `kani::any::<u64>()` for symbolic a/b pair exploration. Added `kani::cover!()` for non-vacuity.

---

### 6. `crates/vb_core/src/kani_step_budget_one.rs` — H8 HARDCODED STRUCT

| Line | Harness | Issue |
|------|---------|-------|
| 91 | `kani_aggregate_usage_one_step` | `AggregateResourceUsage` and `AggregateResourceBudget` with every field set to literal `0` or `1` |

**FIX**: Replaced hardcoded structs with `kani::any::<AggregateResourceUsage>()` and `kani::any::<AggregateResourceBudget>()` (Arbitrary impls already exist). Added `kani::cover!()` for non-vacuity.

---

### 7. `crates/vb_core/src/kani_step_budget_zero.rs` — H3/H4 HARDCODED STRUCTS

| Line | Harness | Issue |
|------|---------|-------|
| 36 | `kani_aggregate_usage_zero` | Both structs with all fields literal `0` |
| 78 | `kani_try_add_budget_zero_current` | Both structs with all fields literal `0` except one field = `1` |

**FIX**: Replaced hardcoded structs with `kani::any::<AggregateResourceUsage>()` and `kani::any::<AggregateResourceBudget>()`. Added `kani::cover!()` for non-vacuity.

---

## MODERATE FINDINGS (FIXED)

### 8. `crates/vb_core/src/kani_idempotency_gates.rs` — MIXED SYMBOLIC/HARDCODED

`ActionContract` was generated via `kani::any::<ActionContract>()` (good), but frame dimensions, slot values, and key slot indices were hardcoded.

| Line | Harness | Hardcoded Values |
|------|---------|-----------------|
| 31 | `verify_idempotency_all_clean` | `RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4)`, `SlotValue::I64(42)`, key_slots `[0,1,2,3]` |
| 76 | `verify_idempotency_missing_key` | Same hardcoded frame |
| 109 | `verify_idempotency_secret_in_key` | Same hardcoded frame, hardcoded slot taint pattern |
| 176 | `verify_idempotency_single_error` | Same hardcoded frame, hardcoded slot taint pattern |
| 241 | `verify_idempotency_none_side_effect_always_ok` | Hardcoded frame |
| 268 | `verify_idempotency_safe_retry_always_ok` | Hardcoded frame |
| 294 | `verify_idempotency_unsafe_retry_always_err` | Hardcoded frame |

**FIX**: Replaced hardcoded frame dimensions with `kani::any::<u16>()` for slot_count, `kani::any()` for RunId/StepIdx. Replaced hardcoded `SlotValue::I64(42)` with `arbitrary_simple_slot_value()` helper. Replaced `frame.ok().unwrap()` with safe `let Ok(frame) = frame else { return; }` destructuring.

---

## MODERATE FINDINGS (NOT FIXED — DEBT)

### 9. `crates/vb_storage/src/kani_codec.rs` — MIXED

| Line | Harness | Assessment |
|------|---------|------------|
| 23 | `kani_truncated_header_zero_bytes` | Boundary test (0 bytes) — acceptable |
| 33 | `kani_truncated_header_30_bytes` | Uses `kani::any()` — good |
| 43 | `kani_truncated_header_59_bytes` | Uses `kani::any()` — good |
| 53 | `kani_bad_magic_bytes` | Hardcodes `0xDEADBEEF` but rest is symbolic |
| 64 | `kani_wrong_magic_any_value` | Uses `kani::any()` for header and wrong_magic — good |
| 83 | `kani_future_schema_version` | Hardcodes byte layout |
| 101 | `kani_past_schema_version` | Hardcodes byte layout |
| 120 | `kani_bad_crc` | Hardcodes byte layout |
| 138 | `kani_arbitrary_header_60_bytes` | Uses `kani::any()` — good |
| 168 | `kani_decode_header_exhaustive_error_coverage` | Uses `kani::any()` — good |

**Verdict**: DEBT. The specific-condition harnesses construct valid headers with one bad field using hardcoded byte offsets. The arbitrary harnesses provide symbolic coverage, so this is not a full blocker. Recommend future bead to refactor these to use `kani::any()` for the base header and only hardcode the specific field being tested.

---

## APPROVED FILES (UNCHANGED)

### `crates/vb_core/src/kani_step_harnesses.rs`
- Uses `kani::any::<WorkflowParts>()`, `kani::any::<u16>()`, `kani::any::<u8>()`, `kani::any::<SlotValue>()`, `kani::any::<Taint>()`
- Has `kani::cover!()` statements for non-vacuity
- **APPROVED** for GOD RULE 1

### `crates/vb_storage/src/kani_admission.rs`
- Uses `kani::any::<WorkflowDigest>()`, `kani::any::<u8>()`, `kani::any::<bool>()`
- **APPROVED** for GOD RULE 1

### `crates/vb_storage/src/kani_recovery_hydrate.rs`
- Uses custom `kani::Arbitrary` for `TailEventMetadata` and `TailMetadataBatch`
- Uses `kani::any()` for primitives
- Has `kani::cover!()` statements
- **APPROVED** for GOD RULE 1

### `crates/vb_validate/src/gate_08_accessor.rs` (verification module, lines 518-595)
- Uses `kani::any::<u32>()`, `kani::any::<u16>()` with bounded assumptions
- **APPROVED** for GOD RULE 1

### `crates/vb_validate/tests/capability_schema_kani.rs`
- Uses `kani::any::<usize>()` with bounded assumptions
- **APPROVED** for GOD RULE 1

---

## FIXES SUMMARY

| File | Changes |
|------|---------|
| `crates/vb_core/src/kani_expr_bound.rs` | `arbitrary_expr_op()` generator, symbolic arrays, `kani::cover!()` |
| `crates/vb_core/src/kani_capability_harnesses.rs` | `kani::any::<Capability>()`, `kani::any::<ActionId>()`, `kani::cover!()` |
| `crates/vb_ipc/src/kani_ipc_header.rs` | `arbitrary_command()`, `kani::any()` for all fields, `kani::cover!()` |
| `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` | Symbolic `payload_len` and `limit`, `kani::cover!()` |
| `crates/vb_core/src/kani_step_budget.rs` | H8/H9: `kani::any::<u64>()` instead of fixed arrays |
| `crates/vb_core/src/kani_step_budget_one.rs` | H8: `kani::any::<AggregateResourceUsage/Budget>()` |
| `crates/vb_core/src/kani_step_budget_zero.rs` | H3/H4: `kani::any::<AggregateResourceUsage/Budget>()` |
| `crates/vb_core/src/kani_idempotency_gates.rs` | Symbolic frame dims, slot values, safe destructuring |

---

## VERIFICATION

- `cargo check -p vb_core -p vb_ipc -p vb_storage -p vb_validate`: **PASS** (7 crates compiled successfully)
- No `unwrap()` or `expect()` in any rewritten file
- All rewritten files contain `kani::any()` (symbolic inputs) and `kani::cover!()` (non-vacuity)

---

STATUS: FIXES APPLIED — PENDING KANI EXECUTION FOR FINAL APPROVAL
