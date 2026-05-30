# Architectural Drift Report: `vb_runtime/src/action.rs`

**File:** `crates/vb_runtime/src/action.rs`
**Line count:** 904
**Status:** 🔴 REFACTOR REQUIRED — Gross violation of &lt;300 line rule

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Verdict |
|--------|-------|-------|---------|
| Total lines | 904 | 300 | 🔴 OVER BY 604 LINES |
| Production code | 167 | 300 | ✅ CLEAN |
| Test code | 736 | N/A | 🔴 ISOLATE |
| Test code / production ratio | 4.4:1 | — | ⚠️ SUSPICIOUS |

**The production code is perfectly fine. The test module is 2.4× the total limit.**

---

## 2. RESPONSIBILITY MAP

### 2a. Production code responsibilities (lines 1–167)

| Symbol | Lines | Responsibility | Violation |
|--------|-------|----------------|-----------|
| `ActionRegistry` | 17–122 | Slot-based action contract registry with register/resolve/dispatch | God object candidate |
| `ActionSlot` enum | 21–25, 124–131 | Empty/Registered state for registry slots | Clean |
| `MAX_REGISTERED_ACTIONS` | 13 | Capacity constant (= 65_535) | Clean |
| `dispatch_generic()` | 140–152 | Table-driven dispatch to `Suspended` outcome | **HARDCODED CAPACITY** |
| `validate_input_bytes()` | 155–166 | Byte-limit validation stub (always passes) | **INCOMPLETE / STUB** |

### 2b. Test module responsibilities (lines 168–904)

| Group | Test count | Lines | Subject |
|-------|-----------|-------|---------|
| Registry basics | 13 | ~200 | register, resolve, len, is_empty, duplicate |
| Gap/gap-slot edge cases | 5 | ~100 | gap slots, placeholder IDs, id mismatches |
| Adversarial BDD | 9 | ~180 | overflow, unknown action, mismatched contract |
| **IdempotencyTracker** | **4** | ~90 | `mark_completed`, `is_completed`, duplicate, keys |
| Large registry | 2 | ~50 | many registrations, registered_contracts iteration |

**Critical finding:** 4 tests (lines 812–903) test `IdempotencyTracker`, which is defined in `idempotency.rs`. Those tests are in the wrong file.

---

## 3. DDD ANALYSIS — Scott Wlaschin Principles

### 3a. Primitive Obsession Violations

| Location | Primitive | Should Be |
|----------|-----------|-----------|
| `dispatch_generic` line 149 | `capacity: 1` (hardcoded u32) | `Capacity(1)` newtype or `RetryPolicy::default().max_attempts` |
| `validate_input_bytes` line 162 | `max_bytes: 0, actual_bytes: 0` | `ByteCount(0)` newtype |

The `capacity: 1` in `dispatch_generic` is domain-significant — it controls retry budget — yet is hardcoded with no traceable origin. This is a latent bug.

### 3b. Workflow / State Transition Clarity

`ActionRegistry::dispatch` does three distinct operations in one function:

```
dispatch(input, contract):
  1. resolve action contract by id       ← LOOKUP
  2. verify contract matches input        ← VALIDATION  
  3. call dispatch_generic                ← DISPATCH
```

These three are not separate state-machine transitions; they are conflated. A proper DDD workflow would model each step explicitly with typed transitions.

### 3c. Parse, Don't Validate

`resolve_compile_time` (line 68–75) uses `.and_then()` + `.filter()` chaining. This is "validate then act" — the `filter` rejects after the lookup succeeds. A `Parse` approach would return `Option<&ActionContract>` with the index already validated, not filter post-lookup.

### 3d. Type-Driven Design — Two `action` modules

| File | Purpose |
|------|---------|
| `crates/vb_runtime/src/action.rs` | Public facade: `ActionRegistry` + `IdempotencyTracker` re-export |
| `crates/vb_runtime/src/engine/action.rs` | Engine helpers: `execute_do`, `resume_action_outcome`, `compute_idempotency_key` |

These are completely different domains (registry vs. execution engine), yet share a name. This is a naming boundary violation. The `engine/action.rs` is properly named for its domain; `action.rs` is a misnomer for a **registry facade**.

---

## 4. ACTION ITEMS

### MUST FIX (enforceable)

| # | Finding | Fix | File target |
|---|---------|-----|-------------|
| 1 | 904-line file (test module 736 lines) | Move all 28 ActionRegistry tests to `action_registry_tests.rs` or `tests/action_registry.rs` | `crates/vb_runtime/src/action/` |
| 2 | 4 `IdempotencyTracker` tests in wrong file | Delete from `action.rs`; already exist correctly in `idempotency.rs` | `crates/vb_runtime/src/action.rs` (delete) |
| 3 | `dispatch_generic` hardcodes `capacity: 1` | Replace with `RetryPolicy::default().max_attempts` or a dedicated `DispatchContext` parameter | `crates/vb_runtime/src/action.rs` |
| 4 | Two different `action` modules with same name | Rename `crates/vb_runtime/src/action.rs` → `action_registry.rs` or `registry.rs` | Rename + update `lib.rs` |
| 5 | `validate_input_bytes` is a stub | Either complete it or remove the dead code; a stub that always returns `Ok` is a hazard | `crates/vb_runtime/src/action.rs` |

### SHOULD FIX (architectural)

| # | Finding | Fix |
|---|---------|-----|
| 6 | `ActionRegistry` conflates lookup/validation/dispatch | Extract `resolve_compile_time` → `SlotIndex::parse()` newtype; dispatch becomes a two-step explicit workflow |
| 7 | `ActionSlot` is an internal enum but is not in a private submodule | Move to `registry.rs` private module |
| 8 | `MAX_REGISTERED_ACTIONS` is a raw `usize = 65_535` constant | Wrap as `RegistryCapacity(u16)` newtype |

---

## 5. PROPOSED SPLIT

```
crates/vb_runtime/src/
  action/              ← new directory (replaces action.rs)
    mod.rs             ← barrel: pub mod registry; pub mod dispatch; pub use registry::*
    registry.rs        ← ActionRegistry, ActionSlot, MAX_REGISTERED_ACTIONS (lines 1-137)
    dispatch.rs        ← dispatch_generic, validate_input_bytes (lines 140-166)
    tests/             ← test isolation
      registry_tests.rs ← 28 ActionRegistry tests (lines 168-904 minus 4 IdempotencyTracker tests)
```

After split:
- `registry.rs`: ~137 lines ✅
- `dispatch.rs`: ~30 lines ✅
- `registry_tests.rs`: ~700 lines (still over limit — consider further split by test category)
- `mod.rs`: ~15 lines ✅

---

## 6. VERDICT

```
╔════════════════════════════════════════════════════════════╗
║  ARCHITECTURAL DRIFT: CONFIRMED                           ║
║  File: action.rs (904 lines)                              ║
║  Violation: <300 line rule — OVER BY 604 LINES            ║
║  DDD Issues: 3 primitive obsession, 1 god object,         ║
║              1 naming boundary violation                  ║
║  Files Affected: 1                                        ║
║  Fix Complexity: MEDIUM (test module extraction)          ║
╚════════════════════════════════════════════════════════════╝
```

**Recommended first action:** Extract the `#[cfg(test)]` module into `tests/action_registry_tests.rs` and delete the 4 `IdempotencyTracker` tests (they already live in `idempotency.rs`). This alone removes 736 lines. Then rename `action.rs` → `registry.rs` and create a `dispatch.rs` for the two free functions.
