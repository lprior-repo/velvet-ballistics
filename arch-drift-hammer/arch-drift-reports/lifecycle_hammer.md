# Architectural Drift Report: `lifecycle.rs`

**File**: `crates/vb_cli/src/lifecycle.rs`
**Status**: VIOLATION — 484 lines (exceeds 300-line limit by 184 lines / 61%)
**Date**: 2026-05-29
**Enforcer**: architectural-drift

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 484 | 300 | ❌ OVER |
| Violation | +184 | 0 | ❌ 61% over |

---

## 2. LIFECYCLE RESPONSIBILITY MAP

### 2.1 Exported Public API (6 functions)

| Function | Lines | Responsibility | Cohesion |
|----------|-------|----------------|----------|
| `cancel` | 69–135 | Cancel a run | ⚠️ Mixed |
| `resume` | 150–220 | Resume a cancelled/waiting run | ⚠️ Mixed |
| `retry` | 235–302 | Retry a failed run | ⚠️ Mixed |
| `answer` | 318–405 | Provide answer to waiting run | ⚠️ Mixed |
| `replay` | 417–448 | Reconstruct all run states from journal | ✓ Single |
| `test_helpers::create_run_header` | 472–483 | Test infrastructure | ✓ Isolated |

### 2.2 Internal Helpers

| Function | Lines | Responsibility |
|----------|-------|----------------|
| `current_state_from_journal` | 40–53 | Derive state for single run |
| `EventSeqExt::increment` | 451–459 | EventSeq increment behavior |

### 2.3 DDD Violation: Cross-Cutting Concerns Entangled

The four command functions (`cancel`, `resume`, `retry`, `answer`) each contain **four overlapping concerns**:

1. **State Derivation** — identical `current_state_from_journal()` call
2. **Transition Validation** — duplicate `check_lifecycle_transition()` and state checks
3. **Terminal State Detection** — duplicate `is_terminal()` checks
4. **Journal Writing** — identical sequence calculation and append pattern

This violates **DDD Single Responsibility**: one function should do one thing.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Raw `String` for Answer Content (Line 318)

```rust
pub fn answer(run: RunId, answer: String, journal: &FjallJournal)
```

**Problem**: `String` is a language primitive. Should be a domain value object.

**Should be**: `AnswerContent` or `Answer` wrapper type with validation.

### 3.2 Hardcoded `attempt: 1` (Line 122)

```rust
let event = JournalEvent::RunCancelled {
    run,
    seq: next_seq,
    attempt: 1,  // ← magic number
    reason: None,
};
```

**Problem**: Magic literal instead of computed attempt count.

**Should be**: Derived from existing event count for this run.

### 3.3 Magic `SlotIdx::new(0)` (Line 390)

```rust
slot_idx: vb_core::ids::SlotIdx::new(0), // Default slot for answer
```

**Problem**: "Default slot" is unexplained magic. Which slot? Why 0?

**Should be**: Explicit `DefaultSlot` constant or documented behavior.

### 3.4 Raw `status: 1` in Test Helper (Line 479)

```rust
let header = vb_storage::RunHeaderRecord {
    status: 1,  // ← what does 1 mean?
    accepted_at_ms: 0,
    // ...
};
```

**Problem**: Integer status with no symbolic meaning in test helper.

**Should be**: Use actual `RunStatus` enum from domain.

### 3.5 Raw `accepted_at_ms: 0` Timestamp (Line 480)

```rust
accepted_at_ms: 0,  // ← Unix epoch start
```

**Problem**: Integer timestamp instead of `DateTime<Utc>` or `SystemTime`.

### 3.6 `EventSeq::new(self.get().saturating_add(1))` (Line 457)

```rust
fn increment(self) -> Self {
    Self::new(self.get().saturating_add(1))
}
```

**Problem**: Manual arithmetic on wrapped integer instead of `EventSeq::successor()`.

**Should be**: Already-existing `EventSeq::successor()` or equivalent.

### 3.7 Answer Hash Encoding (Lines 382–386)

```rust
let answer_symbol = vb_core::ids::SymbolId::new(
    answer.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    }) % u32::MAX,
);
```

**Problem**: Hand-rolled string hashing instead of using a proper hash function.

---

## 4. DUPLICATED CODE PATTERNS

### 4.1 Sequence Calculation (Identical in 4 places)

```
cancel:       lines 106–116
resume:       lines 191–201
retry:        lines 272–283
answer:       lines 366–377
```

```rust
let next_seq = journal
    .events_for_run(run)
    .map_err(|e| CoreError::JournalWriteFailure { ... })?
    .last()
    .map(|e| e.seq().increment())
    .unwrap_or(EventSeq::ZERO);
```

**Refactor**: Extract to `fn calculate_next_seq(run: RunId, journal: &FjallJournal)`.

### 4.2 Journal Append Pattern (Identical in 4 places)

```rust
journal
    .append_journaled(&event)
    .map_err(|e| CoreError::JournalWriteFailure {
        code: CoreError::JOURNAL_WRITE_FAILURE_CODE,
        context: e.to_string(),
        timestamp: Utc::now(),
        bead_id: Some(run),
    })?;
```

**Refactor**: Extract to `fn append_event(journal: &FjallJournal, event: &JournalEvent, run: RunId)`.

### 4.3 Duplicate/InvalidTransition/StaleRequest Error Building

Each function builds similar error structs with repeated context formatting.

**Refactor**: Use a helper enum variant builder or state machine validation errors.

---

## 5. STATE MACHINE VIOLATIONS

### 5.1 Redundant Transition Check (answer function, line 356)

```rust
// Check if transition is valid (WaitingAnswer -> Answer is valid) - redundant but required
if !check_lifecycle_transition(current_state, LifecycleCommand::Answer) {
```

The comment admits redundancy. The state-specific error handling above already validates the transition.

**Fix**: Remove redundant check; the `WaitingAnswer` branch is already the valid transition.

### 5.2 Inconsistent Terminal State Handling

| Function | Terminal Check | Then |
|----------|---------------|------|
| `cancel` | `is_terminal()` → `StaleRequest` | Terminal states blocked |
| `resume` | `Completed` → `StaleRequest`, else `InvalidTransition` | Inconsistent |
| `retry` | `is_terminal()` → `StaleRequest` | ✓ Consistent |
| `answer` | `Completed` → `DuplicateRequest`, others → `StaleRequest/InvalidTransition` | Inconsistent |

---

## 6. ARCHITECTURAL BOUNDARY VIOLATIONS

### 6.1 `test_helpers` Module Entangled in Production File (Lines 461–484)

```rust
// TEST INFRASTRUCTURE — NOT PRODUCTION API
pub mod test_helpers {
```

**Problem**: Test-only code occupies 24 lines in production module.

**Should be**: Behind `#[cfg(test)]` or in separate `vb_cli/tests/` integration test file.

### 6.2 Direct `vb_storage::RunHeaderRecord` Construction (Lines 475–482)

The `create_run_header` test helper directly constructs a storage record, bypassing domain abstraction.

---

## 7. REFACTORING PRESCRIPTION

### Phase 1: Extract Value Objects (No Logic Change)

1. Create `AnswerContent(String)` wrapper with validation
2. Create `AttemptCount(u32)` for attempt tracking
3. Replace magic numbers with named constants

### Phase 2: Extract Shared Helpers (Reduce Duplication)

1. `fn next_sequence(run: RunId, journal: &FjallJournal) -> LifecycleResult<EventSeq>`
2. `fn append_event(journal: &FjallJournal, event: JournalEvent, run: RunId) -> LifecycleResult<()>`
3. `fn validate_transition(state: LifecycleState, cmd: LifecycleCommand) -> LifecycleResult<()>`

### Phase 3: Collapse Lifecycle Commands (Reduce File to ~300 lines)

Replace 4 × 66-line functions with:

```
cancel    → ~20 lines (delegates to shared helpers)
resume    → ~20 lines
retry    → ~20 lines
answer   → ~25 lines (has unique answer encoding)
replay   → ~30 lines
helpers  → ~10 lines
```

### Phase 4: Move Test Infrastructure

Extract `test_helpers` to `vb_cli/tests/lifecycle_test_helpers.rs` behind integration test feature.

---

## 8. SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Line count violation | 1 | CRITICAL |
| Primitive obsession | 7 | HIGH |
| Duplicated code patterns | 3 | HIGH |
| State machine inconsistencies | 2 | MEDIUM |
| Test code in production | 1 | MEDIUM |
| Redundant validation | 1 | LOW |

**Priority**: Refactor to extract shared helpers and value objects first, then collapse function bodies. Target: 280 lines (-204 lines, -42%).

---

## 9. VERIFICATION COMMANDS

```bash
# Verify current line count
wc -l crates/vb_cli/src/lifecycle.rs

# After refactoring, should show ≤300
moon run :clippy --package vb_cli
moon run :test --package vb_cli
```
