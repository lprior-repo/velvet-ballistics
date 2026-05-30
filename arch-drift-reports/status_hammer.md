# Architectural Drift Report: `status.rs`

**File**: `crates/vb_cli/src/status.rs`
**Size**: 576 lines (**VIOLATION: exceeds 300-line limit by 192%**)
**Date**: 2026-05-29
**Enforcer**: arch-drift-hammer

---

## Executive Summary

This file has **ONE CRITICAL VIOLATION** (file size) and **THREE/PRIMARY OBSESSION** violations requiring immediate refactoring.

---

## 1. File Size Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 576 | 300 | **FAIL** (192%) |
| Module code | ~210 | 200 | MARGINAL |
| Match arm code | 162 | — | ISOLATE |
| Test code | ~103 | 100 | MARGINAL |

**Required Action**: Split into multiple modules before any feature work proceeds.

---

## 2. Primitive Obsession Violations

### VIOLATION A: `ReplayExplainEntry` Uses Raw `u16` Instead of Typed IDs

**Location**: Lines 68–82

```rust
pub struct ReplayExplainEntry {
    pub seq: u64,                    // OK — seq is raw but tied to journal semantics
    pub event_type: &'static str,    // OK — string interning is intentional
    pub workflow_digest: Option<WorkflowDigest>,  // OK
    pub record_kind: Option<vb_storage::RecordKind>,  // OK
    pub step: Option<u16>,           // **PRIMITIVE OBSESSION** — should be `Option<StepIdx>`
    pub action: Option<u16>,         // **PRIMITIVE OBSESSION** — should be `Option<ActionId>`
}
```

**Problem**: `StepIdx` and `ActionId` exist in `vb_core::ids`. The struct fields use raw `u16` instead, creating:
- No compile-time guarantee of correct ID space
- Loss of semantic intent in the type signature
- Inconsistency with the rest of the codebase which uses typed IDs

**Evidence**: In `build_explain_entry` (line 331), we see `Some(s.get())` extracting a raw `u16` from `StepIdx`, then wrapping it in `Option<u16>` instead of `Option<StepIdx>`.

---

### VIOLATION B: `derive_status_from_events` — Internal State Uses Raw Primitives

**Location**: Lines 136–139

```rust
let mut pending_action: Option<ActionId> = None;
let mut pending_step: Option<StepIdx> = None;
let mut retry_step: Option<StepIdx> = None;
let mut terminal_state: Option<DerivedStatus> = None;
```

**Problem**: Local variables are correctly typed, BUT the extraction from events is inconsistent:

```rust
// Line 155-158: Correct — uses typed ActionId
JournalEvent::ActionScheduled { action, step, .. } if pending_action.is_none() => {
    pending_action = Some(*action);
    pending_step = Some(*step);
}

// Line 449-451: WRONG — extracts raw u16 via .get()
JournalEvent::ActionScheduledTicket { ticket, .. } => (
    "ActionScheduledTicket",
    None,
    Some(vb_storage::RecordKind::ActionScheduled),
    Some(ticket.step.get()),      // **PRIMITIVE** — ticket.step is StepIdx, we extract u16
    Some(ticket.action.get()),    // **PRIMITIVE** — ticket.action is ActionId, we extract u16
),
```

**The `ticket` field is a typed `ActionScheduledTicket` that already has `StepIdx` and `ActionId` — we downgrade to raw `u16` here.**

---

### VIOLATION C: `StatusError::Inconsistency` Uses `String` Instead of Proper Error Domain

**Location**: Lines 25–28

```rust
Inconsistency {
    reason: String,  // **PRIMITIVE OBSESSION** — should use a proper error detail type
}
```

**Problem**: `String` for error context violates the error taxonomy principle. Compare:

- `StatusError::RunNotFound { run_id: RunId }` — correctly typed
- `StatusError::Inconsistency { reason: String }` — weakly typed string

**Correct Pattern**: Should have a proper `InconsistencyKind` enum or structured error context:

```rust
Inconsistency {
    kind: InconsistencyKind,
    context: HashMap<String, String>,  // or structured fields
}
```

---

## 3. Structural Issues

### ISSUE 1: `build_explain_entry` is 162 Lines of Match Arms

**Location**: Lines 308–470

This function handles 20+ event variants in a single monolithic match. Each arm returns a 5-tuple with hardcoded string literals.

**Problems**:
- Violates single-responsibility principle
- String literals like `"RunAccepted"`, `"StepStarted"` are untyped constants
- Silent fallback `_ => ("Unknown", None, None, None, None)` swallows variants

**Refactoring Direction**: 
- Extract a `EventExplainer` trait or `fn explain_event_type(event: &JournalEvent) -> &'static str`
- Use a derive macro to generate the match arms automatically
- Make unknown events a compile-time error, not a runtime silent fallthrough

---

### ISSUE 2: `derive_status_from_events` Scans Events Twice

**Location**: Lines 141–173 (first scan) + Lines 188–190 (second scan)

```rust
// First scan: collect state
for event in events { ... }

// Second scan: check for failed
let has_failed = events
    .iter()
    .any(|e| matches!(e, JournalEvent::RunFailedEvent { .. }));
```

**Problem**: Redundant O(n) scan. The first loop can track `has_failed` inline.

---

### ISSUE 3: Inconsistent Error Handling Between `replay_explain` and `replay_explain_for_run`

**Location**: Lines 223–248 vs 260–283

Both functions:
1. Call `journal.run_headers()`
2. Iterate and fetch events
3. Call `build_run_timeline`

The code is copy-pasted with minor variations. Should be unified.

---

## 4. DDD Assessment

### What This Module Gets Right

1. **Pure status derivation**: `derive_status_from_events` is a pure function with no side effects
2. **Value objects**: `DerivedStatus` is a proper tagged union enum (not a boolean field)
3. **Error domain**: `StatusError` is a proper error enum with typed variants
4. **Documentation**: Each public item has doc comments

### What Violates DDD

1. **Primitive obsession in `ReplayExplainEntry`** — `step` and `action` should be typed IDs
2. **Primitive obsession in error contexts** — `String` instead of structured error details
3. **Anemic intermediate structures** — `ReplayExplainEntry` is a pure data bag with no behavior

---

## 5. Refactoring Roadmap

| Priority | Action | Impact |
|----------|--------|--------|
| **P0** | Split file into `status/derive.rs`, `status/replay.rs`, `status/explain.rs`, `status/test.rs` | Reduces to <300 lines per module |
| **P0** | Fix `ReplayExplainEntry`: `step: Option<u16>` → `step: Option<StepIdx>` | Eliminates primitive obsession |
| **P0** | Fix `ReplayExplainEntry`: `action: Option<u16>` → `action: Option<ActionId>` | Eliminates primitive obsession |
| **P1** | Fix `ActionScheduledTicket` arm to use typed IDs directly | Consistent type usage |
| **P1** | Replace `StatusError::Inconsistency { reason: String }` with typed `InconsistencyKind` | Stronger error domain |
| **P2** | Extract `explain_event_type()` helper | Reduces match arm boilerplate |
| **P2** | Merge duplicate `run_headers()` calls in `replay_explain` / `replay_explain_for_run` | DRY |

---

## 6. Verification Plan

After refactoring:

- [ ] All modules under 300 lines
- [ ] `cargo check --all-features` passes
- [ ] `cargo clippy --all-features` passes with zero warnings
- [ ] `cargo test` passes
- [ ] `cargo miri test` passes (no UB)
- [ ] `moon ci` passes

---

## Conclusion

**DECISION**: This file MUST be refactored before landing any feature work.

**Files to create**:
- `crates/vb_cli/src/status/mod.rs` (reexports)
- `crates/vb_cli/src/status/derive.rs` (~150 lines)
- `crates/vb_cli/src/status/replay.rs` (~150 lines)  
- `crates/vb_cli/src/status/explain.rs` (~150 lines)
- `crates/vb_cli/src/status/test.rs` (~103 lines, keep inline tests or move to integration test)

**Files to delete**:
- `crates/vb_cli/src/status.rs` (replaced by module)

---
*Report generated by arch-drift-hammer*
