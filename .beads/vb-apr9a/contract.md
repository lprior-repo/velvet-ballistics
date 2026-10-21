# Fuzz Contract: vb-apr9a PS007 Core/Storage Bridge

**Scope:** Checked conversion and safe selector split. Behavior unchanged.

**Domain:**
- Input: arbitrary byte slice.
- Routing: first byte is selector; remainder is payload.
- Policy value: u64 in [1, 100000000].
- Event length: u64 in [1, 10000].

**Boundary Conditions:**
- Empty input returns immediately.
- Short payload under 8 bytes returns immediately.
- Malformed conversion returns immediately.
- Out-of-range policy or event length returns immediately.

**Routing:**
- Selector byte modulo 3 on remainder payload.
- Case 0: bridge value conversion.
- Case 1: bridge defaults constants.
- Case 2: multiple events count.

**Error Taxonomy:**
- Fallible conversion handled by return.
- No silent truncation; checked division used.
- No panic on malformed input.

**Assertions:**
- Storage limit positive.
- Storage limit equals policy.
- U32 round-trip for values within u32 range.
- Storage default equals core policy.
- Max encoded within storage default.
- Max count positive or encoded length exceeds limit.

**Hazards:**
- Integer conversion between u64 and u32.
- Slice indexing and length validation.
- Division by zero guarded by length check.

**Oracle Strength:**
- Weak scalar checks only.
- Does not prove storage crash or core integration correctness.
- Validates conversion invariants and constant alignment.

**Non-Goals:**
- No proof planning.
- No integration test coverage.
- No performance guarantees.