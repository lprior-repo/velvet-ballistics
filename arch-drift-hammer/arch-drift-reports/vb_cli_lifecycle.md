# Architectural Drift Report: `vb_cli/lifecycle.rs`

**File**: `crates/vb_cli/src/lifecycle.rs`  
**Total Lines**: 484  
**Threshold**: 300  
**Status**: ❌ VIOLATION

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 484 | 300 | ❌ OVER by 184 lines |

**Required Action**: File MUST be split into multiple modules.

---

## 2. DDD Cohesion Analysis

### Domain Relevance: ✅ STRONG
- Lifecycle state machine properly documented (Pending → Active → WaitingAnswer → Cancelled/Completed/Failed)
- Valid transitions table in doc comment
- Explicit state transition functions: `cancel`, `resume`, `retry`, `answer`
- Uses proper domain types: `RunId`, `LifecycleState`, `LifecycleCommand`, `RunState`, `JournalEvent`

### Cohesion Score: MODERATE (concerns below)

---

## 3. Identified Violations

### HIGH PRIORITY

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 1 | **File size exceedance** | Lines 1-484 | 484 lines >> 300 line limit. Must split. |
| 2 | **Primitive Obsession** | Line 318 `answer(run: RunId, answer: String, ...)` | `String` for answer content should be a NewType wrapper (e.g., `AnswerContent` or `AnswerValue`) |
| 3 | **Comment acknowledges tech debt** | Lines 380-381 | "ConstValue doesn't support String, so we encode the answer as a symbol" — this is a known limitation baked into the API signature |

### MEDIUM PRIORITY

| # | Issue | Location | Description |
|---|-------|----------|-------------|
| 4 | **Extension trait in production module** | Lines 451-459 `EventSeqExt` | Should be in `vb_storage` or with `EventSeq` definition |
| 5 | **Test infrastructure co-located** | Lines 463-483 `test_helpers` | Should be in `crates/vb_cli/tests/` or a `test_utils` crate, not in production source |
| 6 | **Repetitive error construction** | Throughout | `CoreError::LifecycleDuplicateRequest/StaleRequest/InvalidTransition` constructed identically across 4 functions — extract to helper |

### LOW PRIORITY / ACCEPTABLE

| # | Observation | Location | Notes |
|---|-------------|----------|-------|
| 7 | `replay()` function | Lines 417-448 | Appropriate for this module (journal replay is lifecycle recovery concern) |
| 8 | State transition validation | Lines 94-103, 180-188, etc. | Uses `check_lifecycle_transition` correctly |

---

## 4. DDD Smell Assessment

| Smell | Present | Severity |
|-------|---------|----------|
| Primitive Obsession | ✅ Yes | HIGH — `String` for answer |
| State Machine exposed as data | ❌ No | Well-modeled as functions |
| Anemic Domain Model | ❌ No | Commands have behavior |
| Feature Envy | ❌ No | Operations on `RunId` are appropriate |
| Invalid domain behavior in I/O | ❌ No | Clean separation: state lookup → validation → journal write |
| God Module | ⚠️ Yes | 484 lines, 5+ distinct responsibilities |

**Overall DDD Smell**: MODERATE — file is well-organized within itself but violates single responsibility at module level.

---

## 5. Recommended Split

```
vb_cli/src/lifecycle/
├── mod.rs           # Re-export command functions (30-50 lines)
├── cancel.rs        # cancel() function (~65 lines)
├── resume.rs        # resume() function (~70 lines)
├── retry.rs         # retry() function (~65 lines)
├── answer.rs        # answer() function (~90 lines, needs AnswerContent NewType)
├── replay.rs        # replay() function (~40 lines)
└── test_helpers.rs # ONLY if test utilities are truly needed in lib
```

**Note**: `test_helpers` module at lines 463-483 should be removed from production source entirely and placed in integration tests.

---

## 6. Priority & Remediation

| Priority | Action | Estimated Effort |
|----------|--------|------------------|
| **P0** | Split file into `lifecycle/` directory module | Refactor |
| **P0** | Create `AnswerContent` NewType for `answer()` parameter | Small |
| **P1** | Move `test_helpers` to test support crate | Medium |
| **P1** | Extract `EventSeqExt` to `vb_storage` | Small |
| **P2** | Extract error construction helpers | Small |

---

## Summary

```
Lines:        484 (VIOLATION: +184 over limit)
DDD Cohesion: MODERATE (strong domain modeling, primitive obsession on answer)
Priority:     HIGH — structural refactor required
Status:       REFACTOR REQUIRED
```
