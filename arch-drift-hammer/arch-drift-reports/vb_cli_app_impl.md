# Architectural Drift Report: `vb_cli/src/app_impl.rs`

**File**: `crates/vb_cli/src/app_impl.rs`
**Analysis Date**: 2026-05-29
**Status**: CRITICAL DRIFT DETECTED

---

## Executive Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| **Total Lines** | 6296 | 300 | ❌ **21x OVER LIMIT** |
| **DDD Cohesion** | God Module | Single Domain | ❌ **VIOLATED** |
| **Oversized Functions** | 15+ | 0 | ❌ **CRITICAL** |
| **Inline Tests** | 2 modules | 0 | ❌ **VIOLATED** |
| **Module Separation** | Absent | Required | ❌ **VIOLATED** |

---

## 1. Line Count Analysis

**Total Lines**: 6296 lines
**Limit**: 300 lines
**Violation**: 5996 lines over threshold (2099% of limit)

This file is **21x larger** than the maximum allowed size. It should be split into **at least 21 modules** of equal size.

---

## 2. DDD Cohesion Analysis

### ❌ SMELL DETECTED: **God Module / Smart UI Anti-Pattern**

The filename `app_impl.rs` suggests "application implementation," but the file actually contains multiple distinct domain concepts violently mixed together:

| Concept | Lines | Domain |
|---------|-------|--------|
| CLI Dispatch (run_from_env) | ~100 | **CLI Layer** |
| Command Handlers (cmd_*) | ~1500 | **Command Layer** |
| Action Registry Display | ~500 | **Action Domain** |
| Step Execution | ~600 | **Execution Domain** |
| Storage/Journal Operations | ~800 | **Storage Domain** |
| Output/Formatting | ~1000 | **Presentation Layer** |
| Error Handling | ~400 | **Error Domain** |
| Explanation/Validation Messages | ~1500 | **User Guidance Domain** |

### Missing Module Boundaries

The file violates Scott Wlaschin's DDD principles by mixing:
- **CLI entry point** (should be `cli/entry.rs`)
- **Command handlers** (should be `cli/commands/*.rs`)
- **Action registry display** (should be `cli/display/action_registry.rs`)
- **Step execution logic** (should be `cli/commands/step.rs`)
- **Storage helpers** (should be `cli/storage.rs` or `storage/journal.rs`)
- **Output formatting** (should be `cli/display/*.rs`)
- **Error formatting** (should be `cli/error/*.rs`)

---

## 3. Violations Catalog

### 3.1 Oversized Functions (Top 10 by Line Count)

| Function | Lines | Start | Violation |
|----------|-------|-------|-----------|
| `explain_error` | ~300+ | ~3940 | Pattern match arm explosion |
| `explain_compile_repair_hint` | ~280+ | ~4227 | Massive match with 60+ arms |
| `explain_validation_error` | ~270+ | ~4573 | 100+ match arms |
| `explain_verification_failure` | ~100+ | ~4488 | Deep nesting |
| `cmd_verify` | ~140 | ~796 | Too many responsibilities |
| `cmd_explain` | ~120 | ~3763 | Complex multi-phase logic |
| `cmd_compile` | ~160 | ~1051 | Duplicated emit logic |
| `cmd_doctor` | ~280 | ~5518 | Too many diagnostic checks |
| `cmd_submit` | ~160 | ~1347 | Mixed concerns |
| `cmd_run` | ~160 | ~1214 | Duplicated compile logic |

### 3.2 Inline Test Modules

```rust
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mode_activation_tests.rs"]
mod mode_activation_tests;
```

**Violation**: Tests should not be inlined via `#[path = ...]`. They should be in separate test files imported normally.

### 3.3 Missing Module Separation

The following concepts should be extracted into separate modules:

1. **cli/commands/** - All `cmd_*` functions
2. **cli/display/** - Output formatting, JSON/YAML/Postcard encoding
3. **cli/error/** - Error formatting and handling
4. **cli/action_registry.rs** - Action contract display
5. **cli/step.rs** - Step execution logic
6. **cli/storage.rs** - Journal/storage helpers

---

## 4. Specific Code Smells

### 4.1 Duplicated Compilation Logic
- `cmd_run` (line 1214) duplicates compile logic from `cmd_compile` (line 1051)
- `cmd_run_step` (line 1528) duplicates compile logic
- `cmd_run_compiled` (line 2012) duplicates compile logic

### 4.2 God Struct: `ActionContractDetail`
Lines 542-558: Should be in its own module with display logic separated.

### 4.3 God Struct: `ActionTableRow`
Lines 584-593: Simple data struct with no behavior - belongs in domain.

### 4.4 God Struct: `StepStateSnapshots`
Lines 1809-1839: Only used by step execution - should be in `cli/step.rs`.

### 4.5 God Struct: `InputMappingError`
Lines 2085-2099: Belongs in domain error types, not CLI.

### 4.6 Duplicated Output Formatting
- `print_event` (line 2642) and `event_to_json` (line 2733) are duplicated event display logic
- Should be consolidated in `cli/display/events.rs`

### 4.7 Duplicated Journal Opening
`cmd_inspect`, `cmd_events`, `cmd_replay`, `cmd_retry`, `cmd_resume`, `cmd_cancel`, `cmd_diff`, `cmd_incident` all open `FjallJournal` identically.

---

## 5. Remediation Priority

### **P0 - CRITICAL** (Immediate Action Required)

1. **Split this file into at least 20 modules**
   - `cli/commands.rs` - Command dispatch
   - `cli/commands/verify.rs`
   - `cli/commands/validate.rs`
   - `cli/commands/explain.rs`
   - `cli/commands/compile.rs`
   - `cli/commands/run.rs`
   - `cli/commands/submit.rs`
   - `cli/commands/inspect.rs`
   - `cli/commands/events.rs`
   - `cli/commands/replay.rs`
   - `cli/commands/trace.rs`
   - `cli/commands/retry.rs`
   - `cli/commands/resume.rs`
   - `cli/commands/cancel.rs`
   - `cli/commands/incident.rs`
   - `cli/commands/diff.rs`
   - `cli/commands/graph.rs`
   - `cli/commands/simulate.rs`
   - `cli/commands/bench.rs`
   - `cli/commands/doctor.rs`
   - `cli/display/action_registry.rs`
   - `cli/display/output.rs`
   - `cli/storage.rs`

2. **Extract error explanation into `cli/error/explanations.rs`**

3. **Move inline test modules to proper locations**

### Estimated Refactoring Effort
- **Junior**: 2-3 weeks (dangerous, high risk of breaking CLI)
- **Senior**: 1 week with proper test coverage
- **Preferred**: Extract modules incrementally, one at a time, maintaining CI green

---

## 6. Conclusion

This file exhibits **severe architectural drift**:
- Line count is **21x the limit**
- Multiple domain concepts violently mixed
- Massive pattern match explosions for error explanation
- Duplicated logic across command handlers
- Inline test paths instead of proper module organization

**Recommendation**: This file must be split before any new features are added. The current state makes the CLI unmaintainable.

---

*Report generated by architectural-drift agent*
