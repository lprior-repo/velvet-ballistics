# ARCHITECTURAL DRIFT HAMMER REPORT
**File**: `crates/vb_compile/src/kani_wait_digest.rs`
**Line Count**: 303 / 300 (VIOLATION)
**Status**: RED

---

## EXECUTIVE SUMMARY

This file violates the **<300 line rule** (303 lines) and is a textbook case of **Primitive Obsession** — raw `Option<String>` fields are used throughout instead of domain-typed value objects. The string-bounding and charset-validation logic is copy-pasted **4 times** across 4 harnesses. This is a **DDD boundary violation**: verification harness code is not exempt from Scott Wlaschin's "make illegal states unrepresentable" doctrine.

---

## VIOLATIONS

### 1. LINE COUNT: 303 / 300 (VIOLATION)

```rust
// Total: 303 lines
// Limit: 300 lines
// OVERAGE: 3 lines
```

### 2. PRIMITIVE OBSESSION — `Option<String>` for Domain Fields

**Location**: Every harness

**Problem**: `event: Option<String>` and `timeout: Option<String>` are raw primitives. These represent **semantically distinct domain concepts**:
- `event`: A WaitEvent label (must be bounded alphanumeric + underscore)
- `timeout`: A WaitUntil deadline (same constraints, different semantics)

No `WaitFields`, `SlotText`, or `DigestibleWait` type exists. Validation is ad-hoc and repeated.

**Evidence**:
```rust
// Lines 36-52: Manual validation in harness 1
let event: Option<String> = kani::any();
if let Some(ref s) = event {
    kani::assume(s.len() <= 16);
    for ch in s.chars() {
        kani::assume(ch.is_ascii_alphanumeric() || ch == '_');
    }
}
```

This exact pattern repeats at lines 86-97, 154-171, 230-241.

### 3. REPEATED STRING-BOUNDING LOGIC (4x COPY-PASTE)

| Harness | Lines | Pattern |
|---------|-------|---------|
| `wait_digest_step_primitive_no_panic` | 40-52 | `kani::assume(len <= 16)` + charset |
| `wait_until_vs_wait_event_no_collision` | 86-97 | `kani::assume(len <= 8)` + charset |
| `wait_configurations_pairwise_distinct` | 154-171 | `kani::assume(len <= 4)` + lowercase-only |
| `wait_digest_both_copies_no_panic` | 230-241 | `kani::assume(len <= 16)` + charset |

**DRY Violation**: The bounding logic is not reusable. A `fn bound_slot_text(s: &str, max_len: usize)` helper exists nowhere.

### 4. NO DOMAIN TYPE FOR WAIT CONFIGURATION

The file constructs `StepPrimitive::Wait { event, timeout }` directly in each harness. There is no:
- `WaitDigest` value object
- `DigestibleWaitFields` wrapper
- `WaitSlot` with encapsulated validation

**Expected DDD**:
```rust
// Hypothetical — not present
struct DigestibleWait {
    event: Option<SlotText<16>>,  // bounded, validated charset
    timeout: Option<SlotText<16>>,
}
```

### 5. SCATTERED VALIDATION RULES

| Rule | Locations |
|------|-----------|
| `len <= 16` | Lines 41, 48, 231, 237 |
| `len <= 8` | Lines 87, 93 |
| `len <= 4` | Lines 155, 161, 167 |
| `is_ascii_alphanumeric() \|\| ch == '_'` | Lines 44, 50, 89, 95, 233, 239 |
| `is_ascii_lowercase()` | Lines 157, 163, 169 |

These bounds should be **constants on a type**, not magic numbers in harnesses.

---

## RESPONSIBILITY MAP

| Proof ID | Harness | Responsibility | Status |
|----------|---------|----------------|--------|
| PO-001 | `wait_digest_step_primitive_no_panic` | Panic-freedom of Wait arm | LIVE |
| PO-005 | `wait_until_vs_wait_event_no_collision` | WaitUntil vs WaitEvent discrimination | LIVE |
| PO-013 | `wait_configurations_pairwise_distinct` | Pairwise distinct digests (3 shapes) | LIVE |
| PO-015 | `wait_digest_both_copies_no_panic` | Cold-path panic-freedom | LIVE |
| PO-010 | (blocked) | Cross-path equivalence | BLOCKED_DEAD_CODE |

---

## PRESCRIPTION

### Phase 1: Extract Value Objects (Non-Negotiable)

```rust
// New file: vb_compile/src/kani_wait_digest/types.rs

use vb_yaml::ast::StepPrimitive;

/// Bounded slot text for wait fields (alphanumeric + underscore)
#[derive(Debug, Clone)]
pub struct SlotText<const MAX_LEN: usize>(String);

impl<const MAX_LEN: usize> SlotText<MAX_LEN> {
    pub fn new(s: String) -> Option<Self> {
        if s.len() > MAX_LEN {
            return None;
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        Some(Self(s))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

/// Wait fields with enforced validation
#[derive(Debug, Clone)]
pub struct WaitFields {
    pub event: Option<SlotText<16>>,
    pub timeout: Option<SlotText<16>>,
}

impl WaitFields {
    /// Must have at least one Some
    pub fn is_legal(&self) -> bool {
        self.event.is_some() || self.timeout.is_some()
    }

    pub fn to_step_primitive(&self) -> StepPrimitive {
        StepPrimitive::Wait {
            event: self.event.as_ref().map(|s| s.as_str().to_string()),
            timeout: self.timeout.as_ref().map(|s| s.as_str().to_string()),
        }
    }
}
```

### Phase 2: Extract Kani Arbitrary Helper

```rust
// New file: vb_compile/src/kani_wait_digest/arbitrary.rs

use super::types::{SlotText, WaitFields};
use kani::Kani;

impl<const MAX_LEN: usize> kani::Arbitrary for SlotText<MAX_LEN> {
    fn any() -> Self {
        let s: String = kani::any();
        kani::assume(s.len() <= MAX_LEN);
        for ch in s.chars() {
            kani::assume(ch.is_ascii_alphanumeric() || ch == '_');
        }
        // Safety: SlotText::new cannot fail under these assumptions
        unsafe { std::mem::transmute(Self::new(s).unwrap_unchecked()) }
    }
}
```

### Phase 3: Shrink Harnesses to <300 Lines

After Phase 1+2, each harness should be ~20 lines:
```rust
#[kani::proof]
#[kani::unwind(10)]
fn wait_digest_step_primitive_no_panic() {
    let event: Option<SlotText<16>> = kani::any();
    let timeout: Option<SlotText<16>> = kani::any();
    kani::assume(event.is_some() || timeout.is_some());

    let wait = vb_yaml::ast::StepPrimitive::Wait { event, timeout };
    let mut hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, &wait);
    kani::assert(true, "panic-free");
}
```

---

## VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

- **Line Count**: OVER LIMIT (303 > 300)
- **Primitive Obsession**: CRITICAL — raw `Option<String>` in 4 harnesses
- **DRY**: FAIL — string bounding repeated 4x
- **DDD Cohesion**: FAIL — no domain types, validation scattered

**RECOMMENDATION**: Refactor into `kani_wait_digest/` module with `types.rs` and `arbitrary.rs`. Shrink file to ≤300 lines. Create a follow-up bead.

---

*Architectural drift enforcer. Compliance is not optional.*
