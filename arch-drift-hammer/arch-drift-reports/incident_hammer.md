# Architectural Drift Report: `incident.rs`

**File:** `crates/vb_storage/src/journal/incident.rs`  
**Line Count:** 412 lines (EXCEEDS 300-line limit by 112 lines)  
**Classification:** CRITICAL DRIFT - REQUIRES IMMEDIATE REFACTOR

---

## Executive Summary

This file violates both the <300 line rule and Scott Wlaschin DDD principles through systematic primitive obsession. The domain types `StepIdx` and `ActionId` (defined in `vb_core/src/ids/mod.rs:55-58`) are correctly used in `JournalEvent`, but are unwrapped to raw `u16` in incident analysis, breaking type-level guarantees and semantic coherence.

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 412 | 300 | **+112 over** |
| Test code lines | 238 (lines 173-412) | - | ~58% test code |

The file is 37% over the line limit. The test module alone (238 lines) is a smell indicating the production code should be split into smaller, more focused modules.

---

## 2. Primitive Obsession Violations

### 2.1 `SideEffect` Value Object (Lines 10-16)

**Current (VIOLATION):**
```rust
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: u16,      // RAW - should be StepIdx
    pub action: u16,    // RAW - should be ActionId
    pub certainty: SideEffectCertainty,
}
```

**Required Domain Model:**
```rust
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: StepIdx,      // Domain newtype preserves semantics
    pub action: ActionId,   // Domain newtype preserves semantics
    pub certainty: SideEffectOutcome, // Renamed for domain accuracy
}
```

**Violation Severity:** CRITICAL  
**Risk:** Raw `u16` values lose semantic context. Any `u16` can be passed where a step or action is expected with zero type safety.

---

### 2.2 `IncidentAnalysis` Struct (Lines 26-32)

**Current (VIOLATION):**
```rust
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: String,              // STRINGLY TYPED
    pub failed_at_step: Option<u16>,       // RAW u16, not StepIdx
    pub side_effects: Vec<SideEffect>,     // Untyped collection
}
```

**Required Domain Model:**
```rust
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: FailureCode,         // Domain enum
    pub failed_at_step: Option<StepIdx>,   // Preserved domain type
    pub side_effects: SideEffectLog,       // Typed collection wrapper
}

// FailureCode enum - makes illegal states unrepresentable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    RunFailed,
    RunCancelled,
    Unknown,  // For codes we don't recognize
}

// SideEffectLog - domain collection with invariants
#[derive(Debug, Clone, Default)]
pub struct SideEffectLog(Vec<SideEffect>);

impl SideEffectLog {
    pub fn iter(&self) -> impl Iterator<Item = &SideEffect> { self.0.iter() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn len(&self) -> usize { self.0.len() }
    // Domain methods: confirmed(), failed(), at_step(), etc.
}
```

**Violation Severity:** CRITICAL  
**Risk:** `String` for `failure_code` allows arbitrary strings like "RunFaled" (typo) or "Foobar". The type system cannot enforce valid failure codes.

---

### 2.3 `analyze_incident_events` Function (Lines 35-81)

**Current (VIOLATION):**
```rust
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    // ...
    last_step_started = Some(step.get());  // UNWRAPPING StepIdx -> u16
    // ...
    side_effects.push(SideEffect {
        step: step.get(),      // .get() on StepIdx
        action: action.get(),  // .get() on ActionId
        // ...
    });
    // ...
    failure_code = "RunFailed".to_string();  // STRINGLY TYPED
```

**Correct Approach:**
The function should preserve domain types throughout:
```rust
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut failure_code = FailureCode::Unknown;
    let mut failed_at_step: Option<StepIdx> = None;
    let mut side_effects = SideEffectLog::default();

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                failed_at_step = Some(*step);
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: *step,
                    action: *action,
                    certainty: SideEffectOutcome::Confirmed,
                });
            }
            JournalEvent::RunFailedEvent { .. } => {
                failure_code = FailureCode::RunFailed;
            }
            // ...
        }
    }
    IncidentAnalysis { failure_found, failure_code, failed_at_step, side_effects }
}
```

**Violation Severity:** CRITICAL  
**Risk:** The `.get()` calls throughout this function are leaky abstractions that discard domain semantics at every call site.

---

### 2.4 `build_repair_hints` Function (Lines 84-116)

**Current (VIOLATION):**
```rust
pub fn build_repair_hints(
    failure_code: &str,           // STRINGLY TYPED
    side_effects: &[SideEffect],   // Untyped slice
    failed_at_step: Option<u16>,  // RAW u16
) -> Vec<String> {                // STRING COLLECTION
```

**Required Domain Model:**
```rust
pub fn build_repair_hints(
    failure_code: FailureCode,
    side_effects: &SideEffectLog,
    failed_at_step: Option<StepIdx>,
) -> RepairPlan {  // Value object, not raw strings
}

#[derive(Debug, Clone, Default)]
pub struct RepairPlan {
    hints: Vec<RepairHint>,
}

#[derive(Debug, Clone)]
pub enum RepairHint {
    InvestigateStepOutput,
    ReviewSideEffects,
    RetryFromStep(StepIdx),
    CheckCancellationIntent,
    ReviewCleanupNeeds,
}
```

**Violation Severity:** HIGH  
**Risk:** Returning `Vec<String>` means hints are just bags of bytes. The caller must parse strings to understand what happened. A domain `RepairPlan` with typed `RepairHint` variants enables programmatic response.

---

### 2.5 Test Code Primitive Obsession (Lines 173-412)

The test helpers and test cases use raw `u16` values:
```rust
fn step_event(step: u16) -> JournalEvent {  // Takes u16, not StepIdx
    JournalEvent::StepStarted {
        step: StepIdx::new(step),  // Must wrap
        // ...
    }
}
```

This pattern forces every caller to do the wrapping, instead of having `StepIdx` be the natural type everywhere.

---

## 3. Scott Wlaschin DDD Violations

### 3.1 "Make Illegal States Unrepresentable"

**Current:** `failure_code: String` allows any Unicode string. There's no way to enumerate valid failure codes at the type level.

**Correct:** `failure_code: FailureCode` enum limits values to known variants. The compiler enforces exhaustiveness.

### 3.2 "Types Over Conventions"

**Current:** Conventions like `side_effects.is_empty()` check array length.

**Correct:** `side_effects.is_empty()` should be a method on `SideEffectLog` that encapsulates the internal representation.

### 3.3 "Domain Types Should Be First-Class"

**Current:** The domain knows about `StepIdx` and `ActionId` but incident analysis converts them to primitives immediately.

**Correct:** Incident analysis should work with domain types end-to-end.

### 3.4 "Value Objects Should Be Self-Validating"

**Current:** `SideEffect` accepts any `u16` for step/action.

**Correct:** `SideEffect` would use `StepIdx` and `ActionId` which are already validated via their constructors.

---

## 4. Recommended Refactoring Plan

### Phase 1: Introduce Domain Types (New file: `incident/domain.rs`)

```rust
// FailureCode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    RunFailed,
    RunCancelled,
    Unknown,
}

// SideEffectOutcome (renamed from SideEffectCertainty for clarity)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectOutcome {
    Confirmed,
    Failed,
}

// SideEffect value object - uses domain types
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: StepIdx,
    pub action: ActionId,
    pub outcome: SideEffectOutcome,
}

// SideEffectLog collection wrapper
#[derive(Debug, Clone, Default)]
pub struct SideEffectLog(Vec<SideEffect>);

impl SideEffectLog {
    pub fn push(&mut self, effect: SideEffect) { self.0.push(effect); }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn iter(&self) -> impl Iterator<Item = &SideEffect> { self.0.iter() }
    pub fn confirmed(&self) -> impl Iterator<Item = &SideEffect> {
        self.0.iter().filter(|s| s.outcome == SideEffectOutcome::Confirmed)
    }
}

// RepairHint enum
#[derive(Debug, Clone)]
pub enum RepairHint {
    InvestigateStepOutput,
    ReviewSideEffects,
    RetryFromStep(StepIdx),
    CheckCancellationIntent,
    ReviewCleanupNeeds,
}

// RepairPlan value object
#[derive(Debug, Clone, Default)]
pub struct RepairPlan {
    hints: Vec<RepairHint>,
}

impl RepairPlan {
    pub fn is_empty(&self) -> bool { self.hints.is_empty() }
    pub fn len(&self) -> usize { self.hints.len() }
    pub fn iter(&self) -> impl Iterator<Item = &RepairHint> { self.hints.iter() }
}

// IncidentAnalysis - uses all domain types
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: FailureCode,
    pub failed_at_step: Option<StepIdx>,
    pub side_effects: SideEffectLog,
}
```

### Phase 2: Move Functions to Domain Module

- `analyze_incident_events` → `IncidentAnalysis::from_events`
- `build_repair_hints` → `RepairPlan::from_incident`

### Phase 3: Trim File to ≤300 Lines

Expected outcome:
- `incident.rs`: ~100 lines (reexports + thin wrapper)
- `incident/domain.rs`: ~200 lines (all domain types)
- Total: ~300 lines with proper separation

---

## 5. Summary of Violations

| # | Violation | Location | Severity | Lines Affected |
|---|-----------|----------|----------|----------------|
| 1 | Line count exceeds 300 | File | CRITICAL | 412 (112 over) |
| 2 | `step: u16` primitive | SideEffect | CRITICAL | 13 |
| 3 | `action: u16` primitive | SideEffect | CRITICAL | 14 |
| 4 | `failure_code: String` | IncidentAnalysis | CRITICAL | 29 |
| 5 | `failed_at_step: Option<u16>` | IncidentAnalysis | CRITICAL | 30 |
| 6 | `.get()` unwrapping | analyze_incident_events | CRITICAL | 45,49,50,56,57 |
| 7 | Stringly-typed failure codes | analyze_incident_events | CRITICAL | 63,68 |
| 8 | `Vec<String>` return | build_repair_hints | HIGH | 89 |
| 9 | Raw `&str` parameter | build_repair_hints | HIGH | 85 |
| 10 | Untyped collection | IncidentAnalysis | HIGH | 31 |

---

## 6. Verification Commands

After refactoring, these gates must pass:

```bash
# Line count check
wc -l crates/vb_storage/src/journal/incident.rs  # Must be ≤300

# Type-level enforcement (no .get() calls on StepIdx/ActionId in incident module)
grep -n '\.get()' crates/vb_storage/src/journal/incident.rs  # Should return nothing in incident.rs

# No raw u16 for domain concepts
grep -n 'u16.*step\|u16.*action' crates/vb_storage/src/journal/incident.rs  # Should return nothing
```
