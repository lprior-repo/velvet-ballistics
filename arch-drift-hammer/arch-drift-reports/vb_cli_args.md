# Architectural Drift Report: `vb_cli/src/args.rs`

**File**: `crates/vb_cli/src/args.rs`  
**Analysis Date**: 2026-05-29  
**Status**: `SEVERE DRIFT — REFACTOR REQUIRED`

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **2969** | 300 | ❌ **EXCEEDED BY 2669 LINES** |

---

## 2. DDD Cohesion Smell

**Grade: F (Unacceptable)**

### Cohesion Analysis

This file is a **God File** containing multiple DDD anti-patterns:

| Concern | Lines | Domain Concept |
|---------|-------|----------------|
| Value objects/enums | 1–335 | Domain types |
| Command enum + variants | 67–216 | Domain workflow |
| ParseError enum | 289–316 | Error model |
| Parsing functions | 336–1538 | Application layer |
| Flag validation | 1543–1702 | Application layer |
| ParseError Display impl | 1704–1803 | Error presentation |
| Inline tests | 1805–2969 | Test code (1165 lines!) |

### Cross-Domain Violation

**Line 886**: `vb_runtime::shard::ShardConfig::default()`  
The CLI argument parser **imports and calls runtime internals**. This is a hard boundary violation — the `vb_cli` crate should not know about `vb_runtime` internals.

---

## 3. Complete Violations List

### Critical (Must Fix)

1. **LINE COUNT EXCEEDED** (2969 > 300)
   - Severity: SEVERE
   - Impact: File is 10x over the limit

2. **Cross-domain dependency** (Line 886)
   - `vb_runtime::shard::ShardConfig` imported in `validate_status_options()`
   - Violates hexagonal architecture: CLI must not couple to runtime internals
   - Fix: Remove validation that depends on runtime config, or pass limits as arguments

3. **Inline test module** (Lines 1805–2969)
   - 1165 lines of tests inside production source
   - Violates `tests/` placement rule per workspace convention
   - Fix: Move to `crates/vb_cli/tests/args_tests.rs` or similar

### Major (Strongly Recommended)

4. **Primitive Obsession — `run_id: String`**
   - Lines: 75, 132, 137, 143, 148, 154, 159, 172, 177, 194, 210
   - Should be `RunId = String` newtype
   - "Parse, don't validate" is not honored — run_ids are stored as raw strings

5. **Primitive Obsession — `step: u16`**
   - Lines: 92, 179
   - Should be `StepId = u16` newtype

6. **Primitive Obsession — `action_id: u16`**
   - Line: 92
   - Should be `ActionId = u16` newtype

7. **Too Many Command Variants**
   - `Command` enum has **25 variants** (Lines 68–216)
   - This is a maintenance hazard
   - Consider command groups or subcommands as enum variants

8. **ParseError Enum Bloat**
   - 27 error variants (Lines 289–316)
   - Many are stringly-typed
   - Consider consolidating with context-rich errors

### Moderate

9. **FlagSpec Enum Proliferation** (Lines 319–322)
   - `FlagSpec` is an internal optimization, acceptable

10. **Duplicated Parse State Structs**
    - `ActionListParseState` (Lines 324–328) and `ActionInspectParseState` (Lines 330–334)
    - Nearly identical, suggest generic `ActionParseState<T>`

11. **Duplicate `as_str()` Implementations**
    - `VerifyProfile::as_str()` (Lines 35–41)
    - `EventStatus::as_str()` (Lines 56–65)
    - `DurabilityMode::as_str()` (Lines 271–277)
    - Suggest trait implementation or macro

---

## 4. File Structure Recommendations

```
crates/vb_cli/src/
├── args/
│   ├── mod.rs           # Re-exports
│   ├── types.rs         # Value objects (OutputFormat, VerifyProfile, etc.)
│   ├── command.rs       # Command enum
│   ├── error.rs         # ParseError + Display
│   ├── parse.rs         # parse_args + entry point
│   ├── parse_status.rs  # Status/status-system parsing
│   ├── parse_action.rs # Action list/inspect parsing
│   ├── parse_trace.rs  # Trace parsing
│   ├── parse_run.rs    # Run/run-compiled/submit parsing
│   ├── parse_other.rs   # Remaining commands
│   └── flags.rs         # FlagSpec, known_flag_spec, helpers
└── main.rs
```

**After split**: Each file should be < 300 lines.

---

## 5. Remediation Priority

| Priority | Violation | Effort | Risk |
|----------|-----------|--------|------|
| **P0** | Line count (2969 → <300) | High | High |
| **P0** | Cross-domain `ShardConfig` dep | Medium | High |
| **P1** | Move inline tests out | Medium | Low |
| **P1** | `RunId`/`StepId`/`ActionId` newtypes | Low | Low |
| **P2** | Command enum reduction | High | Medium |
| **P2** | ParseError consolidation | Medium | Low |

---

## 6. Summary

```yaml
lines_count: 2969
limit: 300
violations: 11
ddr_grade: F
cohesion: God File
remediation_priority: CRITICAL
```

**Recommendation**: Split into ~10 files per DDD bounded context. Extract tests immediately. Remove `ShardConfig` dependency via trait injection or configuration passing.
