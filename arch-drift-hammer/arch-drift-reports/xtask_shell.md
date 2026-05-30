# Architectural Drift Report: `xtask/src/shell.rs`

## File Summary
| Attribute | Value |
|-----------|-------|
| **File** | `xtask/src/shell.rs` |
| **Total Lines** | 80 |
| **Line Limit** | 300 |
| **Status** | ✅ WITHIN LIMIT |

---

## 1. Line Count Check
- **Result**: ✅ PASS
- **Lines**: 80 / 300 max
- **Margin**: 220 lines remaining

---

## 2. DDD Cohesion Analysis

### Cohesion Score: **HIGH** (Acceptable for xtask/shell layer)

The file exhibits **functional cohesion** — all functions serve a single purpose: providing shell-output utilities for the xtask build tool.

| Function | Purpose | Domain Role |
|----------|---------|-------------|
| `render_top_level_help()` | Renders CLI help text | Presentation |
| `render_top_level_version()` | Renders version string | Presentation |
| `run_required_command()` | Executes required command family | Orchestration |
| `exit_with_xtask_error()` | Handles error output | Error Presentation |
| `normalized_args()` | Filters CLI arguments | Input Normalization |
| `write_stdout()` | Writes formatted output to stdout | I/O Wrapper |

### Cohesion Assessment
This is a **thin shell abstraction layer** for xtask automation. DDD modeling is not applicable here as this is infrastructure code (build tooling), not domain code.

---

## 3. Violations

### Violation 1: Magic Number
**Location**: `normalized_args()` line 64
```rust
let is_legacy_separator = index == 1 && arg == "--";
```
**Issue**: The literal `1` is a magic number representing argument position.
**Severity**: MINOR (cosmetic, non-domain)
**Remediation**: Define `const LEGACY_SEPARATOR_INDEX: usize = 1;`

### Violation 2: Primitive Obsession (Argument Position)
**Location**: `normalized_args()` line 64
**Issue**: Using raw `usize` index instead of a named concept for argument position.
**Severity**: MINOR
**Remediation**: Already covered by Violation 1's fix.

### Violation 3: Hardcoded Exit Code
**Location**: `exit_with_xtask_error()` line 57
```rust
std::process::exit(2);
```
**Issue**: Exit code `2` is hardcoded. While conventional for xtask errors, a named constant improves readability.
**Severity**: MINOR (idiomatic for xtask)
**Remediation**: Define `const XTASK_ERROR_EXIT_CODE: i32 = 2;`

---

## 4. DDD Smell Assessment

| Smell | Present | Notes |
|-------|---------|-------|
| Primitive Obsession | ⚠️ Minor | Magic number `1` for arg index |
| Anemic Domain Model | ❌ N/A | Not a domain file (xtask/shell) |
| Feature Envy | ❌ None | Functions only access their own data |
| Long Method | ❌ None | Longest: 17 lines (`normalized_args`) |
| Shotgun Surgery | ❌ None | Single file, focused purpose |
| Parallel Inheritance | ❌ None | No hierarchies present |

---

## 5. Priority Assessment

| Category | Priority | Rationale |
|----------|----------|-----------|
| **Line Count** | ✅ NONE | Within limit |
| **DDD Violations** | **LOW** | Minor magic numbers only |
| **Refactor Urgency** | **OPTIONAL** | Cosmetic improvements only |

---

## 6. Recommendations

1. **Optional**: Add `const LEGACY_SEPARATOR_INDEX: usize = 1;` for self-documentation
2. **Optional**: Add `const XTASK_ERROR_EXIT_CODE: i32 = 2;` for clarity
3. **No structural changes needed** — file serves its purpose well as a thin shell abstraction

---

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║  STATUS: PERFECT                                              ║
║  Lines: 80/300  │  DDD Smell: LOW  │  Priority: NONE        ║
╚══════════════════════════════════════════════════════════════╝
```

This file is architecturally sound for its role as an xtask shell-output utility. The minor violations are cosmetic and do not impact maintainability or correctness.
