# Architectural Drift Report: `vb_ipc/src/ids.rs`

**File**: `crates/vb_ipc/src/ids.rs`  
**Total Lines**: 453  
**Threshold**: 300  
**Violation**: YES — 153 lines over threshold

---

## 1. LINE COUNT VIOLATION

| Section | Lines |
|---------|-------|
| Module docs + imports | 1–8 |
| `AskTicketId` impl | 10–42 |
| `ActionTicketId` impl | 44–76 |
| `tests` module | 78–453 |
| **Total** | **453** |

Production code: **76 lines**. Test code: **377 lines** (83% bloat).

---

## 2. TEST DUPLICATION CANCER

The test module is infected with systematic duplication. The following test pairs are **IDENTICAL**:

| Original (line) | Duplicate (line) | Delta |
|-----------------|------------------|-------|
| `ask_ticket_id_ordering_by_wire_value` (169) | `ask_ticket_id_ordering_by_wire_value` (300) | 131 |
| `action_ticket_id_ordering_by_wire_value` (177) | `action_ticket_id_ordering_by_wire_value` (309) | 132 |
| `ask_ticket_id_step_idx_masks_upper_bits` (185) | `ask_ticket_id_step_idx_masks_upper_bits` (338) | 153 |
| `action_ticket_id_step_idx_masks_upper_bits` (193) | `action_ticket_id_step_idx_masks_upper_bits` (347) | 154 |
| `ask_ticket_id_serde_roundtrip` (201) | `ask_ticket_id_serde_roundtrip` (359) | 158 |
| `action_ticket_id_serde_roundtrip` (212) | `action_ticket_id_serde_roundtrip` (370) | 158 |
| `ask_ticket_id_serde_roundtrip_boundary` (223) | `ask_ticket_id_serde_roundtrip_boundary` (381) | 158 |
| `action_ticket_id_serde_roundtrip_boundary` (236) | `action_ticket_id_serde_roundtrip_boundary` (394) | 158 |
| `ask_ticket_id_hash_consistency` (249) | `ask_ticket_id_hash_consistency` (411) | 162 |
| `action_ticket_id_hash_consistency` (259) | `action_ticket_id_hash_consistency` (421) | 162 |

**10 test functions are exact duplicates.** That's ~140 lines of pure waste.

---

## 3. SCOTT WLASCHIN DDD VIOLATIONS

### 3.1 Primitive Obsession (Identical Impl Blocks)

`AskTicketId` and `ActionTicketId` have **byte-for-byte identical** impl blocks:

```rust
impl AskTicketId {
    pub const fn from_wire(raw: u64) -> Self { Self(raw) }
    pub const fn wire_value(self) -> u64 { self.0 }
    pub const fn step_idx(self) -> u16 { (self.0 & 0xFFFF) as u16 }
}

impl ActionTicketId {
    pub const fn from_wire(raw: u64) -> Self { Self(raw) }
    pub const fn wire_value(self) -> u64 { self.0 }
    pub const fn step_idx(self) -> u16 { (self.0 & 0xFFFF) as u16 }
}
```

**Problem**: This is a code smell. Two distinct domain types share identical behavior because they're both just `u64` wrappers. This suggests:
- The types aren't truly semantically distinct (despite being type-distinct at the Rust level)
- OR there's a missing abstraction (a shared `WireId` trait or a generic `NewtypeId<T>` pattern)

### 3.2 Comment-Document Lie

The `from_wire` doc comment claims:

> Panics if the lower 16 bits exceed `u16::MAX` (which is impossible for a valid 64-bit integer, but validates the encoding invariant).

This is **misleading documentation**. The code does:
```rust
pub const fn from_wire(raw: u64) -> Self {
    Self(raw)  // No validation whatsoever
}
```

There is no panic. The comment describes a **phantom check that doesn't exist**. This is a documentation defect.

### 3.3 No Validation of Encoding Invariants

The comment says "validates the encoding invariant" but `from_wire` accepts any `u64` unconditionally. The encoding invariant (lower 16 bits = step index) is **never checked**. Callers can construct malformed IDs and the type system silently accepts them.

**Correct pattern**: Use a constructor that actually validates:
```rust
impl AskTicketId {
    pub const fn from_wire(raw: u64) -> Self {
        assert!(raw <= u16::MAX as u64, "step index overflow");
        Self(raw)
    }
}
```

Or remove the false claim from the doc comment.

---

## 4. RECOMMENDATIONS

### 4.1 Immediate (Line Count Fix)

Move the entire `tests` module to `crates/vb_ipc/tests/ids_tests.rs`.  
The production code (76 lines) fits comfortably under 300.

### 4.2 Short Term (DDD Cohesion)

Extract a shared trait if the two ID types truly share behavior:

```rust
/// Shared behavior for wire-format ticket IDs.
trait WireTicketId: Sized {
    fn from_wire(raw: u64) -> Self;
    fn wire_value(self) -> u64;
    fn step_idx(self) -> u16;
}
```

Then implement once, derive traits generically.

### 4.3 Medium Term (Validation)

Either:
- **Add** the validation the docs claim exists, OR
- **Remove** the false doc comment claiming validation happens

### 4.4 Remove Duplicates

Delete the 10 duplicate test functions (lines 300–428 that duplicate 169–280).

---

## 5. VERDICT

| Violation | Severity |
|-----------|----------|
| Line count (453 > 300) | **CRITICAL** |
| Test duplication (10 pairs) | **HIGH** |
| Primitive obsession | **MEDIUM** |
| Misleading documentation | **MEDIUM** |

**Action Required**: Split tests into separate file. Remove duplicate tests. Add validation or fix docs.
