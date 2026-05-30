# Architectural Drift Report: `contract_encoding.rs`

**File**: `crates/vb_core/src/contract_encoding.rs`
**Line count**: 457 (VIOLATION: limit is 300)
**Status**: `REFACTOR REQUIRED`

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 457 | 300 | ❌ VIOLATION |
| Production code | ~60 | — | — |
| Test code | ~368 | — | — |
| Code:test ratio | 1:6.1 | — | — |

The test module (lines 89–457) is **368 lines**. The production function (lines 27–87) is **~60 lines**. The file bundles 6× more test code with its subject, violating single-responsibility.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Encoding uses raw primitives directly

The `encode_contract_bytes` function operates on raw primitive types with no domain newtypes:

```rust
pub fn encode_contract_bytes(contract: &ResourceContract) -> Vec<u8> {
    buf.extend_from_slice(&contract.max_steps.to_le_bytes());      // u16 raw
    buf.extend_from_slice(&contract.max_slots.to_le_bytes());      // u16 raw
    buf.extend_from_slice(&contract.max_constants.to_le_bytes());  // u16 raw
    buf.extend_from_slice(&contract.max_accessors.to_le_bytes()); // u16 raw
    buf.extend_from_slice(&contract.max_expressions.to_le_bytes());// u16 raw
    buf.extend_from_slice(&[contract.max_expr_stack]);             // u8  raw
    buf.extend_from_slice(&contract.max_step_budget_per_tick.to_le_bytes()); // u64 raw
    buf.extend_from_slice(&contract.max_transitions_per_tick.to_le_bytes());// u64 raw
    buf.extend_from_slice(&contract.max_input_bytes.to_le_bytes());     // u32 raw
    buf.extend_from_slice(&contract.max_output_bytes.to_le_bytes());    // u32 raw
    buf.extend_from_slice(&contract.max_blob_bytes.to_le_bytes());     // u64 raw
    buf.extend_from_slice(&contract.max_ipc_payload_bytes.to_le_bytes());// u32 raw
    buf.extend_from_slice(&contract.max_retry_attempts.to_le_bytes());  // u16 raw
    buf.extend_from_slice(&contract.max_fanout.to_le_bytes());          // u16 raw
    buf.extend_from_slice(&contract.max_collect_items.to_le_bytes());   // u32 raw
    buf.extend_from_slice(&contract.max_queue_depth.to_le_bytes());     // u32 raw
    buf.extend_from_slice(&contract.max_journal_batch_bytes.to_le_bytes());// u32 raw
    buf.push(u8::from(contract.allows_secret_results));               // bool raw
}
```

**Required newtypes** (DDD Value Objects):
- `StepCount(u16)` — node count
- `SlotCount(u16)` — runtime slots
- `ConstantCount(u16)` — constant pool entries
- `AccessorCount(u16)` — accessor programs
- `ExpressionCount(u16)` — expression programs
- `ExprStackDepth(u8)` — expression stack entries
- `StepBudget(u64)` — deterministic transitions per tick
- `TransitionCount(u64)` — transitions per tick
- `ByteCount(u32/u64)` — various byte limits (input, output, blob, IPC, journal)
- `RetryPolicy(u16)` — retry attempts
- `FanoutFactor(u16)` — branch fanout
- `CollectLimit(u32)` — collect items
- `QueueDepth(u32)` — runtime queue depth
- `SecretResultFlag` — secret-tainted results policy (bool → enum)

### 2.2 `allows_secret_results: bool` is untyped

A `bool` carries no domain semantics. Replace with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretResultPolicy {
    Forbidden,
    Permitted,
}
```

### 2.3 No `Parse, don't validate` — encoding trusts input blindly

The function does zero validation. Any `ResourceContract` with `max_steps = 0` or `max_steps = u16::MAX` passes through unchanged. The encoding is not self-describing; callers must know the exact type of each field to decode.

---

## 3. DUAL `ResourceContract` DEFINITIONS — ARCHITECTURE LEAK

Two incompatible `ResourceContract` structs exist:

| Location | Fields | Has `allows_secret_results` | Has `max_transitions_per_tick` |
|----------|--------|------------------------------|--------------------------------|
| `workflow/mod.rs:191` | 17 | ✅ | ✅ |
| `compiled_workflow.rs:130` | 15 | ❌ | ❌ |

`contract_encoding.rs` encodes ALL 17 fields including `allows_secret_results` and `max_transitions_per_tick` (lines 53–54, 83–84). But `compiled_workflow::ResourceContract` only has 15 fields. This means **the encoding format is tied to the `workflow` module's definition, creating a hard coupling** between the encoder and that specific struct variant.

If a `compiled_workflow::ResourceContract` were passed to `encode_contract_bytes`, it would not compile (missing fields). But if the two structs ever diverge further, the encoder silently encodes the wrong thing.

---

## 4. TAG-LENGTH-VALUE CONVENTION IS DUPLICATED IN TESTS

The field tags are declared **three times** in this file:
1. In `encode_contract_bytes()` (lines 32–83) — production
2. In test `encode_contract_bytes_contains_all_17_field_tags_in_order` (lines 138–157)
3. In test `encode_contract_bytes_field_tags_are_unique` (lines 244–286)

This is not "once and only once." If a field is added, all three must be updated in lockstep or the tests give false confidence.

---

## 5. TEST REDUNDANCY (Low severity, but noted)

- `encode_contract_bytes_is_deterministic` (lines 97–106) and `encode_contract_bytes_is_deterministic_for_random_contract` (lines 108–120) test the same property.
- `encode_contract_bytes_uses_little_endian_for_max_steps_u16` and the `max_transitions_per_tick` and `max_blob_bytes` tests are identical in structure — boilerplate that should be a parameterized test.
- The extreme-values test (lines 350–398) manually sets 17 fields — this could be a `proptest` property.

---

## 6. REQUIRED REFACTOR PLAN

### Step 1: Split tests into separate file

Move lines 89–457 → `contract_encoding_tests.rs` (or `contract_encoding/tests.rs`).

Target: `crates/vb_core/src/contract_encoding/tests_contract_encoding.rs`

### Step 2: Extract encoding field descriptors

Create a const array of field descriptors:

```rust
/// Field descriptor for canonical encoding.
struct FieldDesc {
    tag: &'static [u8],
    encode: fn(&ResourceContract, &mut Vec<u8>),
}
```

The production function then iterates this table. Tags live in one place only.

### Step 3: Address primitive obsession at the `ResourceContract` level

The `ResourceContract` struct in `workflow/mod.rs` should use domain types:

```rust
pub struct ResourceContract {
    pub max_steps: StepCount,
    pub max_slots: SlotCount,
    // ...
    pub allows_secret_results: SecretResultPolicy,
}
```

This is a larger refactor touching `workflow/mod.rs`. See bead creation recommendation below.

### Step 4: Version the encoding format

Add a version byte prefix so the format can evolve:

```rust
buf.extend_from_slice(b"resource_contract_v1");
```

---

## 7. VERDICT

| Rule | Status |
|------|--------|
| < 300 lines | ❌ VIOLATION (457 lines) |
| Primitive obsession | ❌ VIOLATION (17 raw primitives) |
| Single responsibility | ❌ VIOLATION (encoder + 368-line test module) |
| Parse don't validate | ⚠ WARNING (no input validation) |
| DDD cohesion | ❌ VIOLATION (no value objects) |

**The file cannot be approved as-is. Splitting is mandatory.**
