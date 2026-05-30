# Architectural Drift Report: `vb_runtime/src/runtime.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/runtime.rs`  
**Analyzed:** 2026-05-29  
**Status:** CRITICAL DRIFT DETECTED

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| **Total lines** | **2722** | 300 | 🔴 EXCEEDS BY 2422 |
| Production code | ~590 (lines 1-589) | 300 | 🔴 EXCEEDS BY 290 |
| Inline test code | ~2132 (lines 590-2722) | — | 🔴 VIOLATION |

---

## 2. DDD Cohesion Analysis

**Filename reflects domain concept:** `runtime.rs` → Multi-shard runtime orchestrator

| Criterion | Assessment |
|-----------|------------|
| Single domain concept? | ✅ Yes — Runtime is the shard orchestration boundary |
| Filename matches content? | ✅ Yes |
| **DDD Smell Detected** | ⚠️ **YES — God Module due to inline tests** |

**Cohesion Verdict:** The `Runtime` struct represents the correct DDD boundary (multi-shard orchestration), but the file has been polluted with test code, making it a "God Module" in practice.

---

## 3. Violations清单

### 🔴 VIOLATION 1: File Size Exceeded (CRITICAL)
- **Lines:** 2722 total / ~590 production
- **Limit:** 300 lines
- **Exceeded by:** 2422 lines (89% over limit)
- **Remediation:** Mandatory split

### 🔴 VIOLATION 2: Inline Tests Contamination
- **Location:** Lines 590–2722 (`#[cfg(test)] mod tests`)
- **Line count:** ~2132 lines
- **Violations:**
  - Inline test module should be extracted to `tests/runtime_tests.rs` or `runtime/mod.rs` with `runtime/tests/`
  - Test code mixes with production module public API

### 🔴 VIOLATION 3: Inline Test Helper Functions Polluting Production Scope
| Function | Lines | Purpose |
|----------|-------|---------|
| `RejectCompletionJournal` | 608–642 | Test double for journal |
| `suspended_workflow()` | 644–670 | Workflow fixture |
| `action_then_finish_workflow()` | 672–708 | Workflow fixture |
| `runtime_config()` | 710–718 | Test config builder |
| `contract_required_capability()` | 720–722 | Test helper |
| `action_contract()` | 724–737 | Test helper |
| `action_contracts_through()` | 739–757 | Test helper |
| `action_grants()` | 759–761 | Test helper |
| `ticket()` | 763–780 | Test helper |
| `encoded_len()` | 782–790 | Test helper |
| `active_frame()` | 792–798 | Test helper |
| `submit_suspended()` | 800–809 | Test helper |
| `submit_action_then_finish()` | 811–824 | Test helper |
| `finished_workflow()` | 990–1025 | Workflow fixture |
| `wait_then_finish_workflow()` | 2676–2721 | Workflow fixture |
| `assert_suspended_run_is_found()` | 2471–2480 | Test assertion helper |

**Total test helpers:** 16 functions (~600 lines of test infrastructure)

### ⚠️ VIOLATION 4: Oversized Production Functions
| Function | Lines | Threshold | Status |
|----------|-------|-----------|--------|
| `list_active_runs()` | 61 (494–554) | 30 | 🔴 |
| `collect_metrics()` | 58 (413–470) | 30 | 🔴 |
| `migrate_shard()` | 48 (268–315) | 30 | ⚠️ |
| `tick_shard()` | 42 (221–262) | 30 | ⚠️ |
| `new_with_journal()` | 21 (52–69) | 30 | ✅ |

### 🔴 VIOLATION 5: Missing Module Separation
- Tests are inline in production source file
- No `tests/` directory for integration tests
- No `runtime/tests.rs` or `runtime/mod.rs` split

---

## 4. Remediation Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0 (CRITICAL)** | Extract inline tests to `tests/runtime_tests.rs` | High |
| **P0 (CRITICAL)** | Extract test helpers to test module or test file | High |
| **P1 (HIGH)** | Split production code to meet 300-line limit | Medium |
| **P2 (MEDIUM)** | Break oversized functions `list_active_runs`, `collect_metrics` | Medium |
| **P3 (LOW)** | Consider extracting `ShardRouting` trait for `shard_index`/`shard_for` | Low |

---

## 5. Specific Refactoring Plan

### Step 1: Extract Tests
```rust
// Move to: crates/vb_runtime/src/runtime_tests.rs
#[cfg(test)]
mod tests { /* all 2132 lines */ }
```

### Step 2: Split Production Code (Target: 3 files ~200 lines each)
- `runtime.rs` — Core Runtime struct + constructor + tick/shard routing
- `runtime/commands.rs` — Submit, cancel, resume, complete action, fail action, answer ask
- `runtime/observation.rs` — Snapshot, metrics, list events, list active runs, counters

### Step 3: Extract Shard Routing Logic
```rust
// runtime/shard_router.rs
trait ShardRouter { /* shard_index, shard_for, shard_for_mut */ }
```

---

## 6. Evidence

```
File: runtime.rs
Total: 2722 lines
├── Production (1-589): ~589 lines
│   ├── Struct definitions: ~20 lines
│   ├── Core impl (new, tick, migrate): ~120 lines  
│   ├── Command API: ~200 lines
│   └── Observation API: ~249 lines
└── Tests (590-2722): ~2132 lines (78% of file)
    ├── Test helpers: ~600 lines
    └── Test cases: ~1532 lines (67 tests)
```

---

**Report Generated:** 2026-05-29  
**Next Action:** Extract inline tests to `tests/runtime_tests.rs`, then split production code
