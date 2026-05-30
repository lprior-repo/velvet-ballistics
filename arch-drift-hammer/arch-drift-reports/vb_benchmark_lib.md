# Architectural Drift Report: `vb_benchmark/src/lib.rs`

## File Overview
- **Path**: `crates/vb_benchmark/src/lib.rs`
- **Total Lines**: 933
- **Status**: VIOLATION (exceeds 300-line limit)

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 933 | 300 | ❌ OVER by 633 lines |

---

## 2. DDD Cohesion Analysis

### Cohesion Score: **LOW**

This module suffers from ** shotgun surgery** anti-pattern. It attempts to serve multiple unrelated bounded contexts:

| Error Type | Bounded Context | Concern |
|------------|-----------------|---------|
| `EvidenceError` | Evidence Gate | Benchmark validation |
| `YamlBenchmarkError` | YAML Workflow | Parse/validation |
| `StorageBenchmarkError` | Storage | Journal I/O |
| `IpcBenchmarkError` | IPC | Frame encode/decode |
| `RecoveryBenchmarkError` | Recovery | Hydration |
| `RuntimeBenchmarkError` | Runtime | Step/primitive eval |

**Cohesion Violation**: These 6 error enums share no common domain behavior. They should reside in their respective feature modules, not a shared `lib.rs`.

### Single Responsibility Principle Violation
The module mixes:
- Data structures (`BenchmarkMetadata`)
- Validation logic (`capture_metadata`, `check_evidence_gate`)
- Calculation utilities (`budget_utilization_percent`, `result_exceeds_threshold`)
- Error types for 6 distinct domains

---

## 3. DDD Violations

### 3.1 Primitive Obsession (HIGH PRIORITY)

Raw types used directly instead of NewTypes:

| Field | Type | Issue |
|-------|------|-------|
| `name` | `String` | Should be `BenchmarkName` |
| `command` | `String` | Should be `Command` |
| `commit_hash` | `String` | Should be `CommitHash` (validated hex) |
| `environment` | `String` | Should be `Environment` |
| `baseline_us` | `Option<u64>` | Should be `BaselineMicros` |
| `result_us` | `u64` | Should be `ResultMicros` |
| `budget_us` | `u64` | Should be `BudgetMicros` |

### 3.2 Missing Value Objects

- `CommitHash` should validate ASCII hex format (already validated in `capture_metadata` but not as a type)
- `ThresholdPercent` should wrap `u64` with validation (0-100 range)

### 3.3 Anemic Domain Model

`BenchmarkMetadata` is a pure data bag with no behavior. Domain logic (`capture_metadata`, `check_evidence_gate`) lives in functions rather than methods on the struct.

### 3.4 Anomalous Error Taxonomy

Error types for unrelated domains (`YamlBenchmarkError`, `StorageBenchmarkError`, etc.) are defined here but likely used elsewhere. This creates implicit coupling between crates.

---

## 4. Structural Violations

| Violation | Severity | Location |
|-----------|----------|----------|
| File exceeds 300 lines | CRITICAL | Entire file |
| Multiple bounded contexts | HIGH | lib.rs (lines 79-189) |
| Primitive obsession | HIGH | `BenchmarkMetadata` struct |
| Anemic domain model | MEDIUM | `BenchmarkMetadata` |
| Tests inline with production | MEDIUM | `#[cfg(test)]` module |

---

## 5. Recommendations

### Immediate (CRITICAL)
1. **Split this file** into at minimum:
   - `src/types.rs` - `BenchmarkMetadata` and value objects
   - `src/errors.rs` - Error types (but consider moving domain-specific errors to their crates)
   - `src/evidence.rs` - Evidence gate logic
   - `src/validation.rs` - Budget/threshold calculations
   - `src/lib.rs` - Re-exports only

### Short-term (HIGH)
2. **Create NewTypes** for all primitive-wrapped fields
3. **Move domain-specific errors** to their respective feature crates
4. **Extract tests** to `tests/` integration files or `src/*_tests.rs`

### Medium-term (MEDIUM)
5. **Model `BenchmarkMetadata` as an aggregate** with methods for validation
6. **Consider a `BenchmarkRun` entity** with state transitions (Pending → Running → Completed/Failed)

---

## 6. Summary

| Metric | Value |
|--------|-------|
| **Lines** | 933 (❌ 633 over limit) |
| **Violations** | 5 critical/high |
| **DDD Smell** | Shotgun surgery, Primitive obsession, Anemic model |
| **Priority** | **P0 - CRITICAL** (must split before any feature work) |

---

*Report generated: 2026-05-29*
*Analyzer: architectural-drift skill*
