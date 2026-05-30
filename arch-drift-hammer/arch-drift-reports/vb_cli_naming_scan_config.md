# Architectural Drift Report: `vb_cli::naming_scan::config`

## File
- **Path**: `crates/vb_cli/src/naming_scan/config.rs`
- **Total Lines**: 351

## Line Count Violation
| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Line count | 351 | 300 | **VIOLATION** (+51 lines) |

## DDD Cohesion Analysis

### Responsibility Classification
| Function | Responsibility | DDD Concept |
|----------|-----------------|-------------|
| `canonical_spelling_table()` | Value object construction | Domain Model |
| `validate_scan_config()` | Workflow/Application Service | Application Service |
| `invalid_config()` | Error factory | Error Handling |
| `validate_patterns()` | Validation rule | Validation |
| `validate_allowlist()` | Validation rule | Validation |
| `table_from_entries()` | Transformation | Application Service |
| `validate_entry()` | Validation rule | Validation |
| `duplicate_error()` | Error factory | Error Handling |
| `missing_error()` | Error factory | Error Handling |
| `required_kinds()` | Domain constant | Domain Model |
| `kind_name()` | Display conversion | Domain Model |
| `expected_token()` | Mapping function | Domain Model |
| `fingerprint_for_destination()` | Fingerprinting | Infrastructure |

### Cohesion Score: **LOW**
The module mixes **three unrelated concerns**:
1. **Domain constants and types** (`canonical_spelling_table`, `required_kinds`, `kind_name`, `expected_token`)
2. **Validation logic** (`validate_patterns`, `validate_allowlist`, `validate_entry`, `table_from_entries`)
3. **Error construction** (`invalid_config`, `duplicate_error`, `missing_error`)

Tests (lines 164-351) constitute **188 lines (53%)** of the file, further fragmenting cohesion.

## Violations

### 1. LINE COUNT EXCEEDED
- **Severity**: CRITICAL
- **Rule**: `max_300_lines_per_file`
- **Required**: Split into ≤300 line files

### 2. Low DDD Cohesion (Bloaters / Large Class)
- **Severity**: HIGH
- **Rule**: `single_responsibility_cohesion`
- **Smell**: Too many unrelated responsibilities in one module

### 3. Test Code Polluting Production Module
- **Severity**: MEDIUM
- **Observation**: 188 of 351 lines (53%) are tests — this is a `tests` module hiding inside production code

## DDD Smell Summary
| Smell | Category | Lines Affected |
|-------|----------|----------------|
| Large Class | Bloaters | Entire file |
| Mixed Responsibilities | Cohesion | 1-162 |
| Test Pollution | Structure | 164-351 |

## Recommended Split

```
naming_scan/
├── config.rs          (→ keep: core config logic, 1-163)
├── config_errors.rs   (new: error types/factories)
├── config_validation.rs (new: validation functions)
└── config_tests.rs    (move: all #[cfg(test)] code)
```

## Priority Assessment
- **Priority**: **HIGH**
- **Rationale**: Line count violation is a hard architectural constraint. Low cohesion degrades maintainability.
- **Effort**: Medium (move ~50 lines to new files, update module structure)

## Status
**ARCH-DRIFT-DETECTED**
