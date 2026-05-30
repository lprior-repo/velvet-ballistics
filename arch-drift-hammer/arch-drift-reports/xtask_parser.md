# Architectural Drift Report: `xtask/src/parser.rs`

## File: `xtask/src/parser.rs`

### Line Count
| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 164 | ✅ PASS (< 300) |
| Code Lines | ~140 | ✅ |
| Test Lines | ~24 | ✅ |

---

## DDD Cohesion Analysis

### Domain Elements
| Element | Role | Assessment |
|---------|------|------------|
| `XtaskCommand` | Core domain enum (value object) | ✅ Clean representation |
| `ParsedCommandName<'a>` | Newtype wrapper for command strings | ✅ Good DDD practice |
| `parse_xtask_command` | Entry point / workflow orchestrator | ✅ |
| `collect_args` | Data transformation | ✅ |
| `classify_top_level_command` | State classification | ✅ |
| `parse_required_command` | Command resolution | ✅ |
| `parse_legacy` | Legacy command routing | ⚠️ Primitive obsession smell |

### Cohesion Verdict
**COHESION: ACCEPTABLE** — The module has a single clear responsibility (parsing xtask commands). The functions flow logically from entry point to classification to parsing.

---

## Violations

### 1. Parse-Validate Confusion (Medium Priority)
**Location**: `validate_bead_option` (lines 98-118), `validate_format_option` (lines 144-164)

**Problem**: These functions are named as validators but perform parsing (extracting values from token iterators). This violates the "Parse, don't validate" principle — they should be integrated into the parsing flow, not separate post-parsing validators.

**Current behavior**: Manually iterate tokens, extract values, then validate extracted values.

**DDD Impact**: The validation logic is decoupled from the parsing logic, making the workflow harder to reason about.

**Recommendation**: Rename to `parse_bead_option` / `parse_format_option` and integrate validation into the parse step using `Result` return type properly.

### 2. Primitive Obsession in Legacy Commands (Low Priority)
**Location**: `parse_legacy` (lines 71-91)

**Problem**: Legacy command names are matched as raw `&'static str` literals. If legacy commands grow, this becomes harder to maintain.

**DDD Impact**: Low — legacy commands are explicitly transitional and will be deprecated.

**Recommendation**: Consider a `LegacyCommand` newtype wrapper if legacy commands proliferate.

---

## Summary

| Category | Status |
|----------|--------|
| Line Count | ✅ PASS |
| DDD Cohesion | ✅ ACCEPTABLE |
| Parse-Validate Principle | ⚠️ VIOLATION (Medium) |
| Primitive Obsession | ⚠️ MINOR SMELL (Low) |

---

## Priority

**MEDIUM** — The file itself is well-structured and under 300 lines. The parse-validate confusion is a design smell but not architecturally critical. No immediate refactoring required unless the validation logic needs to change.

---

## Recommendation

**STATUS: PERFECT** — File meets all structural requirements. The parse-validate smell is a stylistic concern rather than architectural drift. Monitor if `validate_bead_option` or `validate_format_option` grow in complexity.
