# ARCHITECTURAL DRIFT REPORT
## Target: `crates/vb_cli/src/commands_journal.rs`
## Severity: CRITICAL — 1,157 lines (285% over 300-line hard cap)

---

## EXECUTIVE SUMMARY

| Category | Finding | Severity |
|---|---|---|
| **Line Count** | 1,157 lines (limit: 300) | 🔴 CRITICAL |
| **Primitive Obsession** | Raw `u16`/`u64` in public structs, stringly-typed `event_type` | 🔴 CRITICAL |
| **Single Responsibility** | 18-variant match in `trace_one` (~200 lines) | 🔴 CRITICAL |
| **DDD Cohesion** | Trace domain conflated with Retry/Resume analysis in one file | 🟡 HIGH |
| **Test Isolation** | 720-line inline test module violates pure/beast separation | 🟡 HIGH |

---

## 1. LINE COUNT VIOLATION

**Hard cap: 300 lines. Actual: 1,157 lines. Overflow: 857 lines (285%).**

### Breakdown by Concern

| Region | Lines |占比 | Concern |
|---|---|---|---|
| `trace_one` function alone | ~210 | 18% | Single function 70% over the entire file limit |
| Inline `#[cfg(test)]` module | ~722 | 62% | Tests embedded in production module |
| Production types + logic | ~225 | 19% | Core domain logic |
| **Total** | **1,157** | **100%** | |

**Verdict: File MUST be split. No exceptions.**

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `u16` for Step/Action Indices

`TraceEntry` (lines 14-24) exposes raw primitives:

```rust
pub(crate) struct TraceEntry {
    pub index: usize,
    pub event_type: &'static str,   // ← VIOLATION: stringly-typed
    pub step: Option<u16>,          // ← VIOLATION: raw u16
    pub status: Option<TraceStatus>,
    pub action: Option<u16>,        // ← VIOLATION: raw u16
    pub seq: u64,                   // ← VIOLATION: raw u64
    pub extra_json: Vec<(&'static str, serde_json::Value)>, // ← VIOLATION: unstructured bag
}
```

**Correct types exist in the codebase and ARE used in tests:**
- `vb_core::ids::StepIdx` — wraps `u16`
- `vb_core::ids::ActionId` — wraps `u16`
- `vb_storage::types::EventSeq` — wraps `u64`

Yet `TraceEntry` reverts to raw primitives, defeating the type system's guarantees.

### 2.2 Raw `u16` in `TraceFilters`

```rust
pub(crate) struct TraceFilters {
    pub(crate) step: Option<u16>,       // ← Should be `Option<StepIdx>`
    pub(crate) action: Option<u16>,      // ← Should be `Option<ActionId>`
    pub(crate) status: Option<TraceStatus>,
    pub(crate) since_seq: Option<u64>,  // ← Should be `Option<EventSeq>`
    pub(crate) until_seq: Option<u64>,  // ← Should be `Option<EventSeq>`
    pub(crate) limit: Option<usize>,
}
```

### 2.3 Raw `u16` in Analysis Results

```rust
pub(crate) struct RetryAnalysis {
    pub failed_at_step: Option<u16>,        // ← Should be `Option<StepIdx>`
    pub last_successful_step: Option<u16>,   // ← Should be `Option<StepIdx>`
    pub can_retry: bool,
    pub reason: String,
}

pub(crate) struct ResumeAnalysis {
    pub suspended_at_step: Option<u16>,      // ← Should be `Option<StepIdx>`
    pub can_resume: bool,
    pub reason: String,
}
```

### 2.4 Stringly-Typed `event_type`

`event_type: &'static str` — event classification uses string literals instead of a proper enum:

```rust
// VIOLATION: "RunAccepted", "StepStarted", etc. are untyped strings
pub event_type: &'static str,
```

**Fix:** Introduce `enum TraceEventVariant { RunAccepted, RunAdmission, StepStarted, ... }` and derive `as_str()` for display formatting.

### 2.5 Unstructured `extra_json` Bag

```rust
pub extra_json: Vec<(&'static str, serde_json::Value)>,
```

This is an untyped key-value bag. Each event variant has different fields — this defeats static analysis and makes it impossible to enforce completeness.

**Preferred:** Per-variant result structs or a typed `TraceExtra` enum with variant-specific fields.

---

## 3. SINGLE RESPONSIBILITY VIOLATION

### 3.1 `trace_one` — 18-Arm Match Expression (~210 lines)

The entire `trace_one` function (lines 100-311) is a single match against 18 `JournalEvent` variants. Every variant maps to a `TraceEntry`. This is a textbook God Function.

**Symptoms:**
- Adding a new `JournalEvent` variant requires editing this 210-line function
- No isolation between event-to-trace mapping rules
- Impossible to test mapping logic for one variant in isolation without the full function

**Scott Wlaschin DDD alignment:** In proper DDD, each event type would have its own mapping/value construction, organized by the aggregate or entity it belongs to.

### 3.2 `analyze_retry` / `analyze_resume` — Conflated Scanning

Both `analyze_retry` and `analyze_resume` perform linear scans over the same event list with similar-but-separate `for event in events` loops. This is duplicated iteration logic.

**Fix:** Introduce a `JournalScanner` trait or a single `scan_journal` function that extracts key facts in one pass, then derive retry/resume analysis from the scanned state.

---

## 4. DDD COHESION VIOLATIONS

### 4.1 Three Domains, One File

The file mixes three distinct domain concerns:

| Domain | Responsibility | Entry Points |
|---|---|---|
| **Trace** | Build and filter trace entries from events | `build_trace`, `filter_trace`, `trace_one` |
| **Retry Analysis** | Determine if a failed run can retry | `analyze_retry` |
| **Resume Analysis** | Determine if a suspended run can resume | `analyze_resume` |

These should be separate modules (or even separate crates for严格执行 DDD).

### 4.2 Inline Test Module (~722 lines)

The `#[cfg(test)]` module contains 720 lines of tests — 2.4x the entire 300-line budget for the **entire file**, production + tests. Tests are a first-class citizen and should have their own file/folder under `tests/` or a sibling module.

**Rule:** A file that exceeds 300 lines because of its inline test module is a code smell — the module itself is too large, and the test module should be extracted.

---

## 5. REQUIRED REFACTORS (Priority Order)

### P0 — Must Fix (Blocking)

1. **Split the file immediately.** Suggested structure:
   ```
   src/
     commands_journal.rs          # Re-exports, stays under 300 lines
     trace/
       mod.rs                     # TraceEntry, TraceStatus, TraceFilters, build_trace, filter_trace
       trace_map.rs               # trace_one — extract per-variant mapping to separate functions
     retry/
       mod.rs                     # RetryAnalysis, analyze_retry
     resume/
       mod.rs                     # ResumeAnalysis, analyze_resume
   ```
   **Target: Each file ≤ 300 lines.**

2. **Replace raw primitives in public structs.** Use existing `StepIdx`, `ActionId`, `EventSeq` types throughout `TraceEntry`, `TraceFilters`, `RetryAnalysis`, `ResumeAnalysis`.

3. **Replace `&'static str` event_type with `enum TraceVariant`.**

### P1 — Should Fix

4. **Extract inline tests to `tests/commands_journal_tests.rs`** or a sibling `commands_journal_test` module at package level.

5. **Eliminate `extra_json: Vec<(&'static str, serde_json::Value)>`.** Replace with typed per-variant extra structs or a closed enum.

6. **Deduplicate journal scanning.** `analyze_retry` and `analyze_resume` both iterate the full event list. Extract common scan logic.

---

## 6. THREAT MODEL

| Risk | Likelihood | Impact |
|---|---|---|
| Adding new `JournalEvent` variant breaks trace mapping | HIGH (no compile enforcement) | HIGH |
| Silent type coercion errors from raw u16/u64 | MEDIUM | HIGH |
| `extra_json` bag causes runtime missing-field errors | MEDIUM | MEDIUM |
| God function `trace_one` merge conflicts on parallel work | HIGH | HIGH |

---

## 7. VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

The file is 285% over the hard 300-line cap and violates Scott Wlaschin DDD at multiple levels:

- **Primitive obsession** in every public struct (raw `u16`/`u64`/`str` where typed value objects exist)
- **Single Responsibility** violated by `trace_one` handling 18 variants in one 210-line function
- **DDD Cohesion** violated by three unrelated domain concerns in one file
- **Test isolation** violated by 720-line inline test module

**This file cannot be merged until split and primitives replaced. The existence of correct types (`StepIdx`, `ActionId`, `EventSeq`) in the codebase that are deliberately NOT used in public structs is a红灯 — it means the types exist and were considered, but the implementation regressed.**

---

*Report generated by arch-drift-hammer agent. Workspace: `arch-drift-hammer` JJ branch.*
