# Architectural Drift Report: `vb_runtime::shard::mod.rs`

**Analyzed:** `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/mod.rs`
**Date:** 2026-05-29
**Severity:** HIGH

---

## 1. Line Count Analysis

| File | Lines | Limit (300) | Status |
|------|-------|-------------|--------|
| mod.rs | 32 | ✓ | PASS |
| impl_.rs | 13 | ✓ | PASS |
| lifecycle.rs | 17 | ✓ | PASS |
| tests.rs | 33 | ✓ | PASS |
| completion_watermark.rs | 199 | ✓ | PASS |
| transitions.rs | 202 | ✗ | **VIOLATION** |
| timer_wheel.rs | 452 | ✗ | **VIOLATION** |
| directive.rs | 473 | ✗ | **VIOLATION** |
| types.rs | 883 | ✗ | **VIOLATION** |
| helpers.rs | 2,492 | ✗ | **CRITICAL VIOLATION** |
| helpers/tests.rs | 2,125 | ✗ | **CRITICAL VIOLATION** |
| **TOTAL** | **6,921** | — | **FAIL** |

**`mod.rs` itself is only 32 lines (PASS)**, but the shard module as a whole contains multiple files that violate the 300-line limit.

---

## 2. DDD Cohesion Analysis

**Domain Concept:** `Shard` — A single-threaded execution context owning mutable run state.

**Cohesion Assessment:** MODERATE

The module exports concepts that belong together:
- `ShardDirective`, `ShardCommand` — Commands the shard understands
- `ShardConfig`, `ShardHealth`, `ShardStatus` — Configuration and health
- `RunState`, `RuntimeState`, `RuntimeEvent` — State management
- Lifecycle and transitions — State machine behavior
- Timer wheel — Scheduling mechanism
- Completion watermark — Completion tracking

**However**, the module also mixes in:
- `helpers.rs` (2,492 lines) — Pure helper functions, suggests missing abstraction boundaries
- `helpers/tests.rs` (2,125 lines) — Test infrastructure co-located with production code

### DDD Smells Identified:

| Smell | Severity | Description |
|-------|----------|-------------|
| **God Helper Module** | HIGH | `helpers.rs` at 2,492 lines suggests functions that should be organized into domain-specific modules or extracted to separate domain services |
| **Public Test Module** | MEDIUM | `pub mod tests` exposes test code at the module boundary — tests should be private `#[cfg(test)]` modules or in integration test files |
| **Test Helpers Exposed** | MEDIUM | `pub use helpers::{...}` re-exports test utilities publicly. These should be private implementation details |
| **Cross-Crate Test Re-exports** | MEDIUM | `pub use vb_core::ids::RunId` imported specifically "for tests" — test dependencies should not leak into public API |

---

## 3. Violations Summary

### File Size Violations (4 files exceed 300 lines):

1. **`helpers.rs` — 2,492 lines (730% over limit)**
   - CRITICAL: Must be decomposed into smaller, focused modules
   - Suggested split: `helpers/retry.rs`, `helpers/snapshot.rs`, `helpers/timer.rs`, `helpers/scheduling.rs`

2. **`helpers/tests.rs` — 2,125 lines (608% over limit)**
   - CRITICAL: Test code should not exceed limits either
   - Suggested: Split into `helpers/tests/retry_tests.rs`, `helpers/tests/snapshot_tests.rs`, etc.

3. **`types.rs` — 883 lines (194% over limit)**
   - HIGH: Consider splitting into `types/state.rs`, `types/commands.rs`, `types/events.rs`, `types/config.rs`

4. **`directive.rs` — 473 lines (58% over limit)**
   - MEDIUM: Borderline, consider splitting if it grows further

5. **`timer_wheel.rs` — 452 lines (51% over limit)**
   - MEDIUM: Borderline, but timer wheel is a well-defined concept that could be extracted to `vb_timer_wheel` crate

6. **`transitions.rs` — 202 lines (under limit)**
   - PASS

### Architectural Violations:

| Rule | Violation |
|------|-----------|
| File size < 300 lines | 5 files exceed limit |
| No `pub mod tests` | `pub mod tests;` exposes test code publicly |
| No test utilities in public API | `pub use helpers::{...}` for test helpers |
| Cross-crate test deps | `pub use vb_core::ids::RunId` for tests |

---

## 4. Priority Assessment

| Priority | Item | Effort |
|----------|------|--------|
| **P0 - CRITICAL** | Decompose `helpers.rs` (2,492 lines) | High (requires architectural design) |
| **P0 - CRITICAL** | Decompose `helpers/tests.rs` (2,125 lines) | High |
| **P1 - HIGH** | Split `types.rs` (883 lines) | Medium |
| **P2 - MEDIUM** | Review `pub mod tests` usage | Low |
| **P2 - MEDIUM** | Make test helpers private | Low |
| **P3 - LOW** | `timer_wheel.rs`, `directive.rs` monitoring | Ongoing |

---

## 5. Recommendations

1. **Immediate:** Create a decomposition plan for `helpers.rs` — it contains 2,492 lines of pure helper functions that likely map to distinct domain operations (retry handling, snapshot management, timer registration, slot seeding, etc.)

2. **Short-term:** Convert `pub mod tests` to `mod tests` with `#[cfg(test)]` gating

3. **Short-term:** Move test helpers to a separate `test_helpers` module marked `#[cfg(test)]`

4. **Medium-term:** Split `types.rs` into `state_types.rs`, `command_types.rs`, `event_types.rs`, `config_types.rs`

5. **Consider:** Extracting `timer_wheel.rs` to a standalone `vb_timer_wheel` crate if it has independent utility

---

## 6. Metric Summary

```
Lines Count:       32 (mod.rs only) / 6,921 (module total)
DDD Cohesion:     MODERATE — concepts are related but module is bloated
Violations:        5 file-size, 3 architectural
Priority:          P0 — Critical helpers.rs decomposition required
```
