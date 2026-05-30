# Architectural Drift Report: `vb_ipc/src/metrics.rs`

## File Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 995 |
| **Line Limit** | 300 |
| **Violation Status** | **CRITICAL** |
| **DDD Cohesion** | HIGH (appropriate for IPC metrics DTOs) |
| **Priority** | HIGH |

---

## 1. Line Count Analysis

| Section | Lines | Percentage |
|---------|-------|------------|
| Production code (types + doc comments) | 75 | 7.5% |
| Test module | 920 | 92.5% |
| **Total** | 995 | 100% |

**Verdict**: ❌ **FAILS** — 995 lines exceeds the 300-line limit by **695 lines (232% over)**.

---

## 2. DDD Cohesion Analysis

### Module Purpose
`metrics.rs` provides IPC metrics types for the `vb_ipc` crate — specifically, data transfer objects (DTOs) for runtime telemetry responses.

### Cohesion Score: **HIGH** ✓

| Type | Responsibility | Cohesion |
|------|---------------|----------|
| `RuntimeMetrics` | Top-level metrics container aggregating all sub-metrics | ✓ |
| `ShardMetrics` | Per-shard runtime snapshot (queues, timers, frame pool, trace ring) | ✓ |
| `JournalMetrics` | Journal writer queue and event/run totals | ✓ |
| `IpcMetrics` | IPC client connection and command processing counts | ✓ |
| `AggregateMetrics` | Cross-shard aggregate totals (active/waiting/failed/finished runs) | ✓ |

**Cohesion Assessment**: All 5 types serve a single purpose — IPC metrics serialization. Naming is precise, fields are relevant, and no type leaks responsibility outside the metrics domain.

### DDD Smell Detected

**Smell Type**: `PureDataBundle` (Benign)

This module contains only passive data structures with `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`. No domain logic, no behavior, no invariants. For **IPC boundary types** (DTOs at the edge of a crate), this is **acceptable and even desirable** — boundary crossing objects should be anemic to avoid leaking internal concerns.

However, this anemic design means the module **cannot enforce domain rules** — any validation must occur at the boundary where these types are used.

---

## 3. Violations

### V1: File Size Exceeded — CRITICAL ❌

| Check | Required | Actual | Delta |
|-------|----------|--------|-------|
| Total lines | ≤ 300 | 995 | +695 (+232%) |

**Remediation**: Extract `#[cfg(test)]` module (lines 77–995) into `tests/metrics_tests.rs` within the `vb_ipc` crate. This would reduce `metrics.rs` to **76 lines**.

---

### V2: Test-to-Code Ratio Imbalance — MAJOR ⚠️

| Metric | Value |
|--------|-------|
| Test lines | 920 |
| Production lines | 75 |
| Ratio | **12.3:1** |

**Interpretation**: For every 1 line of production code, there are 12 lines of tests. While metrics serialization warrants thorough testing, embedding 920 lines of tests inline **obscures the production interface** and violates the single-responsibility principle of modules.

**Remediation**: Move tests to `tests/metrics_tests.rs` or `tests/metrics_postcard_tests.rs`.

---

### V3: Mixed Production and Test Concerns — MINOR ⚠️

The file mixes production types (lines 1–76) with a single monolithic `#[cfg(test)]` module (lines 77–995). Standard Rust convention is:
- **Preferred**: Tests in sibling `tests/` directory or `tests/*.rs` files
- **Acceptable**: Inline `#[cfg(test)]` module for **lightweight** tests (< 100 lines)
- **Non-compliant**: Inline `#[cfg(test)]` module exceeding the file's production footprint

**Remediation**: Extract inline tests per V1.

---

## 4. Recommendation

### Immediate Action Required

| Action | Effort | Impact |
|--------|--------|--------|
| Extract `#[cfg(test)]` block to `tests/metrics_postcard_tests.rs` | Low | Reduces file to 76 lines (74% reduction) |

### Post-Refactor Status

| Metric | Pre-Refactor | Post-Refactor |
|--------|--------------|---------------|
| `metrics.rs` lines | 995 | 76 |
| Compliance | ❌ FAIL | ✓ PASS |
| Test coverage | Preserved | Preserved (in separate file) |

### File After Refactor (`metrics.rs`)

```rust
#![forbid(unsafe_code)]
//! IPC metrics types.

use serde::{Deserialize, Serialize};

/// Runtime metrics response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeMetrics { ... }

/// Per-shard metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardMetrics { ... }

/// Journal metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalMetrics { ... }

/// IPC connection metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcMetrics { ... }

/// Aggregate totals across all shards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateMetrics { ... }
```

---

## 5. Priority Classification

| Priority | Rationale |
|----------|-----------|
| **HIGH** | File exceeds size limit by 232%. Test bloat obscures production API. Immediate refactoring required before landing any new work on this module. |

---

**Report Generated**: 2026-05-29  
**Analyzer**: architectural-drift agent  
**File**: `crates/vb_ipc/src/metrics.rs`
